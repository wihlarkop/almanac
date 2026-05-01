use axum::response::Json;
use serde_json::json;

pub async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok", "version": "0.1.0"}))
}
