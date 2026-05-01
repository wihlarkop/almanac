use axum::{
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ModelFilter {
    provider: Option<String>,
    status: Option<String>,
    capability: Option<String>,
}

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

    let models: Vec<serde_json::Value> = state
        .models
        .iter()
        .filter(|m| {
            if let Some(ref p) = filter.provider {
                if m["provider"].as_str() != Some(p.as_str()) {
                    return false;
                }
            }
            if let Some(ref s) = filter.status {
                if m["status"].as_str() != Some(s.as_str()) {
                    return false;
                }
            }
            if let Some(ref caps) = filter.capability {
                for cap in caps.split(',') {
                    if m["capabilities"][cap.trim()].as_bool() != Some(true) {
                        return false;
                    }
                }
            }
            true
        })
        .cloned()
        .collect();

    let mut resp_headers = HeaderMap::new();
    resp_headers.insert(
        "cache-control",
        HeaderValue::from_static("public, max-age=300"),
    );
    resp_headers.insert("etag", HeaderValue::from_str(&state.etag).unwrap());

    (resp_headers, Json(models)).into_response()
}

pub async fn get_model() -> StatusCode {
    StatusCode::NOT_FOUND
}
