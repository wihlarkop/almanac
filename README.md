# almanac

The model catalog and validator for LLM developers.

A free, open-source registry of LLM models answering:
- What models exist across all providers?
- Is this model string valid / deprecated?
- Does my code use it correctly?

## Structure

- `providers/` — one YAML file per provider
- `models/<provider>/` — one YAML file per model
- `aliases.yaml` — canonical aliases (e.g. `claude-opus-4` → `claude-opus-4-7`)
- `schema/` — JSON Schema for YAML validation
- `validator/` — Rust binary that validates all YAML files against the schema

## Validate locally

```bash
cargo run -p almanac-validator
```

## Contributing

Each model lives in its own YAML file. Add or update models by opening a PR — CI validates the schema automatically.