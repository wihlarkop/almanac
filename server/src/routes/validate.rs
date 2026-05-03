use crate::{
    catalog::{Model, ModelStatus},
    error::ApiError,
    fuzzy,
    request::RequestContext,
    response::ApiResponse,
    state::AppState,
};
use axum::{
    Extension,
    extract::{State, rejection::JsonRejection},
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ValidateRequest {
    pub model: String,
    pub provider: Option<String>,
    #[schema(value_type = Object)]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub modalities: Option<ValidateModalities>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ValidateModalities {
    pub input: Option<Vec<String>>,
    pub output: Option<Vec<String>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ValidateResponse {
    pub valid: bool,
    pub canonical_id: Option<String>,
    pub errors: Vec<ValidationIssue>,
    pub warnings: Vec<ValidationIssue>,
}

#[derive(Serialize, utoipa::ToSchema)]
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

#[utoipa::path(
    post,
    path = "/api/v1/validate",
    request_body = ValidateRequest,
    responses(
        (status = 200, description = "Validation result", body = ApiResponse<ValidateResponse>),
        (status = 400, description = "Invalid JSON body", body = ApiResponse<crate::response::EmptyData>)
    )
)]
pub async fn validate(
    State(state): State<Arc<RwLock<AppState>>>,
    Extension(context): Extension<RequestContext>,
    payload: Result<Json<ValidateRequest>, JsonRejection>,
) -> Result<Json<ApiResponse<ValidateResponse>>, ApiError> {
    let Json(req) = payload.map_err(|error| ApiError::BadRequest {
        message: error.body_text(),
    })?;
    let state = state.read().await;

    let model = state
        .models_by_id
        .get(&req.model)
        .and_then(|index| state.models.get(*index))
        .or_else(|| {
            state.aliases.get(&req.model).and_then(|canonical| {
                state
                    .models_by_id
                    .get(canonical)
                    .and_then(|index| state.models.get(*index))
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
            let id = m.id.clone();
            let replacement: Vec<String> = m
                .replacement
                .as_deref()
                .filter(|s| !s.is_empty())
                .map(|s| vec![s.to_string()])
                .unwrap_or_default();

            match m.status {
                ModelStatus::Retired => {
                    errors.push(ValidationIssue {
                        code: "MODEL_RETIRED".to_string(),
                        message: format!("'{}' has been retired and is no longer available", id),
                        suggestions: replacement,
                        parameter: None,
                        modality: None,
                        direction: None,
                    });
                }
                ModelStatus::Deprecated => {
                    warnings.push(ValidationIssue {
                        code: "MODEL_DEPRECATED".to_string(),
                        message: format!("'{}' is deprecated", id),
                        suggestions: replacement,
                        parameter: None,
                        modality: None,
                        direction: None,
                    });
                }
                ModelStatus::Deprecating => {
                    warnings.push(ValidationIssue {
                        code: "MODEL_DEPRECATING".to_string(),
                        message: format!("'{}' is being deprecated soon", id),
                        suggestions: replacement,
                        parameter: None,
                        modality: None,
                        direction: None,
                    });
                }
                ModelStatus::Active => {}
            }

            if let Some(ref req_provider) = req.provider
                && m.provider != req_provider.as_str()
            {
                errors.push(ValidationIssue {
                    code: "PROVIDER_MISMATCH".to_string(),
                    message: format!(
                        "'{}' belongs to provider '{}', not '{}'",
                        id, m.provider, req_provider
                    ),
                    suggestions: vec![],
                    parameter: None,
                    modality: None,
                    direction: None,
                });
            }

            validate_parameters(m, &req, &mut errors, &mut warnings);
            validate_modalities(m, &req, &mut errors);

            Some(id)
        }
    };

    Ok(Json(ApiResponse::ok_with_context(
        ValidateResponse {
            valid: errors.is_empty(),
            canonical_id,
            errors,
            warnings,
        },
        &context,
    )))
}

fn validate_parameters(
    model: &Model,
    req: &ValidateRequest,
    errors: &mut Vec<ValidationIssue>,
    warnings: &mut Vec<ValidationIssue>,
) {
    let Some(parameters) = &req.parameters else {
        return;
    };

    for parameter in parameters.keys() {
        if model.parameters.rejected.iter().any(|p| p == parameter) {
            errors.push(ValidationIssue {
                code: "PARAMETER_REJECTED".to_string(),
                message: format!("parameter '{}' is rejected by this model", parameter),
                suggestions: vec![],
                parameter: Some(parameter.clone()),
                modality: None,
                direction: None,
            });
        } else if model
            .parameters
            .deprecated_for_this_model
            .iter()
            .any(|p| p == parameter)
        {
            warnings.push(ValidationIssue {
                code: "PARAMETER_DEPRECATED".to_string(),
                message: format!("parameter '{}' is deprecated for this model", parameter),
                suggestions: vec![],
                parameter: Some(parameter.clone()),
                modality: None,
                direction: None,
            });
        } else if !model.parameters.supported.iter().any(|p| p == parameter) {
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

fn validate_modalities(model: &Model, req: &ValidateRequest, errors: &mut Vec<ValidationIssue>) {
    let Some(modalities) = &req.modalities else {
        return;
    };

    if let Some(requested) = &modalities.input {
        for modality in requested {
            if !model.modalities.input.iter().any(|m| m == modality) {
                errors.push(ValidationIssue {
                    code: "MODALITY_UNSUPPORTED".to_string(),
                    message: format!(
                        "input modality '{}' is not supported by this model",
                        modality
                    ),
                    suggestions: model.modalities.input.clone(),
                    parameter: None,
                    modality: Some(modality.clone()),
                    direction: Some("input".to_string()),
                });
            }
        }
    }

    if let Some(requested) = &modalities.output {
        for modality in requested {
            if !model.modalities.output.iter().any(|m| m == modality) {
                errors.push(ValidationIssue {
                    code: "MODALITY_UNSUPPORTED".to_string(),
                    message: format!(
                        "output modality '{}' is not supported by this model",
                        modality
                    ),
                    suggestions: model.modalities.output.clone(),
                    parameter: None,
                    modality: Some(modality.clone()),
                    direction: Some("output".to_string()),
                });
            }
        }
    }
}
