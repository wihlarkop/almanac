use crate::catalog::{Model, Provider};
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

const ENV_INCLUDE_PROVIDERS: &str = "CATALOG_INCLUDE_PROVIDERS";
const ENV_EXCLUDE_PROVIDERS: &str = "CATALOG_EXCLUDE_PROVIDERS";
const ENV_INCLUDE_MODELS: &str = "CATALOG_INCLUDE_MODELS";
const ENV_EXCLUDE_MODELS: &str = "CATALOG_EXCLUDE_MODELS";
const ENV_SCOPE_FILE: &str = "CATALOG_SCOPE_FILE";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogScope {
    pub include_providers: HashSet<String>,
    pub exclude_providers: HashSet<String>,
    pub include_models: HashSet<ModelRef>,
    pub exclude_models: HashSet<ModelRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModelRef {
    pub provider: String,
    pub id: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeFile {
    #[serde(default)]
    include: ScopeGroup,
    #[serde(default)]
    exclude: ScopeGroup,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScopeGroup {
    #[serde(default)]
    providers: Vec<String>,
    #[serde(default)]
    models: Vec<String>,
}

impl CatalogScope {
    pub fn disabled() -> Self {
        Self::default()
    }

    pub fn from_env() -> Result<Self> {
        Self::from_values(
            std::env::var(ENV_INCLUDE_PROVIDERS).ok(),
            std::env::var(ENV_EXCLUDE_PROVIDERS).ok(),
            std::env::var(ENV_INCLUDE_MODELS).ok(),
            std::env::var(ENV_EXCLUDE_MODELS).ok(),
            std::env::var(ENV_SCOPE_FILE).ok(),
        )
    }

    pub(crate) fn from_values(
        include_providers: Option<String>,
        exclude_providers: Option<String>,
        include_models: Option<String>,
        exclude_models: Option<String>,
        scope_file: Option<String>,
    ) -> Result<Self> {
        let has_env_scope = include_providers.as_deref().is_some_and(has_value)
            || exclude_providers.as_deref().is_some_and(has_value)
            || include_models.as_deref().is_some_and(has_value)
            || exclude_models.as_deref().is_some_and(has_value);

        let scope_file = scope_file.filter(|value| has_value(value));
        if has_env_scope && scope_file.is_some() {
            bail!(
                "catalog scope is configured with both CATALOG_SCOPE_FILE and catalog scope env vars; remove either CATALOG_SCOPE_FILE or catalog scope env vars"
            );
        }

        if let Some(path) = scope_file {
            return Self::from_file(Path::new(path.trim()));
        }

        Ok(Self {
            include_providers: parse_provider_list(include_providers.as_deref()),
            exclude_providers: parse_provider_list(exclude_providers.as_deref()),
            include_models: parse_model_list(include_models.as_deref())?,
            exclude_models: parse_model_list(exclude_models.as_deref())?,
        })
    }

    pub fn from_file(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading catalog scope file {}", path.display()))?;
        let file: ScopeFile = serde_yaml::from_str(&raw)
            .with_context(|| format!("parsing catalog scope file {}", path.display()))?;

        Ok(Self {
            include_providers: normalize_provider_vec(file.include.providers),
            exclude_providers: normalize_provider_vec(file.exclude.providers),
            include_models: parse_model_vec(file.include.models)?,
            exclude_models: parse_model_vec(file.exclude.models)?,
        })
    }

    pub fn is_disabled(&self) -> bool {
        self.include_providers.is_empty()
            && self.exclude_providers.is_empty()
            && self.include_models.is_empty()
            && self.exclude_models.is_empty()
    }

    pub fn apply(
        &self,
        providers: Vec<Provider>,
        models: Vec<Model>,
        aliases: HashMap<String, String>,
    ) -> Result<ScopedCatalog> {
        if self.is_disabled() {
            return Ok(ScopedCatalog {
                providers,
                models,
                aliases,
            });
        }

        self.validate_known_entries(&providers, &models)?;

        let visible_models: Vec<Model> = models
            .into_iter()
            .filter(|model| self.includes_model(model))
            .filter(|model| !self.excludes_model(model))
            .collect();

        if visible_models.is_empty() {
            bail!("catalog scope produced zero visible models");
        }

        let visible_model_ids: HashSet<String> =
            visible_models.iter().map(|model| model.id.clone()).collect();
        let visible_provider_ids: HashSet<String> = visible_models
            .iter()
            .map(|model| model.provider.clone())
            .collect();

        let visible_providers = providers
            .into_iter()
            .filter(|provider| visible_provider_ids.contains(&provider.id))
            .collect();

        let visible_aliases = aliases
            .into_iter()
            .filter(|(_, target)| visible_model_ids.contains(target))
            .collect();

        Ok(ScopedCatalog {
            providers: visible_providers,
            models: visible_models,
            aliases: visible_aliases,
        })
    }

    fn includes_model(&self, model: &Model) -> bool {
        if self.include_providers.is_empty() && self.include_models.is_empty() {
            return true;
        }

        self.include_providers.contains(&model.provider)
            || self.include_models.contains(&ModelRef {
                provider: model.provider.clone(),
                id: model.id.clone(),
            })
    }

    fn excludes_model(&self, model: &Model) -> bool {
        self.exclude_providers.contains(&model.provider)
            || self.exclude_models.contains(&ModelRef {
                provider: model.provider.clone(),
                id: model.id.clone(),
            })
    }

    fn validate_known_entries(&self, providers: &[Provider], models: &[Model]) -> Result<()> {
        let provider_ids: HashSet<&str> =
            providers.iter().map(|provider| provider.id.as_str()).collect();
        let model_refs: HashSet<ModelRef> = models
            .iter()
            .map(|model| ModelRef {
                provider: model.provider.clone(),
                id: model.id.clone(),
            })
            .collect();

        for provider in self
            .include_providers
            .iter()
            .chain(self.exclude_providers.iter())
        {
            if !provider_ids.contains(provider.as_str()) {
                bail!("catalog scope references unknown provider '{provider}'");
            }
        }

        for model_ref in self.include_models.iter().chain(self.exclude_models.iter()) {
            if !model_refs.contains(model_ref) {
                bail!(
                    "catalog scope references unknown model '{}/{}'",
                    model_ref.provider,
                    model_ref.id
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct ScopedCatalog {
    pub providers: Vec<Provider>,
    pub models: Vec<Model>,
    pub aliases: HashMap<String, String>,
}

fn has_value(value: &str) -> bool {
    !value.trim().is_empty()
}

fn parse_provider_list(value: Option<&str>) -> HashSet<String> {
    value
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_provider_vec(values: Vec<String>) -> HashSet<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn parse_model_list(value: Option<&str>) -> Result<HashSet<ModelRef>> {
    value
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_model_ref)
        .collect()
}

fn parse_model_vec(values: Vec<String>) -> Result<HashSet<ModelRef>> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| parse_model_ref(&value))
        .collect()
}

pub(crate) fn parse_model_ref(value: &str) -> Result<ModelRef> {
    let parts: Vec<_> = value.split('/').collect();
    if parts.len() != 2 || parts[0].trim().is_empty() || parts[1].trim().is_empty() {
        bail!("catalog scope model reference '{value}' must use provider/model-id");
    }

    Ok(ModelRef {
        provider: parts[0].trim().to_string(),
        id: parts[1].trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(id: &str) -> Provider {
        Provider {
            id: id.to_string(),
            display_name: id.to_string(),
            website: format!("https://{id}.example.com"),
            api_docs: Some(format!("https://{id}.example.com/docs")),
        }
    }

    fn model(provider: &str, id: &str) -> Model {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "provider": provider,
            "display_name": id,
            "status": "active",
            "context_window": 128000,
            "max_output_tokens": 4096,
            "modalities": {"input": ["text"], "output": ["text"]},
            "capabilities": {},
            "parameters": {
                "supported": [],
                "rejected": [],
                "deprecated_for_this_model": []
            },
            "last_verified": "2026-05-10",
            "confidence": "official",
            "endpoint_family": "chat_completions",
            "sources": []
        }))
        .unwrap()
    }

    #[test]
    fn model_ref_requires_provider_and_model_id() {
        assert_eq!(
            parse_model_ref("openai/gpt-4o").unwrap(),
            ModelRef {
                provider: "openai".to_string(),
                id: "gpt-4o".to_string(),
            }
        );

        assert!(parse_model_ref("gpt-4o").is_err());
        assert!(parse_model_ref("openai/").is_err());
        assert!(parse_model_ref("/gpt-4o").is_err());
        assert!(parse_model_ref("openai/gpt-4o/extra").is_err());
    }

    #[test]
    fn env_scope_rejects_yaml_mode_conflict() {
        let err = CatalogScope::from_values(
            Some("openai".to_string()),
            None,
            None,
            None,
            Some("scope.yaml".to_string()),
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("remove either CATALOG_SCOPE_FILE or catalog scope env vars")
        );
    }

    #[test]
    fn env_scope_parses_trimmed_deduplicated_lists() {
        let scope = CatalogScope::from_values(
            Some(" openai,anthropic,openai ".to_string()),
            Some(" xai ".to_string()),
            Some("openai/gpt-4o, anthropic/claude-sonnet-4-6".to_string()),
            Some("openai/gpt-4o-mini".to_string()),
            None,
        )
        .unwrap();

        assert_eq!(scope.include_providers.len(), 2);
        assert!(scope.include_providers.contains("openai"));
        assert!(scope.include_providers.contains("anthropic"));
        assert!(scope.exclude_providers.contains("xai"));
        assert!(scope.include_models.contains(&ModelRef {
            provider: "openai".to_string(),
            id: "gpt-4o".to_string(),
        }));
        assert!(scope.exclude_models.contains(&ModelRef {
            provider: "openai".to_string(),
            id: "gpt-4o-mini".to_string(),
        }));
    }

    #[test]
    fn yaml_scope_parses_file() {
        let path = std::env::temp_dir().join(format!(
            "almanac-scope-{}.yaml",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            r#"
include:
  providers:
    - openai
  models:
    - google/gemini-2.5-pro
exclude:
  providers:
    - xai
  models:
    - openai/gpt-4o-mini
"#,
        )
        .unwrap();

        let scope = CatalogScope::from_file(&path).unwrap();

        assert!(scope.include_providers.contains("openai"));
        assert!(scope.exclude_providers.contains("xai"));
        assert!(scope.include_models.contains(&ModelRef {
            provider: "google".to_string(),
            id: "gemini-2.5-pro".to_string(),
        }));
        assert!(scope.exclude_models.contains(&ModelRef {
            provider: "openai".to_string(),
            id: "gpt-4o-mini".to_string(),
        }));

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn apply_scope_includes_providers_and_removes_unusable_aliases() {
        let scope = CatalogScope {
            include_providers: HashSet::from(["openai".to_string()]),
            ..CatalogScope::default()
        };
        let providers = vec![provider("openai"), provider("anthropic")];
        let models = vec![
            model("openai", "gpt-4o"),
            model("anthropic", "claude-sonnet"),
        ];
        let aliases = HashMap::from([
            ("gpt-latest".to_string(), "gpt-4o".to_string()),
            ("claude-latest".to_string(), "claude-sonnet".to_string()),
        ]);

        let scoped = scope.apply(providers, models, aliases).unwrap();

        assert_eq!(scoped.providers.len(), 1);
        assert_eq!(scoped.providers[0].id, "openai");
        assert_eq!(scoped.models.len(), 1);
        assert_eq!(scoped.models[0].id, "gpt-4o");
        assert!(scoped.aliases.contains_key("gpt-latest"));
        assert!(!scoped.aliases.contains_key("claude-latest"));
    }

    #[test]
    fn apply_scope_exclude_wins_over_include() {
        let scope = CatalogScope {
            include_providers: HashSet::from(["openai".to_string()]),
            exclude_models: HashSet::from([ModelRef {
                provider: "openai".to_string(),
                id: "gpt-4o".to_string(),
            }]),
            ..CatalogScope::default()
        };

        let err = scope
            .apply(
                vec![provider("openai")],
                vec![model("openai", "gpt-4o")],
                HashMap::new(),
            )
            .unwrap_err();

        assert!(err.to_string().contains("zero visible models"));
    }

    #[test]
    fn apply_scope_rejects_unknown_entries() {
        let scope = CatalogScope {
            include_providers: HashSet::from(["missing".to_string()]),
            ..CatalogScope::default()
        };

        let err = scope
            .apply(
                vec![provider("openai")],
                vec![model("openai", "gpt-4o")],
                HashMap::new(),
            )
            .unwrap_err();

        assert!(err.to_string().contains("unknown provider 'missing'"));
    }
}
