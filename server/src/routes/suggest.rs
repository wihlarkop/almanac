use crate::{
    error::ApiError, fuzzy, request::RequestContext, response::ApiResponse, state::AppState,
};
use axum::{
    Extension,
    extract::{Query, State, rejection::QueryRejection},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SuggestQuery {
    /// Fuzzy search query matched against model id, display name, and aliases
    #[param(example = "claude-opus-4")]
    pub q: String,
    /// Restrict suggestions to a specific provider
    #[param(example = "anthropic")]
    pub provider: Option<String>,
    /// Maximum number of suggestions to return (default 5, max 20)
    #[param(example = 5)]
    pub limit: Option<usize>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SuggestResult {
    pub id: String,
    pub provider: String,
    pub score: f64,
    pub matched: String,
    pub match_type: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/suggest",
    tag = "Discovery",
    operation_id = "suggest_models",
    summary = "Suggest models",
    description = "Returns ranked fuzzy suggestions for a query string.",
    params(SuggestQuery),
    responses(
        (
            status = 200,
            description = "Ranked suggestions",
            body = ApiResponse<Vec<SuggestResult>>,
            examples(
                ("alias" = (
                    summary = "Alias match",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": [
                            {
                                "id": "claude-opus-4-7",
                                "provider": "anthropic",
                                "score": 1.0,
                                "matched": "claude-opus-4",
                                "match_type": "alias"
                            }
                        ],
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 400, description = "Invalid query parameters", body = ApiResponse<crate::response::EmptyData>)
    )
)]
pub async fn suggest(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    query: Result<Query<SuggestQuery>, QueryRejection>,
) -> Result<Json<ApiResponse<Vec<SuggestResult>>>, ApiError> {
    let Query(params) = query.map_err(|error| ApiError::BadRequest {
        message: error.body_text(),
    })?;
    let state = state.read().await;

    let provider = params
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|provider| !provider.is_empty());
    let limit = params.limit.filter(|limit| *limit > 0).unwrap_or(5).min(20);

    // Fetch all candidates first to report total_data, then truncate to limit
    let all_candidates: Vec<SuggestResult> =
        fuzzy::top_suggestions(&state, &params.q, provider, usize::MAX, 0.7)
            .into_iter()
            .filter_map(|candidate| {
                let model = state
                    .models_by_id
                    .get(&candidate.canonical_id)
                    .and_then(|index| state.models.get(*index))?;
                Some(SuggestResult {
                    id: candidate.canonical_id,
                    provider: model.provider.clone(),
                    score: (candidate.score * 1000.0).round() / 1000.0,
                    matched: candidate.matched,
                    match_type: match_type_name(candidate.match_type).to_string(),
                })
            })
            .collect();

    let total = all_candidates.len();
    let results: Vec<SuggestResult> = all_candidates.into_iter().take(limit).collect();

    Ok(Json(ApiResponse::paginated_with_context(
        results, limit, 0, total, &context,
    )))
}

fn match_type_name(match_type: fuzzy::MatchType) -> &'static str {
    match match_type {
        fuzzy::MatchType::Id => "id",
        fuzzy::MatchType::Alias => "alias",
        fuzzy::MatchType::DisplayName => "display_name",
    }
}
