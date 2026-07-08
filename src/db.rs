use crate::{error::{AppError, Result}, models::*};
use sqlx::MySqlPool;

// ── Scorecards ────────────────────────────────────────────────────────────────

pub async fn list_scorecards(db: &MySqlPool) -> Result<Vec<ScorecardRow>> {
    Ok(sqlx::query_as!(
        ScorecardRow,
        "SELECT id, name, csv, created_at FROM scorecards ORDER BY created_at DESC"
    ).fetch_all(db).await?)
}

pub async fn get_scorecard(db: &MySqlPool, id: u64) -> Result<ScorecardRow> {
    sqlx::query_as!(
        ScorecardRow,
        "SELECT id, name, csv, created_at FROM scorecards WHERE id = ?",
        id
    ).fetch_optional(db).await?
    .ok_or(AppError::NotFound)
}

pub async fn insert_scorecard(db: &MySqlPool, name: &str, csv: &str) -> Result<u64> {
    let result = sqlx::query!(
        "INSERT INTO scorecards (name, csv) VALUES (?, ?)",
        name, csv
    ).execute(db).await?;
    Ok(result.last_insert_id())
}

pub async fn delete_scorecard(db: &MySqlPool, id: u64) -> Result<()> {
    sqlx::query!("DELETE FROM scorecards WHERE id = ?", id)
        .execute(db).await?;
    Ok(())
}

// ── Agents ────────────────────────────────────────────────────────────────────

pub async fn list_agents(db: &MySqlPool) -> Result<Vec<AgentRow>> {
    Ok(sqlx::query_as!(
        AgentRow,
        r#"SELECT id, name, metadata AS "metadata: sqlx::types::Json<serde_json::Value>", created_at FROM agents ORDER BY name ASC"#
    ).fetch_all(db).await?)
}

pub async fn get_agent(db: &MySqlPool, id: u64) -> Result<AgentRow> {
    sqlx::query_as!(
        AgentRow,
        r#"SELECT id, name, metadata AS "metadata: sqlx::types::Json<serde_json::Value>", created_at FROM agents WHERE id = ?"#,
        id
    ).fetch_optional(db).await?
    .ok_or(AppError::NotFound)
}

pub async fn insert_agent(db: &MySqlPool, name: &str, metadata: &serde_json::Value) -> Result<u64> {
    let result = sqlx::query!(
        "INSERT INTO agents (name, metadata) VALUES (?, ?)",
        name, serde_json::to_string(metadata).unwrap_or_default()
    ).execute(db).await?;
    Ok(result.last_insert_id())
}

pub async fn delete_agent(db: &MySqlPool, id: u64) -> Result<()> {
    sqlx::query!("DELETE FROM agents WHERE id = ?", id)
        .execute(db).await?;
    Ok(())
}

// ── Reviews ───────────────────────────────────────────────────────────────────

pub async fn list_reviews_in_range(
    db: &MySqlPool,
    scorecard_id: u64,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> Result<Vec<ReviewRow>> {
    Ok(sqlx::query_as!(
        ReviewRow,
        r#"SELECT
            id, scorecard_id, agent_id, reviewer, date,
            selections AS "selections: sqlx::types::Json<serde_json::Value>",
            comments   AS "comments:   sqlx::types::Json<serde_json::Value>",
            score, adj_score, created_at
           FROM reviews
           WHERE scorecard_id = ? AND date BETWEEN ? AND ?
           ORDER BY date ASC"#,
        scorecard_id, start_date, end_date
    ).fetch_all(db).await?)
}

pub async fn get_review(db: &MySqlPool, id: u64) -> Result<ReviewRow> {
    sqlx::query_as!(
        ReviewRow,
        r#"SELECT
            id, scorecard_id, agent_id, reviewer, date,
            selections AS "selections: sqlx::types::Json<serde_json::Value>",
            comments   AS "comments:   sqlx::types::Json<serde_json::Value>",
            score, adj_score, created_at
           FROM reviews WHERE id = ?"#,
        id
    ).fetch_optional(db).await?
    .ok_or(AppError::NotFound)
}

pub async fn insert_review(
    db: &MySqlPool,
    scorecard_id: u64,
    agent_id: u64,
    reviewer: &str,
    date: chrono::NaiveDate,
    selections: &serde_json::Value,
    comments: &serde_json::Value,
    score: f64,
    adj_score: Option<f64>,
) -> Result<u64> {
    let result = sqlx::query!(
        "INSERT INTO reviews (scorecard_id, agent_id, reviewer, date, selections, comments, score, adj_score)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        scorecard_id,
        agent_id,
        reviewer,
        date,
        serde_json::to_string(selections).unwrap_or_default(),
        serde_json::to_string(comments).unwrap_or_default(),
        score,
        adj_score,
    ).execute(db).await?;
    Ok(result.last_insert_id())
}

// ── Reports ───────────────────────────────────────────────────────────────────

pub async fn list_reports(db: &MySqlPool) -> Result<Vec<ReportRow>> {
    Ok(sqlx::query_as!(
        ReportRow,
        "SELECT id, scorecard_id, label, start_date, end_date, created_at
         FROM reports ORDER BY start_date DESC"
    ).fetch_all(db).await?)
}

pub async fn get_report(db: &MySqlPool, id: u64) -> Result<ReportRow> {
    sqlx::query_as!(
        ReportRow,
        "SELECT id, scorecard_id, label, start_date, end_date, created_at
         FROM reports WHERE id = ?",
        id
    ).fetch_optional(db).await?
    .ok_or(AppError::NotFound)
}

pub async fn previous_reports(
    db: &MySqlPool,
    scorecard_id: u64,
    before_start_date: chrono::NaiveDate,
    limit: u32,
) -> Result<Vec<ReportRow>> {
    Ok(sqlx::query_as!(
        ReportRow,
        "SELECT id, scorecard_id, label, start_date, end_date, created_at
         FROM reports
         WHERE scorecard_id = ? AND start_date < ?
         ORDER BY start_date DESC
         LIMIT ?",
        scorecard_id, before_start_date, limit
    ).fetch_all(db).await?)
}

pub async fn insert_report(
    db: &MySqlPool,
    scorecard_id: u64,
    label: &str,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
) -> Result<u64> {
    let result = sqlx::query!(
        "INSERT INTO reports (scorecard_id, label, start_date, end_date) VALUES (?, ?, ?, ?)",
        scorecard_id, label, start_date, end_date
    ).execute(db).await?;
    Ok(result.last_insert_id())
}
