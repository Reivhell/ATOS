# Validation report — Agentic Trading OS v0.5.3

## Change under test

The native Rust backtest previously selected the median adjacent-candle timestamp delta. That can hide missing bars and distort the cadence used to annualize risk statistics. The backtest now requires exact uniform cadence across every adjacent candle, uses checked timestamp subtraction, and fails closed with a row-specific diagnostic when a gap or irregular delta is found. Native regression tests cover missing intervals, non-integral cadence, and timestamp-delta overflow. A Node contract test guards this policy and the native test presence.

## Results

- `npm test`: **PASS**, 94 tests, 0 failures.
- `npm run verify:repo`: **PASS**. This is a static contract check only; it warns that both dependency lockfiles are absent.
- `npm run build`: **BLOCKED**, `svelte-check: not found` because frontend dependencies are not installed.
- Native Rust tests, formatting, and Clippy: **NOT RUN**; `cargo`, `rustc`, and `rustfmt` are unavailable in this environment.
- `npm run release:preflight`: **BLOCKED as expected**, due to missing `package-lock.json`, `src-tauri/Cargo.lock`, installed frontend executables, and Rust toolchain.
- ZIP archive integrity and SHA-256 are checked during packaging; these do not certify application runtime behavior.

## Production decision

**NOT PRODUCTION-READY / NOT RELEASE-CERTIFIED.** The native cadence change is implemented and covered by authored Rust regression tests, but those tests have not been compiled or executed here. Do not certify or ship until CI performs a clean locked dependency install, frontend check/build, `cargo fmt --check`, `cargo test --locked`, `cargo clippy --locked`, Tauri packaging, and desktop acceptance on supported target platforms.

## Important scope constraints

- Exact uniform cadence currently targets continuous-market series. Session-based instruments require a trusted venue/session calendar; do not use this strict policy on scheduled closures without that calendar.
- The existing `medianCandleIntervalMs` response field is retained for compatibility; for accepted data it now holds the verified uniform interval.
- Technical-analysis inventory remains 6 calculated, 1 workflow, and 357 catalog-only entries. Catalog-only concepts still block strict downstream handoff.
- No configured LLM/provider dispatch adapter is certified; a created work plan is not evidence that agents executed.
- Live order execution remains disabled.
