# Local validation report — QuantDinger AI Trading OS 0.1.8

Date: 2026-10-27 (UTC)

This report distinguishes checks run in the authoring environment from checks blocked by missing compiler toolchains or dependency lockfiles.

## Scope of this milestone

- Add typed Rust/frontend candle-series contracts and a `get_candle_series` Tauri command.
- Expose quote-symbol coverage separately from exact symbol/interval candle-series capabilities.
- Fail closed when a requested pair is unsupported; do not return BTCUSDT 5m data under another symbol or timeframe.
- Validate responses before rendering: exact request identity, source/status/freshness metadata, OHLCV fields and price relationships, integer/monotonic timestamps, at least 30 candles, median interval cadence, and recency when the provider claims a series is live.
- Prevent older async chart responses from overwriting newer requests.
- Show the current supported BTCUSDT 5m series and a warning when paper-order symbol differs from the plotted candle symbol.

## Executed successfully

- `npm test` — **27/27 dependency-free Node tests passed** across exports (5), dataset provenance (2), strict technical-analysis catalog/calculations (9), dashboard data models (4), and market-data request/response validation (7).
- `node --check scripts/verify-repo.mjs` — pass.
- `npm run verify:repo` — pass after the v0.1.8 report was added. This check is a static repository guardrail, not a compiler or test runner.
- JSON parsing — pass for `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `tsconfig.json`, and the strict technical-analysis master-list manifest.
- YAML parsing — pass for `.github/workflows/ci.yml` using PyYAML.
- Version alignment — `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` are aligned at `0.1.8`.
- Market-data adapter boundary reviewed: only BTCUSDT 5m is currently advertised as a candle series; supported quotes for ETHUSDT/SOLUSDT/BNBUSDT/DOGEUSDT do not imply candle-history support.
- Archive integrity — the final ZIP is tested with `unzip -t` and its SHA-256 is included separately.

## Blocked or not executed

- `npm run build` — blocked because `svelte-check` is not installed in this environment. `node_modules` is absent. A Svelte typecheck/Vite build result is therefore **not available**.
- `npm install --package-lock-only --ignore-scripts --no-audit --fund=false --fetch-retries=0 --fetch-timeout=10000` — blocked with `EAI_AGAIN` resolving `registry.npmjs.org`; no package lockfile was fabricated.
- `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` — attempted and blocked with `cargo: command not found`. `cargo fmt`, Clippy, and Rust compilation are likewise unavailable because the Rust toolchain (`cargo`/`rustc`) is not installed. Newly added Rust tests are present in source but **not executed here**.
- `package-lock.json` and `src-tauri/Cargo.lock` — not present. Dependency resolution is therefore not pinned and the tagged-release workflow intentionally blocks release builds until both lockfiles are committed.
- Native Tauri packaging and visual acceptance on Linux, Windows, and macOS — not run.

## Release decision

**Not release-ready.** The dependency-free model tests and repository guardrails pass, but they do not prove that the full Svelte/Tauri application compiles or that Rust tests pass. Before tagging a release, generate and review both lockfiles in a connected dependency-resolution environment, then pass frontend build, Rust tests/format/Clippy, and all three OS packaging jobs.

Live trading remains unimplemented and locked. The current candle provider supports only BTCUSDT 5m; the other listed quote symbols and additional intervals are not presented as available candle datasets. All orders remain paper-only and pass through the Rust risk gateway.
