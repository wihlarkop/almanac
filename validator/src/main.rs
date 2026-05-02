use std::{
    collections::{HashMap, HashSet},
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
    let schema: serde_json::Value =
        serde_json::from_str(&raw).with_context(|| format!("parsing schema {}", path.display()))?;
    jsonschema::validator_for(&schema)
        .with_context(|| format!("compiling schema {}", path.display()))
}

fn load_yaml(path: &Path) -> Result<serde_json::Value> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_yaml::from_str(&raw).with_context(|| format!("parsing YAML {}", path.display()))
}

fn yaml_files_in(dir: &Path) -> Vec<PathBuf> {
    if !dir.exists() {
        return vec![];
    }
    let mut paths: Vec<_> = WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "yaml"))
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
        .filter(|e| e.path().extension().is_some_and(|x| x == "yaml"))
        .map(|e| e.path().to_path_buf())
        .collect();
    paths.sort();
    paths
}

fn stem(path: &Path) -> &str {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
}

#[derive(Clone)]
struct ModelMeta {
    path: PathBuf,
    status: String,
    replacement: Option<String>,
    release_date: Option<String>,
    deprecation_date: Option<String>,
    sunset_date: Option<String>,
    parameters_supported: Vec<String>,
    parameters_rejected: Vec<String>,
    parameters_deprecated: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
struct FreshnessStats {
    total_models: usize,
    stale_models: usize,
    missing_pricing: usize,
    missing_last_verified: usize,
}

fn optional_string(data: &serde_json::Value, key: &str) -> Option<String> {
    data[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn string_array(data: &serde_json::Value, path: &[&str]) -> Vec<String> {
    let mut current = data;
    for key in path {
        current = &current[*key];
    }
    current
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn report_error(errors: &mut Vec<String>, message: String) {
    println!("  \u{2717} {message}");
    errors.push(message);
}

fn validate_date_order(
    errors: &mut Vec<String>,
    rel: &str,
    left_name: &str,
    left: &Option<String>,
    right_name: &str,
    right: &Option<String>,
) {
    if let (Some(left), Some(right)) = (left, right) {
        if left > right {
            report_error(
                errors,
                format!("{rel}: {left_name} '{left}' must be on or before {right_name} '{right}'"),
            );
        }
    }
}

fn validate_no_parameter_overlap(errors: &mut Vec<String>, rel: &str, meta: &ModelMeta) {
    let supported: HashSet<_> = meta.parameters_supported.iter().collect();
    let rejected: HashSet<_> = meta.parameters_rejected.iter().collect();
    let deprecated: HashSet<_> = meta.parameters_deprecated.iter().collect();

    for param in supported.intersection(&rejected) {
        report_error(
            errors,
            format!("{rel}: parameter '{param}' cannot be both supported and rejected"),
        );
    }

    for param in supported.intersection(&deprecated) {
        report_error(
            errors,
            format!(
                "{rel}: parameter '{param}' cannot be both supported and deprecated for this model"
            ),
        );
    }

    for param in rejected.intersection(&deprecated) {
        report_error(
            errors,
            format!(
                "{rel}: parameter '{param}' cannot be both rejected and deprecated for this model"
            ),
        );
    }
}

fn parse_ymd(value: &str) -> Option<(i32, u32, u32)> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let day = day as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as i64
}

fn days_between(left: &str, right: &str) -> Option<i64> {
    let (left_year, left_month, left_day) = parse_ymd(left)?;
    let (right_year, right_month, right_day) = parse_ymd(right)?;
    Some(
        days_from_civil(right_year, right_month, right_day)
            - days_from_civil(left_year, left_month, left_day),
    )
}

fn freshness_stats(
    models: &[serde_json::Value],
    today: &str,
    stale_after_days: i64,
) -> FreshnessStats {
    let mut stats = FreshnessStats {
        total_models: models.len(),
        stale_models: 0,
        missing_pricing: 0,
        missing_last_verified: 0,
    };

    for model in models {
        if model["pricing"].as_object().is_none() {
            stats.missing_pricing += 1;
        }

        match model["last_verified"].as_str() {
            None => stats.missing_last_verified += 1,
            Some(last_verified) => {
                if days_between(last_verified, today).unwrap_or(0) > stale_after_days {
                    stats.stale_models += 1;
                }
            }
        }
    }

    stats
}

fn main() -> Result<()> {
    let root = repo_root();
    let provider_validator = load_schema(&root.join("schema/provider.schema.json"))?;
    let model_validator = load_schema(&root.join("schema/model.schema.json"))?;

    let mut errors: Vec<String> = Vec::new();
    let mut provider_ids: HashSet<String> = HashSet::new();
    let mut model_ids: HashMap<String, String> = HashMap::new();
    let mut model_meta: HashMap<String, ModelMeta> = HashMap::new();
    let mut model_values: Vec<serde_json::Value> = Vec::new();

    // --- Providers ---
    println!("Validating providers...");
    for path in yaml_files_in(&root.join("providers")) {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
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
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
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
                            let msg =
                                format!("{rel}: provider '{provider}' not found in providers/");
                            println!("  \u{2717} {msg}");
                            errors.push(msg);
                        } else {
                            let provider_dir = path
                                .parent()
                                .and_then(|p| p.file_name())
                                .and_then(|s| s.to_str())
                                .unwrap_or_default();

                            if provider != provider_dir {
                                report_error(
                                    &mut errors,
                                    format!("{rel}: provider '{provider}' must match directory '{provider_dir}'"),
                                );
                                continue;
                            }

                            if let Some(previous) = model_ids.get(actual) {
                                report_error(
                                    &mut errors,
                                    format!("{rel}: duplicate model id '{actual}' already defined in {previous}"),
                                );
                                continue;
                            }

                            model_ids.insert(actual.to_string(), rel.clone());
                            model_meta.insert(
                                actual.to_string(),
                                ModelMeta {
                                    path: path.clone(),
                                    status: data["status"].as_str().unwrap_or_default().to_string(),
                                    replacement: optional_string(&data, "replacement"),
                                    release_date: optional_string(&data, "release_date"),
                                    deprecation_date: optional_string(&data, "deprecation_date"),
                                    sunset_date: optional_string(&data, "sunset_date"),
                                    parameters_supported: string_array(
                                        &data,
                                        &["parameters", "supported"],
                                    ),
                                    parameters_rejected: string_array(
                                        &data,
                                        &["parameters", "rejected"],
                                    ),
                                    parameters_deprecated: string_array(
                                        &data,
                                        &["parameters", "deprecated_for_this_model"],
                                    ),
                                },
                            );
                            model_values.push(data);
                            println!("  \u{2713} {rel}");
                        }
                    }
                }
            }
        }
    }

    // --- Aliases ---
    for meta in model_meta.values() {
        let rel = rel_path(&root, &meta.path);

        if let Some(replacement) = &meta.replacement {
            if !model_meta.contains_key(replacement) {
                report_error(
                    &mut errors,
                    format!("{rel}: replacement '{replacement}' does not match any model id"),
                );
            }
        }

        validate_date_order(
            &mut errors,
            &rel,
            "release_date",
            &meta.release_date,
            "deprecation_date",
            &meta.deprecation_date,
        );
        validate_date_order(
            &mut errors,
            &rel,
            "deprecation_date",
            &meta.deprecation_date,
            "sunset_date",
            &meta.sunset_date,
        );
        validate_date_order(
            &mut errors,
            &rel,
            "release_date",
            &meta.release_date,
            "sunset_date",
            &meta.sunset_date,
        );
        validate_no_parameter_overlap(&mut errors, &rel, meta);
    }

    println!("\nValidating aliases...");
    let aliases_path = root.join("aliases.yaml");
    if aliases_path.exists() {
        let known_ids: HashSet<String> = model_meta.keys().cloned().collect();

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
                        } else if let Some(target) = model_meta.get(t) {
                            if target.status == "retired" {
                                let msg = format!(
                                    "aliases.yaml: '{alias}' \u{2192} '{t}' points to retired model"
                                );
                                println!("  \u{2717} {msg}");
                                errors.push(msg);
                                continue;
                            }
                            println!("  \u{2713} {alias} \u{2192} {t}");
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
    let freshness = freshness_stats(&model_values, "2026-05-03", 90);
    println!(
        "Freshness report: {} model(s), {} stale > 90 days, {} missing pricing, {} missing last_verified",
        freshness.total_models,
        freshness.stale_models,
        freshness.missing_pricing,
        freshness.missing_last_verified
    );

    if errors.is_empty() {
        println!("All {total} file(s) valid");
        Ok(())
    } else {
        println!("Found {} error(s) across {total} file(s)", errors.len());
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn meta_with_parameters(
        supported: Vec<&str>,
        rejected: Vec<&str>,
        deprecated: Vec<&str>,
    ) -> ModelMeta {
        ModelMeta {
            path: PathBuf::from("models/test/test-model.yaml"),
            status: "active".to_string(),
            replacement: None,
            release_date: None,
            deprecation_date: None,
            sunset_date: None,
            parameters_supported: supported.into_iter().map(str::to_string).collect(),
            parameters_rejected: rejected.into_iter().map(str::to_string).collect(),
            parameters_deprecated: deprecated.into_iter().map(str::to_string).collect(),
        }
    }

    #[test]
    fn optional_string_ignores_null_and_empty_values() {
        let data = json!({
            "missing_date": null,
            "empty_replacement": "",
            "replacement": "gpt-4o"
        });

        assert_eq!(optional_string(&data, "missing_date"), None);
        assert_eq!(optional_string(&data, "empty_replacement"), None);
        assert_eq!(
            optional_string(&data, "replacement"),
            Some("gpt-4o".to_string())
        );
    }

    #[test]
    fn string_array_reads_nested_string_arrays() {
        let data = json!({
            "parameters": {
                "supported": ["temperature", "top_p"]
            }
        });

        assert_eq!(
            string_array(&data, &["parameters", "supported"]),
            vec!["temperature".to_string(), "top_p".to_string()]
        );
        assert!(string_array(&data, &["parameters", "rejected"]).is_empty());
    }

    #[test]
    fn date_order_validation_reports_later_left_date() {
        let mut errors = Vec::new();

        validate_date_order(
            &mut errors,
            "models/test/test-model.yaml",
            "release_date",
            &Some("2025-01-02".to_string()),
            "deprecation_date",
            &Some("2025-01-01".to_string()),
        );

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains(
            "release_date '2025-01-02' must be on or before deprecation_date '2025-01-01'"
        ));
    }

    #[test]
    fn parameter_overlap_validation_reports_conflicting_arrays() {
        let mut errors = Vec::new();
        let meta = meta_with_parameters(
            vec!["temperature", "top_p"],
            vec!["temperature"],
            vec!["top_p"],
        );

        validate_no_parameter_overlap(&mut errors, "models/test/test-model.yaml", &meta);

        assert_eq!(errors.len(), 2);
        assert!(errors
            .iter()
            .any(|e| e.contains("temperature") && e.contains("supported and rejected")));
        assert!(errors
            .iter()
            .any(|e| e.contains("top_p") && e.contains("supported and deprecated")));
    }

    #[test]
    fn freshness_stats_count_stale_and_missing_pricing_models() {
        let models = vec![
            json!({
                "id": "fresh-priced",
                "last_verified": "2026-05-01",
                "pricing": {"currency": "USD", "input": 1.0, "output": 2.0}
            }),
            json!({
                "id": "stale-unpriced",
                "last_verified": "2025-12-31"
            }),
            json!({
                "id": "missing-date",
                "pricing": {"currency": "USD", "input": 1.0, "output": 2.0}
            }),
        ];

        let stats = freshness_stats(&models, "2026-05-03", 90);

        assert_eq!(stats.total_models, 3);
        assert_eq!(stats.stale_models, 1);
        assert_eq!(stats.missing_pricing, 1);
        assert_eq!(stats.missing_last_verified, 1);
    }
}
