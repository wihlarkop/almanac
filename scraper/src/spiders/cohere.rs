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
        let mut models = extract_model_ids(res.body, "cohere", res.url);
        // Reject AWS Bedrock-style IDs like "cohere.command-r-plus-v1:0"
        // The ":0" suffix is valid in Bedrock but fails the almanac schema pattern.
        models.retain(|m| !m.id.contains(':'));
        Ok(SpiderOutput::new().items(models))
    }
}
