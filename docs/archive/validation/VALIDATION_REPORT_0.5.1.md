# Validation report — Agentic Trading OS v0.5.1

## Change under test

Market-data response validation now checks every adjacent candle timestamp against the requested interval instead of comparing only the median delta. Internal missing candles and irregular deltas fail closed before the response reaches chart or indicator consumers. Regression cases cover a missing interval and a non-integral cadence gap.

## Results

- `npm test`: **PASS**, 90 tests passed, 0 failed across the repository's Node test groups.
- `npm run verify:repo`: **PASS**. This is a static repository contract check, not a build or native test.
- `npm run build`: **BLOCKED**, exit 127 because `svelte-check` is not installed in the environment.
- Native Rust compile/tests: **NOT RUN**; `cargo`, `rustc`, and `rustfmt` are not available in the environment.
- Reproducible dependency resolution: **BLOCKED**; `package-lock.json` and `src-tauri/Cargo.lock` are absent. npm lockfile generation was not completed in this environment.
- ZIP integrity and SHA-256: verified at packaging time.

## Production decision

**NOT PRODUCTION-READY / NOT RELEASE-CERTIFIED.** Do not promote this artifact to a production release until dependency lockfiles are generated and reviewed, CI performs a clean frontend install/build, native Rust tests and Clippy/format checks run, and platform packaging is verified.

## Scope limitations

Exact adjacent-candle cadence is appropriate only for configured continuous-market series. Markets with scheduled closures, holidays, or venue-specific breaks require a trusted session calendar and a session-aware expected-cadence policy. This validator does not prove provider authenticity.

The existing system constraints remain unchanged: catalog coverage is still 6 calculated, 1 workflow, and 357 catalog-only items; provider/agent dispatcher execution is not certified; live trading remains disabled.
