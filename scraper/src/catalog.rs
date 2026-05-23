use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub provider: String,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    #[serde(default)]
    pub pricing: PricingEntry,
}

#[derive(Debug, Deserialize, Default)]
pub struct PricingEntry {
    pub input: Option<f64>,
    pub output: Option<f64>,
}

impl CatalogEntry {
    pub fn input_price(&self) -> Option<f64> {
        self.pricing.input
    }
    pub fn output_price(&self) -> Option<f64> {
        self.pricing.output
    }
}

pub fn load_catalog(models_dir: &Path) -> Result<HashMap<String, CatalogEntry>> {
    let mut map = HashMap::new();
    for entry in WalkDir::new(models_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml"))
    {
        let content = std::fs::read_to_string(entry.path())
            .with_context(|| format!("reading {}", entry.path().display()))?;
        let model: CatalogEntry = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {}", entry.path().display()))?;
        map.insert(model.id.clone(), model);
    }
    Ok(map)
}
