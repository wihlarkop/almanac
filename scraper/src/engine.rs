use crate::model::ScrapedModel;
use crate::spider::{HtmlResponse, Spider};
use anyhow::Result;
use std::collections::{HashSet, VecDeque};

/// Fetches all start_urls, then any follow_urls returned by the spider,
/// until the queue is empty or all URLs have been visited.
pub async fn run_spider<S: Spider>(spider: S) -> Result<Vec<ScrapedModel>> {
    let client = reqwest::Client::builder()
        .user_agent("almanac-scraper/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut queue: VecDeque<String> = spider.start_urls().into_iter().collect();
    let mut visited: HashSet<String> = HashSet::new();
    let mut results: Vec<ScrapedModel> = Vec::new();

    while let Some(url) = queue.pop_front() {
        if visited.contains(&url) {
            continue;
        }
        visited.insert(url.clone());

        let mut req = client.get(&url);
        for (name, value) in spider.headers() {
            req = req.header(name, value);
        }

        let body = match req.send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    tracing::warn!("failed to read body for {url}: {e}");
                    continue;
                }
            },
            Err(e) => {
                tracing::warn!("failed to fetch {url}: {e}");
                continue;
            }
        };

        let response = HtmlResponse {
            url: &url,
            body: &body,
        };

        match spider.scrape(&response).await {
            Ok(output) => {
                results.extend(output.items);
                for follow_url in output.follow_urls {
                    if !visited.contains(&follow_url) {
                        queue.push_back(follow_url);
                    }
                }
            }
            Err(e) => tracing::warn!("spider {} failed on {url}: {e}", spider.name()),
        }
    }

    Ok(results)
}
