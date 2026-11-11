# Backtest cadence integrity — 0.5.3

## Problem

The previous backtest inferred candle cadence from the median of adjacent timestamp deltas. A median can conceal internal missing bars. This can distort the strategy's observed path and annualized Sharpe assumptions while still producing apparently valid performance metrics.

## Change

- The native Rust backtest now establishes cadence from the first adjacent candle pair and requires every subsequent delta to match it exactly.
- Timestamp subtraction uses `checked_sub`; an unrepresentable delta fails closed.
- Gap errors identify the row, observed and expected milliseconds, and a missing-interval estimate when the delta is an exact multiple.
- Native regression tests cover a missing interval, an irregular non-integral delta, and timestamp-delta overflow.
- A dependency-free Node contract test guards the presence of the native policy and regression cases.

## Scope and limitations

This policy targets continuous markets. It is not suitable for session-based markets with legitimate overnight/weekend closures until a trusted venue calendar defines expected intervals. The field `medianCandleIntervalMs` remains for compatibility, but on accepted series it now contains the verified uniform cadence, not a median chosen to hide gaps.

## Validation status

Node contract tests can verify that the native guard and Rust regression cases remain present, but cannot prove Rust compilation or execution. `cargo test`, Clippy, frontend build, locked dependency installation, and desktop packaging must pass in CI before release certification. Live order execution remains disabled.
