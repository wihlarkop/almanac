use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use walkdir::WalkDir;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("validator/ must have a parent directory")
        .to_path_buf()
}

fn load_schema(path: &Path) -> Result<jsonschema::Validator> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading schema {}", path.display()))?;
    let schema: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing schema {}", path.display()))?;
    jsonschema::validator_for(&schema)
        .with_context(|| format!("compiling schema {}", path.display()))
}

fn load_yaml(path: &Path) -> Result<serde_json::Value> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    serde_yaml::from_str(&raw)
        .with_context(|| format!("parsing YAML {}", path.display()))
}

fn yaml_files_in(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return vec![];
    }
    let mut paths: Vec<_> = WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "yaml"))
        .map(|e| e.path().to_path_buf())
        .collect();
    paths.sort();
    paths
}

fn yaml_files_recursive(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return vec![];
    }
    let mut paths: Vec<_> = WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "yaml"))
        .map(|e| e.path().to_path_buf())
        .collect();
    paths.sort();
    paths
}

fn stem(path: &Path) -> &str {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or_default()
}

fn main() -> Result<()> {
    let root = repo_root();
    let provider_validator = load_schema(&root.join("schema/provider.schema.json"))?;
    let model_validator = load_schema(&root.join("schema/model.schema.json"))?;

    let mut errors: Vec<String> = Vec::new();
    let mut provider_ids: HashSet<String> = HashSet::new();

    // --- Providers ---
    println!("Validating providers...");
    for path in yaml_files_in(&root.join("providers")) {
        let rel = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
        match load_yaml(&path) {
            Err(e) => {
                println!("  \u{2717} {rel}: {e}");
                errors.push(format!("{rel}: {e}"));
            }
            Ok(data) => {
                let errs: Vec<_> = provider_validator.iter_errors(&data).collect();
                if !errs.is_empty() {
                    for e in errs {
                        println!("  \u{2717} {rel}: {e}");
                        errors.push(format!("{rel}: {e}"));
                    }
                } else {
                    let expected = stem(&path);
                    let actual = data["id"].as_str().unwrap_or_default();
                    if actual != expected {
                        let msg = format!("{rel}: id '{actual}' must match filename '{expected}'");
                        println!("  \u{2717} {msg}");
                        errors.push(msg);
                    } else {
                        provider_ids.insert(actual.to_string());
                        println!("  \u{2713} {rel}");
                    }
                }
            }
        }
    }

    // --- Models ---
    println!("\nValidating models...");
    for path in yaml_files_recursive(&root.join("models")) {
        let rel = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
        match load_yaml(&path) {
            Err(e) => {
                println!("  \u{2717} {rel}: {e}");
                errors.push(format!("{rel}: {e}"));
            }
            Ok(data) => {
                let errs: Vec<_> = model_validator.iter_errors(&data).collect();
                if !errs.is_empty() {
                    for e in errs {
                        println!("  \u{2717} {rel}: {e}");
                        errors.push(format!("{rel}: {e}"));
                    }
                } else {
                    let expected = stem(&path);
                    let actual = data["id"].as_str().unwrap_or_default();
                    if actual != expected {
                        let msg = format!("{rel}: id '{actual}' must match filename '{expected}'");
                        println!("  \u{2717} {msg}");
                        errors.push(msg);
                    } else {
                        let provider = data["provider"].as_str().unwrap_or_default();
                        if !provider_ids.is_empty() && !provider_ids.contains(provider) {
                            let msg = format!("{rel}: provider '{provider}' not found in providers/");
                            println!("  \u{2717} {msg}");
                            errors.push(msg);
                        } else {
                            println!("  \u{2713} {rel}");
                        }
                    }
                }
            }
        }
    }

    // --- Aliases ---
    println!("\nValidating aliases...");
    let aliases_path = root.join("aliases.yaml");
    if aliases_path.exists() {
        let known_ids: HashSet<String> = yaml_files_recursive(&root.join("models"))
            .iter()
            .map(|p| stem(p).to_string())
            .collect();

        match load_yaml(&aliases_path) {
            Err(e) => {
                println!("  \u{2717} aliases.yaml: {e}");
                errors.push(format!("aliases.yaml: {e}"));
            }
            Ok(data) => {
                if let Some(aliases) = data["aliases"].as_object() {
                    let mut entries: Vec<_> = aliases.iter().collect();
                    entries.sort_by_key(|(k, _)| k.as_str());
                    for (alias, target) in entries {
                        let t = target.as_str().unwrap_or_default();
                        if !known_ids.contains(t) {
                            let msg = format!("aliases.yaml: '{alias}' \u{2192} '{t}' does not match any model id");
                            println!("  \u{2717} {msg}");
                            errors.push(msg);
                        } else {
                            println!("  \u{2713} {alias} \u{2192} {t}");
                        }
                    }
                }
            }
        }
    }

    // --- Summary ---
    let provider_count = yaml_files_in(&root.join("providers")).len();
    let model_count = yaml_files_recursive(&root.join("models")).len();
    let alias_count = if aliases_path.exists() { 1 } else { 0 };
    let total = provider_count + model_count + alias_count;

    println!("\n{}", "\u{2500}".repeat(40));
    if errors.is_empty() {
        println!("All {total} file(s) valid");
        Ok(())
    } else {
        println!("Found {} error(s) across {total} file(s)", errors.len());
        std::process::exit(1);
    }
}
