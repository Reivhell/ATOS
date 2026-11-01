# Causal prefix-invariance contract — v0.5.0

## Purpose

A causal research detector must not rewrite an event that was already observable merely because additional future candles are appended. Confirmation indices alone are useful but insufficient: downstream classification, event batching, lifecycle logic, or an optimization could still accidentally depend on later rows.

## Contract

The native Rust test `research_detector_outputs_are_prefix_invariant_and_never_rewrite_past_events` generates a deterministic 96-candle OHLCV sequence and compares every prefix of length 3 through 96 against the full-series output. It checks:

- confirmed swings filtered by `confirmed_at_index < prefix_len`;
- FVGs filtered by `formed_at_index < prefix_len`;
- FVG lifecycle observations filtered by `observed_at_index < prefix_len`;
- liquidity sweeps filtered by `swept_at_index < prefix_len`;
- structure breaks filtered by `broken_at_index < prefix_len`.

The prefix run must equal the filtered full-series output exactly, including event identity, order, provenance indices, prices, and classifications. Any mismatch fails the native unit test.

## Scope and limitations

- This is a deterministic property-style regression, not a proof over all possible price sequences.
- The test is authored in Rust and included in `cargo test`; the current work environment has no `cargo` or `rustc`, so execution must be confirmed by native CI.
- Existing Node tests only verify the source contract contains the native test; they do not execute Rust logic.
- The detector family remains research-only. This contract does not validate strategy profitability, make a catalog-only method implemented, enable specialist dispatch, or authorize live orders.
