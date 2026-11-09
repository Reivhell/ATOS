# Research Decision Engine — v0.5.8

## Contract

`evaluateResearchDecision()` accepts one validated market-data context, a set of structured domain signals, and an explicit intent. It returns `research-decision.v1`. It is deterministic, synchronous, side-effect-free, and does not send orders.

Each signal must identify a unique registered domain (`trend`, `momentum`, `structure`, or `location`), a valid direction (`bullish`, `bearish`, `neutral`), a heuristic signal strength in `[0, 1]`, unique evidence references, the exact UTC cutoff, and the exact SHA-256 dataset fingerprint. Signal strength is not a probability of profit or a calibrated confidence interval.

## Prerequisites

- Explicit supported intent: `research` or `paper_trade`; `live_trade` is rejected.
- Explicit data validation status, provider/source provenance, symbol, timeframe, UTC ISO-8601 cutoff, and canonical dataset SHA-256. The current desktop analysis path accepts this fingerprint only from the immutable native candle response and requires its provider snapshot to remain live at the application boundary.
- Unique signal domains. One domain cannot cast multiple votes to inflate coverage.
- Signals must share the exact snapshot cutoff and dataset fingerprint.
- At least three independent domains and three directional domain votes are required before a directional research candidate is considered. Neutral/unknown signals do not vote.
- At least two-thirds of directional domains must agree, mean heuristic strength must meet the policy threshold, and the weighted score difference must be at least 2.
- `paper_trade` additionally requires a passing native Rust risk-gateway result and a traceable receipt identifier. This JavaScript helper is still not authoritative even if those fields are supplied; native Rust must independently revalidate.

## Verdicts

- `BLOCK`: malformed/missing prerequisites or invalid provenance/evidence.
- `ABSTAIN_INSUFFICIENT_EVIDENCE`: insufficient independent domains or directional coverage.
- `ABSTAIN_WEAK_EVIDENCE`: directional evidence falls below the strength threshold.
- `ABSTAIN_CONFLICTING_SIGNALS`: independent signals do not agree sufficiently.
- `RESEARCH_LONG_CANDIDATE` / `RESEARCH_SHORT_CANDIDATE`: a research candidate only; never a broker command.

Every result includes `orderIntent: null`, `executionAuthority: "none"`, and `liveExecutionAuthorized: false`.

## Known limitations

- The desktop analysis view now receives the Rust-computed fingerprint bound to the exact OHLCV response. Its research candidate remains advisory and never receives a native risk receipt or paper-order grant; `paper_trade` is still blocked unless a separate Rust risk-gateway receipt is supplied and independently revalidated.
- The rules are deterministic heuristics; the thresholds have not been calibrated or independently validated on held-out markets. They must not be described as a validated strategy, investment advice, or probability of profit.
- Market-structure events and indicator readings are research evidence, not promoted implementations for the 364-entry strict master catalog.
- Native Rust backtest/risk and strict-completion commands remain the authoritative controls. No live execution is authorized.
