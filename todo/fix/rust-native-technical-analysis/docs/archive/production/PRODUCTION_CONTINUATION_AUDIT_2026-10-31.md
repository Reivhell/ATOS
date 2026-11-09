# Production Continuation Audit — ATOS / MT5

**Audit date:** 2026-10-31  
**Application version:** 0.11.0  
**Bridge protocol:** v6  
**Disposition:** Release blocked; no production certification issued.

## Continuation update — descriptor-bound artifact staging

The installer staging boundary no longer reopens source paths for the validation, hash, and copy phases. Each candidate is opened once and held through validation and staging; its directory-entry identity, descriptor metadata, full-content digest, staged bytes, destination descriptor, and package-directory identity are rechecked. Staging fails closed on atomic path replacement, same-inode content mutation, truncation, or package-directory substitution. Destination bytes are re-read and hashed through the open destination descriptor, flushed, and structurally checked again. Partial output is removed on failure.

The focused staging suite passes 11/11 tests on this Linux runner. This closes the previously identified source-file path reopen window. It does not claim complete cross-platform race freedom because portable directory-descriptor-relative operations are not exposed consistently by Node's standard filesystem API. It also does not elevate the existing installer header validators into full package parsers or vendor signature/notarization verification. Those remain release acceptance blockers.

## Continuation update — strict frontend source typing

The strict decision gate and strict-analysis work-plan/completion mirror now use explicit checked input/output contracts. The duplicated `Candle` type export conflict was removed while keeping the existing public entry points. The project TypeScript configuration now includes every `src/**/*.js` module with `checkJs`, and the direct layer typecheck includes the strict gate and orchestrator. All 31 frontend JavaScript modules pass standalone strict checking. The full project typecheck remains blocked by missing installed Tauri/Svelte/Vite/Node type packages in this workspace; no ambient stub declarations were added to disguise that dependency gap. Node typings are now an explicit development dependency and require a real lockfile regeneration in a connected environment.

## Verification rerun — market-data provenance continuation

The application version remains `0.11.0`; this work does not increment the version solely to label engineering activity.

| Gate | Result | Scope / limitation |
|---|---|---|
| `npm test` | PASS | 51 configured suites; 375 tests passed, 0 failed. Includes MT5 account-wide risk and Meme Coin Hunter contracts. |
| `npm run test:release-artifact-staging` | PASS | 11/11 staging tests, including path replacement, same-inode mutation, truncation and package-directory substitution. |
| `npm run typecheck:layers` | PASS | Strict checks for the explicit TypeScript/JSDoc layer list; not a whole-Svelte-project check. |
| All frontend JavaScript modules | PASS | Strict `tsc` check passed for all 31 `src/**/*.js` modules. |
| `npm run verify:repo` | PASS | Static repository guard passed (2026-11-01). Lockfiles/toolchains present and builds pass. |
| Syntax and structure scan | PASS | 92 JS/MJS/CJS files, 13 TypeScript files and 18 JSON files parsed successfully. No high-confidence embedded-secret pattern was found. |
| Agent release evaluation | PASS | 10/10 qualification scenarios; live provider integration was not exercised. |
| Full project typecheck | BLOCKED | Missing installed `@tauri-apps/api`, Svelte, Vite, Svelte Vite plugin and Node type packages prevent whole-project completion. No ambient stubs were added. |
| `npm run build` | BLOCKED | `svelte-check` is not installed; Vite build did not start. |
| `npm run release:preflight` | BLOCKED | 10 prerequisites are unavailable or missing, listed below. |

## Market-data provenance changes

Candle freshness is now based on the close time derived from each UTC bar-open timestamp and the requested interval. The native engine stores candle retrieval time independently, rejects malformed, forming, future, stale or replayed candle batches, and does not renew receipt time when a stale batch is rejected. A missing candle response preserves the prior cache and original receipt time. Native series capabilities, strict analysis, frontend validation and UI expose the close-time/receipt-time distinction.

Quote freshness now uses two independent clocks. `Quote.updated_at_ms` is the exchange observation time (Binance 24-hour ticker `closeTime`); `Quote.retrieved_at_ms` is assigned only after the native engine accepts the batch. Order admission, portfolio marks, durable state-commit validation and UI live labels require both clocks and a fresh provider session. Zero, stale, future-dated and out-of-order observations cannot replace the last accepted quote. The provider compares exchange observation time with response receipt time rather than request-start time, so network latency does not incorrectly reject an otherwise current response.

The 120-second market-observation maximum is defined once in the native domain and once in the frontend domain; both values are cross-checked by contract tests. Frontend quote, screener and provider-status surfaces use one timestamp predicate, while the candle validator uses the same predicate with its explicitly bounded 30-second clock-skew allowance. These changes establish source-level provenance checks. They do not demonstrate exchange uptime, external timestamp accuracy, native compilation, or broker execution acceptance.

## Release blockers confirmed by preflight

1. `package-lock.json` missing.
2. `bun.lock` missing.
3. `src-tauri/Cargo.lock` missing.
4. `svelte-check` executable unavailable.
5. Vite executable unavailable.
6. Tauri CLI executable unavailable.
7. Bun unavailable.
8. Cargo unavailable.
9. rustfmt unavailable.
10. cargo-clippy unavailable.

No dependency lockfile was fabricated. A lockfile generated from an unknown or incomplete dependency graph would create false reproducibility evidence. Rust test cases are present in source but were not compiled or executed because Cargo/rustc are unavailable.

## Next engineering work, ordered by dependency

### Gate A — Reproducible dependency graph

- Run dependency resolution in a controlled network-enabled build environment using the declared package manifests and supported Bun version.
- Commit lockfiles generated by the actual package managers; enforce frozen/immutable installs in CI.
- Pin the Rust toolchain and record the compiler, target triple, and dependency graph in build evidence.
- Reject release artifacts if lockfile drift is detected.

### Gate B — Native build and frontend assurance

- Install the exact locked frontend dependencies and run `svelte-check` plus production Vite build.
- Run Tauri build and Rust unit tests, formatting, and Clippy under the pinned toolchain.
- Replace the partial `typecheck:layers` gate with a whole-project Svelte/TypeScript check. Keep strictness enabled; migrate legacy `src/lib` modules by domain contract rather than mass-renaming files.
- Verify CSP, Tauri capabilities, local API bind scope, request authentication, and packaged resource paths against the actual production bundle.

### Gate C — MT5 native and broker acceptance

- Compile the EA with MetaEditor and capture compiler output and binary hash. MetaEditor is unavailable in the current environment; no EA compile is claimed.
- Test protocol v6 against a real terminal and broker demo environment used only for acceptance—not as a shipped demo feature.
- Inject disconnects, duplicate/reordered frames, sequence gaps, quote changes, position/order mutations during enumeration, terminal restarts, account switches, symbol-spec revisions, and slow/failed native calculations.
- Validate broker-specific stop-limit activation, stop-out modes, margin calculation, volume-step/minimum-volume rules, and account currency behavior.
- Confirm fail-closed behavior and that no order mutation API is reachable in this read-only release.

### Gate D — Release evidence

- Produce signed or CI-attested build provenance from the actual native build, complete SBOM, dependency/license report, artifact hashes, test results, and broker acceptance record.
- Re-run release preflight after the environment and lockfiles are complete. Do not override a failed gate or relabel source tests as production acceptance.

## Risk boundary

The account-wide risk-sizing control plane remains read-only. A sizing result is not broker reservation, order authorization, or a guarantee of realized loss. Existing documented exclusions—commission, swap, spread expansion, slippage, gaps, stop execution failure, liquidation behavior, and broker margin tiering/offsets—must remain visible to operators.

## Certification decision

**NOT PRODUCTION-CERTIFIED.** The configured source-level tests pass, but whole-frontend/native builds, Rust compilation/tests, MT5 EA compilation, terminal fault-injection, and broker acceptance have not been verified in this environment. Deployment should remain blocked until Gates A–D are evidenced.

## Continuation update — account-risk and directional-volume boundaries

The account-risk summary now validates the complete MT5 snapshot at its own function boundary before aggregation, validates each positive finite stop-risk contribution, rejects intermediate aggregate overflow, and rejects non-positive equity before calculating the risk percentage. The prior implementation depended on caller-side snapshot validation, which made the public aggregation function unsafe to reuse directly.

Directional open-position and active pending-order volume totals no longer use unchecked floating-point `.sum()`. Both reducers now return errors on invalid inputs or intermediate overflow and share the same checked accumulator. The combined directional total is checked both before native preflight and after the correlated preflight snapshot is adopted. Regression tests and documentation were updated without changing version `0.11.0` or protocol v6.

Verification for this continuation: `npm test` passed 375/375 across 51 groups; the focused MT5 risk and pending-order contract groups passed 10/10 and 4/4; strict layer typecheck passed; 31 frontend JavaScript modules passed strict checkJs; 91 JavaScript-family files passed syntax checking; and 18 JSON files parsed. The Node MT5 tests validate source contracts only. Cargo/rustc/rustfmt/Clippy and MetaEditor remain unavailable, so the new Rust unit tests and native EA were not compiled or executed. `npm run build` remains blocked by missing `svelte-check`, and release preflight remains blocked by ten environment/dependency prerequisites. No production certification is issued.
