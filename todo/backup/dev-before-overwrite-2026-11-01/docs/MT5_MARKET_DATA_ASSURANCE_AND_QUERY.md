# MT5 Market Data Assurance & Query Layer

**Repository line:** 0.10.1 (version unchanged)
**Status:** Implemented source; native Rust compilation and live MT5 verification remain release gates.

## Purpose

The durable MT5 store can now be queried as a bounded, account-scoped historical dataset. The query path is designed for operator inspection and later research/backtest integration. It does not submit orders and it does not turn broker data into a trading recommendation.

## Query contract

The native Tauri command `query_mt5_market_data_series` accepts:

- `brokerServer` and `accountLogin` to select an exact broker/account namespace;
- `symbol` and `timeframe` to select one series;
- `fromMs` and `toMs` as inclusive stored MT5 timestamp milliseconds. The desktop window selector anchors `toMs` to the last authenticated `terminal.serverTimeMs`, not the workstation clock, to avoid silently clipping data when broker and local clocks differ;
- optional `afterMs` for keyset pagination;
- `limit` from 1 to 5,000.

The allowed range is capped at 366 days and epoch-millisecond values must remain within JavaScript's exact safe-integer range. Pages are returned oldest-first. A page returns `hasMore` and `nextAfterMs`; the next request must retain the same range and series and pass the previous cursor. The desktop UI uses pages of 1,000 bars. Native queries are additionally bound to the last authenticated MT5 broker server and account login held by the bridge runtime; callers cannot select an arbitrary account namespace through the query command.

The backend computes the coverage and quality summary across the complete requested range, not merely the current page. The data page, coverage aggregate, revision count, ordered dataset manifest and interval analysis use a single SQLite deferred read transaction so concurrent ingestion cannot produce a mixed-snapshot response. Pagination requests are separate transactions, so the UI compares each page's dataset manifest with the first loaded page and refuses to append when ingestion or a broker correction changed the dataset between requests. The response includes a deterministic SHA-256 manifest over broker/account/series/range identity and ordered stored bar fingerprints. This manifest is a reproducibility identifier, not a digital signature, HMAC or proof against offline database tampering.

## Data assurance

Every returned row is checked again before export:

1. Numeric values and OHLC invariants are revalidated.
2. Stored integer counters and receipt timestamps are checked for invalid/negative values.
3. The SHA-256 payload fingerprint is recomputed from the stored bar values and compared with the persisted hash.
4. Corrupt records cause the query to fail closed; they are not silently omitted or normalized. Integer values outside JavaScript's exact safe-integer range are also rejected rather than exported with precision loss.

Revision counts are read from `mt5_bar_revisions`, where each observed broker correction retains prior and replacement fingerprints. The current bar row exposes its revision counter, first/last receipt timestamps, and payload SHA-256.

## Interval anomaly semantics

For fixed-duration MT5 timeframes (M1 through W1), the query calculates the elapsed time between adjacent stored bar-open timestamps. A discontinuity greater than 1.5 nominal timeframe intervals is returned as an **unclassified interval anomaly candidate**. The response includes the previous and next bar timestamps, elapsed milliseconds, and the number of nominal intervals elapsed. At most 64 candidate examples are returned while the total count covers the full query window.

These are intentionally not called confirmed missing candles. The current data model does not include each broker's symbol trading sessions, holidays, daylight-saving transitions, or special trading schedules. Weekend closures and normal market breaks can therefore appear as candidates. No gap repair, synthetic candle insertion, or backtest fill-forward occurs.

`MN1` is a calendar-month timeframe and has no fixed millisecond duration. It is therefore rejected by this fixed-interval audit/query endpoint rather than approximated as 30 days.

## Operational boundary

- The dataset is scoped by broker server, account login, symbol and timeframe.
- Reads are local desktop Tauri commands; they do not make broker network requests.
- This layer does not yet create a broker-session calendar or a certified completeness statement.
- This layer does not yet feed the existing backtest engine. A later adapter must preserve source identity, hash/revision provenance, interval caveats and the exact selected time range when constructing a backtest input.
- The UI formats timestamps using the device locale, while stored broker-provided numeric timestamps are preserved unchanged. No broker timezone/session calendar normalization is applied. The EA remains read-only and has no order submission or position mutation capability.

## Verification status

The Node static contract tests verify range bounds, keyset pagination, read-snapshot use, payload revalidation, session caveats, and UI command wiring. Rust unit tests cover oldest-first pagination, full-window interval anomaly calculation, invalid bounds/limits, calendar-month rejection, and refusal to export tampered bar values. Rust tests were authored but are not claimed as executed because the current build environment does not contain Cargo/Rust. Svelte/Tauri production compilation and real terminal integration remain outstanding gates.
