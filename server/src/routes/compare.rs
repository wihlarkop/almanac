use crate::{
    catalog::{Model, Pricing},
    error::ApiError,
    request::RequestContext,
    response::ApiResponse,
    state::AppState,
};
use axum::{
    Extension,
    extract::{Query, State, rejection::QueryRejection},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct CompareQuery {
    models: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CompareResponse {
    models: Vec<Model>,
    summary: CompareSummary,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CompareSummary {
    max_context_window: u64,
    max_output_tokens: u64,
    lowest_input_price: Option<Pricing>,
}

#[utoipa::path(
    get,
    path = "/api/v1/compare",
    params(CompareQuery),
    responses(
        (
            status = 200,
            description = "Model comparison",
            body = ApiResponse<CompareResponse>,
            examples(
                ("comparison" = (
                    summary = "Compare two models",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": {
                            "models": [
                                {
                                    "id": "gpt-4o",
                                    "provider": "openai",
                                    "display_name": "GPT-4o",
                                    "status": "active",
                                    "context_window": 128000,
                                    "max_output_tokens": 16384,
                                    "modalities": { "input": ["text", "image"], "output": ["text"] },
                                    "capabilities": { "tools": true, "vision": true },
                                    "parameters": { "supported": ["temperature"], "rejected": [], "deprecated_for_this_model": [] },
                                    "pricing": { "currency": "USD", "input": 2.5, "output": 10.0 },
                                    "last_verified": "2026-05-02",
                                    "confidence": "official",
                                    "endpoint_family": "responses",
                                    "sources": []
                                }
                            ],
                            "summary": {
                                "max_context_window": 128000,
                                "max_output_tokens": 16384,
                                "lowest_input_price": { "currency": "USD", "input": 2.5, "output": 10.0 }
                            }
                        },
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 400, description = "Invalid compare query", body = ApiResponse<crate::response::EmptyData>),
        (status = 404, description = "Model not found", body = ApiResponse<crate::response::EmptyData>)
    )
)]
pub async fn compare(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    query: Result<Query<CompareQuery>, QueryRejection>,
) -> Result<Json<ApiResponse<CompareResponse>>, ApiError> {
    let Query(query) = query.map_err(|error| ApiError::BadRequest {
        message: error.body_text(),
    })?;

    let refs = parse_model_refs(&query.models)?;
    if refs.len() < 2 {
        return Err(ApiError::BadRequest {
            message: "compare requires at least two unique models".to_string(),
        });
    }
    if refs.len() > 5 {
        return Err(ApiError::BadRequest {
            message: "compare supports at most five unique models".to_string(),
        });
    }

    let state = state.read().await;
    let mut models = Vec::with_capacity(refs.len());
    for (provider, id) in refs {
        let model = state
            .models_by_provider_id
            .get(&(provider.clone(), id.clone()))
            .and_then(|index| state.models.get(*index))
            .cloned()
            .ok_or(ApiError::ModelNotFound { provider, id })?;
        models.push(model);
    }

    let summary = CompareSummary {
        max_context_window: models
            .iter()
            .map(|model| model.context_window)
            .max()
            .unwrap_or(0),
        max_output_tokens: models
            .iter()
            .map(|model| model.max_output_tokens)
            .max()
            .unwrap_or(0),
        lowest_input_price: models
            .iter()
            .filter_map(|model| model.pricing.clone())
            .min_by(|left, right| left.input.total_cmp(&right.input)),
    };

    Ok(Json(ApiResponse::ok_with_context(
        CompareResponse { models, summary },
        &context,
    )))
}

fn parse_model_refs(raw: &str) -> Result<Vec<(String, String)>, ApiError> {
    let mut seen = HashSet::new();
    let mut refs = Vec::new();

    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        let Some((provider, id)) = part.split_once('/') else {
            return Err(ApiError::BadRequest {
                message: "models must use provider/id references".to_string(),
            });
        };
        let provider = provider.trim();
        let id = id.trim();
        if provider.is_empty() || id.is_empty() {
            return Err(ApiError::BadRequest {
                message: "models must use provider/id references".to_string(),
            });
        }

        let key = (provider.to_string(), id.to_string());
        if seen.insert(key.clone()) {
            refs.push(key);
        }
    }

    Ok(refs)
}
