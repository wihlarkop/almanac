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

## Validate Locally

```bash
cargo run -p almanac-validator
```

The validator checks schema validity, filename/id consistency, provider references, aliases, lifecycle dates, replacements, and parameter conflicts.

## Run Tests

```bash
cargo test
```

## Run the API Server

```bash
cargo run -p almanac-server
```

The server listens on `0.0.0.0:8080` by default.

Optional environment variables:

- `PORT` - server port.
- `DATA_DIR` - directory containing `providers/`, `models/`, and `aliases.yaml`.

## API Examples

Health:

```bash
curl http://localhost:8080/v1/health
```

List providers:

```bash
curl http://localhost:8080/v1/providers
```

List all models:

```bash
curl http://localhost:8080/v1/models
```

Filter models:

```bash
curl "http://localhost:8080/v1/models?provider=openai&status=active&capability=vision"
```

Get one model:

```bash
curl http://localhost:8080/v1/models/openai/gpt-4o
```

Validate a model string:

```bash
curl -X POST http://localhost:8080/v1/validate \
  -H "content-type: application/json" \
  -d '{"model":"gpt-4o","provider":"openai"}'
```

Suggest likely model IDs:

```bash
curl "http://localhost:8080/v1/suggest?q=claude-opus-4.7"
```

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
7. Run:

```bash
cargo run -p almanac-validator
cargo test
```
