use crate::state::AppState;
use axum::Router;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use utoipa::{OpenApi as DeriveOpenApi, openapi::OpenApi};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};
use utoipa_swagger_ui::SwaggerUi;

mod health;
mod models;
mod providers;
mod suggest;
mod validate;

#[derive(DeriveOpenApi)]
#[openapi(
    info(
        title = "Almanac API",
        version = env!("CARGO_PKG_VERSION"),
        description = "Model catalog, validation, suggestions, and provider metadata for LLM developers."
    )
)]
struct ApiDoc;

pub fn router(state: Arc<RwLock<AppState>>) -> Router {
    let (api_router, openapi) = api_router().split_for_parts();

    api_router
        .merge(docs_router(openapi))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
}

pub fn api_router() -> OpenApiRouter<Arc<RwLock<AppState>>> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health::health))
        .routes(routes!(providers::list_providers))
        .routes(routes!(models::list_models))
        .routes(routes!(models::get_model))
        .routes(routes!(validate::validate))
        .routes(routes!(suggest::suggest))
}

fn docs_router(openapi: OpenApi) -> Router<Arc<RwLock<AppState>>> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", openapi.clone()))
        .merge(Scalar::with_url("/scalar", openapi))
}
