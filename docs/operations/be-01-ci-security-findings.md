# BE-01 CI and security findings

Base: `acad1dad586aa4389a1c4463ac0fb6a96f49d066` (2026-11-17).

## Docker MUSL failure

The `Dockerfile` installs `x86_64-unknown-linux-musl` in the `chef` stage before the repository is copied. `rust-toolchain.toml` pins `1.96.0`; the image uses the floating `rust:1.96-slim` tag. In hosted Docker run [44061025429](https://github.com/wihlarkop/almanac/actions/runs/44061025429), `cargo chef cook` completed under the base toolchain, then the final builder command selected 1.96.0 and failed with `E0463: can't find crate for core` for the MUSL target. This supports a base-image/toolchain mismatch: the target installation can apply to the floating tag's toolchain, then `rust-toolchain.toml` selects a different patch toolchain. The Dockerfile now pins `rust:1.96.0-slim` so the installed target and selected toolchain match. This is source/log-supported. On the current PR head, hosted Docker run [44261271663](https://github.com/wihlarkop/almanac/actions/runs/44261271663) passed both **Build smoke image** and **Run smoke test**, validating the exact version pin.

Local Docker verification: **NOT RUN**. The Docker executable is present, but `docker version` cannot connect to the Docker engine named pipe; its user config is also inaccessible.

## Hosted validation and dependency audit

### Historical PR #70 baseline

PR #70's Validate run [44061025121](https://github.com/wihlarkop/almanac/actions/runs/44061025121) had `validate` **PASS** (fmt, clippy, YAML validator, tests, server build) and `audit` **FAIL**. Audit run logs recorded 20 vulnerabilities in these locked package versions:

| Package | Locked version | Advisory IDs |
| --- | --- | --- |
| `h2` | 0.4.13 | RUSTSEC-2026-0258 |
| `quinn-proto` | 0.11.14 | RUSTSEC-2026-0185, RUSTSEC-2026-0338, RUSTSEC-2026-0339, RUSTSEC-2026-0340, RUSTSEC-2026-0342, RUSTSEC-2026-0523, RUSTSEC-2026-0524 |
| `quinn-udp` | 0.5.14 | RUSTSEC-2026-0538 |
| `rustls` | 0.23.40 | RUSTSEC-2026-0285, RUSTSEC-2026-0346, RUSTSEC-2026-0348, RUSTSEC-2026-0349 |
| `rustls-webpki` | 0.103.13 | RUSTSEC-2026-0382 |
| `webpki-root-certs` | 1.0.7 | RUSTSEC-2026-0383 |
| `webpki-roots` | 1.0.7 | RUSTSEC-2026-0381 |
| `zip` | 3.0.0 | RUSTSEC-2026-0367, RUSTSEC-2026-0368, RUSTSEC-2026-0369, RUSTSEC-2026-0429 |

### PR #77 hosted run on source head 5119de2db39785019912be3ae0b761d47c13d530

Validate run [44261271636](https://github.com/wihlarkop/almanac/actions/runs/44261271636) on head `5119de2db39785019912be3ae0b761d47c13d530`: `validate` **PASS** (fmt, Clippy, YAML validator, tests, server build); `audit` **FAIL**. The audit database was updated `2026-11-17T11:14:35+01:00`; it scanned 356 lock entries and found 22 vulnerabilities:

| Package | Locked version | Advisory IDs |
| --- | --- | --- |
| `h2` | 0.4.13 | RUSTSEC-2026-0258 |
| `quinn-proto` | 0.11.14 | RUSTSEC-2026-0185, RUSTSEC-2026-0338, RUSTSEC-2026-0339, RUSTSEC-2026-0340, RUSTSEC-2026-0342, RUSTSEC-2026-0523, RUSTSEC-2026-0524 |
| `quinn-udp` | 0.5.14 | RUSTSEC-2026-0538 |
| `rustls` | 0.23.40 | RUSTSEC-2026-0285, RUSTSEC-2026-0346, RUSTSEC-2026-0348, RUSTSEC-2026-0349 |
| `rustls-webpki` | 0.103.13 | RUSTSEC-2026-0382 |
| `webpki-root-certs` | 1.0.7 | RUSTSEC-2026-0383 |
| `webpki-roots` | 1.0.7 | RUSTSEC-2026-0381 |
| `zip` | 3.0.0 | RUSTSEC-2026-0367, RUSTSEC-2026-0368, RUSTSEC-2026-0369, RUSTSEC-2026-0429, RUSTSEC-2026-0545, RUSTSEC-2026-0548 |

The current run also reported 3 unmaintained packages (`fxhash 0.2.1`, `paste 1.0.15`, `unsafe-libyaml 0.2.11`) and 4 unsound packages (`anyhow 1.0.102`, `event-listener 5.4.1`, `rustix 1.1.4`, `tokio 1.52.3`). The hosted Docker run [44261271663](https://github.com/wihlarkop/almanac/actions/runs/44261271663) completed with the `smoke` job **PASS**, including image build and smoke test. This validates the Dockerfile pin at that SHA; the separate security audit remains unresolved.

A local `cargo audit --json --file Cargo.lock` invocation reported 22 vulnerability findings and 356 locked dependencies, but its JSON output was not retained. A repeat was blocked because the advisory database lock is under the read-only user Cargo directory. Do not treat that count as the hosted run's exact list or assume the difference is a code change.

The current checkout uses workspace version `0.27.0` consistently in manifests, but the three workspace-package entries in `Cargo.lock` still say `0.1.0` (`almanac-server`, `almanac-validator`, and `almanac-scraper`). This explains why Cargo says the lockfile needs updating. PR #49's description independently says it syncs `Cargo.lock` to workspace version `0.27.0`, so this stale lock state is present on the current main base and that PR already carries the lockfile correction alongside its feature. The current PR #49 branch is not an approved dependency base and was not copied or modified here. A `cargo update` attempt failed fetching crates.io's `config.json` because network access is unavailable; no lockfile was edited. Resolve the workspace lock mismatch and dependency advisories together on an approved integration base before merging.

## Checks and status

- `cargo fmt --check`: **PASS**
- `git diff --check`: **PASS**
- Local Clippy, validator, Rust tests, server build: **NOT RUN** (the hosted Validate job passed these on the prior exact PR head)
- `cargo audit --json --file Cargo.lock`: **FAIL / BLOCKED** as detailed above
- `cargo update`: **BLOCKED**, crates.io unavailable
- Local Docker image build: **NOT RUN**, Docker engine unavailable; hosted smoke build/test: **PASS** on run 44261271663
- Latest hosted runs on source head `5119de2db39785019912be3ae0b761d47c13d530`: Validate `validate` **PASS**, audit **FAIL** (run 44261271636); Docker smoke **PASS** (run 44261271663). This report was updated afterward in a documentation-only commit.

The required audit remains red and this Draft PR is not a complete CI/security remediation. Re-run Cargo dependency resolution in a network-enabled environment, trace affected packages to their consumers and features, apply compatible patched versions through Cargo, and rerun the existing checks before considering merge. No security check was weakened or bypassed.
