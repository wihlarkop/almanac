use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct XaiSpider;

#[async_trait::async_trait]
impl Spider for XaiSpider {
    fn name(&self) -> &str {
        "xai"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.x.ai/docs/models".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        Ok(SpiderOutput::new().items(extract_model_ids(res.body, "xai", res.url)))
    }
}
