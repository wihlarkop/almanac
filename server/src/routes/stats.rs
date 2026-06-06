use crate::{
    catalog::CatalogStats, request::RequestContext, response::ApiResponse, state::AppState,
};
use axum::response::Json;
use axum::{Extension, extract::State};
use std::sync::Arc;
use tokio::sync::RwLock;

#[utoipa::path(
    get,
    path = "/api/v1/stats",
    tag = "Catalog",
    operation_id = "catalog_stats",
    summary = "Catalog statistics",
    description = "Returns pre-computed statistics about the loaded model catalog.",
    responses((
        status = 200,
        description = "Catalog statistics",
        body = ApiResponse<CatalogStats>,
        examples(
            ("ok" = (
                summary = "Catalog stats",
                value = json!({
                    "success": true,
                    "message": "OK",
                    "data": {
                        "total_models": 300,
                        "total_providers": 30,
                        "models_by_status": { "active": 250, "deprecated": 50 },
                        "models_by_provider": { "openai": 20 },
                        "models_by_endpoint_family": { "chat_completions": 200 },
                        "models_by_input_modality": { "text": 300 },
                        "models_by_output_modality": { "text": 300 },
                        "free_models": 5,
                        "models_without_pricing": 10,
                        "cheapest_input": { "model_id": "gpt-4o-mini", "provider": "openai", "price": 0.00015 },
                        "most_expensive_input": null,
                        "cheapest_output": null,
                        "most_expensive_output": null,
                        "last_updated": "2026-06-06T00:00:00Z"
                    },
                    "meta": { "timestamp": "2026-06-06T00:00:00Z" },
                    "error": null
                })
            ))
        )
    ))
)]
pub async fn catalog_stats(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
) -> Json<ApiResponse<CatalogStats>> {
    let state = state.read().await;
    Json(ApiResponse::ok_with_context(state.stats.clone(), &context))
}
