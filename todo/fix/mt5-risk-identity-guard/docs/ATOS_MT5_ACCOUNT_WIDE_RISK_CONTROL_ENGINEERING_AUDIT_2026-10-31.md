# ATOS MT5 Account-Wide Risk Control Plane — Engineering Audit

**Audit date:** 2026-10-31  
**Repository application version:** 0.10.1 (unchanged)  
**MT5 bridge protocol:** v6  
**EA:** `MQL5/Experts/ATOS_Bridge.mq5`  
**Release status:** Not production-certified; native build and broker acceptance gates remain blocked.

## Executive summary

This engineering increment extends the existing authenticated, read-only MT5 bridge into an account-wide risk-sizing and margin-preflight control plane. It combines every open position and active pending order across the account, uses terminal-native P/L and margin calculations, constrains candidate risk by remaining portfolio capacity, and rejects account/symbol permission conflicts and inconsistent account snapshots.

It also corrects an accounting error in the prior risk model: current equity already includes floating P/L, so calculating existing-position loss from entry and subtracting that amount from current equity would double-count the open loss. Open-position stop risk is now measured from the current mark to the recorded stop-loss. Pending-order risk continues to be measured from its activation/entry price because the pending order is not yet represented in floating P/L.

This remains a calculation and risk-control boundary. There is no live order submission, modification, close, cancellation, or order reservation API.

## Implemented architecture

### 1. Protocol v6 account and exposure contract

The terminal snapshot contains:

- Account balance, equity, used/free/initial/maintenance margin, margin-call and stop-out thresholds, stop-out mode, account margin mode, hedge permission, pending-order limit, and separate account/terminal/EA trading-permission flags.
- A bounded, complete inventory of open positions and active pending orders.
- Per-item stop-risk state: `UNPROTECTED`, `CALCULATED`, or `CALCULATION_FAILED`, plus an optional amount in account currency.
- Existing quote, instrument specification, and session-calendar telemetry.

Rust rejects malformed or contradictory status/value combinations, duplicate ticket identities, invalid numeric data, disconnected terminal state, and inventories exceeding protocol limits. It never truncates the account inventory and then treats the result as complete.

### 2. Account-wide stop-risk model

For open positions, MT5 `OrderCalcProfit` evaluates the additional price movement from the current position mark to the recorded SL. That is the incremental equity loss still possible after the current mark, avoiding a second subtraction of floating P/L already contained in equity.

For pending orders, MT5 `OrderCalcProfit` evaluates the planned loss from the pending activation/entry price to the order's SL. Pending-order P/L is not yet included in account equity, so the full planned stop loss is included in account exposure.

The aggregate spans all reported symbols, not only the chart symbol. An unprotected or unvalued position/order blocks additional exposure. The 5% account-wide ceiling is applied to current equity. The remaining headroom is calculated as:

`max(0, equity * 5% - current_account_stop_risk)`

A new candidate is sized against the smaller of its requested per-trade budget and this remaining account capacity. The lot amount is rounded down to the broker's volume step. If the broker's minimum lot exceeds the available risk budget, the result is blocked instead of silently raising risk to the minimum lot.

### 3. Broker-native candidate preflight

Rust issues one UUID-correlated request at a time through the authenticated ACK channel. The request has a five-second expiry. The EA obtains a current terminal quote and returns:

- Candidate margin from `OrderCalcMargin`.
- Candidate stop-loss P/L from `OrderCalcProfit`.
- Bid/ask and quote timestamp.
- Account equity, used margin, free margin, and current broker stop-out thresholds/mode.
- Request identity and calculation timestamp.

Rust validates identity, side, requested volume, quote direction, quote age, calculation age, expiry, finite numbers, and account-state consistency. A future-dated calculation timestamp is rejected even if it would fit within the wider generic bridge clock-skew allowance. A difference greater than 0.5% of equity in equity, used margin, or free margin between the telemetry snapshot and the native calculation blocks the result.

Rust also validates broker account-level trading permission; symbol disabled/long-only/short-only/close-only/full mode; and `SYMBOL_ORDER_MARKET` plus `SYMBOL_ORDER_SL` flags. Terminal Algo Trading and EA live-trading flags are visible but do not gate this read-only calculation. A future execution controller must independently revalidate terminal and EA permissions at the actual submission boundary.

### 4. Margin and stressed-equity controls

The candidate's terminal-calculated margin is added to current used margin without assuming a hedged/netting relief. The engine checks:

- Candidate margin against current free margin.
- A projected free-margin reserve of at least 20% of current equity.
- Percentage-based stop-out reserve: maximum of 300%, twice the broker call level, or three times the broker stop-out level.
- Money-based stop-out reserve: maximum of twice the broker call amount, three times the broker stop-out amount, or 10% of current equity.
- Stressed equity after all currently reported stop risks and the candidate stop risk.

If projected used margin is zero, percentage margin level is not applicable; the engine no longer fabricates a 0% margin level and blocks a no-margin account for that reason alone. Free-margin and stressed-equity checks still apply. Non-finite or overflowed projections are omitted and block approval rather than being serialized as JSON numbers.

### 5. UI and API integration

The existing MT5 workbench exposes account-wide position/pending-order stop risk, protected/unprotected/unresolved counts, risk-budget headroom, native candidate margin, projected free margin, stressed margin level when applicable, broker thresholds, and directional symbol volume.

Rust remains the source of truth for all decisions. Tauri and Svelte expose the read-only calculation result; they do not make a second client-side risk decision.

## Verification performed

| Check | Result |
|---|---:|
| Node regression tests (`node --test scripts/*.test.mjs`) | **322 passed, 0 failed** |
| TypeScript shared-layer check (`npm run typecheck:layers`) | Passed |
| Tauri bridge/type adapter check with temporary `invoke` declaration | Passed |
| JavaScript/MJS syntax scan | 94 files passed |
| JSON parsing | 18 files passed |
| Rust `include_str!` asset paths | 10/10 resolved |
| Tauri command declarations vs registration | 38/38 matched |
| MQL5 delimiter scan | Braces, parentheses, brackets balanced |
| MQL5 literal `StringFormat` placeholder/argument scan | 18 calls passed |
| Composite `BuildSnapshot` formatter parity | Covered by MT5 instrument-registry regression test |
| EA trade-mutation API scan | No order/position mutation API found |
| Source secret/local database artifact scan | No `.env`, key/PEM, or local database artifact found |
| Repository guardrail (`node scripts/verify-repo.mjs`) | Passed with lockfile warnings |

The JavaScript/TypeScript checks are source-level checks. The temporary declaration used for the Tauri adapter check supplies only the missing `@tauri-apps/api/core.invoke` type signature; it is not a substitute for installing the real Tauri dependencies or compiling the application.

## Release blockers and unverified acceptance gates

`npm run release:preflight` remains **BLOCKED (9 issues)**:

1. `package-lock.json` is absent.
2. `bun.lock` is absent.
3. `src-tauri/Cargo.lock` is absent.
4. `svelte-check` is unavailable.
5. Vite is unavailable.
6. Tauri CLI is unavailable.
7. Cargo is unavailable.
8. rustfmt is unavailable.
9. cargo clippy is unavailable.

Consequently:

- Rust unit tests were added but not executed in this environment.
- The Rust crate was not compiled, formatted, or linted here.
- The Svelte production build was not executed.
- The Tauri desktop application was not built.
- The MQL5 EA was not compiled by MetaEditor.
- No real MT5 terminal, broker demo/live account, broker-specific margin tier, hedging/netting scenario, disconnect/fault injection, or cross-platform packaged install was available for acceptance testing.

No lockfiles were fabricated and no release gate was weakened to obtain a green status.

## Material limitations

- MQL5 `OrderCalcMargin` calculates a candidate request as if no current positions or pending orders existed. ATOS adds the returned amount to current used margin without assuming offset relief, but this remains a conservative projection rather than a complete broker/server margin calculation. It can diverge from broker-specific aggregate margin tiers, hedged treatment, margin rates, or trade-server rules.
- This implementation does not call `OrderCheck`, does not reserve margin, and does not guarantee that a later order request would be accepted.
- Risk values exclude commission, swap, financing, widened spreads, slippage, gaps, stop execution failure, and broker liquidation behavior.
- Account and order inventories are point-in-time snapshots. State can change immediately after preflight.
- A broker-reported stop-out reserve is not a guarantee against margin call or negative balance. ATOS does not promise protection from debt or negative equity.
- Account-wide risk is bounded by the inventory delivered by the connected terminal and the hard protocol limits. The bridge rejects over-cap inventories rather than reporting them as complete.

## Compatibility and installation note

Protocol v6 is incompatible with the preceding protocol. Compile and install the EA from this repository path before reconnecting:

`MQL5/Experts/ATOS_Bridge.mq5`

The application version remains `0.10.1`; no cosmetic version increment was made. The protocol number reflects the changed wire contract.

## Release decision

**Engineering implementation: completed for this increment. Production certification: not granted.** Native toolchain execution, MetaEditor compilation, and real-broker acceptance are mandatory before this control plane can be treated as production-approved. Live execution must remain disabled until a separate order lifecycle, durable outbox/idempotency, `OrderCheck`/server reconciliation, and failure-recovery architecture is implemented and validated.
