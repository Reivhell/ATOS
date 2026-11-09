# 10 — Implementation Roadmap

## Status at 2026-10-27 (0.2.0)

- **Implemented baseline:** Tauri/Svelte desktop UI, Rust paper broker/risk gateway, local SQLite persistence/audit, public Binance spot quotes and BTCUSDT 5m candles, deterministic SMA crossover backtest with canonical SHA-256 candle fingerprint and UTC input range, searchable technical-analysis catalog plus six bounded indicator calculators, browser fixture mode, read-only DEX Screener latest-boost discovery with stale-state retention, and an exact typed candle-series request/response boundary.
- **Partially implemented:** risk-focused agent controls (observe-only toggles only), screener detail evidence and heuristic scoring (not contract/holder safety), and local audit/recovery. Cross-platform packaging/build verification has not been run in this authoring environment.
- **Not implemented:** full event bus/scheduler, strategy persistence/versioning, LLM agents, robust research/data pipeline, contract/holder/liquidity-lock enrichment, notifications/alerts, credential vault, remote API, Python SDK, and live trading.

The phases below remain the roadmap for the work not completed. Feature presence in the UI does not imply that a backend capability is implemented.

## Phase 0 — Product and technical decisions
- Confirm target app shell and packaging approach.
- Select first market-data source and first exchange/chain.
- Choose persistence and event transport appropriate for a single-user MVP.
- Freeze domain contracts before expanding adapters.
- Create threat model and risk-policy defaults.

**Exit:** architecture decisions recorded; PRD and API contracts reviewed.

## Phase 1 — Core vertical slice
- Rust domain types and event envelope.
- One market-data adapter and freshness monitoring.
- Event bus and structured logging.
- One simple deterministic strategy.
- Risk gateway.
- Paper broker with orders, fills, positions, and fees.
- Minimal dashboard for status, chart, orders, positions, and event trace.

**Exit:** a user can replay data and paper-trade a strategy end-to-end.

## Phase 2 — Backtesting and reproducibility
- Historical data ingestion and storage.
- Persisted symbol/timeframe selection, candle provenance, and explicit dataset selection for the new technical-analysis research view.
- Backtest runner with fees/slippage assumptions.
- Build/test the next indicator families in small, independently reviewed slices, starting with volume-aware indicators and avoiding look-ahead bias.
- Metrics, trade list, equity curve, drawdown.
- Configuration/data/version hashes.
- Replay and regression tests.

**Exit:** repeatable backtest report with transparent assumptions.

## Phase 3 — Agent runtime
- Provider abstraction.
- Read-only tools first.
- Tool allowlist, budget, timeouts, trace viewer.
- Observe-only mode, then paper-action mode.
- Agent lifecycle and kill switch.

**Exit:** agent can inspect data and propose paper actions without bypassing risk.

## Phase 4 — Meme Coin Screener
- Start with one chain/data source.
- Token/pool discovery and normalization.
- Liquidity and contract/holder risk checks where available.
- Momentum and risk scores separately.
- Filters, detail drawer, watchlist, alerts.
- Historical validation and false-positive tracking.

**Exit:** screener is transparent about evidence, source quality, and uncertainty.

## Phase 5 — Hardening
- Failure recovery, database migrations, performance profiling.
- Security review and dependency audit.
- UI usability and accessibility review.
- Expand exports to include bounded/full-history pagination, embed raw input candle data for offline replay bundles, and verify exported hashes against source datasets.
- Diagnostics and operational telemetry.

## Phase 6 — Controlled live execution (not MVP)
- One venue only.
- Trade-only API keys and permission validation.
- Preflight checks, strict limits, idempotency, reconciliation.
- Manual confirmation and kill switch.
- Small-scale sandbox/testnet validation where supported.

**Exit:** documented operational readiness review. Do not skip directly from a profitable backtest to live trading.


## Dashboard refinement added in v0.1.6

- Match the supplied QuantDinger reference with a denser desktop workspace and preserved separation between research, data, paper orders, and the locked live-trading surface.
- Chart OHLC and volume must always come from the validated currently loaded candle series. Until the Rust market-data adapter supports additional instruments and intervals, the UI must not make XAUUSD, Forex, index, commodity, or alternate-timeframe data appear available.
- Next priority: add explicit provider capabilities and a typed candle-series request (symbol + interval + provenance), then use it to unlock additional chart intervals without changing data silently.


## Strict master-list enforcement added in v0.1.7

- The technical-analysis taxonomy uses an ordered, reviewed 364-entry snapshot with exact category counts and distinct coverage states. Additions, omissions, renames, order changes, and status changes fail the dependency-free regression suite until the snapshot is intentionally reviewed.
- Next work should implement one detector family at a time with explicit closed-candle/no-look-ahead rules, data requirements, uncertainty states, known-vector fixtures, and cross-market validation before assigning `Calculated now`.


## Explicit market-data capability contract added in v0.1.8

- The backend exposes quote-symbol coverage separately from candle-series capability metadata, including exact symbol, interval, availability, source, candle count, most recent candle timestamp, and freshness/provenance state.
- A typed `get_candle_series` request accepts exchange-style interval literals and verifies the cached cadence. Only BTCUSDT 5m is currently supported; requests for ETHUSDT candles or other intervals fail explicitly and never fall back.
- The chart consumes the exact response requested and clears old data if the new request cannot be satisfied. A UI-boundary validator checks response identity, source/freshness metadata, OHLC validity, timestamp ordering, minimum history, and expected cadence. Request epochs prevent late responses from overwriting newer selections. Selecting another symbol for a paper order shows a persistent chart/order mismatch warning because quote availability does not mean candle history exists.
- Next priority: extend the provider adapter one explicit series at a time, store interval identity with the candle dataset, and test provider capabilities, timestamp continuity, stale-state policy, and request routing before exposing additional series.


## Single-instance persistence hardening added in v0.1.9

- Native startup holds an exclusive transaction on a separate SQLite guard database for the lifetime of the paper-account engine. This prevents two processes from owning stale in-memory snapshots of the same account and silently overwriting each other. The OS-managed SQLite lock is released on process termination without PID-file cleanup.
- A Rust regression test verifies second-instance rejection, first-instance continuity, and successful restart after clean close. The test is added but has not been run in the current authoring environment because the Rust toolchain is unavailable.
- Before release, verify the lock and recovery behavior on Linux, Windows, and macOS, including forced termination, app-data paths with non-ASCII characters, filesystem permission errors, and restore-from-backup workflows.
