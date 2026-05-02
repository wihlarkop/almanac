use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

mod health;
mod models;
mod providers;
mod suggest;
mod validate;

pub fn router(state: Arc<RwLock<AppState>>) -> Router {
    Router::new()
        .route("/v1/health", get(health::health))
        .route("/v1/providers", get(providers::list_providers))
        .route("/v1/models", get(models::list_models))
        .route("/v1/models/{provider}/{id}", get(models::get_model))
        .route("/v1/validate", post(validate::validate))
        .route("/v1/suggest", get(suggest::suggest))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
