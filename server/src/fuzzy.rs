use crate::state::AppState;
use strsim::jaro_winkler;

pub fn top_matches(
    state: &AppState,
    query: &str,
    limit: usize,
    threshold: f64,
) -> Vec<(String, f64)> {
    let mut seen = std::collections::HashSet::new();

    let mut candidates: Vec<(String, f64)> = state
        .models
        .iter()
        .filter_map(|m| {
            m["id"]
                .as_str()
                .map(|id| (id.to_string(), jaro_winkler(query, id)))
        })
        .chain(
            state
                .aliases
                .keys()
                .map(|alias| (alias.clone(), jaro_winkler(query, alias.as_str()))),
        )
        .filter(|(_, score)| *score >= threshold)
        .filter(|(id, _)| seen.insert(id.clone()))
        .collect();

    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(limit);
    candidates
}
