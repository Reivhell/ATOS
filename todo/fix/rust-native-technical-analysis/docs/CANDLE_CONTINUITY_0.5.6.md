# Native Candle Continuity Gate — v0.5.6

## Risk addressed

The previous backend path could sort candles and deduplicate timestamps before saving them. That normalization could hide an out-of-order or duplicated provider row, and a median interval check could accept a series with an internal gap. The frontend already rejected gaps when rendering, but native consumers need their own authoritative boundary.

## New contract

A candle series is accepted only when all of the following hold:

- At least 30 closed candles are present.
- Every OHLCV field is finite; prices are positive; volume is non-negative; and high/low contain the complete candle range.
- Every open timestamp is aligned to the UTC grid for the requested interval.
- Each adjacent timestamp differs by exactly one interval, computed with checked subtraction. Duplicate, unordered, missing, or irregular bars fail closed.
- The Binance parser rejects malformed rows rather than silently skipping them. Only a single still-forming candle at the very end of the provider response is excluded.

The Rust parser, native cache read, snapshot capability, and frontend chart boundary enforce this policy. If candle validation fails after quotes are valid, quotes may still refresh, but the candle series is withheld and a warning is written to the audit trail. No repaired/partial candle series reaches charts, indicators, strict-analysis work plans, or backtests.

## Market-session limitation

This cadence policy is intentionally for configured continuous crypto candles (currently BTCUSDT 5m). Do not reuse it for session-based instruments without a trusted venue calendar and explicit holiday/session semantics.

## Verification boundary

Node source/model tests run in the current workspace. Rust unit tests are added for aligned continuous series, internal gaps, duplicates, ordering, off-grid timestamps, malformed OHLCV, short history, parser gap rejection, and parser order/duplicate rejection. The Rust toolchain is unavailable in the current workspace, so those tests remain authored but unexecuted until native CI passes.
