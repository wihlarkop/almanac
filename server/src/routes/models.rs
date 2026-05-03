use crate::{
    catalog::Model,
    error::ApiError,
    request::RequestContext,
    response::{ApiResponse, catalog_headers},
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

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ModelFilter {
    provider: Option<String>,
    status: Option<String>,
    capability: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
    sort: Option<String>,
    order: Option<String>,
    modality_input: Option<String>,
    modality_output: Option<String>,
    min_context: Option<u64>,
    max_input_price: Option<f64>,
}

impl ModelFilter {
    fn provider(&self) -> Option<&str> {
        non_empty(self.provider.as_deref())
    }

    fn status(&self) -> Option<&str> {
        non_empty(self.status.as_deref())
    }

    fn capability(&self) -> Option<&str> {
        non_empty(self.capability.as_deref())
    }

    fn modality_input(&self) -> Option<&str> {
        non_empty(self.modality_input.as_deref())
    }

    fn modality_output(&self) -> Option<&str> {
        non_empty(self.modality_output.as_deref())
    }

    fn sort(&self) -> Option<&str> {
        non_empty(self.sort.as_deref())
    }

    fn order(&self) -> Option<&str> {
        non_empty(self.order.as_deref())
    }

    fn limit(&self) -> Option<usize> {
        self.limit.filter(|limit| *limit > 0)
    }
}

#[utoipa::path(
    get,
    path = "/v1/models",
    params(ModelFilter),
    responses(
        (status = 200, description = "Paginated model list", body = ApiResponse<Vec<Model>>),
        (status = 400, description = "Invalid query parameters", body = ApiResponse<crate::response::EmptyData>),
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

    if let Some(inm) = headers.get("if-none-match") {
        if inm.as_bytes() == state.etag.as_bytes() {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }

    let mut models: Vec<Model> = state
        .models
        .iter()
        .filter(|m| {
            if let Some(provider) = filter.provider() {
                if m.provider != provider {
                    return false;
                }
            }
            if let Some(status) = filter.status() {
                if m.status.as_str() != status {
                    return false;
                }
            }
            if let Some(caps) = filter.capability() {
                for cap in caps.split(',') {
                    if m.capabilities.get(cap.trim()) != Some(&true) {
                        return false;
                    }
                }
            }
            if let Some(modalities) = filter.modality_input() {
                for modality in modalities.split(',') {
                    if !m
                        .modalities
                        .input
                        .iter()
                        .any(|supported| supported == modality.trim())
                    {
                        return false;
                    }
                }
            }
            if let Some(modalities) = filter.modality_output() {
                for modality in modalities.split(',') {
                    if !m
                        .modalities
                        .output
                        .iter()
                        .any(|supported| supported == modality.trim())
                    {
                        return false;
                    }
                }
            }
            if let Some(min_context) = filter.min_context {
                if m.context_window < min_context {
                    return false;
                }
            }
            if let Some(max_input_price) = filter.max_input_price {
                if m.pricing
                    .as_ref()
                    .map(|pricing| pricing.input)
                    .unwrap_or(f64::MAX)
                    > max_input_price
                {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    sort_models(&mut models, filter.sort(), filter.order());

    let total = models.len();
    let offset = filter.offset.unwrap_or(0).min(total);
    let limit = filter
        .limit()
        .unwrap_or(DEFAULT_LIMIT)
        .min(total.saturating_sub(offset));
    let data: Vec<_> = models.into_iter().skip(offset).take(limit).collect();

    (
        catalog_headers(&state.etag),
        Json(ApiResponse::paginated_with_context(
            data, limit, offset, total, &context,
        )),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/models/{provider}/{id}",
    params(
        ("provider" = String, Path, description = "Provider id"),
        ("id" = String, Path, description = "Model id")
    ),
    responses(
        (status = 200, description = "Model metadata", body = ApiResponse<Model>),
        (status = 304, description = "Catalog not modified"),
        (status = 404, description = "Model not found", body = ApiResponse<crate::response::EmptyData>)
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
        .models
        .iter()
        .find(|m| m.provider == provider && m.id == id);

    match model {
        None => ApiError::ModelNotFound { provider, id }.into_response(),
        Some(m) => {
            if let Some(inm) = headers.get("if-none-match") {
                if inm.as_bytes() == state.etag.as_bytes() {
                    return StatusCode::NOT_MODIFIED.into_response();
                }
            }
            (
                catalog_headers(&state.etag),
                Json(ApiResponse::ok_with_context(m.clone(), &context)),
            )
                .into_response()
        }
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let value = value.trim();
        if value.is_empty() { None } else { Some(value) }
    })
}

fn sort_models(models: &mut [Model], sort: Option<&str>, order: Option<&str>) {
    let sort = sort.unwrap_or("provider");
    let descending = order == Some("desc");

    models.sort_by(|a, b| {
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
