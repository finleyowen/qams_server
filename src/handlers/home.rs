use askama::Template;
use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use std::collections::HashMap;

use crate::{db, error::{AppError, Result}, AppState};

/// How many of the most recent reports to list on the homepage.
const RECENT_REPORTS: usize = 5;

// ── Templates ─────────────────────────────────────────────────────────────────

/// A report row in the homepage's "Recent reports" list.
pub struct RecentReport {
    pub id:             u64,
    pub label:          String,
    pub scorecard_name: String,
    pub start_date:     String,
    pub end_date:       String,
}

#[derive(Template)]
#[template(path = "home.html")]
struct HomeTemplate {
    scorecard_count: usize,
    agent_count:     usize,
    review_count:    u64,
    report_count:    usize,
    recent_reports:  Vec<RecentReport>,
}

fn render<T: Template>(t: T) -> Result<impl IntoResponse> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Internal(format!("Template error: {e}")))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET / — landing page with section links and recent activity.
pub async fn index(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let scorecards   = db::list_scorecards(&state.db).await?;
    let agents       = db::list_agents(&state.db).await?;
    let review_count = db::count_reviews(&state.db).await?;
    let reports      = db::list_reports(&state.db).await?;

    let scorecard_names: HashMap<u64, &str> = scorecards
        .iter()
        .map(|sc| (sc.id, sc.name.as_str()))
        .collect();

    let recent_reports = reports
        .iter()
        .take(RECENT_REPORTS)
        .map(|r| RecentReport {
            id:             r.id,
            label:          r.label.clone(),
            scorecard_name: scorecard_names.get(&r.scorecard_id).copied().unwrap_or("—").to_string(),
            start_date:     r.start_date.format("%d %b %Y").to_string(),
            end_date:       r.end_date.format("%d %b %Y").to_string(),
        })
        .collect();

    render(HomeTemplate {
        scorecard_count: scorecards.len(),
        agent_count:     agents.len(),
        review_count,
        report_count:    reports.len(),
        recent_reports,
    })
}
