# Agentic Trading OS (ATOS)

**A local-first trading research desktop built with Rust, Tauri 2, and Svelte 5.** ATOS brings market-data inspection, MT5 telemetry, strategy backtesting, paper-account controls, and AI-assisted research into one workspace.

> **Safety status:** ATOS is research- and paper-trading-only. The MT5 bridge is read-only and does not submit, modify, close, or cancel broker orders. The project is not production-certified; passing source-level checks does not replace native builds, MetaEditor compilation, or real-terminal and broker acceptance tests.

[![Node tests](https://img.shields.io/badge/Node%20tests-375%20passing-brightgreen)](https://github.com/Reivhell/ATOS)
[![Rust tests](https://img.shields.io/badge/Rust%20tests-239%20passing-brightgreen)](https://github.com/Reivhell/ATOS)
[![Release status](https://img.shields.io/badge/release-not%20production--certified-orange)](https://github.com/Reivhell/ATOS)

## Contents

- [What ATOS does](#what-atos-does)
- [Core capabilities](#core-capabilities)
- [Architecture](#architecture)
- [Requirements](#requirements)
- [Getting started](#getting-started)
- [Verification status](#verification-status)
- [Safety and current limitations](#safety-and-current-limitations)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License and security](#license-and-security)

## What ATOS does

ATOS is a local desktop workspace for researching markets, testing strategies, inspecting account telemetry, and supervising AI-assisted analysis. It is designed around explicit boundaries: Rust owns authoritative state and risk decisions; the UI presents those decisions; AI output is advisory rather than execution authority.

The current application combines MT5 telemetry with public market-data sources and local research workflows. It is not a cloud trading service, and it does not provide live broker order execution.

## Core capabilities

- **Market data:** Rust-native provider integration, canonical data models, freshness checks, and inspectable market-data provenance.
- **MT5 bridge:** Authenticated, read-only terminal telemetry with durable tick/candle ingestion, per-series cursors, and historical data inspection.
- **Account-wide risk controls:** Aggregated stop-risk across open positions and active pending orders, protection-state checks, risk-budget headroom, and broker-native candidate margin preflight.
- **Backtesting:** Deterministic research workflows, out-of-sample windows, next-bar-open execution, and dataset fingerprints.
- **AI-assisted research:** Allowlisted providers, OS credential-store integration, bounded commentary, and no execution authority.
- **State integrity:** Local SQLite persistence with SHA-256 hash-chained commits and a tamper-evident audit trail.
- **Desktop interface:** Svelte 5 panels hosted by Tauri 2 for the dashboard, market data, MT5 bridge, and agent runtime.

## Architecture

ATOS is a local modular monolith. The frontend communicates with the Rust backend through typed Tauri commands. Domain and application logic remain separate from transport and infrastructure adapters.

```text
Svelte 5 presentation
        |
        v
Tauri 2 typed IPC
        |
        v
Rust application and domain layers
        |
        +----------------+----------------+
        |                |                |
        v                v                v
 Trading runtime   Research/backtest   AI research runtime
        |                |                |
        +----------------+----------------+
                         |
                         v
           SQLite / MT5 telemetry / public data
```

Key source areas:

| Path | Responsibility |
|---|---|
| `src-tauri/src/domain/` | Domain models, invariants, and state-transition policy |
| `src-tauri/src/application/` | Backtesting, market-data queries, research workflows, and strict-analysis use cases |
| `src-tauri/src/runtime/trading_core.rs` | Paper-account state, risk enforcement, and persistence lifecycle |
| `src-tauri/src/infrastructure/mt5/` | MT5 telemetry transport and durable market-data ingestion |
| `src-tauri/src/infrastructure/ai/` | AI provider orchestration, credential handling, and agent runtime |
| `src-tauri/src/adapters/tauri/` | Tauri bootstrap and typed command adapters |
| `src/lib/` | Frontend domain, engines, application services, adapters, and presentation |
| `MQL5/Experts/ATOS_Bridge.mq5` | Read-only MetaTrader 5 terminal bridge |

See the [system architecture](docs/03_SYSTEM_ARCHITECTURE.md) for dependency rules, runtime ownership, failure behavior, and known architectural debt.

## Requirements

- Node.js 20 or newer
- Bun for the Hono research API
- Rust toolchain for the native backend
- MetaTrader 5 and MetaEditor only when validating the MT5 bridge on a real terminal

Exact toolchain availability and platform requirements can vary. Release validation must use the repository lockfiles and supported native build environment.

## Getting started

Clone the repository and install dependencies from the repository root:

```bash
git clone https://github.com/Reivhell/ATOS.git
cd ATOS

npm ci --no-audit --fund=false
bun install --frozen-lockfile
```

Build the frontend and run the repository checks:

```bash
npm run build
npm test
cargo test --all-targets
npm run verify:repo
```

Run the frontend development server:

```bash
npm run dev
```

Run the local research API separately when needed:

```bash
bun run api/src/server.ts
```

The desktop app and its native backend require the appropriate Tauri and Rust toolchain. A successful frontend build or JavaScript test run alone does not establish that the complete desktop application or MQL5 bridge is ready for release.

## Verification status

The repository's recorded verification results dated 2026-11-01 are:

| Check | Recorded result |
|---|---|
| Frontend build | Passed |
| TypeScript layer checks | Passed |
| Rust formatting | Passed |
| Rust Clippy | No errors |
| Rust tests | 239 passed |
| Node tests | 375 passed across 52 suites |
| Repository guardrails | Passed |
| Release preflight | Passed |

These are repository-reported results, not a claim that every platform and broker combination has been accepted. MetaEditor compilation, real MT5 terminal behavior, broker-specific margin behavior, and packaged cross-platform installation require their own acceptance checks.

## Safety and current limitations

- **No live order execution:** the current EA is read-only. There is no order submission, modification, close, cancellation, or order-reservation API.
- **Risk checks fail closed:** stale or invalid data, incomplete account inventories, unresolved stop risk, and inconsistent snapshots must not be treated as approval to increase exposure.
- **Account-wide risk is a model, not a guarantee:** calculated stop risk does not include all commissions, swaps, financing, spread widening, slippage, gaps, stop execution failure, or broker liquidation behavior.
- **Margin preflight is indicative:** terminal-calculated candidate margin is a conservative projection and may differ from broker/server rules, aggregate margin tiers, or hedged-position treatment. It does not reserve margin or guarantee later order acceptance.
- **Point-in-time data can change:** account and order snapshots may become stale immediately after a calculation.
- **Local-only bridge:** do not expose the MT5 bridge on a network interface. Remote-terminal support requires a separately reviewed transport and security design.
- **Not production-certified:** native build and test execution, MQL5 compilation, real-terminal and broker acceptance, persistence/crash testing, credential-store acceptance, and release packaging must be completed before production approval.

Never use real trading credentials or rely on this software for live trading while production certification is outstanding.

## Documentation

| Document | Purpose |
|---|---|
| [Product requirements](docs/01_PRD.md) | Product scope, goals, user journeys, and principles |
| [System architecture](docs/03_SYSTEM_ARCHITECTURE.md) | Component boundaries, data ownership, invariants, and limitations |
| [Security and trading risk](docs/08_SECURITY_AND_TRADING_RISK.md) | Security posture and risk controls |
| [Testing and acceptance](docs/09_TESTING_AND_ACCEPTANCE.md) | Test strategy and acceptance criteria |
| [Roadmap](docs/10_ROADMAP.md) | Planned work and product direction |
| [Release attestation pipeline](docs/RELEASE_ATTESTATION_PIPELINE_0.9.6.md) | Release evidence and attestation process |
| [MT5 market-data assurance](docs/MT5_MARKET_DATA_ASSURANCE_AND_QUERY.md) | MT5 data validation and query behavior |
| [Archived reports](docs/archive/) | Historical engineering and validation reports |

## Contributing

Contributions should preserve the project's safety boundaries and architectural rules. Before proposing a change, review the relevant documentation, run the applicable checks, and describe any validation that could not be performed in your environment. Do not represent source-level checks as native or broker acceptance.

## License and security

Review the repository's license files for reuse terms. For security concerns and disclosure guidance, see [SECURITY.md](SECURITY.md).
