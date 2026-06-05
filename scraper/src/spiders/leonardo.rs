use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct LeonardoSpider;

#[async_trait::async_trait]
impl Spider for LeonardoSpider {
    fn name(&self) -> &str {
        "leonardo"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.leonardo.ai/".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "leonardo", res.url);
        // Leonardo model IDs: phoenix-*, kino-*, diffusion-*, vision-*
        models.retain(|m| {
            m.id.starts_with("phoenix-")
                || m.id.starts_with("kino-")
                || m.id.starts_with("diffusion-")
                || m.id.starts_with("vision-")
                || m.id.starts_with("leonardo-")
                || m.id.starts_with("albedo-")
        });
        Ok(SpiderOutput::new().items(models))
    }
}
