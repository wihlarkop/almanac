use crate::{fuzzy, state::AppState};
use axum::{extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub model: String,
    pub provider: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub modalities: Option<ValidateModalities>,
}

#[derive(Deserialize)]
pub struct ValidateModalities {
    pub input: Option<Vec<String>>,
    pub output: Option<Vec<String>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
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
                parameter: None,
                modality: None,
                direction: None,
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
                        parameter: None,
                        modality: None,
                        direction: None,
                    });
                }
                "deprecated" => {
                    warnings.push(ValidationIssue {
                        code: "MODEL_DEPRECATED".to_string(),
                        message: format!("'{}' is deprecated", id),
                        suggestions: replacement,
                        parameter: None,
                        modality: None,
                        direction: None,
                    });
                }
                "deprecating" => {
                    warnings.push(ValidationIssue {
                        code: "MODEL_DEPRECATING".to_string(),
                        message: format!("'{}' is being deprecated soon", id),
                        suggestions: replacement,
                        parameter: None,
                        modality: None,
                        direction: None,
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
                        parameter: None,
                        modality: None,
                        direction: None,
                    });
                }
            }

            validate_parameters(m, &req, &mut errors, &mut warnings);
            validate_modalities(m, &req, &mut errors);

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

fn string_set(model: &serde_json::Value, path: &[&str]) -> Vec<String> {
    let mut current = model;
    for key in path {
        current = &current[*key];
    }
    current
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn validate_parameters(
    model: &serde_json::Value,
    req: &ValidateRequest,
    errors: &mut Vec<ValidationIssue>,
    warnings: &mut Vec<ValidationIssue>,
) {
    let Some(parameters) = &req.parameters else {
        return;
    };

    let supported = string_set(model, &["parameters", "supported"]);
    let rejected = string_set(model, &["parameters", "rejected"]);
    let deprecated = string_set(model, &["parameters", "deprecated_for_this_model"]);

    for parameter in parameters.keys() {
        if rejected.iter().any(|p| p == parameter) {
            errors.push(ValidationIssue {
                code: "PARAMETER_REJECTED".to_string(),
                message: format!("parameter '{}' is rejected by this model", parameter),
                suggestions: vec![],
                parameter: Some(parameter.clone()),
                modality: None,
                direction: None,
            });
        } else if deprecated.iter().any(|p| p == parameter) {
            warnings.push(ValidationIssue {
                code: "PARAMETER_DEPRECATED".to_string(),
                message: format!("parameter '{}' is deprecated for this model", parameter),
                suggestions: vec![],
                parameter: Some(parameter.clone()),
                modality: None,
                direction: None,
            });
        } else if !supported.iter().any(|p| p == parameter) {
            warnings.push(ValidationIssue {
                code: "PARAMETER_UNSUPPORTED".to_string(),
                message: format!(
                    "parameter '{}' is not listed as supported by this model",
                    parameter
                ),
                suggestions: vec![],
                parameter: Some(parameter.clone()),
                modality: None,
                direction: None,
            });
        }
    }
}

fn validate_modalities(
    model: &serde_json::Value,
    req: &ValidateRequest,
    errors: &mut Vec<ValidationIssue>,
) {
    let Some(modalities) = &req.modalities else {
        return;
    };

    if let Some(requested) = &modalities.input {
        let supported = string_set(model, &["modalities", "input"]);
        for modality in requested {
            if !supported.iter().any(|m| m == modality) {
                errors.push(ValidationIssue {
                    code: "MODALITY_UNSUPPORTED".to_string(),
                    message: format!(
                        "input modality '{}' is not supported by this model",
                        modality
                    ),
                    suggestions: supported.clone(),
                    parameter: None,
                    modality: Some(modality.clone()),
                    direction: Some("input".to_string()),
                });
            }
        }
    }

    if let Some(requested) = &modalities.output {
        let supported = string_set(model, &["modalities", "output"]);
        for modality in requested {
            if !supported.iter().any(|m| m == modality) {
                errors.push(ValidationIssue {
                    code: "MODALITY_UNSUPPORTED".to_string(),
                    message: format!(
                        "output modality '{}' is not supported by this model",
                        modality
                    ),
                    suggestions: supported.clone(),
                    parameter: None,
                    modality: Some(modality.clone()),
                    direction: Some("output".to_string()),
                });
            }
        }
    }
}
