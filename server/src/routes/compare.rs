use crate::{
    catalog::Model,
    error::ApiError,
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
use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct CompareQuery {
    /// Comma-separated provider/id pairs (2–5 models, e.g. openai/gpt-4o,anthropic/claude-opus-4-7)
    #[param(example = "openai/gpt-4o,anthropic/claude-opus-4-7")]
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
    cheapest_input: Option<CheapestModel>,
    cheapest_output: Option<CheapestOutputModel>,
    pricing_breakdown: Vec<PricingBreakdownEntry>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CheapestModel {
    model_id: String,
    provider: String,
    input_price: f64,
    currency: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CheapestOutputModel {
    model_id: String,
    provider: String,
    output_price: f64,
    currency: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PricingBreakdownEntry {
    model_id: String,
    provider: String,
    currency: Option<String>,
    input: Option<f64>,
    output: Option<f64>,
    cached_input: Option<f64>,
    batch_input: Option<f64>,
    batch_output: Option<f64>,
    request_fee: Option<f64>,
    search_fee: Option<f64>,
    reasoning: Option<f64>,
    per_image: Option<f64>,
    per_second: Option<f64>,
    per_minute: Option<f64>,
    per_million_chars: Option<f64>,
    per_page: Option<f64>,
    comparable_cost: Option<f64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/compare",
    tag = "Catalog",
    operation_id = "compare_models",
    summary = "Compare models",
    description = "Side-by-side comparison of 2–5 models with pricing breakdown.",
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
                                "cheapest_input": { "model_id": "gpt-4o", "provider": "openai", "input_price": 2.5, "currency": "USD" },
                                "cheapest_output": { "model_id": "gpt-4o", "provider": "openai", "output_price": 10.0, "currency": "USD" },
                                "pricing_breakdown": [
                                    {
                                        "model_id": "gpt-4o",
                                        "provider": "openai",
                                        "currency": "USD",
                                        "input": 2.5,
                                        "output": 10.0,
                                        "cached_input": null,
                                        "batch_input": null,
                                        "batch_output": null,
                                        "request_fee": null,
                                        "search_fee": null,
                                        "reasoning": null,
                                        "per_image": null,
                                        "per_second": null,
                                        "per_minute": null,
                                        "per_million_chars": null,
                                        "per_page": null,
                                        "comparable_cost": 2.5
                                    }
                                ]
                            }
                        },
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (
            status = 400,
            description = "Invalid compare query",
            body = ApiResponse<crate::response::EmptyData>,
            examples(
                ("error" = (
                    summary = "Bad request",
                    value = json!({
                        "success": false,
                        "message": "models must use provider/id references",
                        "data": null,
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": { "code": "BAD_REQUEST" }
                    })
                ))
            )
        ),
        (
            status = 404,
            description = "Model not found",
            body = ApiResponse<crate::response::EmptyData>,
            examples(
                ("error" = (
                    summary = "Model not found",
                    value = json!({
                        "success": false,
                        "message": "model not found",
                        "data": null,
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": { "code": "MODEL_NOT_FOUND", "details": { "provider": "openai", "id": "gpt-99" } }
                    })
                ))
            )
        )
    )
)]
pub async fn compare(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    query: Result<Query<CompareQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
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

    if let Some(inm) = headers.get("if-none-match")
        && inm.as_bytes() == state.etag.as_bytes()
    {
        return Ok(StatusCode::NOT_MODIFIED.into_response());
    }

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
        cheapest_input: models
            .iter()
            .filter_map(|model| model.pricing.as_ref().map(|pricing| (model, pricing)))
            .min_by(|(_, left), (_, right)| left.input.total_cmp(&right.input))
            .map(|(model, pricing)| CheapestModel {
                model_id: model.id.clone(),
                provider: model.provider.clone(),
                input_price: pricing.input,
                currency: pricing.currency.clone(),
            }),
        cheapest_output: models
            .iter()
            .filter_map(|model| model.pricing.as_ref().map(|pricing| (model, pricing)))
            .min_by(|(_, left), (_, right)| left.output.total_cmp(&right.output))
            .map(|(model, pricing)| CheapestOutputModel {
                model_id: model.id.clone(),
                provider: model.provider.clone(),
                output_price: pricing.output,
                currency: pricing.currency.clone(),
            }),
        pricing_breakdown: build_pricing_breakdown(&models),
    };

    Ok((
        catalog_headers(&state.etag),
        Json(ApiResponse::ok_with_context(
            CompareResponse { models, summary },
            &context,
        )),
    )
        .into_response())
}

fn comparable_cost(pricing: &crate::catalog::Pricing) -> Option<f64> {
    if pricing.input > 0.0 {
        return Some(pricing.input);
    }
    if let Some(pmc) = pricing.per_million_chars {
        return Some(pmc * 5.0);
    }
    if let Some(pm) = pricing.per_minute {
        return Some(pm * 833.0);
    }
    None
}

fn build_pricing_breakdown(models: &[crate::catalog::Model]) -> Vec<PricingBreakdownEntry> {
    models
        .iter()
        .map(|model| match &model.pricing {
            None => PricingBreakdownEntry {
                model_id: model.id.clone(),
                provider: model.provider.clone(),
                currency: None,
                input: None,
                output: None,
                cached_input: None,
                batch_input: None,
                batch_output: None,
                request_fee: None,
                search_fee: None,
                reasoning: None,
                per_image: None,
                per_second: None,
                per_minute: None,
                per_million_chars: None,
                per_page: None,
                comparable_cost: None,
            },
            Some(p) => PricingBreakdownEntry {
                model_id: model.id.clone(),
                provider: model.provider.clone(),
                currency: Some(p.currency.clone()),
                input: Some(p.input),
                output: Some(p.output),
                cached_input: p.cached_input,
                batch_input: p.batch_input,
                batch_output: p.batch_output,
                request_fee: p.request_fee,
                search_fee: p.search_fee,
                reasoning: p.reasoning,
                per_image: p.per_image,
                per_second: p.per_second,
                per_minute: p.per_minute,
                per_million_chars: p.per_million_chars,
                per_page: p.per_page,
                comparable_cost: comparable_cost(p),
            },
        })
        .collect()
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
