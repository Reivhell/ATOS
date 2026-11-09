# Validation Report — v0.4.8

Date: 2026-10-27

## Change under test

The Rust research-only FVG detector now rejects derived zone widths that are non-finite or non-positive, even when each source OHLC value is finite. The midpoint uses `lower / 2.0 + upper / 2.0` to avoid the intermediate `upper - lower` overflow. The lifecycle input validator independently rejects a non-representable expected width or midpoint. A native Rust regression test uses finite extreme prices whose difference overflows `f64`.

The release workspace also removes an accidentally nested copy of the prior source tree to avoid shipping duplicate source.

## Checks actually executed

- `npm test`: **88 passed, 0 failed** across 14 Node.js test groups. This includes static source-contract tests; it does not compile or execute Rust.
- `npm run verify:repo`: passed with release-blocking warnings for missing `package-lock.json` and `src-tauri/Cargo.lock`.
- `npm run build`: failed with exit code 127 because `svelte-check` is not installed in this environment.
- `cargo test --all-targets`, `cargo fmt --check`, and `cargo clippy`: not run because `cargo`/`rustc` are unavailable.
- ZIP integrity and SHA-256 are checked after packaging.

## Release decision

**NOT PRODUCTION-READY / NOT RELEASE-CERTIFIED.** Native compilation and native regression execution are unverified; the frontend build is blocked by missing dependencies; dependency resolution is not locked. The new native test is present in source but was not executed here.

The SMC detector remains research-only. The 364-item catalog status is unchanged (6 calculated, 1 workflow, 357 catalog-only). Specialist provider dispatch is not proven end-to-end. Live order execution remains disabled.

## Required release gates

1. Install dependencies from a committed lockfile in a clean CI environment.
2. Run `npm ci`, `npm test`, `npm run build`, and repository checks.
3. Generate and commit `Cargo.lock`, then run `cargo fmt --check`, `cargo test --all-targets`, and `cargo clippy --all-targets -- -D warnings` on supported CI targets.
4. Add native fuzz/property tests for OHLCV and derived geometry, including extreme finite values and event ordering.
5. Verify the full Tauri packaging pipeline and an independent audit of the strict decision and durable audit paths.
