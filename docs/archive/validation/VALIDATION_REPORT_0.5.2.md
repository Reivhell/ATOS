# Validation report — Agentic Trading OS v0.5.2

## Change under test

The market-data validator previously checked a bridge object but returned the same mutable object; the UI ignored the validator's return and kept the unchecked reference. v0.5.2 closes that in-process TOCTOU gap by returning a defensive, recursively frozen snapshot containing normalized request identity, response metadata, and copied OHLCV values. The UI now uses that snapshot. A regression test mutates the input after validation and proves the accepted snapshot remains unchanged.

## Results

- `npm run test:market-data`: **PASS**, 8 tests, 0 failures.
- `npm test`: **PASS**, 91 tests, 0 failures.
- `npm run verify:repo`: **PASS**. This is a static contract check, not a frontend or native build.
- `npm run build`: **BLOCKED**, exit 127 because `svelte-check` is absent (`svelte-check: not found`).
- Native Rust compile/tests/Clippy: not run locally because the environment has no `cargo`, `rustc`, or `rustfmt`; CI must pass these checks.
- `npm run release:preflight`: **BLOCKED as expected**, reporting missing npm/Cargo lockfiles, missing installed frontend build executables, and unavailable Rust toolchain commands. It now also runs as a fail-closed check in tagged-release CI.
- Reproducible dependency resolution: blocked locally; `package-lock.json` and `src-tauri/Cargo.lock` are absent, and `npm install --package-lock-only --ignore-scripts --no-audit --fund=false` did not complete in the available environment.

## Production decision

**NOT PRODUCTION-READY / NOT RELEASE-CERTIFIED.** This change materially hardens the frontend data boundary, but it does not authenticate provider data and does not replace native validation. Do not release until dependency lockfiles are committed, a clean locked install and Svelte build pass, native Rust tests/format/Clippy pass, and platform packaging plus desktop acceptance are verified.

## Existing limitations

- Technical-analysis inventory remains 6 calculated, 1 workflow, and 357 catalog-only entries; catalog-only entries continue to block strict downstream handoff.
- No configured LLM/provider agent-dispatch adapter is certified; generated work plans are not evidence of execution.
- Live order execution remains disabled.
- Candle continuity currently models continuous-market crypto series only; session-aware markets need a trusted trading calendar.
