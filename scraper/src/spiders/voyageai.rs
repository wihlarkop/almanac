use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct VoyageAiSpider;

#[async_trait::async_trait]
impl Spider for VoyageAiSpider {
    fn name(&self) -> &str {
        "voyageai"
    }

    fn start_urls(&self) -> Vec<String> {
        vec!["https://docs.voyageai.com/docs/embeddings".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "voyageai", res.url);
        // Voyage AI model IDs: voyage-*, rerank-*
        models.retain(|m| m.id.starts_with("voyage-") || m.id.starts_with("rerank-"));
        Ok(SpiderOutput::new().items(models))
    }
}
