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
                let input_changed = differs_price(model.input_price, entry.input_price());
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

    // For each provider that was scraped, flag active catalog entries that are now absent.
    for (provider, scraped_ids) in &scraped_by_provider {
        for entry in catalog.values() {
            if entry.provider != *provider {
                continue;
            }
            // Only flag active models — already-deprecated entries are expected to be absent.
            if entry.status == "deprecated" {
                continue;
            }
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

fn differs_price(scraped: Option<f64>, catalog: Option<f64>) -> bool {
    match (scraped, catalog) {
        (Some(s), Some(c)) => (s - c).abs() > 0.001,
        (Some(_), None) => true,
        _ => false,
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
            display_name: None,
            context_window: None,
            max_output_tokens: None,
            input_price: input,
            output_price: output,
            source_url: "https://example.com".into(),
        }
    }

    fn catalog_of(entries: Vec<CatalogEntry>) -> HashMap<String, CatalogEntry> {
        entries.into_iter().map(|e| (e.id.clone(), e)).collect()
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
}
