use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    Form,
};
use serde::Deserialize;

use crate::{db, error::{AppError, Result}, models::AgentRow, AppState};

// ── Templates ─────────────────────────────────────────────────────────────────

#[derive(Template)]
#[template(path = "agents_list.html")]
struct ListTemplate {
    agents: Vec<AgentRow>,
}

#[derive(Template)]
#[template(path = "agents_new.html")]
struct NewTemplate;

#[derive(Template)]
#[template(path = "agents_show.html")]
struct ShowTemplate {
    agent:          AgentRow,
    metadata_pairs: Vec<(String, String)>,
}

fn render<T: Template>(t: T) -> Result<impl IntoResponse> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Internal(format!("Template error: {e}")))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let agents = db::list_agents(&state.db).await?;
    render(ListTemplate { agents })
}

pub async fn new_form(State(_state): State<AppState>) -> Result<impl IntoResponse> {
    render(NewTemplate)
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let agent = db::get_agent(&state.db, id).await?;

    // Flatten the JSON metadata object into sorted (key, value) pairs.
    let metadata_pairs: Vec<(String, String)> = agent.metadata.0
        .as_object()
        .map(|obj| {
            let mut pairs: Vec<(String, String)> = obj
                .iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect();
            pairs.sort_by(|a, b| a.0.cmp(&b.0));
            pairs
        })
        .unwrap_or_default();

    render(ShowTemplate { agent, metadata_pairs })
}

#[derive(Deserialize)]
pub struct CreateForm {
    pub name:     String,
    pub metadata: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<CreateForm>,
) -> Result<impl IntoResponse> {
    if form.name.trim().is_empty() {
        return Err(AppError::BadRequest("Agent name cannot be empty".into()));
    }
    let metadata = form.metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::Value::Object(Default::default()));
    let id = db::insert_agent(&state.db, form.name.trim(), &metadata).await?;
    Ok(axum::response::Redirect::to(&format!("/agents/{id}")))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    db::delete_agent(&state.db, id).await?;
    Ok(axum::response::Redirect::to("/agents"))
}