# Validation report — Agentic Trading OS 0.4.5

## Scope

This candidate build focuses on two defects in the Rust research-only market-structure engine:

1. **Outside-bar identity correctness:** a candle can be both a strict swing high and a strict swing low. The prior consumed-pivot set keyed only by candle index, coupling these two distinct levels. The set now keys by `(pivot_index, swing_kind)` and includes a Rust regression fixture that breaks the high first and the low later.
2. **Timeline assembly complexity:** timeline generation previously rescanned all swings and all break events for each candle and rescanned confirmed swings when selecting protected levels. It now indexes swing confirmations and break events once, and tracks the latest previously confirmed swing of each kind. Snapshot assembly is linear in candles plus detected events, after detector results are available.

The structure-break detector itself still scans previously detected swing levels across candles; worst-case performance of that detector has not been optimized or benchmarked in this build. Do not infer whole-pipeline linear complexity.

## Checks performed

- `npm test`: passed; 84 tests passed, 0 failed (Node/source-contract suite; not native Rust execution).
- `npm run verify:repo`: passed with warnings for missing `package-lock.json` and `Cargo.lock`; both remain release blockers.
- `npm run build`: failed because `svelte-check` is not installed (`exit 127`).
- `cargo` and `rustc` are not installed, so native Rust checks could not run.
- ZIP integrity and SHA-256 are checked after package generation.

## Checks not available in this environment

- `cargo fmt --check`
- `cargo test --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- Native Tauri build and compiled SQLite/audit concurrency tests
- Frontend production build (`npm run build`) if `svelte-check` is absent

These are release gates, not optional claims. Source-level Node contract tests do not substitute for compiling and executing Rust tests. Dependency lockfiles are still required for reproducible release builds.

## Release decision

**Not production-certified.** This is an engineering candidate with targeted correctness and complexity improvements. The strict catalog remains 364 concepts across 9 categories: 6 calculated, 1 workflow, and 357 catalog-only. Specialist-provider dispatch is not configured, all-concept completion is not proven, strict handoff remains fail-closed, and live trading remains disabled.
