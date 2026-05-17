use crate::{
    error::ApiError,
    request::{
        MAX_REQUEST_BODY_BYTES, attach_request_context, enforce_request_timeout,
        handle_method_not_allowed, reject_oversized_payload,
    },
    state::AppState,
};
use axum::{
    Router,
    http::{Method, header},
    middleware,
    response::IntoResponse,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
};
use utoipa::{OpenApi as DeriveOpenApi, openapi::OpenApi};
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_scalar::{Scalar, Servable};
use utoipa_swagger_ui::SwaggerUi;

mod aliases;
mod catalog;
mod compare;
mod health;
#[cfg(feature = "metrics")]
mod metrics_route;
mod models;
mod providers;
mod root;
mod search;
mod suggest;
mod validate;

#[derive(DeriveOpenApi)]
#[openapi(
    info(
        title = "Almanac API",
        version = env!("CARGO_PKG_VERSION"),
        description = "Model catalog, validation, suggestions, and provider metadata for LLM developers."
    ),
    tags(
        (name = "Catalog", description = "Models, providers, aliases, and comparisons"),
        (name = "Discovery", description = "Fuzzy search and model suggestions"),
        (name = "Validation", description = "Model and parameter validation"),
        (name = "Observability", description = "Server and catalog health"),
        (name = "Root", description = "API landing")
    )
)]
struct ApiDoc;

pub fn router(state: Arc<RwLock<AppState>>) -> Router {
    let (api_router, openapi) = api_router().split_for_parts();

    #[allow(unused_mut)]
    let mut r = api_router
        .merge(docs_router(openapi))
        .fallback(not_found)
        .with_state(state);

    #[cfg(feature = "metrics")]
    {
        use axum::routing::get;
        r = r.route("/metrics", get(metrics_route::metrics));
        r = r.layer(middleware::from_fn(crate::metrics::record_request));
    }

    // Layer order: last .layer() is outermost (runs first on request, last on response).
    // attach_request_context must be outermost so every response — including those
    // short-circuited by inner middleware — gets an x-request-id header.
    r.layer(CompressionLayer::new())
        .layer(middleware::from_fn(reject_oversized_payload))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::CONTENT_TYPE, header::IF_NONE_MATCH]),
        )
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES as usize))
        .layer(middleware::from_fn(enforce_request_timeout))
        .layer(middleware::map_response(handle_method_not_allowed))
        .layer(middleware::from_fn(attach_request_context))
}

async fn not_found() -> impl IntoResponse {
    ApiError::NotFound {
        message: "route not found".to_string(),
    }
    .into_response()
}

pub fn api_router() -> OpenApiRouter<Arc<RwLock<AppState>>> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(root::root))
        .routes(routes!(health::health))
        .routes(routes!(aliases::list_aliases))
        .routes(routes!(aliases::get_alias))
        .routes(routes!(catalog::health))
        .routes(routes!(catalog::issues))
        .routes(routes!(compare::compare))
        .routes(routes!(providers::list_providers))
        .routes(routes!(providers::get_provider))
        .routes(routes!(models::list_models))
        .routes(routes!(models::get_model))
        .routes(routes!(validate::validate))
        .routes(routes!(search::search))
        .routes(routes!(suggest::suggest))
}

fn docs_router(openapi: OpenApi) -> Router<Arc<RwLock<AppState>>> {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", openapi.clone()))
        .merge(Scalar::with_url("/scalar", openapi))
}
