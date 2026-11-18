# Parallel execution playbook — Almanac

Read [AGENTS.md](../../AGENTS.md), [ROADMAP #52](https://github.com/wihlarkop/almanac/issues/52), [OPS #53](https://github.com/wihlarkop/almanac/issues/53), [QA #54](https://github.com/wihlarkop/almanac/issues/54), and the target issues first. This playbook mirrors the user-approved Movryn multi-feature fan-out/fan-in pattern while retaining Almanac's **existing CI/tests**.

## 1. Scope and dynamic DAG

- A request such as `BE-02` authorizes just one issue; `next eligible parallel wave` means the next bounded set of independent READY work; `full P1 batch` allows in-scope staged dependencies as they become verifiably integrated. **Planning/READY alone is not an execution order**.
- Before dispatch inspect actual remote main SHA, open PRs #49/other new PRs, the web dependency #34, source tree, dirty worktrees, issue status and dependency graph. Classify **COVERED / PROPOSED / READY / IN_PROGRESS / DRAFT_PR / WAITING / BLOCKED / MERGED / DEFERRED_QA**. Do not duplicate current work.
- A dependency arrow is strict only for **code actually consuming a contract**. Read-only discovery/metadata research may proceed early in an approved scope, but must not publish fabricated or unreviewable YAML.
- No global wave barrier: BE-04 inventory can run alongside BE-01; BE-14 performance work can start after BE-01 without waiting for visual-provider work; BE-05..09 cohorts can each progress after safe BE-02/03/04 interfaces are integrated. Shared schema or pricing additions serialize under BE-10's owner.

## 2. Reservation ledger before fan-out

Write an explicit table per authorized batch:

| Issue/task | Agent/model & attested runtime? | Separate branch/worktree | Base and prerequisite SHAs | Reserved paths | Expected evidence |
| --- | --- | --- | --- | --- | --- |
| BE-01 | chosen worker | `worktrees/be-ci` | exact remote main | workflows, Cargo manifests/lock | required checks |
| BE-04 inventory | separate worker | `worktrees/be-coverage` | same base | coverage docs/registry (registry single-writer) | all-62 matrix |
| BE-05 provider subset | separate worker | `worktrees/provider-openai` | integrated BE-02/03/04 when needed | `models/openai/`, `scraper/src/spiders/openai.rs` | verified docs, diff |
| Batch integrator | root or dedicated coordinator | `worktrees/almanac-integration` | pinned feature heads | shared schema/dispatch/lockfiles | merged branch manifest |

This table is illustrative and **does not authorize the example issues**. One writer/worktree per independently edited task; resource limits govern practical concurrency. Direct and nested subagents are permitted only when the harness truly supports them; report effective runtime model/effort `UNVERIFIED` unless attested.

## 3. Provider cohort fan-out and source integrity

- Divide the 62 IDs into [BE-05 #59](https://github.com/wihlarkop/almanac/issues/59), [BE-06 #60](https://github.com/wihlarkop/almanac/issues/60), [BE-07 #61](https://github.com/wihlarkop/almanac/issues/61), [BE-08 #62](https://github.com/wihlarkop/almanac/issues/62), and [BE-09 #63](https://github.com/wihlarkop/almanac/issues/63). Each agent owns a nonoverlapping provider subset; 24 custom, 30 generic, eight missing registration is the **baseline** only.
- Separate **source discovery** (candidate IDs and documentation) from **source verification** (official API ID, pricing scope, parameters/lifecycle/source date) and **catalog publication** (validated reviewed YAML). Never promote absent metadata into zero/false/active or claim an HTTP 403 means retired.
- Reserve shared `scraper/src/main.rs`, `engine.rs`, `writer.rs`, schema, validator, `aliases.yaml`, `Cargo.lock` and workflow for one designated integrator. Avoid simultaneous writes to any common path even if worktrees are separate.
- No paid API calls, authentication bypass, hidden external network, unauthorized robots/ToS bypass, continuous scheduled scraping, publication or production data mutation.


## Creator-only expansion and scraper v2 dispatch (2026-11-17)

Before onboarding a **new provider**, screen whether it is a **first-party model creator** using [BE-16 #71](https://github.com/wihlarkop/almanac/issues/71) and [BE-20 #75](https://github.com/wihlarkop/almanac/issues/75). A company with only inference hosting/routing is excluded from canonical provider entries. Mixed creator/host companies may submit only independently authored model families. The historic 62-provider inventory is a bounded audit baseline, not an upper limit or endorsement of each provider's current lineage.

**Example new candidate:** TypeSafe AI (developer) → Jev (System One typed judgment model), [BE-17 #72](https://github.com/wihlarkop/almanac/issues/72). Official model ownership and real API ID are separately verified. Any new endpoint taxonomy must be integrated by the designated shared schema writer before independently owned TypeSafe provider model YAML is eligible.

**Scraper v2 worktree map** (illustrative only; actual owner scope and resource budget still needed):

| Planned work | Single-writer boundaries | May run before prerequisites? |
| --- | --- | --- |
| BE-16 creator audit | provider ownership/eligibility inventory only | read-only audit yes |
| BE-03 safe transport | `scraper/src/engine.rs`, request/response safety | after BE-01/BE-02 |
| BE-18 adapters | provider source descriptors, adapter trait and shared extraction interface | design yes; code after BE-03 |
| BE-19 evidence/diff | semantic field observations, freshness receipts, review queue | design yes; code after BE-18/BE-10 |
| BE-20 new creator discovery | source research and screened creator candidate list | read-only discovery yes; automated implementation after BE-16/18/19 |
| BE-05..09 existing cohorts + BE-17 Jev | exclusive `models/<creator>/` and spider files for each creator, one registry integrator | provider research yes, code after required contracts |

**Quality gates:** require official source URL and per-field evidence; classify `CANDIDATE` versus `VERIFIED_CREATOR`, `SOURCE_VERIFIED` versus merely scraped; distinguish blocked sources, partial fetches and authentic no-new-model results. Never auto-retire on missing HTML, fabricate zero/free price or mark a provider eligible because a hosting website lists another lab's model. No unauthorized paid calls or live schedule. Maintain compatibility with `almanac-web` [FE-12 #50](https://github.com/wihlarkop/almanac-web/issues/50).

## 4. Feature fan-in, batch fan-in and review

1. Per-feature coordinator integrates its children in a dedicated **non-main** worktree. Preserve exact commits and owned file list; resolve shared changes only under designated writer.
2. Batch integrator combines independently safe feature heads into a **separate non-main integrated tree**. When necessary allow explicit stacked branches/PRs with real base chain. Never claim a dependent PR is mainline-ready before upstream dependency and final combined security/data-integrity review.
3. Prefer focused feature Draft PRs and optionally one integration Draft PR; choose/document **exactly one route into main** so code is not merged twice. If a PR becomes superseded, reconcile explicitly only after verifying integration.
4. During implementation run safe existing Rust checks including fmt, clippy, validator, tests, build, dependency audit; all CI failures are evidence. Do not interpret provider network failure, smoke check or schema validity as proof of fresh live model facts.
5. One combined handoff to ChatGPT: issue→agent→worktree, exact base/head SHAs, dependency tree and combined integration SHA, all paths and sources, check PASS/FAIL/NOT RUN, P0/P1 blockers, remediation and **one API/catalog manual acceptance checklist**. Worker agents do **not** merge into `main` or deploy.
6. ChatGPT independently reviews exact-head feature PRs and the final integrated tree. Request same-PR repair for findings; approved changes can be merged within owner scope. Owner acceptance of API/catalog behavior occurs **once per integrated batch**; [QA #54](https://github.com/wihlarkop/almanac/issues/54) records PASS/FAIL or `DEFERRED/NOT RUN`.
7. No scraping schedule, public release, Docker/registry push, Cloud deploy, schema breaking change or unrelated next phase without separate owner approval.

## 5. Compact handoff format

```text
Authorized batch: BE-xx + BE-yy
Remote base main SHA:
Feature/child tasks: [issue, agent, chosen model/effort, actual runtime VERIFIED/UNVERIFIED, worktree, exclusive files]
Prerequisite source SHA chain:
Feature Draft PRs: [number, base, exact head]
Integrated Draft PR (if used): [number, exact head, included upstream PRs]
Changed contracts/data: [version, provider, official source URL, last_verified, unknown fields]
CI and local checks: [command, PASS/FAIL/NOT RUN/BLOCKED, links]
Security/data quality blockers: [severity, affected workstream]
Final owner manual API acceptance: DEFERRED / NOT RUN
Release/deployment/external scraping: NOT AUTHORIZED
Stop: ChatGPT exact-HEAD review
```
