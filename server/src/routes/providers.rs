use crate::state::AppState;
use axum::{
    extract::State,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Json},
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[utoipa::path(
    get,
    path = "/v1/providers",
    responses(
        (status = 200, description = "Provider list"),
        (status = 304, description = "Catalog not modified")
    )
)]
pub async fn list_providers(
    State(state): State<Arc<RwLock<AppState>>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let state = state.read().await;

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

    (resp_headers, Json(state.providers.clone())).into_response()
}
