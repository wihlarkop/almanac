use almanac_scraper::catalog::load_catalog;
use almanac_scraper::diff::{DiffResult, diff};
use almanac_scraper::engine::run_spider;
use almanac_scraper::model::ScrapedModel;
use almanac_scraper::spiders::{
    alibaba::AlibabaSpider, anthropic::AnthropicSpider, bytedance::ByteDanceSpider,
    cohere::CohereSpider, deepseek::DeepSeekSpider, doc_page::DocPageSpider,
    elevenlabs::ElevenLabsSpider, google::GoogleSpider, inception::InceptionSpider,
    leonardo::LeonardoSpider, luma::LumaSpider, meta::MetaSpider, microsoft::MicrosoftSpider,
    mistral::MistralSpider, mistral_html::MistralHtmlSpider, moonshot::MoonshotSpider,
    openai::OpenAiSpider, perplexity::PerplexitySpider, voyageai::VoyageAiSpider, xai::XaiSpider,
    xiaomi::XiaomiSpider, zai::ZaiSpider,
};
use almanac_scraper::writer::write_model;
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use time::OffsetDateTime;

/// Providers with custom HTML spiders (need filtering or multi-page crawl).
/// All others are handled automatically by SIMPLE_PROVIDERS below.
const CUSTOM_PROVIDERS: &[&str] = &[
    "anthropic",
    "google",
    "mistral",
    "openai",
    "cohere",
    "deepseek",
    "xai",
    "alibaba",
    "moonshot",
    "meta",
    "perplexity",
    "elevenlabs",
    "luma",
    "leonardo",
    "voyageai",
    "zai",
    "microsoft",
    "inception",
    "xiaomi",
    "bytedance",
];

/// Simple providers: scraped with a single public docs URL, no custom logic.
/// Model IDs are extracted from <code> elements via the generic heuristic.
const SIMPLE_PROVIDERS: &[(&str, &str)] = &[
    // Adobe Firefly — static HTML docs
    (
        "adobe",
        "https://developer.adobe.com/firefly-api/docs/guides/models/",
    ),
    // AI21 Labs — Mintlify static docs
    ("ai21", "https://docs.ai21.com/docs/overview"),
    // Amazon Bedrock — AWS static docs page with full model table
    (
        "amazon",
        "https://docs.aws.amazon.com/bedrock/latest/userguide/models-supported.html",
    ),
    // AssemblyAI — static docs
    ("assemblyai", "https://www.assemblyai.com/docs/models"),
    // Baidu ERNIE — Qianfan platform docs
    (
        "baidu",
        "https://qianfan.cloud.baidu.com/doc/WENXINWORKSHOP/s/Nlks5zkzu",
    ),
    // Black Forest Labs — FLUX model docs
    ("bfl", "https://docs.bfl.ml/"),
    // Cartesia — static Mintlify docs
    (
        "cartesia",
        "https://docs.cartesia.ai/build-with-cartesia/tts-models/api-changes",
    ),
    // Deepgram — static docs with model table
    (
        "deepgram",
        "https://developers.deepgram.com/docs/models-languages-overview",
    ),
    ("heygen", "https://docs.heygen.com/reference/list-voices-v2"),
    // HiDream — homepage (docs not yet public)
    ("hidream", "https://www.hidream.ai/"),
    // IBM Granite — Watson X docs with model list
    (
        "ibm",
        "https://www.ibm.com/docs/en/watsonx/saas?topic=solutions-supported-foundation-models",
    ),
    // Ideogram — developer API reference
    (
        "ideogram",
        "https://developer.ideogram.ai/api-reference/api-reference/generate",
    ),
    // Inflection — developer portal (Mintlify)
    (
        "inflection",
        "https://developers.inflection.ai/docs/introduction",
    ),
    // Inworld — static docs
    ("inworld", "https://docs.inworld.ai/docs/tutorial-text/v2/"),
    // Jina AI — models landing page
    ("jina", "https://jina.ai/models/"),
    // Kling — model docs (Kuaishou)
    ("kling", "https://klingai.com/"),
    // Lightricks LTX — static docs
    ("lightricks", "https://docs.ltx.video/"),
    // LMNT — static developer docs
    ("lmnt", "https://docs.lmnt.com/"),
    // Meshy — 3D model static docs
    ("meshy", "https://docs.meshy.ai/"),
    // MiniMax — platform docs
    (
        "minimax",
        "https://platform.minimaxi.com/document/model-introduction",
    ),
    // Naver HyperCLOVA — CLOVA Studio docs
    ("naver", "https://clovastudio.stream.naver.com/docs"),
    // Nomic — static docs (embed models)
    (
        "nomic",
        "https://docs.nomic.ai/reference/endpoints/nomic-embed-text",
    ),
    // NVIDIA NIM — supported models static page
    (
        "nvidia",
        "https://docs.nvidia.com/nim/large-language-models/latest/supported-models.html",
    ),
    // Pika Labs — model info page
    ("pika", "https://pika.art/"),
    // PixVerse — API docs
    ("pixverse", "https://docs.pixverse.ai/"),
    // PlayHT — static API docs (Mintlify)
    ("playht", "https://docs.play.ai/documentation/rest-api"),
    // Recraft — static developer docs
    ("recraft", "https://www.recraft.ai/docs"),
    // Reka — static docs
    ("reka", "https://docs.reka.ai/"),
    // Reve AI — docs
    ("reve", "https://reveai.com/"),
    // Runway — static developer docs
    ("runway", "https://docs.runwayml.com/"),
    // Stability AI — static API reference
    (
        "stabilityai",
        "https://platform.stability.ai/docs/api-reference",
    ),
    // StepFun — platform docs
    (
        "stepfun",
        "https://platform.stepfun.com/docs/overview/concept",
    ),
    // Suno — AI music (no public API docs yet)
    ("suno", "https://suno.com/"),
    // Tencent Hunyuan — cloud docs
    (
        "tencent",
        "https://cloud.tencent.com/document/product/1729/104753",
    ),
    // Tripo 3D — static docs
    ("tripo", "https://platform.tripo3d.ai/docs"),
    // Udio — AI music (no public API docs yet)
    ("udio", "https://www.udio.com/"),
    // Upstage Solar — Mintlify static docs
    (
        "upstage",
        "https://developers.upstage.ai/docs/apis/model-overview",
    ),
    // Vidu — video gen docs
    ("vidu", "https://platform.vidu.studio/docs"),
    // Writer Palmyra — static dev docs
    ("writer", "https://dev.writer.com/api-guides/models"),
    // Yi / 01.AI — developer platform docs
    ("yi", "https://platform.lingyiwanwu.com/docs"),
];

#[derive(Parser, Debug)]
#[command(
    name = "scraper",
    about = "Scrape AI provider docs pages and diff against the model catalog"
)]
struct Args {
    /// Provider name to scrape, or "all" to run every provider sequentially.
    /// Examples: anthropic, openai, google, mistral, all
    #[arg(short, long, default_value = "all")]
    provider: String,

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

/// Runs spiders sequentially — one provider at a time, no parallelism.
/// Each provider is isolated: failures are logged but never block the rest.
async fn run_all_spiders(provider: &str) -> Result<Vec<ScrapedModel>> {
    let mut all = Vec::new();
    let run_all = provider == "all";

    // ── Custom spiders ────────────────────────────────────────────────────────
    macro_rules! run_custom {
        ($name:expr, $spider:expr) => {
            if run_all || provider == $name {
                println!("  Scraping {}...", $name);
                match run_spider($spider).await {
                    Ok(items) => all.extend(items),
                    Err(e) => eprintln!("  [warn] {} failed: {e}", $name),
                }
            }
        };
    }

    run_custom!("anthropic", AnthropicSpider);
    run_custom!("google", GoogleSpider);
    run_custom!("mistral", MistralSpider);
    run_custom!("mistral", MistralHtmlSpider); // multi-page model-cards variant
    run_custom!("openai", OpenAiSpider);
    run_custom!("cohere", CohereSpider);
    run_custom!("deepseek", DeepSeekSpider);
    run_custom!("xai", XaiSpider);
    run_custom!("alibaba", AlibabaSpider);
    run_custom!("moonshot", MoonshotSpider);
    run_custom!("meta", MetaSpider);
    run_custom!("perplexity", PerplexitySpider);
    run_custom!("elevenlabs", ElevenLabsSpider);
    run_custom!("luma", LumaSpider);
    run_custom!("leonardo", LeonardoSpider);
    run_custom!("voyageai", VoyageAiSpider);
    run_custom!("zai", ZaiSpider);
    run_custom!("microsoft", MicrosoftSpider);
    run_custom!("inception", InceptionSpider);
    run_custom!("xiaomi", XiaomiSpider);
    run_custom!("bytedance", ByteDanceSpider);

    // ── Simple providers (DocPageSpider) ──────────────────────────────────────
    for &(name, url) in SIMPLE_PROVIDERS {
        if run_all || provider == name {
            println!("  Scraping {name}...");
            match run_spider(DocPageSpider {
                provider: name,
                start_url: url,
            })
            .await
            {
                Ok(items) => all.extend(items),
                Err(e) => eprintln!("  [warn] {name} failed: {e}"),
            }
        }
    }

    if !run_all && all.is_empty() {
        let known: Vec<&str> = CUSTOM_PROVIDERS
            .iter()
            .copied()
            .chain(SIMPLE_PROVIDERS.iter().map(|(name, _)| *name))
            .collect();
        if !known.contains(&provider) {
            eprintln!(
                "Unknown provider '{provider}'. Use 'all' or one of: {}",
                known.join(", ")
            );
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
