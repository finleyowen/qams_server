use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Form,
};
use serde::Deserialize;

use crate::{db, error::Result, AppState};

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
    // TODO: render scorecards::ShowTemplate
    Ok(scorecard.name)
}

#[derive(Deserialize)]
pub struct CreateForm {
    pub name: String,
    pub csv:  String,
}

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<CreateForm>,
) -> Result<impl IntoResponse> {
    // Validate CSV parses correctly before inserting.
    crate::scorecard::parse_scorecard_csv(&form.csv)
        .map_err(|e| crate::error::AppError::BadRequest(e))?;
    let id = db::insert_scorecard(&state.db, &form.name, &form.csv).await?;
    Ok(axum::response::Redirect::to(&format!("/scorecards/{id}")))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    db::delete_scorecard(&state.db, id).await?;
    Ok(axum::response::Redirect::to("/scorecards"))
}