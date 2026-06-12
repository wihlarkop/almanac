use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct IdeogramSpider;

#[async_trait::async_trait]
impl Spider for IdeogramSpider {
    fn name(&self) -> &str {
        "ideogram"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://developer.ideogram.ai/api-reference/api-reference/generate".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "ideogram", res.url);
        // Ideogram API reference pages include request/response field names
        // (aspect_ratio, style_type, webhook_url, etc.) — keep only model IDs.
        models.retain(|m| m.id.starts_with("ideogram-"));
        Ok(SpiderOutput::new().items(models))
    }
}
