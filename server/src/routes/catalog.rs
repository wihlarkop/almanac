use crate::{
    catalog::Model,
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
use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};
use time::{Date, Month, OffsetDateTime};
use tokio::sync::RwLock;

const STALE_AFTER_DAYS: i64 = 90;

#[derive(Serialize, utoipa::ToSchema)]
pub struct CatalogHealth {
    pub total_models: usize,
    pub total_providers: usize,
    pub total_aliases: usize,
    pub status_counts: BTreeMap<String, usize>,
    pub missing_pricing_count: usize,
    pub stale_verification_count: usize,
    pub oldest_last_verified: Option<String>,
    pub newest_last_verified: Option<String>,
    pub etag: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct CatalogIssues {
    stale_models: Vec<ModelIssue>,
    missing_pricing_models: Vec<ModelIssue>,
    deprecated_models: Vec<ModelIssue>,
    retired_models: Vec<ModelIssue>,
    replacement_gaps: Vec<ModelIssue>,
}

#[derive(Clone, Serialize, utoipa::ToSchema)]
pub struct ModelIssue {
    provider: String,
    id: String,
    display_name: String,
    status: String,
    last_verified: String,
    replacement: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/catalog/health",
    responses(
        (
            status = 200,
            description = "Catalog health summary",
            body = ApiResponse<CatalogHealth>,
            examples(
                ("summary" = (
                    summary = "Catalog health",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": {
                            "total_models": 42,
                            "total_providers": 5,
                            "total_aliases": 12,
                            "status_counts": { "active": 35, "deprecated": 7 },
                            "missing_pricing_count": 1,
                            "stale_verification_count": 3,
                            "oldest_last_verified": "2026-02-01",
                            "newest_last_verified": "2026-05-02",
                            "etag": "\"catalog-example\""
                        },
                        "meta": { "timestamp": "2026-05-03T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 304, description = "Catalog not modified")
    )
)]
pub async fn health(
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

    let today = OffsetDateTime::now_utc().date();
    let stale_before = today - time::Duration::days(STALE_AFTER_DAYS);
    let mut status_counts = BTreeMap::new();
    let mut missing_pricing_count = 0;
    let mut stale_verification_count = 0;
    let mut oldest_last_verified: Option<Date> = None;
    let mut newest_last_verified: Option<Date> = None;

    for model in &state.models {
        *status_counts
            .entry(model.status.as_str().to_string())
            .or_insert(0) += 1;
        if model.pricing.is_none() {
            missing_pricing_count += 1;
        }
        if let Some(last_verified) = parse_catalog_date(&model.last_verified) {
            if last_verified < stale_before {
                stale_verification_count += 1;
            }
            oldest_last_verified = Some(
                oldest_last_verified
                    .map(|date| date.min(last_verified))
                    .unwrap_or(last_verified),
            );
            newest_last_verified = Some(
                newest_last_verified
                    .map(|date| date.max(last_verified))
                    .unwrap_or(last_verified),
            );
        } else {
            stale_verification_count += 1;
        }
    }

    (
        catalog_headers(&state.etag),
        Json(ApiResponse::ok_with_context(
            CatalogHealth {
                total_models: state.models.len(),
                total_providers: state.providers.len(),
                total_aliases: state.aliases.len(),
                status_counts,
                missing_pricing_count,
                stale_verification_count,
                oldest_last_verified: oldest_last_verified.map(|date| date.to_string()),
                newest_last_verified: newest_last_verified.map(|date| date.to_string()),
                etag: state.etag.clone(),
            },
            &context,
        )),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/api/v1/catalog/issues",
    responses(
        (
            status = 200,
            description = "Catalog issue details",
            body = ApiResponse<CatalogIssues>,
            examples(
                ("issues" = (
                    summary = "Catalog issues",
                    value = json!({
                        "success": true,
                        "message": "OK",
                        "data": {
                            "stale_models": [],
                            "missing_pricing_models": [],
                            "deprecated_models": [],
                            "retired_models": [],
                            "replacement_gaps": []
                        },
                        "meta": { "timestamp": "2026-05-04T00:00:00Z" },
                        "error": null
                    })
                ))
            )
        ),
        (status = 304, description = "Catalog not modified")
    )
)]
pub async fn issues(
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

    let today = OffsetDateTime::now_utc().date();
    let stale_before = today - time::Duration::days(STALE_AFTER_DAYS);

    let mut stale_models = Vec::new();
    let mut missing_pricing_models = Vec::new();
    let mut deprecated_models = Vec::new();
    let mut retired_models = Vec::new();
    let mut replacement_gaps = Vec::new();

    for model in &state.models {
        let issue = model_issue(model);

        if parse_catalog_date(&model.last_verified)
            .map(|last_verified| last_verified < stale_before)
            .unwrap_or(true)
        {
            stale_models.push(issue.clone());
        }
        if model.pricing.is_none() {
            missing_pricing_models.push(issue.clone());
        }
        if model.status.as_str() == "deprecated" {
            deprecated_models.push(issue.clone());
        }
        if model.status.as_str() == "retired" {
            retired_models.push(issue.clone());
        }
        if matches!(model.status.as_str(), "deprecated" | "retired") && model.replacement.is_none()
        {
            replacement_gaps.push(issue);
        }
    }

    let data = CatalogIssues {
        stale_models,
        missing_pricing_models,
        deprecated_models,
        retired_models,
        replacement_gaps,
    };

    (
        catalog_headers(&state.etag),
        Json(ApiResponse::ok_with_context(data, &context)),
    )
        .into_response()
}

fn model_issue(model: &Model) -> ModelIssue {
    ModelIssue {
        provider: model.provider.clone(),
        id: model.id.clone(),
        display_name: model.display_name.clone(),
        status: model.status.as_str().to_string(),
        last_verified: model.last_verified.clone(),
        replacement: model.replacement.clone(),
    }
}

fn parse_catalog_date(value: &str) -> Option<Date> {
    let mut parts = value.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u8 = parts.next()?.parse().ok()?;
    let day: u8 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }

    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}
