# almanac-scraper

Scrapes AI provider documentation pages and diffs against the local model catalog. Reports new models and pricing changes.

## Quick Start

```bash
# 1. From the repo root, do a dry run across all providers
cargo run -p almanac-scraper -- --root .

# 2. You'll see output like:
#   Loaded 495 models from catalog.
#   Scraped 23 models from provider pages.
#
#   --- Diff Results ---
#
#   [NEW] google/gemini-3-pro-preview
#     display: Gemini 3 Pro Preview
#     context: Some(1048576) | in: None | out: None
#     source: https://ai.google.dev/gemini-api/docs/models/gemini-3-pro-preview

# 3. If you want to generate YAML stubs for new models:
cargo run -p almanac-scraper -- --root . --write

# 4. The stub files are created at models/<provider>/<id>.yaml
#    Open each one, verify the data, and fill in missing fields manually.

# 5. Run the validator to check your YAML is valid before committing:
cargo run -p almanac-validator -- models/
```

## Usage

```bash
# Dry run — report only, no files written
cargo run -p almanac-scraper -- --root . --provider google
cargo run -p almanac-scraper -- --root . --provider mistral
cargo run -p almanac-scraper -- --root .                    # all providers

# Write YAML stubs for new models (sets confidence: scraped)
cargo run -p almanac-scraper -- --root . --provider google --write
```

Flags:
- `--provider` — `google` | `anthropic` | `mistral` | `all` (default: `all`)
- `--root` — path to the almanac repo root (default: `.`)
- `--write` — auto-generate YAML stubs for new models; never overwrites existing files

## Provider Support

| Provider | New Models | Context Window | Pricing | Notes |
|---|---|---|---|---|
| **Google** | ✅ | ✅ | ❌ | Scrapes `ai.google.dev/gemini-api/docs/models` |
| **Anthropic** | ⚠️ | ⚠️ | ⚠️ | Page is JS-rendered; use browser mode (not yet enabled) |
| **Mistral** | ✅ | ❌ | ❌ | Next.js SSR — IDs and display names only |

## Interpreting and Filling Generated Stubs

When you run with `--write`, the scraper creates a stub YAML at `models/<provider>/<id>.yaml`. It looks like this:

```yaml
id: gemini-3-pro-preview
provider: google
display_name: Gemini 3 Pro Preview
status: active
release_date: null          # <-- fill in: YYYY-MM-DD from provider announcement
deprecation_date: null
sunset_date: null
replacement: null
context_window: 1048576     # <-- auto-filled when available, verify against docs
max_output_tokens: 65536    # <-- auto-filled when available, verify against docs
modalities:
  input: [text]             # <-- update: add image, audio, video if supported
  output: [text]            # <-- update: add image, audio if supported
capabilities:
  tools: false              # <-- check provider docs: does it support function calling?
  vision: false             # <-- check provider docs: does it accept image input?
  streaming: true
  json_mode: false          # <-- check provider docs: structured output / JSON mode?
  prompt_caching: false     # <-- check provider docs: is context caching supported?
  thinking: false           # <-- check provider docs: does it have a reasoning/thinking mode?
parameters:
  supported: []             # <-- list params like: [temperature, top_p, max_output_tokens]
  rejected: []
  deprecated_for_this_model: []
pricing:
  currency: USD
  input: 0.0                # <-- fill in: price per 1M input tokens
  output: 0.0               # <-- fill in: price per 1M output tokens
last_verified: 2026-05-23
confidence: scraped         # <-- change to "official" once you verify against provider docs
endpoint_family: unknown    # <-- update: e.g. chat, embedding, image_generation, etc.
sources:
  - url: https://...        # <-- the page the scraper found it on; add official pricing page
    last_verified: 2026-05-23
```

### Checklist for each stub

1. **`release_date`** — find the announcement post or changelog. Format: `YYYY-MM-DD`.
2. **`context_window` / `max_output_tokens`** — verify against the provider's model card or API docs.
3. **`modalities`** — check what input types the model accepts (text, image, audio, video) and what it outputs.
4. **`capabilities`** — go through each flag against the provider docs:
   - `tools` → function/tool calling supported?
   - `vision` → image input supported?
   - `json_mode` → structured JSON output mode?
   - `prompt_caching` → provider caches context (e.g. Anthropic cache, Google context caching)?
   - `thinking` → reasoning/thinking mode (e.g. Gemini thinking, Claude extended thinking)?
5. **`pricing`** — find the official pricing page. Values are per **1M tokens**.
6. **`confidence`** — change from `scraped` → `official` once you've verified against official docs.
7. **`endpoint_family`** — pick the closest match: `chat`, `embedding`, `image_generation`, `music_generation`, `tts`, `transcription`, `moderation`, `unknown`.
8. **`sources`** — add the official model card or pricing URL alongside the scraped URL.

### Typical sources to check

| Provider | Model docs | Pricing |
|---|---|---|
| Google | `ai.google.dev/gemini-api/docs/models/<id>` | `ai.google.dev/gemini-api/pricing` |
| Anthropic | `docs.anthropic.com/en/docs/about-claude/models/all-models` | `anthropic.com/pricing` |
| Mistral | `docs.mistral.ai/models/model-cards/<id>` | `mistral.ai/technology/#pricing` |

## Adding More Providers

Each provider needs one spider file in `src/spiders/<provider>.rs`:

1. Implement `Spider<Item = ScrapedModel>`
2. Add `pub mod <provider>;` to `src/spiders/mod.rs`
3. Add a `Provider::<Name>` variant in `src/main.rs`
4. Wire into `run_spiders()`

For JS-rendered pages (OpenAI, xAI, Anthropic), enable kumo's `browser` feature in `Cargo.toml` and use `BrowserFetcher` instead of the default HTTP fetcher.

## Output Reference

- `[NEW]` — model ID not found in catalog → generate stub with `--write`, then fill manually
- `[PRICE CHANGE]` — model already in catalog but pricing differs → update the YAML manually

New stubs written with `--write` get `confidence: scraped` and must be verified before committing.
