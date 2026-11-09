# MT5 Instrument Specification Registry

## Scope

Protocol v3 extends the authenticated MT5 snapshot with broker-reported contract metadata for the primary watched symbol. The Rust market-data transaction stores the current specification and an immutable revision row. Repeated heartbeats with an identical canonical JSON fingerprint update only `last_observed_at_ms`; a material change increments the revision and records both previous and replacement SHA-256 fingerprints.

Captured fields include digits, point, contract size, calculation mode, trade mode, execution mode, filling/order flags, tick size and profit/loss tick values, volume min/max/step/limit, stops/freeze levels, base/profit/margin currencies, swap mode and long/short swap values.

## Data integrity

- Specification persistence shares the same SQLite `BEGIN IMMEDIATE` transaction as the frame receipt and market-data writes. ACK therefore cannot precede specification commit.
- The registry key is `(broker_server, account_login, symbol)`; specifications are not shared across brokers or accounts.
- Validation rejects non-finite values, inconsistent volume constraints, malformed currency identifiers and out-of-range enum/flag values.
- A changed specification is append-audited. The current row is a projection; revision history is the record of observed broker changes. SQLite triggers reject updates and deletes to the revision table, and reads verify contiguous revision numbers, predecessor hashes, payload hashes, and agreement with the current projection.
- Profit/loss tick values are broker-calculated account-currency valuations that can change with conversion rates. They refresh in the current projection but are excluded from the static contract fingerprint, preventing false contract revisions on ordinary price/FX movement.
- The instrument-registry phase introduced additive schema v2. The subsequent session-intelligence phase advances this to schema v3 without rewriting existing candle/tick payloads.

## Completeness limitation

The current bridge captures the primary quote symbol's specification only. Other subscribed symbols may be ingested as ticks/bars but do not yet have a specification registry row until selected as the primary symbol. The API marks records `broker_snapshot_v1_partial`; they must not be treated as a complete broker contract profile or used alone for pre-trade margin/profit calculations. Calendar/session metadata, expiration, margin rates, and broker-specific contract overrides are not included yet.

Protocol v3 is intentionally incompatible with the earlier protocol-v2 EA. Install the repository's `MQL5/Experts/ATOS_Bridge.mq5` and reconnect. Do not pair a v2 EA with a v3 application.

## Verification boundary

Node contract tests validate the cross-layer source contract. Rust unit tests cover persistence, revision behavior, and validation but require Cargo to execute. MetaEditor compilation and broker-terminal verification are still required before any production deployment. The EA remains read-only and contains no order mutation API.


## Protocol v4 session intelligence extension

Protocol v4 adds one complete seven-day `SymbolInfoSessionTrade` schedule for each authenticated watched symbol. Schedules are validated for weekday ordering, contiguous session indexes, boundary ranges, symbol scope, and bounded per-day session counts. Current calendars and immutable revision rows are committed atomically with the frame receipt, ticks, bars, and instrument metadata. The dataset manifest incorporates the matching symbol calendar fingerprint, or an explicit unknown-calendar marker.

The calendar is deliberately represented in broker-server wall-clock space. No UTC offset, IANA timezone, daylight-saving transition, holiday, or exceptional closure is inferred. Consequently, the current gap detector continues to report candidates rather than confirmed missing candles. The UI exposes the recurring schedule and its fingerprint but does not claim a holiday-aware completeness certificate. Protocol v4 is incompatible with protocol-v3 EAs; install and compile the repository EA before reconnecting.
