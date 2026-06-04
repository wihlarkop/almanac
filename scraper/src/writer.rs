use crate::model::ScrapedModel;
use anyhow::Result;
use std::path::Path;

pub fn generate_yaml(model: &ScrapedModel, today: &str) -> String {
    let display = model
        .display_name
        .as_deref()
        .unwrap_or(&model.id)
        .to_string();
    // Use 4096 as a safe placeholder — schema requires minimum of 1.
    // Real values should be filled in manually after review.
    let context = model.context_window.unwrap_or(4096);
    let max_out = model.max_output_tokens.unwrap_or(4096);
    let input = model.input_price.unwrap_or(0.0);
    let output = model.output_price.unwrap_or(0.0);

    format!(
        r#"id: {id}
provider: {provider}
display_name: {display_name}
status: active
release_date: null
deprecation_date: null
sunset_date: null
replacement: null
context_window: {context_window}
max_output_tokens: {max_output_tokens}
modalities:
  input: [text]
  output: [text]
capabilities:
  tools: false
  vision: false
  streaming: true
  json_mode: false
  prompt_caching: false
  thinking: false
parameters:
  supported: []
  rejected: []
  deprecated_for_this_model: []
pricing:
  currency: USD
  input: {input}
  output: {output}
last_verified: {today}
confidence: inferred
endpoint_family: chat_completions
sources:
  - url: {source_url}
    last_verified: {today}
"#,
        id = model.id,
        provider = model.provider,
        display_name = display,
        context_window = context,
        max_output_tokens = max_out,
        input = input,
        output = output,
        today = today,
        source_url = model.source_url,
    )
}

pub fn write_model(
    model: &ScrapedModel,
    models_root: &Path,
    today: &str,
) -> Result<std::path::PathBuf> {
    let dir = models_root.join(&model.provider);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.yaml", model.id));

    if path.exists() {
        anyhow::bail!("file already exists, skipping: {}", path.display());
    }

    std::fs::write(&path, generate_yaml(model, today))?;
    Ok(path)
}
