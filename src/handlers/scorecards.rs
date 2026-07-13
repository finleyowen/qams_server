#![allow(dead_code)]

use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    Form,
};
use serde::Deserialize;

use crate::{db, error::{AppError, Result}, models::{CriterionOptionRow, ScorecardRow}, AppState};

// ── Template types ────────────────────────────────────────────────────────────

/// Scorecard enriched with its criterion count for the list view.
pub struct ScorecardSummary {
    pub id:              u64,
    pub name:            String,
    pub criterion_count: usize,
    pub created_at:      chrono::NaiveDateTime,
}

/// Criterion enriched with its options for the show/edit views.
pub struct CriterionWithOptions {
    pub id:       u64,
    pub name:     String,
    pub position: u32,
    pub options:  Vec<CriterionOptionRow>,
}

#[derive(Template)]
#[template(path = "scorecards_list.html")]
struct ListTemplate {
    scorecards: Vec<ScorecardSummary>,
}

#[derive(Template)]
#[template(path = "scorecards_show.html")]
struct ShowTemplate {
    scorecard: ScorecardRow,
    criteria:  Vec<CriterionWithOptions>,
}

#[derive(Template)]
#[template(path = "scorecards_edit.html")]
struct EditTemplate {
    is_edit:                bool,
    scorecard_id:           u64,
    scorecard_name:         String,
    /// JSON array of criteria for the JS builder's `existingData`.
    existing_criteria_json: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn render<T: Template>(t: T) -> Result<impl IntoResponse> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Internal(format!("Template error: {e}")))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let scorecard_rows = db::list_scorecards(&state.db).await?;

    let mut scorecards: Vec<ScorecardSummary> = Vec::new();
    for sc in scorecard_rows {
        let criteria = db::list_criteria(&state.db, sc.id).await?;
        scorecards.push(ScorecardSummary {
            criterion_count: criteria.len(),
            id:         sc.id,
            name:       sc.name,
            created_at: sc.created_at,
        });
    }

    render(ListTemplate { scorecards })
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let scorecard      = db::get_scorecard(&state.db, id).await?;
    let criterion_rows = db::list_criteria(&state.db, id).await?;

    let mut criteria: Vec<CriterionWithOptions> = Vec::new();
    for crit in criterion_rows {
        let options = db::list_options(&state.db, crit.id).await?;
        criteria.push(CriterionWithOptions {
            id:       crit.id,
            name:     crit.name,
            position: crit.position,
            options,
        });
    }

    render(ShowTemplate { scorecard, criteria })
}

pub async fn new_form(State(_state): State<AppState>) -> Result<impl IntoResponse> {
    render(EditTemplate {
        is_edit:                false,
        scorecard_id:           0,
        scorecard_name:         String::new(),
        existing_criteria_json: "[]".to_string(),
    })
}

pub async fn edit_form(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let scorecard      = db::get_scorecard(&state.db, id).await?;
    let criterion_rows = db::list_criteria(&state.db, id).await?;

    let mut criteria_json = Vec::new();
    for crit in &criterion_rows {
        let options = db::list_options(&state.db, crit.id).await?;
        let options_json: Vec<serde_json::Value> = options.iter().map(|opt| {
            serde_json::json!({
                "name":       opt.name,
                "score_type": opt.score_type,
                "points":     opt.points,
            })
        }).collect();
        criteria_json.push(serde_json::json!({
            "name":    crit.name,
            "options": options_json,
        }));
    }

    render(EditTemplate {
        is_edit:                true,
        scorecard_id:           id,
        scorecard_name:         scorecard.name,
        existing_criteria_json: serde_json::to_string(&criteria_json).unwrap_or_else(|_| "[]".into()),
    })
}

// ── Form types ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ScorecardForm {
    pub name:          String,
    pub criteria_json: String,
}

#[derive(Deserialize)]
struct CriterionInput {
    name:    String,
    options: Vec<OptionInput>,
}

#[derive(Deserialize)]
struct OptionInput {
    name:       String,
    score_type: String,
    points:     Option<u32>,
}

// ── Create ────────────────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<ScorecardForm>,
) -> Result<impl IntoResponse> {
    let criteria = parse_and_validate_criteria(&form.criteria_json)?;
    let scorecard_id = insert_scorecard_with_criteria(&state, &form.name, &criteria).await?;
    Ok(axum::response::Redirect::to(&format!("/scorecards/{scorecard_id}")))
}

// ── Update ────────────────────────────────────────────────────────────────────

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Form(form): Form<ScorecardForm>,
) -> Result<impl IntoResponse> {
    db::get_scorecard(&state.db, id).await?; // 404 if not found

    let criteria = parse_and_validate_criteria(&form.criteria_json)?;

    let mut tx = state.db.begin().await?;

    sqlx::query!("UPDATE scorecards SET name = ? WHERE id = ?", form.name, id)
        .execute(&mut *tx).await?;

    // Replace all criteria+options atomically; CASCADE handles criterion_options.
    sqlx::query!("DELETE FROM criteria WHERE scorecard_id = ?", id)
        .execute(&mut *tx).await?;

    for (crit_pos, crit) in criteria.iter().enumerate() {
        let criterion_id = sqlx::query!(
            "INSERT INTO criteria (scorecard_id, name, position) VALUES (?, ?, ?)",
            id, crit.name, crit_pos as u32
        ).execute(&mut *tx).await?.last_insert_id();

        for (opt_pos, opt) in crit.options.iter().enumerate() {
            sqlx::query!(
                "INSERT INTO criterion_options (criterion_id, name, position, score_type, points)
                 VALUES (?, ?, ?, ?, ?)",
                criterion_id, opt.name, opt_pos as u32, opt.score_type, opt.points
            ).execute(&mut *tx).await?;
        }
    }

    tx.commit().await?;
    Ok(axum::response::Redirect::to(&format!("/scorecards/{id}")))
}

// ── Delete ────────────────────────────────────────────────────────────────────

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    db::delete_scorecard(&state.db, id).await?;
    Ok(axum::response::Redirect::to("/scorecards"))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_and_validate_criteria(criteria_json: &str) -> Result<Vec<CriterionInput>> {
    let criteria: Vec<CriterionInput> = serde_json::from_str(criteria_json)
        .map_err(|e| AppError::BadRequest(format!("Invalid criteria JSON: {e}")))?;

    if criteria.is_empty() {
        return Err(AppError::BadRequest("Scorecard must have at least one criterion".into()));
    }

    for crit in &criteria {
        if crit.name.trim().is_empty() {
            return Err(AppError::BadRequest("Criterion name cannot be empty".into()));
        }
        for opt in &crit.options {
            match opt.score_type.as_str() {
                "points" => {
                    if opt.points.is_none() {
                        return Err(AppError::BadRequest(format!(
                            "Option '{}' has score_type 'points' but no points value", opt.name
                        )));
                    }
                }
                "na" | "autofail" => {}
                other => return Err(AppError::BadRequest(format!(
                    "Unknown score_type '{other}'; expected 'points', 'na', or 'autofail'"
                ))),
            }
        }
    }

    Ok(criteria)
}

async fn insert_scorecard_with_criteria(
    state: &AppState,
    name: &str,
    criteria: &[CriterionInput],
) -> Result<u64> {
    let mut tx = state.db.begin().await?;

    let scorecard_id = sqlx::query!(
        "INSERT INTO scorecards (name) VALUES (?)", name
    ).execute(&mut *tx).await?.last_insert_id();

    for (crit_pos, crit) in criteria.iter().enumerate() {
        let criterion_id = sqlx::query!(
            "INSERT INTO criteria (scorecard_id, name, position) VALUES (?, ?, ?)",
            scorecard_id, crit.name, crit_pos as u32
        ).execute(&mut *tx).await?.last_insert_id();

        for (opt_pos, opt) in crit.options.iter().enumerate() {
            sqlx::query!(
                "INSERT INTO criterion_options (criterion_id, name, position, score_type, points)
                 VALUES (?, ?, ?, ?, ?)",
                criterion_id, opt.name, opt_pos as u32, opt.score_type, opt.points
            ).execute(&mut *tx).await?;
        }
    }

    tx.commit().await?;
    Ok(scorecard_id)
}