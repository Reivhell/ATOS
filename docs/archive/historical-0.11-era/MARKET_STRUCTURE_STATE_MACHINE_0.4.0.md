# Market Structure State Machine — 0.4.0

## Purpose

This increment adds a point-in-time structure timeline on top of the existing confirmed-swing and close-confirmed-break research primitives. It is not a trading signal, not a strategy backtest, and not a reviewed implementation of ICT terminology.

## Contract

- Output contains exactly one snapshot per input candle, in original timestamp order.
- Swing highs/lows become active only at their confirmation index, never at their historical pivot index.
- Structure breaks remain close-cross events; wick-only breaches are not accepted by the existing detector.
- Bias is `unknown`, `bullish`, or `bearish`, driven by directional close-confirmed breaks.
- Transition is `none`, `initial_break`, `continuation_break`, `reversal_candidate`, or `ambiguous_break`.
- If a candle breaks eligible levels in both directions, transition becomes `ambiguous_break`; the state machine freezes bias and protected levels instead of arbitrarily resolving event order.
- Protected high/low are explicit operational approximations: the latest confirmed opposing swing available before a directional break. They must not be presented as universal ICT definitions.
- Snapshot data contains active swing levels and protected levels for auditability.

## What this does not claim

- It does not label events as BOS, CHoCH, MSS, MSB, CISD, internal/external structure, or a trade setup.
- It does not promote any catalog item from `Catalog only`.
- It does not connect this research primitive to the strict decision gate or live execution.
- It does not prove Rust compilation: native tests must run in a Rust-enabled CI environment.

## Release acceptance still required

1. Run `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets --all-features -- -D warnings` on supported CI targets.
2. Add hand-authored golden fixtures for trend continuation, reversal candidate, multi-level break, conflicting same-candle breaks, tied pivots, and gaps in timestamps.
3. Perform independent review of the operational definition of protected levels.
4. Integrate only through a versioned, code-owned assessment implementation registry after those checks pass.
5. Keep decision gate fail-closed while any of the 364 entries remains unsupported or lacks evidence.
