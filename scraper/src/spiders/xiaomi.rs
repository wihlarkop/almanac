use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

pub struct XiaomiSpider;

#[async_trait::async_trait]
impl Spider for XiaomiSpider {
    fn name(&self) -> &str {
        "xiaomi"
    }

    fn start_urls(&self) -> Vec<String> {
        // Use the HuggingFace page instead of GitHub — GitHub's __NEXT_DATA__
        // contains its own feature flag names which are mistaken for model IDs.
        vec!["https://huggingface.co/MiMo-AI".into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let mut models = extract_model_ids(res.body, "xiaomi", res.url);
        // Xiaomi MiMo model IDs: mimo-*
        models.retain(|m| m.id.starts_with("mimo-"));
        Ok(SpiderOutput::new().items(models))
    }
}
