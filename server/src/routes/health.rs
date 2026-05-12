use crate::{request::RequestContext, response::ApiResponse, state::AppState};
use axum::response::Json;
use axum::{Extension, extract::State};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthData {
    pub status: &'static str,
    pub version: &'static str,
    pub total_models: usize,
    pub total_providers: usize,
    pub total_aliases: usize,
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses((
        status = 200,
        description = "Server health status",
        body = ApiResponse<HealthData>,
        examples(
            ("ok" = (
                summary = "Healthy server",
                value = json!({
                    "success": true,
                    "message": "OK",
                    "data": {
                        "status": "ok",
                        "version": "0.1.0",
                        "total_models": 120,
                        "total_providers": 12,
                        "total_aliases": 40
                    },
                    "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                    "error": null
                })
            ))
        )
    ))
)]
pub async fn health(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
) -> Json<ApiResponse<HealthData>> {
    let state = state.read().await;
    Json(ApiResponse::ok_with_context(
        HealthData {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            total_models: state.models.len(),
            total_providers: state.providers.len(),
            total_aliases: state.aliases.len(),
        },
        &context,
    ))
}
