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
        output_price: None,
        source_url: res.url.to_string(),
    }]
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
