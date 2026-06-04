use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct CohereSpider;

#[async_trait::async_trait]
impl Spider for CohereSpider {
    fn name(&self) -> &str {
        "cohere"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.cohere.com/v2/docs/models".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        Ok(SpiderOutput::new().items(extract_model_ids(res.body, "cohere", res.url)))
    }
}
