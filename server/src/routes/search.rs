use crate::{
    catalog::Model,
    error::ApiError,
    fuzzy,
    request::RequestContext,
    response::{ApiResponse, catalog_headers},
    state::AppState,
};
use axum::{
    Extension,
    extract::{Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, sync::Arc};
use tokio::sync::RwLock;

use super::models::{ModelFilter, model_matches_filter, sort_models};

const DEFAULT_LIMIT: usize = 20;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SearchQuery {
    /// Fuzzy search query matched against model id, display name, and aliases
    #[param(example = "gpt")]
    q: Option<String>,
    /// Filter by provider id (comma-separated for multiple)
    #[param(example = "openai")]
    provider: Option<String>,
    /// Filter by lifecycle status: active, deprecating, deprecated, retired
    #[param(example = "active")]
    status: Option<String>,
    /// Filter by capability key (comma-separated, e.g. vision,tools)
    #[param(example = "vision")]
    capability: Option<String>,
    /// Maximum number of results to return
    #[param(example = 5)]
    limit: Option<usize>,
    /// Number of results to skip for pagination
    #[param(example = 0)]
    offset: Option<usize>,
    /// Sort field when q is absent: provider, id, status, context_window, max_output_tokens
    #[param(example = "context_window")]
    sort: Option<String>,
    /// Sort direction: asc or desc
    #[param(example = "desc")]
    order: Option<String>,
    /// Filter by required input modality (e.g. image, audio)
    #[param(example = "image")]
    modality_input: Option<String>,
    /// Filter by required output modality
    #[param(example = "text")]
    modality_output: Option<String>,
    /// Minimum context window size in tokens
    #[param(example = 100000)]
    min_context: Option<u64>,
    /// Maximum input price per million tokens in USD
    #[param(example = 1.0)]
    max_input_price: Option<f64>,
}

impl SearchQuery {
    fn q(&self) -> Option<&str> {
        non_empty(self.q.as_deref())
    }

    fn filter(&self) -> ModelFilter {
        ModelFilter {
            provider: self.provider.clone(),
            status: self.status.clone(),
            capability: self.capability.clone(),
            limit: self.limit,
            offset: self.offset,
            sort: self.sort.clone(),
            order: self.order.clone(),
            modality_input: self.modality_input.clone(),
            modality_output: self.modality_output.clone(),
            min_context: self.min_context,
            max_input_price: self.max_input_price,
            endpoint_family: None,
            query: None,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SearchResult {
    model: Model,
    score: Option<f64>,
    matched: Option<String>,
    match_type: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/search",
    tag = "Discovery",
    operation_id = "search_models",
    summary = "Search models",
    description = "Fuzzy + filter search returning ranked results with match metadata.",
    params(SearchQuery),
    responses(
        (
            status = 200,
            description = "Search model catalog",
            body = ApiResponse<Vec<SearchResult>>,
            examples(
                ("alias" = (
                    summary = "Alias search",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": [
                            {
                                "model": {
                                    "id": "claude-opus-4-7",
                                    "provider": "anthropic",
                                    "display_name": "Claude Opus 4.7",
                                    "status": "active",
                                    "context_window": 200000,
                                    "max_output_tokens": 128000,
                                    "modalities": { "input": ["text", "image"], "output": ["text"] },
                                    "capabilities": { "tools": true, "vision": true },
                                    "parameters": { "supported": ["temperature"], "rejected": [], "deprecated_for_this_model": [] },
                                    "pricing": { "currency": "USD", "input": 5.0, "output": 25.0 },
                                    "last_verified": "2026-05-02",
                                    "confidence": "official",
                                    "endpoint_family": "custom",
                                    "sources": []
                                },
                                "score": 1.0,
                                "matched": "claude-opus-4",
                                "match_type": "alias"
                            }
                        ],
                        "meta": {
                            "limit": 20,
                            "offset": 0,
                            "total_data": 1,
                            "timestamp": "2026-05-03T00:00:00Z"
                        },
                        "error": null
                    })
                ))
            )
        ),
        (
            status = 400,
            description = "Invalid search query",
            body = ApiResponse<crate::response::EmptyData>,
            examples(
                ("error" = (
                    summary = "Bad request",
                    value = json!({
                        "success": false,
                        "message": "Failed to deserialize query string: invalid digit found in string",
                        "data": null,
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": { "code": "BAD_REQUEST" }
                    })
                ))
            )
        )
    )
)]
pub async fn search(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    query: Result<Query<SearchQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let Query(query) = query.map_err(|error| ApiError::BadRequest {
        message: error.body_text(),
    })?;
    let q = query.q().map(str::to_string);
    let filter = query.filter();
    let state = state.read().await;

    if let Some(inm) = headers.get("if-none-match")
        && inm.as_bytes() == state.etag.as_bytes()
    {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }
    let mut results = Vec::new();

    for model in state
        .models
        .iter()
        .filter(|model| model_matches_filter(model, &filter))
    {
        if let Some(q) = q.as_deref() {
            let Some(candidate) = fuzzy::best_model_suggestion(&state, model, q, 0.7) else {
                continue;
            };
            results.push(SearchResult {
                model: model.clone(),
                score: Some((candidate.score * 1000.0).round() / 1000.0),
                matched: Some(candidate.matched),
                match_type: Some(match_type_name(candidate.match_type).to_string()),
            });
        } else {
            results.push(SearchResult {
                model: model.clone(),
                score: None,
                matched: None,
                match_type: None,
            });
        }
    }

    if q.is_some() {
        results.sort_by(|left, right| {
            right
                .score
                .partial_cmp(&left.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.model.provider.cmp(&right.model.provider))
                .then_with(|| left.model.id.cmp(&right.model.id))
        });
    } else {
        let mut models: Vec<_> = results.into_iter().map(|result| result.model).collect();
        sort_models(&mut models, filter.sort(), filter.order());
        results = models
            .into_iter()
            .map(|model| SearchResult {
                model,
                score: None,
                matched: None,
                match_type: None,
            })
            .collect();
    }

    let total = results.len();
    let offset = filter.offset().unwrap_or(0).min(total);
    let limit = filter.limit().unwrap_or(DEFAULT_LIMIT);
    let data: Vec<_> = results.into_iter().skip(offset).take(limit).collect();

    Ok((
        catalog_headers(&state.etag),
        Json(ApiResponse::paginated_with_context(
            data, limit, offset, total, &context,
        )),
    )
        .into_response())
}

fn match_type_name(match_type: fuzzy::MatchType) -> &'static str {
    match match_type {
        fuzzy::MatchType::Id => "id",
        fuzzy::MatchType::Alias => "alias",
        fuzzy::MatchType::DisplayName => "display_name",
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let value = value.trim();
        if value.is_empty() { None } else { Some(value) }
    })
}
