# Local validation report — Agentic Trading OS 0.1.4

Date: 2026-10-27 (UTC)

This report distinguishes commands actually executed in the authoring container from compiler, dependency-resolution, and packaging checks that could not be run.

## Executed successfully

- `npm run verify:repo` — pass. This is a dependency-free static repository guardrail; it is not a compiler or full test runner.
- `npm run test:exports` — pass (5/5). Covers CSV format/escaping/formula neutralization and JSON report v2 preservation.
- `npm run test:provenance` — pass (2/2). An independent Node reference for the canonical OHLCV byte format matches the fixed SHA-256 vector and verifies that changes to each input field or row count change the fingerprint. This is a protocol/reference test, not execution of the Rust implementation.
- `node --check` — pass for `scripts/verify-repo.mjs`, `scripts/test-dataset-fingerprint.test.mjs`, and `src/lib/exports.js`.
- JSON parsing — pass for `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, and `tsconfig.json`.
- Version alignment — pass across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` at `0.1.4`.
- `.github/workflows/ci.yml` YAML parsing with PyYAML — pass.

## Blocked or not executed

- `npm run build` — blocked: `svelte-check` is not installed.
- `npm install --package-lock-only ...` — blocked because `registry.npmjs.org` did not resolve (`EAI_AGAIN`). No `package-lock.json` was generated.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` and `cargo fmt` — blocked because `cargo` is not installed. The Rust regression tests are added but were not executed here.
- Clippy, Tauri development runtime, desktop packaging, signed installers, and cross-platform acceptance — not executed in this environment.
- `src-tauri/Cargo.lock` remains absent, so Rust dependencies are not pinned.

## 0.1.4 changes

- Added a canonical SHA-256 fingerprint of the exact OHLCV dataset used by each backtest.
- Added encoding version, input start/end timestamps, and the median candle interval to backtest results.
- Added a provenance panel in the UI and upgraded the JSON report envelope to schema v2.
- Added Rust unit tests for a fixed SHA-256 digest vector, deterministic hashing, changes to all candle fields, and result provenance metadata.
- Added a dependency-free Node reference test for the byte-level fingerprint contract.

## Known limitations and release decision

**Not release-ready.** The fingerprint is a content identifier, not a signature or proof of provider authenticity. The v2 report does not embed raw candles and is not a full offline replay bundle. Both `package-lock.json` and `src-tauri/Cargo.lock` must be committed from a connected dependency-resolution environment. The Svelte compiler/build, Rust test/format/Clippy suite, and Linux/Windows/macOS Tauri packaging must pass before release. Live trading remains unimplemented and must stay disabled.
