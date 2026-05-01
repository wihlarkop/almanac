use std::sync::Arc;
use axum::Router;
use tokio::sync::RwLock;
use crate::state::AppState;

pub fn router(_state: Arc<RwLock<AppState>>) -> Router {
    Router::new()
}
