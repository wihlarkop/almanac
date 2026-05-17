use crate::{
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
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_LIMIT: usize = 100;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct AliasFilter {
    /// Maximum number of results to return
    #[param(example = 100)]
    pub limit: Option<usize>,
    /// Number of results to skip for pagination
    #[param(example = 0)]
    pub offset: Option<usize>,
}

#[derive(Clone, Serialize, utoipa::ToSchema)]
pub struct AliasMapping {
    alias: String,
    canonical_id: String,
    provider: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/aliases",
    tag = "Catalog",
    operation_id = "list_aliases",
    summary = "List aliases",
    description = "Paginated list of model alias mappings with ETag support.",
    params(AliasFilter),
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
                        "meta": { "limit": 100, "offset": 0, "total_data": 1, "timestamp": "2026-05-04T00:00:00Z" },
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
    query: Result<Query<AliasFilter>, QueryRejection>,
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

    let mut all_aliases = state
        .aliases
        .iter()
        .map(|(alias, canonical_id)| alias_mapping(&state, alias, canonical_id))
        .collect::<Vec<_>>();
    all_aliases.sort_by(|left, right| left.alias.cmp(&right.alias));

    let total = all_aliases.len();
    let offset = filter.offset.unwrap_or(0).min(total);
    let limit = filter.limit.filter(|l| *l > 0).unwrap_or(DEFAULT_LIMIT);
    let data: Vec<AliasMapping> = all_aliases.into_iter().skip(offset).take(limit).collect();

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
    path = "/api/v1/aliases/{alias}",
    tag = "Catalog",
    operation_id = "get_alias",
    summary = "Get alias",
    description = "Resolves a single alias to its canonical model id.",
    params(("alias" = String, Path, description = "Alias string", example = "claude-opus-4")),
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
        (
            status = 404,
            description = "Alias not found",
            body = ApiResponse<crate::response::EmptyData>,
            examples(
                ("error" = (
                    summary = "Alias not found",
                    value = json!({
                        "success": false,
                        "message": "alias not found",
                        "data": null,
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": { "code": "ALIAS_NOT_FOUND", "details": { "alias": "unknown-alias" } }
                    })
                ))
            )
        )
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
