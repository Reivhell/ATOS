# 11 — Architecture Decision Records

## ADR-001: Rust owns critical execution and risk boundaries
**Status:** Accepted and implemented for the paper-trading domain.  
Order intent validation, quote-freshness checks, sizing limits, accounting, persistence, and paper fills execute in Rust. Svelte is a presentation layer and cannot directly mutate broker state outside the Tauri command boundary.

## ADR-002: Python remains optional, not part of the runtime
**Status:** Deferred.  
The current product does not require Python: market normalization, the paper broker, risk gateway, and SMA backtester are implemented in Rust. A Python extension may be added later for research or data science only when it has a demonstrated need, an isolated process boundary, typed input/output contracts, resource limits, and no direct broker or secret access. The Python SDK contract in `07_API_AND_PYTHON_SDK.md` is a proposal, not a shipped SDK.

## ADR-003: Paper trading before live trading
**Status:** Accepted and implemented as a hard capability boundary.  
The current binary contains no authenticated exchange client or live order endpoint. The Live Trading screen is intentionally locked. A future live adapter requires a separate security review, credential vault, order reconciliation, venue-side idempotency, explicit confirmation, and acceptance testing.

## ADR-004: Momentum and risk are separate assessments
**Status:** Accepted and implemented as `momentum-risk-v1`.  
The screener computes a transparent heuristic momentum score and a separate risk score. Neither is a calibrated probability, prediction, or safety certification. Version and source metadata are surfaced in the UI and records.

## ADR-005: Start as a modular monolith
**Status:** Accepted.  
The desktop application uses a single local Rust process with modules for state/risk/paper execution, backtesting, provider adapters, and persistence. This avoids premature service deployment while retaining code boundaries.

## ADR-006: Unknown is not safe
**Status:** Accepted and implemented for screener evidence.  
The screener flags missing pool creation time, liquidity, market cap, transaction counts, and momentum windows where applicable. Contract permissions, holder concentration, liquidity locks, wash trading, and executable depth are explicitly marked unverified rather than assumed safe.

## ADR-007: Provider data must retain provenance and freshness
**Status:** Accepted and implemented for current public adapters.  
Every quote and screener result carries source and observation time. Failed refreshes mark provider state stale and retain the last successful timestamp; they do not relabel cached values as newly refreshed. The web runtime returns no market data; market-dependent operations require a native provider snapshot and cannot authorize paper fills without it.
