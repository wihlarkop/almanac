use crate::catalog::CatalogEntry;
use crate::model::ScrapedModel;
use std::collections::HashMap;

#[derive(Debug)]
pub enum DiffResult {
    New(ScrapedModel),
    PriceChanged {
        model: ScrapedModel,
        old_input: Option<f64>,
        old_output: Option<f64>,
    },
}

pub fn diff(scraped: &[ScrapedModel], catalog: &HashMap<String, CatalogEntry>) -> Vec<DiffResult> {
    let mut results = Vec::new();
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
    results
}

fn differs_price(scraped: Option<f64>, catalog: Option<f64>) -> bool {
    match (scraped, catalog) {
        (Some(s), Some(c)) => (s - c).abs() > 0.001,
        (Some(_), None) => true,
        _ => false,
    }
}
