# MT5 Broker-Native Risk Sizing and Account-Wide Control Plane

## Scope

This change adds an account-wide pre-trade risk control plane to the authenticated, read-only MT5 bridge. Rust owns policy and decision logic. The terminal remains the authority for broker-native calculations. No order is placed, modified, closed, deleted, reserved, or authorized by this feature.

## Protocol v6 data contract

The account snapshot now includes broker margin-call and stop-out thresholds, whether those thresholds are percentage- or money-denominated, account margin mode, hedge permission, pending-order cap, initial margin, and maintenance margin. Every position and active pending order includes:

- `riskStatus`: `UNPROTECTED`, `CALCULATED`, or `CALCULATION_FAILED`.
- `riskAtStopLoss`: an optional account-currency *additional* loss amount calculated by terminal-side `OrderCalcProfit`. For an open position, the calculation starts at the current mark price because account equity already includes its floating P/L; for a pending order, it starts at the activation/entry price because that order is not yet represented in floating P/L.

A position/order without an SL is explicitly unprotected. An SL that the terminal cannot value is unresolved. Either state blocks the addition of new risk. The account inventory spans every position and pending order in the snapshot, not only the selected chart symbol. Duplicate tickets, non-finite values, impossible status/payload combinations and over-cap inventories are rejected.

## Risk budget and sizing

The user's requested trade budget remains:

`requested_trade_budget = equity * requested_risk_percent / 100`

ATOS enforces a 5% account-wide stop-loss ceiling. The remaining capacity is:

`remaining_account_capacity = max(0, equity * 5% - current_account_stop_risk)`

The actual budget used for sizing is:

`effective_trade_budget = min(requested_trade_budget, remaining_account_capacity)`

The engine calculates an initial lot size from broker-reported tick size and tick-loss value, rounds volume down to the broker step and to the greatest step-aligned volume not above the broker maximum, and blocks when the minimum lot would exceed the effective budget. It then reconciles the candidate with MT5 `OrderCalcProfit`; where the native loss is higher than the effective budget, it reduces volume and performs a second native calculation. The final approved volume is zero whenever any blocking condition remains.

Existing same-direction positions and remaining active pending-order volume continue to count toward the broker's directional `SYMBOL_VOLUME_LIMIT` for the selected symbol.

## Native margin preflight

Rust may issue one outstanding correlation-ID-bound preflight command with a five-second expiry through the authenticated ACK channel. The EA obtains a fresh terminal tick and calls:

- `OrderCalcMargin` for the candidate side and volume.
- `OrderCalcProfit` for candidate loss to the requested stop.

The result includes quote timestamp, calculation timestamp, account equity, used margin, free margin and broker stop-out thresholds/mode. Rust checks request identity, volume, quote freshness, expiry and numeric validity. A future-dated calculation timestamp is rejected even when it falls inside the generic bridge clock-skew allowance. Before accepting it, Rust also compares equity, used margin and free margin with the telemetry snapshot; drift greater than 0.5% of equity blocks the result. The stop distance is revalidated against the live preflight quote and broker stops level.

The preflight is bound to one immutable telemetry frame rather than an independently sampled account read. The EA captures one `MqlTick` while building the account snapshot and reuses that exact tick for both `OrderCalcMargin` and `OrderCalcProfit`; it does not perform a second quote read for the candidate calculation. The server binds the result to the authenticated frame sequence that carried it, and Rust requires that sequence to still be the latest accepted snapshot before reconciling account-wide risk. The preflight is rejected if quote timestamp/bid/ask, broker margin policy, or account identity differ.

MT5 does not provide an atomic account inventory transaction. Therefore the EA fences both ordinary telemetry frames and preflight frames: after enumerating positions and active orders it re-reads ticket sets, counts, position type/volume/current mark/SL, and pending-order type/state/initial and remaining volume/entry/stop-limit/SL. The EA discards the frame if any risk-bearing field changed during enumeration. It performs the same inventory fence again immediately before transmitting a frame containing a native preflight result. These checks reduce the race window; they do not create a broker-side lock or reservation. The fence also revalidates the paired login, broker server, and account currency before reconnecting, building telemetry, and transmitting; an account switch fails closed and requires an explicit EA restart/re-pair rather than reusing the old session identity.

A stop is treated as protective only when it is strictly on the loss side of the current position mark or pending-order entry price. Equal-price, crossed, or profitable-side stops cannot produce a `CALCULATED` inventory state, and a zero-valued loss result is not accepted as proof of protection. Such exposure remains unresolved and blocks new sizing. The UI retains a completed sizing result as history, but suppresses its volume as an active approval once the authenticated snapshot sequence advances, becomes stale, or disconnects.

Before requesting terminal calculations, Rust checks the broker account-level trading permission, symbol trade mode (disabled, long-only, short-only, close-only or full), and symbol order-mode flags. Market preflight is blocked unless both market-order and stop-loss permissions are present. Terminal Algo Trading and EA live-trading flags remain visible in telemetry but do not gate this read-only calculation; the bridge must not require live-trading enablement merely to inspect risk. Any future execution controller must independently revalidate terminal and EA permissions at submission time.

Candidate margin is added to current used margin without assuming a netting or hedged-margin offset. The engine checks a 20%-of-equity free-margin reserve and a stressed margin-level floor of the maximum of 300%, twice broker margin-call level, or three times broker stop-out level. For money-denominated stop-out mode, it applies a reserve floor based on the broker thresholds and 10% of equity. If projected used margin is zero, percentage margin level is treated as not applicable rather than an artificial 0%; free-margin and stressed-equity checks remain active. Any non-finite projection is blocked and omitted rather than serialized.

These calculations are conservative policy checks, not an `OrderCheck` response and not a server reservation. They do not guarantee the broker will accept a future request or that execution will occur at the calculated price.

## Independent account-inventory validation boundary

`account_risk_inventory` validates the complete snapshot itself before reading or aggregating it. Callers therefore cannot obtain an account-risk summary from a malformed snapshot merely by bypassing the authenticated bridge ingestion path. The boundary rechecks account and terminal metadata, broker contract consistency, quote validity, position/order identity and uniqueness, active pending-order state, volume/price validity, protective-stop direction, and stop-risk status/payload consistency. Every calculated stop-risk contribution must be finite and strictly positive. Risk is accumulated one item at a time; non-finite intermediate totals fail immediately, and non-positive equity cannot produce a risk percentage.

Directional open-position and pending-order volumes use the same checked aggregation discipline. Each item and each intermediate total must be finite and non-negative. The combined same-direction total is checked again after native preflight reconciliation, so the later account snapshot cannot bypass the same arithmetic invariant. This also prevents non-finite diagnostic totals from reaching the serialized sizing result when a broker reports no directional volume cap.

## Fail-closed conditions

New risk is blocked for stale or disconnected telemetry, account identity mismatch, stale contract metadata, invalid stop placement, invalid or unknown margin policy, any unprotected/unresolved account exposure, an exhausted account-wide risk budget, insufficient free margin, inadequate stressed margin level, native calculation errors, stale/expired/future-dated quote or calculation timestamps, disabled account/symbol/order permissions, non-finite projected arithmetic, or account-state drift during preflight.

`SIZED_WITH_LIMITATIONS` means implemented sizing and control checks passed for one point-in-time snapshot. It is not a trade authorization or execution command. `BLOCKED` means the checks did not pass; approved volume and approved stop-loss amount remain zero.

## Explicit limitations

- The account risk inventory is point-in-time and not a broker-side volume or margin reservation. Positions/orders can change immediately after a snapshot.
- Existing-position risk is the incremental loss from current mark to recorded SL; pending-order risk is the full planned loss from activation/entry to SL. This avoids subtracting floating losses already included in current equity a second time. Both are calculated with MT5 `OrderCalcProfit`. The model excludes commissions, swap, financing, spread expansion, slippage, gaps, stop execution failure and broker liquidation behavior.
- If any existing position or pending order lacks an SL or cannot be valued, the engine blocks all new exposure rather than assigning an invented risk value.
- `OrderCalcMargin` estimates a candidate request independently of existing positions and pending orders. ATOS adds that estimate to current used margin, intentionally avoiding any assumed hedge/netting relief. Broker-side offsets, margin tiers, symbol-specific rates and order-type policy can differ.
- Account margin-call and stop-out thresholds are broker-reported snapshots and can change. A threshold comparison cannot guarantee that a broker will not liquidate positions.
- Account-wide aggregation is limited to the open-position and active-pending-order inventory successfully delivered by the attached MT5 terminal. The bridge rejects inventories above the protocol cap rather than truncating them.
- No live order submission, amendment, close, cancel, durable order outbox, or broker reconciliation execution route exists.

## Verification boundary

Rust unit tests cover account-risk aggregation, overflow rejection, remaining account-risk capacity, volume rounding, volume limits, and preflight validation. Node contract tests cover protocol v6 fields, terminal calculation calls, correlation/expiry paths, account-state drift policy, UI visibility, and absence of trade mutation methods. These source and contract checks do not replace native Cargo/Rust tests, Svelte production compilation, MetaEditor compilation, terminal-side fault injection, or integration testing against the target broker.
