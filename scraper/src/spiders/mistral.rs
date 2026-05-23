use crate::model::ScrapedModel;
use async_trait::async_trait;
use kumo::prelude::*;

const START_URL: &str = "https://docs.mistral.ai/getting-started/models/models_overview/";
const BASE: &str = "https://docs.mistral.ai";

pub struct MistralSpider;

#[async_trait]
impl Spider for MistralSpider {
    type Item = ScrapedModel;

    fn name(&self) -> &str {
        "mistral"
    }

    fn start_urls(&self) -> Vec<String> {
        vec![START_URL.into()]
    }

    fn allowed_domains(&self) -> Vec<&str> {
        vec!["docs.mistral.ai"]
    }

    async fn parse(&self, res: &Response) -> Result<Output<Self::Item>, KumoError> {
        let mut output = Output::new();

        if res.url().contains("/model-cards/") {
            let id = res
                .url()
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_string();

            if id.is_empty() {
                return Ok(output);
            }

            let display_name = res.css("h1").first().map(|el| el.text().trim().to_string());
            // Mistral docs are Next.js SSR — context/pricing extraction not yet supported
            let context_window = None;
            let (input_price, output_price) = (None, None);

            output = output.items(vec![ScrapedModel {
                id,
                provider: "mistral".into(),
                display_name,
                context_window,
                max_output_tokens: None,
                input_price,
                output_price,
                source_url: res.url().to_string(),
            }]);
        } else {
            let links: Vec<String> = res
                .css(r#"a[href*="/model-cards/"]"#)
                .iter()
                .filter_map(|el| el.attr("href"))
                .filter(|href| !href.contains('#'))
                .map(|href| {
                    if href.starts_with("http") {
                        href
                    } else {
                        format!("{}{}", BASE, href)
                    }
                })
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();

            for url in links {
                output = output.follow(url);
            }
        }

        Ok(output)
    }
}
