# Agentic Trading OS — Documentation Index

`docs/` retains **essential, current** product/architecture specifications and operations guidance for the `0.0.1-beta` engineering snapshot. Versioned validation reports, audits, and historical engineering notes have been moved to `docs/archive/` (including `docs/archive/historical-0.11-era/`) to remove redundancy from the active doc set.

## Core specifications
- `01_PRD.md` — Product requirements and intended users
- `02_SCOPE_AND_REQUIREMENTS.md` — Scope, requirements, and non-functional expectations
- `03_SYSTEM_ARCHITECTURE.md` — Components, data flow, and service boundaries
- `04_MEMECOIN_SCREENER.md` — Screener signals, scoring, and risk limitations
- `05_UI_UX_SPEC.md` — Navigation, screens, and interaction behavior
- `06_DATA_MODEL_AND_EVENTS.md` — Core entities and event contracts
- `07_API_AND_PYTHON_SDK.md` — API/SDK direction; Python is deferred
- `08_SECURITY_AND_TRADING_RISK.md` — Threat boundaries and trading controls
- `09_TESTING_AND_ACCEPTANCE.md` — Test strategy and acceptance criteria
- `10_ROADMAP.md` — Phases and explicitly deferred capabilities
- `11_ADR.md` — Architecture decision records
- `12_OPEN_QUESTIONS.md` — Baseline decisions and unresolved release decisions
- `13_TECHNICAL_ANALYSIS_CATALOG.md` — Catalog scope and implementation limits
- `14_TECHNICAL_ANALYSIS_MASTER_LIST_CONTRACT.md` — Master list / strict contract

## Implementation & operations (current)
- `IMPLEMENTATION_STATUS.md` — Implemented vs. deliberately unimplemented capabilities and validation state
- `OPERATIONS.md` — First run, persistence, risk behavior, validation commands
- `STRICT_DECISION_LAYER_POLICY.json` — Machine-readable strict decision-layer policy
- `agents/` — Specialist agent docs/schemas used by the strict analysis flow

## Baseline / safety boundary
Tauri 2 + Svelte 5 desktop app with Rust-owned paper trading, risk gateway, persistence/audit, backtest/research, MT5 telemetry ingestion (read-only EA), and bounded public/provider integrations. **Not production-certified.** No live order submission/cancellation/amendment. Historical/versioned reports and audits: see `docs/archive/` and repository `CHANGELOG.md`.
