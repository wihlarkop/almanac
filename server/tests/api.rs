use almanac_server::{
    routes,
    scope::{CatalogScope, ModelRef},
    state,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use std::{collections::HashSet, path::Path, sync::Arc};
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

async fn scoped_app(scope: CatalogScope) -> axum::Router {
    let s =
        state::load_state_with_scope(&data_dir(), &scope).expect("load_state_with_scope failed");
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

fn assert_catalog_cache_headers(headers: &axum::http::HeaderMap) {
    assert_eq!(headers.get("cache-control").unwrap(), "public, max-age=300");
    assert!(headers.contains_key("etag"));
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

async fn scoped_get_json(uri: &str, scope: CatalogScope) -> serde_json::Value {
    let response = scoped_app(scope)
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
async fn cache_policy_is_consistent_for_catalog_read_endpoints() {
    let app = app().await;
    let endpoints = [
        "/api/v1/providers",
        "/api/v1/providers/openai",
        "/api/v1/aliases",
        "/api/v1/aliases/gpt-4o-latest",
        "/api/v1/catalog/health",
        "/api/v1/catalog/issues",
        "/api/v1/models",
        "/api/v1/models/openai/gpt-4o",
    ];

    for endpoint in endpoints {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(endpoint)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK, "{endpoint}");
        assert_catalog_cache_headers(response.headers());
    }
}

#[tokio::test]
async fn health_returns_ok() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
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
    assert!(json["data"]["total_models"].as_u64().unwrap() > 0);
    assert!(json["data"]["total_providers"].as_u64().unwrap() > 0);
    assert!(json["data"]["total_aliases"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn root_returns_api_landing() {
    let response = app()
        .await
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_success_envelope(&json);
    assert_eq!(json["data"]["name"], "Almanac API");
    assert_eq!(json["data"]["version"], "0.1.0");
    assert_eq!(json["data"]["base_path"], "/api/v1");
    assert_eq!(json["data"]["health"], "/api/v1/health");
    assert_eq!(json["data"]["openapi"], "/openapi.json");
}

#[tokio::test]
async fn cors_preflight_allows_api_headers() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/models")
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
async fn old_v1_prefix_returns_not_found() {
    for uri in ["/v1/health", "/v1/models"] {
        let response = app()
            .await
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_error_envelope(&json, "NOT_FOUND");
    }
}

#[tokio::test]
async fn catalog_health_returns_catalog_summary() {
    let json = get_json("/api/v1/catalog/health").await;
    assert_success_envelope(&json);

    let data = &json["data"];
    assert!(data["total_models"].as_u64().unwrap() > 0);
    assert!(data["total_providers"].as_u64().unwrap() > 0);
    assert!(data["total_aliases"].as_u64().unwrap() > 0);
    assert!(data["status_counts"].is_object());
    assert!(data["missing_pricing_count"].as_u64().is_some());
    assert!(data["stale_verification_count"].as_u64().is_some());
    assert!(data["oldest_last_verified"].as_str().is_some());
    assert!(data["newest_last_verified"].as_str().is_some());
    assert!(data["etag"].as_str().is_some());
}

#[tokio::test]
async fn catalog_issues_returns_issue_groups() {
    let json = get_json("/api/v1/catalog/issues").await;
    assert_success_envelope(&json);

    let data = &json["data"];
    assert!(data["stale_models"].as_array().is_some());
    assert!(data["missing_pricing_models"].as_array().is_some());
    assert!(data["deprecated_models"].as_array().is_some());
    assert!(data["retired_models"].as_array().is_some());
    assert!(data["replacement_gaps"].as_array().is_some());
}

#[tokio::test]
async fn providers_returns_array_with_cache_headers() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/providers")
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
                .uri("/api/v1/providers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let etag = response.headers().get("etag").unwrap().clone();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/providers")
                .header("if-none-match", etag)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn provider_detail_returns_provider() {
    let json = get_json("/api/v1/providers/openai").await;
    assert_success_envelope(&json);
    assert_eq!(json["data"]["id"], "openai");
    assert!(json["data"]["display_name"].as_str().is_some());
}

#[tokio::test]
async fn provider_detail_unknown_returns_not_found() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/providers/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_error_envelope(&json, "PROVIDER_NOT_FOUND");
    assert_eq!(json["error"]["details"]["provider"], "does-not-exist");
}

#[tokio::test]
async fn scope_include_provider_hides_other_providers_and_models() {
    let scope = CatalogScope {
        include_providers: HashSet::from(["openai".to_string()]),
        ..CatalogScope::default()
    };

    let providers = scoped_get_json("/api/v1/providers", scope.clone()).await;
    assert_success_envelope(&providers);
    assert!(
        providers["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|provider| provider["id"] == "openai")
    );

    let models = scoped_get_json("/api/v1/models?limit=200", scope.clone()).await;
    assert_success_envelope(&models);
    assert!(
        models["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|model| model["provider"] == "openai")
    );

    let hidden = scoped_app(scope)
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/providers/anthropic")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(hidden.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn scope_exclude_model_removes_alias_target_and_detail_route() {
    let scope = CatalogScope {
        include_providers: HashSet::from(["openai".to_string()]),
        exclude_models: HashSet::from([ModelRef {
            provider: "openai".to_string(),
            id: "gpt-4o".to_string(),
        }]),
        ..CatalogScope::default()
    };

    let aliases = scoped_get_json("/api/v1/aliases", scope.clone()).await;
    assert_success_envelope(&aliases);
    assert!(
        !aliases["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|alias| alias["canonical_id"] == "gpt-4o")
    );

    let response = scoped_app(scope)
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/openai/gpt-4o")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn scope_changes_catalog_etag() {
    let full = state::load_state(&data_dir()).unwrap();
    let scoped = state::load_state_with_scope(
        &data_dir(),
        &CatalogScope {
            include_providers: HashSet::from(["openai".to_string()]),
            ..CatalogScope::default()
        },
    )
    .unwrap();

    assert_ne!(full.etag, scoped.etag);
}

#[tokio::test]
async fn aliases_returns_sorted_aliases() {
    let json = get_json("/api/v1/aliases").await;
    assert_success_envelope(&json);
    let aliases = json["data"].as_array().unwrap();
    assert!(!aliases.is_empty());
    assert!(
        aliases.windows(2).all(|pair| {
            pair[0]["alias"].as_str().unwrap() <= pair[1]["alias"].as_str().unwrap()
        })
    );
    assert!(aliases.iter().any(|alias| {
        alias["alias"].as_str() == Some("claude-opus-4")
            && alias["canonical_id"].as_str() == Some("claude-opus-4-7")
    }));
}

#[tokio::test]
async fn alias_detail_resolves_alias() {
    let json = get_json("/api/v1/aliases/claude-opus-4").await;
    assert_success_envelope(&json);
    assert_eq!(json["data"]["alias"], "claude-opus-4");
    assert_eq!(json["data"]["canonical_id"], "claude-opus-4-7");
    assert_eq!(json["data"]["provider"], "anthropic");
}

#[tokio::test]
async fn alias_detail_unknown_returns_not_found() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/aliases/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_error_envelope(&json, "ALIAS_NOT_FOUND");
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
    assert!(json["paths"]["/api/v1/models"].is_object());
    assert!(json["paths"]["/api/v1/providers/{id}"].is_object());
    assert!(json["paths"]["/api/v1/validate"].is_object());
    assert!(json["paths"]["/api/v1/catalog/health"].is_object());
    assert!(json["paths"]["/api/v1/catalog/issues"].is_object());
    assert!(json["paths"]["/api/v1/compare"].is_object());
    assert!(json["paths"]["/api/v1/search"].is_object());
    assert!(json["paths"]["/api/v1/aliases"].is_object());
    assert!(json["paths"]["/api/v1/aliases/{alias}"].is_object());
    assert!(json["paths"]["/v1/models"].is_null());
    assert!(
        json["paths"]["/api/v1/health"]["get"]["responses"]["200"]["content"]
            ["application/json"]["examples"]
            .is_object()
    );
    assert!(
        json["paths"]["/api/v1/validate"]["post"]["responses"]["200"]["content"]
            ["application/json"]["examples"]
            .is_object()
    );
    assert!(
        json["paths"]["/api/v1/suggest"]["get"]["responses"]["200"]["content"]["application/json"]
            ["examples"]
            .is_object()
    );
    let search_params = json["paths"]["/api/v1/search"]["get"]["parameters"]
        .as_array()
        .unwrap();
    assert!(
        search_params
            .iter()
            .any(|param| param["name"] == "q" && param["example"] == "gpt")
    );
    assert!(
        search_params
            .iter()
            .any(|param| param["name"] == "provider" && param["example"] == "openai")
    );

    let compare_params = json["paths"]["/api/v1/compare"]["get"]["parameters"]
        .as_array()
        .unwrap();
    assert!(compare_params.iter().any(|param| {
        param["name"] == "models" && param["example"] == "openai/gpt-4o,anthropic/claude-opus-4-7"
    }));
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
                .uri("/api/v1/models")
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
                .uri("/api/v1/models?provider=&status=&capability=&limit=0&offset=0&sort=&order=&modality_input=&modality_output=")
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
                .uri("/api/v1/models?limit=not-a-number")
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
                    .uri(format!("/api/v1/models?{query}"))
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
                .uri("/api/v1/models?provider=anthropic")
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
                .uri("/api/v1/models?status=active")
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
                .uri("/api/v1/models?capability=vision")
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
                .uri("/api/v1/models?limit=3&offset=2&sort=id&order=desc")
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
    let json = get_json("/api/v1/models?provider=openai&limit=10&offset=10000").await;

    assert_success_envelope(&json);
    let total = json["meta"]["total_data"].as_u64().unwrap();
    assert!(total > 0);
    assert_eq!(json["data"].as_array().unwrap().len(), 0);
    assert_eq!(json["meta"]["offset"].as_u64().unwrap(), total);
    // meta.limit reflects the requested limit, not the clamped result count
    assert_eq!(json["meta"]["limit"], 10);
}

#[tokio::test]
async fn models_large_limit_reports_actual_remaining_window() {
    let all = get_json("/api/v1/models?provider=openai&limit=1000").await;
    let total = all["meta"]["total_data"].as_u64().unwrap();
    assert!(total > 2);

    let offset = total - 2;
    let json = get_json(&format!(
        "/api/v1/models?provider=openai&limit=1000&offset={offset}"
    ))
    .await;

    assert_success_envelope(&json);
    assert_eq!(json["data"].as_array().unwrap().len(), 2);
    assert_eq!(json["meta"]["total_data"].as_u64().unwrap(), total);
    assert_eq!(json["meta"]["offset"].as_u64().unwrap(), offset);
    // meta.limit reflects the requested limit, not the remaining item count
    assert_eq!(json["meta"]["limit"], 1000);
}

#[tokio::test]
async fn models_filtered_total_data_counts_matches_before_pagination() {
    let all_openai = get_json("/api/v1/models?provider=openai&limit=1000").await;
    let total = all_openai["data"].as_array().unwrap().len() as u64;
    assert!(total > 3);

    let page = get_json("/api/v1/models?provider=openai&limit=3&offset=1").await;

    assert_success_envelope(&page);
    assert_eq!(page["data"].as_array().unwrap().len(), 3);
    assert_eq!(page["meta"]["total_data"].as_u64().unwrap(), total);
    assert_eq!(page["meta"]["limit"], 3);
    assert_eq!(page["meta"]["offset"], 1);
}

#[tokio::test]
async fn models_unknown_sort_falls_back_to_provider_then_id() {
    let json = get_json("/api/v1/models?limit=10&sort=unknown").await;

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
    let json = get_json("/api/v1/models?limit=10&sort=id&order=sideways").await;

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
                .uri("/api/v1/models?modality_input=image&modality_output=text&min_context=100000&max_input_price=1")
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
                .uri("/api/v1/models/anthropic/claude-opus-4-7")
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
                .uri("/api/v1/models/openai/does-not-exist")
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
                .uri("/api/v1/models/openai/gpt-4o")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let etag = response.headers().get("etag").unwrap().clone();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/models/openai/gpt-4o")
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
async fn compare_returns_models_in_requested_order() {
    let json = get_json("/api/v1/compare?models=openai/gpt-4o,anthropic/claude-opus-4-7").await;
    assert_success_envelope(&json);

    let models = json["data"]["models"].as_array().unwrap();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0]["provider"], "openai");
    assert_eq!(models[0]["id"], "gpt-4o");
    assert_eq!(models[1]["provider"], "anthropic");
    assert_eq!(models[1]["id"], "claude-opus-4-7");
    assert!(
        json["data"]["summary"]["max_context_window"]
            .as_u64()
            .is_some()
    );
    assert!(
        json["data"]["summary"]["max_output_tokens"]
            .as_u64()
            .is_some()
    );
}

#[tokio::test]
async fn compare_requires_two_unique_models() {
    for uri in [
        "/api/v1/compare",
        "/api/v1/compare?models=openai/gpt-4o",
        "/api/v1/compare?models=openai/gpt-4o,openai/gpt-4o",
    ] {
        let response = app()
            .await
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_error_envelope(&json, "BAD_REQUEST");
    }
}

#[tokio::test]
async fn compare_rejects_more_than_five_models() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/compare?models=openai/gpt-4o,openai/gpt-4.1,openai/gpt-4.1-mini,openai/gpt-4.1-nano,anthropic/claude-opus-4-7,anthropic/claude-sonnet-4-6")
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
    assert_error_envelope(&json, "BAD_REQUEST");
}

#[tokio::test]
async fn compare_unknown_model_returns_not_found() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/compare?models=openai/gpt-4o,openai/does-not-exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_error_envelope(&json, "MODEL_NOT_FOUND");
}

#[tokio::test]
async fn validate_known_active_model() {
    let body = serde_json::json!({"model": "claude-opus-4-7"});
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/validate")
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
                .uri("/api/v1/suggest?q=claude-opus-4.7")
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
    assert!(results.iter().all(|result| {
        result["provider"].as_str().is_some()
            && result["matched"].as_str().is_some()
            && result["match_type"].as_str().is_some()
            && result["score"].as_f64().is_some()
    }));
    if results.len() > 1 {
        assert!(results[0]["score"].as_f64() >= results[1]["score"].as_f64());
    }
}

#[tokio::test]
async fn suggest_exact_alias_returns_canonical_match_metadata() {
    let json = get_json("/api/v1/suggest?q=claude-opus-4").await;
    assert_success_envelope(&json);

    let result = &json["data"].as_array().unwrap()[0];
    assert_eq!(result["id"], "claude-opus-4-7");
    assert_eq!(result["provider"], "anthropic");
    assert_eq!(result["matched"], "claude-opus-4");
    assert_eq!(result["match_type"], "alias");
    assert_eq!(result["score"], 1.0);
}

#[tokio::test]
async fn suggest_provider_filter_limits_results_to_provider() {
    let json = get_json("/api/v1/suggest?q=claude-sonnet&provider=anthropic&limit=10").await;
    assert_success_envelope(&json);

    let results = json["data"].as_array().unwrap();
    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .all(|result| result["provider"].as_str() == Some("anthropic"))
    );
}

#[tokio::test]
async fn suggest_can_match_display_name() {
    let json = get_json("/api/v1/suggest?q=Claude%20Opus%204.7").await;
    assert_success_envelope(&json);

    let results = json["data"].as_array().unwrap();
    assert!(results.iter().any(|result| {
        result["id"].as_str() == Some("claude-opus-4-7")
            && result["matched"].as_str() == Some("Claude Opus 4.7")
            && result["match_type"].as_str() == Some("display_name")
    }));
}

#[tokio::test]
async fn suggest_missing_query_returns_error_envelope() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/suggest")
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
                .uri("/api/v1/suggest?q=zzzzzzzzz")
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
                .uri("/api/v1/suggest?q=gpt")
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

#[tokio::test]
async fn suggest_limit_can_return_more_than_default() {
    let default = get_json("/api/v1/suggest?q=gpt").await;
    let expanded = get_json("/api/v1/suggest?q=gpt&limit=10").await;

    assert_success_envelope(&default);
    assert_success_envelope(&expanded);

    let default_len = default["data"].as_array().unwrap().len();
    let expanded_len = expanded["data"].as_array().unwrap().len();
    assert!(default_len <= 5);
    assert!(expanded_len > default_len);
    assert!(expanded_len <= 10);
}

#[tokio::test]
async fn suggest_invalid_limit_returns_error_envelope() {
    let response = app()
        .await
        .oneshot(
            Request::builder()
                .uri("/api/v1/suggest?q=gpt&limit=bad")
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
    assert_error_envelope(&json, "BAD_REQUEST");
}

#[tokio::test]
async fn search_matches_alias_with_metadata() {
    let json = get_json("/api/v1/search?q=claude-opus-4").await;
    assert_success_envelope(&json);

    let result = &json["data"].as_array().unwrap()[0];
    assert_eq!(result["model"]["id"], "claude-opus-4-7");
    assert_eq!(result["model"]["provider"], "anthropic");
    assert_eq!(result["matched"], "claude-opus-4");
    assert_eq!(result["match_type"], "alias");
    assert_eq!(result["score"], 1.0);
}

#[tokio::test]
async fn search_without_query_behaves_like_filtered_models() {
    let json = get_json("/api/v1/search?provider=openai&limit=3").await;
    assert_success_envelope(&json);

    let results = json["data"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|result| {
        result["model"]["provider"].as_str() == Some("openai")
            && result["score"].is_null()
            && result["matched"].is_null()
            && result["match_type"].is_null()
    }));
}

#[tokio::test]
async fn search_supports_pagination_and_provider_filter() {
    let json = get_json("/api/v1/search?q=gpt&provider=openai&limit=2&offset=1").await;
    assert_success_envelope(&json);

    let results = json["data"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|result| {
        result["model"]["provider"].as_str() == Some("openai") && result["score"].as_f64().is_some()
    }));
    assert_eq!(json["meta"]["limit"], 2);
    assert_eq!(json["meta"]["offset"], 1);
}
