use crate::model::ScrapedModel;
use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use anyhow::Result;
use scraper::{Html, Selector};

const START_URL: &str = "https://ai.google.dev/gemini-api/docs/models";
const BASE: &str = "https://ai.google.dev";

pub struct GoogleSpider;

#[async_trait::async_trait]
impl Spider for GoogleSpider {
    fn name(&self) -> &str {
        "google"
    }

    fn start_urls(&self) -> Vec<String> {
        vec![START_URL.into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let is_detail =
            res.url.contains("/docs/models/gemini") || res.url.contains("/docs/models/gemma");

        if is_detail {
            Ok(SpiderOutput::new().items(extract_model(res)))
        } else {
            Ok(SpiderOutput {
                items: vec![],
                follow_urls: extract_links(res),
            })
        }
    }
}

fn extract_model(res: &HtmlResponse<'_>) -> Vec<ScrapedModel> {
    let id = res
        .url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_string();
    if id.is_empty() {
        return vec![];
    }

    let doc = Html::parse_document(res.body);
    let section_sel = Selector::parse("section").unwrap();
    let b_sel = Selector::parse("b").unwrap();
    let p_sel = Selector::parse("p").unwrap();
    let h1_sel = Selector::parse("h1").unwrap();

    let mut context_window = None;
    let mut max_output_tokens = None;

    for section in doc.select(&section_sel) {
        let label = section
            .select(&b_sel)
            .next()
            .map(|b| b.text().collect::<String>().to_lowercase())
            .unwrap_or_default();
        let value = section
            .select(&p_sel)
            .nth(1)
            .map(|p| p.text().collect::<String>())
            .unwrap_or_default();

        if label.contains("input token limit") {
            context_window = parse_token_count(value.trim());
        } else if label.contains("output token limit") {
            max_output_tokens = parse_token_count(value.trim());
        }
    }

    let display_name = doc
        .select(&h1_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string());

    vec![ScrapedModel {
        id,
        provider: "google".into(),
        display_name,
        context_window,
        max_output_tokens,
        input_price: None,
        output_price: None,
        source_url: res.url.to_string(),
    }]
}

fn extract_links(res: &HtmlResponse<'_>) -> Vec<String> {
    let doc = Html::parse_document(res.body);
    let sel = Selector::parse(r#"a[href*="/gemini-api/docs/models/"]"#).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut links = Vec::new();

    for el in doc.select(&sel) {
        if let Some(href) = el.value().attr("href") {
            if href.contains('#') || href.ends_with("/models") {
                continue;
            }
            let url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("{BASE}{href}")
            };
            if seen.insert(url.clone()) {
                links.push(url);
            }
        }
    }
    links
}

fn parse_token_count(text: &str) -> Option<u64> {
    if let Ok(n) = text.replace(',', "").parse::<u64>() {
        return Some(n);
    }
    let lower = text.to_lowercase();
    if let Some(n) = lower.find('m').and_then(|pos| {
        lower[..pos]
            .split_whitespace()
            .last()
            .and_then(|s| s.replace(',', "").parse::<f64>().ok())
    }) {
        return Some((n * 1_000_000.0) as u64);
    }
    if let Some(n) = lower.find('k').and_then(|pos| {
        lower[..pos]
            .split_whitespace()
            .last()
            .and_then(|s| s.replace(',', "").parse::<f64>().ok())
    }) {
        return Some((n * 1_000.0) as u64);
    }
    None
}
