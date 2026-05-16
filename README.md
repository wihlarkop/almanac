# Almanac

A free, open-source model catalog and REST API for LLM developers.

488 models · 68 providers · 274 aliases — all queryable, filterable, and comparable via a single HTTP API.

[![CI](https://github.com/wihlarkop/almanac/actions/workflows/validate.yml/badge.svg)](https://github.com/wihlarkop/almanac/actions/workflows/validate.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## What it answers

- What models exist across major LLM providers?
- Is this model string valid for this provider?
- Is this model active, deprecated, or retired?
- What capabilities, modalities, context limits, and pricing are known?
- What canonical model ID does this alias resolve to?
- Which model is cheapest for input/output tokens?

---

## Quick Start

**Run with Docker (pre-built image):**

```bash
docker run -p 8080:8080 ghcr.io/wihlarkop/almanac:latest
```

**Try it:**

```bash
# Health check
curl http://localhost:8080/api/v1/health

# List models
curl http://localhost:8080/api/v1/models?provider=openai&limit=5

# Validate a model parameter
curl -X POST http://localhost:8080/api/v1/validate \
  -H "content-type: application/json" \
  -d '{"model":"gpt-4o","provider":"openai","parameters":{"temperature":0.7}}'

# Search
curl "http://localhost:8080/api/v1/search?q=claude&limit=5"

# Compare models
curl "http://localhost:8080/api/v1/compare?models=openai/gpt-4o,anthropic/claude-opus-4-7"
```

**Interactive API docs:**
- Swagger UI: `http://localhost:8080/swagger-ui/`
- Scalar: `http://localhost:8080/scalar`
- OpenAPI JSON: `http://localhost:8080/openapi.json`

---

## API Reference

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/` | API landing metadata |
| `GET` | `/api/v1/health` | Server health and catalog stats |
| `GET` | `/api/v1/models` | List and filter models |
| `GET` | `/api/v1/models/{provider}/{id}` | Get one model |
| `GET` | `/api/v1/search` | Fuzzy search models |
| `GET` | `/api/v1/compare` | Side-by-side model comparison with pricing breakdown |
| `POST` | `/api/v1/validate` | Validate model + parameter usage |
| `GET` | `/api/v1/suggest` | Suggest canonical model IDs |
| `GET` | `/api/v1/providers` | List providers |
| `GET` | `/api/v1/providers/{id}` | Get provider details |
| `GET` | `/api/v1/aliases` | List aliases |
| `GET` | `/api/v1/aliases/{alias}` | Resolve one alias |
| `GET` | `/api/v1/catalog/health` | Catalog health summary |
| `GET` | `/api/v1/catalog/issues` | Catalog quality issues |
| `GET` | `/metrics` | Prometheus metrics (requires `--features metrics`) |

All responses use a consistent JSON envelope:

```json
{
  "success": true,
  "message": "OK",
  "data": {},
  "meta": { "timestamp": "2026-05-16T00:00:00Z" },
  "error": null
}
```

Paginated responses add `limit`, `offset`, and `total_data` to `meta`. Error responses set `success: false`, `data: null`, and include an `error.code` string.

Every response includes an `x-request-id` header. Responses are gzip-compressed when the client sends `Accept-Encoding: gzip`. List endpoints support `ETag` / `If-None-Match` for conditional requests.

---

## Deployment

See [docs/deployment.md](docs/deployment.md) for full instructions covering:

- **Render** (reference deployment)
- **Fly.io**
- **Railway**
- **Google Cloud Run**
- **Docker / Docker Compose** (self-hosted)
- **Build from source**

---

## Repository Structure

```
providers/          one YAML file per provider
models/<provider>/  one YAML file per model
aliases.yaml        shorthand aliases → canonical model IDs
schema/             JSON Schemas for catalog validation
validator/          Rust CLI that validates the catalog
server/             Rust HTTP API server
docs/               deployment guide, API contracts
```

---

## Run Locally

```bash
# API server
cargo run -p almanac-server

# Release build (much faster)
cargo run -p almanac-server --release

# With Prometheus metrics endpoint at GET /metrics
cargo run -p almanac-server --features metrics
```

Server listens on `0.0.0.0:8080` by default. Set `PORT` to change it.

---

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `8080` | TCP port |
| `DATA_DIR` | `.` | Directory containing catalog files |
| `RUST_LOG` | `almanac_server=info` | Log level filter |
| `LOG_FORMAT` | _(unset)_ | Set to `json` for structured logs |
| `RATE_LIMIT_RPS` | _(unset)_ | Max requests/sec per IP. Unset = disabled |
| `CATALOG_INCLUDE_PROVIDERS` | _(unset)_ | Comma-separated provider IDs to include |
| `CATALOG_EXCLUDE_PROVIDERS` | _(unset)_ | Comma-separated provider IDs to exclude |
| `CATALOG_INCLUDE_MODELS` | _(unset)_ | Comma-separated `provider/model-id` to include |
| `CATALOG_EXCLUDE_MODELS` | _(unset)_ | Comma-separated `provider/model-id` to exclude |
| `CATALOG_SCOPE_FILE` | _(unset)_ | YAML file for fine-grained catalog scoping |

---

## Catalog Scoping

Expose only part of the catalog without editing YAML files:

```bash
# Env vars (simple)
CATALOG_INCLUDE_PROVIDERS=openai,anthropic cargo run -p almanac-server

# Scope file (complex)
CATALOG_SCOPE_FILE=/etc/almanac/scope.yaml cargo run -p almanac-server
```

Scope file format:

```yaml
include:
  providers: [openai, anthropic]
  models: [google/gemini-2.5-pro]
exclude:
  providers: [xai]
  models: [openai/gpt-4o-mini]
```

Includes are applied first, excludes second. Hidden models are removed from all endpoints including search, suggest, validate, compare, and metrics.

---

## Local CI Checks

Run before pushing:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p almanac-validator
cargo test
cargo build -p almanac-server
```

---

## Contributing

When adding or updating a model:

1. Add or update `models/<provider>/<model-id>.yaml`
2. Ensure `id` matches the filename (without `.yaml`)
3. Ensure `provider` matches the directory name and `providers/<provider>.yaml`
4. Set `replacement` when deprecating or retiring a model
5. Update `aliases.yaml` if a new alias is needed
6. Include at least one source URL with a `last_verified` date
7. Run `cargo run -p almanac-validator && cargo test`

See `schema/model.schema.json` for the full model contract and `docs/api-contracts.md` for the response envelope spec.
