use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Form,
};
use serde::Deserialize;

use crate::{db, error::{AppError, Result}, AppState};

#[derive(Deserialize)]
pub struct FormQuery {
    pub scorecard_id: u64,
}

/// GET /reviews?scorecard_id=<id>
/// Renders a blank review form for the given scorecard.
pub async fn form(
    State(state): State<AppState>,
    Query(query): Query<FormQuery>,
) -> Result<impl IntoResponse> {
    let scorecard = db::get_scorecard(&state.db, query.scorecard_id).await?;
    let _agents = db::list_agents(&state.db).await?;
    // TODO: render reviews::FormTemplate (replaces the old scorecard.html)
    Ok(format!("Review form for scorecard: {}", scorecard.name))
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let review = db::get_review(&state.db, id).await?;
    // TODO: render reviews::ShowTemplate
    Ok(format!("Review {} by {}", review.id, review.reviewer))
}

#[derive(Deserialize)]
pub struct SubmitForm {
    pub scorecard_id: u64,
    pub agent_id:     u64,
    pub reviewer:     String,
    pub date:         String,
    /// JSON-encoded selections map: criterion name → option name.
    pub selections:   String,
    /// JSON-encoded comments map: criterion name → comment.
    pub comments:     Option<String>,
    pub score:        f64,
    pub adj_score:    Option<f64>,
}

/// POST /reviews
/// Submits a completed review.
pub async fn submit(
    State(state): State<AppState>,
    Form(form): Form<SubmitForm>,
) -> Result<impl IntoResponse> {
    let date = chrono::NaiveDate::parse_from_str(&form.date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid date format; expected YYYY-MM-DD".into()))?;

    let selections: serde_json::Value = serde_json::from_str(&form.selections)
        .map_err(|e| AppError::BadRequest(format!("Invalid selections JSON: {e}")))?;

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