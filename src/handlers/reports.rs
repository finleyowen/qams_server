use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Form,
};
use serde::Deserialize;
use std::collections::HashMap;

use qams_core::{Report, Review, Scorecard};
use crate::{db, error::{AppError, Result}, AppState};
use crate::scorecard::parse_scorecard_csv;

const DEFAULT_ACCUMULATION_PERIOD: u32 = 4;

pub async fn list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let reports = db::list_reports(&state.db).await?;
    // TODO: render reports::ListTemplate
    Ok(format!("{} reports", reports.len()))
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let report = db::get_report(&state.db, id).await?;
    // TODO: render reports::ShowTemplate (index page)
    Ok(format!("Report: {}", report.label))
}

pub async fn summary(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let (report, _scorecard) = load_report_with_history(&state, id).await?;
    // TODO: render reports::SummaryTemplate
    Ok(format!("Summary for {}", report.label()))
}

pub async fn agent_index(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let (report, _scorecard) = load_report_with_history(&state, id).await?;
    // TODO: render reports::AgentIndexTemplate
    Ok(format!("Agent index for {}", report.label()))
}

pub async fn agent_page(
    State(state): State<AppState>,
    Path((id, agent_id)): Path<(u64, u64)>,
) -> Result<impl IntoResponse> {
    let (report, _scorecard) = load_report_with_history(&state, id).await?;
    let agent = db::get_agent(&state.db, agent_id).await?;
    // TODO: render reports::AgentPageTemplate
    Ok(format!("Agent page for {} in report {}", agent.name, report.label()))
}

#[derive(Deserialize)]
pub struct GenerateForm {
    pub scorecard_id:        u64,
    pub start_date:          String,
    pub end_date:            String,
    pub accumulation_period: Option<u32>,
}

/// POST /reports — generate a new report from reviews in a date range.
pub async fn generate(
    State(state): State<AppState>,
    Form(form): Form<GenerateForm>,
) -> Result<impl IntoResponse> {
    let start = chrono::NaiveDate::parse_from_str(&form.start_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid start_date".into()))?;
    let end = chrono::NaiveDate::parse_from_str(&form.end_date, "%Y-%m-%d")
        .map_err(|_| AppError::BadRequest("Invalid end_date".into()))?;

    let _accumulation_period = form.accumulation_period.unwrap_or(DEFAULT_ACCUMULATION_PERIOD);

    let id = db::insert_report(&state.db, form.scorecard_id, &format!("{}_to_{}", start, end), start, end).await?;
    Ok(axum::response::Redirect::to(&format!("/reports/{id}")))
}

/// Loads a `ReportRow` from the DB, fetches its reviews and the N previous
/// reports, and assembles a `qams_core::Report` ready for rendering.
async fn load_report_with_history(state: &AppState, report_id: u64) -> Result<(Report, Scorecard)> {
    let row = db::get_report(&state.db, report_id).await?;
    let scorecard_row = db::get_scorecard(&state.db, row.scorecard_id).await?;
    let scorecard = parse_scorecard_csv(&scorecard_row.csv)
        .map_err(|e| AppError::Internal(e))?;

    let review_rows = db::list_reviews_in_range(
        &state.db, row.scorecard_id, row.start_date, row.end_date,
    ).await?;

    let reviews = review_rows_to_core(&review_rows);

    let prev_rows = db::previous_reports(
        &state.db, row.scorecard_id, row.start_date, DEFAULT_ACCUMULATION_PERIOD,
    ).await?;

    let mut previous_reports = Vec::new();
    for prev_row in &prev_rows {
        let prev_review_rows = db::list_reviews_in_range(
            &state.db, prev_row.scorecard_id, prev_row.start_date, prev_row.end_date,
        ).await?;
        previous_reports.push(Report::new(
            prev_row.label.clone(),
            prev_row.start_date.to_string(),
            prev_row.end_date.to_string(),
            review_rows_to_core(&prev_review_rows),
            vec![],
        ));
    }
    // previous_reports comes back newest-first from the DB query; reverse to
    // oldest-first as Report expects.
    previous_reports.reverse();

    let report = Report::new(
        row.label.clone(),
        row.start_date.to_string(),
        row.end_date.to_string(),
        reviews,
        previous_reports,
    );

    Ok((report, scorecard))
}

fn review_rows_to_core(rows: &[crate::models::ReviewRow]) -> Vec<Review> {
    rows.iter().map(|r| {
        let selections: HashMap<String, String> = r.selections.0
            .as_object()
            .map(|o| o.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect())
            .unwrap_or_default();

        let comments: HashMap<String, String> = r.comments.0
            .as_object()
            .map(|o| o.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect())
            .unwrap_or_default();

        Review::new(
            r.agent_id.to_string(), // placeholder until we JOIN agents
            r.reviewer.clone(),
            r.date.to_string(),
            selections,
            comments,
            r.score,
            r.adj_score,
            HashMap::new(),
        )
    }).collect()
}