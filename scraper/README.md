# almanac-scraper

Scrapes AI provider documentation pages and diffs against the local model catalog. Reports new models and pricing changes.

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

### Adding More Providers

Each provider needs one spider file in `src/spiders/<provider>.rs`:

1. Implement `Spider<Item = ScrapedModel>`
2. Add `pub mod <provider>;` to `src/spiders/mod.rs`
3. Add a `Provider::<Name>` variant in `src/main.rs`
4. Wire into `run_spiders()`

For JS-rendered pages (OpenAI, xAI, Anthropic), enable kumo's `browser` feature in `Cargo.toml` and use `BrowserFetcher` instead of the default HTTP fetcher.

## Output

- `[NEW]` — model ID not in catalog → add YAML stub with `--write`
- `[PRICE CHANGE]` — model already in catalog but pricing differs → update manually

New stubs written with `--write` get `confidence: scraped` and need manual verification before committing.
