use crate::model::ScrapedModel;
use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use anyhow::Result;
use serde::Deserialize;

const API_URL: &str = "https://api.deepseek.com/v1/models";

pub struct DeepSeekSpider {
    pub api_key: String,
}

impl DeepSeekSpider {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
}

#[async_trait::async_trait]
impl Spider for DeepSeekSpider {
    fn name(&self) -> &str {
        "deepseek"
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
            .data
            .into_iter()
            .map(|entry| ScrapedModel {
                id: entry.id,
                provider: "deepseek".into(),
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
