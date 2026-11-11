# SMC price-indexing optimization — v0.4.9

## Problem

The previous liquidity-sweep and structure-break paths inspected every historical swing for every candle. That work scales with the product of candle count and confirmed-pivot count, even when most price levels are nowhere near the current candle's range. The output can also become large on long, choppy series.

## Design

- Confirmed swings use forward-sliding monotonic deques for left/right window extrema in O(n), are cross-checked against a naive oracle in native unit tests, and are activated by downstream detectors only when `confirmed_at_index < current_index`.
- Buy-side sweep candidates are queried strictly inside `(close, high)`; sell-side candidates are queried strictly inside `(low, close)`. Price keys preserve numeric ordering and normalize `-0.0` and `+0.0` to the same key.
- Bullish structure breaks query levels in `[previous_close, close)` only when the close rises; bearish breaks query `(close, previous_close]` only when the close falls.
- Break levels are removed from their corresponding active map after the first close-cross, preserving one-break-per-pivot semantics while keeping a swing high and swing low at the same pivot independent.
- Events on the same candle are sorted by confirmation index, pivot index, and swing kind, then classified using one pre-candle bias snapshot.
- Repeated wick/close-back-inside sweep observations remain available for the same pivot; this preserves existing semantics rather than silently turning sweeps into single-use events.
- Swing, sweep, structure-break, FVG, and FVG-lifecycle event outputs are bounded at one million entries. Exceeding the limit returns an error and never a partial success payload.

## Complexity model

Let `n` be candles, `p` active pivots/price levels, and `e` emitted events. Swing extrema scanning is O(n). Price-index insertion/query work is approximately O((n + p) log p + e), with range iteration proportional to crossed levels and a deterministic cap on emitted detector events. This does not make the entire product's 364-concept assessment pipeline production-ready, and the state timeline still allocates one snapshot per candle.

## Verification boundary

The repository's dependency-free Node tests assert source contracts and are not native Rust execution. Native unit tests include deterministic generated-candle comparison against a naive swing reference, price-key ordering, causal sweep examples, and multi-level structure-break regression cases. They must pass under `cargo test` in CI before the native change can be certified. Live execution remains disabled and research detector output does not promote catalog status.
