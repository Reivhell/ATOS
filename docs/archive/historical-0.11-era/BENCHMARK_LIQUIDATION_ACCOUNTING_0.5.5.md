# Benchmark liquidation accounting — 0.5.5

## Defect
The benchmark's final-close exit costs (fixed slippage and fee) were applied inside the strategy's `if in_position` liquidation branch. This coupled benchmark accounting to the strategy's terminal position state. When the strategy was already flat, the benchmark retained a gross mark-to-market value while the strategy's closed trades included transaction costs, biasing relative performance.

## Correction
The strategy is liquidated as before when it has an open position. Benchmark liquidation costs are then applied exactly once to the final benchmark point, regardless of the strategy's position state:

`benchmark_final = benchmark_mark_to_market * (1 - slippage_rate) * (1 - fee_rate)`

The benchmark buy quantity continues to account for entry fee and slippage through the entry denominator. The final benchmark value now accounts for exit slippage and exit fee consistently.

## Regression coverage
- Native Rust test `benchmark_liquidation_costs_apply_when_strategy_is_flat` calculates the expected benchmark independently and asserts the final reported value.
- Node contract test asserts the regression exists and benchmark liquidation occurs after the strategy-position conditional.

## Limitations
The Rust regression was authored but not executed in this environment because `cargo`/`rustc` are unavailable. Node tests verify the source contract, not Rust runtime behavior. The benchmark remains a simplified buy-and-hold comparator: it does not model market impact, order-book depth, funding, borrow costs, taxes, or venue-specific execution constraints.
