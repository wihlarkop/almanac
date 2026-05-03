use crate::{
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
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Serialize, utoipa::ToSchema)]
pub struct AliasMapping {
    alias: String,
    canonical_id: String,
    provider: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/aliases",
    responses(
        (
            status = 200,
            description = "Alias mappings",
            body = ApiResponse<Vec<AliasMapping>>,
            examples(
                ("aliases" = (
                    summary = "Alias list",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": [
                            {
                                "alias": "claude-opus-4",
                                "canonical_id": "claude-opus-4-7",
                                "provider": "anthropic"
                            }
                        ],
                        "meta": { "timestamp": "2026-05-04T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 304, description = "Catalog not modified")
    )
)]
pub async fn list_aliases(
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

    let mut aliases = state
        .aliases
        .iter()
        .map(|(alias, canonical_id)| alias_mapping(&state, alias, canonical_id))
        .collect::<Vec<_>>();
    aliases.sort_by(|left, right| left.alias.cmp(&right.alias));

    (
        catalog_headers(&state.etag),
        Json(ApiResponse::ok_with_context(aliases, &context)),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/aliases/{alias}",
    params(("alias" = String, Path, description = "Alias", example = "claude-opus-4")),
    responses(
        (
            status = 200,
            description = "Alias mapping",
            body = ApiResponse<AliasMapping>,
            examples(
                ("alias" = (
                    summary = "Alias detail",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": {
                            "alias": "claude-opus-4",
                            "canonical_id": "claude-opus-4-7",
                            "provider": "anthropic"
                        },
                        "meta": { "timestamp": "2026-05-04T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 304, description = "Catalog not modified"),
        (status = 404, description = "Alias not found", body = ApiResponse<crate::response::EmptyData>)
    )
)]
pub async fn get_alias(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    Path(alias): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let state = state.read().await;
    let Some(canonical_id) = state.aliases.get(&alias) else {
        return ApiError::AliasNotFound { alias }.into_response();
    };

    if let Some(inm) = headers.get("if-none-match")
        && inm.as_bytes() == state.etag.as_bytes()
    {
        return StatusCode::NOT_MODIFIED.into_response();
    }

    (
        catalog_headers(&state.etag),
        Json(ApiResponse::ok_with_context(
            alias_mapping(&state, &alias, canonical_id),
            &context,
        )),
    )
        .into_response()
}

fn alias_mapping(state: &AppState, alias: &str, canonical_id: &str) -> AliasMapping {
    let provider = state
        .models_by_id
        .get(canonical_id)
        .and_then(|index| state.models.get(*index))
        .map(|model| model.provider.clone());

    AliasMapping {
        alias: alias.to_string(),
        canonical_id: canonical_id.to_string(),
        provider,
    }
}
