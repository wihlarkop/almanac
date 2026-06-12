use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct HeyGenSpider;

#[async_trait::async_trait]
impl Spider for HeyGenSpider {
    fn name(&self) -> &str {
        "heygen"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.heygen.com/reference/list-voices-v2".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "heygen", res.url);
        // HeyGen API reference pages include response field names (voice_id,
        // has_more, next_token, etc.) — keep only avatar model IDs.
        models.retain(|m| m.id.starts_with("avatar-"));
        Ok(SpiderOutput::new().items(models))
    }
}
