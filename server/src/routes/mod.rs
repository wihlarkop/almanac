use std::sync::Arc;
use axum::{routing::get, Router};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use crate::state::AppState;

mod health;
mod models;
mod providers;

pub fn router(state: Arc<RwLock<AppState>>) -> Router {
    Router::new()
        .route("/v1/health", get(health::health))
        .route("/v1/providers", get(providers::list_providers))
        .route("/v1/models", get(models::list_models))
        .route("/v1/models/:provider/:id", get(models::get_model))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
