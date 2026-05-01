use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::{fuzzy, state::AppState};

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub model: String,
    pub provider: Option<String>,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub canonical_id: Option<String>,
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Serialize)]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
    pub suggestions: Vec<String>,
}

pub async fn validate(
    State(state): State<Arc<RwLock<AppState>>>,
    Json(req): Json<ValidateRequest>,
) -> Json<ValidateResponse> {
    let state = state.read().await;

    let model = state
        .models
        .iter()
        .find(|m| m["id"].as_str() == Some(req.model.as_str()))
        .or_else(|| {
            state.aliases.get(&req.model).and_then(|canonical| {
                state
                    .models
                    .iter()
                    .find(|m| m["id"].as_str() == Some(canonical.as_str()))
            })
        });

    let mut errors: Vec<ValidationIssue> = Vec::new();
    let mut warnings: Vec<ValidationIssue> = Vec::new();

    let canonical_id = match model {
        None => {
            let suggestions = fuzzy::top_matches(&state, &req.model, 5, 0.7)
                .into_iter()
                .map(|(id, _)| id)
                .collect();
            errors.push(ValidationIssue {
                code: "MODEL_NOT_FOUND".to_string(),
                message: format!("'{}' is not a known model id or alias", req.model),
                suggestions,
            });
            None
        }
        Some(m) => {
            let id = m["id"].as_str().unwrap_or_default().to_string();
            let status = m["status"].as_str().unwrap_or_default();
            let replacement: Vec<String> = m["replacement"]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(|s| vec![s.to_string()])
                .unwrap_or_default();

            match status {
                "retired" => {
                    errors.push(ValidationIssue {
                        code: "MODEL_RETIRED".to_string(),
                        message: format!("'{}' has been retired and is no longer available", id),
                        suggestions: replacement,
                    });
                }
                "deprecated" => {
                    warnings.push(ValidationIssue {
                        code: "MODEL_DEPRECATED".to_string(),
                        message: format!("'{}' is deprecated", id),
                        suggestions: replacement,
                    });
                }
                "deprecating" => {
                    warnings.push(ValidationIssue {
                        code: "MODEL_DEPRECATING".to_string(),
                        message: format!("'{}' is being deprecated soon", id),
                        suggestions: replacement,
                    });
                }
                _ => {}
            }

            if let Some(ref req_provider) = req.provider {
                let model_provider = m["provider"].as_str().unwrap_or_default();
                if model_provider != req_provider.as_str() {
                    errors.push(ValidationIssue {
                        code: "PROVIDER_MISMATCH".to_string(),
                        message: format!(
                            "'{}' belongs to provider '{}', not '{}'",
                            id, model_provider, req_provider
                        ),
                        suggestions: vec![],
                    });
                }
            }

            Some(id)
        }
    };

    Json(ValidateResponse {
        valid: errors.is_empty(),
        canonical_id,
        errors,
        warnings,
    })
}
