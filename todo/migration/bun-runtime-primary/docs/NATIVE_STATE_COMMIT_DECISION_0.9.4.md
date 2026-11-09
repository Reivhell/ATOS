# Native state-commit decision layer — 0.9.4

## Purpose

`src-tauri/src/state_decision.rs` is the native policy gate for the durable `EngineState` snapshot. It evaluates the exact serialized payload read from SQLite against the candidate serialized payload **inside the same SQLite transaction** that writes the new snapshot. The UI and frontend state are not authoritative for this decision.

## Transition rules

- **Bootstrap:** the first persisted snapshot must be an empty paper account with initial cash, no market/screener observations, no positions/orders/agents, and the kill switch engaged.
- **Unknown state changes:** every mutable field is explicitly classified. Adding, removing, or changing an unclassified field blocks commit until policy is updated.
- **Identity and audit:** embedded schema version and initial account balance are immutable. Embedded event history must preserve the existing ordered events, permits only new unique events at the front, and removes only the oldest tail when its bounded capacity is reached.
- **Cash, positions and orders:** cash or position-exposure changes require one newly appended paper fill with unique, non-empty risk checks, a corresponding execution event, and atomic changes to order, cash, and exposure. Rejections require a failed risk check and a matching risk-gateway event. Order rewrites, bulk appends, and silent ledger edits are blocked.
- **Risk and kill switch:** changed risk limits require an explicit risk-policy event. Releasing the kill switch requires a new release event plus a connected, fresh provider snapshot and fresh marks for every open position. Startup, account reset, and operator engagement remain distinct transitions.
- **Market and agents:** market observations, screener observations, and agent control state each require their respective provider/operator/migration evidence event.
- **Derived values:** equity peak, maximum drawdown, UTC daily baseline, and position marks are rechecked against deterministic equations and candidate quotes. A calendar rollover remains valid even when the day's opening equity happens to equal the previous day's value.
- **Durable audit table:** SQLite `BEFORE UPDATE` and `BEFORE DELETE` triggers reject mutation of `audit_events`; audit inserts and the matching state snapshot are part of the same transaction.

## Root-cause correction

Resetting a paper account now explicitly engages the kill switch every time. Previously, resetting while the switch was already released could clear the account while leaving the order-entry lock released. The new regression test covers that exact path. A new process also remains fail-closed until a fresh market snapshot is explicitly reviewed and the operator releases the switch.

## Test and trust boundary

Rust unit tests cover policy decisions, audit immutability, atomic fill transitions, restart-sensitive market evidence, state-field classification and daily rollover. Node tests check the wiring and policy contracts. Node source tests do **not** execute or compile Rust. SQLite triggers require the native Rust test harness for runtime validation; the current workspace lacks the Rust toolchain, so neither Rust compilation nor the native unit tests/Clippy can be claimed as passed here.

This is a local paper-trading state-control boundary, not a cryptographic external tamper-proof ledger. A local user who can replace the database/file or patch the application binary is outside the protection provided by SQLite triggers. No live broker execution authority, mock/demo runtime feature, or live order path is introduced by this release.
