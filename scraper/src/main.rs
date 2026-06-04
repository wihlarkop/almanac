use almanac_scraper::catalog::load_catalog;
use almanac_scraper::diff::{DiffResult, diff};
use almanac_scraper::engine::run_spider;
use almanac_scraper::model::ScrapedModel;
use almanac_scraper::spiders::{
    alibaba::AlibabaSpider, anthropic::AnthropicSpider, cohere::CohereSpider,
    deepseek::DeepSeekSpider, elevenlabs::ElevenLabsSpider, google::GoogleSpider, meta::MetaSpider,
    mistral::MistralSpider, mistral_html::MistralHtmlSpider, moonshot::MoonshotSpider,
    openai::OpenAiSpider, perplexity::PerplexitySpider, xai::XaiSpider,
};
use almanac_scraper::writer::write_model;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use time::OffsetDateTime;

#[derive(Debug, Clone, ValueEnum)]
enum Provider {
    All,
    Alibaba,
    Anthropic,
    Cohere,
    DeepSeek,
    ElevenLabs,
    Google,
    Meta,
    Mistral,
    Moonshot,
    OpenAi,
    Perplexity,
    Xai,
}

#[derive(Parser, Debug)]
#[command(
    name = "scraper",
    about = "Scrape AI provider docs pages and diff against the model catalog"
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

    let scraped = run_all_spiders(&args.provider).await?;
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

/// Runs all spiders sequentially — one provider at a time, no parallelism.
/// Each provider is independent so failures don't block the rest.
async fn run_all_spiders(provider: &Provider) -> Result<Vec<ScrapedModel>> {
    let mut all = Vec::new();
    let run_all = matches!(provider, Provider::All);

    if run_all || matches!(provider, Provider::Anthropic) {
        println!("  Scraping anthropic...");
        match run_spider(AnthropicSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] anthropic failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::Google) {
        println!("  Scraping google...");
        match run_spider(GoogleSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] google failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::Mistral) {
        println!("  Scraping mistral (HTML)...");
        match run_spider(MistralSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] mistral failed: {e}"),
        }
        match run_spider(MistralHtmlSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] mistral-html failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::OpenAi) {
        println!("  Scraping openai...");
        match run_spider(OpenAiSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] openai failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::Cohere) {
        println!("  Scraping cohere...");
        match run_spider(CohereSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] cohere failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::DeepSeek) {
        println!("  Scraping deepseek...");
        match run_spider(DeepSeekSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] deepseek failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::Xai) {
        println!("  Scraping xai...");
        match run_spider(XaiSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] xai failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::Alibaba) {
        println!("  Scraping alibaba...");
        match run_spider(AlibabaSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] alibaba failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::Moonshot) {
        println!("  Scraping moonshot...");
        match run_spider(MoonshotSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] moonshot failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::Meta) {
        println!("  Scraping meta...");
        match run_spider(MetaSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] meta failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::Perplexity) {
        println!("  Scraping perplexity...");
        match run_spider(PerplexitySpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] perplexity failed: {e}"),
        }
    }
    if run_all || matches!(provider, Provider::ElevenLabs) {
        println!("  Scraping elevenlabs...");
        match run_spider(ElevenLabsSpider).await {
            Ok(items) => all.extend(items),
            Err(e) => eprintln!("  [warn] elevenlabs failed: {e}"),
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
