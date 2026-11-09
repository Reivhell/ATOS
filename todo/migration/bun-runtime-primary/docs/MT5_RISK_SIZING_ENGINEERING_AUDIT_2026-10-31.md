# ATOS Engineering Audit — Broker-Native MT5 Risk Sizing

Date: 2026-10-31  
Repository application version: 0.10.1 (intentionally unchanged)  
Scope: authenticated MT5 position-risk sizing, Rust/Tauri/Svelte wiring, regression and release-readiness audit.

## Executive result

Implemented a Rust-owned MT5 position-sizing service exposed through a Tauri command and the MT5 Bridge workbench. The change is functional, not a version-only update. It calculates a stop-loss-based volume from the current authenticated account equity, the current executable bid/ask, broker-reported loss tick value, tick size, stop constraints, volume step/min/max, and directional volume-limit data.

This is not a live execution gateway. It does not submit or amend orders. The existing EA remains read-only. The result is explicitly labelled `SIZED_WITH_LIMITATIONS` when implemented checks pass, and `BLOCKED` when a sizing constraint fails. Blocked results return zero approved volume.

## Implementation inventory

- `src-tauri/src/application/mt5_risk_sizing.rs`: Rust calculation boundary, request/result contracts, identity/freshness/contract consistency checks, broker tick-loss formula, directional volume aggregation, strict volume-step floor, and Rust unit tests.
- `src-tauri/src/application/mod.rs`: application module registration.
- `src-tauri/src/adapters/tauri/commands.rs`: `calculate_mt5_risk_sizing` IPC command.
- `src-tauri/src/adapters/tauri/bootstrap.rs`: command registration.
- `src/lib/types.ts`: typed request/result contracts.
- `src/lib/adapters/tauri/bridge.ts`: validated desktop adapter.
- `src/lib/adapters/tauri/native-runtime-guard.js`: capability boundary registration.
- `src/lib/components/Mt5BridgePanel.svelte`: risk-sizing controls, result, blocking reasons, contract revision/fingerprint and limitations.
- `scripts/test-mt5-risk-sizing.test.mjs`: source-contract regression tests, included in `npm test`.
- `package.json`: risk-sizing suite wired into the primary regression chain.
- `docs/MT5_BROKER_NATIVE_RISK_SIZING_ENGINE_2026-10-31.md`: implementation and operational contract.
- `docs/MT5_BRIDGE_SECURITY_AND_OPERATIONS.md`: bridge boundary and release documentation updated.

## Implemented control sequence

1. Validate broker, account, symbol, side, stop-loss price and risk percentage (0.01%–5.00%).
2. Require a currently authenticated MT5 snapshot and exact broker/account identity match.
3. Require a connected and non-stale bridge/terminal, heartbeat within the bridge freshness boundary, quote age 0–60 seconds, and specification observation age 0–60 seconds.
4. Require the requested symbol to equal the current primary quote symbol. The calculator never borrows another symbol's quote or contract metadata.
5. Require positive finite equity, non-crossed positive bid/ask, and valid broker tick/volume/stop metadata.
6. Verify that the durable contract projection agrees with the live snapshot for point, tick size, loss tick value, volume min/max/step and stop level.
7. Use ask for BUY and bid for SELL. Reject a stop on the wrong side, a stop inside the broker minimum distance, or a stop not aligned to tick size.
8. Calculate estimated loss per lot from stop distance, broker tick size and broker loss tick value. Divide the equity risk budget by that estimate.
9. Round volume strictly down to the broker step and cap it at broker maximum. Do not round up to minimum volume; block when minimum volume would exceed the risk budget.
10. Compare against the existing same-direction positions because `SYMBOL_VOLUME_LIMIT` is directional. Pending orders are not included because the bridge does not currently transport them.
11. Return zero approved volume for every blocked decision. Surface margin, commission, swap, slippage, spread-widening and gap-risk exclusions in the result.

## Core calculation

`risk_budget = current_equity * risk_percent / 100`

`estimated_loss_per_lot = (stop_distance / tick_size) * tick_value_loss`

`raw_volume = risk_budget / estimated_loss_per_lot`

`approved_volume = floor(raw_volume / volume_step) * volume_step`, capped at broker maximum and emitted only if every implemented constraint passes.

The calculation uses the broker's latest loss tick value. It is an estimate, not a guaranteed stop fill or guaranteed maximum loss. Margin requirements, leverage tiers, hedged-margin offsets, commission, swap, slippage and stop-gap loss are not calculated. It must not be treated as an order authorization or as proof that a position cannot cause a margin call or negative balance.

## Verification results

- Node regression suite: **310 passed, 0 failed** (`node --test scripts/*.test.mjs`).
- Primary `npm test` chain: passed after the final changes. The direct complete `.mjs` regression run also passed (310/310).
- Shared TypeScript layer check: passed.
- Tauri adapter/type contract check: passed using a temporary declaration for the unavailable Tauri package; no project dependency was installed or modified by that check.
- Repository guardrail check: passed, with dependency-lock warnings.
- JavaScript/MJS syntax scan: 92 files passed.
- JSON parsing: 18 files passed.
- Tauri command registration parity: 38 declared / 38 registered.
- Rust `include_str!` assets: 10/10 resolved.
- Sensitive/local runtime artifact scan: none found.
- EA trade-mutation API scan: no `OrderSend`, position modification or close API added; EA remains telemetry-only.

## Release blockers and unexecuted validation

`npm run release:preflight` remains **BLOCKED (9 issues)**:

1. `package-lock.json` missing.
2. `bun.lock` missing.
3. `src-tauri/Cargo.lock` missing.
4. `svelte-check` unavailable.
5. Vite unavailable.
6. Tauri CLI unavailable.
7. Cargo unavailable.
8. rustfmt unavailable.
9. cargo clippy unavailable.

Consequently, native Rust compilation/unit tests, Svelte production compilation, Tauri packaging, MetaEditor/MQL5 compilation and real broker integration/fault-injection were not run. The Rust unit tests are authored but not executed in this environment. No release certification is claimed.

## Known remaining gaps

- Margin is not estimated or preflighted; this feature does not protect against margin calls or negative balance.
- Pending orders are not in the bridge snapshot, so the broker directional volume-limit comparison is incomplete.
- Current risk sizing is restricted to the terminal's primary streamed symbol.
- Tick value is a point-in-time broker value; nonlinear conversion changes, commissions, swap, slippage, spread widening and gaps can increase realized loss.
- No durable risk-decision journal, execution mandate, order outbox, broker order lifecycle or reconciliation has been added.

## Release posture

Keep the live execution plane closed. The new calculator improves broker-native position sizing but does not satisfy the remaining prerequisites for live order routing. The next high-impact phase should add a broker-side margin preflight using MT5-native calculation semantics and pending-order/position reconciliation before any execution authority is considered.
