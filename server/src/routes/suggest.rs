use crate::{fuzzy, state::AppState};
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SuggestQuery {
    pub q: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SuggestResult {
    pub id: String,
    pub provider: String,
    pub score: f64,
}

#[utoipa::path(
    get,
    path = "/v1/suggest",
    params(SuggestQuery),
    responses((status = 200, description = "Ranked suggestions", body = [SuggestResult]))
)]
pub async fn suggest(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<SuggestQuery>,
) -> Json<Vec<SuggestResult>> {
    let state = state.read().await;

    let results: Vec<SuggestResult> = fuzzy::top_matches(&state, &params.q, 5, 0.7)
        .into_iter()
        .filter_map(|(id, score)| {
            let canonical = state
                .aliases
                .get(&id)
                .cloned()
                .unwrap_or_else(|| id.clone());
            let model = state
                .models
                .iter()
                .find(|m| m["id"].as_str() == Some(canonical.as_str()))?;
            let provider = model["provider"].as_str().unwrap_or("").to_string();
            Some(SuggestResult {
                id: canonical,
                provider,
                score: (score * 1000.0).round() / 1000.0,
            })
        })
        .collect();

    Json(results)
}
