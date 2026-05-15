# Production Readiness TODO

These items are intentionally deferred from the first production-readiness batch.

- [x] Add JSON log mode for production, controlled by an environment variable.
- [x] Add Prometheus metrics endpoint for request counts, latency, status codes, and catalog counts.
- [x] Add optional source URL reachability checks for catalog validation.
- [x] Add generated catalog summary report.
- [x] Add API changelog and response-contract documentation.
- [x] Add dependency license policy with `cargo-deny`.
- [x] Add application-level rate limiting with a cloneable Axum-compatible limiter, or enforce it at the edge/proxy.
- [ ] Add deployment blueprint for the chosen host.
- [ ] Add load tests or benchmarks for `/v1/models`, `/v1/validate`, and `/v1/suggest`.
- [ ] Add generated SDK workflow after OpenAPI schema stabilizes.

---

# Roadmap

## SDK Clients
- [ ] Generate Python client from OpenAPI spec and publish to PyPI as `almanac-client`
- [ ] Generate TypeScript client from OpenAPI spec and publish to npm as `@almanac/client`
- [ ] Add CI step to auto-regenerate clients on OpenAPI schema changes

## Staleness Bot
- [ ] Add weekly GitHub Actions cron job that flags models with `last_verified` older than 90 days
- [ ] Auto-open an issue listing stale models grouped by provider
- [ ] Consider auto-PR with updated `last_verified` dates after manual verification

## Pricing Comparison Endpoint
- [ ] Add `GET /api/v1/compare/pricing?ids=<id1>,<id2>,...` endpoint
- [ ] Return side-by-side cost breakdown (input, output, per_image, per_second, etc.)
- [ ] Normalize costs to a common unit (e.g. cost per 1M tokens equivalent) where possible

## Changelog Feed
- [ ] Add `GET /api/v1/changelog?since=<date>` endpoint listing models added/deprecated/updated
- [ ] Track model status transitions (active → deprecated, null → active) in git history
- [ ] Consider an RSS/Atom feed for teams that want passive updates

## Website / UI
- [ ] Build a static frontend (Next.js or Astro) for browsing, filtering, and comparing models visually
- [ ] Deploy to a subdomain (e.g. almanac.dev or via Cloudflare Pages)
- [ ] Add model detail pages, provider pages, and a pricing comparison table
