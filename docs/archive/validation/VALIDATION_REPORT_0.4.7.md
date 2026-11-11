# Validation Report — v0.4.7

## Change objective

Harden the JavaScript research-only structure-break detector at its trust boundary. Previously, `detectStructureBreaks` accepted externally supplied pivot records without checking that their type and indices were valid, that the confirmation occurred strictly after the pivot and within the supplied candle series, that the pivot price matched its source candle, or that the same pivot identity was supplied more than once. Such records could create fabricated or non-causal research events.

## Implemented

- Validate pivot type, finite price, integer pivot/confirmation indices, bounds, and `confirmedAt > pivotIndex`.
- Verify high pivots match the source candle high and low pivots match the source candle low.
- Reject duplicate `(type, pivotIndex)` identities.
- Sort simultaneous confirmation batches deterministically by pivot index and type.
- Add regression tests for forged source prices, non-causal and out-of-range references, and duplicate pivot identities.

This boundary validation does not independently prove that a supplied pivot is a unique fractal swing; the caller must still use the confirmed swing detector and the detector remains research-only.

## Validation performed

- `node --test scripts/test-research-detectors.test.mjs`: **8 passed, 0 failed**.
- Full `npm test`: **88 passed, 0 failed** across all Node.js test files. These include static source-contract checks, not native Rust execution.
- `npm run verify:repo`: **passed**, with warnings that `package-lock.json` and `src-tauri/Cargo.lock` are absent.
- `npm run build`: **failed (exit 127)** because `svelte-check` is not installed.
- Native Rust compile/test/clippy: not run because `cargo` and `rustc` are unavailable in this environment.
- Dependency lockfiles remain a release requirement; do not treat this candidate as release-certified.

## Safety and release status

No technical-analysis catalog entries were promoted. Strict handoff rules remain unchanged and live trading remains disabled. This is an engineering candidate, not a production-certified release.
