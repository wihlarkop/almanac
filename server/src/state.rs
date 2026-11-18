use crate::{
    catalog::{CatalogStats, Model, ModelPriceStat, Provider},
    scope::CatalogScope,
};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use time::OffsetDateTime;
use walkdir::WalkDir;

pub struct AppState {
    pub providers: Vec<Provider>,
    pub models: Vec<Model>,
    pub aliases: HashMap<String, String>,
    pub providers_by_id: HashMap<String, usize>,
    pub models_by_id: HashMap<String, usize>,
    pub models_by_provider_id: HashMap<(String, String), usize>,
    pub etag: String,
    pub loaded_at: OffsetDateTime,
    pub stats: CatalogStats,
}

pub fn load_state(data_dir: &Path) -> Result<AppState> {
    load_state_with_scope(data_dir, &CatalogScope::disabled())
}

pub fn load_state_with_scope(data_dir: &Path, scope: &CatalogScope) -> Result<AppState> {
    let providers_dir = data_dir.join("providers");
    let models_dir = data_dir.join("models");
    let aliases_path = data_dir.join("aliases.yaml");

    ensure_dir(&providers_dir)?;
    ensure_dir(&models_dir)?;
    ensure_file(&aliases_path)?;

    let providers: Vec<Provider> = load_yaml_dir(&providers_dir)?;
    let mut models: Vec<Model> = load_yaml_recursive(&models_dir)?;
    for model in &mut models {
        fill_legacy_metadata_states(model);
    }
    let aliases = load_aliases(&aliases_path)?;
    let scoped = scope.apply(providers, models, aliases)?;

    build_state(scoped.providers, scoped.models, scoped.aliases)
}

fn fill_legacy_metadata_states(model: &mut Model) {
    use crate::catalog::{MetadataState, PricingState};

    if model.context_window_state.is_none() {
        model.context_window_state = Some(if model.context_window.is_some() {
            MetadataState::Known
        } else {
            MetadataState::Unknown
        });
    }
    if model.max_output_tokens_state.is_none() {
        model.max_output_tokens_state = Some(if model.max_output_tokens.is_some() {
            MetadataState::Known
        } else {
            MetadataState::Unknown
        });
    }
    if model.modalities_state.is_none() {
        model.modalities_state = Some(if model.modalities.is_some() {
            MetadataState::Known
        } else {
            MetadataState::Unknown
        });
    }
    if model.capabilities_state.is_none() {
        model.capabilities_state = Some(if model.capabilities.is_empty() {
            MetadataState::Unknown
        } else {
            MetadataState::Known
        });
    }
    if model.parameters_state.is_none() {
        let has_parameter_facts = !model.parameters.supported.is_empty()
            || !model.parameters.rejected.is_empty()
            || !model.parameters.deprecated_for_this_model.is_empty();
        model.parameters_state = Some(if has_parameter_facts {
            MetadataState::Known
        } else {
            MetadataState::Unknown
        });
    }
    if model.pricing_state.is_none() {
        model.pricing_state = Some(match &model.pricing {
            None => PricingState::Unknown,
            Some(pricing)
                if pricing.is_free()
                    && model.modalities.as_ref().is_some_and(|modalities| {
                        modalities.input.as_slice() == ["text"]
                            && modalities.output.as_slice() == ["text"]
                    }) =>
            {
                PricingState::Free
            }
            Some(pricing) if pricing.is_free() => PricingState::Unknown,
            Some(_) => PricingState::Published,
        });
    }
}

fn build_state(
    providers: Vec<Provider>,
    models: Vec<Model>,
    aliases: HashMap<String, String>,
) -> Result<AppState> {
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_string(&providers)?.as_bytes());
    hasher.update(serde_json::to_string(&models)?.as_bytes());
    let etag = format!("\"{}\"", hex::encode(hasher.finalize()));
    let providers_by_id = providers
        .iter()
        .enumerate()
        .map(|(index, provider)| (provider.id.clone(), index))
        .collect();
    let models_by_id = models
        .iter()
        .enumerate()
        .map(|(index, model)| (model.id.clone(), index))
        .collect();
    let models_by_provider_id = models
        .iter()
        .enumerate()
        .map(|(index, model)| ((model.provider.clone(), model.id.clone()), index))
        .collect();
    let loaded_at = OffsetDateTime::now_utc();
    let stats = compute_stats(&models, &providers, loaded_at);

    Ok(AppState {
        providers,
        models,
        aliases,
        providers_by_id,
        models_by_id,
        models_by_provider_id,
        etag,
        loaded_at,
        stats,
    })
}

fn compute_stats(
    models: &[Model],
    providers: &[Provider],
    loaded_at: OffsetDateTime,
) -> CatalogStats {
    let mut models_by_status: HashMap<String, usize> = HashMap::new();
    let mut models_by_provider: HashMap<String, usize> = HashMap::new();
    let mut models_by_endpoint_family: HashMap<String, usize> = HashMap::new();
    let mut models_by_input_modality: HashMap<String, usize> = HashMap::new();
    let mut models_by_output_modality: HashMap<String, usize> = HashMap::new();
    let mut free_models = 0usize;
    let mut models_without_pricing = 0usize;
    let mut cheapest_input: Option<ModelPriceStat> = None;
    let mut most_expensive_input: Option<ModelPriceStat> = None;
    let mut cheapest_output: Option<ModelPriceStat> = None;
    let mut most_expensive_output: Option<ModelPriceStat> = None;
    let mut comparable_price_currencies = HashSet::new();

    for model in models {
        *models_by_status
            .entry(model.status.as_str().to_string())
            .or_insert(0) += 1;
        *models_by_provider
            .entry(model.provider.clone())
            .or_insert(0) += 1;
        *models_by_endpoint_family
            .entry(model.endpoint_family.as_str().to_string())
            .or_insert(0) += 1;
        if let Some(modalities) = &model.modalities {
            for m in &modalities.input {
                *models_by_input_modality.entry(m.clone()).or_insert(0) += 1;
            }
            for m in &modalities.output {
                *models_by_output_modality.entry(m.clone()).or_insert(0) += 1;
            }
        }

        if model.is_known_free() {
            free_models += 1;
        } else if model.pricing_is_missing() {
            models_without_pricing += 1;
        } else if let Some(p) = model
            .pricing
            .as_ref()
            .filter(|_| model.has_comparable_token_pricing())
        {
            comparable_price_currencies.insert(p.currency.clone());
            let stat_in = ModelPriceStat {
                model_id: model.id.clone(),
                provider: model.provider.clone(),
                price: p.input,
            };
            cheapest_input = Some(match cheapest_input.take() {
                None => stat_in.clone(),
                Some(prev) => {
                    if p.input < prev.price {
                        stat_in.clone()
                    } else {
                        prev
                    }
                }
            });
            most_expensive_input = Some(match most_expensive_input.take() {
                None => stat_in,
                Some(prev) => {
                    if p.input > prev.price {
                        ModelPriceStat {
                            model_id: model.id.clone(),
                            provider: model.provider.clone(),
                            price: p.input,
                        }
                    } else {
                        prev
                    }
                }
            });
            let stat_out = ModelPriceStat {
                model_id: model.id.clone(),
                provider: model.provider.clone(),
                price: p.output,
            };
            cheapest_output = Some(match cheapest_output.take() {
                None => stat_out.clone(),
                Some(prev) => {
                    if p.output < prev.price {
                        stat_out.clone()
                    } else {
                        prev
                    }
                }
            });
            most_expensive_output = Some(match most_expensive_output.take() {
                None => stat_out,
                Some(prev) => {
                    if p.output > prev.price {
                        ModelPriceStat {
                            model_id: model.id.clone(),
                            provider: model.provider.clone(),
                            price: p.output,
                        }
                    } else {
                        prev
                    }
                }
            });
        }
    }

    let last_updated = loaded_at
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    CatalogStats {
        total_models: models.len(),
        total_providers: providers.len(),
        models_by_status,
        models_by_provider,
        models_by_endpoint_family,
        models_by_input_modality,
        models_by_output_modality,
        free_models,
        models_without_pricing,
        cheapest_input: (comparable_price_currencies.len() == 1)
            .then_some(cheapest_input)
            .flatten(),
        most_expensive_input: (comparable_price_currencies.len() == 1)
            .then_some(most_expensive_input)
            .flatten(),
        cheapest_output: (comparable_price_currencies.len() == 1)
            .then_some(cheapest_output)
            .flatten(),
        most_expensive_output: (comparable_price_currencies.len() == 1)
            .then_some(most_expensive_output)
            .flatten(),
        last_updated,
    }
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
    use crate::catalog::{
        Confidence, EndpointFamily, Modalities, ModelParameters, ModelStatus, Pricing,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_provider(id: &str) -> Provider {
        Provider {
            id: id.to_string(),
            display_name: id.to_string(),
            website: "https://example.com".to_string(),
            api_docs: None,
        }
    }

    fn make_model(
        id: &str,
        provider: &str,
        status: ModelStatus,
        pricing: Option<Pricing>,
    ) -> Model {
        Model {
            id: id.to_string(),
            provider: provider.to_string(),
            display_name: id.to_string(),
            status,
            release_date: None,
            deprecation_date: None,
            sunset_date: None,
            replacement: None,
            context_window: Some(4096),
            context_window_state: Some(crate::catalog::MetadataState::Known),
            max_output_tokens: Some(1024),
            max_output_tokens_state: Some(crate::catalog::MetadataState::Known),
            modalities: Some(Modalities {
                input: vec!["text".to_string()],
                output: vec!["text".to_string()],
            }),
            modalities_state: Some(crate::catalog::MetadataState::Known),
            capabilities: std::collections::HashMap::new(),
            capabilities_state: Some(crate::catalog::MetadataState::Unknown),
            parameters: ModelParameters {
                supported: vec![],
                rejected: vec![],
                deprecated_for_this_model: vec![],
            },
            parameters_state: Some(crate::catalog::MetadataState::Unknown),
            pricing,
            pricing_state: None,
            last_verified: Some("2024-01-01".to_string()),
            confidence: Confidence::Official,
            endpoint_family: EndpointFamily::ChatCompletions,
            sources: vec![],
        }
    }

    #[test]
    fn compute_stats_counts_correctly() {
        let providers = vec![make_provider("p1"), make_provider("p2")];
        let models = vec![
            make_model(
                "m1",
                "p1",
                ModelStatus::Active,
                Some(Pricing {
                    currency: "USD".to_string(),
                    input: 1.0,
                    output: 2.0,
                    cached_input: None,
                    batch_input: None,
                    batch_output: None,
                    request_fee: None,
                    search_fee: None,
                    reasoning: None,
                    per_image: None,
                    per_second: None,
                    per_minute: None,
                    per_million_chars: None,
                    per_page: None,
                    input_audio: None,
                    input_image: None,
                    input_video: None,
                    output_audio: None,
                    tiers: None,
                    pricing_notes: None,
                }),
            ),
            make_model(
                "m2",
                "p2",
                ModelStatus::Active,
                Some(Pricing {
                    currency: "USD".to_string(),
                    input: 5.0,
                    output: 10.0,
                    cached_input: None,
                    batch_input: None,
                    batch_output: None,
                    request_fee: None,
                    search_fee: None,
                    reasoning: None,
                    per_image: None,
                    per_second: None,
                    per_minute: None,
                    per_million_chars: None,
                    per_page: None,
                    input_audio: None,
                    input_image: None,
                    input_video: None,
                    output_audio: None,
                    tiers: None,
                    pricing_notes: None,
                }),
            ),
            make_model("m3", "p1", ModelStatus::Deprecated, None),
        ];

        let stats = compute_stats(&models, &providers, OffsetDateTime::now_utc());

        assert_eq!(stats.total_models, 3);
        assert_eq!(stats.total_providers, 2);
        assert_eq!(stats.models_without_pricing, 1);
        assert_eq!(stats.free_models, 0);
        assert_eq!(*stats.models_by_status.get("active").unwrap(), 2);
        assert_eq!(*stats.models_by_status.get("deprecated").unwrap(), 1);
        assert_eq!(*stats.models_by_provider.get("p1").unwrap(), 2);
        assert_eq!(*stats.models_by_provider.get("p2").unwrap(), 1);
        let cheapest = stats.cheapest_input.unwrap();
        assert_eq!(cheapest.model_id, "m1");
        assert_eq!(cheapest.price, 1.0);
        let priciest = stats.most_expensive_input.unwrap();
        assert_eq!(priciest.model_id, "m2");
        assert_eq!(priciest.price, 5.0);
        assert!(!stats.last_updated.is_empty());
    }

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

    #[test]
    fn missing_legacy_states_are_exposed_as_unknown_or_known_from_values() {
        use crate::catalog::{MetadataState, PricingState};

        let mut model = make_model("m1", "p1", ModelStatus::Active, None);
        model.context_window_state = None;
        model.max_output_tokens = None;
        model.max_output_tokens_state = None;
        model.modalities_state = None;
        model.capabilities.clear();
        model.capabilities_state = None;
        model.parameters = ModelParameters {
            supported: vec![],
            rejected: vec![],
            deprecated_for_this_model: vec![],
        };
        model.parameters_state = None;
        model.pricing_state = None;

        fill_legacy_metadata_states(&mut model);

        assert_eq!(model.context_window_state, Some(MetadataState::Known));
        assert_eq!(model.max_output_tokens_state, Some(MetadataState::Unknown));
        assert_eq!(model.modalities_state, Some(MetadataState::Known));
        assert_eq!(model.capabilities_state, Some(MetadataState::Unknown));
        assert_eq!(model.parameters_state, Some(MetadataState::Unknown));
        assert_eq!(model.pricing_state, Some(PricingState::Unknown));
    }
}
