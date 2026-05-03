use crate::{catalog::Model, state::AppState};
use std::{cmp::Ordering, collections::HashMap};
use strsim::jaro_winkler;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchType {
    Id,
    Alias,
    DisplayName,
}

#[derive(Clone, Debug)]
pub struct SuggestionMatch {
    pub canonical_id: String,
    pub matched: String,
    pub match_type: MatchType,
    pub score: f64,
}

#[derive(Clone)]
struct Candidate {
    suggestion: SuggestionMatch,
    rank: u8,
}

pub fn top_suggestions(
    state: &AppState,
    query: &str,
    provider: Option<&str>,
    limit: usize,
    threshold: f64,
) -> Vec<SuggestionMatch> {
    let query = query.trim();
    if query.is_empty() || limit == 0 {
        return Vec::new();
    }

    let query_normalized = query.to_ascii_lowercase();
    let mut best_by_id: HashMap<String, Candidate> = HashMap::new();

    for model in &state.models {
        if provider.is_some_and(|provider| model.provider != provider) {
            continue;
        }
        add_candidate(
            &mut best_by_id,
            model,
            model.id.as_str(),
            MatchType::Id,
            query,
            &query_normalized,
            threshold,
        );
        add_candidate(
            &mut best_by_id,
            model,
            model.display_name.as_str(),
            MatchType::DisplayName,
            query,
            &query_normalized,
            threshold,
        );
    }

    for (alias, canonical_id) in &state.aliases {
        let Some(model) = state
            .models_by_id
            .get(canonical_id)
            .and_then(|index| state.models.get(*index))
        else {
            continue;
        };
        if provider.is_some_and(|provider| model.provider != provider) {
            continue;
        }
        add_candidate(
            &mut best_by_id,
            model,
            alias.as_str(),
            MatchType::Alias,
            query,
            &query_normalized,
            threshold,
        );
    }

    let mut candidates: Vec<_> = best_by_id.into_values().collect();
    candidates.sort_by(compare_candidates);
    candidates.truncate(limit);
    candidates
        .into_iter()
        .map(|candidate| candidate.suggestion)
        .collect()
}

pub fn top_matches(
    state: &AppState,
    query: &str,
    limit: usize,
    threshold: f64,
) -> Vec<(String, f64)> {
    top_suggestions(state, query, None, limit, threshold)
        .into_iter()
        .map(|candidate| (candidate.canonical_id, candidate.score))
        .collect()
}

fn add_candidate(
    best_by_id: &mut HashMap<String, Candidate>,
    model: &Model,
    matched: &str,
    match_type: MatchType,
    query: &str,
    query_normalized: &str,
    threshold: f64,
) {
    let exact = matched.eq_ignore_ascii_case(query);
    let score = if exact {
        1.0
    } else {
        jaro_winkler(query_normalized, &matched.to_ascii_lowercase())
    };

    if score < threshold {
        return;
    }

    let candidate = Candidate {
        suggestion: SuggestionMatch {
            canonical_id: model.id.clone(),
            matched: matched.to_string(),
            match_type,
            score,
        },
        rank: rank(match_type, exact),
    };

    match best_by_id.get(model.id.as_str()) {
        Some(existing) if compare_candidates(&candidate, existing) != Ordering::Less => {}
        _ => {
            best_by_id.insert(model.id.clone(), candidate);
        }
    }
}

fn rank(match_type: MatchType, exact: bool) -> u8 {
    match (exact, match_type) {
        (true, MatchType::Id) => 0,
        (true, MatchType::Alias) => 1,
        (true, MatchType::DisplayName) => 2,
        (false, _) => 3,
    }
}

fn compare_candidates(left: &Candidate, right: &Candidate) -> Ordering {
    left.rank
        .cmp(&right.rank)
        .then_with(|| {
            right
                .suggestion
                .score
                .partial_cmp(&left.suggestion.score)
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| {
            left.suggestion
                .canonical_id
                .cmp(&right.suggestion.canonical_id)
        })
        .then_with(|| left.suggestion.matched.cmp(&right.suggestion.matched))
}
