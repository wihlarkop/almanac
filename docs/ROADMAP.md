# Almanac development roadmap

> Planning snapshot: **2026-11-17**. [Canonical GitHub ROADMAP-001 #52](https://github.com/wihlarkop/almanac/issues/52) is the execution/DAG ledger; individual linked issues are the requirement and acceptance contracts. This document is a human-readable, repo-local mirror, not a second independently maintained roadmap. **All new implementation items are PROPOSED** until the owner authorizes a bounded issue/phase/batch. Check live GitHub state before dispatch.

## Product mission and boundaries

Almanac is the **developer-facing** source of truth and public Rust/Axum REST API for cross-provider model metadata, API identifiers and aliases, lifecycle, modalities, capability/parameter compatibility, pricing, source provenance, freshness and discovery. The website [almanac-web](https://github.com/wihlarkop/almanac-web) presents this API to users; it does not own another catalog. Preserve existing schema/validator and an easily deployable self-hosted API; no inference gateway, paid provider proxy, or unapproved credentials feature.

## Verified baseline and current risks (main, 2026-11-17)

- Repository `main` commit at planning time: `acad1dad586aa4389a1c4463ac0fb6a96f49d066`. Catalog files: **817 `models/<provider>/*.yaml` + 62 `providers/*.yaml`**; README says 292 aliases (not re-counted here). Do not confuse count with freshness or correctness.
- Scraper dispatch: **24 custom + 30 generic = 54 registrations**. Eight provider catalog entries have **no scraper registration**: `haiper`, `hidream`, `kling`, `pika`, `reve`, `suno`, `synthesia`, `udio`. A registration is not evidence of successful extraction.
- `scraper/src/writer.rs` generates potentially misleading fallback 4096 input/output context, zero prices, guessed capabilities, assumed active model and a fresh date. [BE-02](https://github.com/wihlarkop/almanac/issues/56) must make unknown values explicit before auto-generated catalog changes can be trusted.
- `scraper/src/engine.rs` currently processes HTTP responses without checking non-2xx status before HTML extraction; error/zero-result distinctions, host boundaries and backoff need review. The manual scraper workflow has `workflow_dispatch` only; no scheduled automation is presently asserted.
- Existing [backend PR #49](https://github.com/wihlarkop/almanac/pull/49) adds confidence stats/`unpriced_reason` and overlaps BE-10/11; **still open** at planning. Its observed validation workflow failed at formatting and dependency audit; older failed CI is not automatically resolved. Related [web PR #34](https://github.com/wihlarkop/almanac-web/pull/34) is also open and depends on backend compatibility.
- Keep existing `cargo fmt`, clippy, validator, tests, build, security audit and source-based data checks. Do **not** adopt other repos' no-tests policy.

## Proposed implementation issues and dependencies

| Phase | Issue / deliverable | Strict implementation dependencies | Parallel eligibility |
| --- | --- | --- | --- |
| P0 stability | [BE-01 #55](https://github.com/wihlarkop/almanac/issues/55) — CI, audit, lockfile, baseline | none | independent of BE-04 source inventory |
| P0 safe data | [BE-02 #56](https://github.com/wihlarkop/almanac/issues/56) — fail-closed unknown metadata and writer/schema | BE-01 checks | schema/validator/writer single writer |
| P0 scraper core | [BE-03 #57](https://github.com/wihlarkop/almanac/issues/57) — transport, extraction, host bounds, error receipts | BE-01, BE-02 | scraper core single writer |
| P0 coverage | [BE-04 #58](https://github.com/wihlarkop/almanac/issues/58) — inventory 62, add eight registrations | research: none; integration: BE-03 for shared dispatch | inventory in parallel, one registry writer |
| P1 core model cohort | [BE-05 #59](https://github.com/wihlarkop/almanac/issues/59) — seven core providers | BE-02, BE-03, BE-04 | provider-owned disjoint paths |
| P1 cloud/open-weight | [BE-06 #60](https://github.com/wihlarkop/almanac/issues/60) — eight platform providers | BE-02, BE-03, BE-04 | alongside BE-05/07/08/09 |
| P1 long tail | [BE-07 #61](https://github.com/wihlarkop/almanac/issues/61) — 18 chat/search/embedding providers | BE-02, BE-03, BE-04 | alongside other cohorts |
| P1 speech/music | [BE-08 #62](https://github.com/wihlarkop/almanac/issues/62) — nine speech/audio/music providers | BE-02, BE-03, BE-04 and BE-10 if new pricing units are needed | disjoint provider paths |
| P1 visual/3D | [BE-09 #63](https://github.com/wihlarkop/almanac/issues/63) — 20 image/video/3D providers | BE-02, BE-03, BE-04 and BE-10 if new pricing units are needed | disjoint provider paths |
| P1 pricing contract | [BE-10 #64](https://github.com/wihlarkop/almanac/issues/64) — modality pricing/provenance semantics | BE-02; review PR #49 overlap | schema/API owner; parallel read-only research |
| P1 catalog audit | [BE-11 #65](https://github.com/wihlarkop/almanac/issues/65) — lifecycle, source verification and alias freshness for 817 entries | BE-02, BE-04; review relevant cohort outputs and BE-10 | shard source research, serialize alias/schema integration |
| P1 safe automation | [BE-12 #66](https://github.com/wihlarkop/almanac/issues/66) — review-only discovery runs and optional schedule | BE-03, BE-04 and at least one verified provider flow | workflow owner; **schedule requires owner authorization** |
| P2 API developer experience | [BE-13 #67](https://github.com/wihlarkop/almanac/issues/67) — OpenAPI, filters/compare/validation, SDK readiness | BE-01, BE-02, BE-10 for changed contracts | route owners if disjoint |
| P2 runtime operations | [BE-14 #68](https://github.com/wihlarkop/almanac/issues/68) — benchmarks/cache/metrics/rate limits/deploy docs | BE-01; BE-13 for public API behavior changes only | independently after baseline |
| P2 release integration | [BE-15 #69](https://github.com/wihlarkop/almanac/issues/69) — data versioning, consumer contracts, rollback/acceptance | BE-10, BE-11, BE-13 + matching web changes | integrated final owner |

**DAG is dependency-based, not a forced global wave barrier.** For example, BE-14 can proceed after BE-01 without waiting for a separate long-tail provider cohort. Read-only provider gap research can start while schema work is still blocked. Implement shared-contract-dependent provider writers only against an authorized verified integrated prerequisite, not a speculative stub. Work within one explicitly authorized multi-feature batch may use traceable staged integration/stacked PRs, but cannot bypass security/data truth or cross-repo compatibility gates.

## Complete 62-provider coverage ownership — five nonoverlapping cohorts

| Provider cohort | Count | Provider IDs |
| --- | ---: | --- |
| [BE-05](https://github.com/wihlarkop/almanac/issues/59) Core LLM | 7 | `openai`, `anthropic`, `google`, `xai`, `mistral`, `deepseek`, `cohere` |
| [BE-06](https://github.com/wihlarkop/almanac/issues/60) Platforms / open weights | 8 | `amazon`, `microsoft`, `meta`, `alibaba`, `bytedance`, `tencent`, `baidu`, `nvidia` |
| [BE-07](https://github.com/wihlarkop/almanac/issues/61) Chat/search/embeddings | 18 | `ai21`, `ibm`, `inception`, `inflection`, `jina`, `minimax`, `moonshot`, `naver`, `nomic`, `perplexity`, `reka`, `stepfun`, `upstage`, `voyageai`, `writer`, `xiaomi`, `yi`, `zai` |
| [BE-08](https://github.com/wihlarkop/almanac/issues/62) Audio/music | 9 | `assemblyai`, `cartesia`, `deepgram`, `elevenlabs`, `inworld`, `lmnt`, `playht`, `suno`, `udio` |
| [BE-09](https://github.com/wihlarkop/almanac/issues/63) Image/video/3D | 20 | `adobe`, `bfl`, `haiper`, `heygen`, `hidream`, `ideogram`, `kling`, `leonardo`, `lightricks`, `luma`, `meshy`, `pika`, `pixverse`, `recraft`, `reve`, `runway`, `stabilityai`, `synthesia`, `tripo`, `vidu` |
| **Total** | **62** | All catalog provider IDs present at planning time |

Provider **availability and scraper feasibility must be verified**. Some offerings may be consumer-only or restrict official API/document access; mark unsupported, blocked or unknown instead of fabricating model information. Work on each cohort may be split further into provider-specific worktrees with exclusive YAML/spider ownership.

## Definition of done per provider

1. Match provider catalog IDs, documented API model IDs/aliases and official current source(s); record every source URL, last verified timestamp, jurisdiction/region/version qualification and scraping method.
2. Verify status, release/deprecation/sunset/replacement, context/output limits, input/output modality, supported parameter/capability and provider price unit/tier. Distinguish **not found**, **not published**, **not applicable** and **known free**.
3. Confirm parser errors and 403/429/5xx do not silently replace catalog data; produce discovery/diff and source confidence with no false retirement inferred from missing docs.
4. Validate changed YAML/schema with current Rust validator and relevant checks; compare to baseline. Review scraped new entries manually before merging and record unresolved fields; no fabricated defaults.
5. Source review + GitHub CI do **not** prove current live API provider availability or production deployment. Record unperformed live checks as `NOT RUN`.

## API and web handoff

- Backend changes to `Model`, `Pricing`, catalog stats, statuses, aliases or validation must include compatibility notes and corresponding [frontend issues](https://github.com/wihlarkop/almanac-web/issues/35).
- Fix/review existing [backend PR #49](https://github.com/wihlarkop/almanac/pull/49) and [web PR #34](https://github.com/wihlarkop/almanac-web/pull/34) **in place** where overlapping. Do not assume either merged; verify exact SHA and deploy API first when the web requires new fields.
- Keep `/api/v1` backward compatibility unless explicitly approved; schema/OpenAPI type changes, caching and pricing unit changes must be audited as contracts.

## Operation and acceptance

[AGENTS.md](../AGENTS.md) and [parallel playbook](operations/parallel-execution.md) apply. [OPS #53](https://github.com/wihlarkop/almanac/issues/53) tracks one consolidated batch handoff, [QA #54](https://github.com/wihlarkop/almanac/issues/54) tracks one owner-run end-of-batch API/catalog acceptance. Backend tests/validation run during implementation, not on demand per owner QA. No automatic schedule, paid requests, data/public release or deploy without separate owner approval.

**Possible first bounded batch (proposal, not authorization):** BE-01 CI + BE-04 coverage inventory + BE-14 source-only performance discovery; review existing PR #49 for overlap. Follow-on: safe writer/transport/price contracts, then independently owned provider cohorts, then freshness/integration.
