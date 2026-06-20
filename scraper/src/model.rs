use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScrapedModel {
    pub id: String,
    pub provider: String,
    pub display_name: Option<String>,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub input_price: Option<f64>,
    /// Additional plausible input prices when a provider lists several billing
    /// tiers for one model (e.g. pay-as-you-go vs growth). The catalog only has
    /// to match ONE of these (or `input_price`) for the price to count as
    /// unchanged, so a multi-tier pricing table doesn't produce false drift.
    #[serde(default)]
    pub input_price_candidates: Vec<f64>,
    pub output_price: Option<f64>,
    pub source_url: String,
}

impl ScrapedModel {
    pub fn yaml_path_relative(&self) -> String {
        format!("models/{}/{}.yaml", self.provider, self.id)
    }
}
