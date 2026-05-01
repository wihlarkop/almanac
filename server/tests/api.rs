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

// ── validate ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn validate_known_active_model() {
    let body = serde_json::json!({"model": "claude-opus-4-7"});
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["valid"], true);
    assert_eq!(json["canonical_id"], "claude-opus-4-7");
    assert!(json["errors"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn validate_alias_resolves_to_canonical() {
    let body = serde_json::json!({"model": "claude-opus-4"});
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["valid"], true);
    assert_eq!(json["canonical_id"], "claude-opus-4-7");
}

#[tokio::test]
async fn validate_unknown_model_returns_not_found_with_suggestions() {
    let body = serde_json::json!({"model": "claude-opus-4.7"});
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["valid"], false);
    assert!(json["canonical_id"].is_null());
    assert_eq!(json["errors"][0]["code"], "MODEL_NOT_FOUND");
    let suggestions = json["errors"][0]["suggestions"].as_array().unwrap();
    assert!(!suggestions.is_empty());
    assert!(suggestions.iter().any(|s| s.as_str() == Some("claude-opus-4-7")
        || s.as_str() == Some("claude-opus-4")));
}

#[tokio::test]
async fn validate_retired_model_returns_error() {
    let body = serde_json::json!({"model": "gpt-4-vision-preview"});
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["valid"], false);
    assert_eq!(json["errors"][0]["code"], "MODEL_RETIRED");
}

#[tokio::test]
async fn validate_deprecated_model_returns_warning() {
    let body = serde_json::json!({"model": "gpt-3.5-turbo"});
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["valid"], true);
    assert!(json["errors"].as_array().unwrap().is_empty());
    assert_eq!(json["warnings"][0]["code"], "MODEL_DEPRECATED");
}

#[tokio::test]
async fn validate_provider_mismatch_returns_error() {
    let body = serde_json::json!({"model": "gpt-4o", "provider": "anthropic"});
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["valid"], false);
    assert_eq!(json["errors"][0]["code"], "PROVIDER_MISMATCH");
}

// ── suggest ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn suggest_returns_ranked_matches() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/suggest?q=claude-opus-4.7")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let results = json.as_array().unwrap();
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r["id"].as_str() == Some("claude-opus-4-7")));
    if results.len() > 1 {
        assert!(results[0]["score"].as_f64() >= results[1]["score"].as_f64());
    }
}

#[tokio::test]
async fn suggest_no_matches_returns_empty() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/suggest?q=zzzzzzzzz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn suggest_max_five_results() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/suggest?q=gpt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.as_array().unwrap().len() <= 5);
}
