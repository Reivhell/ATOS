# Validation report — Agentic Trading OS v0.10.0

**Date:** 2026-10-30  
**Candidate:** v0.10.0 — Rust-native research orchestration and immutable evidence workspace  
**Release status:** **BLOCKED — NOT PRODUCTION-CERTIFIED**

## Executive result

This candidate adds a substantial Research Lab subsystem rather than a cosmetic version change. Research hypotheses, workflow execution and evidence run cards are now integrated through Tauri into a separate Rust-owned SQLite workspace. The research pipeline consumes the existing native backtest result for the exact supported `BTCUSDT` 5m series, then performs out-of-sample review, deterministic moving-block bootstrap sensitivity analysis, drawdown review and a bounded research committee decision.

The research subsystem has no API for creating orders, modifying balances or positions, changing risk limits, releasing the account kill switch, or enabling live execution. A passing decision means only that the result may enter a separately supervised paper-validation phase. The five specialist role labels are deterministic Rust validators; they are not autonomous LLM agents.

**The package is not safe to label production-ready or deploy as a certified release.** JavaScript and source-contract checks pass, but native Rust tests/builds and Svelte/Vite/Tauri packaging could not run in this workspace. The release preflight remains fail-closed on 10 blockers.

## Material changes

- Added a standalone Rust `ResearchWorkspace` database, deliberately separate from the trading account-state database.
- Added hypothesis registration and invalidation with optimistic revision checks and audit events. The creation boundary accepts only the exact native `BTCUSDT` 5m market-data contract.
- Added a validated six-node research DAG with independent OOS and block-bootstrap worker branches, panic-to-error handling, and pre-commit evidence validation.
- Added reproducible run cards containing the strategy parameters, dataset SHA-256 and time range, fee/slippage/risk assumptions, all gate thresholds, review graph, statistical summaries, caveats, blockers and card hash.
- Added SQLite immutability triggers for hypothesis identity, run cards and event history; strict normalized trigger-definition checks; replay-based hypothesis lifecycle validation; startup and pre-write transaction verification; append-only SHA-256 event chain; and payload/card semantic validation.
- Added bounded resource policies: up to 200 hypotheses, 100 immutable run cards, 1,024 audit events, 1 MB per card, 500,000 bootstrap observations and a 25,000,000 resampled-observation work budget.
- Corrected the resampling method label and separated return quantiles from drawdown gates. Replaced the basic xorshift generator with seeded PCG-XSH-RR 64/32 and rejection-sampled block indices to avoid modulo bias. This sampler is for reproducible statistical resampling, not cryptographic randomness.
- Added UI lifecycle controls, stale-invalidated-hypothesis selection rejection, duplicate-create protection, and accurate total-vs-loaded run-card counts.

## Local verification results

| Check | Result | Details |
|---|---|---|
| `npm test` | **PASS** | 266 tests passed, 0 failed across 41 npm test command groups |
| `npm run typecheck:layers` | **PASS** | Strict TypeScript/JavaScript layer typecheck |
| JavaScript syntax scan | **PASS** | All 85 `.js`, `.mjs`, and `.cjs` files under `src`, `api`, and `scripts` checked with `node --check` |
| `npm run verify:repo` | **PASS with warnings** | Static repository guardrails pass; reproducible dependency lockfiles are absent |
| SQLite schema/trigger smoke check | **PASS** | Python standard-library SQLite verified the research DDL, immutable identity/no-delete triggers, and allowed transactional lifecycle updates; this is not a Rust integration test |
| AI Agent release qualification | **PASS** | 10/10 baseline application-boundary qualification remains green through release preflight |
| Research workspace contract suite | **PASS** | Seven source-contract tests are included in the 266-test suite |
| Rust native tests / `cargo test` | **NOT RUN** | `cargo` is not installed in this environment |
| `rustfmt` / Clippy | **NOT RUN** | `rustfmt` and `clippy` are not installed |
| Svelte/Vite production build | **BLOCKED** | `npm run build` exits because `svelte-check` is unavailable; Vite executable is also missing |
| Tauri packaging | **NOT RUN** | Tauri CLI is unavailable |
| Release preflight | **BLOCKED** | 10 release blockers remain; see below |

The JavaScript suite and type-layer checks are source-level evidence. They do not substitute for compiling or executing the Rust module, parsing/compiling the Svelte component, native SQLite tests, or producing a desktop installer.

## Release blockers (10)

1. `package-lock.json` is missing; npm dependency resolution is not pinned.
2. `bun.lock` is missing; Bun/Hono dependency resolution is not pinned.
3. `src-tauri/Cargo.lock` is missing; Rust dependency resolution is not pinned.
4. `svelte-check` is unavailable.
5. Vite is unavailable.
6. Tauri CLI is unavailable.
7. `cargo --version` cannot run.
8. `rustfmt --version` cannot run.
9. `cargo clippy --version` cannot run.
10. Eight third-party GitHub Actions still use mutable references and must be reviewed and pinned to full 40-character commit SHAs: `Swatinem/rust-cache@v2`, `actions/attest@v4`, `actions/checkout@v4`, `actions/setup-node@v4`, `actions/upload-artifact@v4`, `anchore/sbom-action@v0`, `dtolnay/rust-toolchain@stable`, and `oven-sh/setup-bun@v2`.

No lockfiles or action SHAs were fabricated. The preceding v0.9.6 validation record states that external registry/GitHub metadata resolution failed due DNS/network restrictions. That limitation remains; action hashes must be reviewed from authoritative upstream releases and committed, not guessed.

## Not verified

- Rust compilation, unit tests, formatting, Clippy, linker compatibility and platform-specific native build behavior.
- Svelte component compilation, Vite bundling, Tauri desktop packaging and packaged-app runtime behavior.
- SQLite research workspace runtime tests on actual target platforms; the new Rust unit tests are written but not executed here.
- Live provider transport, operating-system keychain behavior, local model/provider integrations and Bun/Hono integration.
- Signed GitHub Artifact Attestations against real installers, SBOM verification for produced installers, and complete tagged CI execution.
- Windows Authenticode, Apple Developer ID signing/notarization, Linux installer acceptance, upgrade/recovery testing and end-to-end platform acceptance.

## Functional limits and operational cautions

- Research currently supports only the validated `BTCUSDT` 5m native candle cache. Other symbols/timeframes are rejected rather than silently relabeling data.
- The strategy implementation remains the existing SMA crossover. This release does not add a general strategy language, optimizer, broad alpha library, multi-market loader or autonomous LLM strategy team.
- The bootstrap output is a deterministic sensitivity distribution conditional on historical strategy returns. It is not a return forecast, confidence guarantee, live-risk model or liquidity simulation.
- Dataset SHA-256 and the local hash chain provide local integrity/tamper evidence, not publisher authentication, remote anchoring or protection from a privileged attacker able to rewrite the local database and process.
- The workspace caps immutable artifacts at 100 run cards and 1,024 audit events. The release does not include a rotate/archive workflow; operators must preserve a verified database backup and must not delete the only copy of research evidence. Archive/rotation support is an explicit operational gap before sustained high-volume use.
- Live brokerage and MT5 execution remain unimplemented and disabled. The app must continue to be operated as a research and paper-trading system only.

## Release decision

**Do not deploy this candidate as production-certified.** The delivered source contains a meaningful research-workflow expansion and the runnable local checks pass, but essential native/frontend builds, signed supply-chain evidence, lockfiles, immutable action pins and platform acceptance remain unverified or blocked. Production signoff requires the blockers above to be resolved, then a clean tagged CI build on Linux, Windows and macOS with native tests, strict formatting/Clippy, frontend/Tauri packaging, real integration tests, attestation verification and OS signing where applicable.
