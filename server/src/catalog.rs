use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Provider {
    pub id: String,
    pub display_name: String,
    pub website: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_docs: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Model {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub status: ModelStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecation_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sunset_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
    pub context_window: u64,
    pub max_output_tokens: u64,
    pub modalities: Modalities,
    pub capabilities: HashMap<String, bool>,
    pub parameters: ModelParameters,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<Pricing>,
    pub last_verified: String,
    pub confidence: Confidence,
    pub endpoint_family: EndpointFamily,
    pub sources: Vec<Source>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Active,
    Deprecating,
    Deprecated,
    Retired,
}

impl ModelStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Deprecating => "deprecating",
            Self::Deprecated => "deprecated",
            Self::Retired => "retired",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Modalities {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ModelParameters {
    pub supported: Vec<String>,
    pub rejected: Vec<String>,
    pub deprecated_for_this_model: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Pricing {
    pub currency: String,
    pub input: f64,
    pub output: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_input: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_input: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_output: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_fee: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_fee: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_image: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_second: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_minute: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_million_chars: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing_notes: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Official,
    Inferred,
    Community,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndpointFamily {
    ChatCompletions,
    Responses,
    Custom,
    Agent,
    Search,
    Embeddings,
    Reranking,
    ImageGeneration,
    Speech,
    Transcription,
    VideoGeneration,
    Realtime,
    Ocr,
    MusicGeneration,
    MeshGeneration,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Source {
    pub url: String,
    pub last_verified: String,
}
