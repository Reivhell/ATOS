# ATOS — Production Engineering Continuation: MT5 Risk Aggregation Hardening

**Date:** 2026-10-31  
**Application version:** 0.11.0 (unchanged)  
**MT5 bridge protocol:** v6  
**Release disposition:** BLOCKED — no production certification issued.

## Engineering objective

Close two arithmetic and trust-boundary weaknesses in the MT5 account-wide risk control path. The work changes the invariants at the domain boundary rather than adding a user-facing demo or increasing the version number.

## Changes delivered

### 1. Account-risk aggregation now owns snapshot validation

`account_risk_inventory()` now invokes `validate_snapshot()` before reading or aggregating any account data. The function no longer relies on each caller having passed through the authenticated bridge ingestion path first. Invalid account or terminal metadata, malformed broker contract data, duplicate/invalid positions or orders, invalid pending-order states, malformed volume/price values, inconsistent protective-stop state, and malformed stop-risk payloads are rejected at the aggregation boundary.

Each `CALCULATED` stop-risk contribution is additionally required to be finite and strictly positive. The aggregate is updated item-by-item and fails immediately when an intermediate total becomes non-finite. Non-positive account equity cannot produce a risk percentage. This is fail-closed behavior for direct callers as well as the normal bridge path.

### 2. Directional exposure aggregation is checked and fallible

The previous open-position and active-pending-order reducers used floating-point `.sum()` and returned an unqualified number. They now return `Result<f64, String>` and share a checked accumulator that rejects negative/non-finite inputs and intermediate overflow. The position and pending-order totals are combined through that same checked path.

The sizing engine performs the aggregation both before native terminal preflight and again against the exact correlated snapshot after preflight. The second pass is not allowed to bypass arithmetic validation. This also prevents a non-finite directional inventory diagnostic from reaching the serialized sizing response when the broker reports no directional cap.

### 3. Regression fixtures and contract tests

Rust unit tests were added for direct-boundary snapshot validation, negative risk amounts, unknown risk states, zero equity, overflowing account stop-risk, non-finite directional inputs and overflowed position totals. Existing calculated-position fixtures were corrected to put BUY stops strictly below the current mark; after the boundary began enforcing the complete snapshot contract, the old fixtures would otherwise fail for an unrelated stop-direction error and would not genuinely test aggregation.

Node contract tests now assert that the Rust aggregation boundary validates snapshots and that both directional reducers use the checked accumulator. These tests verify source contracts; they do not execute the Rust unit tests.

## Changed files

- `src-tauri/src/infrastructure/mt5/bridge.rs`
- `src-tauri/src/application/mt5_risk_sizing.rs`
- `scripts/test-mt5-account-risk-control-plane.test.mjs`
- `scripts/test-mt5-pending-order-reconciliation.test.mjs`
- `docs/MT5_BROKER_NATIVE_RISK_SIZING_ENGINE_2026-10-31.md`
- `docs/MT5_PENDING_ORDER_EXPOSURE_RECONCILIATION_2026-10-31.md`

The application version and bridge protocol were not changed. The bridge remains read-only; no order submission, amendment, close, cancellation or reservation capability was introduced.

## Verification

| Gate | Result | Scope / limitation |
|---|---|---|
| `npm test` | **PASS — 375/375** | 51 Node test groups, zero failures. Includes the new source-contract checks. |
| `npm run test:mt5-account-risk-control-plane` | **PASS — 10/10** | Account-wide MT5 contract and boundary checks. |
| `npm run test:mt5-pending-orders` | **PASS — 4/4** | Pending-order and directional-volume contract checks. |
| `npm run typecheck:layers` | **PASS** | Strict configured frontend/API layers. |
| All `src/**/*.js` strict checkJs | **PASS — 31 files** | Standalone strict TypeScript check of frontend JavaScript modules. |
| JavaScript syntax scan | **PASS — 91 files** | `node --check` over JS/MJS/CJS files in `src`, `api`, and `scripts`. |
| JSON parse scan | **PASS — 18 files** | Repository JSON files outside generated/vendor directories. |
| `npm run verify:repo` | **PASS with warnings** | Static guard passed; absent lockfiles remain explicit release blockers. |
| `npm run build` | **BLOCKED** | `svelte-check` is not installed; Vite did not run. |
| `npm run release:preflight` | **BLOCKED — 10 issues** | Missing lockfiles, frontend/Tauri build executables, Bun and Rust toolchain commands. |
| Rust tests/build/format/Clippy | **NOT RUN** | Cargo, rustc, rustfmt and Clippy are unavailable. New Rust unit tests are present but unexecuted. |
| MetaEditor EA compile and broker acceptance | **NOT RUN** | No MetaEditor or broker-connected MT5 acceptance environment is available. |

## Remaining production blockers

1. Generate authentic `package-lock.json`, `bun.lock` and `src-tauri/Cargo.lock` using the declared package managers and controlled dependency resolution. No lockfile was fabricated.
2. Install exact dependencies and run immutable installs, whole-project Svelte/TypeScript checking, Vite production build and Tauri packaging.
3. Run Cargo compilation, Rust unit tests, rustfmt and Clippy using the pinned Rust toolchain.
4. Compile `MQL5/Experts/ATOS_Bridge.mq5` with MetaEditor and retain compiler output and binary hash.
5. Perform MT5 terminal fault-injection and broker acceptance for account switching, disconnect/reconnect, inventory changes, partial fills, stale/reordered frames, symbol contract revisions and native margin/profit calculation failures.
6. Produce signed/CI-attested provenance, SBOM, dependency/license evidence and a complete release evidence record.

## Certification decision

**NOT PRODUCTION-CERTIFIED.** The source-level and Node test gates pass, but the changed Rust implementation has not been compiled or executed in this environment. The frontend build, dependency reproducibility, native packaging, EA compilation and real broker acceptance also remain unverified. Release and deployment must stay blocked until those gates pass with evidence.
