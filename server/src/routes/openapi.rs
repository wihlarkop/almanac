use axum::{
    http::{header, HeaderMap, HeaderValue},
    response::{Html, IntoResponse, Json},
};

pub async fn openapi_json() -> impl IntoResponse {
    Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Almanac API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Model catalog, validation, suggestions, and provider metadata for LLM developers."
        },
        "paths": {
            "/v1/health": {
                "get": {
                    "summary": "Health check",
                    "responses": {
                        "200": { "description": "Server is healthy" }
                    }
                }
            },
            "/v1/providers": {
                "get": {
                    "summary": "List providers",
                    "responses": {
                        "200": { "description": "Provider list" }
                    }
                }
            },
            "/v1/models": {
                "get": {
                    "summary": "List and filter models",
                    "parameters": [
                        { "name": "provider", "in": "query", "schema": { "type": "string" } },
                        { "name": "status", "in": "query", "schema": { "type": "string" } },
                        { "name": "capability", "in": "query", "schema": { "type": "string" } },
                        { "name": "modality_input", "in": "query", "schema": { "type": "string" } },
                        { "name": "modality_output", "in": "query", "schema": { "type": "string" } },
                        { "name": "min_context", "in": "query", "schema": { "type": "integer", "minimum": 1 } },
                        { "name": "max_input_price", "in": "query", "schema": { "type": "number", "minimum": 0 } },
                        { "name": "limit", "in": "query", "schema": { "type": "integer", "minimum": 0 } },
                        { "name": "offset", "in": "query", "schema": { "type": "integer", "minimum": 0 } },
                        { "name": "sort", "in": "query", "schema": { "type": "string", "enum": ["id", "provider", "status", "context_window", "max_output_tokens"] } },
                        { "name": "order", "in": "query", "schema": { "type": "string", "enum": ["asc", "desc"] } }
                    ],
                    "responses": {
                        "200": { "description": "Paginated model list" },
                        "304": { "description": "Catalog not modified" }
                    }
                }
            },
            "/v1/models/{provider}/{id}": {
                "get": {
                    "summary": "Get one model",
                    "parameters": [
                        { "name": "provider", "in": "path", "required": true, "schema": { "type": "string" } },
                        { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": {
                        "200": { "description": "Model metadata" },
                        "304": { "description": "Catalog not modified" },
                        "404": { "description": "Model not found" }
                    }
                }
            },
            "/v1/validate": {
                "post": {
                    "summary": "Validate model and request compatibility",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ValidateRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Validation result" }
                    }
                }
            },
            "/v1/suggest": {
                "get": {
                    "summary": "Suggest likely model IDs",
                    "parameters": [
                        { "name": "q", "in": "query", "required": true, "schema": { "type": "string" } }
                    ],
                    "responses": {
                        "200": { "description": "Ranked suggestions" }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "ValidateRequest": {
                    "type": "object",
                    "required": ["model"],
                    "properties": {
                        "model": { "type": "string" },
                        "provider": { "type": "string" },
                        "parameters": {
                            "type": "object",
                            "additionalProperties": true
                        },
                        "modalities": {
                            "type": "object",
                            "properties": {
                                "input": { "type": "array", "items": { "type": "string" } },
                                "output": { "type": "array", "items": { "type": "string" } }
                            }
                        }
                    }
                }
            }
        }
    }))
}

pub async fn swagger_ui() -> impl IntoResponse {
    let html = r##"<!doctype html>
<html>
  <head>
    <title>Almanac API - Swagger UI</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist/swagger-ui.css" />
  </head>
  <body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist/swagger-ui-bundle.js"></script>
    <script>
      window.ui = SwaggerUIBundle({ url: "/openapi.json", dom_id: "#swagger-ui" });
    </script>
  </body>
</html>"##;

    html_response(html)
}

pub async fn scalar() -> impl IntoResponse {
    let html = r#"<!doctype html>
<html>
  <head>
    <title>Almanac API - Scalar</title>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
  </head>
  <body>
    <script
      id="api-reference"
      data-url="/openapi.json"
      data-theme="default"
      src="https://cdn.jsdelivr.net/npm/@scalar/api-reference">
    </script>
  </body>
</html>"#;

    html_response(html)
}

fn html_response(html: &'static str) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    (headers, Html(html))
}
