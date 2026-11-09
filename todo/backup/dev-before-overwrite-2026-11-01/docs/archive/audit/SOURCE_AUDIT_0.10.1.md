# Source audit — Agentic Trading OS v0.10.1

**Audit date:** 2026-10-30  
**Baseline:** Agentic Trading OS v0.10.0 research-workbench candidate  
**Target:** v0.10.1

## Material changes

- Added an expanding-window nested walk-forward SMA parameter tournament in `src-tauri/src/backtest.rs`.
- Added bounded candidate generation, minimum training-trade eligibility, deterministic Sharpe/drawdown/lexicographic tie-breaking, fixed chronological OOS blocks, candidate/fold/work caps, dataset fingerprinting, and aggregate fold statistics.
- Added `TradingCore::run_research_backtest_bundle` in `src-tauri/src/engine.rs`; the primary backtest and all tournament folds consume one immutable candle/provider snapshot. The engine mutex is released before CPU-heavy evaluation, and the path does not mutate account state.
- Extended `src-tauri/src/research_workspace.rs` with a tournament evidence contract, a nested DAG node, committee gate v2, and independent semantic reconstruction of candidate eligibility, canonical grid, expanding-window chronology, selected winner and aggregate metrics before immutable persistence. The run-card verifier now enforces the canonical DAG node order, exact dependency edges, timestamps, bounded detail, node status vocabulary, and consistency with the stored specialist/committee decisions.
- Updated `src/lib/types.ts` and `src/App.svelte` to expose an opt-in tournament control and render candidate/fold evidence returned by the native command.
- Added Rust native regression tests for determinism, fold bounds, invalid fold policy and invariance of first-fold training selection when future candles change.
- Added Node source-contract coverage for the native/bridge/UI contract and version alignment.
- Aligned application version metadata and user-agent strings to 0.10.1.
- Replaced all eight mutable third-party GitHub Action refs with upstream-reviewed full commit SHAs; moved checkout/setup-node/upload-artifact to current Node 24-compatible action lines.
- Added `rust-toolchain.toml` pinning Rust 1.99.1 with rustfmt and Clippy; CI explicitly installs the same compiler instead of tracking moving `stable`.
- Changed the regular Bun and frontend CI jobs to frozen lockfile installs (`bun install --frozen-lockfile`, `npm ci`), preventing dependency drift in non-release CI runs.

## Security and correctness boundaries

1. Strategy candidate selection and backtests run in Rust; the model does not generate or execute strategy code.
2. Candidate selection is performed inside each fold's expanding training prefix. The next test block is passed to the backtest only after selection has been made.
3. A candidate requires at least three training trades. An unqualified fold has no selected parameters or OOS metrics and contributes a failure through the requested-fold denominator.
4. Work is estimated and bounded before running; candidate count and fold count have hard limits.
5. The tournament consumes validated native BTCUSDT 5m candles only; it does not relabel or substitute unsupported market data.
6. The engine method only reads native candle state. Research results do not write to the account ledger, create order intent, change risk controls, or confer execution authority.
7. Run-card validation recomputes the stored fold summary and verifies selected parameters against the recorded deterministic training scores before persistence. The full workflow graph is checked against a canonical edge/status contract, not merely accepted because it is any acyclic graph.
8. Historical fold evidence remains subject to backtest limitations: no exchange order book, dynamic spread, market impact, queueing, latency or continuous position carry.

## Audit findings and release blockers

The source-level changes are guarded by 269/269 passing Node tests across 41 command groups, strict TypeScript layer typecheck, an 85-file JavaScript syntax scan, repository guardrails, and a parsed five-job workflow YAML file. This environment does not contain Cargo/rustc/rustfmt/Clippy or the frontend build dependencies, so the newly added Rust code and Svelte component have not received their native compiler checks. The GitHub Actions immutable-reference guard now passes. Release preflight remains fail-closed on 9 local prerequisites: three missing lockfiles, three missing frontend/Tauri executables, and three missing Rust toolchain commands. The compiler/action versions are now pinned, but this workspace cannot execute those tools.

No dependency lockfile was fabricated. All eight action SHAs were checked against upstream release/commit pages and pinned in CI. v0.10.1 is a source candidate, not a production-certified release. See `VALIDATION_REPORT_0.10.1.md`.
