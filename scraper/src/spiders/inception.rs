use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct InceptionSpider;

#[async_trait::async_trait]
impl Spider for InceptionSpider {
    fn name(&self) -> &str {
        "inception"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.inceptionlabs.ai/".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "inception", res.url);
        // Inception Labs model IDs: mercury-*
        models.retain(|m| m.id.starts_with("mercury-"));
        Ok(SpiderOutput::new().items(models))
    }
}
