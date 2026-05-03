use crate::catalog::{Model, Provider};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path};
use walkdir::WalkDir;

pub struct AppState {
    pub providers: Vec<Provider>,
    pub models: Vec<Model>,
    pub aliases: HashMap<String, String>,
    pub etag: String,
}

pub fn load_state(data_dir: &Path) -> Result<AppState> {
    let providers_dir = data_dir.join("providers");
    let models_dir = data_dir.join("models");
    let aliases_path = data_dir.join("aliases.yaml");

    ensure_dir(&providers_dir)?;
    ensure_dir(&models_dir)?;
    ensure_file(&aliases_path)?;

    let providers = load_yaml_dir(&providers_dir)?;
    let models = load_yaml_recursive(&models_dir)?;
    let aliases = load_aliases(&aliases_path)?;

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

fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!(
            "required catalog directory '{}' does not exist",
            path.display()
        );
    }
    if !path.is_dir() {
        bail!(
            "required catalog path '{}' is not a directory",
            path.display()
        );
    }
    Ok(())
}

fn ensure_file(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("required catalog file '{}' does not exist", path.display());
    }
    if !path.is_file() {
        bail!("required catalog path '{}' is not a file", path.display());
    }
    Ok(())
}

fn load_yaml_dir<T>(dir: &Path) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
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

fn load_yaml_recursive<T>(dir: &Path) -> Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_state_fails_when_required_catalog_paths_are_missing() {
        let data_dir = std::env::temp_dir().join(format!(
            "almanac-missing-catalog-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&data_dir).unwrap();

        let err = match load_state(&data_dir) {
            Ok(_) => panic!("load_state should fail for missing catalog paths"),
            Err(error) => error,
        };

        assert!(err.to_string().contains("required catalog directory"));
        std::fs::remove_dir(&data_dir).unwrap();
    }
}
