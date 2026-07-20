use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    Form,
};
use serde::Deserialize;
use std::collections::HashMap;

use qams_core::{Report, Review, Scorecard};
use crate::{db, error::{AppError, Result}, models::ReviewRow, AppState};

// ── Shared presentation types ─────────────────────────────────────────────────

/// A column header in a cross-report table (summary or agent index).
pub struct ColLabel {
    pub label:      String,
    pub is_current: bool,
}

/// A data cell in a cross-report table.
pub struct TableCell {
    pub is_empty: bool,
    pub value:    f64,
    pub display:  String,
}

impl TableCell {
    fn some(score: f64) -> Self {
        TableCell { is_empty: false, value: score * 100.0, display: format!("{:.0}%", score * 100.0) }
    }
    fn none() -> Self {
        TableCell { is_empty: true, value: 0.0, display: String::new() }
    }
}

/// A row in the overall summary table.
pub struct SummaryRow {
    pub criterion: String,
    pub cells:     Vec<TableCell>,
}

/// A row in the agent-report index table.
pub struct AgentIndexRow {
    pub agent_id:   u64,
    pub agent_name: String,
    pub cells:      Vec<TableCell>,
}

/// A column header in the individual agent view.
pub struct ReviewHeader {
    pub date:          String,
    pub reviewer:      String,
    pub score_display: Option<String>,
}

/// A cell in the individual agent criterion table.
pub struct AgentCriterionCell {
    pub option:      String,
    pub css_class:   String,
    pub has_comment: bool,
    pub comment:     String,
}

/// A row in the individual agent criterion table.
pub struct AgentCriterionRow {
    pub criterion: String,
    pub cells:     Vec<AgentCriterionCell>,
}

// ── Templates ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "reports_list.html")]
struct ListTemplate {
    reports: Vec<crate::models::ReportRow>,
}

#[derive(Template)]
#[template(path = "reports_new.html")]
struct NewTemplate {
    scorecards: Vec<crate::models::ScorecardRow>,
}

#[derive(Template)]
#[template(path = "reports_show.html")]
struct ShowTemplate {
    report_id:            u64,
    report_label:         String,
    scorecard_id:         u64,
    scorecard_name:       String,
    start_date:           String,
    end_date:             String,
    review_count:         usize,
    accumulation_display: String,
    agent_links:          Vec<(u64, String)>,
}

#[derive(Template)]
#[template(path = "reports_summary.html")]
struct SummaryTemplate {
    report_id:     u64,
    report_label:  String,
    column_labels: Vec<ColLabel>,
    rows:          Vec<SummaryRow>,
}

#[derive(Template)]
#[template(path = "reports_agents.html")]
struct AgentIndexTemplate {
    report_id:     u64,
    report_label:  String,
    column_labels: Vec<ColLabel>,
    rows:          Vec<AgentIndexRow>,
}

#[derive(Template)]
#[template(path = "reports_agent.html")]
struct AgentPageTemplate {
    report_id:      u64,
    report_label:   String,
    agent_name:     String,
    review_headers: Vec<ReviewHeader>,
    criterion_rows: Vec<AgentCriterionRow>,
}

fn render<T: Template>(t: T) -> Result<impl IntoResponse> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Internal(format!("Template error: {e}")))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let reports = db::list_reports(&state.db).await?;
    render(ListTemplate { reports })
}

pub async fn new_form(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let scorecards = db::list_scorecards(&state.db).await?;
    render(NewTemplate { scorecards })
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let row         = db::get_report(&state.db, id).await?;
    let scorecard   = db::get_scorecard(&state.db, row.scorecard_id).await?;
    let review_rows = db::list_reviews_in_range(
        &state.db, row.scorecard_id, row.start_date, row.end_date,
    ).await?;

    let accumulation_display = match row.accumulation_period {
        Some(n) => format!("{} previous report{}", n, if n == 1 { "" } else { "s" }),
        None    => "All available".to_string(),
    };

    // Collect unique agent ids from reviews then resolve names.
    let agent_names = build_agent_name_map(&state, row.scorecard_id).await?;
    let mut seen_agents: Vec<(u64, String)> = Vec::new();
    for r in &review_rows {
        if !seen_agents.iter().any(|(id, _)| *id == r.agent_id) {
            let name = agent_names.get(&r.agent_id)
                .cloned()
                .unwrap_or_else(|| format!("Agent {}", r.agent_id));
            seen_agents.push((r.agent_id, name));
        }
    }

    render(ShowTemplate {
        report_id:            id,
        report_label:         row.label,
        scorecard_id:         row.scorecard_id,
        scorecard_name:       scorecard.name,
        start_date:           row.start_date.to_string(),
        end_date:             row.end_date.to_string(),
        review_count:         review_rows.len(),
        accumulation_display,
        agent_links:          seen_agents,
    })
}

pub async fn summary(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let (report, scorecard) = load_report_with_history(&state, id).await?;
    let chain = report.report_chain();

    let column_labels: Vec<ColLabel> = chain.iter().map(|r| ColLabel {
        label:      r.label().to_string(),
        is_current: std::ptr::eq(*r, &report),
    }).collect();

    let rows: Vec<SummaryRow> = scorecard.criterion_order().iter().map(|crit| {
        let cells = chain.iter().map(|r| {
            match r.team_criterion_average(&scorecard, crit) {
                Some(avg) => TableCell::some(avg),
                None      => TableCell::none(),
            }
        }).collect();
        SummaryRow { criterion: crit.clone(), cells }
    }).collect();

    render(SummaryTemplate {
        report_id:    id,
        report_label: report.label().to_string(),
        column_labels,
        rows,
    })
}

pub async fn agent_index(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let (report, _scorecard) = load_report_with_history(&state, id).await?;
    let chain = report.report_chain();

    let column_labels: Vec<ColLabel> = chain.iter().map(|r| ColLabel {
        label:      r.label().to_string(),
        is_current: std::ptr::eq(*r, &report),
    }).collect();

    // Collect all agents across the whole chain, in first-seen order.
    let mut all_agents: Vec<String> = Vec::new();
    for r in &chain {
        for agent in r.agents() {
            if !all_agents.iter().any(|a| a == agent) {
                all_agents.push(agent.to_string());
            }
        }
    }

    // Resolve agent names → DB ids for links.
    let agents_db = db::list_agents(&state.db).await?;
    let name_to_id: HashMap<&str, u64> = agents_db.iter()
        .map(|a| (a.name.as_str(), a.id))
        .collect();

    let rows: Vec<AgentIndexRow> = all_agents.iter().map(|agent| {
        let cells = chain.iter().map(|r| {
            match r.agent_average_score(agent) {
                Some(avg) => TableCell::some(avg),
                None      => TableCell::none(),
            }
        }).collect();
        AgentIndexRow {
            agent_id:   name_to_id.get(agent.as_str()).copied().unwrap_or(0),
            agent_name: agent.clone(),
            cells,
        }
    }).collect();

    render(AgentIndexTemplate {
        report_id:    id,
        report_label: report.label().to_string(),
        column_labels,
        rows,
    })
}

pub async fn agent_page(
    State(state): State<AppState>,
    Path((id, agent_id)): Path<(u64, u64)>,
) -> Result<impl IntoResponse> {
    let (report, scorecard) = load_report_with_history(&state, id).await?;
    let agent = db::get_agent(&state.db, agent_id).await?;
    let agent_name = &agent.name;

    // Column headers: one per review in the current report, chronological.
    let agent_reviews = report.agent_reviews(agent_name);
    let review_headers: Vec<ReviewHeader> = agent_reviews.iter().map(|r| {
        let score_pct = r.effective_score() * 100.0;
        ReviewHeader {
            date:          r.date().to_string(),
            reviewer:      r.reviewer().to_string(),
            score_display: Some(format!("{score_pct:.0}%")),
        }
    }).collect();

    // Rows: one per criterion, cells in same review order.
    let criterion_rows: Vec<AgentCriterionRow> = scorecard.criterion_order().iter().map(|crit_name| {
        let entries = report.agent_criterion_selections(agent_name, crit_name);
        let cells: Vec<AgentCriterionCell> = entries.iter().map(|entry| {
            let crit = scorecard.criterion(crit_name);
            let max_pts = crit.map(|c| c.max_points()).unwrap_or(0);
            let css_class = crit
                .and_then(|c| c.option(entry.option))
                .map(|score| match score {
                    qams_core::CriterionScore::Autofail       => "opt-autofail",
                    qams_core::CriterionScore::NotApplicable  => "opt-na",
                    qams_core::CriterionScore::Points(p) if *p == max_pts && max_pts > 0 => "opt-full",
                    _ => "",
                })
                .unwrap_or("")
                .to_string();

            let comment = entry.comment.unwrap_or("").to_string();
            AgentCriterionCell {
                option:      entry.option.to_string(),
                css_class,
                has_comment: !comment.is_empty(),
                comment,
            }
        }).collect();

        AgentCriterionRow { criterion: crit_name.clone(), cells }
    }).collect();

    render(AgentPageTemplate {
        report_id:      id,
        report_label:   report.label().to_string(),
        agent_name:     agent_name.clone(),
        review_headers,
        criterion_rows,
    })
}

#[derive(Deserialize)]
pub struct GenerateForm {
    pub scorecard_id: u64,
    pub start_date:   String,
    pub end_date:     String,
    /// Stored as a plain string so we can handle the empty-string case that
    /// HTML forms submit when a number input is left blank.
    pub accumulation_period: Option<String>,
}

pub async fn generate(
    State(state): State<AppState>,
    Form(form): Form<GenerateForm>,
) -> Result<impl IntoResponse> {
    let start = chrono::NaiveDate::parse_from_str(&form.start_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid start_date".into()))?;
    let end = chrono::NaiveDate::parse_from_str(&form.end_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid end_date".into()))?;

    if end < start {
        return Err(AppError::BadRequest("end_date must be on or after start_date".into()));
    }

    let accumulation_period: Option<u32> = match form.accumulation_period.as_deref() {
        None | Some("") => None,
        Some(s) => Some(s.parse::<u32>()
            .map_err(|_| AppError::BadRequest("accumulation_period must be a positive integer".into()))?),
    };

    let label = format!("{}_to_{}", start, end);
    let id = db::insert_report(
        &state.db, form.scorecard_id, &label, start, end, accumulation_period,
    ).await?;

    Ok(axum::response::Redirect::to(&format!("/reports/{id}")))
}

// ── Core assembly ─────────────────────────────────────────────────────────────

pub async fn load_report_with_history(state: &AppState, report_id: u64) -> Result<(Report, Scorecard)> {
    let row      = db::get_report(&state.db, report_id).await?;
    let scorecard = db::load_scorecard_core(&state.db, row.scorecard_id).await?;
    let agent_names = build_agent_name_map(state, row.scorecard_id).await?;

    let review_rows = db::list_reviews_in_range(
        &state.db, row.scorecard_id, row.start_date, row.end_date,
    ).await?;
    let reviews = review_rows_to_core(&review_rows, &agent_names);

    let limit = match row.accumulation_period {
        Some(n) => n,
        None    => db::count_previous_reports(&state.db, row.scorecard_id, row.start_date).await?,
    };

    let mut previous_reports = Vec::new();
    if limit > 0 {
        let prev_rows = db::previous_reports(
            &state.db, row.scorecard_id, row.start_date, limit,
        ).await?;
        for prev_row in &prev_rows {
            let prev_reviews = review_rows_to_core(
                &db::list_reviews_in_range(
                    &state.db, prev_row.scorecard_id, prev_row.start_date, prev_row.end_date,
                ).await?,
                &agent_names,
            );
            previous_reports.push(Report::new(
                prev_row.label.clone(),
                prev_row.start_date.to_string(),
                prev_row.end_date.to_string(),
                prev_reviews,
                vec![],
            ));
        }
        previous_reports.reverse(); // oldest first
    }

    Ok((Report::new(
        row.label.clone(),
        row.start_date.to_string(),
        row.end_date.to_string(),
        reviews,
        previous_reports,
    ), scorecard))
}

async fn build_agent_name_map(state: &AppState, _scorecard_id: u64) -> Result<HashMap<u64, String>> {
    Ok(db::list_agents(&state.db).await?.into_iter().map(|a| (a.id, a.name)).collect())
}

fn review_rows_to_core(rows: &[ReviewRow], agent_names: &HashMap<u64, String>) -> Vec<Review> {
    rows.iter().map(|r| {
        let agent_name = agent_names.get(&r.agent_id)
            .cloned()
            .unwrap_or_else(|| format!("Agent {}", r.agent_id));

        let selections: HashMap<String, String> = r.selections.0.as_object()
            .map(|o| o.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect())
            .unwrap_or_default();

        let comments: HashMap<String, String> = r.comments.0.as_object()
            .map(|o| o.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect())
            .unwrap_or_default();

        Review::new(agent_name, r.reviewer.clone(), r.date.to_string(),
            selections, comments, r.score, r.adj_score, HashMap::new())
    }).collect()
}