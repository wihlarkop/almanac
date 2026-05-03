# almanac

Almanac is a free, open-source model catalog and validator for LLM developers.

It answers practical questions:

- What models exist across major LLM providers?
- Is this model string valid?
- Is this model active, deprecated, or retired?
- Which provider owns this model?
- What capabilities, modalities, limits, pricing, and parameters are known?
- What canonical model ID should this alias resolve to?

## Repository Structure

- `providers/` - one YAML file per provider.
- `models/<provider>/` - one YAML file per model.
- `aliases.yaml` - shorthand aliases mapped to canonical model IDs.
- `schema/` - JSON Schemas used to validate catalog files.
- `validator/` - Rust CLI that validates the catalog.
- `server/` - Rust HTTP API for querying and validating model metadata.

## Local CI Checks

Run the same core checks before pushing:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p almanac-validator
cargo test
cargo build -p almanac-server
```

Dependency audit is enforced in GitHub Actions:

```powershell
cargo install cargo-audit --version 0.22.1 --locked
cargo audit
```

The repository pins Rust with `rust-toolchain.toml`. If Rustup prompts to install the pinned
toolchain, accept it before running the checks.

## Validate Locally

```bash
cargo run -p almanac-validator
```

The validator checks schema validity, filename/id consistency, provider references, aliases, lifecycle dates, replacements, and parameter conflicts.

## Run the API Server

```bash
cargo run -p almanac-server
```

The server listens on `0.0.0.0:8080` by default.

## Run with Docker

```bash
docker build -t almanac-server:local .
docker run --rm -p 8080:8080 almanac-server:local
```

The production image includes the public catalog under `/data` and runs the server as a non-root
user. The Docker build context intentionally excludes `docs/`.

Optional environment variables:

- `PORT` - server port.
- `DATA_DIR` - directory containing `providers/`, `models/`, and `aliases.yaml`.
- `RUST_LOG` - tracing filter, for example `almanac_server=debug,tower_http=info`.

The server logs startup, catalog loading, request traces, and shutdown events. It handles Ctrl+C
and SIGTERM as graceful shutdown signals.

Successful JSON responses use this envelope:

```json
{
  "success": true,
  "message": "OK",
  "data": {},
  "meta": {
    "timestamp": "2026-05-02T21:12:00Z"
  },
  "error": null
}
```

Paginated responses add `limit`, `offset`, and `total_data` to `meta`. Error responses use the
same envelope with `success=false`, `data=null`, and an `error.code` value.

Every API response includes an `x-request-id` header. Successful JSON responses also include the
same request id and request execution time in `meta`. Browser access is enabled with explicit CORS
for GET and POST API calls, and basic security headers are set on responses.

Requests are limited to 64 KiB bodies and a 10 second server-side timeout.

## CI Checks

The GitHub Actions validation workflow runs:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo run -p almanac-validator`
- `cargo test`
- `cargo build -p almanac-server`
- RustSec dependency audit

## API Examples

Health:

```bash
curl http://localhost:8080/api/v1/health
```

List providers:

```bash
curl http://localhost:8080/api/v1/providers
```

List all models:

```bash
curl http://localhost:8080/api/v1/models
```

Filter models:

```bash
curl "http://localhost:8080/api/v1/models?provider=openai&status=active&capability=vision"
```

Paginate, sort, and filter models:

```bash
curl "http://localhost:8080/api/v1/models?limit=20&offset=0&sort=context_window&order=desc&modality_input=image&min_context=100000"
```

Model list pagination defaults to `limit=20` and `offset=0`.

Get one model:

```bash
curl http://localhost:8080/api/v1/models/openai/gpt-4o
```

Validate a model string:

```bash
curl -X POST http://localhost:8080/api/v1/validate \
  -H "content-type: application/json" \
  -d '{"model":"gpt-4o","provider":"openai"}'
```

Validate request compatibility:

```bash
curl -X POST http://localhost:8080/api/v1/validate \
  -H "content-type: application/json" \
  -d '{
    "model":"grok-4.20-reasoning",
    "provider":"xai",
    "parameters":{"temperature":0.7,"stream":true},
    "modalities":{"input":["text","image"],"output":["text"]}
  }'
```

Suggest likely model IDs:

```bash
curl "http://localhost:8080/api/v1/suggest?q=claude-opus-4.7"
```

API documentation:

```bash
curl http://localhost:8080/openapi.json
```

Interactive docs are available at:

- `http://localhost:8080/swagger-ui/`
- `http://localhost:8080/scalar`

## Model File Format

Each model YAML file includes:

- canonical model ID
- provider ID
- display name
- lifecycle status
- release, deprecation, and sunset dates
- replacement model
- context and output token limits
- modalities
- capabilities
- supported, rejected, and deprecated parameters
- model-level last verified date
- metadata confidence
- endpoint family
- pricing
- source URLs with verification dates

See `schema/model.schema.json` for the full contract.

## Contributing

When adding or changing a model:

1. Add or update `models/<provider>/<model-id>.yaml`.
2. Make sure `id` matches the filename without `.yaml`.
3. Make sure `provider` matches both `providers/<provider>.yaml` and the model directory.
4. Add a `replacement` when deprecating or retiring a model, if a successor exists.
5. Update `aliases.yaml` only when the alias should resolve to a non-retired canonical model.
6. Include at least one source URL and `last_verified` date.
7. Keep model-level `last_verified`, `confidence`, and `endpoint_family` current.
8. Run:

```bash
cargo run -p almanac-validator
cargo test
```
