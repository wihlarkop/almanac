use crate::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
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
        (status = 200, description = "Paginated model list"),
        (status = 304, description = "Catalog not modified")
    )
)]
pub async fn list_models(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(filter): Query<ModelFilter>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let state = state.read().await;

    if let Some(inm) = headers.get("if-none-match") {
        if inm.as_bytes() == state.etag.as_bytes() {
            return StatusCode::NOT_MODIFIED.into_response();
        }
    }

    let mut models: Vec<serde_json::Value> = state
        .models
        .iter()
        .filter(|m| {
            if let Some(provider) = filter.provider() {
                if m["provider"].as_str() != Some(provider) {
                    return false;
                }
            }
            if let Some(status) = filter.status() {
                if m["status"].as_str() != Some(status) {
                    return false;
                }
            }
            if let Some(caps) = filter.capability() {
                for cap in caps.split(',') {
                    if m["capabilities"][cap.trim()].as_bool() != Some(true) {
                        return false;
                    }
                }
            }
            if let Some(modalities) = filter.modality_input() {
                for modality in modalities.split(',') {
                    if !array_contains(&m["modalities"]["input"], modality.trim()) {
                        return false;
                    }
                }
            }
            if let Some(modalities) = filter.modality_output() {
                for modality in modalities.split(',') {
                    if !array_contains(&m["modalities"]["output"], modality.trim()) {
                        return false;
                    }
                }
            }
            if let Some(min_context) = filter.min_context {
                if m["context_window"].as_u64().unwrap_or(0) < min_context {
                    return false;
                }
            }
            if let Some(max_input_price) = filter.max_input_price {
                if m["pricing"]["input"].as_f64().unwrap_or(f64::MAX) > max_input_price {
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

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "cache-control",
        HeaderValue::from_static("public, max-age=300"),
    );
    resp_headers.insert("etag", HeaderValue::from_str(&state.etag).unwrap());

    (
        resp_headers,
        Json(serde_json::json!({
            "data": data,
            "meta": {
                "total": total,
                "limit": limit,
                "offset": offset
            }
        })),
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
        (status = 200, description = "Model metadata"),
        (status = 304, description = "Catalog not modified"),
        (status = 404, description = "Model not found")
    )
)]
pub async fn get_model(
    State(state): State<Arc<RwLock<AppState>>>,
    Path((provider, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let state = state.read().await;

    let model = state.models.iter().find(|m| {
        m["provider"].as_str() == Some(provider.as_str()) && m["id"].as_str() == Some(id.as_str())
    });

    match model {
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "not found",
                "code": "MODEL_NOT_FOUND"
            })),
        )
            .into_response(),
        Some(m) => {
            if let Some(inm) = headers.get("if-none-match") {
                if inm.as_bytes() == state.etag.as_bytes() {
                    return StatusCode::NOT_MODIFIED.into_response();
                }
            }
            let mut resp_headers = HeaderMap::new();
            resp_headers.insert(
                "cache-control",
                HeaderValue::from_static("public, max-age=300"),
            );
            resp_headers.insert("etag", HeaderValue::from_str(&state.etag).unwrap());
            (resp_headers, Json(m.clone())).into_response()
        }
    }
}

fn array_contains(value: &serde_json::Value, expected: &str) -> bool {
    value
        .as_array()
        .map(|items| items.iter().any(|item| item.as_str() == Some(expected)))
        .unwrap_or(false)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let value = value.trim();
        if value.is_empty() { None } else { Some(value) }
    })
}

fn sort_models(models: &mut [serde_json::Value], sort: Option<&str>, order: Option<&str>) {
    let sort = sort.unwrap_or("provider");
    let descending = order == Some("desc");

    models.sort_by(|a, b| {
        let ordering = match sort {
            "context_window" | "max_output_tokens" => a[sort]
                .as_u64()
                .unwrap_or(0)
                .cmp(&b[sort].as_u64().unwrap_or(0)),
            "status" | "id" | "provider" => a[sort]
                .as_str()
                .unwrap_or_default()
                .cmp(b[sort].as_str().unwrap_or_default()),
            _ => a["provider"]
                .as_str()
                .unwrap_or_default()
                .cmp(b["provider"].as_str().unwrap_or_default())
                .then(
                    a["id"]
                        .as_str()
                        .unwrap_or_default()
                        .cmp(b["id"].as_str().unwrap_or_default()),
                ),
        };

        if descending {
            ordering.reverse()
        } else {
            ordering
        }
    });
}
