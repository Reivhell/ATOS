# Market-data assurance and anomaly audit — 0.6.0

## Purpose

The analysis engine now independently audits the temporal contract of its own input, even when an adapter has already validated the series. It also exposes bounded, deterministic warnings for a small set of possible latest-candle anomalies. This is defense in depth; it is not a claim that any public data provider is authentic or correct.

## Canonical timeframe contract

`src/lib/domain/market-data/timeframe-contract.js` is the one JavaScript source of truth for supported timeframes and their UTC interval widths: `1m`, `5m`, `15m`, `1h`, `4h`, and `1d`. The market-data UI boundary, standalone analysis engine, and opt-in Bun/Hono validator import this contract instead of maintaining separate interval tables.

`getLatestClosedBarAtMs()` derives the canonical observation cutoff as `bar_open + interval - 1ms`. The analysis engine uses this close timestamp for its identity and evidence `asOf`; the immutable market-data application boundary supplies the same value from the native response, and the Hono API derives it from the requested timeframe. A caller-provided close timestamp that differs from the final candle interval blocks the report. The local provider receipt timestamp remains a separate ingestion-time field and is never substituted for market observation time. The native `CandleSeriesResponse` also carries `datasetSha256`, computed in Rust from the exact returned OHLCV vector using the canonical v1 byte encoding shared with backtest and strict analysis. The frontend validates the lowercase SHA-256 envelope and preserves the digest in the immutable snapshot; it never synthesizes a replacement hash.

`auditCandleSeries()` verifies, within a hard 10,000-candle bound:

- OHLCV fields are finite and prices/volume obey the positive-price, non-negative-volume, and high/low range invariants.
- Every timestamp is a positive safe integer in milliseconds and aligned to the timeframe's UTC grid.
- Timestamps are strictly increasing and every adjacent candle is exactly one supported interval apart. No sorting, deduplication, gap filling, or interval inference occurs.
- Fault recording is capped at 64 entries, preserving bounded output even for adversarial malformed inputs.

A hard violation makes the analysis envelope `invalid_data`, suppresses numeric indicator metrics and directional signals, and leaves the decision gate fail-closed. The boundary audit complements, rather than replaces, the native Rust candle validator.

## Latest-bar anomaly warnings

The audit checks only the most recent bar against a preceding baseline; it does not rewrite candle history. It currently looks for:

- A close-to-close log return of at least 3% that is at least eight robust deviations from the prior return baseline. Scale uses median absolute deviation with a conservative nonzero floor.
- Latest volume at least 8× the preceding 20-candle median and at least eight robust deviations above that baseline.
- Twenty consecutive bars with identical open/high/low/close values.

These heuristics yield `warning` status, not `invalid` status. A market shock may be real, so the engine exposes the warning and metrics instead of silently discarding the candle. The warning pauses candidate handoff until the warning no longer applies; it does not claim that the candle is corrupt. The audit is not a provider-authenticity verdict, a manipulation detector, or a probability-of-loss estimate. Thresholds are explicit constants in source and require instrument/venue-specific calibration before use as automated risk rules.

## Output contract

The analysis envelope is `technical-analysis-envelope.v2`, with `dataQuality.audit` carrying:

- `status`: `clean`, `warning`, or `invalid`;
- structured `hardIssues` and `warnings` with stable reason codes;
- timeframe, interval width, candle count, first/last timestamps and expected span;
- latest close-return percentage, robust return deviation, and latest-volume/median ratio when computable.

Existing `dataQuality.issues` also includes human-readable audit reasons for the current dashboard. Advisory anomaly warnings change the engine status to `partial` and force the research verdict to `BLOCK` until a warning-free snapshot is supplied. Data validity remains `true` because the warning is not proof of corruption; status and handoff policy are separate.

## Application research eligibility

The technical-analysis application service requires an exact immutable snapshot, matching symbol/timeframe, a Rust-issued dataset fingerprint, and a live provider-backed candle snapshot before it marks market data as decision-eligible. A historical or stale snapshot may still be inspected for context, but its decision layer remains blocked. This is a research-only eligibility check: it does not mint a native risk receipt, authorize a paper-order transition, or grant live execution authority.

## Safety boundaries

- This feature gives no provider authentication, dataset signature, Rust risk receipt, candidate handoff, order intent, or live-execution authority.
- The JavaScript engine is not the authoritative native risk gateway.
- The checks target continuous UTC-grid candle series. Session-aware instruments need a trusted venue calendar and a separately reviewed temporal contract.
- Tests are deterministic Node regression tests; they do not replace Bun/Hono runtime tests, Svelte production build, Rust/native execution, or OS desktop acceptance.
