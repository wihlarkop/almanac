use crate::response::ApiResponse;
use axum::response::Json;
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthData {
    pub status: &'static str,
    pub version: &'static str,
}

#[utoipa::path(
    get,
    path = "/v1/health",
    responses((status = 200, description = "Server health status", body = ApiResponse<HealthData>))
)]
pub async fn health() -> Json<ApiResponse<HealthData>> {
    Json(ApiResponse::ok(HealthData {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }))
}
