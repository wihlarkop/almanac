use axum::{extract::MatchedPath, extract::Request, middleware::Next, response::Response};
use prometheus::{
    CounterVec, Encoder, HistogramOpts, HistogramVec, IntGauge, Opts, TextEncoder,
    register_counter_vec, register_histogram_vec, register_int_gauge,
};
use std::sync::OnceLock;

pub struct HttpMetrics {
    pub requests_total: CounterVec,
    pub request_duration_seconds: HistogramVec,
}

pub struct CatalogMetrics {
    pub models: IntGauge,
    pub providers: IntGauge,
    pub aliases: IntGauge,
}

static HTTP: OnceLock<HttpMetrics> = OnceLock::new();
static CATALOG: OnceLock<CatalogMetrics> = OnceLock::new();

pub fn init() {
    HTTP.get_or_init(|| HttpMetrics {
        requests_total: register_counter_vec!(
            Opts::new(
                "http_requests_total",
                "Total HTTP requests by method, path, and status"
            ),
            &["method", "path", "status"]
        )
        .expect("register http_requests_total"),
        request_duration_seconds: register_histogram_vec!(
            HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds"
            )
            .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5]),
            &["method", "path"]
        )
        .expect("register http_request_duration_seconds"),
    });
    CATALOG.get_or_init(|| CatalogMetrics {
        models: register_int_gauge!("catalog_models_total", "Total models in the catalog")
            .expect("register catalog_models_total"),
        providers: register_int_gauge!("catalog_providers_total", "Total providers in the catalog")
            .expect("register catalog_providers_total"),
        aliases: register_int_gauge!("catalog_aliases_total", "Total aliases in the catalog")
            .expect("register catalog_aliases_total"),
    });
}

pub fn set_catalog_counts(models: usize, providers: usize, aliases: usize) {
    if let Some(c) = CATALOG.get() {
        c.models.set(models as i64);
        c.providers.set(providers as i64);
        c.aliases.set(aliases as i64);
    }
}

pub fn render() -> String {
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    encoder
        .encode(&prometheus::gather(), &mut buf)
        .unwrap_or_default();
    String::from_utf8(buf).unwrap_or_default()
}

pub async fn record_request(req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    if let Some(m) = HTTP.get() {
        m.requests_total
            .with_label_values(&[&method, &path, &status])
            .inc();
        m.request_duration_seconds
            .with_label_values(&[&method, &path])
            .observe(elapsed);
    }

    response
}
