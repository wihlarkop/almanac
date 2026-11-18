use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Provider {
    pub id: String,
    pub display_name: String,
    pub website: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub api_docs: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Model {
    pub id: String,
    pub provider: String,
    pub display_name: String,
    pub status: ModelStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub deprecation_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub sunset_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub replacement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub context_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub context_window_state: Option<MetadataState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub max_output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub max_output_tokens_state: Option<MetadataState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub modalities: Option<Modalities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub modalities_state: Option<MetadataState>,
    pub capabilities: HashMap<String, bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub capabilities_state: Option<MetadataState>,
    pub parameters: ModelParameters,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub parameters_state: Option<MetadataState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub pricing: Option<Pricing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub pricing_state: Option<PricingState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub last_verified: Option<String>,
    pub confidence: Confidence,
    pub endpoint_family: EndpointFamily,
    pub sources: Vec<Source>,
}

impl Model {
    pub fn pricing_is_available(&self) -> bool {
        match self.pricing_state {
            Some(PricingState::Free) => true,
            Some(PricingState::Published) => self.pricing.is_some(),
            Some(
                PricingState::Unknown | PricingState::NotPublished | PricingState::NotApplicable,
            ) => false,
            None => self.pricing.is_some(),
        }
    }

    pub fn pricing_is_missing(&self) -> bool {
        match self.pricing_state {
            Some(PricingState::Unknown | PricingState::NotPublished) => true,
            Some(PricingState::Free | PricingState::NotApplicable | PricingState::Published) => {
                false
            }
            None => self.pricing.is_none(),
        }
    }

    pub fn has_comparable_token_pricing(&self) -> bool {
        let token_endpoint = matches!(
            self.endpoint_family,
            EndpointFamily::ChatCompletions | EndpointFamily::Responses
        );
        let text_only = self.modalities.as_ref().is_some_and(|modalities| {
            modalities.input.as_slice() == ["text"] && modalities.output.as_slice() == ["text"]
        });
        let price_state_known =
            self.pricing_state.is_none() || self.pricing_state == Some(PricingState::Published);

        token_endpoint && text_only && self.pricing.is_some() && price_state_known
    }

    pub fn is_known_free(&self) -> bool {
        self.pricing_state == Some(PricingState::Free)
            || (self.pricing_state.is_none()
                && self.has_comparable_token_pricing()
                && self.pricing.as_ref().is_some_and(Pricing::is_free))
    }

    pub fn is_known_paid(&self) -> bool {
        (self.pricing_state.is_none() || self.pricing_state == Some(PricingState::Published))
            && self
                .pricing
                .as_ref()
                .is_some_and(|pricing| !pricing.is_free())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    Unknown,
    Active,
    Deprecating,
    Deprecated,
    Retired,
}

impl ModelStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Active => "active",
            Self::Deprecating => "deprecating",
            Self::Deprecated => "deprecated",
            Self::Retired => "retired",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetadataState {
    Known,
    Unknown,
    NotApplicable,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PricingState {
    Published,
    Free,
    Unknown,
    NotPublished,
    NotApplicable,
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
    #[schema(nullable = false)]
    pub cached_input: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub batch_input: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub batch_output: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub request_fee: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub search_fee: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub reasoning: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub per_image: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub per_second: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub per_minute: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub per_million_chars: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub per_page: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub input_audio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub input_image: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub input_video: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub output_audio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub tiers: Option<Vec<PricingTier>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub pricing_notes: Option<String>,
}

impl Pricing {
    pub fn is_free(&self) -> bool {
        let optional_rates = [
            self.cached_input,
            self.batch_input,
            self.batch_output,
            self.request_fee,
            self.search_fee,
            self.reasoning,
            self.per_image,
            self.per_second,
            self.per_minute,
            self.per_million_chars,
            self.per_page,
            self.input_audio,
            self.input_image,
            self.input_video,
            self.output_audio,
        ];
        self.input == 0.0
            && self.output == 0.0
            && optional_rates
                .into_iter()
                .flatten()
                .all(|price| price == 0.0)
            && self.tiers.as_ref().is_none_or(|tiers| {
                tiers.iter().all(|tier| {
                    tier.input == 0.0
                        && tier.output == 0.0
                        && tier.cached_input.is_none_or(|price| price == 0.0)
                })
            })
    }
}

/// A single prompt-length pricing tier (see `Pricing::tiers`).
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct PricingTier {
    /// Inclusive upper bound (tokens) of this tier's prompt length; `None` for the unbounded top tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub max_prompt_tokens: Option<u64>,
    pub input: f64,
    pub output: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub cached_input: Option<f64>,
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
    Unknown,
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

impl EndpointFamily {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::ChatCompletions => "chat_completions",
            Self::Responses => "responses",
            Self::Custom => "custom",
            Self::Agent => "agent",
            Self::Search => "search",
            Self::Embeddings => "embeddings",
            Self::Reranking => "reranking",
            Self::ImageGeneration => "image_generation",
            Self::Speech => "speech",
            Self::Transcription => "transcription",
            Self::VideoGeneration => "video_generation",
            Self::Realtime => "realtime",
            Self::Ocr => "ocr",
            Self::MusicGeneration => "music_generation",
            Self::MeshGeneration => "mesh_generation",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Source {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(nullable = false)]
    pub last_verified: Option<String>,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct ModelPriceStat {
    pub model_id: String,
    pub provider: String,
    pub price: f64,
}

#[derive(Clone, Debug, Serialize, utoipa::ToSchema)]
pub struct CatalogStats {
    pub total_models: usize,
    pub total_providers: usize,
    pub models_by_status: std::collections::HashMap<String, usize>,
    pub models_by_provider: std::collections::HashMap<String, usize>,
    pub models_by_endpoint_family: std::collections::HashMap<String, usize>,
    pub models_by_input_modality: std::collections::HashMap<String, usize>,
    pub models_by_output_modality: std::collections::HashMap<String, usize>,
    pub free_models: usize,
    pub models_without_pricing: usize,
    pub cheapest_input: Option<ModelPriceStat>,
    pub most_expensive_input: Option<ModelPriceStat>,
    pub cheapest_output: Option<ModelPriceStat>,
    pub most_expensive_output: Option<ModelPriceStat>,
    pub last_updated: String,
}

#[cfg(test)]
mod tests {
    use super::Model;

    #[test]
    fn legacy_catalog_model_keeps_known_api_values() {
        let model: Model =
            serde_yaml::from_str(include_str!("../../models/openai/gpt-4o.yaml")).unwrap();
        let api_value = serde_json::to_value(model).unwrap();

        assert_eq!(api_value["context_window"], 128000);
        assert_eq!(api_value["max_output_tokens"], 16384);
        assert_eq!(api_value["status"], "active");
        assert!(api_value["modalities"].is_object());
    }

    #[test]
    fn unknown_metadata_remains_explicit_in_api_json() {
        let model: Model = serde_yaml::from_str(
            r#"
id: candidate
provider: example
display_name: Candidate
status: unknown
context_window: null
context_window_state: unknown
max_output_tokens: null
max_output_tokens_state: unknown
modalities: null
modalities_state: unknown
capabilities: {}
capabilities_state: unknown
parameters:
  supported: []
  rejected: []
  deprecated_for_this_model: []
parameters_state: unknown
pricing: null
pricing_state: unknown
last_verified: null
confidence: unknown
endpoint_family: unknown
sources:
  - url: https://example.com/models
"#,
        )
        .unwrap();
        let api_value = serde_json::to_value(model).unwrap();

        assert_eq!(api_value["status"], "unknown");
        assert!(api_value.get("context_window").is_none());
        assert_eq!(api_value["context_window_state"], "unknown");
        assert!(api_value.get("modalities").is_none());
        assert_eq!(api_value["modalities_state"], "unknown");
        assert!(api_value.get("pricing").is_none());
        assert_eq!(api_value["pricing_state"], "unknown");
        assert!(api_value.get("last_verified").is_none());
        assert!(api_value["sources"][0].get("last_verified").is_none());
    }
}
