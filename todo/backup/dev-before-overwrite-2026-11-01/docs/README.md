# Agentic Trading OS — Documentation Index

This directory contains the original 12 product/architecture specification documents plus the current implementation and operations notes. The specifications remain the product contract; `IMPLEMENTATION_STATUS.md` describes which capabilities exist in code and which remain deliberately unimplemented.

## Product and architecture specifications

- `01_PRD.md` — Product requirements and intended users
- `02_SCOPE_AND_REQUIREMENTS.md` — Scope, requirements, and non-functional expectations
- `03_SYSTEM_ARCHITECTURE.md` — Components, data flow, and service boundaries
- `04_MEMECOIN_SCREENER.md` — Screener signals, scoring, and risk limitations
- `05_UI_UX_SPEC.md` — Navigation, screens, and interaction behavior
- `06_DATA_MODEL_AND_EVENTS.md` — Core entities and event contracts
- `07_API_AND_PYTHON_SDK.md` — API/SDK direction; Python is deferred in the current runtime
- `08_SECURITY_AND_TRADING_RISK.md` — Threat boundaries and trading controls
- `09_TESTING_AND_ACCEPTANCE.md` — Test strategy and acceptance criteria
- `10_ROADMAP.md` — Phases and explicitly deferred capabilities
- `11_ADR.md` — Architecture decision records
- `12_OPEN_QUESTIONS.md` — Current baseline decisions and unresolved release decisions
- `13_TECHNICAL_ANALYSIS_CATALOG.md` — Master catalog scope, implemented calculator subset, method notes, and limits

## Implementation and operations

- `ARCHITECTURE_LAYERS_0.5.8.md` — Layer boundaries, dependency direction, and analysis/decision contracts
- `RESEARCH_DECISION_ENGINE_0.5.8.md` — Evidence voting, provenance requirements, and fail-closed outcomes
- `VALIDATION_REPORT_0.5.8.md` — Test evidence, release blockers, and certification status
- `IMPLEMENTATION_STATUS.md` — Implemented vs. deliberately unimplemented capabilities and validation state
- `OPERATIONS.md` — First run, persistence, risk behavior, validation commands, and manual acceptance flows
- `BACKTEST_ROBUSTNESS_0.5.7.md` — Execution-cost sensitivity and chronological out-of-sample evaluation contract

## Current baseline

The delivered codebase is a Tauri 2 + Svelte 5/TypeScript desktop application with a Rust paper broker, risk gateway, SQLite persistence/audit ledger, a deterministic SMA crossover backtester, a searchable technical-analysis catalog with six indicator readings, public read-only Binance market data, and a bounded DEX Screener boosted-token feed.

This is not an assertion of live-trading readiness. Live orders, exchange credentials, wallet signing, autonomous LLM agents, full-chain token safety analysis, and signed installers are not implemented. See `IMPLEMENTATION_STATUS.md` for the release boundary and outstanding verification.

## Current increment

- `MARKET_DATA_ASSURANCE_0.6.0.md` — canonical timeframe contract, fail-closed timeline audit, and advisory latest-bar anomaly diagnostics
- `ARCHITECTURE_LAYERS_0.6.0.md` — updated domain/engine boundaries
- `VALIDATION_REPORT_0.6.0.md` — current test evidence and production-release blockers
