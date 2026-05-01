use almanac_server::{routes, state};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::{path::Path, sync::Arc};
use tokio::sync::RwLock;
use tower::ServiceExt;

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

async fn app() -> axum::Router {
    let s = state::load_state(&data_dir()).expect("load_state failed");
    let shared = Arc::new(RwLock::new(s));
    routes::router(shared)
}

#[tokio::test]
async fn health_returns_ok() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/v1/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["version"], "0.1.0");
}
