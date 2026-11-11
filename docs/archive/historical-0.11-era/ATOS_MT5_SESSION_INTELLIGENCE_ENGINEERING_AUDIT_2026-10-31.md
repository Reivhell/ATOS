# ATOS — MT5 Session Intelligence Engineering Audit

**Audit date:** 2026-10-31  
**Application package version:** 0.10.1 (intentionally unchanged)  
**MT5 bridge protocol:** 4  
**MT5 market-data schema:** 3  
**Release decision:** BLOCKED — native build, dependency-lock, and terminal compilation gates remain open.

## 1. Delivered engineering scope

This tranche adds a broker-session metadata plane to the existing Rust/Tauri MT5 telemetry architecture. It is not a demo or mock path and does not add live order execution.

- `MQL5/Experts/ATOS_Bridge.mq5` now captures all seven broker weekdays and all reported `SymbolInfoSessionTrade` windows for each authenticated watched symbol.
- Session enumeration is bounded to 16 windows per weekday. Unsupported overflow, malformed boundaries, or unexpected terminal API errors withhold the complete frame. The EA does not advance tick/bar cursors until the Rust receiver commits and acknowledges the frame.
- Session schedules carry a separate UTC capture instant. Session boundaries remain broker-server wall-clock seconds from midnight; the adapter does not infer UTC offsets, timezone identifiers, DST, holidays, or exceptional closures.
- The EA refresh interval is configurable from 30 to 3,600 seconds (default 300). Schedules are sent on initial authentication/reconnection and refresh, not on every heartbeat. Unchanged heartbeats omit the schedule payload only when the Rust store can prove a complete, valid, fresh calendar already exists for every authenticated symbol.
- The receiver rejects missing initial calendars, incomplete or duplicate updates, out-of-scope symbols, future-dated captures, and stale retained calendars. A retained calendar older than the one-hour freshness ceiling causes the frame to fail closed.
- Rust persists the current calendar projection and immutable revision history keyed by broker server, account login, and symbol. Calendar updates are committed in the same `BEGIN IMMEDIATE` transaction as ticks, bars, instrument metadata, aggregate counters, and the durable frame receipt.
- Calendar history is append-only. Reads verify payload fingerprints and contiguous predecessor/revision chains. On a calendar refresh, the receiver verifies the existing history before extending it.
- Capture timestamps are excluded from the schedule fingerprint. This prevents periodic refreshes from creating false material revisions. A material weekly-session change creates a new immutable revision.
- Dataset manifests now bind the current symbol-specific session-calendar fingerprint, or an explicit `SESSION-CALENDAR-UNKNOWN` marker. Changing the calendar changes the dataset fingerprint even when candle payloads remain unchanged.
- A new authenticated Tauri query exposes the current calendar and its provenance. The MT5 panel displays weekday windows, overnight windows, capture time, revision, and fingerprint.

## 2. Architecture and trust boundaries

The data path remains:

`MQL5 EA → authenticated loopback protocol v4 → Rust bridge validation → SQLite transaction → application query facade → Tauri IPC → Svelte presentation`.

The transport remains loopback-only. Authentication identity is bound to account login/server and the exact symbol/timeframe subscription. Calendar updates are required to cover the complete authenticated symbol set. Sparse heartbeats may omit the calendar payload, but the durable store independently verifies that retained state exists, matches its fingerprint and latest immutable revision, and remains fresh before committing the frame.

The schedule is not used as a broker execution permission, margin model, position-sizing input, or trade authorization. The EA remains read-only and contains no order submission, modification, cancellation, or position-close API.

## 3. Data semantics and explicit non-claims

The MQL5 reference defines `SymbolInfoSessionTrade` session boundaries as seconds from midnight with the date component ignored. ATOS preserves these values in broker-server wall-clock space. The UTC capture instant is a separate field and is not used to reinterpret the weekly session boundaries.

A recurring weekly schedule is not a dated holiday calendar. The current implementation does not resolve broker timezone offsets, DST transitions, exchange holidays, exceptional closures, or broker-specific calendar overrides. Therefore, interval anomalies remain unclassified candidates. ATOS does not claim that a gap is a confirmed missing tradable candle, even when a weekly session calendar exists.

The existing instrument specification remains explicitly partial (`broker_snapshot_v1_partial`). Neither the session calendar nor the instrument registry is sufficient for safe live margin calculation or order sizing.

## 4. Migration and compatibility

- SQLite schema v3 is additive. It creates `mt5_session_calendars` and `mt5_session_calendar_revisions`; existing ticks, bars, and instrument specification projections are not rewritten.
- A schema v2 upgrade test was added to preserve the existing instrument specification projection while adding the calendar tables.
- Bridge protocol v4 is incompatible with protocol v3. Operators must compile and install the repository EA at `MQL5/Experts/ATOS_Bridge.mq5` before reconnecting.
- Package version remains 0.10.1. No cosmetic version bump was made.

## 5. Validation performed in this environment

| Check | Result |
|---|---|
| Node regression suite (`node --test scripts/*.test.mjs`) | 305 passed, 0 failed |
| Package test chain (`npm test`) | Passed |
| Shared TypeScript layers | Passed |
| Tauri adapter + shared types with a temporary declaration stub for the absent Tauri package | Passed |
| Repository guardrail scan | Passed; dependency lockfiles remain warnings |
| JSON parse scan | 18/18 files passed |
| Rust `include_str!` embedded asset resolution | 10/10 resolved |
| Tauri command declaration/registration consistency | 37/37 registered commands resolve |
| SQLite schema v3 smoke test | Passed; calendar tables and append-only triggers verified |
| MQL5 lexical brace scan | Passed |
| EA trade-mutation API scan | Passed; no trade mutation APIs found |
| Private/runtime artifact scan | Passed |

The native Rust unit tests were added for schema v1/v2 upgrades, immutable calendar revisions, capture-time stability, dataset fingerprint binding, missing/stale retained calendars, invalid weekday/boundary metadata, and overnight windows. They could not be executed in this environment because `cargo`, `rustc`, `rustfmt`, and `cargo clippy` are unavailable.

## 6. Release blockers — not certified production-ready

`npm run release:preflight` remains **BLOCKED (9 issues)**:

1. `package-lock.json` is missing.
2. `bun.lock` is missing.
3. `src-tauri/Cargo.lock` is missing.
4. `svelte-check` is unavailable.
5. `vite` is unavailable.
6. Tauri CLI is unavailable.
7. Cargo is unavailable.
8. rustfmt is unavailable.
9. cargo clippy is unavailable.

MetaEditor is also unavailable, so the MQL5 EA was not compiled. No live MT5 terminal/broker integration or broker-specific schedule validation was performed. The Node contract suite and SQLite schema smoke test do not replace native Rust tests, Svelte production build, MetaEditor compilation, or real terminal acceptance tests.

## 7. Required next gates

1. Restore pinned package and Rust dependencies and run the full native Rust test/clippy/rustfmt pipeline.
2. Run Svelte type/build checks and the Tauri release build.
3. Compile the protocol-v4 EA in MetaEditor and test connect, reconnect, sparse heartbeat, scheduled refresh, terminal restart, missing calendar, stale calendar, and broker session enumeration against real broker terminals.
4. Add a dated exception-calendar subsystem only after an authoritative broker-specific source and timezone/DST provenance are available. Do not infer holidays from empty weekly windows.
5. Keep the live execution control plane closed until independent risk, order lifecycle, reconciliation, and fault-injection gates are completed.
