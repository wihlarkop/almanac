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
        // but keep the real `cohere-transcribe-*` audio model IDs, which also
        // carry the `cohere-` prefix.
        models.retain(|m| {
            let id = m.id.as_str();
            if id.starts_with("cohere-transcribe") {
                return true;
            }
            !id.contains(':') && !id.starts_with("cohere.") && !id.starts_with("cohere-")
        });
        Ok(SpiderOutput::new().items(models))
    }
}
