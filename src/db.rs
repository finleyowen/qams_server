#![allow(dead_code)]
use crate::{error::{AppError, Result}, models::*};
use sqlx::MySqlPool;
use std::collections::HashMap;
use qams_core::{Criterion, CriterionScore, Scorecard};

// ── Scorecards ────────────────────────────────────────────────────────────────

pub async fn list_scorecards(db: &MySqlPool) -> Result<Vec<ScorecardRow>> {
    Ok(sqlx::query_as!(
        ScorecardRow,
        "SELECT id, name, default_period_days, created_at FROM scorecards ORDER BY created_at DESC"
    ).fetch_all(db).await?)
}

pub async fn get_scorecard(db: &MySqlPool, id: u64) -> Result<ScorecardRow> {
    sqlx::query_as!(
        ScorecardRow,
        "SELECT id, name, default_period_days, created_at FROM scorecards WHERE id = ?",
        id
    ).fetch_optional(db).await?
    .ok_or(AppError::NotFound)
}

pub async fn insert_scorecard(db: &MySqlPool, name: &str, default_period_days: Option<u32>) -> Result<u64> {
    let result = sqlx::query!(
        "INSERT INTO scorecards (name, default_period_days) VALUES (?, ?)",
        name, default_period_days
    ).execute(db).await?;
    Ok(result.last_insert_id())
}

pub async fn update_scorecard_name(db: &MySqlPool, id: u64, name: &str, default_period_days: Option<u32>) -> Result<()> {
    sqlx::query!(
        "UPDATE scorecards SET name = ?, default_period_days = ? WHERE id = ?",
        name, default_period_days, id
    ).execute(db).await?;
    Ok(())
}

pub async fn delete_scorecard(db: &MySqlPool, id: u64) -> Result<()> {
    sqlx::query!("DELETE FROM scorecards WHERE id = ?", id)
        .execute(db).await?;
    Ok(())
}

// ── Criteria ──────────────────────────────────────────────────────────────────

pub async fn list_criteria(db: &MySqlPool, scorecard_id: u64) -> Result<Vec<CriterionRow>> {
    Ok(sqlx::query_as!(
        CriterionRow,
        "SELECT id, scorecard_id, name, position, created_at
         FROM criteria WHERE scorecard_id = ? ORDER BY position ASC",
        scorecard_id
    ).fetch_all(db).await?)
}

pub async fn insert_criterion(
    db: &MySqlPool,
    scorecard_id: u64,
    name: &str,
    position: u32,
) -> Result<u64> {
    let result = sqlx::query!(
        "INSERT INTO criteria (scorecard_id, name, position) VALUES (?, ?, ?)",
        scorecard_id, name, position
    ).execute(db).await?;
    Ok(result.last_insert_id())
}

pub async fn delete_criteria_for_scorecard(db: &MySqlPool, scorecard_id: u64) -> Result<()> {
    sqlx::query!("DELETE FROM criteria WHERE scorecard_id = ?", scorecard_id)
        .execute(db).await?;
    Ok(())
}

// ── Criterion options ─────────────────────────────────────────────────────────

pub async fn list_options(db: &MySqlPool, criterion_id: u64) -> Result<Vec<CriterionOptionRow>> {
    Ok(sqlx::query_as!(
        CriterionOptionRow,
        "SELECT id, criterion_id, name, position, score_type, points, created_at
         FROM criterion_options WHERE criterion_id = ? ORDER BY position ASC",
        criterion_id
    ).fetch_all(db).await?)
}

pub async fn list_options_for_scorecard(
    db: &MySqlPool,
    scorecard_id: u64,
) -> Result<Vec<CriterionOptionRow>> {
    Ok(sqlx::query_as!(
        CriterionOptionRow,
        "SELECT co.id, co.criterion_id, co.name, co.position, co.score_type, co.points, co.created_at
         FROM criterion_options co
         JOIN criteria c ON c.id = co.criterion_id
         WHERE c.scorecard_id = ?
         ORDER BY c.position ASC, co.position ASC",
        scorecard_id
    ).fetch_all(db).await?)
}

pub async fn insert_option(
    db: &MySqlPool,
    criterion_id: u64,
    name: &str,
    position: u32,
    score_type: &str,
    points: Option<u32>,
) -> Result<u64> {
    let result = sqlx::query!(
        "INSERT INTO criterion_options (criterion_id, name, position, score_type, points)
         VALUES (?, ?, ?, ?, ?)",
        criterion_id, name, position, score_type, points
    ).execute(db).await?;
    Ok(result.last_insert_id())
}

// ── Build qams_core::Scorecard from DB rows ───────────────────────────────────

/// Loads a fully-populated `qams_core::Scorecard` for the given scorecard ID.
/// Fires two queries (criteria + options for the whole scorecard) then
/// assembles the core type in memory.
pub async fn load_scorecard_core(db: &MySqlPool, scorecard_id: u64) -> Result<Scorecard> {
    let criterion_rows = list_criteria(db, scorecard_id).await?;
    let option_rows    = list_options_for_scorecard(db, scorecard_id).await?;

    // Group options by criterion_id.
    let mut options_by_criterion: HashMap<u64, Vec<&CriterionOptionRow>> = HashMap::new();
    for opt in &option_rows {
        options_by_criterion.entry(opt.criterion_id).or_default().push(opt);
    }

    // Derive the global option order from the first criterion that has all
    // options — in practice all criteria share the same option column set,
    // so we collect distinct option names in position order across all rows.
    let mut option_order: Vec<String> = Vec::new();
    for opt in &option_rows {
        if !option_order.contains(&opt.name) {
            option_order.push(opt.name.clone());
        }
    }

    let mut criteria_map: HashMap<String, Criterion> = HashMap::new();
    let mut criterion_order: Vec<String> = Vec::new();

    for crit_row in &criterion_rows {
        criterion_order.push(crit_row.name.clone());

        let options: HashMap<String, CriterionScore> = options_by_criterion
            .get(&crit_row.id)
            .map(|opts| {
                opts.iter().map(|opt| {
                    let score = match opt.score_type.as_str() {
                        "na"       => CriterionScore::NotApplicable,
                        "autofail" => CriterionScore::Autofail,
                        _          => CriterionScore::Points(opt.points.unwrap_or(0)),
                    };
                    (opt.name.clone(), score)
                }).collect()
            })
            .unwrap_or_default();

        criteria_map.insert(crit_row.name.clone(), Criterion::new(options));
    }

    Ok(Scorecard::new(criteria_map, option_order, criterion_order))
}

// ── Agents ────────────────────────────────────────────────────────────────────

pub async fn list_agents(db: &MySqlPool) -> Result<Vec<AgentRow>> {
    Ok(sqlx::query_as!(
        AgentRow,
        r#"SELECT id, name, metadata AS "metadata: sqlx::types::Json<serde_json::Value>", created_at
           FROM agents ORDER BY name ASC"#
    ).fetch_all(db).await?)
}

pub async fn get_agent(db: &MySqlPool, id: u64) -> Result<AgentRow> {
    sqlx::query_as!(
        AgentRow,
        r#"SELECT id, name, metadata AS "metadata: sqlx::types::Json<serde_json::Value>", created_at
           FROM agents WHERE id = ?"#,
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
        scorecard_id, agent_id, reviewer, date,
        serde_json::to_string(selections).unwrap_or_default(),
        serde_json::to_string(comments).unwrap_or_default(),
        score, adj_score,
    ).execute(db).await?;
    Ok(result.last_insert_id())
}

// ── Reports ───────────────────────────────────────────────────────────────────

pub async fn list_reports(db: &MySqlPool) -> Result<Vec<ReportRow>> {
    Ok(sqlx::query_as!(
        ReportRow,
        "SELECT id, scorecard_id, label, start_date, end_date, accumulation_period, created_at
         FROM reports ORDER BY start_date DESC"
    ).fetch_all(db).await?)
}

pub async fn get_report(db: &MySqlPool, id: u64) -> Result<ReportRow> {
    sqlx::query_as!(
        ReportRow,
        "SELECT id, scorecard_id, label, start_date, end_date, accumulation_period, created_at
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
        "SELECT id, scorecard_id, label, start_date, end_date, accumulation_period, created_at
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
    accumulation_period: Option<u32>,
) -> Result<u64> {
    let result = sqlx::query!(
        "INSERT INTO reports (scorecard_id, label, start_date, end_date, accumulation_period) VALUES (?, ?, ?, ?, ?)",
        scorecard_id, label, start_date, end_date, accumulation_period
    ).execute(db).await?;
    Ok(result.last_insert_id())
}

/// Returns how many previous reports exist for a scorecard before a given date.
/// Used to implement the "all available" accumulation period default.
pub async fn count_previous_reports(
    db: &MySqlPool,
    scorecard_id: u64,
    before_start_date: chrono::NaiveDate,
) -> Result<u32> {
    let row = sqlx::query!(
        "SELECT COUNT(*) as count FROM reports WHERE scorecard_id = ? AND start_date < ?",
        scorecard_id, before_start_date
    ).fetch_one(db).await?;
    Ok(row.count as u32)
}

/// Returns the end date of the most recent report for a given scorecard,
/// used to auto-fill the next report's start date.
pub async fn last_report_end_date(
    db: &MySqlPool,
    scorecard_id: u64,
) -> Result<Option<chrono::NaiveDate>> {
    let row = sqlx::query!(
        "SELECT end_date FROM reports WHERE scorecard_id = ? ORDER BY end_date DESC LIMIT 1",
        scorecard_id
    ).fetch_optional(db).await?;
    Ok(row.map(|r| r.end_date))
}