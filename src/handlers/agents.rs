use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Form,
};
use serde::Deserialize;

use crate::{db, error::Result, AppState};

pub async fn list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let agents = db::list_agents(&state.db).await?;
    // TODO: render agents::ListTemplate
    Ok(format!("{} agents", agents.len()))
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let agent = db::get_agent(&state.db, id).await?;
    // TODO: render agents::ShowTemplate
    Ok(agent.name)
}

#[derive(Deserialize)]
pub struct CreateForm {
    pub name:     String,
    /// Optional JSON metadata blob from the form.
    pub metadata: Option<String>,
}

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<CreateForm>,
) -> Result<impl IntoResponse> {
    let metadata = form.metadata
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::Value::Object(Default::default()));
    let id = db::insert_agent(&state.db, &form.name, &metadata).await?;
    Ok(axum::response::Redirect::to(&format!("/agents/{id}")))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    db::delete_agent(&state.db, id).await?;
    Ok(axum::response::Redirect::to("/agents"))
}
