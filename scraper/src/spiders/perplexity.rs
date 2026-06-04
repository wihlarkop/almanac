use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct PerplexitySpider;

#[async_trait::async_trait]
impl Spider for PerplexitySpider {
    fn name(&self) -> &str {
        "perplexity"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.perplexity.ai/guides/model-cards".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        Ok(SpiderOutput::new().items(extract_model_ids(res.body, "perplexity", res.url)))
    }
}
