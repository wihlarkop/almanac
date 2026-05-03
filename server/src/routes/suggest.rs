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
    responses(
        (status = 200, description = "Ranked suggestions", body = ApiResponse<Vec<SuggestResult>>),
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

    let results: Vec<SuggestResult> = fuzzy::top_matches(&state, &params.q, 5, 0.7)
        .into_iter()
        .filter_map(|(id, score)| {
            let canonical = state
                .aliases
                .get(&id)
                .cloned()
                .unwrap_or_else(|| id.clone());
            let model = state.models.iter().find(|model| model.id == canonical)?;
            Some(SuggestResult {
                id: canonical,
                provider: model.provider.clone(),
                score: (score * 1000.0).round() / 1000.0,
            })
        })
        .collect();

    Ok(Json(ApiResponse::ok_with_context(results, &context)))
}
