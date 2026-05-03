use crate::{
    catalog::Provider,
    request::RequestContext,
    response::{ApiResponse, catalog_headers},
    state::AppState,
};
use axum::{
    Extension,
    extract::State,
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
