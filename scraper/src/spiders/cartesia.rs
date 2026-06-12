use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct CartesiaSpider;

#[async_trait::async_trait]
impl Spider for CartesiaSpider {
    fn name(&self) -> &str {
        "cartesia"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.cartesia.ai/build-with-cartesia/tts-models/api-changes".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "cartesia", res.url);
        // Cartesia docs embed code snippets with API field names (model_id,
        // output_format, sample_rate, etc.) — keep only the Sonic model family.
        models.retain(|m| m.id.starts_with("sonic-") || m.id == "sonic");
        Ok(SpiderOutput::new().items(models))
    }
}
