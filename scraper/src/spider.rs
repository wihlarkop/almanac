use crate::model::ScrapedModel;
use anyhow::Result;

/// Raw HTTP response passed to each spider's scrape method.
/// Holds the URL and the full response body as a string (HTML or JSON).
pub struct HtmlResponse<'a> {
    pub url: &'a str,
    pub body: &'a str,
}

/// What a spider returns from one page.
#[derive(Default)]
pub struct SpiderOutput {
    /// Models extracted from this page.
    pub items: Vec<ScrapedModel>,
    /// Additional URLs the engine should fetch next.
    pub follow_urls: Vec<String>,
}

impl SpiderOutput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: Vec<ScrapedModel>) -> Self {
        self.items = items;
        self
    }

    pub fn follow(mut self, url: String) -> Self {
        self.follow_urls.push(url);
        self
    }
}

/// A spider scrapes one or more pages for a single provider.
///
/// Designed to mirror kumo's Spider trait for easy future migration.
#[async_trait::async_trait]
pub trait Spider: Send + Sync {
    /// Unique name used in logs and CLI filtering.
    fn name(&self) -> &str;

    /// Entry-point URLs the engine fetches first.
    fn start_urls(&self) -> Vec<String>;

    /// Optional HTTP headers sent with every request for this spider.
    /// Used for Authorization on JSON API spiders.
    fn headers(&self) -> Vec<(String, String)> {
        vec![]
    }

    /// Parse one page. Return extracted models and any follow URLs.
    async fn scrape(&self, response: &HtmlResponse<'_>) -> Result<SpiderOutput>;
}
