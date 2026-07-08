use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ScorecardRow {
    pub id:         u64,
    pub name:       String,
    pub csv:        String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentRow {
    pub id:         u64,
    pub name:       String,
    pub metadata:   sqlx::types::Json<serde_json::Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReviewRow {
    pub id:           u64,
    pub scorecard_id: u64,
    pub agent_id:     u64,
    pub reviewer:     String,
    pub date:         NaiveDate,
    pub selections:   sqlx::types::Json<serde_json::Value>,
    pub comments:     sqlx::types::Json<serde_json::Value>,
    pub score:        f64,
    pub adj_score:    Option<f64>,
    pub created_at:   NaiveDateTime,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReportRow {
    pub id:           u64,
    pub scorecard_id: u64,
    pub label:        String,
    pub start_date:   NaiveDate,
    pub end_date:     NaiveDate,
    pub created_at:   NaiveDateTime,
}