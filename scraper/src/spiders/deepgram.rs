use crate::model::ScrapedModel;
use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

const DOCS_URL: &str = "https://developers.deepgram.com/docs/models-languages-overview";
const PRICING_URL: &str = "https://deepgram.com/pricing";

pub struct DeepgramSpider;

#[async_trait::async_trait]
impl Spider for DeepgramSpider {
    fn name(&self) -> &str {
        "deepgram"
    }

    fn start_urls(&self) -> Vec<String> {
        vec![DOCS_URL.into(), PRICING_URL.into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        if res.url.contains("pricing") {
            scrape_pricing(res)
        } else {
            scrape_docs(res)
        }
    }
}

// ── Docs page: extract active model IDs ──────────────────────────────────────

fn scrape_docs(res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
    let mut models = extract_model_ids(res.body, "deepgram", res.url);
    // Keep only Deepgram STT/TTS model families; discard language codes and
    // UI artifacts that slip through the generic extractor.
    models.retain(|m| {
        let id = m.id.as_str();
        id.starts_with("nova-")
            || id.starts_with("flux-")
            || id.starts_with("base-")
            || id.starts_with("enhanced-")
            || id.starts_with("whisper-")
            || id.starts_with("aura-")
    });
    Ok(SpiderOutput::new().items(models))
}

// ── Pricing page: extract per-minute PAYG rates ──────────────────────────────

fn scrape_pricing(res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
    let html = res.body;
    let mut items = Vec::new();

    // Each tuple: (catalog_model_id, unique keyword from the tier description)
    // We capture the *first* $X.XXXX/min that appears after the keyword, which
    // is always the PAYG streaming rate in Deepgram's pricing table layout.
    let tiers: &[(&str, &str)] = &[
        // Flux English — "turn detection" appears only in the Flux English row
        ("flux-general-en", "turn detection, natural interruption"),
        // Flux Multilingual — only multilingual row has this phrase
        (
            "flux-general-multi",
            "multiple languages within a single conversation",
        ),
        // Nova-3 Monolingual — "highest performing model" is its description header
        ("nova-3-general", "highest performing model"),
    ];

    for &(id, keyword) in tiers {
        if let Some(price) = first_per_min_price_after(html, keyword) {
            items.push(ScrapedModel {
                id: id.into(),
                provider: "deepgram".into(),
                display_name: None,
                context_window: None,
                max_output_tokens: None,
                // per_minute rate stored in input_price for drift detection;
                // catalog.rs CatalogEntry::input_price() falls back to per_minute.
                input_price: Some(price),
                output_price: None,
                source_url: res.url.into(),
            });
        }
    }

    Ok(SpiderOutput::new().items(items))
}

/// Scans `html` from the position of `keyword` and returns the numeric value
/// of the first `$X.XXXX/min` token found after that position.
fn first_per_min_price_after(html: &str, keyword: &str) -> Option<f64> {
    let offset = html.find(keyword)?;
    parse_first_per_min_price(&html[offset..])
}

fn parse_first_per_min_price(s: &str) -> Option<f64> {
    let mut search = s;
    loop {
        let dollar = search.find('$')?;
        let after = &search[dollar + 1..];
        let num_end = after
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(after.len());
        let num_str = &after[..num_end];
        let rest = &after[num_end..];
        if rest.starts_with("/min") && !num_str.is_empty() {
            if let Ok(v) = num_str.parse::<f64>() {
                if (0.001..=0.5).contains(&v) {
                    return Some(v);
                }
            }
        }
        // Advance past this dollar sign and keep searching.
        search = &search[dollar + 1..];
    }
}
