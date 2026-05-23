use crate::model::ScrapedModel;
use async_trait::async_trait;
use kumo::prelude::*;

const START_URL: &str = "https://ai.google.dev/gemini-api/docs/models";
const BASE: &str = "https://ai.google.dev";

pub struct GoogleSpider;

#[async_trait]
impl Spider for GoogleSpider {
    type Item = ScrapedModel;

    fn name(&self) -> &str {
        "google"
    }

    fn start_urls(&self) -> Vec<String> {
        vec![START_URL.into()]
    }

    fn allowed_domains(&self) -> Vec<&str> {
        vec!["ai.google.dev"]
    }

    async fn parse(&self, res: &Response) -> Result<Output<Self::Item>, KumoError> {
        let mut output = Output::new();

        if res.url().contains("/docs/models/gemini") || res.url().contains("/docs/models/gemma") {
            let id = res
                .url()
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_string();

            if !id.is_empty() {
                let mut context_window = None;
                let mut max_output_tokens = None;

                for section in res.css("section").iter() {
                    let label = section
                        .css("b")
                        .first()
                        .map(|b| b.text().to_lowercase())
                        .unwrap_or_default();
                    let value = section
                        .css("p")
                        .iter()
                        .nth(1)
                        .map(|p| p.text())
                        .unwrap_or_default();
                    if label.contains("input token limit") {
                        context_window = parse_token_count(value.trim());
                    } else if label.contains("output token limit") {
                        max_output_tokens = parse_token_count(value.trim());
                    }
                }

                let model = ScrapedModel {
                    id,
                    provider: "google".into(),
                    display_name: res.css("h1").first().map(|el| el.text().trim().to_string()),
                    context_window,
                    max_output_tokens,
                    input_price: None,
                    output_price: None,
                    source_url: res.url().to_string(),
                };
                output = output.items(vec![model]);
            }
        } else {
            let links: Vec<String> = res
                .css(r#"a[href*="/gemini-api/docs/models/"]"#)
                .iter()
                .filter_map(|el| el.attr("href"))
                .filter(|href| {
                    !href.ends_with("/models")
                        && !href.contains('#')
                        && (href.contains("/models/gemini") || href.contains("/models/gemma"))
                })
                .map(|href| {
                    if href.starts_with("http") {
                        href
                    } else {
                        format!("{}{}", BASE, href)
                    }
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            for url in links {
                output = output.follow(url);
            }
        }

        Ok(output)
    }
}

fn parse_token_count(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    // Plain comma-separated integer: "1,048,576"
    if let Ok(n) = trimmed.replace(',', "").parse::<u64>() {
        return Some(n);
    }
    let lower = trimmed.to_lowercase();
    // "1M", "1 million"
    if let Some(m_pos) = lower.find('m') {
        let before = lower[..m_pos].trim();
        if let Some(n) = before.split_whitespace().last().and_then(|s| s.replace(',', "").parse::<f64>().ok()) {
            return Some((n * 1_000_000.0) as u64);
        }
    }
    // "128k"
    if let Some(k_pos) = lower.find('k') {
        let before = lower[..k_pos].trim();
        if let Some(n) = before.split_whitespace().last().and_then(|s| s.replace(',', "").parse::<f64>().ok()) {
            return Some((n * 1_000.0) as u64);
        }
    }
    None
}
