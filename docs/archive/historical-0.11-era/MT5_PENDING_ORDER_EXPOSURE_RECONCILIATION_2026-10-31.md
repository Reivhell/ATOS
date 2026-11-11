# MT5 Pending-Order Exposure Reconciliation — 2026-10-31

## Scope

Protocol v6 extends the authenticated terminal snapshot with a bounded, complete inventory of active pending orders. Rust validates ticket uniqueness, symbol identity syntax, order type/state, millisecond setup timestamps, remaining volume, and all broker prices before accepting a frame. `ORDER_STATE_PARTIAL` (3) is accepted because a partially executed order remains active while its unfilled volume is still pending; `ORDER_VOLUME_CURRENT` is the remaining volume included in directional-limit and stop-risk aggregation. Excluding this state would reject the entire account snapshot during a partial fill. Exceeding the 200-order cap rejects the whole snapshot; the bridge never publishes a partial order inventory.

## Risk-sizing impact

Directional `SYMBOL_VOLUME_LIMIT` checks now aggregate open positions and remaining active pending-order volume for the selected symbol and side. MQL5 order types BUY_LIMIT/BUY_STOP/BUY_STOP_LIMIT map to the buy direction; SELL_LIMIT/SELL_STOP/SELL_STOP_LIMIT map to sell. Partial fills use `ORDER_VOLUME_CURRENT`, while the resulting filled exposure is represented by the positions inventory. The validator accepts the active/in-flight states 0, 1, 3, 7, 8, and 9; it rejects terminal `ORDER_STATE_CANCELED` (2), `ORDER_STATE_FILLED` (4), `ORDER_STATE_REJECTED` (5), and `ORDER_STATE_EXPIRED` (6), as well as inconsistent remaining volume; the EA's account-inventory fence rechecks both the order state and remaining volume before transmission.

## Protocol v6 account-risk and margin-preflight impact

Protocol v6 extends each account snapshot with broker stop-out mode, margin-call threshold, stop-out threshold, initial/maintenance margin, margin mode, hedge permission, and the broker pending-order cap. Each position and pending order carries an explicit stop-risk state (`UNPROTECTED`, `CALCULATED`, or `CALCULATION_FAILED`) and an optional loss-at-SL amount calculated by MT5 `OrderCalcProfit` in the account currency. The Rust inventory aggregates all symbols and blocks new sizing whenever any position/order has no SL or cannot be valued.

Rust can serialize one bounded, expiring margin-preflight request at a time through the authenticated ACK channel. The EA computes candidate margin with `OrderCalcMargin` and candidate stop-loss with `OrderCalcProfit`, then returns an identity-correlated result in the next telemetry frame. Rust rejects expired, stale, mismatched, non-finite, or account-state-drifting results. It compares equity, used margin, free margin, stop-out mode and broker thresholds against the snapshot before using the result.

The effective trade risk budget is the lower of the user's requested per-trade budget and the remaining capacity under ATOS's 5% account-wide stop-loss ceiling. Projected margin is conservatively estimated by adding candidate margin to currently used margin; this deliberately does not assume netting or hedged-margin relief. The bridge remains read-only: `OrderCalcMargin` and `OrderCalcProfit` are calculation functions, not trade mutation APIs. No `OrderSend`, `OrderModify`, `OrderDelete`, or close/cancel route was added.

## Broker API grounding

The serializer refreshes each selected order immediately before reading its properties and uses `ORDER_TIME_SETUP_MSC` for the millisecond setup timestamp. The order property names and active/in-flight state values follow the official MQL5 order-properties reference. `SYMBOL_VOLUME_LIMIT` explicitly applies to the aggregate open position plus pending orders in one direction; the risk-sizing implementation now matches that broker definition.

References: [MQL5 Order Properties](https://www.mql5.com/en/docs/constants/tradingconstants/orderproperties), [MQL5 Symbol Properties](https://www.mql5.com/en/docs/constants/environment_state/marketinfoconstants).

## Compatibility

Protocol v6 is incompatible with protocol v5. Compile and install the repository EA at `MQL5/Experts/ATOS_Bridge.mq5` before reconnecting.

## Partial-fill reconciliation correction

The protocol validator previously omitted `ORDER_STATE_PARTIAL` from the accepted active-order states. An earlier implementation mistakenly treated numeric state 2 as partial; the corrected enum contract is `ORDER_STATE_PARTIAL = 3` and `ORDER_STATE_CANCELED = 2`. MQL5 documents this as an active state, with `ORDER_VOLUME_CURRENT` retaining the unfilled remainder. The omission caused the full telemetry frame to be rejected when a pending order partially filled. The accepted-state contract and native regression fixture now include state 3; the test also asserts that an unprotected partial remainder remains part of account-wide exposure and blocks additional risk.

The position-sizing volume normalizer also caps the maximum in broker step units before applying the result. This avoids returning a non-step-aligned volume when a broker's reported maximum is not an exact multiple of its volume step.


## Final risk-sizing freshness fence

The sizing response is timestamped at completion, not at function entry. After broker-native margin/profit calculation and inventory reconciliation, Rust rechecks that the authenticated account identity, symbol, terminal connection, quote timestamp and telemetry sequence still match the frame used for the decision. Quote age and broker specification age are evaluated again at completion. A newer frame, disconnect, stale quote, or expired specification blocks the returned sizing result rather than allowing a preflight based on an older snapshot to appear current.


## Checked directional aggregation

The directional exposure aggregator now returns an error instead of a floating-point total when any contributing volume is non-finite or negative, or when an intermediate sum overflows. Open-position and active pending-order totals are combined through the same checked helper. The calculation is repeated against the correlated account inventory after terminal-native preflight, and the result remains fail-closed if that later inventory cannot be aggregated safely. Rust unit coverage includes overflowing position totals and non-finite inputs.
