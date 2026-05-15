use crate::{
    catalog::Provider,
    error::ApiError,
    request::RequestContext,
    response::{ApiResponse, catalog_headers},
    state::AppState,
};
use axum::{
    Extension,
    extract::{Path, Query, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_LIMIT: usize = 50;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ProviderFilter {
    #[param(example = 50)]
    pub limit: Option<usize>,
    #[param(example = 0)]
    pub offset: Option<usize>,
}

#[utoipa::path(
    get,
    path = "/api/v1/providers",
    params(ProviderFilter),
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
                        "meta": { "limit": 50, "offset": 0, "total_data": 1, "timestamp": "2026-05-03T00:00:00Z" },
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
    query: Result<Query<ProviderFilter>, QueryRejection>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Query(filter) = match query {
        Ok(q) => q,
        Err(error) => {
            return ApiError::BadRequest {
                message: error.body_text(),
            }
            .into_response();
        }
    };
    let state = state.read().await;

    if let Some(inm) = headers.get("if-none-match")
        && inm.as_bytes() == state.etag.as_bytes()
    {
        return StatusCode::NOT_MODIFIED.into_response();
    }

    let total = state.providers.len();
    let offset = filter.offset.unwrap_or(0).min(total);
    let limit = filter.limit.filter(|l| *l > 0).unwrap_or(DEFAULT_LIMIT);
    let data: Vec<Provider> = state
        .providers
        .iter()
        .skip(offset)
        .take(limit)
        .cloned()
        .collect();

    (
        catalog_headers(&state.etag),
        Json(ApiResponse::paginated_with_context(
            data, limit, offset, total, &context,
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
