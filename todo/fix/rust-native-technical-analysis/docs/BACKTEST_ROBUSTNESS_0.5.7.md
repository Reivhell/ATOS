# Backtest Robustness Suite — v0.5.7

## Purpose

A single historical return is not sufficient to assess whether a strategy result depends on cheap execution or one favorable time interval. This increment adds two deterministic, research-only checks to every successful Rust backtest: cost sensitivity and chronological out-of-sample (OOS) window evaluation.

## Execution-cost sensitivity

The baseline configuration is unchanged. Two scenarios rerun the same strategy and candle set with higher fee/slippage assumptions:

- **Moderate cost shock:** fee = `min(max(base × 1.5, base + 2 bps), 500 bps)`; slippage = `min(max(base × 1.5, base + 5 bps), 2,000 bps)`.
- **Severe cost shock:** fee = `min(max(base × 2, base + 5 bps), 500 bps)`; slippage = `min(max(base × 2, base + 10 bps), 2,000 bps)`.

The report returns applied costs, cap flags, final equity, return, return delta in percentage points versus baseline, max drawdown, rough Sharpe, and trade count. These are deliberate cost shocks, not claims that the values represent a specific exchange, spread, depth, latency, or market impact. If a ceiling is reached, the response explicitly flags it; a capped scenario can be less severe than the requested multiplier.

## Out-of-sample window evaluation

- The strategy parameters stay fixed; the suite is **not** a parameter optimizer.
- Each test window contains `max(2 × slow SMA period, 30)` bars.
- Each window receives `slow SMA period + 2` preceding bars as indicator warm-up. The simulator starts at the evaluation index with flat inventory and fresh initial capital; warm-up bars cannot create trades or contribute returns to the fold.
- At least two complete test windows are required. The suite reports up to the 12 most recent complete windows. An incomplete tail is excluded; the main report adds a caveat when no qualifying folds are available.
- Each fold reports its timestamp range, tested bar count, return, drawdown, rough Sharpe and closed-trade count. Positions/cash do not carry between windows.

The regression suite includes a future-value invariance test: modifying candle values after an already-completed fold must not alter that fold's metrics. Native Rust unit tests are authored for this behavior, but they have not been executed in this workspace because the Rust toolchain is unavailable.

## Interpretation boundary

OOS windows are a diagnostic for variation across sampled historical periods, not a proof of future profitability. Fold restarts remove path dependence across windows, and the rough Sharpe estimate may be unstable in short windows. The fixed cost shocks do not model order-book liquidity, spreads, latency, partial fills or impact. Provider authenticity and data completeness remain separate concerns.

The cost stress and OOS outputs are included in the normal backtest JSON export so reports are reviewable outside the UI. This increment does not add live execution, change risk-gateway authority, or promote catalog-only technical-analysis concepts.
