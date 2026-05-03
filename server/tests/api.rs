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

fn assert_success_envelope(json: &serde_json::Value) {
    assert_eq!(json["success"], true);
    assert_eq!(json["message"], "OK");
    assert!(json["error"].is_null());
    assert!(json["meta"]["timestamp"].as_str().is_some());
}

fn assert_error_envelope(json: &serde_json::Value, code: &str) {
    assert_eq!(json["success"], false);
    assert!(json["data"].is_null());
    assert_eq!(json["error"]["code"], code);
    assert!(json["meta"]["timestamp"].as_str().is_some());
}

async fn get_json(uri: &str) -> serde_json::Value {
    let response = app()
        .await
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

fn model_ids(json: &serde_json::Value) -> Vec<String> {
    json["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|model| model["id"].as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn health_returns_ok() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));
    assert_eq!(
        response.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_success_envelope(&json);
    assert!(json["meta"]["request_id"].as_str().is_some());
    assert!(json["meta"]["execution_time_seconds"].as_f64().is_some());
    assert_eq!(json["data"]["status"], "ok");
    assert_eq!(json["data"]["version"], "0.1.0");
}

#[tokio::test]
async fn cors_preflight_allows_api_headers() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/v1/models")
                .header("origin", "https://example.com")
                .header("access-control-request-method", "GET")
                .header(
                    "access-control-request-headers",
                    "content-type,if-none-match",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .unwrap(),
        "*"
    );
}

#[tokio::test]
async fn missing_route_returns_error_envelope() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert!(response.headers().contains_key("x-request-id"));

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["success"], false);
    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn providers_returns_array_with_cache_headers() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/providers")
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
    assert!(response.headers().contains_key("etag"));

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_success_envelope(&json);
    assert!(json["data"].as_array().unwrap().len() >= 3);
}

#[tokio::test]
async fn providers_etag_returns_304() {
    let app = app().await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/providers")
                .body(Body::empty())
                .unwrap(),
        )
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
async fn openapi_json_returns_spec() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
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
    assert_eq!(json["openapi"], "3.1.0");
    assert_eq!(json["info"]["title"], "Almanac API");
    assert!(json["paths"]["/v1/models"].is_object());
    assert!(json["paths"]["/v1/validate"].is_object());
    assert!(json["components"]["schemas"]["Model"].is_object());
    assert!(json["components"]["schemas"]["Provider"].is_object());
    assert!(json["components"]["schemas"]["ApiResponse_Model"].is_object());
    assert!(json["components"]["schemas"]["ApiResponse_ValidateResponse"].is_object());
}

#[tokio::test]
async fn swagger_ui_route_serves_html() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/swagger-ui/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/html"));
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.contains("Swagger UI"));
}

#[tokio::test]
async fn scalar_route_serves_html() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/scalar")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("text/html"));
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();
    assert!(html.to_lowercase().contains("scalar"));
}

#[tokio::test]
async fn models_returns_all_with_cache_headers() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models")
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
    assert!(response.headers().contains_key("etag"));

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_success_envelope(&json);
    assert!(json["meta"]["request_id"].as_str().is_some());
    assert!(json["meta"]["execution_time_seconds"].as_f64().is_some());
    assert_eq!(json["data"].as_array().unwrap().len(), 20);
    assert!(json["meta"]["total_data"].as_u64().unwrap() >= 30);
    assert_eq!(json["meta"]["limit"], 20);
    assert_eq!(json["meta"]["offset"], 0);
}

#[tokio::test]
async fn models_ignores_blank_query_filters_and_zero_limit() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models?provider=&status=&capability=&limit=0&offset=0&sort=&order=&modality_input=&modality_output=")
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

    assert_success_envelope(&json);
    assert_eq!(json["data"].as_array().unwrap().len(), 20);
    assert!(json["meta"]["total_data"].as_u64().unwrap() >= 30);
    assert_eq!(json["meta"]["limit"], 20);
    assert_eq!(json["meta"]["offset"], 0);
}

#[tokio::test]
async fn models_invalid_query_returns_error_envelope() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models?limit=not-a-number")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["data"].is_null());
    assert_eq!(json["error"]["code"], "BAD_REQUEST");
    assert!(json["meta"]["timestamp"].as_str().is_some());
}

#[tokio::test]
async fn models_invalid_numeric_query_values_return_error_envelope() {
    for query in [
        "limit=not-a-number",
        "offset=not-a-number",
        "min_context=not-a-number",
        "max_input_price=not-a-number",
    ] {
        let response = app()
            .await
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/models?{query}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_error_envelope(&json, "BAD_REQUEST");
    }
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
    assert_success_envelope(&json);
    let models = json["data"].as_array().unwrap();
    assert!(!models.is_empty());
    assert!(
        models
            .iter()
            .all(|m| m["provider"].as_str() == Some("anthropic"))
    );
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
    assert_success_envelope(&json);
    let models = json["data"].as_array().unwrap();
    assert!(!models.is_empty());
    assert!(
        models
            .iter()
            .all(|m| m["status"].as_str() == Some("active"))
    );
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
    assert_success_envelope(&json);
    let models = json["data"].as_array().unwrap();
    assert!(!models.is_empty());
    assert!(
        models
            .iter()
            .all(|m| m["capabilities"]["vision"].as_bool() == Some(true))
    );
}

#[tokio::test]
async fn models_support_limit_offset_and_sorting() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models?limit=3&offset=2&sort=id&order=desc")
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
    assert_success_envelope(&json);
    let models = json["data"].as_array().unwrap();
    assert_eq!(models.len(), 3);
    assert_eq!(json["meta"]["limit"], 3);
    assert_eq!(json["meta"]["offset"], 2);
    assert!(json["meta"]["total_data"].as_u64().unwrap() >= 3);

    let ids: Vec<_> = models
        .iter()
        .map(|m| m["id"].as_str().unwrap().to_string())
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_by(|a, b| b.cmp(a));
    assert_eq!(ids, sorted);
}

#[tokio::test]
async fn models_offset_past_total_returns_empty_page_with_clamped_offset() {
    let json = get_json("/v1/models?provider=openai&limit=10&offset=10000").await;

    assert_success_envelope(&json);
    let total = json["meta"]["total_data"].as_u64().unwrap();
    assert!(total > 0);
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
    assert_eq!(json["meta"]["offset"].as_u64().unwrap(), total);
    assert_eq!(json["meta"]["limit"], 0);
}

#[tokio::test]
async fn models_large_limit_reports_actual_remaining_window() {
    let all = get_json("/v1/models?provider=openai&limit=1000").await;
    let total = all["meta"]["total_data"].as_u64().unwrap();
    assert!(total > 2);

    let offset = total - 2;
    let json = get_json(&format!(
        "/v1/models?provider=openai&limit=1000&offset={offset}"
    ))
    .await;

    assert_success_envelope(&json);
    assert_eq!(json["data"].as_array().unwrap().len(), 2);
    assert_eq!(json["meta"]["total_data"].as_u64().unwrap(), total);
    assert_eq!(json["meta"]["offset"].as_u64().unwrap(), offset);
    assert_eq!(json["meta"]["limit"], 2);
}

#[tokio::test]
async fn models_filtered_total_data_counts_matches_before_pagination() {
    let all_openai = get_json("/v1/models?provider=openai&limit=1000").await;
    let total = all_openai["data"].as_array().unwrap().len() as u64;
    assert!(total > 3);

    let page = get_json("/v1/models?provider=openai&limit=3&offset=1").await;

    assert_success_envelope(&page);
    assert_eq!(page["data"].as_array().unwrap().len(), 3);
    assert_eq!(page["meta"]["total_data"].as_u64().unwrap(), total);
    assert_eq!(page["meta"]["limit"], 3);
    assert_eq!(page["meta"]["offset"], 1);
}

#[tokio::test]
async fn models_unknown_sort_falls_back_to_provider_then_id() {
    let json = get_json("/v1/models?limit=10&sort=unknown").await;

    assert_success_envelope(&json);
    let pairs: Vec<_> = json["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|model| {
            (
                model["provider"].as_str().unwrap().to_string(),
                model["id"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let mut sorted = pairs.clone();
    sorted.sort();
    assert_eq!(pairs, sorted);
}

#[tokio::test]
async fn models_unknown_order_behaves_as_ascending() {
    let json = get_json("/v1/models?limit=10&sort=id&order=sideways").await;

    assert_success_envelope(&json);
    let ids = model_ids(&json);
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[tokio::test]
async fn models_filter_by_modality_context_and_price() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/models?modality_input=image&modality_output=text&min_context=100000&max_input_price=1")
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
    assert_success_envelope(&json);
    let models = json["data"].as_array().unwrap();
    assert!(!models.is_empty());
    assert!(models.iter().all(|m| {
        m["modalities"]["input"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some("image"))
            && m["modalities"]["output"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str() == Some("text"))
            && m["context_window"].as_u64().unwrap() >= 100000
            && m["pricing"]["input"].as_f64().unwrap_or(f64::MAX) <= 1.0
    }));
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
    assert_success_envelope(&json);
    assert_eq!(json["data"]["id"], "claude-opus-4-7");
    assert_eq!(json["data"]["provider"], "anthropic");
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
    assert_eq!(json["success"], false);
    assert!(json["data"].is_null());
    assert_eq!(json["message"], "model not found");
    assert_eq!(json["error"]["code"], "MODEL_NOT_FOUND");
    assert_eq!(json["error"]["details"]["provider"], "openai");
    assert_eq!(json["error"]["details"]["id"], "does-not-exist");
    assert!(json["meta"]["timestamp"].as_str().is_some());
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
                .header("content-length", body.to_string().len().to_string())
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], true);
    assert_eq!(json["data"]["canonical_id"], "claude-opus-4-7");
    assert!(json["data"]["errors"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn validate_invalid_json_returns_error_envelope() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .header("content-type", "application/json")
                .body(Body::from("{"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["data"].is_null());
    assert_eq!(json["error"]["code"], "BAD_REQUEST");
    assert!(json["meta"]["timestamp"].as_str().is_some());
}

#[tokio::test]
async fn validate_oversized_body_is_rejected() {
    let oversized = "x".repeat(70 * 1024);
    let body = serde_json::json!({
        "model": "gpt-4o",
        "parameters": {
            "payload": oversized
        }
    });
    let body = body.to_string();
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/validate")
                .header("content-type", "application/json")
                .header("content-length", body.len().to_string())
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], true);
    assert_eq!(json["data"]["canonical_id"], "claude-opus-4-7");
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], false);
    assert!(json["data"]["canonical_id"].is_null());
    assert_eq!(json["data"]["errors"][0]["code"], "MODEL_NOT_FOUND");
    let suggestions = json["data"]["errors"][0]["suggestions"].as_array().unwrap();
    assert!(!suggestions.is_empty());
    assert!(
        suggestions
            .iter()
            .any(|s| s.as_str() == Some("claude-opus-4-7") || s.as_str() == Some("claude-opus-4"))
    );
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], false);
    assert_eq!(json["data"]["errors"][0]["code"], "MODEL_RETIRED");
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], true);
    assert!(json["data"]["errors"].as_array().unwrap().is_empty());
    assert_eq!(json["data"]["warnings"][0]["code"], "MODEL_DEPRECATED");
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], false);
    assert_eq!(json["data"]["errors"][0]["code"], "PROVIDER_MISMATCH");
}

#[tokio::test]
async fn validate_rejected_parameter_returns_error() {
    let body = serde_json::json!({
        "model": "grok-4.20-reasoning",
        "parameters": {
            "reasoning_effort": "high"
        }
    });
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], false);
    assert!(
        json["data"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["code"] == "PARAMETER_REJECTED" && e["parameter"] == "reasoning_effort")
    );
}

#[tokio::test]
async fn validate_unsupported_parameter_returns_warning() {
    let body = serde_json::json!({
        "model": "sonar",
        "parameters": {
            "temperature": 0.3
        }
    });
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], true);
    assert!(
        json["data"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["code"] == "PARAMETER_UNSUPPORTED" && e["parameter"] == "temperature")
    );
}

#[tokio::test]
async fn validate_modality_mismatch_returns_error() {
    let body = serde_json::json!({
        "model": "sonar",
        "modalities": {
            "input": ["image"],
            "output": ["text"]
        }
    });
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], false);
    assert!(
        json["data"]["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["code"] == "MODALITY_UNSUPPORTED" && e["modality"] == "image")
    );
}

#[tokio::test]
async fn validate_supported_request_shape_stays_valid() {
    let body = serde_json::json!({
        "model": "grok-4.20-reasoning",
        "provider": "xai",
        "parameters": {
            "temperature": 0.7,
            "stream": true
        },
        "modalities": {
            "input": ["text", "image"],
            "output": ["text"]
        }
    });
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["valid"], true);
    assert_eq!(json["data"]["canonical_id"], "grok-4.20-reasoning");
    assert!(json["data"]["errors"].as_array().unwrap().is_empty());
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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    let results = json["data"].as_array().unwrap();
    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .any(|r| r["id"].as_str() == Some("claude-opus-4-7"))
    );
    if results.len() > 1 {
        assert!(results[0]["score"].as_f64() >= results[1]["score"].as_f64());
    }
}

#[tokio::test]
async fn suggest_missing_query_returns_error_envelope() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/v1/suggest")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["success"], false);
    assert!(json["data"].is_null());
    assert_eq!(json["error"]["code"], "BAD_REQUEST");
    assert!(json["meta"]["timestamp"].as_str().is_some());
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
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert!(json["data"].as_array().unwrap().is_empty());
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

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_success_envelope(&json);
    assert!(json["data"].as_array().unwrap().len() <= 5);
}
