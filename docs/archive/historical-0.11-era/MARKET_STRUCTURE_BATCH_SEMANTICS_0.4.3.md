# Market-structure event batch semantics — v0.4.3

## Problem corrected

The previous research detector processed eligible structure levels one by one and changed directional bias inside the event loop. If one candle close crossed multiple confirmed swing levels in the same direction, the first event could be labeled `initial_break` while a later event from the same candle became `continuation_break`. That label depended on event iteration order, not on a new market observation.

## Contract now enforced

1. Eligible swing levels must be confirmed strictly before the breaking candle.
2. A break requires a close crossing from the prior close through the level; wick-only breaches are not accepted.
3. The detector gathers all eligible levels for a candle before updating bias.
4. Every event in the candle batch uses the same pre-candle bias snapshot.
5. Distinct crossed levels are preserved as distinct events.
6. Bias updates once after the batch, not once per event.
7. The detector remains research-only. Its `initial_break`, `continuation_break`, and `reversal_candidate` labels are operational labels and are not automatically equivalent to BOS, CHoCH, MSS, or MSB.

## Regression coverage

A Rust unit test constructs two separately confirmed swing highs and a later candle that closes above both. It asserts that both break events are retained, both have the same prior bias, and both are classified against that pre-candle state. The test is authored but must be executed with `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` in native CI; source-contract tests are not a substitute.

## Scope and limitations

This change fixes event-order-dependent classification in the research detector. It does not implement CISD, protected-level strategy semantics, internal/external structure, liquidity, order blocks, or the complete 364-concept assessment pipeline. No master-list availability status is promoted. The strict gate remains fail-closed and live trading remains disabled.
