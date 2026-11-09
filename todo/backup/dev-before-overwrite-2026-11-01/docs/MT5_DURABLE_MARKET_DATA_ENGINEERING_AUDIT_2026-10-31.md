# ATOS MT5 Durable Market-Data Ingestion — Engineering Audit

**Audit date:** 2026-10-31  
**Repository line:** 0.10.1 (version intentionally unchanged)  
**Scope:** Rust MT5 bridge, SQLite market-data store, MQL5 Expert Advisor, Tauri command wiring, Svelte status presentation, regression contracts.

## Executive result

The MT5 bridge was extended from account telemetry into a durable, bounded multi-symbol market-data ingestion path. The EA is included at `MQL5/Experts/ATOS_Bridge.mq5` in the repository archive. The bridge remains read-only: it has no order submission, modification, cancellation, close, or position-mutation path.

The current source passed 289 Node test subtests across 43 command groups, strict TypeScript layer typechecking, and repository guardrails. A lexical structure scan of the 828-line MQL5 adapter passed; a source scan found no order mutation API calls.

**This is not a production release certification.** The environment lacks Cargo/Rustfmt/Clippy, the Svelte/Vite/Tauri build executables, and MetaEditor. Rust unit tests were added but could not be executed; MQL5 compilation and real terminal/broker integration were not performed. Release preflight correctly remains blocked by nine issues.

## Architecture delivered

### 1. Durable market-data store

`src-tauri/src/mt5_market_data.rs` creates an isolated `mt5-market-data.sqlite3` database, separate from the paper-account ledger and research workspace. It uses SQLite WAL, `synchronous=FULL`, foreign-key enforcement, and a bounded five-second busy timeout. A schema `user_version` is recorded; opening a database created by a newer schema version fails closed rather than silently downgrading it.

The store persists:

- Tick observations keyed by content fingerprint.
- Closed OHLC bars keyed by broker server, account, symbol, timeframe, and bar-open time.
- Bar revision audit records with previous/replacement payload hashes.
- Broker/account symbol coverage and transactional aggregate counters.
- Exact frame receipts keyed by client, session, and sequence, storing the hash of the original frame bytes.

Each ingestion uses one `BEGIN IMMEDIATE` transaction. Tick writes, bar inserts/revisions, symbol registry updates, counters, and the sequence receipt commit together. The bridge sends an acknowledgement only after transaction commit. Exact replay of an already committed frame is idempotent; a reused sequence with different bytes and any sequence gap are rejected.

The status surface reports storage availability after successful status queries. Negative/corrupt counters produce an error; they are not silently converted to zero. Availability is deliberately not represented as a full disk-health or write-readiness certification.

### 2. Authentication and data-scope binding

The Rust listener binds to IPv4 loopback only (`127.0.0.1:47650`) and accepts one active terminal session. Pairing credentials are stored in the operating-system credential vault. Frames have strict schemas, bounded size, timestamp validation, nonce replay tracking, and sequential acknowledgement checks.

A frame is additionally checked against the identity and allow-list established at authentication:

- Account login and broker server must remain identical to the authenticated identity.
- The primary quote symbol must be in the requested symbol set.
- Every tick symbol must be subscribed.
- Every bar symbol and timeframe pair must be subscribed.

Out-of-scope frames are rejected before durable storage. This closes an account-binding and subscription-boundary gap identified during source review.

### 3. Per-series recovery and reconciliation

The authentication response returns durable cursors for each subscribed symbol's tick stream and each symbol/timeframe bar series.

The MQL5 EA:

- Supports up to 8 broker-native symbols and 6 configured timeframes.
- Reads at most 512 ticks per symbol and 64 closed bars per symbol/timeframe in a frame.
- Seeds an empty tick series from the most recent 512 ticks and an empty bar series from the latest 64 closed bars; this is not full-history backfill.
- Resumes from server-issued durable cursors after reconnect or application restart.
- Transfers closed-bar backlog oldest-first in bounded chunks and refuses to jump forward when the exact prior candle cannot be found.
- Re-reads the latest three already-committed closed bars every 60 seconds; broker corrections are stored as auditable revisions.
- Advances process-local cursors and its reconciliation timer only after the exact durable frame sequence is acknowledged.

Tick history is necessarily less exact than candle history. `MqlTick` has no exchange-wide unique event ID; the implementation uses a content fingerprint to suppress replay. Indistinguishable events sharing the same symbol and millisecond cannot be proven distinct, and more than 512 such events cannot be guaranteed to paginate without loss. Tick counts must not be treated as an exchange-grade event ledger.

### 4. Presentation and operational path

The Tauri service exposes bridge status and durable ingestion status. The Svelte MT5 panel shows current account telemetry only while the heartbeat is fresh and presents durable tick/bar/symbol totals from the SQLite store. Stale snapshots are not presented as current observations. The EA's canonical installation path is documented and tested as `MQL5/Experts/ATOS_Bridge.mq5`.

## Important operational limits

- Transport is loopback TCP with a high-entropy pairing token, **not encrypted in transit**. It is intended for the same trusted computer; it must not be forwarded, tunneled, or exposed to another host.
- The user must enter the pairing token into MT5 EA inputs. The terminal's EA configuration/data directory should be treated as secret-bearing local state.
- The first tick import is bounded to the latest 512 ticks per symbol. There is no independent historical tick backfill service.
- Broker-reported time fields are preserved; automatic timezone/session normalization is not implemented.
- Local SQLite retention, partitioning, compaction, and multi-year high-frequency throughput have not been benchmarked. Institutional-scale archival claims would be premature.
- The bridge does not submit or mutate trades. No live execution authority is added.

## Verification matrix

| Check | Result | Notes |
|---|---|---|
| `npm test` | PASS — 289/289 | 43 command groups; Node regression/contract suite |
| `npm run typecheck:layers` | PASS | Strict TypeScript layer check |
| `npm run verify:repo` | PASS with warnings | Static repository guardrails; not a native build |
| MQL5 lexical structure scan | PASS | 828 lines; balanced delimiters, strings, and comments |
| MQL5 order-mutation API scan | PASS | No order submit/modify/delete/close calls found |
| Rust unit tests | NOT RUN | 19 MT5-focused unit tests exist; Cargo/Rust are unavailable |
| Rust compile / Clippy / Rustfmt | NOT RUN | Toolchain executables unavailable |
| Svelte/Vite/Tauri production build | NOT RUN | `svelte-check`, Vite, and Tauri CLI are unavailable |
| MetaEditor compile | NOT RUN | MetaEditor/MQL5 compiler unavailable |
| Real MT5 terminal/broker integration | NOT RUN | No terminal/broker session available in this environment |
| Release preflight | BLOCKED — 9 issues | Details below |

### Release preflight blockers

1. `package-lock.json` is missing.
2. `bun.lock` is missing.
3. `src-tauri/Cargo.lock` is missing.
4. `svelte-check` executable is unavailable.
5. Vite executable is unavailable.
6. Tauri CLI executable is unavailable.
7. Cargo is unavailable.
8. Rustfmt is unavailable.
9. Cargo Clippy is unavailable.

The three missing lockfiles prevent deterministic dependency resolution. The absent compilers prevent the native/frontend build and native tests from being verified. These blockers are not waived by passing Node tests.

## Native tests authored but not executed

The MT5 Rust modules now contain 19 unit tests covering symbol-name validation, numeric and OHLC invariants, frame atomicity/idempotency/sequence strictness, tick replay suppression, bar revision audit, future-schema rejection, per-series cursor recovery, subscription validation, status counters, and authenticated account/series scope binding. They require execution under the pinned Rust toolchain before release.

## Next production gates

1. Restore deterministic npm/Bun/Cargo lockfiles using the repository's reviewed dependency policy; do not generate or bless dependency versions without review.
2. Install the pinned Rust toolchain and frontend dependencies, then run Cargo tests, Rustfmt, Clippy with warnings treated according to CI policy, Svelte checks, Vite production build, and Tauri packaging.
3. Compile the EA in MetaEditor against the target MT5 build and test socket lifecycle, timers, symbol selection, history synchronization and reconnect behavior.
4. Run fault-injection tests for process termination before/after SQLite commit, ACK loss, database busy/full/corruption, broker history gaps, terminal restarts, and broker bar corrections.
5. Validate against an explicitly authorized MT5 demo or non-production account first; compare imported ticks/candles and cursors with terminal history across quiet and high-activity periods. Do not infer live execution readiness from data-ingestion success.
6. Benchmark and design retention/partitioning for the target symbols, tick rate, timeframe mix, and operational retention window.

Only after these gates pass should this source be considered eligible for a signed production release. Live order execution remains a separate engineering workstream with mandate authorization, deterministic pre-trade risk, idempotent durable order outbox, broker reconciliation, and independent fault-injection coverage.


## Follow-up: closed-bar freshness authority

The candle cache now carries its own retrieval timestamp instead of inheriting the latest quote refresh time. Candle open time remains the timestamp used by charts and backtests; the latest closed-bar timestamp is derived from the requested interval as `bar_open + interval - 1ms`. Live eligibility requires both a recent retrieval receipt (120 seconds) and a latest fully closed bar no older than one interval plus a 30-second provider grace. A stale replay cannot refresh the candle receipt timestamp, forming/future bars are rejected, and an omitted candle response retains the prior cache without relabelling it as newly received. The Rust domain validator is shared by cache ingestion, native candle reads, strict analysis, and unified market-data provenance; the frontend independently checks the same timestamp relationship before rendering.
