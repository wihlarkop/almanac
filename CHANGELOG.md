# Changelog

All notable changes to this project are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

---

## [Unreleased]

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
