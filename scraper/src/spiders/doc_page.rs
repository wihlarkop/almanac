use crate::model::ScrapedModel;
use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use anyhow::Result;
use scraper::{Html, Selector};

/// Scrapes a single public docs page and extracts model IDs from <code> elements.
///
/// Works for most provider docs regardless of framework, because model API
/// names almost always appear inside inline code blocks in docs.
pub struct DocPageSpider {
    pub provider: &'static str,
    pub start_url: &'static str,
}

/// Like DocPageSpider but also follows links matching a path fragment to
/// per-model detail pages (e.g. "/model-cards/", "/models/").
pub struct MultiPageDocSpider {
    pub provider: &'static str,
    pub start_url: &'static str,
    pub base_url: &'static str,
    /// Substring that must appear in an href for the engine to follow it.
    pub follow_href_contains: &'static str,
}

// ── DocPageSpider ─────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl Spider for DocPageSpider {
    fn name(&self) -> &str {
        self.provider
    }

    fn start_urls(&self) -> Vec<String> {
        vec![self.start_url.into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let models = extract_model_ids(res.body, self.provider, res.url);
        Ok(SpiderOutput::new().items(models))
    }
}

// ── MultiPageDocSpider ────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl Spider for MultiPageDocSpider {
    fn name(&self) -> &str {
        self.provider
    }

    fn start_urls(&self) -> Vec<String> {
        vec![self.start_url.into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let is_detail = res.url != self.start_url;

        if is_detail {
            let models = extract_model_ids(res.body, self.provider, res.url);
            Ok(SpiderOutput::new().items(models))
        } else {
            let follow_urls =
                extract_follow_links(res.body, self.base_url, self.follow_href_contains);
            Ok(SpiderOutput {
                items: vec![],
                follow_urls,
            })
        }
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Extracts strings from <code> elements that look like model API identifiers.
pub fn extract_model_ids(html: &str, provider: &str, source_url: &str) -> Vec<ScrapedModel> {
    let doc = Html::parse_document(html);
    let code_sel = Selector::parse("code").unwrap();

    let mut seen = std::collections::HashSet::new();
    let mut models = Vec::new();

    for el in doc.select(&code_sel) {
        let text: String = el.text().collect::<String>();
        let text = text.trim();
        if looks_like_model_id(text) && seen.insert(text.to_string()) {
            models.push(ScrapedModel {
                id: text.to_string(),
                provider: provider.into(),
                display_name: None,
                context_window: None,
                max_output_tokens: None,
                input_price: None,
                output_price: None,
                source_url: source_url.into(),
            });
        }
    }

    models
}

/// Follows links whose href contains the given fragment.
pub fn extract_follow_links(html: &str, base_url: &str, href_contains: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    let a_sel = Selector::parse("a[href]").unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut links = Vec::new();

    for el in doc.select(&a_sel) {
        if let Some(href) = el.value().attr("href") {
            if !href.contains(href_contains) || href.contains('#') {
                continue;
            }
            let url = if href.starts_with("http") {
                href.to_string()
            } else if href.starts_with('/') {
                format!("{}{}", base_url.trim_end_matches('/'), href)
            } else {
                continue;
            };
            if seen.insert(url.clone()) {
                links.push(url);
            }
        }
    }

    links
}

/// Returns true if the string looks like a model API identifier:
/// - 5–80 characters long
/// - No spaces
/// - Contains at least one hyphen, dot, or underscore
/// - Only alphanumeric + `-`, `.`, `_`, `:`
/// - Does not look like a file path or URL fragment
fn looks_like_model_id(s: &str) -> bool {
    if s.len() < 5 || s.len() > 80 {
        return false;
    }
    if s.contains(' ') || s.contains('/') || s.contains('\n') {
        return false;
    }
    if !s.contains('-') && !s.contains('.') && !s.contains('_') {
        return false;
    }
    // Must start with a letter or digit
    if !s
        .chars()
        .next()
        .map(|c| c.is_alphanumeric())
        .unwrap_or(false)
    {
        return false;
    }
    s.chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '-' | '.' | '_' | ':'))
}
