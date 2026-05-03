use crate::{
    catalog::Provider,
    error::ApiError,
    request::RequestContext,
    response::{ApiResponse, catalog_headers},
    state::AppState,
};
use axum::{
    Extension,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[utoipa::path(
    get,
    path = "/api/v1/providers",
    responses(
        (
            status = 200,
            description = "Provider list",
            body = ApiResponse<Vec<Provider>>,
            examples(
                ("providers" = (
                    summary = "Provider list",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": [
                            {
                                "id": "openai",
                                "display_name": "OpenAI",
                                "website": "https://openai.com",
                                "api_docs": "https://platform.openai.com/docs"
                            }
                        ],
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 304, description = "Catalog not modified")
    )
)]
pub async fn list_providers(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let state = state.read().await;

    if let Some(inm) = headers.get("if-none-match")
        && inm.as_bytes() == state.etag.as_bytes()
    {
        return StatusCode::NOT_MODIFIED.into_response();
    }

    (
        catalog_headers(&state.etag),
        Json(ApiResponse::ok_with_context(
            state.providers.clone(),
            &context,
        )),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/providers/{id}",
    params(("id" = String, Path, description = "Provider id", example = "openai")),
    responses(
        (
            status = 200,
            description = "Provider metadata",
            body = ApiResponse<Provider>,
            examples(
                ("provider" = (
                    summary = "Provider detail",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": {
                            "id": "openai",
                            "display_name": "OpenAI",
                            "website": "https://openai.com",
                            "api_docs": "https://platform.openai.com/docs"
                        },
                        "meta": { "timestamp": "2026-05-04T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 304, description = "Catalog not modified"),
        (status = 404, description = "Provider not found", body = ApiResponse<crate::response::EmptyData>)
    )
)]
pub async fn get_provider(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let state = state.read().await;

    let provider = state.providers.iter().find(|provider| provider.id == id);
    match provider {
        None => ApiError::ProviderNotFound { provider: id }.into_response(),
        Some(provider) => {
            if let Some(inm) = headers.get("if-none-match")
                && inm.as_bytes() == state.etag.as_bytes()
            {
                return StatusCode::NOT_MODIFIED.into_response();
            }
            (
                catalog_headers(&state.etag),
                Json(ApiResponse::ok_with_context(provider.clone(), &context)),
            )
                .into_response()
        }
    }
}
