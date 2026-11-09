# Validation report — Agentic Trading OS v0.5.0

Date: 2026-10-27  
Status: **engineering candidate; not production-certified**

## Substantive change

Added a native Rust causal prefix-invariance regression covering five research detector outputs: confirmed swings, FVG formation, FVG lifecycle observations, liquidity sweeps, and close-confirmed structure breaks. A deterministic generated 96-candle series is evaluated at every prefix; events visible by that prefix must exactly match the same subset of full-series events. This expands anti-lookahead verification from individual confirmation-index checks to an end-to-end detector-family invariant.

The Node source-contract test asserts that the native regression remains present, but it does not execute the Rust test.

## Verification executed

| Check | Result | Boundary |
|---|---|---|
| `npm test` | **PASS — 90 passed, 0 failed** | Includes Node tests and Rust source-contract tests; does not compile or execute Rust. |
| `npm run verify:repo` | **PASS with 2 release-blocking warnings** | Static check passes; warns that both dependency lockfiles are absent. |
| Native prefix-invariance Rust test | **Not run** | `cargo` and `rustc` are unavailable in this environment. |
| `npm run build` | **FAIL — exit 127** | `svelte-check` is not installed; frontend typecheck/bundle did not run. |
| Lockfile reproducibility | **Blocked** | `package-lock.json` and `src-tauri/Cargo.lock` are absent. |

## Remaining release blockers

- Run `cargo fmt --all -- --check`, `cargo test --all-targets`, and `cargo clippy --all-targets --all-features -- -D warnings` in native CI.
- Commit and verify `package-lock.json` and `src-tauri/Cargo.lock`; use lockfile-enforcing installs for release builds.
- Run frontend typecheck and production bundling with installed dependencies, followed by Tauri desktop builds on supported operating systems.
- Configure and test a real specialist provider/dispatcher before claiming the nine specialist agents execute; the current repository still has no configured runtime LLM dispatch.
- The catalog retains 6 `Calculated`, 1 `Workflow`, and 357 `Catalog only` entries. Research outputs must not promote unsupported entries.
- Live order execution remains disabled.

## Release decision

**Do not label v0.5.0 production-ready or release-certified yet.** The dependency-free Node suite passes and the causal native regression has been authored, but its execution is unverified until Rust CI runs it. Frontend build, dependency lockfiles, native quality gates, and runtime acceptance remain mandatory.
