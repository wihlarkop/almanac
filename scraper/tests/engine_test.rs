use almanac_scraper::model::ScrapedModel;
use almanac_scraper::spider::{HtmlResponse, Spider, SpiderOutput};
use anyhow::Result;

struct EchoSpider {
    url: String,
    model_id: String,
}

#[async_trait::async_trait]
impl Spider for EchoSpider {
    fn name(&self) -> &str {
        "echo"
    }
    fn start_urls(&self) -> Vec<String> {
        vec![self.url.clone()]
    }
    async fn scrape(&self, _res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        Ok(SpiderOutput::new().items(vec![ScrapedModel {
            id: self.model_id.clone(),
            provider: "test".into(),
            display_name: None,
            context_window: None,
            max_output_tokens: None,
            input_price: None,
            output_price: None,
            source_url: self.url.clone(),
        }]))
    }
}

// Compile-check: verifies EchoSpider implements the Spider trait correctly.
// Real HTTP behavior is tested via spider-level fixtures in other test files.
#[tokio::test]
async fn echo_spider_implements_spider_trait() {
    let spider = EchoSpider {
        url: "http://example.com".into(),
        model_id: "m1".into(),
    };
    let res = HtmlResponse {
        url: "http://example.com",
        body: "",
    };
    let output = spider.scrape(&res).await.unwrap();
    assert_eq!(output.items.len(), 1);
    assert_eq!(output.items[0].id, "m1");
}
