# Research SMC Detectors — v0.3.5

## Added: confirmed swing classification

`classify_confirmed_swings(candles, left_bars, right_bars)` derives confirmed swings from validated candles, then compares each swing only with the previous confirmed pivot of the same kind.

- High pivots yield `initial_high`, `higher_high`, `lower_high`, or `equal_high`.
- Low pivots yield `initial_low`, `higher_low`, `lower_low`, or `equal_low`.
- Each record contains the source pivot index/time, first known confirmation index/time, compared prior pivot (if any), and prices on both sides of the comparison.
- Classification is available at the confirmation bar, never retroactively at the pivot bar. Equal prices are explicit rather than incorrectly labelled HH/LH/HL/LL.
- The function accepts candle data and window parameters rather than caller-supplied swing records, reducing forged-reference risk. It reuses strict OHLCV/timestamp validation and bounds.

## Interpretation limits

This is a geometric relative-swing classifier only. It does not determine trend regime, internal/external structure, protected highs/lows, dealing ranges, BOS, CHoCH, MSS, MSB, CISD, or a trade setup. Different market participants use different structural conventions; those semantics need separate versioned definitions and tests.

## Gate and status

This primitive remains research-only and is not wired to the strict analysis completion pipeline. `Swing High` and `Swing Low` remain `Catalog only`; no registry entry was promoted. The full 364-entry strict gate remains fail-closed, and live execution remains disabled.

## Validation limits

Rust unit tests were added in source but have not been executed here because `cargo` and `rustc` are unavailable. Node source-contract tests check that key Rust contracts exist; they do not prove native compilation or runtime behavior. Production use requires native Rust tests, formatting/Clippy, broad fixture review, strict-pipeline integration tests, dependency lockfiles, frontend build, and independent review.
