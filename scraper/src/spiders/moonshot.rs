use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct MoonshotSpider;

#[async_trait::async_trait]
impl Spider for MoonshotSpider {
    fn name(&self) -> &str {
        "moonshot"
    }

    fn start_urls(&self) -> Vec<String> {
        vec![
            "https://platform.moonshot.cn/docs/intro".into(),
            "https://platform.kimi.ai/docs/models".into(),
        ]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "moonshot", res.url);
        models.retain(|m| m.id.starts_with("moonshot-") || m.id.starts_with("kimi-"));
        Ok(SpiderOutput::new().items(models))
    }
}
