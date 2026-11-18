use crate::{
    catalog::Model, error::ApiError, request::RequestContext, response::ApiResponse,
    state::AppState,
};
use axum::{
    Extension,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_LIMIT: usize = 20;
pub(crate) const MAX_LIMIT: usize = 100;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ModelFilter {
    /// Filter by provider id (comma-separated for multiple)
    #[param(example = "openai")]
    pub(crate) provider: Option<String>,
    /// Filter by lifecycle status: active, deprecating, deprecated, retired, unknown
    #[param(example = "active")]
    pub(crate) status: Option<String>,
    /// Filter by capability key (comma-separated, e.g. vision,tools)
    #[param(example = "vision")]
    pub(crate) capability: Option<String>,
    /// Maximum number of results to return
    #[param(example = 20)]
    pub(crate) limit: Option<usize>,
    /// Number of results to skip for pagination
    #[param(example = 0)]
    pub(crate) offset: Option<usize>,
    /// Sort field: provider, id, status, context_window, max_output_tokens, input_price, output_price
    #[param(example = "context_window")]
    pub(crate) sort: Option<String>,
    /// Sort direction: asc or desc
    #[param(example = "desc")]
    pub(crate) order: Option<String>,
    /// Filter by required input modality (e.g. image, audio)
    #[param(example = "image")]
    pub(crate) modality_input: Option<String>,
    /// Filter by required output modality
    #[param(example = "text")]
    pub(crate) modality_output: Option<String>,
    /// Minimum context window size in tokens
    #[param(example = 100000)]
    pub(crate) min_context: Option<u64>,
    /// Maximum input price per million tokens in USD
    #[param(example = 1.0)]
    pub(crate) max_input_price: Option<f64>,
    /// Maximum output price per million tokens in USD
    #[param(example = 5.0)]
    pub(crate) max_output_price: Option<f64>,
    /// Filter by pricing availability: available, unavailable, free, paid
    #[param(example = "paid")]
    pub(crate) pricing: Option<String>,
    /// Filter by endpoint family (e.g. chat_completions, responses)
    #[param(example = "chat_completions")]
    pub(crate) endpoint_family: Option<String>,
    /// Substring match on model id or display name
    #[param(example = "gpt")]
    pub(crate) query: Option<String>,
}

impl ModelFilter {
    pub(crate) fn providers(&self) -> Vec<&str> {
        match non_empty(self.provider.as_deref()) {
            None => vec![],
            Some(s) => s
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }

    pub(crate) fn statuses(&self) -> Vec<&str> {
        match non_empty(self.status.as_deref()) {
            None => vec![],
            Some(s) => s
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }

    pub(crate) fn capability(&self) -> Option<&str> {
        non_empty(self.capability.as_deref())
    }

    pub(crate) fn modality_input(&self) -> Option<&str> {
        non_empty(self.modality_input.as_deref())
    }

    pub(crate) fn modality_output(&self) -> Option<&str> {
        non_empty(self.modality_output.as_deref())
    }

    pub(crate) fn sort(&self) -> Option<&str> {
        non_empty(self.sort.as_deref())
    }

    pub(crate) fn order(&self) -> Option<&str> {
        non_empty(self.order.as_deref())
    }

    pub(crate) fn limit(&self) -> Option<usize> {
        self.limit.filter(|limit| *limit > 0).map(clamp_limit)
    }

    pub(crate) fn offset(&self) -> Option<usize> {
        self.offset
    }

    pub(crate) fn endpoint_family(&self) -> Option<&str> {
        non_empty(self.endpoint_family.as_deref())
    }

    pub(crate) fn pricing(&self) -> Option<&str> {
        non_empty(self.pricing.as_deref())
    }

    pub(crate) fn query(&self) -> Option<&str> {
        non_empty(self.query.as_deref())
    }
}

pub(crate) fn clamp_limit(limit: usize) -> usize {
    limit.min(MAX_LIMIT)
}

#[utoipa::path(
    get,
    path = "/api/v1/models",
    tag = "Catalog",
    operation_id = "list_models",
    summary = "List models",
    description = "Paginated, filterable model list with ETag support.",
    params(ModelFilter),
    responses(
        (
            status = 200,
            description = "Paginated model list",
            body = ApiResponse<Vec<Model>>,
            examples(
                ("models" = (
                    summary = "Model page",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": [
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
            description = "Invalid query parameters",
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
        ),
        (status = 304, description = "Catalog not modified")
    )
)]
pub async fn list_models(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    query: Result<Query<ModelFilter>, QueryRejection>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Query(filter) = match query {
        Ok(query) => query,
        Err(error) => {
            return ApiError::BadRequest {
                message: error.body_text(),
            }
            .into_response();
        }
    };
    let state = state.read().await;

    if let Some(inm) = headers.get("if-none-match")
        && inm.as_bytes() == state.etag.as_bytes()
    {
        return StatusCode::NOT_MODIFIED.into_response();
    }

    let mut models: Vec<Model> = state
        .models
        .iter()
        .filter(|model| model_matches_filter(model, &filter))
        .cloned()
        .collect();

    sort_models(&mut models, filter.sort(), filter.order());

    let total = models.len();
    let offset = filter.offset().unwrap_or(0).min(total);
    let limit = filter.limit().unwrap_or(DEFAULT_LIMIT);
    let data: Vec<_> = models.into_iter().skip(offset).take(limit).collect();

    Json(ApiResponse::paginated_with_context(
        data, limit, offset, total, &context,
    ))
    .into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/models/{provider}/{id}",
    tag = "Catalog",
    operation_id = "get_model",
    summary = "Get model",
    description = "Returns metadata for a single model by provider and id.",
    params(
        ("provider" = String, Path, description = "Provider id", example = "openai"),
        ("id" = String, Path, description = "Model id", example = "gpt-4o")
    ),
    responses(
        (
            status = 200,
            description = "Model metadata",
            body = ApiResponse<Model>,
            examples(
                ("model" = (
                    summary = "Single model",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": {
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
                        },
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 304, description = "Catalog not modified"),
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
pub async fn get_model(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    Path((provider, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let state = state.read().await;

    let model = state
        .models_by_provider_id
        .get(&(provider.clone(), id.clone()))
        .and_then(|index| state.models.get(*index));

    match model {
        None => ApiError::ModelNotFound { provider, id }.into_response(),
        Some(m) => {
            if let Some(inm) = headers.get("if-none-match")
                && inm.as_bytes() == state.etag.as_bytes()
            {
                return StatusCode::NOT_MODIFIED.into_response();
            }
            Json(ApiResponse::ok_with_context(m.clone(), &context)).into_response()
        }
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let value = value.trim();
        if value.is_empty() { None } else { Some(value) }
    })
}

fn price_is_within_usd(currency: &str, price: f64, maximum: f64) -> bool {
    currency == "USD" && price <= maximum
}

pub(crate) fn model_matches_filter(model: &Model, filter: &ModelFilter) -> bool {
    let providers = filter.providers();
    if !providers.is_empty() && !providers.contains(&model.provider.as_str()) {
        return false;
    }
    let statuses = filter.statuses();
    if !statuses.is_empty() && !statuses.contains(&model.status.as_str()) {
        return false;
    }
    if let Some(caps) = filter.capability() {
        if model.capabilities_state != Some(crate::catalog::MetadataState::Known) {
            return false;
        }
        for cap in caps.split(',') {
            if model.capabilities.get(cap.trim()) != Some(&true) {
                return false;
            }
        }
    }
    if let Some(modalities) = filter.modality_input() {
        for modality in modalities.split(',') {
            if !model.modalities.as_ref().is_some_and(|known| {
                known
                    .input
                    .iter()
                    .any(|supported| supported == modality.trim())
            }) {
                return false;
            }
        }
    }
    if let Some(modalities) = filter.modality_output() {
        for modality in modalities.split(',') {
            if !model.modalities.as_ref().is_some_and(|known| {
                known
                    .output
                    .iter()
                    .any(|supported| supported == modality.trim())
            }) {
                return false;
            }
        }
    }
    if let Some(min_context) = filter.min_context
        && model
            .context_window
            .map(|value| value < min_context)
            .unwrap_or(true)
    {
        return false;
    }
    if filter.max_input_price.is_some()
        && (!model.has_comparable_token_pricing()
            || model
                .pricing
                .as_ref()
                .is_none_or(|pricing| pricing.currency != "USD"))
    {
        return false;
    }
    if let Some(max_input_price) = filter.max_input_price
        && model.pricing.as_ref().is_none_or(|pricing| {
            !price_is_within_usd(&pricing.currency, pricing.input, max_input_price)
        })
    {
        return false;
    }
    if filter.max_output_price.is_some()
        && (!model.has_comparable_token_pricing()
            || model
                .pricing
                .as_ref()
                .is_none_or(|pricing| pricing.currency != "USD"))
    {
        return false;
    }
    if let Some(max_output_price) = filter.max_output_price
        && model.pricing.as_ref().is_none_or(|pricing| {
            !price_is_within_usd(&pricing.currency, pricing.output, max_output_price)
        })
    {
        return false;
    }
    if let Some(pricing_filter) = filter.pricing() {
        match pricing_filter {
            "available" if !model.pricing_is_available() => return false,
            "unavailable" if !model.pricing_is_missing() => return false,
            "free" if !model.is_known_free() => return false,
            "paid" if !model.is_known_paid() => return false,
            _ => {}
        }
    }
    if let Some(ef) = filter.endpoint_family()
        && model.endpoint_family.as_str() != ef
    {
        return false;
    }
    if let Some(q) = filter.query() {
        let q = q.to_lowercase();
        let name_match = model.display_name.to_lowercase().contains(&q);
        let id_match = model.id.to_lowercase().contains(&q);
        if !name_match && !id_match {
            return false;
        }
    }
    true
}

pub(crate) fn sort_models(models: &mut [Model], sort: Option<&str>, order: Option<&str>) {
    let sort = sort.unwrap_or("provider");
    let descending = order == Some("desc");

    models.sort_by(|a, b| {
        use std::cmp::Ordering;

        if matches!(sort, "input_price" | "output_price") {
            let a_price = token_price_for_sort(a, sort);
            let b_price = token_price_for_sort(b, sort);
            return match (a_price, b_price) {
                (Some((a_currency, a_value)), Some((b_currency, b_value))) => {
                    match a_currency.cmp(b_currency) {
                        Ordering::Equal if descending => {
                            b_value.partial_cmp(&a_value).unwrap_or(Ordering::Equal)
                        }
                        Ordering::Equal => a_value.partial_cmp(&b_value).unwrap_or(Ordering::Equal),
                        currency_order => currency_order,
                    }
                }
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => a.provider.cmp(&b.provider).then(a.id.cmp(&b.id)),
            };
        }

        let ordering = match sort {
            "context_window" => a.context_window.cmp(&b.context_window),
            "max_output_tokens" => a.max_output_tokens.cmp(&b.max_output_tokens),
            "status" => a.status.as_str().cmp(b.status.as_str()),
            "id" => a.id.cmp(&b.id),
            "provider" => a.provider.cmp(&b.provider),
            _ => a.provider.cmp(&b.provider).then(a.id.cmp(&b.id)),
        };

        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
}

fn token_price_for_sort<'a>(model: &'a Model, sort: &str) -> Option<(&'a str, f64)> {
    if !model.has_comparable_token_pricing() {
        return None;
    }
    let pricing = model.pricing.as_ref()?;
    let currency = pricing.currency.as_str();
    if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return None;
    }
    let value = match sort {
        "input_price" => pricing.input,
        "output_price" => pricing.output,
        _ => return None,
    };
    Some((currency, value))
}

#[cfg(test)]
mod pricing_filter_tests {
    use super::{ModelFilter, model_matches_filter, price_is_within_usd, sort_models};
    use crate::catalog::{
        Confidence, EndpointFamily, Modalities, Model, ModelParameters, ModelStatus, Pricing,
        PricingState,
    };
    use std::collections::HashMap;

    fn model(id: &str, currency: Option<&str>, input: f64, output: f64) -> Model {
        Model {
            id: id.to_string(),
            provider: "test".to_string(),
            display_name: id.to_string(),
            status: ModelStatus::Active,
            release_date: None,
            deprecation_date: None,
            sunset_date: None,
            replacement: None,
            context_window: None,
            context_window_state: None,
            max_output_tokens: None,
            max_output_tokens_state: None,
            modalities: Some(Modalities {
                input: vec!["text".to_string()],
                output: vec!["text".to_string()],
            }),
            modalities_state: None,
            capabilities: HashMap::new(),
            capabilities_state: None,
            parameters: ModelParameters {
                supported: vec![],
                rejected: vec![],
                deprecated_for_this_model: vec![],
            },
            parameters_state: None,
            pricing: currency.map(|currency| Pricing {
                currency: currency.to_string(),
                input,
                output,
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
                input_audio: None,
                input_image: None,
                input_video: None,
                output_audio: None,
                tiers: None,
                pricing_notes: None,
            }),
            pricing_state: currency.map(|_| PricingState::Published),
            last_verified: None,
            confidence: Confidence::Unknown,
            endpoint_family: EndpointFamily::ChatCompletions,
            sources: vec![],
        }
    }

    #[test]
    fn usd_price_filter_does_not_compare_other_currencies() {
        assert!(price_is_within_usd("USD", 0.5, 1.0));
        assert!(!price_is_within_usd("EUR", 0.5, 1.0));
        assert!(!price_is_within_usd("EUR", 2.0, 1.0));
    }

    #[test]
    fn price_sort_groups_currencies_without_comparing_their_amounts() {
        let mut models = vec![
            model("eur", Some("EUR"), 0.01, 0.02),
            model("usd-high", Some("USD"), 0.9, 1.8),
            model("usd-low", Some("USD"), 0.02, 0.04),
        ];

        sort_models(&mut models, Some("input_price"), Some("asc"));
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["eur", "usd-low", "usd-high"]
        );

        sort_models(&mut models, Some("output_price"), Some("desc"));
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["eur", "usd-high", "usd-low"]
        );
    }

    #[test]
    fn descending_price_sort_keeps_unknown_prices_last_and_unpriced_is_not_free() {
        let mut unknown = model("unknown", None, 0.0, 0.0);
        unknown.pricing_state = Some(PricingState::Unknown);
        let mut models = vec![
            unknown.clone(),
            model("low", Some("USD"), 0.1, 0.2),
            model("high", Some("USD"), 1.0, 2.0),
        ];

        sort_models(&mut models, Some("input_price"), Some("desc"));
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>(),
            ["high", "low", "unknown"]
        );

        let free_filter = ModelFilter {
            provider: None,
            status: None,
            capability: None,
            limit: None,
            offset: None,
            sort: None,
            order: None,
            modality_input: None,
            modality_output: None,
            min_context: None,
            max_input_price: None,
            max_output_price: None,
            pricing: Some("free".to_string()),
            endpoint_family: None,
            query: None,
        };
        assert!(!model_matches_filter(&unknown, &free_filter));
        assert!(!unknown.is_known_free());
    }
}
