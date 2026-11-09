# Agentic Trading OS v0.9.3 — Validation Report

**Status: release candidate, not production-certified.**

## Root-cause fix

The prior market-data/screener refresh guard enforced only a minimum interval between request starts. The market-data client has an 8-second request timeout while its throttle interval is 3 seconds; therefore, a second request could begin while the first remained in flight. A late first response could overwrite newer state.

v0.9.3 adds a separate `AtomicBool` in-flight flag for market-data and screener refreshes, and a `RefreshLease` RAII guard that remains alive across the async provider command. The guard rejects overlapping requests and releases on function return, early error return, or future cancellation. The existing per-provider rate limits remain active. The two provider classes have separate flags and do not block each other.

## Verification performed in this workspace

- `npm test`: **238 tests passed, 0 failed**, across 34 Node test invocations.
- `npm run typecheck:layers`: passed.
- `npm run verify:repo`: passed; dependency-lock warnings remain.
- `npm run eval:agents -- --report-file ...`: **10/10 scenarios passed**, no category gaps. The evaluation is deterministic and does not claim live provider integration.
- `node --check` across JavaScript/MJS/CJS source under `src`, `api`, and `scripts`: passed.
- Static source scan: 90 code files, 18,221 lines; no matches for the scanned TODO/FIXME, `eval`/`new Function`, explicit `.innerHTML =` sink, or credential-adjacent logging patterns.
- Release preflight: **blocked by 9 issues** (three missing dependency lockfiles, three missing frontend/Tauri executables, and three missing Rust toolchain commands).

## Tests added

Rust unit tests were added for:

- rejecting overlapping requests even after the rate-limit window elapsed;
- enforcing the minimum interval;
- releasing the in-flight lease after a throttled rejection;
- rejecting wall-clock rollback without leaving the lease locked.

A Node source-contract regression test verifies the native lease is wired into both provider commands and that these Rust tests exist.

## Important verification boundary

The environment has Node 22 and TypeScript `tsc`, but no Cargo, rustfmt, Bun, `node_modules`, `svelte-check`, Vite, or Tauri CLI. Therefore Rust compilation/tests/Clippy, Svelte build, Bun/Hono integration, real provider transport acceptance, OS credential-store acceptance, and desktop packaging were **not executed**. Source-contract tests do not substitute for compiling and running the native Rust tests.

Missing lockfiles:

- `package-lock.json`
- `bun.lock`
- `src-tauri/Cargo.lock`

These files were not fabricated. The release remains blocked until dependency resolution is generated and reviewed in the proper toolchains, and the full frontend/native release pipeline passes.

## Safety scope

No mock/demo runtime data, broker integration, live trading authority, or order intent was added. This remains research/paper-only; `executionAuthority = none`, `liveExecutionAuthorized = false`, and `orderIntent = null` remain mandatory invariants.
