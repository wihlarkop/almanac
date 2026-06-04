use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct OpenAiSpider;

#[async_trait::async_trait]
impl Spider for OpenAiSpider {
    fn name(&self) -> &str {
        "openai"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://platform.openai.com/docs/models".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        Ok(SpiderOutput::new().items(extract_model_ids(res.body, "openai", res.url)))
    }
}
