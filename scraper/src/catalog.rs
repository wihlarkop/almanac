use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub provider: String,
    #[serde(default)]
    pub status: String,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    #[serde(default)]
    pub pricing: PricingEntry,
    /// Catalog confidence ("official", "inferred", "community", ...). Drives how
    /// the diff treats scraped price drift: verified `official` values are
    /// protected (drift is only flagged for human review), while `inferred`
    /// stubs are safe to enrich.
    #[serde(default)]
    pub confidence: String,
}

impl CatalogEntry {
    /// Whether this entry's data is human-verified and must not be overwritten
    /// by a scraped value without review.
    pub fn is_official(&self) -> bool {
        self.confidence == "official"
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PricingEntry {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub per_minute: Option<f64>,
    pub per_million_chars: Option<f64>,
}

impl CatalogEntry {
    pub fn input_price(&self) -> Option<f64> {
        // Audio providers store per_minute instead of per-token pricing;
        // treat it as the comparable "input" unit for drift detection.
        self.pricing.input.or(self.pricing.per_minute)
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
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "yaml"))
    {
        let content = std::fs::read_to_string(entry.path())
            .with_context(|| format!("reading {}", entry.path().display()))?;
        let model: CatalogEntry = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {}", entry.path().display()))?;
        map.insert(model.id.clone(), model);
    }
    Ok(map)
}
