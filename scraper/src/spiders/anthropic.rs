use crate::model::ScrapedModel;
use async_trait::async_trait;
use kumo::prelude::*;

const START_URL: &str = "https://platform.claude.com/docs/en/docs/about-claude/models/all-models";

pub struct AnthropicSpider;

#[async_trait]
impl Spider for AnthropicSpider {
    type Item = ScrapedModel;

    fn name(&self) -> &str {
        "anthropic"
    }

    fn start_urls(&self) -> Vec<String> {
        vec![START_URL.into()]
    }

    fn allowed_domains(&self) -> Vec<&str> {
        vec!["platform.claude.com", "docs.anthropic.com"]
    }

    async fn parse(&self, res: &Response) -> Result<Output<Self::Item>, KumoError> {
        let mut models: Vec<ScrapedModel> = Vec::new();

        for table in res.css("table").iter() {
            let headers: Vec<String> = table
                .css("thead th")
                .iter()
                .map(|th| th.text().trim().to_string())
                .collect();

            if headers.len() < 2 || headers[0].to_lowercase() != "feature" {
                continue;
            }

            let n_models = headers.len() - 1;
            let mut api_ids: Vec<Option<String>> = vec![None; n_models];
            let mut input_prices: Vec<Option<f64>> = vec![None; n_models];
            let mut output_prices: Vec<Option<f64>> = vec![None; n_models];
            let mut contexts: Vec<Option<u64>> = vec![None; n_models];
            let mut max_outputs: Vec<Option<u64>> = vec![None; n_models];

            for row in table.css("tbody tr").iter() {
                let cells: Vec<String> = row
                    .css("td")
                    .iter()
                    .map(|td| td.text().trim().to_string())
                    .collect();

                if cells.is_empty() {
                    continue;
                }

                let label = cells[0].to_lowercase();

                for (i, cell) in cells.iter().skip(1).enumerate() {
                    if i >= n_models {
                        break;
                    }
                    if label.contains("claude api id") || label.contains("api id") {
                        api_ids[i] = Some(cell.trim().to_string());
                    } else if label.contains("pricing") {
                        let (inp, out) = parse_pricing(cell);
                        input_prices[i] = inp;
                        output_prices[i] = out;
                    } else if label.contains("context window") {
                        contexts[i] = parse_token_size(cell);
                    } else if label.contains("max output") {
                        max_outputs[i] = parse_token_size(cell);
                    }
                }
            }

            for i in 0..n_models {
                if let Some(id) = &api_ids[i] {
                    models.push(ScrapedModel {
                        id: id.clone(),
                        provider: "anthropic".into(),
                        display_name: Some(headers[i + 1].clone()),
                        context_window: contexts[i],
                        max_output_tokens: max_outputs[i],
                        input_price: input_prices[i],
                        output_price: output_prices[i],
                        source_url: START_URL.into(),
                    });
                }
            }
        }

        Ok(Output::new().items(models))
    }
}

fn parse_pricing(text: &str) -> (Option<f64>, Option<f64>) {
    let mut input = None;
    let mut output = None;
    for line in text.lines() {
        let lower = line.to_lowercase();
        if let Some(price) = extract_dollar_amount(line) {
            if lower.contains("input") {
                input = Some(price);
            } else if lower.contains("output") {
                output = Some(price);
            } else if input.is_none() {
                input = Some(price);
            } else {
                output = Some(price);
            }
        }
    }
    (input, output)
}

fn extract_dollar_amount(text: &str) -> Option<f64> {
    let s = text.trim().trim_start_matches('$');
    s.split_whitespace()
        .next()
        .and_then(|n| n.replace(',', "").parse().ok())
}

fn parse_token_size(text: &str) -> Option<u64> {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find('m') {
        let before = lower[..pos].trim();
        if let Some(n) = before
            .split_whitespace()
            .last()
            .and_then(|s| s.replace(',', "").parse::<f64>().ok())
        {
            return Some((n * 1_000_000.0) as u64);
        }
    }
    if let Some(pos) = lower.find('k') {
        let before = lower[..pos].trim();
        if let Some(n) = before
            .split_whitespace()
            .last()
            .and_then(|s| s.replace(',', "").parse::<f64>().ok())
        {
            return Some((n * 1_000.0) as u64);
        }
    }
    None
}
