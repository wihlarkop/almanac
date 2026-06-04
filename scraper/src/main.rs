use almanac_scraper::catalog::load_catalog;
use almanac_scraper::diff::{DiffResult, diff};
use almanac_scraper::engine::run_spider;
use almanac_scraper::model::ScrapedModel;
use almanac_scraper::spiders::{
    anthropic::AnthropicSpider, cohere::CohereSpider, deepseek::DeepSeekSpider,
    google::GoogleSpider, mistral::MistralSpider, openai::OpenAiSpider,
};
use almanac_scraper::writer::write_model;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use time::OffsetDateTime;

#[derive(Debug, Clone, ValueEnum)]
enum Provider {
    Anthropic,
    Cohere,
    DeepSeek,
    Google,
    Mistral,
    OpenAi,
    All,
}

#[derive(Parser, Debug)]
#[command(
    name = "scraper",
    about = "Scrape AI provider pages and diff against the model catalog"
)]
struct Args {
    #[arg(short, long, default_value = "all")]
    provider: Provider,

    #[arg(short, long, default_value = ".")]
    root: PathBuf,

    /// Write new model YAMLs to models/<provider>/ (does not commit).
    #[arg(short, long)]
    write: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter("warn").init();

    let args = Args::parse();
    let models_dir = args.root.join("models");
    let today = today_str();

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
                    match write_model(m, &models_dir, &today) {
                        Ok(path) => println!("  -> wrote {}", path.display()),
                        Err(e) => println!("  -> skipped: {e}"),
                    }
                }
            }
            DiffResult::PriceChanged {
                model: m,
                old_input,
                old_output,
            } => {
                println!(
                    "\n[PRICE CHANGE] {}/{}\n  input:  {:?} -> {:?}\n  output: {:?} -> {:?}",
                    m.provider, m.id, old_input, m.input_price, old_output, m.output_price,
                );
            }
        }
    }

    Ok(())
}

async fn run_spiders(provider: &Provider) -> Result<Vec<ScrapedModel>> {
    let mut all = Vec::new();
    let run_all = matches!(provider, Provider::All);

    if run_all || matches!(provider, Provider::Anthropic) {
        all.extend(run_spider(AnthropicSpider).await?);
    }
    if run_all || matches!(provider, Provider::Google) {
        all.extend(run_spider(GoogleSpider).await?);
    }
    if run_all || matches!(provider, Provider::Mistral) {
        all.extend(run_spider(MistralSpider).await?);
    }
    if run_all || matches!(provider, Provider::OpenAi) {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            all.extend(run_spider(OpenAiSpider::new(key)).await?);
        } else {
            tracing::warn!("OPENAI_API_KEY not set — skipping OpenAI spider");
        }
    }
    if run_all || matches!(provider, Provider::Cohere) {
        if let Ok(key) = std::env::var("COHERE_API_KEY") {
            all.extend(run_spider(CohereSpider::new(key)).await?);
        } else {
            tracing::warn!("COHERE_API_KEY not set — skipping Cohere spider");
        }
    }
    if run_all || matches!(provider, Provider::DeepSeek) {
        if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            all.extend(run_spider(DeepSeekSpider::new(key)).await?);
        } else {
            tracing::warn!("DEEPSEEK_API_KEY not set — skipping DeepSeek spider");
        }
    }

    Ok(all)
}

fn today_str() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}-{:02}-{:02}",
        now.year(),
        now.month() as u8,
        now.day()
    )
}
