use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Form,
};
use serde::Deserialize;

use crate::{db, error::{AppError, Result}, AppState};

pub async fn list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let scorecards = db::list_scorecards(&state.db).await?;
    // TODO: render scorecards::ListTemplate
    Ok(format!("{} scorecards", scorecards.len()))
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let scorecard = db::get_scorecard(&state.db, id).await?;
    let criteria  = db::list_criteria(&state.db, id).await?;
    // TODO: render scorecards::ShowTemplate
    Ok(format!("{} ({} criteria)", scorecard.name, criteria.len()))
}

/// The scorecard creation form accepts a name plus a JSON description of the
/// criteria and their options. Example `criteria_json`:
/// ```json
/// [
///   {
///     "name": "Greeting",
///     "options": [
///       {"name": "YES",  "score_type": "points", "points": 1},
///       {"name": "NO",   "score_type": "points", "points": 0},
///       {"name": "N/A",  "score_type": "na"}
///     ]
///   }
/// ]
/// ```
#[derive(Deserialize)]
pub struct CreateForm {
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

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<CreateForm>,
) -> Result<impl IntoResponse> {
    let criteria: Vec<CriterionInput> = serde_json::from_str(&form.criteria_json)
        .map_err(|e| AppError::BadRequest(format!("Invalid criteria JSON: {e}")))?;

    if criteria.is_empty() {
        return Err(AppError::BadRequest("Scorecard must have at least one criterion".into()));
    }

    // Validate score types and points values before touching the DB.
    for crit in &criteria {
        if crit.name.is_empty() {
            return Err(AppError::BadRequest("Criterion name cannot be empty".into()));
        }
        for opt in &crit.options {
            match opt.score_type.as_str() {
                "points" => {
                    if opt.points.is_none() {
                        return Err(AppError::BadRequest(format!(
                            "Option '{}' has score_type 'points' but no points value",
                            opt.name
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

    // Insert everything in a transaction so a partial failure leaves no orphans.
    let mut tx = state.db.begin().await?;

    let scorecard_id = sqlx::query!(
        "INSERT INTO scorecards (name) VALUES (?)", form.name
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

    Ok(axum::response::Redirect::to(&format!("/scorecards/{scorecard_id}")))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    // Cascades to criteria and criterion_options via FK ON DELETE CASCADE.
    db::delete_scorecard(&state.db, id).await?;
    Ok(axum::response::Redirect::to("/scorecards"))
}