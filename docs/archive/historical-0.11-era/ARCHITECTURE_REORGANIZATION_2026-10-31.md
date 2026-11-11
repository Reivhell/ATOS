# Architecture Reorganization Record — 2026-10-31

## Objective

Move the backend away from a flat Rust source directory and a multi-responsibility Tauri `lib.rs` toward explicit layer boundaries without changing the existing IPC command names, paper-account semantics, MT5 data contracts, or EA installation path. This is an architectural restructuring of the existing implementation, not a new trading capability release.

## Before and after

Before, ten large Rust implementation files lived directly under `src-tauri/src/`. `lib.rs` contained the application bootstrap, Tauri command handlers, provider HTTP refresh/parsing and authoritative strict-analysis snapshot assembly. The frontend already had domain/engine/application/adapter/presentation/infrastructure directories, but the Rust tree did not mirror those concerns.

After, Rust code is organized by responsibility:

```text
src-tauri/src/
  lib.rs
  main.rs
  adapters/
    tauri/
      bootstrap.rs
      commands.rs
  application/
    backtest.rs
    market_data_query.rs
    research_workspace.rs
    strict_analysis.rs
    strict_analysis_service.rs
  domain/
    models.rs
    research_smc.rs
    state_decision.rs
  runtime/
    trading_core.rs
  infrastructure/
    ai/agent_runtime.rs
    mt5/bridge.rs
    mt5/market_data.rs
    providers/public_market.rs
```

The frontend remains organized under `src/lib/domain`, `engines`, `application`, `adapters/tauri`, `infrastructure`, and `presentation`; Svelte panels remain under `src/lib/components`.

## Source migration map

| Previous source path | Current source path | Responsibility |
|---|---|---|
| `src-tauri/src/engine.rs` | `src-tauri/src/runtime/trading_core.rs` | Account runtime, paper risk/order transitions, account persistence |
| `src-tauri/src/models.rs` | `src-tauri/src/domain/models.rs` | Shared business/data contracts |
| `src-tauri/src/state_decision.rs` | `src-tauri/src/domain/state_decision.rs` | Account state-transition policy |
| `src-tauri/src/research_smc.rs` | `src-tauri/src/domain/research_smc.rs` | Deterministic SMC research primitives |
| `src-tauri/src/backtest.rs` | `src-tauri/src/application/backtest.rs` | Backtest use cases and evaluation |
| `src-tauri/src/strict_analysis.rs` | `src-tauri/src/application/strict_analysis.rs` | Strict-analysis work plans, assessment and audit gate |
| `src-tauri/src/research_workspace.rs` | `src-tauri/src/application/research_workspace.rs` | Research hypotheses, workflows and run-card evidence |
| `src-tauri/src/ai_agent_runtime.rs` | `src-tauri/src/infrastructure/ai/agent_runtime.rs` | Provider, credential, model artifact and agent runtime adapter |
| `src-tauri/src/mt5_bridge.rs` | `src-tauri/src/infrastructure/mt5/bridge.rs` | Authenticated local MT5 telemetry transport |
| `src-tauri/src/mt5_market_data.rs` | `src-tauri/src/infrastructure/mt5/market_data.rs` | Durable MT5 ingestion and dataset query store |
| Tauri handlers from `lib.rs` | `src-tauri/src/adapters/tauri/commands.rs` | IPC request/response adapter |
| Builder/managed state from `lib.rs` | `src-tauri/src/adapters/tauri/bootstrap.rs` | Application composition and command registration |
| Provider refresh/parser from `lib.rs` | `src-tauri/src/infrastructure/providers/public_market.rs` | Bounded public provider integration |
| Strict authoritative series assembly from `lib.rs` | `src-tauri/src/application/strict_analysis_service.rs` | Application-level trusted snapshot assembly |

The Tauri command identifiers were preserved. The command adapter now declares 35 commands; bootstrap registers those same 35 names exactly once. The composition root is 11 lines and contains only module declarations plus the `run` export. Unused legacy crate-level module aliases were removed so internal code cannot silently keep depending on the old flat module API.

## Architectural controls added

The canonical Node architecture suite now verifies:

- Rust layer files exist at their intended paths.
- `lib.rs` remains a small composition root and does not own Tauri handlers, provider HTTP, or persistence behavior.
- The IPC adapter does not open SQLite or construct HTTP clients.
- Public provider integration is independent of Tauri IPC.
- Shared domain contracts do not import Tauri, HTTP, SQLite, keyring or browser fetch APIs.
- Existing frontend compatibility files remain small forwarding shims rather than duplicate implementations.
- Every `#[tauri::command]` declaration has exactly one bootstrap registration and there are no stale registrations.
- All compile-time `include_str!` assets in strict analysis resolve after the move.

## Regression and static verification

Latest completed workspace run:

- `npm test`: 295 tests, 0 failures, across 44 command groups.
- `npm run typecheck:layers`: passed.
- `npm run verify:repo`: passed with three warnings for missing dependency lockfiles.
- `npm run release:preflight`: blocked with nine issues.
- Tauri command parity: 35 declarations, 35 registrations, no duplicates.
- Strict-analysis `include_str!`: 10 paths resolved.
- JavaScript syntax scan: 89 files, 0 failures.
- JSON syntax scan: 18 files, 0 failures.

The Node tests are source/contract tests. They do not compile Rust or prove that the native binary links. `cargo check`, `cargo test`, `cargo fmt --check`, Clippy, Svelte/Vite production build, Tauri packaging, MQL5 MetaEditor compilation, real terminal integration and broker acceptance were not run because the respective toolchains/terminal are unavailable in this workspace.

## Unfinished architecture work

This reorganization creates enforceable module boundaries but deliberately does not disguise the remaining large aggregates as finished hexagonal architecture.

1. `runtime/trading_core.rs` remains a large aggregate that contains account state lifecycle, SQLite schema/migrations, instance lock, state persistence, audit commit chain and paper risk/order operations. Extract a repository/persistence adapter only with executable native regression tests for transaction atomicity, process lock, backup/recovery and hash-chain continuity.
2. `application/research_workspace.rs` still contains workflow policy, statistical analysis and SQLite persistence. Split contracts/policy, statistical evaluator and repository without changing immutable run-card semantics.
3. `application/strict_analysis.rs` still contains strict-analysis contracts, work-plan lifecycle, specialist dispatch checks and SQLite audit persistence. Extract a dedicated audit repository and contract validation module.
4. `infrastructure/ai/agent_runtime.rs` still combines provider routing, credential lifecycle, imported artifact registry and event audit storage. Split provider clients, secret-store port, artifact registry and audit repository while preserving allowlists, bounds and zeroization.
5. `infrastructure/providers/public_market.rs` still combines provider request construction, parsing/normalization and refresh orchestration. Split transport clients and normalized market-data contracts from runtime state updates.
6. Add explicit application ports where they resolve actual dependencies, then inject concrete adapters in `bootstrap.rs`. Avoid speculative trait proliferation.
7. Add broker-session/timezone/DST/holiday rules before using time-gap analysis as a market-closure oracle.
8. Design and review a secure local IPC transport before allowing MT5 terminal connectivity from another host. The current bridge is intentionally loopback-only and is not encrypted for network transit.

## Release and execution boundary

No live order execution was added. `MQL5/Experts/ATOS_Bridge.mq5` remains the read-only telemetry EA. The existing paper order path remains under native risk controls. AI cannot create or sign an execution mandate, change risk limits, or bypass the risk gateway.

The package remains an engineering candidate, not a production-certified release. The nine preflight blockers are:

- Missing `package-lock.json`, `bun.lock`, and `src-tauri/Cargo.lock`.
- Missing `svelte-check`, Vite, and Tauri CLI executables.
- Missing Cargo, Rustfmt, and Cargo Clippy commands.

Do not remove release gates or fabricate lockfiles to make the preflight green. Resolve dependency locks through reviewed package resolution, restore the supported build toolchains, execute native and frontend gates in CI, compile the EA with MetaEditor, and perform controlled MT5 terminal acceptance before certification.
