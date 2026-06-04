use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::{extract_follow_links, extract_model_ids};
use anyhow::Result;

/// Mistral HTML spider — follows model-card links from the overview page.
/// (Separate from `mistral.rs` which is the legacy name; this is the same logic.)
pub struct MistralHtmlSpider;

#[async_trait::async_trait]
impl Spider for MistralHtmlSpider {
    fn name(&self) -> &str {
        "mistral-html"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.mistral.ai/getting-started/models/models_overview/".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        if res.url.contains("/model-cards/") {
            Ok(SpiderOutput::new().items(extract_model_ids(res.body, "mistral", res.url)))
        } else {
            Ok(SpiderOutput {
                items: vec![],
                follow_urls: extract_follow_links(
                    res.body,
                    "https://docs.mistral.ai",
                    "/model-cards/",
                ),
            })
        }
    }
}
