use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct MicrosoftSpider;

#[async_trait::async_trait]
impl Spider for MicrosoftSpider {
    fn name(&self) -> &str {
        "microsoft"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://learn.microsoft.com/en-us/azure/ai-services/openai/concepts/models".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "microsoft", res.url);
        // Microsoft-developed models only: phi-*, mai-*, trellis-*
        // Reject gpt-* — those are OpenAI models hosted on Azure, not developed by Microsoft.
        models.retain(|m| {
            m.id.starts_with("phi-") || m.id.starts_with("mai-") || m.id.starts_with("trellis-")
        });
        Ok(SpiderOutput::new().items(models))
    }
}
