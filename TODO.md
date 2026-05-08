# Production Readiness TODO

These items are intentionally deferred from the first production-readiness batch.

- [x] Add JSON log mode for production, controlled by an environment variable.
- [x] Add Prometheus metrics endpoint for request counts, latency, status codes, and catalog counts.
- [x] Add optional source URL reachability checks for catalog validation.
- [ ] Add generated catalog summary report.
- [x] Add API changelog and response-contract documentation.
- [ ] Add dependency license policy with `cargo-deny`.
- [x] Add application-level rate limiting with a cloneable Axum-compatible limiter, or enforce it at the edge/proxy.
- [ ] Add deployment blueprint for the chosen host.
- [ ] Add load tests or benchmarks for `/v1/models`, `/v1/validate`, and `/v1/suggest`.
- [ ] Add generated SDK workflow after OpenAPI schema stabilizes.
