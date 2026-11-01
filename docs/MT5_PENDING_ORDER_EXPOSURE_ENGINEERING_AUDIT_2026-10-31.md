# ATOS MT5 Pending-Order Exposure Reconciliation — Engineering Audit

Date: 2026-10-31  
Application version: 0.10.1 (intentionally unchanged)  
MT5 bridge protocol: 5  
Scope: authenticated account inventory, pending-order telemetry, directional volume-limit reconciliation, and risk-sizing integration.

## Executive result

Implemented a protocol-level expansion of the read-only MT5 bridge. The broker snapshot now contains both open positions and the complete bounded active pending-order inventory. Rust validates the inventory before accepting a frame. The broker-native sizing service aggregates same-direction open-position volume and remaining pending-order volume before evaluating `SYMBOL_VOLUME_LIMIT`.

This closes a specific correctness gap in the previous risk-sizing calculation: active pending orders were not represented in the bridge snapshot and therefore could not be included in the broker's directional aggregate-volume constraint.

## Architecture changes

### MQL5 adapter — protocol v5

`MQL5/Experts/ATOS_Bridge.mq5` now serializes active pending orders from `OrdersTotal()` / `OrderGetTicket()`. Each order is explicitly refreshed through `OrderSelect(ticket)` before property reads. The payload includes:

- ticket and broker-native symbol;
- order type and state;
- initial and remaining volume;
- open, current, and stop-limit prices;
- stop loss and take profit;
- millisecond setup timestamp and expiration timestamp;
- magic identifier.

The EA refuses to emit a partial snapshot when the active pending-order inventory exceeds 200, an order cannot be selected, or an order has invalid volume/price data. The frame remains read-only; no order placement, modification, cancellation, or deletion API was added.

### Rust bridge validation

`Mt5PendingOrderSnapshot` is a strict serde contract with unknown-field rejection. The receiver validates:

- a maximum of 200 pending orders;
- unique order tickets;
- bounded, non-empty ticket/symbol/magic identifiers;
- pending order type range 2..=7;
- live/in-flight order states only (started, placed, partial, request-add, request-modify, request-cancel);
- positive setup time and non-negative expiration time;
- finite, non-negative price values;
- positive initial and remaining volume, with remaining volume no greater than initial volume.

The `pendingOrders` property is required by protocol v5. A protocol-v4 payload cannot silently deserialize as a v5 snapshot.

### Risk sizing

The risk-sizing service now computes:

`total_directional_symbol_volume = open_position_volume + remaining_pending_order_volume`

BUY pending order types are BUY_LIMIT (2), BUY_STOP (4), and BUY_STOP_LIMIT (6). SELL pending order types are SELL_LIMIT (3), SELL_STOP (5), and SELL_STOP_LIMIT (7). Partially filled orders contribute `ORDER_VOLUME_CURRENT`; the filled portion is represented in the open-position inventory.

When `SYMBOL_VOLUME_LIMIT` is non-zero, a proposed volume that would exceed the broker's aggregate same-direction limit is blocked. The result separately exposes open volume, pending volume, and combined directional volume.

## Safety and non-goals

- No live order route exists.
- No order is submitted, amended, canceled, or deleted.
- The pending-order inventory is a point-in-time broker snapshot, not a reservation. It may change immediately after sizing; it is not atomic with any future order request.
- This is not margin preflight. Candidate-specific `OrderCalcMargin`, margin tiers, hedged-margin offsets, commission, swap, slippage, and gap loss remain uncomputed.
- It does not calculate account-wide cross-symbol stress loss or guarantee protection from margin call or negative balance.
- Over-cap or malformed inventory blocks the entire frame; it is never truncated and treated as complete.

## Official API contract references

The implementation follows MetaQuotes' official [MQL5 Order Properties](https://www.mql5.com/en/docs/constants/tradingconstants/orderproperties) for `ORDER_VOLUME_INITIAL`, `ORDER_VOLUME_CURRENT`, `ORDER_PRICE_OPEN`, `ORDER_PRICE_CURRENT`, `ORDER_PRICE_STOPLIMIT`, `ORDER_TIME_SETUP_MSC`, order types, and order states. The directional aggregation follows the official [MQL5 Symbol Properties](https://www.mql5.com/en/docs/constants/environment_state/marketinfoconstants) definition of `SYMBOL_VOLUME_LIMIT`, which applies to open position plus pending-order volume in one direction.

## Validation evidence

- `node --test scripts/*.test.mjs`: 314 tests passed, 0 failed.
- `npm test`: passed, including the newly registered pending-order contract suite.
- `npm run typecheck:layers`: passed.
- Tauri adapter TypeScript check: passed using a temporary declaration for the unavailable installed Tauri API package; no project source was changed for the stub.
- `node scripts/verify-repo.mjs`: passed with dependency-lock warnings.
- JavaScript/MJS syntax scan: 93/93 files passed.
- JSON parse scan: 18/18 files passed.
- MQL5 snapshot `StringFormat` placeholder/argument parity: passed.
- Static EA scan: no `OrderSend`, `OrderModify`, or `OrderDelete` calls.
- Tauri command declaration/registration parity: 38/38; Rust `include_str!` assets: 10/10 resolved.
- Repository secret/artifact scan: no `.env`, database, PEM, key, or log artifacts found in the source tree.
- Repository archive integrity and SHA-256 checksum are verified at packaging time.

## Release blockers

Release preflight remains blocked by nine environment/dependency issues:

1. `package-lock.json` absent.
2. `bun.lock` absent.
3. `src-tauri/Cargo.lock` absent.
4. `svelte-check` unavailable.
5. Vite unavailable.
6. Tauri CLI unavailable.
7. Cargo unavailable.
8. rustfmt unavailable.
9. cargo clippy unavailable.

Therefore Rust compilation and Rust unit-test execution have not been performed in this environment. Svelte production build and MetaEditor compilation have not been performed. No real MT5 terminal or broker account integration test has been run. The Node contract tests validate repository contracts and wiring; they do not substitute for those native validations.

## Installation compatibility

Protocol v5 is incompatible with protocol v4. Compile and install the repository EA at `MQL5/Experts/ATOS_Bridge.mq5` before reconnecting ATOS. The application version remains 0.10.1 because this delivery is an implementation increment, not a release-version exercise.
