# Source audit — Agentic Trading OS v0.9.5

**Date:** 2026-10-29  
**Scope:** `src/`, `api/`, `scripts/`, and `src-tauri/src/`; durable persistence, native state-decision, paper-order, provider lifecycle, AI-agent qualification, and new state-commit chain.

## Architectural change: durable state commit chain

`src-tauri/src/engine.rs` now records a SHA-256 receipt for each accepted persisted state transition. Each receipt binds a monotonically increasing sequence, commit timestamp, native decision transition, exact serialized state-payload digest, previous receipt hash, and its own record hash. State payload, receipt, and embedded audit rows are written in the same SQLite transaction. Update/delete triggers enforce append-only behavior for normal SQL mutation.

Before a subsequent write, the engine verifies receipt sequence continuity, predecessor linkage, record hashes, and the latest receipt's payload digest. Startup performs the same verification before generic corruption recovery can move a database aside, preventing an established chain mismatch from being silently treated as ordinary recoverable corruption. SQLite `user_version` 2 marks chain activation; schema-v2 databases require a non-empty chain and refuse startup if the receipt table or chain is missing. Existing v1 databases without receipts are anchored at the next accepted commit with `LEGACY_BASELINE_UNVERIFIED`; pre-anchor history is not represented as verified.

A `#[cfg(test)]` fixture writer appends a clearly labelled `TEST_FIXTURE_BYPASS` receipt so tests can construct intentionally invalid state without breaking chain bookkeeping. It is not compiled into production runtime. Native regressions cover valid-JSON payload tampering and removal of an activated chain.

## Other retained policy controls

- Native state-commit decision policy remains the gate for every durable state write.
- Paper fills must atomically reconcile order, cash, quantity, cost basis, fees, and realized PnL with the prior snapshot.
- Risk evidence is independently re-evaluated against the previous state; unknown changed fields fail closed.
- Audit history and order records are append-only; kill-switch release requires fresh connected provider evidence and fresh portfolio quotes.
- Provider refresh requests retain exclusive in-flight leases to prevent overlapping stale responses.
- Process startup re-engages the kill switch, and account reset always re-engages it.

## Automated source screening

Inventory: **93 source files / 19,872 lines** across the declared source roots. `node --check` covered **75 JavaScript-family files**.

| Pattern | Matches |
|---|---:|
| TODO / FIXME / XXX / HACK markers | 0 |
| `eval(` / `new Function(` | 0 |
| direct `.innerHTML =` writes | 0 |
| credential-adjacent logging patterns | 0 |

These are heuristic source scans, not a formal security audit or proof that every control-flow path is correct.

## Validation status

- Node suite: 242 tests across 35 npm test-script blocks, 0 failures.
- `npm run typecheck:layers`: passed.
- `npm run verify:repo`: passed with missing-lockfile warnings.
- Agent qualification: 10/10 scenarios passed, `policyPassed=true`; live provider integration remains untested.
- Native Rust test, formatting, Clippy, frontend production build, Tauri package, OS keychain and cross-platform acceptance are not verified in this workspace because the toolchain/dependencies are unavailable.

## Security limits

The chain is **tamper-evident, not tamper-proof**. A local administrator who can replace the whole database, remove triggers, and recompute all hashes can forge a new chain because there is no external notarization or hardware-backed signature. Existing operating-system access control, secure backups, and endpoint protections remain required. See `docs/STATE_COMMIT_CHAIN_0.9.5.md`.

## Release decision

**RELEASE BLOCKED — NOT PRODUCTION-CERTIFIED.** Native compilation and integration evidence must be produced by pinned CI before deployment. The local source-contract suite cannot substitute for compiling and executing the new Rust persistence code.
