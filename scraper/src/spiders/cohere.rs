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
        // Reject deployment-namespace IDs:
        //   "cohere.command-r-plus-v1:0"  — AWS Bedrock (dot-prefixed, colon-versioned)
        //   "cohere-rerank-v4-pro"         — "Unique per deployment" section in Cohere docs
        models.retain(|m| {
            !m.id.contains(':') && !m.id.starts_with("cohere.") && !m.id.starts_with("cohere-")
        });
        Ok(SpiderOutput::new().items(models))
    }
}
