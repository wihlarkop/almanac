use crate::model::ScrapedModel;
use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use anyhow::Result;
use serde::Deserialize;

const API_URL: &str = "https://api.cohere.com/v1/models";

pub struct CohereSpider {
    pub api_key: String,
}

impl CohereSpider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[derive(Deserialize)]
struct ModelList {
    models: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    name: String,
}

#[async_trait::async_trait]
impl Spider for CohereSpider {
    fn name(&self) -> &str {
        "cohere"
    }

    fn start_urls(&self) -> Vec<String> {
        vec![API_URL.into()]
    }

    fn headers(&self) -> Vec<(String, String)> {
        vec![("Authorization".into(), format!("Bearer {}", self.api_key))]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let list: ModelList = serde_json::from_str(res.body)?;
        let items = list
            .models
            .into_iter()
            .map(|entry| ScrapedModel {
                id: entry.name,
                provider: "cohere".into(),
                display_name: None,
                context_window: None,
                max_output_tokens: None,
                input_price: None,
                output_price: None,
                source_url: API_URL.into(),
            })
            .collect();
        Ok(SpiderOutput::new().items(items))
    }
}
