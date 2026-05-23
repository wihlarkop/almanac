use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapedModel {
    pub id: String,
    pub provider: String,
    pub display_name: Option<String>,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub source_url: String,
}

impl ScrapedModel {
    pub fn yaml_path_relative(&self) -> String {
        format!("models/{}/{}.yaml", self.provider, self.id)
    }
}
