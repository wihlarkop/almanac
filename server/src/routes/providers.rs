use axum::{extract::State, response::Json};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::state::AppState;

pub async fn list_providers(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    Json(serde_json::Value::Array(state.providers.clone()))
}
