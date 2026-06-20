use crate::model::ScrapedModel;
use crate::pricing::all_unit_prices;
use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use crate::spiders::doc_page::extract_model_ids;
use anyhow::Result;

const DOCS_URL: &str = "https://developers.deepgram.com/docs/models-languages-overview";
const PRICING_URL: &str = "https://deepgram.com/pricing";

/// Sanity range for Deepgram per-minute PAYG rates ($/min).
const PER_MIN_RANGE: std::ops::RangeInclusive<f64> = 0.001..=0.5;

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

    // Each tuple: (catalog_model_id, unique keyword from the tier description).
    // Deepgram lists two billing tiers ($/min) side by side per model row
    // (pay-as-you-go and growth), in an order that varies by row, so we capture
    // *all* the rates in the row and let the diff match the catalog against any
    // of them — avoiding false drift from picking the wrong column.
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
        let mut rates = per_min_rates_after(html, keyword);
        if rates.is_empty() {
            continue;
        }
        let primary = rates.remove(0);
        items.push(ScrapedModel {
            id: id.into(),
            provider: "deepgram".into(),
            // per_minute rate stored in input_price for drift detection;
            // catalog.rs CatalogEntry::input_price() falls back to per_minute.
            input_price: Some(primary),
            input_price_candidates: rates,
            source_url: res.url.into(),
            ..Default::default()
        });
    }

    Ok(SpiderOutput::new().items(items))
}

/// Number of $/min tiers Deepgram lists per model row (pay-as-you-go + growth).
const TIERS_PER_ROW: usize = 2;

/// Collects the per-minute rates listed for the model whose description contains
/// `keyword`. Each row shows exactly [`TIERS_PER_ROW`] tiers; we scan a window
/// large enough to include both (the second sits ~450 raw chars in) and take
/// only the first two, so the following row's rates can never bleed in.
fn per_min_rates_after(html: &str, keyword: &str) -> Vec<f64> {
    let Some(offset) = html.find(keyword) else {
        return Vec::new();
    };
    let end = (offset + 700).min(html.len());
    all_unit_prices(&html[offset..end], "/min", &PER_MIN_RANGE)
        .into_iter()
        .take(TIERS_PER_ROW)
        .collect()
}
