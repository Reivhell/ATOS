# 09 — Testing and Acceptance Criteria

## Test layers
- Export serialization: CSV quoting/embedded newlines, spreadsheet-formula neutralization, UTC timestamps, and versioned backtest JSON v2 output and preservation of data-provenance fields. Run `npm run test:exports` and `npm run test:provenance`; both suites use only Node built-ins. The provenance suite exercises an independent byte-format reference against a cross-language SHA-256 vector; the actual Rust implementation remains covered by Rust unit tests.
- Technical analysis: `npm run test:technical-analysis` checks catalog coverage/unique IDs, bounded implementation labels, known indicator behaviors, insufficient history, and malformed-candle fail-closed behavior. These tests do not validate all SMC/ICT interpretations or establish profitability.
- Unit tests: calculations, risk rules, state transitions, scoring features.
- Contract tests: adapters against normalized schemas.
- Integration tests: market data → strategy → risk → paper execution.
- Replay tests: deterministic event replay and backtest reproducibility, including an assertion that changing bar opens while keeping closes fixed changes fills/results.
- Failure tests: disconnects, stale data, duplicate events, timeouts, database failures.
- Security tests: secret redaction, permissions, untrusted tool output, prompt injection.
- UI tests: critical journeys, error states, keyboard navigation.
- Screener validation: historical replay, false-positive analysis, source freshness, risk flags.

## MVP acceptance criteria
1. App displays provider health and market-data freshness.
2. One strategy can run against historical and streaming/paper data.
3. Same strategy version, parameters, and dataset reproduce the same backtest within defined numerical tolerances; completed-candle signals fill no earlier than the next bar open.
4. Every order intent has a recorded risk decision.
5. Risk rejection prevents the paper broker from receiving the order.
6. A paper order can move through valid state transitions and produce fills/positions.
7. Agent tool calls are allowlisted, time-bounded, and auditable.
8. Agent stop action prevents future tool execution after shutdown completes.
9. Screener rows show momentum score, risk score, timestamps, sources, and missing-data states.
10. A stale or failed screener provider is visible; stale values are not presented as fresh.
11. No secret appears in normal logs or UI.
12. Live execution is unavailable unless explicitly implemented, configured, and confirmed.
13. Audit CSV exports reflect the visible filter and neutralize formula-like text; backtest JSON v2 exports preserve result parameters, curve, caveats, and the SHA-256 fingerprint/time range for canonical candle inputs. The report clearly states that raw candles are not embedded and source authenticity is not attested.

## Definition of done for each feature
- Requirements and failure behavior documented.
- Unit and integration tests added.
- Structured logs and useful error messages present.
- UI loading, empty, stale, and error states handled.
- Security and permission implications reviewed.
- Documentation updated.


Backtest acceptance also requires a genuine fast/slow moving-average cross (not merely an already-positive fast-minus-slow state), next-bar-open execution, and rejection of non-finite calculations caused by extreme OHLCV values. Regression tests cover these cases.

Research provenance acceptance: the same canonical candle set must generate the same SHA-256 fingerprint across supported operating systems, while changing any timestamp/OHLCV field changes the fingerprint. The fingerprint encoding version must be explicit and must never be described as a provider signature.

Technical-analysis acceptance: the UI must clearly separate calculations available in this version from concepts that are catalog-only; indicators must not fabricate values when source bars are insufficient or malformed; the page must not generate or submit orders.
