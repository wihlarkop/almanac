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
    },
    /// An active catalog entry for this provider was not found in the current scrape.
    /// This may mean the model was removed from docs (possibly deprecated upstream).
    MissingFromDocs {
        provider: String,
        id: String,
    },
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
                    results.push(DiffResult::PriceChanged {
                        model: model.clone(),
                        old_input: entry.input_price(),
                        old_output: entry.output_price(),
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
