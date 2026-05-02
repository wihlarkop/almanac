use axum::response::Json;
use serde_json::json;

#[utoipa::path(
    get,
    path = "/v1/health",
    responses((status = 200, description = "Server health status"))
)]
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok", "version": "0.1.0"}))
}
