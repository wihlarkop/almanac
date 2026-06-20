use crate::catalog::CatalogEntry;
use crate::model::ScrapedModel;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub enum DiffResult {
    New(ScrapedModel),
    PriceChanged {
        model: ScrapedModel,
        old_input: Option<f64>,
        old_output: Option<f64>,
        kind: PriceChangeKind,
    },
    /// An active catalog entry for this provider was not found in the current scrape.
    /// This may mean the model was removed from docs (possibly deprecated upstream).
    MissingFromDocs {
        provider: String,
        id: String,
    },
}

/// How a scraped price that differs from the catalog should be treated,
/// based on the catalog entry's confidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceChangeKind {
    /// Catalog entry is an `inferred`/stub with no human-verified price —
    /// the scraped value is a safe enrichment.
    Enrich,
    /// Catalog entry is `official` (human-verified) — the scraped value differs,
    /// so flag it for review but do NOT overwrite automatically.
    DriftAlert,
}

pub fn diff(scraped: &[ScrapedModel], catalog: &HashMap<String, CatalogEntry>) -> Vec<DiffResult> {
    let mut results = Vec::new();

    // Collect scraped IDs grouped by provider.
    let mut scraped_by_provider: HashMap<&str, HashSet<&str>> = HashMap::new();
    for model in scraped {
        scraped_by_provider
            .entry(model.provider.as_str())
            .or_default()
            .insert(model.id.as_str());
    }

    for model in scraped {
        match catalog.get(&model.id) {
            None => results.push(DiffResult::New(model.clone())),
            Some(entry) => {
                let input_changed = input_drifted(model, entry.input_price());
                let output_changed = differs_price(model.output_price, entry.output_price());
                if input_changed || output_changed {
                    let kind = if entry.is_official() {
                        PriceChangeKind::DriftAlert
                    } else {
                        PriceChangeKind::Enrich
                    };
                    results.push(DiffResult::PriceChanged {
                        model: model.clone(),
                        old_input: entry.input_price(),
                        old_output: entry.output_price(),
                        kind,
                    });
                }
            }
        }
    }

    // For each provider that was scraped, flag active catalog entries that are
    // now absent — but only when the scrape was complete enough to trust an
    // absence. Many spiders only discover a subset of a provider's model IDs
    // (JS-heavy pages, partial doc tables); for those, "missing" is just an
    // extraction gap, not a deprecation, so we suppress it below the coverage
    // threshold to avoid flooding the report with false positives.
    for (provider, scraped_ids) in &scraped_by_provider {
        let active: Vec<&CatalogEntry> = catalog
            .values()
            .filter(|e| e.provider == *provider && e.status != "deprecated")
            .collect();
        if active.is_empty() {
            continue;
        }
        let matched = active
            .iter()
            .filter(|e| scraped_ids.contains(e.id.as_str()))
            .count();
        let coverage = matched as f64 / active.len() as f64;
        if coverage < MISSING_COVERAGE_THRESHOLD {
            continue;
        }
        for entry in active {
            if !scraped_ids.contains(entry.id.as_str()) {
                results.push(DiffResult::MissingFromDocs {
                    provider: provider.to_string(),
                    id: entry.id.clone(),
                });
            }
        }
    }

    results
}

/// A scrape must cover at least this fraction of a provider's active catalog
/// before we trust that a *missing* model signals an upstream removal rather
/// than an incomplete extraction.
const MISSING_COVERAGE_THRESHOLD: f64 = 0.7;

fn differs_price(scraped: Option<f64>, catalog: Option<f64>) -> bool {
    match (scraped, catalog) {
        (Some(s), Some(c)) => (s - c).abs() > 0.001,
        (Some(_), None) => true,
        _ => false,
    }
}

/// The scraped input price has drifted from the catalog only if the catalog
/// value matches NONE of the scraped tiers (primary `input_price` plus any
/// `input_price_candidates`). This keeps multi-tier pricing tables from flagging
/// drift just because the catalog tracks a different published tier.
fn input_drifted(model: &ScrapedModel, catalog: Option<f64>) -> bool {
    let scraped: Vec<f64> = model
        .input_price
        .into_iter()
        .chain(model.input_price_candidates.iter().copied())
        .collect();
    match catalog {
        // Catalog has no price yet: drift (enrichment) if the scrape found one.
        None => !scraped.is_empty(),
        Some(c) => !scraped.is_empty() && !scraped.iter().any(|s| (s - c).abs() <= 0.001),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::PricingEntry;

    fn catalog_entry(
        id: &str,
        confidence: &str,
        input: Option<f64>,
        output: Option<f64>,
    ) -> CatalogEntry {
        CatalogEntry {
            id: id.into(),
            provider: "moonshot".into(),
            status: "active".into(),
            context_window: None,
            max_output_tokens: None,
            pricing: PricingEntry {
                input,
                output,
                per_minute: None,
                per_million_chars: None,
            },
            confidence: confidence.into(),
        }
    }

    fn scraped(id: &str, input: Option<f64>, output: Option<f64>) -> ScrapedModel {
        ScrapedModel {
            id: id.into(),
            provider: "moonshot".into(),
            input_price: input,
            output_price: output,
            source_url: "https://example.com".into(),
            ..Default::default()
        }
    }

    fn catalog_of(entries: Vec<CatalogEntry>) -> HashMap<String, CatalogEntry> {
        entries.into_iter().map(|e| (e.id.clone(), e)).collect()
    }

    #[test]
    fn catalog_matching_any_scraped_tier_is_not_drift() {
        // Provider lists two tiers (0.0048 / 0.0077); catalog tracks the 2nd.
        let catalog = catalog_of(vec![catalog_entry("nova", "official", Some(0.0077), None)]);
        let mut m = scraped("nova", Some(0.0048), None);
        m.input_price_candidates = vec![0.0077];
        let results = diff(&[m], &catalog);
        assert!(
            !results
                .iter()
                .any(|r| matches!(r, DiffResult::PriceChanged { .. })),
            "catalog value matches a published tier — no drift expected"
        );
    }

    #[test]
    fn catalog_matching_no_scraped_tier_is_drift() {
        let catalog = catalog_of(vec![catalog_entry("nova", "official", Some(0.99), None)]);
        let mut m = scraped("nova", Some(0.0048), None);
        m.input_price_candidates = vec![0.0077];
        let results = diff(&[m], &catalog);
        assert!(
            results
                .iter()
                .any(|r| matches!(r, DiffResult::PriceChanged { .. })),
            "catalog value matches no published tier — drift expected"
        );
    }

    #[test]
    fn price_drift_on_official_entry_is_a_drift_alert() {
        let catalog = catalog_of(vec![catalog_entry(
            "kimi",
            "official",
            Some(0.95),
            Some(4.00),
        )]);
        let results = diff(&[scraped("kimi", Some(1.90), Some(8.00))], &catalog);
        match results.as_slice() {
            [
                DiffResult::PriceChanged {
                    kind, old_input, ..
                },
            ] => {
                assert_eq!(*kind, PriceChangeKind::DriftAlert);
                assert_eq!(*old_input, Some(0.95));
            }
            other => panic!("expected one DriftAlert PriceChanged, got {other:?}"),
        }
    }

    #[test]
    fn price_drift_on_inferred_stub_is_enrichable() {
        // A stub with placeholder 0/0 pricing that now has a real scraped price.
        let catalog = catalog_of(vec![catalog_entry(
            "kimi",
            "inferred",
            Some(0.0),
            Some(0.0),
        )]);
        let results = diff(&[scraped("kimi", Some(0.95), Some(4.00))], &catalog);
        match results.as_slice() {
            [DiffResult::PriceChanged { kind, .. }] => {
                assert_eq!(*kind, PriceChangeKind::Enrich);
            }
            other => panic!("expected one Enrich PriceChanged, got {other:?}"),
        }
    }

    #[test]
    fn matching_price_produces_no_change() {
        let catalog = catalog_of(vec![catalog_entry(
            "kimi",
            "official",
            Some(0.95),
            Some(4.00),
        )]);
        let results = diff(&[scraped("kimi", Some(0.95), Some(4.00))], &catalog);
        assert!(
            !results
                .iter()
                .any(|r| matches!(r, DiffResult::PriceChanged { .. })),
            "no PriceChanged expected when prices match"
        );
    }

    fn five_active_models() -> HashMap<String, CatalogEntry> {
        catalog_of(
            ["m1", "m2", "m3", "m4", "m5"]
                .iter()
                .map(|id| catalog_entry(id, "official", Some(1.0), Some(1.0)))
                .collect(),
        )
    }

    #[test]
    fn missing_from_docs_suppressed_when_coverage_is_low() {
        // Scrape found only 1 of 5 catalog models (20%) — the spider clearly
        // didn't extract everything, so absences are not trustworthy.
        let results = diff(
            &[scraped("m1", Some(1.0), Some(1.0))],
            &five_active_models(),
        );
        assert!(
            !results
                .iter()
                .any(|r| matches!(r, DiffResult::MissingFromDocs { .. })),
            "missing should be suppressed below the coverage threshold"
        );
    }

    #[test]
    fn missing_from_docs_emitted_when_coverage_is_high() {
        // Found 4 of 5 (80%); m5 is genuinely absent from the scrape.
        let found: Vec<ScrapedModel> = ["m1", "m2", "m3", "m4"]
            .iter()
            .map(|id| scraped(id, Some(1.0), Some(1.0)))
            .collect();
        let results = diff(&found, &five_active_models());
        let missing: Vec<&str> = results
            .iter()
            .filter_map(|r| match r {
                DiffResult::MissingFromDocs { id, .. } => Some(id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(missing, vec!["m5"]);
    }
}
