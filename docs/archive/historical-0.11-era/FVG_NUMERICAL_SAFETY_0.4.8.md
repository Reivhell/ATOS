# FVG numerical safety — v0.4.8

## Failure mode

Checking that each input price is finite is insufficient to guarantee finite derived geometry. For example, a lower boundary near `-1e308` and an upper boundary near `1e308` are both finite `f64` values, while `upper - lower` overflows to positive infinity. Returning that as an FVG width creates a structurally invalid zone that can poison downstream comparisons and serialization.

## Contract

- Source candle values must pass the existing OHLCV validator.
- Derived FVG width must be finite and strictly positive.
- Midpoint is calculated as `lower / 2.0 + upper / 2.0` to avoid the width subtraction as an intermediate.
- Non-representable geometry returns an error; it is never silently clamped, normalized, or emitted.
- Lifecycle validation independently recomputes and validates geometry against source candles.
- These observations are research-only and do not authorize trading or promote a catalog entry.

## Verification boundary

A native Rust unit test was added for extreme finite inputs. The current environment has no Rust toolchain, so that unit test remains unexecuted here. The Node test only checks that the native regression guard exists; it is not evidence that Rust compiles or the unit test passes.
