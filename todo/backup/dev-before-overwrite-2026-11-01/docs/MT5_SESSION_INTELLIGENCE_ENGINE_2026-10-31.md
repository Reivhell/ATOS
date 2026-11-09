# MT5 Broker Session Intelligence — 2026-10-31

## Implemented architecture

- Protocol v4 transports broker-native `SymbolInfoSessionTrade` windows for all seven weekdays and every authenticated watched symbol on initial connection/reconnection and scheduled refresh. Unchanged heartbeats omit the schedule payload only when the durable store can prove that a complete, fresh calendar already exists for every subscribed symbol; missing or stale retained state fails closed.
- The EA bounds the per-day schedule to 16 sessions and withholds the complete telemetry frame if it detects an unsupported overflow or an unexpected API error. It refreshes the schedule on a configurable 30–3,600 second interval (default 300 seconds), reuses the last verified capture between refreshes, and includes the UTC capture instant separately from broker-wall-clock session boundaries. It sends the cached schedule once after each successful authentication and only sends it again on refresh, avoiding repeated large JSON payloads on every one-second heartbeat. The receiver rejects a future-dated or more-than-one-hour-old capture. It does not advance local tick/bar cursors when the schedule cannot be captured.
- Rust enforces exact symbol-subscription coverage, exactly seven ordered weekdays, contiguous session indexes, valid seconds-from-midnight boundaries, and unique symbols.
- SQLite schema v3 adds a current session-calendar projection and append-only revision history with SHA-256 fingerprints. Calendar updates share the market-data `BEGIN IMMEDIATE` transaction and durable frame receipt.
- Reads verify the current schedule payload and the complete contiguous predecessor/revision fingerprint chain.
- Dataset manifests include the current symbol-specific calendar fingerprint. If no calendar exists, the manifest includes an explicit `SESSION-CALENDAR-UNKNOWN` commitment.
- Tauri exposes an account-identity-bound calendar query. The MT5 panel renders weekly sessions, overnight windows, revision, and fingerprint.

## Deliberate safety boundaries

The MQL5 API provides recurring weekday session windows, not a full dated holiday calendar or a timezone identifier. ATOS therefore preserves the broker-server wall-clock values and does not convert to UTC, guess DST, or invent holidays. The gap detector remains conservative: it reports interval anomalies as unclassified candidates, never as confirmed missing tradable candles. A later dated-exception subsystem must have explicit provenance, broker/server scope, effective date ranges, and immutable revisions before it can affect continuity classification.

The EA remains read-only. This work does not add order placement, amendment, cancellation, close, position sizing, or AI-to-broker execution.

## Validation status

Source-level Node contract tests cover protocol v4, schedule completeness and bounds, persistence/revision tables, append-only history, dataset provenance binding, and Tauri query wiring. Rust unit tests were added for calendar revision semantics, dataset fingerprint changes, invalid/incomplete weeks, overnight sessions, and migration. Native Rust execution and MetaEditor compilation require toolchains not present in the build environment and are not claimed as passed.

## Platform reference

The adapter follows the MQL5 `SymbolInfoSessionTrade` contract: session indexes are enumerated from zero, and returned `from`/`to` values represent seconds from midnight with the date component ignored. The API returns `false` when the requested session entry is unavailable; the terminal's documented `ERR_MARKET_SESSION_INDEX` (`4307`) is treated as the normal end-of-list sentinel during bounded enumeration. References: [SymbolInfoSessionTrade](https://www.mql5.com/en/docs/marketinformation/symbolinfosessiontrade) and [MQL5 runtime error codes](https://www.mql5.com/en/docs/constants/errorswarnings/errorcodes).
