use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path};
use walkdir::WalkDir;

pub struct AppState {
    pub providers: Vec<serde_json::Value>,
    pub models: Vec<serde_json::Value>,
    pub aliases: HashMap<String, String>,
    pub etag: String,
}

pub fn load_state(data_dir: &Path) -> Result<AppState> {
    let providers = load_yaml_dir(&data_dir.join("providers"))?;
    let models = load_yaml_recursive(&data_dir.join("models"))?;
    let aliases = load_aliases(&data_dir.join("aliases.yaml"))?;

    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_string(&providers)?.as_bytes());
    hasher.update(serde_json::to_string(&models)?.as_bytes());
    let etag = format!("\"{}\"", hex::encode(hasher.finalize()));

    Ok(AppState {
        providers,
        models,
        aliases,
        etag,
    })
}

fn load_yaml_dir(dir: &Path) -> Result<Vec<serde_json::Value>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            serde_yaml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
        })
        .collect()
}

fn load_yaml_recursive(dir: &Path) -> Result<Vec<serde_json::Value>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut paths: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "yaml"))
        .map(|e| e.path().to_path_buf())
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let raw = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            serde_yaml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
        })
        .collect()
}

fn load_aliases(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let val: serde_json::Value =
        serde_yaml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    let mut aliases = HashMap::new();
    if let Some(map) = val["aliases"].as_object() {
        for (k, v) in map {
            if let Some(s) = v.as_str() {
                aliases.insert(k.clone(), s.to_string());
            }
        }
    }
    Ok(aliases)
}
