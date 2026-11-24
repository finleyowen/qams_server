#![allow(dead_code)]
use askama::Template;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse},
    Form,
};
use serde::Deserialize;

use crate::{db, error::{AppError, Result}, models::{CriterionOptionRow, GradeRow, ScorecardRow}, AppState};

// ── Template types ────────────────────────────────────────────────────────────

/// Scorecard enriched with its criterion count for the list view.
pub struct ScorecardSummary {
    pub id:                  u64,
    pub name:                String,
    pub criterion_count:     usize,
    pub created_at:          chrono::NaiveDateTime,
}

/// Criterion enriched with its options for the show/edit views.
pub struct CriterionWithOptions {
    pub id:       u64,
    pub name:     String,
    pub position: u32,
    pub options:  Vec<CriterionOptionRow>,
}

#[derive(Template)]
#[template(path = "scorecards_list.html")]
struct ListTemplate {
    scorecards: Vec<ScorecardSummary>,
}

#[derive(Template)]
#[template(path = "scorecards_show.html")]
struct ShowTemplate {
    scorecard: ScorecardRow,
    criteria:  Vec<CriterionWithOptions>,
    grades:    Vec<GradeRow>,
}

#[derive(Template)]
#[template(path = "scorecards_edit.html")]
struct EditTemplate {
    is_edit:                bool,
    scorecard_id:           u64,
    scorecard_name:         String,
    default_period_days:    String,
    /// JSON array of criteria for the JS builder's `existingData`.
    /// Script-safe (see `script_json`); rendered with `|safe`.
    existing_criteria_json: String,
    /// JSON array of grades for the JS builder. Script-safe, rendered with `|safe`.
    existing_grades_json:   String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Serialises a value as JSON that is safe to embed in an inline `<script>`
/// with `|safe`: escaping `<`, `>` and `&` stops data like `</script>` from
/// ending the script block early.
fn script_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "[]".into())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn render<T: Template>(t: T) -> Result<impl IntoResponse> {
    t.render()
        .map(Html)
        .map_err(|e| AppError::Internal(format!("Template error: {e}")))
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn list(State(state): State<AppState>) -> Result<impl IntoResponse> {
    let scorecard_rows = db::list_scorecards(&state.db).await?;

    let mut scorecards: Vec<ScorecardSummary> = Vec::new();
    for sc in scorecard_rows {
        let criteria = db::list_criteria(&state.db, sc.id).await?;
        scorecards.push(ScorecardSummary {
            criterion_count: criteria.len(),
            id:         sc.id,
            name:       sc.name,
            created_at: sc.created_at,
        });
    }

    render(ListTemplate { scorecards })
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let scorecard      = db::get_scorecard(&state.db, id).await?;
    let criterion_rows = db::list_criteria(&state.db, id).await?;

    let mut criteria: Vec<CriterionWithOptions> = Vec::new();
    for crit in criterion_rows {
        let options = db::list_options(&state.db, crit.id).await?;
        criteria.push(CriterionWithOptions {
            id:       crit.id,
            name:     crit.name,
            position: crit.position,
            options,
        });
    }

    let grades = db::list_grades(&state.db, id).await?;

    render(ShowTemplate { scorecard, criteria, grades })
}

pub async fn new_form(State(_state): State<AppState>) -> Result<impl IntoResponse> {
    render(EditTemplate {
        is_edit:                false,
        scorecard_id:           0,
        scorecard_name:         String::new(),
        default_period_days:    String::new(),
        existing_criteria_json: "[]".to_string(),
        existing_grades_json:   "[]".to_string(),
    })
}

pub async fn edit_form(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    let scorecard      = db::get_scorecard(&state.db, id).await?;
    let criterion_rows = db::list_criteria(&state.db, id).await?;

    let mut criteria_json = Vec::new();
    for crit in &criterion_rows {
        let options = db::list_options(&state.db, crit.id).await?;
        let options_json: Vec<serde_json::Value> = options.iter().map(|opt| {
            serde_json::json!({
                "name":       opt.name,
                "score_type": opt.score_type,
                "points":     opt.points,
            })
        }).collect();
        criteria_json.push(serde_json::json!({
            "name":    crit.name,
            "options": options_json,
        }));
    }

    let grades_json: Vec<serde_json::Value> = db::list_grades(&state.db, id).await?
        .iter()
        .map(|g| serde_json::json!({
            "name":       g.name,
            "min_score":  g.min_score,
            "bg_color":   g.bg_color,
            "text_color": g.text_color,
        }))
        .collect();

    render(EditTemplate {
        is_edit:                true,
        scorecard_id:           id,
        scorecard_name:         scorecard.name,
        default_period_days:    scorecard.default_period_days.map(|d| d.to_string()).unwrap_or_default(),
        existing_criteria_json: script_json(&criteria_json),
        existing_grades_json:   script_json(&grades_json),
    })
}

// ── Form types ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ScorecardForm {
    pub name:                String,
    pub criteria_json:       String,
    /// Absent when a client doesn't send grades; treated as "no grades".
    pub grades_json:         Option<String>,
    pub default_period_days: Option<String>,
}

fn parse_period_days(s: &Option<String>) -> Option<u32> {
    s.as_deref().filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
}

#[derive(Deserialize)]
struct CriterionInput {
    name:    String,
    options: Vec<OptionInput>,
}

#[derive(Deserialize)]
struct OptionInput {
    name:       String,
    score_type: String,
    points:     Option<u32>,
}

// ── Create ────────────────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<AppState>,
    Form(form): Form<ScorecardForm>,
) -> Result<impl IntoResponse> {
    let criteria = parse_and_validate_criteria(&form.criteria_json)?;
    let grades = parse_and_validate_grades(form.grades_json.as_deref())?;
    let period_days = parse_period_days(&form.default_period_days);
    let scorecard_id = insert_scorecard_with_criteria(&state, &form.name, period_days, &criteria, &grades).await?;
    Ok(axum::response::Redirect::to(&format!("/scorecards/{scorecard_id}")))
}

// ── Update ────────────────────────────────────────────────────────────────────

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Form(form): Form<ScorecardForm>,
) -> Result<impl IntoResponse> {
    db::get_scorecard(&state.db, id).await?; // 404 if not found

    let criteria = parse_and_validate_criteria(&form.criteria_json)?;
    let grades = parse_and_validate_grades(form.grades_json.as_deref())?;

    let mut tx = state.db.begin().await?;

    let period_days = parse_period_days(&form.default_period_days);
    sqlx::query!("UPDATE scorecards SET name = ?, default_period_days = ? WHERE id = ?", form.name, period_days, id)
        .execute(&mut *tx).await?;

    // Replace all criteria+options atomically; CASCADE handles criterion_options.
    sqlx::query!("DELETE FROM criteria WHERE scorecard_id = ?", id)
        .execute(&mut *tx).await?;

    for (crit_pos, crit) in criteria.iter().enumerate() {
        let criterion_id = sqlx::query!(
            "INSERT INTO criteria (scorecard_id, name, position) VALUES (?, ?, ?)",
            id, crit.name, crit_pos as u32
        ).execute(&mut *tx).await?.last_insert_id();

        for (opt_pos, opt) in crit.options.iter().enumerate() {
            sqlx::query!(
                "INSERT INTO criterion_options (criterion_id, name, position, score_type, points)
                 VALUES (?, ?, ?, ?, ?)",
                criterion_id, opt.name, opt_pos as u32, opt.score_type, opt.points
            ).execute(&mut *tx).await?;
        }
    }

    // Grades are replaced wholesale too; nothing references grade ids.
    sqlx::query!("DELETE FROM grades WHERE scorecard_id = ?", id)
        .execute(&mut *tx).await?;
    for g in &grades {
        sqlx::query!(
            "INSERT INTO grades (scorecard_id, name, min_score, bg_color, text_color) VALUES (?, ?, ?, ?, ?)",
            id, g.name, g.min_score, g.bg_color, g.text_color
        ).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(axum::response::Redirect::to(&format!("/scorecards/{id}")))
}

// ── Delete ────────────────────────────────────────────────────────────────────

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<u64>,
) -> Result<impl IntoResponse> {
    db::delete_scorecard(&state.db, id).await?;
    Ok(axum::response::Redirect::to("/scorecards"))
}

// ── Grades ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GradeForm {
    pub name:       String,
    /// Kept as a string so a blank input gives a clear error rather than a
    /// form-rejection.
    pub min_score:  String,
    pub bg_color:   String,
    pub text_color: String,
}

/// A grade form after validation.
struct GradeInput {
    name:       String,
    min_score:  f64,
    bg_color:   String,
    text_color: String,
}

/// POST /scorecards/{id}/grades
pub async fn create_grade(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Form(form): Form<GradeForm>,
) -> Result<impl IntoResponse> {
    db::get_scorecard(&state.db, id).await?;
    let existing = db::list_grades(&state.db, id).await?;
    let g = validate_grade(&form, &existing, None)?;
    db::insert_grade(&state.db, id, &g.name, g.min_score, &g.bg_color, &g.text_color).await?;
    Ok(axum::response::Redirect::to(&format!("/scorecards/{id}#grades")))
}

/// POST /scorecards/{id}/grades/{grade_id}
pub async fn update_grade(
    State(state): State<AppState>,
    Path((id, grade_id)): Path<(u64, u64)>,
    Form(form): Form<GradeForm>,
) -> Result<impl IntoResponse> {
    let existing = db::list_grades(&state.db, id).await?;
    if !existing.iter().any(|g| g.id == grade_id) {
        return Err(AppError::NotFound);
    }
    let g = validate_grade(&form, &existing, Some(grade_id))?;
    db::update_grade(&state.db, id, grade_id, &g.name, g.min_score, &g.bg_color, &g.text_color).await?;
    Ok(axum::response::Redirect::to(&format!("/scorecards/{id}#grades")))
}

/// POST /scorecards/{id}/grades/{grade_id}/delete
pub async fn delete_grade(
    State(state): State<AppState>,
    Path((id, grade_id)): Path<(u64, u64)>,
) -> Result<impl IntoResponse> {
    db::delete_grade(&state.db, id, grade_id).await?;
    Ok(axum::response::Redirect::to(&format!("/scorecards/{id}#grades")))
}

/// Validates a grade form against the scorecard's other grades. `editing` is
/// the id of the grade being updated, which is excluded from clash checks.
fn validate_grade(form: &GradeForm, existing: &[GradeRow], editing: Option<u64>) -> Result<GradeInput> {
    let min_score: f64 = form.min_score.trim().parse()
        .map_err(|_| AppError::BadRequest("Minimum score must be a number".into()))?;
    let grade = validate_grade_fields(&form.name, min_score, &form.bg_color, &form.text_color)?;
    let others: Vec<(&str, f64)> = existing.iter()
        .filter(|g| Some(g.id) != editing)
        .map(|g| (g.name.as_str(), g.min_score))
        .collect();
    check_grade_clash(&grade, &others)?;
    Ok(grade)
}

fn validate_grade_fields(name: &str, min_score: f64, bg_color: &str, text_color: &str) -> Result<GradeInput> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Grade name cannot be empty".into()));
    }
    if !(0.0..=100.0).contains(&min_score) {
        return Err(AppError::BadRequest("Minimum score must be between 0 and 100".into()));
    }
    Ok(GradeInput {
        name,
        min_score,
        bg_color:   parse_hex_color(bg_color, "Background colour")?,
        text_color: parse_hex_color(text_color, "Text colour")?,
    })
}

/// Grade names and minimum scores must both be unique within a scorecard.
fn check_grade_clash(grade: &GradeInput, others: &[(&str, f64)]) -> Result<()> {
    for (name, min_score) in others {
        if name.eq_ignore_ascii_case(&grade.name) {
            return Err(AppError::BadRequest(format!("A grade named '{}' already exists", grade.name)));
        }
        if *min_score == grade.min_score {
            return Err(AppError::BadRequest(format!(
                "Grade '{name}' already starts at {}%", grade.min_score
            )));
        }
    }
    Ok(())
}

/// Parses and validates the edit page's `grades_json` (a JSON array of
/// `{name, min_score, bg_color, text_color}`).
fn parse_and_validate_grades(grades_json: Option<&str>) -> Result<Vec<GradeInput>> {
    #[derive(Deserialize)]
    struct GradeJson {
        name:       String,
        min_score:  f64,
        bg_color:   String,
        text_color: String,
    }

    let raw: Vec<GradeJson> = match grades_json.map(str::trim) {
        None | Some("") => Vec::new(),
        Some(json) => serde_json::from_str(json)
            .map_err(|e| AppError::BadRequest(format!("Invalid grades JSON: {e}")))?,
    };

    let mut grades: Vec<GradeInput> = Vec::new();
    for g in &raw {
        let grade = validate_grade_fields(&g.name, g.min_score, &g.bg_color, &g.text_color)?;
        let others: Vec<(&str, f64)> = grades.iter().map(|o| (o.name.as_str(), o.min_score)).collect();
        check_grade_clash(&grade, &others)?;
        grades.push(grade);
    }
    Ok(grades)
}

/// Accepts `#rrggbb` (as sent by `<input type="color">`) and normalises to lowercase.
fn parse_hex_color(value: &str, field: &str) -> Result<String> {
    let v = value.trim();
    let valid = v.len() == 7
        && v.starts_with('#')
        && v[1..].chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(AppError::BadRequest(format!("{field} must be a hex colour like #16a34a")));
    }
    Ok(v.to_ascii_lowercase())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_and_validate_criteria(criteria_json: &str) -> Result<Vec<CriterionInput>> {
    let criteria: Vec<CriterionInput> = serde_json::from_str(criteria_json)
        .map_err(|e| AppError::BadRequest(format!("Invalid criteria JSON: {e}")))?;

    if criteria.is_empty() {
        return Err(AppError::BadRequest("Scorecard must have at least one criterion".into()));
    }

    for crit in &criteria {
        if crit.name.trim().is_empty() {
            return Err(AppError::BadRequest("Criterion name cannot be empty".into()));
        }
        for opt in &crit.options {
            match opt.score_type.as_str() {
                "points" => {
                    if opt.points.is_none() {
                        return Err(AppError::BadRequest(format!(
                            "Option '{}' has score_type 'points' but no points value", opt.name
                        )));
                    }
                }
                "na" | "autofail" => {}
                other => return Err(AppError::BadRequest(format!(
                    "Unknown score_type '{other}'; expected 'points', 'na', or 'autofail'"
                ))),
            }
        }
    }

    Ok(criteria)
}

async fn insert_scorecard_with_criteria(
    state: &AppState,
    name: &str,
    default_period_days: Option<u32>,
    criteria: &[CriterionInput],
    grades: &[GradeInput],
) -> Result<u64> {
    let mut tx = state.db.begin().await?;

    let scorecard_id = sqlx::query!(
        "INSERT INTO scorecards (name, default_period_days) VALUES (?, ?)", name, default_period_days
    ).execute(&mut *tx).await?.last_insert_id();

    for (crit_pos, crit) in criteria.iter().enumerate() {
        let criterion_id = sqlx::query!(
            "INSERT INTO criteria (scorecard_id, name, position) VALUES (?, ?, ?)",
            scorecard_id, crit.name, crit_pos as u32
        ).execute(&mut *tx).await?.last_insert_id();

        for (opt_pos, opt) in crit.options.iter().enumerate() {
            sqlx::query!(
                "INSERT INTO criterion_options (criterion_id, name, position, score_type, points)
                 VALUES (?, ?, ?, ?, ?)",
                criterion_id, opt.name, opt_pos as u32, opt.score_type, opt.points
            ).execute(&mut *tx).await?;
        }
    }

    for g in grades {
        sqlx::query!(
            "INSERT INTO grades (scorecard_id, name, min_score, bg_color, text_color) VALUES (?, ?, ?, ?, ?)",
            scorecard_id, g.name, g.min_score, g.bg_color, g.text_color
        ).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(scorecard_id)
}