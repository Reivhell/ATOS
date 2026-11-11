# Research SMC Detectors — v0.3.4

## Added: confirmed swing points

`detect_confirmed_swings(candles, left_bars, right_bars)` is a deterministic Rust research primitive that reports unique fractal `swing_high` / `swing_low` geometry.

- A pivot is emitted only after `right_bars` subsequent candles exist; `confirmed_at_index` and `confirmed_at_ms` make the earliest knowable point explicit.
- Both left/right windows must be non-zero and no larger than 10,000 bars.
- Strict inequalities reject tied highs/lows rather than silently choosing a pivot from an ambiguous equal-price cluster.
- Candle validation reuses the bounded OHLCV validation contract: finite values, valid OHLC, nonnegative volume, strictly increasing timestamps, and maximum input size.
- Output ordering is deterministic by confirmation index, pivot index, and kind.
- This is not a complete BOS, CHoCH, MSS, HH/HL/LH/LL, protected swing, or strategy detector. It must not be interpreted as one.

## Gate and status

This primitive is research-only and is not wired to the strict analysis completion pipeline. `Swing High` and `Swing Low` remain `Catalog only`; no registry entry was promoted. The full 364-entry strict gate remains fail-closed, and live execution remains disabled.

## Validation limits

Rust unit tests were added in source but could not be executed unless `cargo` and `rustc` are installed. Node source-contract tests verify presence of key safety semantics; they do not prove the Rust module compiles or behaves correctly. Production use requires Rust test execution, Clippy, review, fixture-based validation, and native integration tests.
