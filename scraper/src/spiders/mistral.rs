use crate::model::ScrapedModel;
use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use anyhow::Result;
use scraper::{Html, Selector};

const START_URL: &str = "https://docs.mistral.ai/getting-started/models/models_overview/";
const BASE: &str = "https://docs.mistral.ai";

pub struct MistralSpider;

#[async_trait::async_trait]
impl Spider for MistralSpider {
    fn name(&self) -> &str {
        "mistral"
    }

    fn start_urls(&self) -> Vec<String> {
        vec![START_URL.into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        if res.url.contains("/model-cards/") {
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
    let doc = Html::parse_document(res.body);

    // Model card pages render the canonical API model ID in a <button> element
    // (a copy-to-clipboard widget). Extract it rather than using the verbose URL slug
    // (e.g. "mistral-medium-3-5" from the button vs "mistral-medium-3-5-26-04" from the URL).
    let btn_sel = Selector::parse("button").unwrap();
    let model_id = doc
        .select(&btn_sel)
        .map(|el| el.text().collect::<String>().trim().to_string())
        .find(|t| is_canonical_mistral_id(t));

    // Fall back to the URL slug if the button isn't present in the SSR HTML.
    let id = model_id.unwrap_or_else(|| {
        res.url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string()
    });

    if id.is_empty() {
        return vec![];
    }

    let h1_sel = Selector::parse("h1").unwrap();
    let display_name = doc
        .select(&h1_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string());

    vec![ScrapedModel {
        id,
        provider: "mistral".into(),
        display_name,
        context_window: None,
        max_output_tokens: None,
        input_price: None,
        input_price_candidates: Vec::new(),
        output_price: None,
        source_url: res.url.to_string(),
    }]
}

/// Returns true if `s` looks like a compact Mistral API model ID (not a verbose dated URL slug).
/// Dated slugs like "mistral-medium-3-5-26-04" have 5+ hyphens; compact IDs have ≤3.
fn is_canonical_mistral_id(s: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "mistral-",
        "codestral-",
        "mixtral-",
        "pixtral-",
        "devstral-",
        "voxtral-",
        "magistral-",
        "ministral-",
        "leanstral-",
        "labs-",
        "open-mistral-",
        "open-mixtral-",
        "ocr-",
    ];
    let hyphen_count = s.chars().filter(|&c| c == '-').count();
    !s.ends_with("-latest") && hyphen_count <= 3 && PREFIXES.iter().any(|p| s.starts_with(p))
}

fn extract_links(res: &HtmlResponse<'_>) -> Vec<String> {
    let doc = Html::parse_document(res.body);
    let sel = Selector::parse(r#"a[href*="/model-cards/"]"#).unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut links = Vec::new();

    for el in doc.select(&sel) {
        if let Some(href) = el.value().attr("href") {
            if href.contains('#') {
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
