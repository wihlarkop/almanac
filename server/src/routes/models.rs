use axum::{extract::State, response::Json};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::state::AppState;

pub async fn list_models(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    Json(serde_json::Value::Array(state.models.clone()))
}

pub async fn get_model() -> axum::http::StatusCode {
    axum::http::StatusCode::NOT_FOUND
}
