use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct ByteDanceSpider;

#[async_trait::async_trait]
impl Spider for ByteDanceSpider {
    fn name(&self) -> &str {
        "bytedance"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://www.volcengine.com/docs/82379/1382513".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "bytedance", res.url);
        // Keep only known ByteDance model families — the Volcengine docs page
        // embeds JS bundles (e.g. react-dom) that pass the generic heuristic.
        models.retain(|m| {
            m.id.starts_with("doubao-")
                || m.id.starts_with("seed-")
                || m.id.starts_with("seed3d-")
                || m.id.starts_with("seedance-")
                || m.id.starts_with("seedream-")
        });
        Ok(SpiderOutput::new().items(models))
    }
}
