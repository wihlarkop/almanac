use crate::model::ScrapedModel;
use anyhow::{Context, Result};
use serde_json::json;
use std::path::Path;

pub fn generate_yaml(model: &ScrapedModel, _today: &str) -> String {
    let context_window = model.context_window.filter(|value| *value > 0);
    let max_output_tokens = model.max_output_tokens.filter(|value| *value > 0);
    // A scrape date is a retrieval date, not an independent verification date.
    // Keep observations that exist, while marking every omitted fact unknown.
    let document = json!({
        "id": model.id,
        "provider": model.provider,
        "display_name": model.display_name.as_deref().unwrap_or(&model.id),
        "status": "unknown",
        "release_date": null,
        "deprecation_date": null,
        "sunset_date": null,
        "replacement": null,
        "context_window": context_window,
        "context_window_state": if context_window.is_some() { "known" } else { "unknown" },
        "max_output_tokens": max_output_tokens,
        "max_output_tokens_state": if max_output_tokens.is_some() { "known" } else { "unknown" },
        "modalities": null,
        "modalities_state": "unknown",
        "capabilities": {},
        "capabilities_state": "unknown",
        "parameters": { "supported": [], "rejected": [], "deprecated_for_this_model": [] },
        "parameters_state": "unknown",
        "pricing": null,
        // ScrapedModel has no typed unit/currency/provenance, so even observed
        // numeric candidates are not safe to publish as pricing yet.
        "pricing_state": "unknown",
        "last_verified": null,
        "confidence": "unknown",
        "endpoint_family": "unknown",
        "sources": [{ "url": model.source_url }]
    });

    serde_yaml::to_string(&document).expect("serializing a JSON value to YAML cannot fail")
}

pub fn write_model(
    model: &ScrapedModel,
    models_root: &Path,
    today: &str,
) -> Result<std::path::PathBuf> {
    validate_path_component(&model.provider).context("invalid provider path component")?;
    validate_path_component(&model.id).context("invalid model id path component")?;
    let dir = models_root.join(&model.provider);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.yaml", model.id));

    if path.exists() {
        anyhow::bail!("file already exists, skipping: {}", path.display());
    }

    std::fs::write(&path, generate_yaml(model, today))?;
    Ok(path)
}

fn validate_path_component(value: &str) -> Result<()> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        || !value.as_bytes()[0].is_ascii_lowercase() && !value.as_bytes()[0].is_ascii_digit()
    {
        anyhow::bail!("must be a lowercase catalog identifier")
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ScrapedModel;

    fn candidate() -> ScrapedModel {
        ScrapedModel {
            id: "sample-model".to_string(),
            provider: "sample-provider".to_string(),
            display_name: Some("Sample Model".to_string()),
            source_url: "https://example.com/models?a=one&b=two".to_string(),
            ..ScrapedModel::default()
        }
    }

    #[test]
    fn generated_yaml_preserves_unknown_facts_as_unknown() {
        let yaml = generate_yaml(&candidate(), "2026-11-18");
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(value["context_window"], serde_yaml::Value::Null);
        assert_eq!(value["context_window_state"], "unknown");
        assert_eq!(value["max_output_tokens"], serde_yaml::Value::Null);
        assert_eq!(value["status"], "unknown");
        assert_eq!(value["modalities"], serde_yaml::Value::Null);
        assert_eq!(value["capabilities_state"], "unknown");
        assert_eq!(value["pricing"], serde_yaml::Value::Null);
        assert_eq!(value["pricing_state"], "unknown");
        assert_eq!(value["last_verified"], serde_yaml::Value::Null);
        assert_eq!(value["confidence"], "unknown");
        assert_eq!(value["endpoint_family"], "unknown");
        assert_eq!(
            value["sources"][0]["url"],
            "https://example.com/models?a=one&b=two"
        );
        assert!(value["sources"][0].get("last_verified").is_none());
    }

    #[test]
    fn writer_rejects_path_traversal_identifiers() {
        let mut model = candidate();
        model.id = "../outside".to_string();
        let dir = tempfile::tempdir().unwrap();

        assert!(write_model(&model, dir.path(), "2026-11-18").is_err());
        assert!(!dir.path().join("outside.yaml").exists());
    }

    #[test]
    fn generated_yaml_keeps_valid_scraped_limits_and_rejects_zero_as_unknown() {
        let mut model = candidate();
        model.context_window = Some(0);
        model.max_output_tokens = Some(1024);
        let value: serde_yaml::Value =
            serde_yaml::from_str(&generate_yaml(&model, "2026-11-18")).unwrap();

        assert_eq!(value["context_window"], serde_yaml::Value::Null);
        assert_eq!(value["context_window_state"], "unknown");
        assert_eq!(value["max_output_tokens"], 1024);
        assert_eq!(value["max_output_tokens_state"], "known");
    }
}
