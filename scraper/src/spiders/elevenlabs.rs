use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct ElevenLabsSpider;

#[async_trait::async_trait]
impl Spider for ElevenLabsSpider {
    fn name(&self) -> &str {
        "elevenlabs"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://elevenlabs.io/docs/models".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "elevenlabs", res.url);
        // Catalog convention is hyphenated IDs (eleven-multilingual-v2).
        // Reject underscore variants (eleven_multilingual_v2) — they are duplicates of the canonical entries.
        models.retain(|m| m.id.starts_with("eleven-"));
        Ok(SpiderOutput::new().items(models))
    }
}
