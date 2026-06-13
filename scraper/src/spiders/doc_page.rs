use crate::model::ScrapedModel;
use crate::spider::{HtmlResponse, Spider, SpiderOutput};
use anyhow::Result;
use scraper::{Html, Selector};

/// Scrapes a single public docs page and extracts model IDs.
///
/// Extraction strategy (applied in order, results merged):
/// 1. `<code>` and `<pre>` elements — works for static HTML docs
/// 2. `__NEXT_DATA__` JSON — works for Next.js SSR pages
/// 3. Inline `<script>` content — catches model IDs embedded in JS bundles
pub struct DocPageSpider {
    pub provider: &'static str,
    pub start_url: &'static str,
}

/// Like DocPageSpider but follows links matching a path fragment to
/// per-model detail pages (e.g. "/model-cards/", "/models/").
pub struct MultiPageDocSpider {
    pub provider: &'static str,
    pub start_url: &'static str,
    pub base_url: &'static str,
    /// Substring that must appear in an href for the engine to follow it.
    pub follow_href_contains: &'static str,
}

// ── DocPageSpider ─────────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl Spider for DocPageSpider {
    fn name(&self) -> &str {
        self.provider
    }

    fn start_urls(&self) -> Vec<String> {
        vec![self.start_url.into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        Ok(SpiderOutput::new().items(extract_model_ids(res.body, self.provider, res.url)))
    }
}

// ── MultiPageDocSpider ────────────────────────────────────────────────────────

#[async_trait::async_trait]
impl Spider for MultiPageDocSpider {
    fn name(&self) -> &str {
        self.provider
    }

    fn start_urls(&self) -> Vec<String> {
        vec![self.start_url.into()]
    }

    async fn scrape(&self, res: &HtmlResponse<'_>) -> Result<SpiderOutput> {
        let is_detail = res.url != self.start_url;
        if is_detail {
            Ok(SpiderOutput::new().items(extract_model_ids(res.body, self.provider, res.url)))
        } else {
            Ok(SpiderOutput {
                items: vec![],
                follow_urls: extract_follow_links(
                    res.body,
                    self.base_url,
                    self.follow_href_contains,
                ),
            })
        }
    }
}

// ── Public helpers (used by custom spiders too) ───────────────────────────────

/// Full extraction pipeline: code/pre elements + __NEXT_DATA__ + script scanning.
pub fn extract_model_ids(html: &str, provider: &str, source_url: &str) -> Vec<ScrapedModel> {
    let mut seen = std::collections::HashSet::new();
    let mut models = Vec::new();

    let mut push = |id: String| {
        if seen.insert(id.clone()) {
            models.push(ScrapedModel {
                id,
                provider: provider.into(),
                display_name: None,
                context_window: None,
                max_output_tokens: None,
                input_price: None,
                output_price: None,
                source_url: source_url.into(),
            });
        }
    };

    // Strategy 1: <code> and <pre> elements (static HTML, Mintlify, GitBook, etc.)
    let doc = Html::parse_document(html);
    let code_sel = Selector::parse("code, pre").unwrap();
    for el in doc.select(&code_sel) {
        let text: String = el.text().collect::<String>();
        for candidate in text.split_whitespace() {
            let c = candidate.trim_matches(|c: char| !c.is_alphanumeric());
            if looks_like_model_id(c) {
                push(c.to_string());
            }
        }
    }

    // Strategy 2: __NEXT_DATA__ JSON blob (Next.js SSR pages)
    let script_sel = Selector::parse("script").unwrap();
    for el in doc.select(&script_sel) {
        let id_attr = el.value().attr("id").unwrap_or("");
        let type_attr = el.value().attr("type").unwrap_or("");
        let src_attr = el.value().attr("src");

        let text: String = el.text().collect::<String>();

        // Strategy 3 (inline script scanning) is intentionally disabled —
        // it produces too many false positives from UI framework code.
        let _ = src_attr;

        let is_json_blob =
            (id_attr == "__NEXT_DATA__" || type_attr == "application/json") && !text.is_empty();
        if is_json_blob {
            let parsed = serde_json::from_str::<serde_json::Value>(&text);
            if let Ok(json) = parsed {
                extract_from_json(&json, &mut |s| push(s.to_string()));
            }
        }
    }

    models
}

/// Follows links whose href contains the given fragment.
pub fn extract_follow_links(html: &str, base_url: &str, href_contains: &str) -> Vec<String> {
    let doc = Html::parse_document(html);
    let a_sel = Selector::parse("a[href]").unwrap();
    let mut seen = std::collections::HashSet::new();
    let mut links = Vec::new();

    for el in doc.select(&a_sel) {
        if let Some(href) = el.value().attr("href") {
            if !href.contains(href_contains) || href.contains('#') {
                continue;
            }
            let url = if href.starts_with("http") {
                href.to_string()
            } else if href.starts_with('/') {
                format!("{}{}", base_url.trim_end_matches('/'), href)
            } else {
                continue;
            };
            if seen.insert(url.clone()) {
                links.push(url);
            }
        }
    }

    links
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Recursively walk a JSON value and call `push` for any string that looks
/// like a model ID. Stops recursing into arrays > 500 elements to avoid
/// blowing up on large data payloads.
fn extract_from_json(value: &serde_json::Value, push: &mut impl FnMut(&str)) {
    match value {
        serde_json::Value::String(s) if looks_like_model_id(s.trim()) => {
            push(s.trim());
        }
        serde_json::Value::String(_) => {}
        serde_json::Value::Array(arr) => {
            for item in arr.iter().take(500) {
                extract_from_json(item, push);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                // For keys that typically hold a model identifier, check the value directly
                let is_model_key = matches!(
                    key.as_str(),
                    "id" | "model"
                        | "model_id"
                        | "modelId"
                        | "name"
                        | "slug"
                        | "api_name"
                        | "apiName"
                        | "identifier"
                );
                let s_opt = if is_model_key { val.as_str() } else { None };
                if let Some(t) = s_opt {
                    let t = t.trim();
                    if looks_like_model_id(t) {
                        push(t);
                    }
                }
                extract_from_json(val, push);
            }
        }
        _ => {}
    }
}

/// Strings that appear on ReadMe.io-hosted doc pages and look like model IDs
/// but are actually platform infrastructure identifiers.
const BLOCKLIST: &[&str] = &[
    "get-started",
    "execute-request",
    "list-endpoints",
    "list-specs",
    "get-endpoint",
    "search-endpoints",
    "get-server-variables",
    "readme.io",
    "readmessl.com",
    "dash.readme.com",
    "ssl.readmessl.com",
    "readme_search_v2",
    "landing_page",
    "top-left",
    "image-generation",
    "client.chat.completions.create",
    "reasoning_effort",
    "request_id",
    "response_id",
    "error_code",
    "status_code",
    "created_at",
    "updated_at",
    // Provider brand names / URL slugs that aren't model IDs
    "voyage-ai",
    "model-introduction",
    "cloud-based",
    "dream-machine",
    // API response field names scraped from code examples
    "image_url",
    "top_logprobs",
    "finish_reason",
    "tool_calls",
];

/// Returns true if `s` looks like a model API identifier.
///
/// Constraints (tuned to minimise false positives from docs page noise):
/// - 5–80 characters, no spaces/slashes/newlines/colons
/// - Contains at least one hyphen, dot, or underscore
/// - Must contain at least one ASCII letter (rejects pure version numbers like `0.00206815`)
/// - All characters ASCII lowercase, digits, or `-` `.` `_`
/// - Must not start with `data-` (HTML data attribute prefix)
/// - Not in the platform identifier blocklist
pub fn looks_like_model_id(s: &str) -> bool {
    if s.len() < 5 || s.len() > 80 {
        return false;
    }
    if s.contains(' ') || s.contains('/') || s.contains('\n') || s.contains('\\') || s.contains(':')
    {
        return false;
    }
    if !s.contains('-') && !s.contains('.') && !s.contains('_') {
        return false;
    }
    // Must start with a lowercase letter (not a digit) — rejects `0.00206815`, `5.760.1`
    if !s
        .chars()
        .next()
        .map(|c| c.is_ascii_lowercase())
        .unwrap_or(false)
    {
        return false;
    }
    if s.starts_with("data-") {
        return false;
    }
    // Reject file extensions — catches bundle artifacts, audio placeholders, config files
    const FILE_EXTS: &[&str] = &[
        ".js", ".ts", ".jsx", ".tsx", ".css", ".vue", ".py", ".go",
        ".json", ".yaml", ".yml", ".wav", ".mp3", ".mp4", ".csv",
    ];
    if FILE_EXTS.iter().any(|ext| s.ends_with(ext)) {
        return false;
    }
    // Reject Python/JS method-call paths: more than 2 dots means chained accessors, not a model ID
    if s.chars().filter(|&c| c == '.').count() > 2 {
        return false;
    }
    // Reject semver-like version strings: v1.2.3, v1.16.0, etc.
    if s.starts_with('v')
        && s[1..]
            .split('.')
            .all(|p| p.chars().all(|c| c.is_ascii_digit()))
    {
        return false;
    }
    // Must contain at least one letter (extra guard against pure numeric strings)
    if !s.chars().any(|c| c.is_ascii_lowercase()) {
        return false;
    }
    if BLOCKLIST.contains(&s) {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.' | '_'))
}
