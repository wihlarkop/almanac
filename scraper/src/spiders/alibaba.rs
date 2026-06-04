use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct AlibabaSpider;

#[async_trait::async_trait]
impl Spider for AlibabaSpider {
    fn name(&self) -> &str {
        "alibaba"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://www.alibabacloud.com/help/en/model-studio/getting-started/models".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "alibaba", res.url);
        // Keep only Qwen model IDs
        models.retain(|m| m.id.to_lowercase().starts_with("qwen"));
        Ok(SpiderOutput::new().items(models))
    }
}
