use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use axum::http::{HeaderMap, HeaderValue};

pub fn catalog_headers(etag: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        "cache-control",
        HeaderValue::from_static("public, max-age=300"),
    );
    if let Ok(etag) = HeaderValue::from_str(etag) {
        headers.insert("etag", etag);
    }
    headers
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct EmptyData {}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
    pub meta: Meta,
    pub error: Option<ApiErrorBody>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct Meta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_data: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_time_seconds: Option<f64>,
    pub timestamp: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ApiErrorBody {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Object)]
    pub details: Option<serde_json::Value>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            message: "OK".to_string(),
            data: Some(data),
            meta: Meta::new(),
            error: None,
        }
    }

    pub fn paginated(data: T, limit: usize, offset: usize, total_data: usize) -> Self {
        Self {
            success: true,
            message: "OK".to_string(),
            data: Some(data),
            meta: Meta::paginated(limit, offset, total_data),
            error: None,
        }
    }
}

impl ApiResponse<()> {
    pub fn error(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self::error_with_details(message, code, None)
    }

    pub fn error_with_details(
        message: impl Into<String>,
        code: impl Into<String>,
        details: Option<serde_json::Value>,
    ) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            meta: Meta::new(),
            error: Some(ApiErrorBody {
                code: code.into(),
                details,
            }),
        }
    }
}

impl Meta {
    pub fn new() -> Self {
        Self {
            limit: None,
            offset: None,
            total_data: None,
            execution_time_seconds: None,
            timestamp: timestamp(),
        }
    }

    pub fn paginated(limit: usize, offset: usize, total_data: usize) -> Self {
        Self {
            limit: Some(limit),
            offset: Some(offset),
            total_data: Some(total_data),
            execution_time_seconds: None,
            timestamp: timestamp(),
        }
    }
}

fn timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
