use crate::{request::RequestContext, response::ApiResponse};
use axum::Extension;
use axum::response::Json;
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthData {
    pub status: &'static str,
    pub version: &'static str,
}

#[utoipa::path(
    get,
    path = "/api/v1/health",
    responses((status = 200, description = "Server health status", body = ApiResponse<HealthData>))
)]
pub async fn health(
    Extension(context): Extension<RequestContext>,
) -> Json<ApiResponse<HealthData>> {
    Json(ApiResponse::ok_with_context(
        HealthData {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
        },
        &context,
    ))
}
