# ATOS MT5 Market-Data Assurance — Engineering Audit

**Repository baseline:** `0.10.1` (version deliberately unchanged)  
**Date:** 2026-10-31  
**Implementation status:** source changes complete for this tranche; release qualification is blocked pending native toolchains and broker-terminal verification.

## Executive summary

This tranche adds a bounded, account-scoped historical series query over the durable MT5 SQLite store. It adds dataset provenance, read-side integrity checks, full-range quality metrics, keyset pagination, and a Svelte workbench. A cross-page manifest check prevents the UI from combining page results from different dataset revisions when ingestion occurs between requests.

The implementation is not represented as a production-certified release. The repository's release gate remains fail-closed, Rust/Svelte/Tauri compilation could not be executed in this environment, and no live MetaTrader 5 terminal or broker account was available for integration testing.

## Implemented engineering changes

### 1. Bounded, account-scoped query contract

Native Tauri command: `query_mt5_market_data_series`.

- Query identity is bound to the last authenticated broker server and account login held by the MT5 bridge runtime; caller-supplied namespace changes are rejected.
- Series key is exact broker/account/symbol/timeframe plus inclusive `fromMs`/`toMs` range.
- Range maximum is 366 days; page limit is 1–5,000 bars.
- Epoch milliseconds and returned integer fields are rejected when outside JavaScript's exact safe-integer range.
- `MN1` is excluded from this fixed-duration endpoint because a calendar month cannot be modelled as a fixed millisecond interval.
- Results are ordered oldest-first and use `afterMs` keyset pagination.

### 2. Read-consistent dataset assurance

Coverage, revision count, ordered manifest calculation, interval analysis, and page retrieval run within one SQLite deferred read transaction. This avoids mixing aggregate results and bars from different database states within one response.

Each returned bar is revalidated before export:

- finite OHLC values and OHLC invariants;
- non-negative and representable integer counters;
- receipt timestamp ordering;
- stored SHA-256 syntax and recomputed payload fingerprint.

Corrupt rows are rejected rather than silently skipped or normalized.

### 3. Dataset manifest and pagination consistency

A deterministic SHA-256 manifest commits to the broker/account/series/range identity and ordered stored bar fingerprints. It is a reproducibility identifier, not a digital signature, HMAC, or proof against offline database tampering.

A SQLite read transaction is scoped to one request, not the entire multi-page browsing session. Therefore, every subsequent page returns its own manifest. The UI now compares that manifest to the first loaded page and refuses to append a page if ingestion or a broker correction changed the dataset between requests. The operator must restart the query to load a consistent view.

### 4. Quality analysis semantics

The query reports full-window bar count, first/last bar time, revision count, nominal interval, interval-anomaly count, maximum observed interval, and up to 64 candidate examples.

A discontinuity greater than 1.5 nominal fixed-timeframe intervals is reported as an **unclassified interval anomaly candidate**, not a confirmed missing candle. The current model does not contain broker-specific symbol sessions, holidays, daylight-saving transitions, or special trading schedules. Weekends and ordinary market closures can therefore appear as candidates. No synthetic bars or forward-filled data are created.

### 5. Frontend and runtime boundary

- Added typed Tauri adapter and series query workbench to `Mt5BridgePanel.svelte`.
- Query window end uses the authenticated terminal's reported broker `serverTimeMs`, not the workstation's local wall clock.
- The currently selected query is snapshotted for pagination; changing visible form fields does not rewrite the active query's cursor/range.
- Added the missing native-runtime capability identifiers for existing MT5 calls and the new series-query call.
- Querying is local to the desktop Tauri process and SQLite store; it does not call the broker or submit orders.

## Files changed in this tranche

- `README.md`
- `package.json` (new test script; version unchanged)
- `docs/MT5_MARKET_DATA_ASSURANCE_AND_QUERY.md` (query contract and operational caveats)
- `docs/ATOS_MT5_MARKET_DATA_ASSURANCE_AUDIT_2026-10-31.md` (this audit)
- `scripts/test-mt5-market-data-assurance-contract.test.mjs`
- `src/lib/adapters/tauri/bridge.ts`
- `src/lib/adapters/tauri/native-runtime-guard.js`
- `src/lib/components/Mt5BridgePanel.svelte`
- `src/lib/types.ts`
- `src-tauri/src/lib.rs`
- `src-tauri/src/mt5_bridge.rs`
- `src-tauri/src/mt5_market_data.rs`

The Expert Advisor remains at `MQL5/Experts/ATOS_Bridge.mq5` and remains read-only. No order submission or position/order mutation capability was added.

## Verification results

| Check | Result | Interpretation |
|---|---:|---|
| `npm test` | 292 passed / 0 failed across 44 test groups | Node regression suite passed; these are not Rust runtime tests. |
| `npm run typecheck:layers` | Passed | Existing TypeScript layer checks passed. |
| Isolated Tauri adapter TypeScript check | Passed | `bridge.ts` and `types.ts` checked with a temporary generic Tauri `invoke` declaration; this is not a full Svelte build. |
| `npm run verify:repo` | Passed with lockfile warnings | Static repository guardrails passed. |
| SQLite schema/query smoke check | Passed | SQLite accepted the current schema and the exercised coverage, revision, `LAG`, and keyset-page query shapes against seeded rows. This is not Rust integration execution. |
| MQL5 lexical scan | Passed | 828 lines scanned; strings/comments and structural delimiters balanced. |
| MQL5 order-mutation API scan | Passed | No `OrderSend`, `OrderModify`, `OrderDelete`, `PositionClose`, `PositionModify`, `PositionOpen`, or `CTrade` usage found. |
| ZIP archive integrity | Passed | `unzip -t` reported no compressed-data errors; 348 entries were inspected, including the EA and audit report. |

## Release blockers — not waived

`npm run release:preflight` remains **BLOCKED (9 issues)**:

1. `package-lock.json` missing.
2. `bun.lock` missing.
3. `src-tauri/Cargo.lock` missing.
4. `svelte-check` executable unavailable.
5. Vite executable unavailable.
6. Tauri CLI executable unavailable.
7. Cargo unavailable.
8. Rustfmt unavailable.
9. Cargo Clippy unavailable.

Consequently, Rust compilation/unit tests, Rust formatting/Clippy, full Svelte/Vite/Tauri build, and packaged desktop binary verification have not been completed. Rust unit tests were added/retained as source but are not claimed as executed.

A MetaEditor compiler and connected MT5 terminal/broker account were also unavailable. Real broker synchronization, broker-specific timestamps/session semantics, and terminal reconnect behaviour remain integration gates.

## Explicit non-goals and next gates

This query layer is not yet wired into the backtest engine. Backtest input construction must wait until session-calendar semantics, broker server-time normalization, data completeness rules, and provenance propagation are implemented and tested. Connecting the current unclassified interval candidates directly to the existing strict-continuity backtest would misclassify market closures or encourage unsafe synthetic fills.

Next engineering gates:

1. Pin dependency resolution with real npm/Bun/Cargo lockfiles generated from the intended toolchains.
2. Run Rust compilation, unit tests, formatting and Clippy; run Svelte check, Vite build and Tauri packaging.
3. Compile the EA in MetaEditor and run a fault-injected terminal integration suite (disconnect during frame, process restart, duplicate ACK, sequence discontinuity, stale cursor, broker bar correction, account switch and market closure).
4. Add broker/session-calendar capture and historical offset treatment before classifying intervals as confirmed data gaps or feeding this source into backtests.
5. Only then design the backtest adapter to preserve broker identity, selected range, per-bar fingerprints, revision history and manifest.

**Release decision:** source tranche complete; production certification denied until all listed gates pass. Live trading remains outside this component and disabled.
