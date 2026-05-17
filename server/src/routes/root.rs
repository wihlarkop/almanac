use crate::{request::RequestContext, response::ApiResponse};
use axum::{Extension, response::Json};
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct RootData {
    pub name: &'static str,
    pub version: &'static str,
    pub base_path: &'static str,
    pub health: &'static str,
    pub openapi: &'static str,
    pub swagger_ui: &'static str,
    pub scalar: &'static str,
}

#[utoipa::path(
    get,
    path = "/",
    tag = "Root",
    operation_id = "root",
    summary = "Landing",
    description = "Returns API metadata and documentation links.",
    responses((
        status = 200,
        description = "API landing metadata",
        body = ApiResponse<RootData>,
        examples(
            ("landing" = (
                summary = "API landing response",
                value = json!({
                    "success": true,
                    "message": "OK",
                    "data": {
                        "name": "Almanac API",
                        "version": "0.1.0",
                        "base_path": "/api/v1",
                        "health": "/api/v1/health",
                        "openapi": "/openapi.json",
                        "swagger_ui": "/swagger-ui/",
                        "scalar": "/scalar"
                    },
                    "meta": {
                        "timestamp": "2026-05-03T00:00:00Z",
                        "request_id": "req-example",
                        "execution_time_seconds": 0.001
                    },
                    "error": null
                })
            ))
        )
    ))
)]
pub async fn root(Extension(context): Extension<RequestContext>) -> Json<ApiResponse<RootData>> {
    Json(ApiResponse::ok_with_context(
        RootData {
            name: "Almanac API",
            version: env!("CARGO_PKG_VERSION"),
            base_path: "/api/v1",
            health: "/api/v1/health",
            openapi: "/openapi.json",
            swagger_ui: "/swagger-ui/",
            scalar: "/scalar",
        },
        &context,
    ))
}
