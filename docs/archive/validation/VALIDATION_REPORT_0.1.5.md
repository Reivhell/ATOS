# Local validation report — Agentic Trading OS 0.1.5

Date: 2026-10-27 (UTC)

This report distinguishes checks executed in the authoring container from compiler/build validation that requires external toolchains or dependencies.

## Scope of this revision

- Added a searchable/filterable technical-analysis catalog organized into nine top-level categories.
- Added current coverage labels so indexed concepts are not presented as implemented detectors.
- Added six OHLCV indicator readings: SMA 9/21, EMA 20, RSI 14, MACD 12/26/9, ATR 14, and Bollinger Bands 20/2σ.
- Added Node regression tests for catalog coverage, unique IDs, availability boundaries, baseline indicator outputs, insufficient history, and malformed-candle fail-closed behavior.
- Kept live trading disabled; this page does not generate orders.

## Executed successfully

- `npm run verify:repo` — pass. Static repository guardrails only; not a compiler.
- `npm run test:exports` — 5/5 pass.
- `npm run test:provenance` — 2/2 pass.
- `npm run test:technical-analysis` — 7/7 pass.
- `node --check` for the new JS module and test scripts — pass.
- Standalone strict TypeScript checking of `src/lib/technical-analysis.js` — pass.
- Version alignment across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` — pass at `0.1.5`.
- JSON parsing for app and TypeScript configuration plus CI YAML parsing — pass.
- Catalog has 364 entries across 9 categories; 6 calculated families + the existing backtesting workflow are marked available, while 357 entries remain catalog-only.

## Blocked in this environment

- `npm run build` — blocked: `svelte-check` is not installed (`command not found`).
- `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` — blocked: `cargo` is not installed.
- `cargo fmt` and Clippy — blocked because Rust toolchain is unavailable.
- Lockfile creation and dependency resolution cannot be verified in this environment; the required `package-lock.json` and `src-tauri/Cargo.lock` are still absent.

## Environment limitations

- The full Svelte typecheck/build requires dependencies such as `svelte-check`; if not installed, it must be run in a connected Node dependency-resolution environment.
- Rust formatting/tests/Clippy require the Rust toolchain and Tauri system prerequisites.
- `package-lock.json` and `src-tauri/Cargo.lock` must be generated and committed in a connected, reproducible dependency-resolution environment before tagged-release packaging.
- No cross-platform installer test is implied by the static repository checks.

## Release decision

**Not release-ready until compiler/build tests, lockfile generation, Rust tests/Clippy, and cross-platform packaging pass.** Catalog presence is not implementation of all listed methods, and current indicator readings are research aids, not validated trade signals. Live execution remains unimplemented and must not be enabled as part of this revision.
