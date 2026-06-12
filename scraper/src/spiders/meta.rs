use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::{extract_follow_links, extract_model_ids};
use anyhow::Result;

pub struct MetaSpider;

#[async_trait::async_trait]
impl Spider for MetaSpider {
    fn name(&self) -> &str {
        "meta"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://www.llama.com/docs/model-cards-and-prompt-formats/llama3_1/".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "meta", res.url);
        let follow_urls =
            extract_follow_links(res.body, "https://www.llama.com", "/docs/model-cards");
        if models.is_empty() {
            return Ok(SpiderOutput {
                items: vec![],
                follow_urls,
            });
        }
        models.retain(|m| {
            m.id.starts_with("llama-")
                || m.id.starts_with("llama3")
                || m.id.starts_with("llama4")
                || m.id.starts_with("codellama-")
                || m.id.starts_with("meta-llama-")
        });
        Ok(SpiderOutput::new().items(models))
    }
}
