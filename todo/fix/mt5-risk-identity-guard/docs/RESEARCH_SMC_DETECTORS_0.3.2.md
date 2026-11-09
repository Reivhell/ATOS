# Research-only SMC detectors — v0.3.2

## Scope

This release adds two deterministic Rust research primitives in `src-tauri/src/research_smc.rs`:

1. **Three-candle FVG geometry** — bullish when candle `i` low is above candle `i-2` high; bearish when candle `i` high is below candle `i-2` low. The event becomes observable at the close of candle `i`. The output includes zone bounds, midpoint, width, formation index, and timestamp. It does not infer later fill/reclaim status.
2. **Confirmed-pivot liquidity sweep** — detects a wick crossing a unique swing high/low and a close back inside that level. A pivot cannot be used until `pivot_index + right_bars`; a sweep must occur strictly after confirmation. This is a price-pattern proxy and does not prove actual resting orders, stop placement, participant intent, or a literal stop hunt.

## Input and temporal contract

- Minimum 3 candles, maximum 2,000,000 candles.
- OHLCV values must be finite; volume must be non-negative; OHLC relationships must be valid.
- Candle timestamps must be strictly increasing.
- Pivot left/right windows must be positive.
- No event may use a pivot before its confirmation index.
- The detector is a research primitive only. It does not make trade recommendations or authorize orders.

## Status and promotion policy

The master-list status for `FVG (Fair Value Gap)` and `Liquidity Sweep` remains **Catalog only**. These functions are not yet wired into the authoritative assessment pipeline and have not been validated with native Rust execution in this environment. Promotion requires `cargo test`, Clippy, known-vector and adversarial fixtures, cross-symbol/timeframe validation, code review, and explicit code-owned registry changes. The existence of these functions does not satisfy the strict decision gate.

## Known limitations

- FVG definition is a transparent three-candle geometric convention; community/strategy definitions may differ.
- Sweep detection uses confirmed fractal pivots and a close-back-inside condition. It is not equivalent to proving a stop run or liquidity event in the order book.
- Equal highs/lows, gap fill state, IFVG, BPR, displacement, order blocks, sessions, and strategy-context filters are not implemented here.
- No detector output is currently emitted as an item-level assessment to the Rust decision gate.
