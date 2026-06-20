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

    // Collapse Aura voice variants to their base family id, and surface the bare
    // Flux family id — both are tracked in the catalog as bare family ids that the
    // raw extractor never produces. See `normalize_family_ids` for details.
    normalize_family_ids(&mut models);

    Ok(SpiderOutput::new().items(models))
}

/// Rewrites scraped Deepgram docs ids so they match the bare *family* ids the
/// catalog tracks, then de-duplicates.
///
/// Two transforms, both designed to avoid flooding the catalog with per-variant
/// stubs:
///
/// 1. **Aura voice collapse.** Deepgram's TTS docs expose Aura models only as
///    per-voice, per-language ids — `aura-2-thalia-en`, `aura-2-andromeda-en`,
///    … (100+ of them) — but the catalog tracks just the bare family `aura-2`
///    (and `aura-1`). We rewrite any `aura-N-<voice>-<lang>` id down to
///    `aura-N`, emitting ONLY the base family and never the voice variants.
///    A bare `aura-N` is already correct and passes through unchanged.
///
/// 2. **Flux family surfacing.** The catalog id is the bare `flux`, but the
///    generic extractor can never emit it: `flux` has no hyphen/dot/underscore
///    and so fails `looks_like_model_id`. Whenever the docs/pricing surface any
///    `flux-*` row (e.g. `flux-general-en`), we also emit the bare `flux`
///    family id. The `flux-*` ids themselves are preserved.
fn normalize_family_ids(models: &mut Vec<ScrapedModel>) {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(models.len());
    let mut has_flux_variant = false;

    for mut m in models.drain(..) {
        if let Some(base) = aura_base_family(&m.id) {
            m.id = base.to_string();
        }
        if m.id.starts_with("flux-") {
            has_flux_variant = true;
        }
        if seen.insert(m.id.clone()) {
            out.push(m);
        }
    }

    // Surface the bare Flux family id from any `flux-*` variant, reusing a
    // variant's source_url so the emitted stub points at a real page.
    if has_flux_variant && seen.insert("flux".to_string()) {
        let source_url = out
            .iter()
            .find(|m| m.id.starts_with("flux-"))
            .map(|m| m.source_url.clone())
            .unwrap_or_default();
        out.push(ScrapedModel {
            id: "flux".into(),
            provider: "deepgram".into(),
            source_url,
            ..Default::default()
        });
    }

    *models = out;
}

/// Returns the base Aura family id (`aura-1` / `aura-2`) for a per-voice variant
/// id like `aura-2-thalia-en`, or `None` if `id` is not an Aura variant that
/// should be collapsed (bare `aura-2` and non-Aura ids return `None`).
fn aura_base_family(id: &str) -> Option<&'static str> {
    for base in ["aura-2", "aura-1"] {
        // Match `aura-2-<something>` but NOT the bare `aura-2` itself.
        if let Some(rest) = id.strip_prefix(base)
            && rest.starts_with('-')
        {
            return Some(base);
        }
    }
    None
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

    // Surface the bare `flux` family id whenever the pricing page yields any
    // Flux row. The catalog tracks `flux` as a bare family id that the generic
    // extractor can never emit (no hyphen ⇒ fails `looks_like_model_id`), so the
    // pricing page is the most reliable place to anchor it. The diff aggregates
    // scraped ids across all pages, so emitting it here clears the
    // "missing from docs" false flag without depending on docs-page content.
    if items.iter().any(|m| m.id.starts_with("flux-")) {
        items.push(ScrapedModel {
            id: "flux".into(),
            provider: "deepgram".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(out: &SpiderOutput) -> Vec<String> {
        let mut v: Vec<String> = out.items.iter().map(|m| m.id.clone()).collect();
        v.sort();
        v
    }

    fn docs(body: &str) -> SpiderOutput {
        let res = HtmlResponse {
            url: DOCS_URL,
            body,
        };
        scrape_docs(&res).unwrap()
    }

    #[test]
    fn aura_base_family_collapses_voice_variants_only() {
        assert_eq!(aura_base_family("aura-2-thalia-en"), Some("aura-2"));
        assert_eq!(aura_base_family("aura-2-andromeda-en"), Some("aura-2"));
        assert_eq!(aura_base_family("aura-1-asteria-en"), Some("aura-1"));
        // Bare family ids are already correct and must NOT be collapsed/changed.
        assert_eq!(aura_base_family("aura-2"), None);
        assert_eq!(aura_base_family("aura-1"), None);
        // Non-Aura ids are untouched.
        assert_eq!(aura_base_family("nova-3-general"), None);
    }

    #[test]
    fn docs_collapses_aura_voice_variants_to_base_family() {
        // The TTS docs expose 100+ per-voice variants; we must emit ONLY the
        // base family `aura-2`, never the voice variants (no flooding).
        let html = "<code>aura-2-thalia-en</code> <code>aura-2-andromeda-en</code> \
                    <code>aura-2-hera-en</code> <code>aura-1-asteria-en</code>";
        let out = docs(html);
        let got = ids(&out);
        assert!(got.contains(&"aura-2".to_string()), "expected base aura-2: {got:?}");
        assert!(got.contains(&"aura-1".to_string()), "expected base aura-1: {got:?}");
        // No per-voice variant should survive.
        assert!(
            !got.iter().any(|id| id.starts_with("aura-2-") || id.starts_with("aura-1-")),
            "voice variants leaked through: {got:?}"
        );
    }

    #[test]
    fn docs_keeps_bare_aura_family_when_already_present() {
        let html = "<code>aura-2</code> <code>aura-2-thalia-en</code>";
        let out = docs(html);
        // Collapse + dedup ⇒ exactly one `aura-2`.
        assert_eq!(ids(&out), vec!["aura-2".to_string()]);
    }

    #[test]
    fn docs_surfaces_bare_flux_from_flux_variant() {
        let html = "<code>flux-general-en</code>";
        let out = docs(html);
        let got = ids(&out);
        assert!(got.contains(&"flux".to_string()), "bare flux not surfaced: {got:?}");
        assert!(got.contains(&"flux-general-en".to_string()), "variant dropped: {got:?}");
    }

    #[test]
    fn docs_no_flux_when_no_flux_variant() {
        let html = "<code>nova-3-general</code>";
        let out = docs(html);
        assert!(!ids(&out).contains(&"flux".to_string()));
    }

    #[test]
    fn pricing_surfaces_bare_flux_family() {
        // Minimal pricing HTML: a Flux English row with two $/min tiers.
        let html = "<div>turn detection, natural interruption</div>\
                    <span>$0.0065/min</span><span>$0.0050/min</span>";
        let res = HtmlResponse {
            url: PRICING_URL,
            body: html,
        };
        let out = scrape_pricing(&res).unwrap();
        let got = ids(&out);
        assert!(got.contains(&"flux".to_string()), "bare flux not surfaced: {got:?}");
        assert!(got.contains(&"flux-general-en".to_string()), "variant dropped: {got:?}");
    }
}
