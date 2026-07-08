use std::collections::{HashMap, HashSet};
use qams_core::{Criterion, CriterionScore, Scorecard};

/// Parses a scorecard from a CSV string.
pub fn parse_scorecard_csv(csv: &str) -> Result<Scorecard, String> {
    let mut lines = csv.lines();

    let header_line = lines.next().ok_or("CSV is empty")?;
    let header_cells: Vec<&str> = header_line.split(',').collect();
    let option_names: Vec<&str> = header_cells[1..].to_vec();

    {
        let mut seen = HashSet::new();
        for name in &option_names {
            if !seen.insert(*name) {
                return Err(format!("Duplicate option name: '{name}'"));
            }
        }
    }

    let mut criteria: HashMap<String, Criterion> = HashMap::new();
    let mut criterion_order: Vec<String> = Vec::new();

    for (row_idx, line) in lines.enumerate() {
        if line.trim().is_empty() { continue; }
        let cells: Vec<&str> = line.split(',').collect();
        let crit_name = cells[0];
        if crit_name.is_empty() {
            return Err(format!("Row {} has an empty criterion name", row_idx + 2));
        }
        if criteria.contains_key(crit_name) {
            return Err(format!("Duplicate criterion name: '{crit_name}'"));
        }

        let mut options: HashMap<String, CriterionScore> = HashMap::new();
        for (col_idx, opt_name) in option_names.iter().enumerate() {
            let cell = cells.get(col_idx + 1).copied().unwrap_or("").trim();
            let score = match cell {
                "" => continue,
                "N" => CriterionScore::NotApplicable,
                "F" => CriterionScore::Autofail,
                other => {
                    let points: u32 = other.parse().map_err(|_| {
                        format!("Invalid cell value '{other}' at criterion '{crit_name}', option '{opt_name}'")
                    })?;
                    CriterionScore::Points(points)
                }
            };
            options.insert(opt_name.to_string(), score);
        }

        criterion_order.push(crit_name.to_string());
        criteria.insert(crit_name.to_string(), Criterion::new(options));
    }

    Ok(Scorecard::new(
        criteria,
        option_names.iter().map(|s| s.to_string()).collect(),
        criterion_order,
    ))
}