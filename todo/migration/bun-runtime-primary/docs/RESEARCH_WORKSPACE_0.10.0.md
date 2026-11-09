# Rust-native Research Workspace — v0.10.0

## Purpose and authority boundary

Research Lab is a manual-first research and evidence subsystem. It turns a written hypothesis into a reproducible native backtest, independent statistical reviews, a deterministic risk gate, and an immutable run card. It is not an autonomous trading authority. A research decision cannot create an order intent, release the account kill switch, or bypass the existing Rust order/risk gateway.

Research data is stored in `research-workspace.sqlite3`, separate from the existing account-state database. The research module does not access `EngineState`, place orders, modify risk limits, toggle the kill switch, or persist account state. It reads the account's current fee/slippage assumptions through `TradingCore::research_risk_limits`, which validates the account state without invoking the dashboard snapshot path that may persist valuation marks.

## Workflow

The workflow is a validated directed acyclic graph:

1. `data_integrity` — verifies the native backtest result's data encoding, SHA-256, period, source, and candle interval.
2. `deterministic_backtest` — executes the existing Rust SMA crossover backtest using the native BTCUSDT 5m candle cache.
3. `walk_forward_review` — reviews chronological out-of-sample folds produced by the native backtest engine.
4. `block_bootstrap` — independently resamples contiguous blocks of historical strategy returns. The seed is derived from the dataset fingerprint. PCG-XSH-RR 64/32 supplies reproducible sampling and rejection sampling avoids modulo bias in bounded block-start selection. The generator is intended for deterministic statistical resampling, not cryptographic use. The work is bounded by the configured observation and simulation limits.
5. `risk_review` — compares both observed maximum drawdown and the resampled 95th-percentile maximum drawdown with the configured drawdown ceiling.
6. `investment_committee` — aggregates OOS, bootstrap, trade-count, Sharpe, and severe cost-stress blockers. A passing result is eligible only for a separately supervised paper-validation phase.

The OOS and bootstrap reviews run in separate scoped worker threads against the same immutable `BacktestResult`. Any worker panic or malformed evidence aborts the workflow before the run card is persisted. The graph is validated for duplicate nodes, repeated/missing dependencies, and cycles before persistence.

The role labels (Data Steward, Quant Researcher, Independent Robustness Reviewer, Risk Officer, Investment Committee) are deterministic Rust validators in this release. They are not independent LLM agents and do not conduct natural-language strategy discovery. The written research goal is stored as provenance; the current native strategy implementation remains the existing SMA crossover.

## Evidence and persistence

Every run card stores:

- hypothesis ID and the exact hypothesis revision evaluated;
- goal, instrument, interval, and creation time;
- account fee/slippage/risk context read at run time;
- exact research thresholds: maximum drawdown, minimum positive OOS ratio, and minimum Sharpe ratio;
- strategy periods, capital, fills/trade statistics, and cost stress scenarios;
- dataset source, time bounds, candle interval, encoding version, and SHA-256 fingerprint;
- OOS fold summary, moving-block bootstrap configuration/seed/distribution, workflow DAG, specialist findings, blockers, and caveats;
- SHA-256 of the serialized run-card payload.

Run cards are append-only. The database also stores an append-only SHA-256 event chain. Each new event includes sequence, timestamp, aggregate ID, event type, canonical JSON payload, previous hash, and current hash. Startup and explicit integrity checks validate event continuity, hashes, card hashes, card identity/schema, hypothesis lifecycle state by replaying its creation/review/invalidation events, and the exact normalized SQL definitions of all immutability triggers. Hypothesis identity fields are immutable at the SQLite layer and hypothesis records cannot be deleted. A missing or weakened trigger, unsupported schema, altered event, or altered run card fails closed.

Hypothesis updates use optimistic revision checks. A research run records the revision it began from and commits its run card plus the hypothesis lifecycle update in a single SQLite transaction. If the hypothesis was invalidated or revised while the backtest ran, persistence is rejected rather than committing stale evidence.

The workspace currently caps the registry at 200 hypotheses, the immutable run-card ledger at 100 cards, and each serialized run card at 1 MB. It does not silently delete old research evidence to make room. Export/archive tooling is a separate follow-up requirement before the cap is reached.

## Current support boundary

This release deliberately accepts exactly `BTCUSDT` on `5m`, matching the current validated native candle-cache contract. The research module rejects other symbols and intervals rather than relabeling the BTCUSDT series. It uses real native backtest output; there are no sample candles, fabricated reports, or simulated provider responses.

The run card is not an execution artifact. `ELIGIBLE_FOR_PAPER_VALIDATION` means only that the defined research gates passed. It does not place a paper order, authorize live orders, prove that a strategy is profitable, or establish future performance. Moving-block bootstrap is a conditional sensitivity analysis of historical strategy returns, not a forecast or guarantee. The current SMA backtest remains a narrow strategy family and is not a strategy optimizer.

## Vibe-Trading feature adoption assessment

The upstream Vibe-Trading project describes a broad research loop from planning and grounding through execution, statistical validation, and delivery, with hypothesis records, run cards, multi-agent reviews, cross-market data, shadow-account diagnostics, and numerous export/integration paths. This project adopts the concepts that fit ATOS's current native contracts:

| Vibe-Trading concept | ATOS v0.10.0 decision | Reason |
|---|---|---|
| Hypothesis → research → backtest → evidence report | Adopted | Implemented as durable hypothesis lifecycle and native workflow DAG |
| Reproducible run cards | Adopted | Immutable content-hashed card captures source fingerprint, parameters, thresholds, assumptions and findings |
| Walk-forward and bootstrap validation | Adopted | Uses current native OOS folds and a bounded deterministic moving-block bootstrap |
| Multi-specialist review / committee | Adapted | Independent deterministic Rust validators; not LLM agents or generated trading signals |
| Cross-market / multi-loader backtesting | Deferred | Existing native dataset provider exposes only the validated BTCUSDT 5m series; adding unsupported markets would violate provenance |
| Alpha zoo / wide factor library | Deferred | Current Rust backtest does not implement the corresponding factor definitions, point-in-time data, or validation contracts |
| Shadow Account / broker-journal rule extraction | Deferred | Requires validated broker exports, account identity mapping, execution-cost semantics, and a separately modeled shadow ledger; none is created from dummy data |
| Broker connectors / MT5 / autonomous order execution | Excluded | Not compatible with the current no-live-execution security boundary and user's explicit need for loss containment |
| Scheduler, chat channels, plugin ecosystem and strategy exports | Deferred | Not necessary for the core research-evidence path; would expand the operational and credential surface before native release certification |

Upstream reference: [HKUDS/Vibe-Trading](https://github.com/HKUDS/Vibe-Trading). The repository is actively developed, so the table is a feature-fit assessment for this ATOS codebase rather than a claim that every upstream feature has been copied or validated.

## Operational requirements before deployment

This source candidate still requires the release gates in `VALIDATION_REPORT_0.10.0.md`: lockfiles, reviewed 40-character action pins, Rust native tests/Clippy, Svelte/Vite/Tauri build, provider/keychain integration tests, cross-platform package acceptance, signed attestations, and applicable OS signing/notarization. The research subsystem must not be described or operated as production-certified until those gates pass on the actual release workflow.
