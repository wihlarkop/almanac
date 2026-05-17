use crate::response::ApiResponse;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("model not found")]
    ModelNotFound { provider: String, id: String },
    #[error("provider not found")]
    ProviderNotFound { provider: String },
    #[error("alias not found")]
    AliasNotFound { alias: String },
    #[error("{message}")]
    BadRequest { message: String },
    #[error("{message}")]
    NotFound { message: String },
    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

impl ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ModelNotFound { .. } => StatusCode::NOT_FOUND,
            Self::ProviderNotFound { .. } => StatusCode::NOT_FOUND,
            Self::AliasNotFound { .. } => StatusCode::NOT_FOUND,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::ModelNotFound { .. } => "MODEL_NOT_FOUND",
            Self::ProviderNotFound { .. } => "PROVIDER_NOT_FOUND",
            Self::AliasNotFound { .. } => "ALIAS_NOT_FOUND",
            Self::BadRequest { .. } => "BAD_REQUEST",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Internal(_) => "INTERNAL_SERVER_ERROR",
        }
    }

    fn details(&self) -> Option<serde_json::Value> {
        match self {
            Self::ModelNotFound { provider, id } => Some(serde_json::json!({
                "provider": provider,
                "id": id
            })),
            Self::ProviderNotFound { provider } => Some(serde_json::json!({
                "provider": provider
            })),
            Self::AliasNotFound { alias } => Some(serde_json::json!({
                "alias": alias
            })),
            Self::BadRequest { .. } | Self::NotFound { .. } | Self::Internal(_) => None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        if let Self::Internal(ref err) = self {
            tracing::error!(error = %err, "internal server error");
        }
        let body =
            ApiResponse::error_with_details(self.to_string(), self.error_code(), self.details());

        (status, Json(body)).into_response()
    }
}
