#![allow(dead_code, unused_variables)]

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::{Html, IntoResponse},
    Form,
};
use serde::Deserialize;

use crate::{db, error::{AppError, Result}, models::{AgentRow, CriterionOptionRow, CriterionRow as ModelCriterionRow, ReviewRow, ScorecardRow}, AppState};

// ── Template types ─────────────────────────────────────────────────────────────

/// One option cell as seen by the template.
pub struct OptionCell {
    pub name:      String,
    pub available: bool,
    pub css_class: String,
    pub title:     String,
}

/// One criterion row as seen by the template.
pub struct TemplateCriterionRow {
    pub name:    String,
    /// Sanitised for use as an HTML id.
    pub uid:     String,
    /// One cell per global option column (may be unavailable).
    pub options: Vec<OptionCell>,
}

pub struct SelectionEntry {
    pub criterion: String,
    pub option:    String,
    pub comment:   String,
}

#[derive(Template)]
#[template(path = "reviews_index.html")]
struct IndexTemplate {
    scorecards: Vec<ScorecardRow>,
}

#[derive(Template)]
#[template(path = "reviews_form.html")]
struct FormTemplate {
    scorecard_id:   u64,
    scorecard_name: String,
    agents:         Vec<AgentRow>,
    option_order:   Vec<String>,
    criteria:       Vec<TemplateCriterionRow>,
    /// JSON for the client-side scorer.
    criteria_js:    String,
}

#[derive(Template)]
#[template(path = "reviews_show.html")]
struct ShowTemplate {
    review:           ReviewRow,
    agent_name:       String,
    score_display:    String,
    adj_score_display: String,
    selection_entries: Vec<SelectionEntry>,
}

fn render<T: Template>(t: T) -> Result<impl IntoResponse> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Internal(format!("Template error: {e}")))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /reviews — scorecard picker
pub async fn index(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let scorecards = db::list_scorecards(&state.db).await?;
    render(IndexTemplate { scorecards })
}

#[derive(Deserialize)]
pub struct FormQuery {
    pub scorecard_id: u64,
}

/// GET /reviews/form?scorecard_id=<id>
pub async fn form(
    State(state): State<AppState>,
    Query(query): Query<FormQuery>,
) -> Result<impl IntoResponse> {
    let scorecard      = db::get_scorecard(&state.db, query.scorecard_id).await?;
    let agents         = db::list_agents(&state.db).await?;
    let criterion_rows = db::list_criteria(&state.db, query.scorecard_id).await?;
    let option_rows    = db::list_options_for_scorecard(&state.db, query.scorecard_id).await?;

    // Derive global option order (distinct names in position order).
    let mut option_order: Vec<String> = Vec::new();
    for opt in &option_rows {
        if !option_order.contains(&opt.name) {
            option_order.push(opt.name.clone());
        }
    }

    // Group options by criterion id.
    use std::collections::HashMap;
    let mut by_crit: HashMap<u64, Vec<_>> = HashMap::new();
    for opt in &option_rows {
        by_crit.entry(opt.criterion_id).or_default().push(opt);
    }

    // Build template criterion rows.
    let mut criteria: Vec<TemplateCriterionRow> = Vec::new();
    for crit in &criterion_rows {
        let crit_opts = by_crit.get(&crit.id).cloned().unwrap_or_default();

        // For each global option column, find the matching option or mark unavailable.
        let max_pts = crit_opts.iter()
            .filter(|o| o.score_type == "points")
            .filter_map(|o| o.points)
            .max()
            .unwrap_or(0);

        let option_cells: Vec<OptionCell> = option_order.iter().map(|col_name| {
            match crit_opts.iter().find(|o| &o.name == col_name) {
                None => OptionCell {
                    name:      col_name.clone(),
                    available: false,
                    css_class: String::new(),
                    title:     String::new(),
                },
                Some(opt) => {
                    let (css_class, title) = match opt.score_type.as_str() {
                        "autofail" => ("opt-autofail".into(), "Autofail".into()),
                        "na"       => ("opt-na".into(),       "Not applicable".into()),
                        _ if opt.points == Some(max_pts) && max_pts > 0
                                   => ("opt-full".into(),     "Full points".into()),
                        _          => ("opt-points".into(),   "Partial points".into()),
                    };
                    OptionCell { name: col_name.clone(), available: true, css_class, title }
                }
            }
        }).collect();

        let uid = crit.name.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();

        criteria.push(TemplateCriterionRow {
            name: crit.name.clone(),
            uid,
            options: option_cells,
        });
    }

    // Build criteria_js for client-side scoring.
    let criteria_js = build_criteria_js(&criterion_rows, &by_crit);

    render(FormTemplate {
        scorecard_id:   query.scorecard_id,
        scorecard_name: scorecard.name,
        agents,
        option_order,
        criteria,
        criteria_js,
    })
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let review = db::get_review(&state.db, id).await?;
    let agent  = db::get_agent(&state.db, review.agent_id).await?;
    let criterion_rows = db::list_criteria(&state.db, review.scorecard_id).await?;

    let score_display     = format!("{:.0}%", review.score * 100.0);
    let adj_score_display = review.adj_score
        .map(|s| format!("{:.0}%", s * 100.0))
        .unwrap_or_default();

    // Build selection entries in criterion order.
    let selection_entries: Vec<SelectionEntry> = criterion_rows.iter().map(|crit| {
        let option = review.selections.0
            .get(&crit.name)
            .and_then(|v| v.as_str())
            .unwrap_or("—")
            .to_string();
        let comment = review.comments.0
            .get(&crit.name)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        SelectionEntry { criterion: crit.name.clone(), option, comment }
    }).collect();

    render(ShowTemplate {
        review,
        agent_name: agent.name,
        score_display,
        adj_score_display,
        selection_entries,
    })
}

#[derive(Deserialize)]
pub struct SubmitForm {
    pub scorecard_id: u64,
    pub agent_id:     u64,
    pub reviewer:     String,
    pub date:         String,
    pub selections:   String,
    pub comments:     Option<String>,
    pub score:        f64,
    pub adj_score:    Option<f64>,
}

/// POST /reviews
pub async fn submit(
    State(state): State<AppState>,
    Form(form): Form<SubmitForm>,
) -> Result<impl IntoResponse> {
    let date = chrono::NaiveDate::parse_from_str(&form.date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid date format; expected YYYY-MM-DD".into()))?;

    let selections: serde_json::Value = serde_json::from_str(&form.selections)
        .map_err(|e| AppError::BadRequest(format!("Invalid selections: {e}")))?;

    let comments: serde_json::Value = form.comments
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::Value::Object(Default::default()));

    let id = db::insert_review(
        &state.db,
        form.scorecard_id,
        form.agent_id,
        &form.reviewer,
        date,
        &selections,
        &comments,
        form.score,
        form.adj_score,
    ).await?;

    Ok(axum::response::Redirect::to(&format!("/reviews/{id}")))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_criteria_js(
    criterion_rows: &[ModelCriterionRow],
    by_crit: &std::collections::HashMap<u64, Vec<&CriterionOptionRow>>,
) -> String {
    let items: Vec<String> = criterion_rows.iter().map(|crit| {
        let opts = by_crit.get(&crit.id).cloned().unwrap_or_default();
        let max_pts = opts.iter()
            .filter(|o| o.score_type == "points")
            .filter_map(|o| o.points)
            .max()
            .unwrap_or(0);

        let options_js: Vec<String> = opts.iter().map(|opt| {
            let val = match opt.score_type.as_str() {
                "na"       => r#"{type:"na"}"#.to_string(),
                "autofail" => r#"{type:"autofail"}"#.to_string(),
                _          => format!(r#"{{type:"points",value:{}}}"#, opt.points.unwrap_or(0)),
            };
            format!(r#""{}":{}"#, escape_js(&opt.name), val)
        }).collect();

        format!(
            r#"{{name:"{}",options:{{{}}}}}"#,
            escape_js(&crit.name),
            options_js.join(","),
        )
    }).collect();

    format!("[{}]", items.join(","))
}

fn escape_js(s: &str) -> String {
    s.chars().map(|c| match c {
        '"'  => r#"\""#.to_string(),
        '\\' => r"\\".to_string(),
        '\n' => r"\n".to_string(),
        c    => c.to_string(),
    }).collect()
}