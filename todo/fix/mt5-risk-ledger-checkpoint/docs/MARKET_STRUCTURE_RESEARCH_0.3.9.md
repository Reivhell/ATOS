# Market Structure Research Primitives — 0.3.9

## Scope

`research_smc::detect_confirmed_structure_breaks` is a deterministic, Rust-native, research-only primitive. It detects a close crossing a previously confirmed unique fractal swing high/low. Each event records the level pivot index, confirmation index, break index/time, close, direction, prior directional state, and conservative classification.

## Causality and semantics

- A swing level is eligible only when `confirmed_at_index < broken_at_index`.
- A bullish break requires previous close at/below the level and current close strictly above it. Bearish is symmetric.
- A wick through a level with no close-through is not emitted.
- Each pivot level is emitted at most once within a detector run.
- Classifications are `initial_break`, `continuation_break`, and `reversal_candidate`. They intentionally do not claim universal equivalence to BOS, CHoCH, MSS, or MSB; those labels vary by methodology and need a reviewed strategy-specific contract.
- The detector validates the entire candle series before calculating; it does not inspect future candles to assign an event to a historical bar. Swing confirmation itself requires the configured right-side bars.

## Non-goals and safety boundary

This is not a trade signal, not a full market-structure state machine, and not connected to the strict decision assessment pipeline. It does not calculate protected highs/lows, internal/external structure, dealing ranges, or strategy-specific BOS/CHoCH/MSS/MSB. Catalog availability remains unchanged. All specialist assessments remain mandatory; missing or catalog-only concepts still block handoff. Live execution remains disabled.

## Remaining production work

1. Compile and run Rust unit tests on a supported Rust toolchain.
2. Add golden fixtures across markets, timeframes, equal highs/lows, gaps, multi-level breaks, and alternate structure definitions.
3. Define a versioned strategy contract for BOS/CHoCH/MSS/MSB before naming the observations as those concepts.
4. Integrate reviewed output with the native assessment contract and provenance checks only after independent review.
