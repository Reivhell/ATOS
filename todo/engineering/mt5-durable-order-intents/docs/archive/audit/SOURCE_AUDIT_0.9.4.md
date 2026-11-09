# Source audit — Agentic Trading OS v0.9.4

**Date:** 2026-10-29  
**Scope:** `src/`, `api/`, `scripts/`, and `src-tauri/src/`; critical persistence, paper order, kill-switch, provider lifecycle, and AI-agent qualification paths. This is a source review plus automated screening—not a formal security audit or proof that every runtime path is correct.

## Architectural change

`src-tauri/src/state_decision.rs` is called from `TradingCore::persist` after reading the exact prior `app_state.payload` within the same SQLite transaction and before the candidate upsert. A rejected decision aborts persistence. The state contract is allowlisted: unknown changed or removed fields block until explicitly classified.

## Key policy checks reviewed

- First committed state must be an empty paper account with cash equal to initial balance, no synthetic observations/orders/positions/agents, and kill switch engaged.
- `schemaVersion` and `initialBalance` are immutable after bootstrap.
- Embedded event history allows only unique new events at the front and bounded eviction from the oldest end; existing event payloads cannot be rewritten.
- SQLite audit rows have `BEFORE UPDATE` and `BEFORE DELETE` triggers. State and new audit rows are committed in one transaction.
- Cash and exposure changes are gated by order-ledger transitions. A fill requires one new order, matching audit evidence, all required risk rules, atomic order/cash/exposure persistence, and recomputed price, slippage, fee, average cost, quantity, and realized PnL. Historical order records cannot be rewritten.
- All 12 order risk-check names and Boolean outcomes are independently matched against state, including provider provenance, quote freshness, cash/inventory, notional, position cap, portfolio marks, daily loss, and execution mode.
- Risk limit changes, market/screener observations, agent controls, and kill-switch transitions require the corresponding audit evidence. Kill-switch release requires fresh connected provider evidence and fresh quotes for the full current portfolio.
- UTC daily-baseline rollover no longer depends on the equity value changing; equal prior-close and new-day opening equity is valid.
- Paper-account reset restores initial cash, clears orders and positions, and always re-engages kill switch.
- The existing provider refresh exclusive-lease fix remains present so overlapping slow provider refreshes cannot race after the throttle window expires.
- Raw fixture persistence helper is `#[cfg(test)]` only and is not a production runtime bypass.

## Automated source screening

Scan inventory: **93 files, 19,697 lines** across the stated code paths.

| Pattern | Matches |
|---|---:|
| TODO / FIXME / XXX / HACK markers | 0 |
| `eval(` / `new Function(` | 0 |
| direct `.innerHTML =` writes | 0 |
| credential-adjacent `console`/`println!`/`eprintln!` patterns | 0 |

Additionally, `node --check` covered 75 JavaScript-family files with no syntax errors. The application test suite passed 241 assertions and layer typechecking passed.

## Residual risks and known limits

- **Critical validation gap:** no Rust compiler or toolchain was available. The 12 Rust policy unit tests exist but are unexecuted, and the core new policy module is not compiler-verified here.
- Dependency versions are not reproducibly locked (`package-lock.json`, `bun.lock`, `src-tauri/Cargo.lock` absent). Registry lookup failed due DNS `EAI_AGAIN`; no synthetic lockfiles were created.
- `svelte-check`, Vite, Tauri CLI, Cargo, rustfmt and Clippy are missing; no native/frontend production build was generated.
- Provider live integration, OS credential store, platform-specific packaging and signed artifacts remain unverified.
- SQLite triggers and application policy protect against normal application-level mutation, not an attacker with local authority to replace the database or patch the binary. This is not a cryptographically tamper-proof external ledger.
- The pattern scan is heuristic; zero matches do not prove absence of vulnerabilities or exhaustively validate every code path.

## Decision

The v0.9.4 state-control design is stricter than the previous snapshot-only invariants and the available JavaScript checks pass. However, **release readiness remains blocked** until the native/compiler/dependency/integration gates listed in `VALIDATION_REPORT_0.9.4.md` pass on pinned CI. Do not claim production certification on the basis of this audit.
