# Validation report — Agentic Trading OS v0.4.3

Date: 2026-10-27

## Change under test

The Rust research-only `detect_confirmed_structure_breaks` function now resolves all eligible structure levels crossed by a candle as a batch. Event classifications share one pre-candle bias snapshot and directional bias updates once after processing the batch. This removes event-order-dependent labels for multi-level same-candle breaks. A Rust unit test covers two confirmed swing highs crossed by one later candle, and a Node source-contract test checks the implementation contract.

## Checks performed in this environment

- `npm test`: **82 passed, 0 failed**. This is a combination of dependency-free JavaScript tests and source-contract tests. It does not compile or execute Rust unit tests.
- `npm run verify:repo`: passed with warnings for the missing dependency lockfiles. The guardrail is static and does not prove native compilation.
- `npm run build`: failed with exit 127 because `svelte-check` is not installed in the authoring environment.
- Rust toolchain: `cargo` and `rustc` are unavailable in this environment. `cargo fmt`, `cargo test --all-targets`, and `cargo clippy` remain unexecuted and are mandatory in native CI.
- Dependency reproducibility: `package-lock.json` and `src-tauri/Cargo.lock` are not present. Do not tag a release until both are generated and committed by a network-enabled, toolchain-enabled environment.

## Unchanged strict controls

- Master list remains 364 contextual entries across nine top-level categories.
- Availability remains 6 calculated, 1 workflow available, and 357 catalog only.
- Specialist provider dispatch is unconfigured; absent genuine specialist receipts block handoff.
- Live trading execution remains disabled.

## Release decision

**Not production-ready / not release-certified.** The source-level correctness change is implemented and its regression test is authored, but native Rust test execution, frontend build, pinned dependencies, and the real specialist runtime remain outstanding. The master-list gate must remain fail-closed.
