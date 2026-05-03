use crate::{
    response::{ApiResponse, catalog_headers},
    state::AppState,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
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

    (
        catalog_headers(&state.etag),
        Json(ApiResponse::ok(state.providers.clone())),
    )
        .into_response()
}
