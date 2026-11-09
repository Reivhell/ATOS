# Engineering Update — Account-Wide Risk Inventory Integrity

**Date:** 2026-10-31  
**Application version:** 0.10.1 (unchanged)  
**MT5 bridge protocol:** v6  
**Release certification:** BLOCKED — native toolchain, dependency locks, and broker acceptance evidence are unavailable in this environment.

## Scope delivered

This increment hardens the existing MT5 account-wide risk-sizing control plane and removes JavaScript implementation files from the Bun/Hono API boundary. It does not add live order execution, order mutation, or broker-side risk reservation.

### MT5 stop-loss integrity

- A stop is counted as protected only when it is strictly on the loss side of the current position mark or pending-order executable entry reference.
- Zero, equal-price, crossed, and profitable-side stop outcomes are not accepted as verified protection. A malformed or zero-loss native result is unresolved and blocks new exposure.
- Rust independently validates direction and positive finite risk amounts; the EA does not coerce non-loss outcomes to zero and label them calculated.
- Stop-limit orders must carry a positive broker-reported stop-limit price before their exposure can be accepted as valued.

### Snapshot and preflight integrity

- The EA captures one `MqlTick` for a telemetry snapshot and reuses that exact quote for candidate `OrderCalcMargin` and `OrderCalcProfit`. It does not fetch a second quote for preflight.
- The Rust bridge assigns the native preflight result the authenticated telemetry frame sequence that carried it. The sequence is Rust-owned and is not accepted from an EA-provided field.
- Risk sizing requires the preflight sequence to equal the latest accepted bridge sequence and checks quote timestamp/bid/ask, broker stop-out policy, account identity, equity, used margin, and free margin against that same snapshot.
- The EA fences normal snapshot creation by re-reading the entire ticket set and all risk-affecting fields after enumeration: position symbol/type/volume/current mark/SL and pending-order symbol/type/state/initial and remaining volume/open and stop-limit prices/SL. It drops the frame if any field changes.
- A frame carrying a preflight result is fenced again after market data/session collection and immediately before transmission. A detected race drops the frame; the waiting Rust request times out closed rather than receiving a stale approval.
- The Svelte panel keeps completed output as historical evidence but hides its volume from the active-approval field once the bridge disconnects, telemetry becomes stale, or the accepted sequence changes.

### Bun/Hono TypeScript boundary

The API runtime helper modules are now TypeScript: `validation.ts`, `rate-limit.ts`, `request-rate-policy.ts`, `request-origin-policy.ts`, and `request-observability.ts`. API imports use TypeScript source paths; `server.ts` remains `Bun.serve` and `app.ts` remains Hono. Validation and limiter helpers are included in the strict `typecheck:layers` command. Node 22 test commands that directly import these TypeScript helpers enable Node's built-in type stripping; Bun remains the production API runtime.

The migration is deliberately scoped. **Thirty existing JavaScript modules remain under `src/lib`**, mostly the legacy analytical/domain implementation with JSDoc checks. They were not mass-renamed: a trial conversion exposed missing explicit parameter/contract types in multiple modules. No `noImplicitAny` relaxation or suppressed type error was introduced. A complete Svelte/TypeScript migration remains an explicit follow-on engineering workstream and is not claimed as delivered here.

## Verification

| Check | Result |
|---|---:|
| `npm test` | **325 passed, 0 failed** across 49 suite processes |
| `npm run typecheck:layers` | **PASS** — strict TypeScript + checked JSDoc layers and new API TypeScript modules |
| `node scripts/verify-repo.mjs` | **PASS** — repository guard; reports absent lockfiles as warnings |
| MT5 risk-control targeted tests | **15 passed, 0 failed** |
| API boundary/rate-limit/context/observability targeted tests | **32 passed, 0 failed** |
| Release preflight | **BLOCKED (9 issues)** |

The verification above is source-level/test-harness evidence. It is not a native application build or broker acceptance test.

## Release blockers and acceptance gates

The release preflight reports:

1. Missing `package-lock.json`.
2. Missing `bun.lock`.
3. Missing `src-tauri/Cargo.lock`.
4. Missing `svelte-check` executable.
5. Missing Vite executable.
6. Missing Tauri CLI executable.
7. `cargo` unavailable.
8. `rustfmt` unavailable.
9. `cargo clippy` unavailable.

The environment also has no Bun executable and no MetaEditor/MT5 terminal. Consequently the following are not verified: Bun/Hono runtime boot, Svelte production compilation, Tauri packaging, Rust compile/unit tests/clippy/formatting, MQL5 compilation, terminal-side race/fault injection, and broker-specific margin/stop-limit behavior. Dependency locks were not fabricated, and release gates were not weakened.

## Safety boundary retained

This feature remains read-only. It does not call `OrderSend`, `OrderModify`, `OrderDelete`, position-close APIs, `OrderCheck`, or a broker-side reservation facility. `OrderCalcMargin` is still only a candidate calculation independent of aggregate broker margin offsets/tiering. Risk also excludes commission, swap, spread expansion, slippage, gaps, stop execution failure, and liquidation behavior. Therefore a `SIZED_WITH_LIMITATIONS` result is not an order authorization, broker guarantee, or permission to automate execution.

## Version decision

The application version remains **0.10.1**. This increment is tracked by concrete code changes, tests, and this engineering record; no version-only change was made.
