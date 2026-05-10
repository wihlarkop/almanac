# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

### Added

**Server**
- `GET /metrics` Prometheus endpoint (optional `--features metrics` cargo feature): `http_requests_total`, `http_request_duration_seconds`, `catalog_models_total`, `catalog_providers_total`, `catalog_aliases_total`. Path labels use matched route templates to avoid cardinality explosion.
- Deployment-time catalog scoping via env vars or `CATALOG_SCOPE_FILE`, with include/exclude rules for providers and models.

**Validator**
- Catalog summary report printed after every validation run: provider/model/alias totals, status breakdown, per-provider counts, and coverage stats (pricing, context window, staleness).
- `--check-urls` flag for HTTP HEAD reachability checks on all source URLs.

**Catalog — new providers (19 added, total 29)**
- **Groq** — llama-3.3-70b-versatile, llama-3.1-8b-instant, llama-3.1-70b-versatile, llama-3.2-11b/90b-vision-preview, gemma2-9b-it, mixtral-8x7b-32768, llama-guard-3-8b
- **Amazon** — Nova Micro, Nova Lite, Nova Pro, Nova Premier
- **Microsoft** — Phi-4, Phi-4 Mini, Phi-4 Multimodal, Phi-3.5 MoE, Phi-3.5 Mini, Phi-3.5 Vision
- **Cerebras** — llama3.1-8b, llama3.3-70b
- **Baidu** — ERNIE 4.5 21B/300B, ERNIE X1, ERNIE X1 Turbo
- **Tencent** — Hunyuan A13B Instruct, Hunyuan Turbo, Hunyuan Pro, Hunyuan Lite
- **ByteDance** — Doubao 1.5 Pro 32K/256K, Doubao 1.5 Thinking Pro, Seed 2.0 Pro
- **Reka** — Reka Core, Reka Flash, Reka Edge
- **AI21 Labs** — Jamba 1.5 Large, Jamba 1.5 Mini
- **NVIDIA** — Nemotron-4-340B, Llama 3.3 Nemotron Super 49B, Llama 3.3 Nemotron Nano 8B
- **Upstage** — Solar Pro, Solar Mini
- **Inflection AI** — Inflection 3 Pi
- **Xiaomi MiMo** — MiMo V2 Pro, MiMo V2 Flash
- **MiniMax** — MiniMax M2.7, M2.5, M2.1, Text-01
- **Moonshot AI** — Kimi K2.5, Kimi K2.6, Moonshot V1 8K/32K/128K (additions)
- **Z.AI (Zhipu)** — GLM-4.5, GLM-4.5 Flash, GLM-4.7 Flash (additions)
- **Inception AI** — Mercury, Mercury 2, Mercury Coder (diffusion LLMs)
- **Writer** — Palmyra X5 (1M context)
- **01.AI (Yi)** — Yi Lightning, Yi Large

**Catalog — model updates**
- xAI: Grok 4.3 added; `grok-code-fast-1` marked deprecated with replacement

**Aliases**
- 131 total aliases (up from ~80 at 0.1.0); new shorthand entries for all added providers

### Changed
- `ureq` upgraded from 2.x to 3.3.0; updated `AgentBuilder` → `Agent::config_builder`, `Error::Status` → `Error::StatusCode`
- Workspace `Cargo.toml` dependencies reorganized into named groups (error handling, async runtime, web framework, serialization, etc.)
- Removed redundant `tower` entry from server `[dev-dependencies]`

---

## [0.1.0] - 2026-05-09

### Added

**API server**
- `GET /api/v1/health` — server health and version
- `GET /api/v1/providers` — list all providers with ETag caching
- `GET /api/v1/providers/{id}` — single provider detail
- `GET /api/v1/models` — paginated model list with filtering (`provider`, `status`, `capability`, `modality_input`, `modality_output`, `min_context`, `max_input_price`), sorting (`provider`, `id`, `status`, `context_window`, `max_output_tokens`), and offset pagination
- `GET /api/v1/models/{provider}/{id}` — single model detail
- `GET /api/v1/aliases` — list all shorthand aliases
- `GET /api/v1/aliases/{alias}` — resolve a single alias to its canonical model id
- `POST /api/v1/validate` — validate a model string; returns canonical id, errors, and fuzzy suggestions
- `GET /api/v1/search` — full-text search across model ids, display names, and provider names
- `GET /api/v1/suggest` — ranked fuzzy suggestions for a partial model string
- `GET /api/v1/compare` — side-by-side comparison of two models
- `GET /api/v1/catalog/health` — catalog freshness and coverage stats (total models, missing pricing, stale records)
- `GET /api/v1/catalog/issues` — detailed catalog data quality issues

**Response contract**
- Uniform `ApiResponse<T>` envelope on every endpoint: `success`, `message`, `data`, `meta`, `error`
- `meta` includes `request_id`, `execution_time_seconds`, `timestamp`, and pagination fields
- Standard error codes: `MODEL_NOT_FOUND`, `PROVIDER_NOT_FOUND`, `ALIAS_NOT_FOUND`, `BAD_REQUEST`, `RATE_LIMIT_EXCEEDED`, `REQUEST_TIMEOUT`, `PAYLOAD_TOO_LARGE`, `INTERNAL_SERVER_ERROR`
- ETag + `Cache-Control: public, max-age=300` on catalog endpoints
- `X-Request-Id` propagation
- Security headers on all responses (`X-Content-Type-Options`, `X-Frame-Options`, `Referrer-Policy`)

**Infrastructure**
- `LOG_FORMAT=json` env var for structured JSON logging in production
- `RATE_LIMIT_RPS` env var for per-IP rate limiting (disabled by default)
- `PORT` and `DATA_DIR` env vars for server configuration
- Graceful shutdown on `SIGTERM` and `Ctrl+C`
- SIGHUP catalog hot-reload (Unix only)
- Docker image with non-root user and embedded catalog
- OpenAPI spec served at `/openapi.json`; interactive docs at `/swagger-ui` and `/scalar`

**Catalog**
- 201 model entries across 10 providers: Anthropic, OpenAI, Google, Meta, Mistral, Cohere, DeepSeek, xAI, Alibaba (Qwen), Perplexity
- Provider metadata in `providers/`
- Alias resolution in `aliases.yaml`
- Per-model fields: id, provider, display_name, status, lifecycle dates, context_window, max_output_tokens, modalities, capabilities, parameters, pricing, confidence, endpoint_family, sources, last_verified

**Validator** (`cargo run -p almanac-validator`)
- Schema validation for all provider and model YAML files
- Filename/id consistency checks
- Provider reference checks
- Alias chain and shadow detection
- Lifecycle date ordering and replacement checks
- Parameter overlap checks
- Freshness and pricing coverage report
- `--check-urls` flag for optional source URL reachability checks

---

## Versioning policy

This project uses [Semantic Versioning](https://semver.org/).

- **Patch** — bug fixes, data corrections, new model entries, dependency updates
- **Minor** — new endpoints, new response fields (additive), new query parameters
- **Major** — breaking changes to existing response shapes, removed endpoints, renamed fields
