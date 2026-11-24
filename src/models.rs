use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ScorecardRow {
    pub id:                  u64,
    pub name:                String,
    /// Default reporting period length in days. None means no default.
    pub default_period_days: Option<u32>,
    pub created_at:          NaiveDateTime,
}

/// A criterion belonging to a scorecard, with its display/sort order.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CriterionRow {
    pub id:           u64,
    pub scorecard_id: u64,
    pub name:         String,
    /// 0-based display order within the scorecard.
    pub position:     u32,
    pub created_at:   NaiveDateTime,
}

/// One selectable option on a criterion.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct CriterionOptionRow {
    pub id:           u64,
    pub criterion_id: u64,
    pub name:         String,
    /// 0-based display order within the criterion.
    pub position:     u32,
    /// One of: "points", "na", "autofail"
    pub score_type:   String,
    /// Only set when score_type = "points".
    pub points:       Option<u32>,
    pub created_at:   NaiveDateTime,
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
    /// JSON object: criterion name → selected option name.
    pub selections:   sqlx::types::Json<serde_json::Value>,
    /// JSON object: criterion name → comment string.
    pub comments:     sqlx::types::Json<serde_json::Value>,
    pub score:        f64,
    pub adj_score:    Option<f64>,
    pub created_at:   NaiveDateTime,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ReportRow {
    pub id:                   u64,
    pub scorecard_id:         u64,
    pub label:                String,
    pub start_date:           NaiveDate,
    pub end_date:             NaiveDate,
    /// NULL means use all available previous reports.
    pub accumulation_period:  Option<u32>,
    pub created_at:           NaiveDateTime,
}
/// A named score band on a scorecard. A score receives the grade with the
/// highest `min_score` it reaches.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GradeRow {
    pub id:           u64,
    pub scorecard_id: u64,
    pub name:         String,
    /// Minimum score as a percentage (0–100).
    pub min_score:    f64,
    /// CSS hex colour, e.g. "#16a34a".
    pub bg_color:     String,
    pub text_color:   String,
    pub created_at:   NaiveDateTime,
}

/// The grade for a score given as a fraction (0.0–1.0), if any.
/// `grades` must be sorted by `min_score` ascending (as `db::list_grades` returns them).
pub fn grade_for(grades: &[GradeRow], score: f64) -> Option<&GradeRow> {
    // Round to the displayed precision so "80%" never shows as below an 80 threshold.
    let pct = (score * 1000.0).round() / 10.0;
    grades.iter().rev().find(|g| pct >= g.min_score)
}
