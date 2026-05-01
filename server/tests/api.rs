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

#[tokio::test]
async fn providers_returns_array_with_cache_headers() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/v1/providers").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "public, max-age=300"
    );
    assert!(response.headers().contains_key("etag"));

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().len() >= 3);
}

#[tokio::test]
async fn providers_etag_returns_304() {
    let app = app().await;

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/v1/providers").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let etag = response.headers().get("etag").unwrap().clone();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/providers")
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn models_returns_all_with_cache_headers() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/v1/models").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "public, max-age=300"
    );
    assert!(response.headers().contains_key("etag"));

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().len() >= 30);
}

#[tokio::test]
async fn models_filter_by_provider() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models?provider=anthropic")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let models = json.as_array().unwrap();
    assert!(!models.is_empty());
    assert!(models
        .iter()
        .all(|m| m["provider"].as_str() == Some("anthropic")));
}

#[tokio::test]
async fn models_filter_by_status() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models?status=active")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let models = json.as_array().unwrap();
    assert!(!models.is_empty());
    assert!(models
        .iter()
        .all(|m| m["status"].as_str() == Some("active")));
}

#[tokio::test]
async fn models_filter_by_capability() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models?capability=vision")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let models = json.as_array().unwrap();
    assert!(!models.is_empty());
    assert!(models
        .iter()
        .all(|m| m["capabilities"]["vision"].as_bool() == Some(true)));
}

#[tokio::test]
async fn get_model_found() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models/anthropic/claude-opus-4-7")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("cache-control").unwrap(),
        "public, max-age=300"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], "claude-opus-4-7");
    assert_eq!(json["provider"], "anthropic");
}

#[tokio::test]
async fn get_model_not_found() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models/openai/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "MODEL_NOT_FOUND");
}

#[tokio::test]
async fn get_model_etag_304() {
    let app = app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/models/openai/gpt-4o")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let etag = response.headers().get("etag").unwrap().clone();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/models/openai/gpt-4o")
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
}
