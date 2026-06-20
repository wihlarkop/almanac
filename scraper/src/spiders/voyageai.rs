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
        // Voyage splits its catalog across separate docs pages — embeddings,
        // rerankers and multimodal each live on their own page, so we crawl all
        // three to avoid false "missing from docs" for rerank-* / multimodal IDs.
        vec![
            "https://docs.voyageai.com/docs/embeddings".into(),
            "https://docs.voyageai.com/docs/reranker".into(),
            "https://docs.voyageai.com/docs/multimodal-embeddings".into(),
        ]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "voyageai", res.url);
        // Voyage AI model IDs: voyage-*, rerank-*
        models.retain(|m| m.id.starts_with("voyage-") || m.id.starts_with("rerank-"));
        Ok(SpiderOutput::new().items(models))
    }
}
