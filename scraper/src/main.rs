pub mod catalog;
pub mod diff;
pub mod model;
pub mod spiders;
pub mod writer;

use anyhow::Result;
use catalog::load_catalog;
use clap::{Parser, ValueEnum};
use diff::{diff, DiffResult};
use kumo::prelude::*;
use model::ScrapedModel;
use spiders::{anthropic::AnthropicSpider, google::GoogleSpider, mistral::MistralSpider};
use std::path::PathBuf;
use time::OffsetDateTime;
use writer::write_model;

#[derive(Debug, Clone, ValueEnum)]
enum Provider {
    Google,
    Anthropic,
    Mistral,
    All,
}

#[derive(Parser, Debug)]
#[command(name = "scraper", about = "Scrape AI provider pages and diff against the model catalog")]
struct Args {
    #[arg(short, long, default_value = "all")]
    provider: Provider,

    #[arg(short, long, default_value = ".")]
    root: PathBuf,

    #[arg(short, long)]
    write: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("kumo=warn").init();

    let args = Args::parse();
    let models_dir = args.root.join("models");

    let catalog = load_catalog(&models_dir)?;
    println!("Loaded {} models from catalog.", catalog.len());

    let scraped = run_spiders(&args.provider).await?;
    println!("Scraped {} models from provider pages.", scraped.len());

    let results = diff(&scraped, &catalog);

    if results.is_empty() {
        println!("\nAll scraped models are already in the catalog.");
        return Ok(());
    }

    println!("\n--- Diff Results ---");
    for result in &results {
        match result {
            DiffResult::New(m) => {
                println!(
                    "\n[NEW] {}/{}\n  display: {}\n  context: {:?} | in: {:?} | out: {:?}\n  source: {}",
                    m.provider,
                    m.id,
                    m.display_name.as_deref().unwrap_or("unknown"),
                    m.context_window,
                    m.input_price,
                    m.output_price,
                    m.source_url,
                );
                if args.write {
                    let today = today_str();
                    match write_model(m, &models_dir, &today) {
                        Ok(path) => println!("  -> wrote {}", path.display()),
                        Err(e) => println!("  -> skipped: {e}"),
                    }
                }
            }
            DiffResult::PriceChanged { model: m, old_input, old_output } => {
                println!(
                    "\n[PRICE CHANGE] {}/{}\n  input:  {:?} -> {:?}\n  output: {:?} -> {:?}\n  source: {}",
                    m.provider, m.id, old_input, m.input_price, old_output, m.output_price, m.source_url,
                );
            }
        }
    }

    Ok(())
}

async fn run_spiders(provider: &Provider) -> Result<Vec<ScrapedModel>> {
    let mut all = Vec::new();
    if matches!(provider, Provider::Google | Provider::All) {
        all.extend(run_spider(GoogleSpider).await?);
    }
    if matches!(provider, Provider::Anthropic | Provider::All) {
        all.extend(run_spider(AnthropicSpider).await?);
    }
    if matches!(provider, Provider::Mistral | Provider::All) {
        all.extend(run_spider(MistralSpider).await?);
    }
    Ok(all)
}

async fn run_spider<S>(spider: S) -> Result<Vec<ScrapedModel>>
where
    S: Spider + 'static,
{
    let mut items = Vec::new();
    let mut stream = CrawlEngine::builder()
        .concurrency(3)
        .middleware(DefaultHeaders::new().user_agent("almanac-scraper/0.1"))
        .stream(spider)
        .await?;

    while let Some(value) = stream.next().await {
        match serde_json::from_value::<ScrapedModel>(value) {
            Ok(model) => items.push(model),
            Err(e) => tracing::warn!("failed to deserialize scraped item: {e}"),
        }
    }

    Ok(items)
}

fn today_str() -> String {
    let now = OffsetDateTime::now_utc();
    format!("{:04}-{:02}-{:02}", now.year(), now.month() as u8, now.day())
}
