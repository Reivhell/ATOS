# Research SMC detectors — v0.3.3

## Scope added

This iteration extends the v0.3.2 Rust-only research module with `assess_fvg_lifecycle(candles, gaps)`. It returns chronological observations for each supplied FVG:

- `partial_fill`: a post-formation candle intersects the zone without reaching the far boundary.
- `full_fill`: a post-formation candle reaches or passes the far boundary. A full fill suppresses a redundant partial-fill event for that same gap.
- `inversion_candidate`: a post-formation candle closes beyond the far boundary. This is intentionally not named or treated as a confirmed IFVG/reclaimed setup.

Events include direction, formation index, observation index/time, and zone bounds. The function validates candle data and rejects invalid gap bounds, direction, formation index, or formation timestamp. It only examines indices strictly greater than the formation index and sorts the resulting events by observed index.

## Important semantic limits

- Fill behavior is a declared wick-intersection convention; other strategies may use body-only, close-based, or mitigated-at-midpoint definitions.
- The implementation emits at most one partial-fill event, one full-fill event, and one inversion-candidate event per gap. It is an event summary, not a full per-candle state machine.
- An `inversion_candidate` is not proof of a valid IFVG, trade setup, market intent, or future price direction.
- The lifecycle evaluator is not connected to the native strict-assessment pipeline and cannot promote catalog entries or authorize a decision.
- FVG remains `Catalog only` pending native execution, known-vector and adversarial validation, broader market/timeframe coverage, independent review, and an explicit registry change.

## Verification boundary

Node static tests inspect the source contract only; they do not compile Rust or execute the Rust unit tests. The environment used for this artifact has no `cargo`/`rustc` and no installed `svelte-check`. See `VALIDATION_REPORT_0.3.3.md`.
