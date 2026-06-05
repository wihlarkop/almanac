use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct LumaSpider;

#[async_trait::async_trait]
impl Spider for LumaSpider {
    fn name(&self) -> &str {
        "luma"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.lumalabs.ai/".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "luma", res.url);
        // Luma model IDs: ray-*, photon-*, dream-machine-*
        models.retain(|m| {
            m.id.starts_with("ray-")
                || m.id.starts_with("photon-")
                || m.id.starts_with("dream-machine")
        });
        Ok(SpiderOutput::new().items(models))
    }
}
