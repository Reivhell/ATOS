# 06 — Data Model and Event Contracts

## Core entities
- `MarketInstrument`: id, venue/chain, symbol/address, base/quote, precision, metadata.
- `MarketEvent`: event_id, instrument_id, event_type, event_time, received_at, source, sequence, payload.
- `StrategyDefinition`: id, name, version, runtime, parameters, required_data, permissions.
- `StrategyRun`: id, strategy_version, mode, start/end time, config hash, status.
- `AgentDefinition`: id, objective, provider/model, tool_allowlist, budget, action_mode.
- `AgentRun`: id, agent_id, start/end, status, trace_id, token/cost usage.
- `Signal`: id, source_id, instrument_id, direction/type, strength, created_at, evidence_refs.
- `OrderIntent`: id, idempotency_key, origin, instrument_id, side, order_type, quantity, price/limits.
- `RiskDecision`: id, order_intent_id, approved, rule_results, reason, timestamp.
- `Order`: id, venue_order_id, state, requested/filled quantity, average price, timestamps.
- `Fill`: id, order_id, quantity, price, fee, timestamp.
- `Position`: instrument_id, quantity, average entry, realized/unrealized PnL.
- `ScreenerSnapshot`: token/pool identifiers, observed_at, source, feature values, scores, risk flags.
- `BacktestResult` (implemented): run ID, strategy parameters, metrics, equity curve, caveats, data source, canonical OHLCV SHA-256 fingerprint, candle range, and median candle interval.
- `AuditEvent`: actor, action, target, timestamp, trace_id, outcome, redacted details.

## Backtest dataset fingerprint v1

The implemented result contract uses `datasetSha256`, `datasetEncodingVersion`, `datasetStartMs`, `datasetEndMs`, and `medianCandleIntervalMs`. Encoding version 1 hashes the UTF-8 domain separator `agentic-trading-os.ohlcv.canonical.v1` followed by a NUL byte, the candle count as an unsigned 64-bit little-endian integer, then every candle in sequence: timestamp as signed 64-bit little-endian, followed by `open`, `high`, `low`, `close`, and `volume` as little-endian IEEE-754 binary64 bit patterns. The SHA-256 output is lowercase hexadecimal.

The fingerprint is a content identifier for matching datasets, not a signature or proof of provider authenticity. The JSON report contains provenance metadata but not raw candles, so it is not a complete offline replay bundle.

## Event naming
Use stable, versioned event names such as:
- `market.candle.v1`
- `market.trade.v1`
- `strategy.signal.v1`
- `agent.run.started.v1`
- `agent.tool.called.v1`
- `order.intent.created.v1`
- `risk.decision.v1`
- `order.state.changed.v1`
- `execution.fill.v1`
- `screener.snapshot.updated.v1`
- `system.provider.health.v1`

## Event envelope
Every event should include:
- unique event ID;
- schema version;
- event time and receive time;
- source/component;
- correlation/trace ID;
- payload;
- optional sequence or deduplication key.

## State rules
- Use explicit order-state transitions; reject illegal transitions.
- Preserve raw provider payloads selectively for debugging, with secrets and personal data excluded.
- Record the exact strategy version and configuration hash for every run.
- Store timestamps in UTC; render local timezone in UI.
- Use decimal/fixed-point representations for prices and quantities where practical; avoid binary floating-point for money accounting.


## Candle time and receipt semantics

`Candle.time` is the UTC bar-open timestamp. For a requested interval, the native domain derives the inclusive close timestamp as `bar_open + interval_duration - 1ms`; this is the market-time basis used for freshness, not cache-ingest time. The engine persists `candle_retrieved_at_ms` separately from quote/provider refresh time and returns both `latestClosedBarAtMs` and `retrievedAtMs`. Live eligibility requires the latest fully closed bar to be no older than one interval plus 30 seconds and the candle receipt to be no older than 120 seconds. A stale replay cannot renew the receipt, forming/future bars cannot be treated as closed, and a missing candle response does not relabel the prior cache as newly received. Legacy persisted state without a candle receipt remains cached-only until a new valid candle batch is received.

Quote timestamps follow the same provenance rule. `Quote.updated_at_ms` is the exchange-reported observation time (`closeTime` for Binance's 24-hour ticker response, whose JSON timestamps are milliseconds), while `Quote.retrieved_at_ms` is stamped by the native engine only after it accepts the provider batch. A quote is live only while both timestamps and the enclosing provider session remain fresh. The engine rejects zero, stale, future-dated, and out-of-order exchange observations without replacing the previous quote; a legacy quote with no receipt timestamp remains cached-only until a new validated observation arrives. Order admission, portfolio-mark checks, strict state-commit validation, and UI live labels all require both clocks.
