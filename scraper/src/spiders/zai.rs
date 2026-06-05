use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct ZaiSpider;

#[async_trait::async_trait]
impl Spider for ZaiSpider {
    fn name(&self) -> &str {
        "zai"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.z.ai/".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "zai", res.url);
        // Z.AI (Zhipu) model IDs: glm-*
        models.retain(|m| m.id.starts_with("glm-"));
        Ok(SpiderOutput::new().items(models))
    }
}
