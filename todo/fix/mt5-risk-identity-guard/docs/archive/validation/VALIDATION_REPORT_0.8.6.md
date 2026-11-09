# Validation report — Agentic Trading OS 0.8.6

Date: 2026-10-29

## Scope

Introduced a content-addressed canonical evidence registry and made it a required input for decision-signal acceptance. The registry normalizes JSON deterministically, hashes canonical evidence with Web Crypto SHA-256, resolves parent derivation references, rejects cycles and missing parents, bounds memory/CPU inputs, and freezes its output. Decision provenance now records the registry digest and verification state.

## Tests

- Canonical evidence registry suite: 7 tests covering stable canonicalization, alias convergence, derivation lineage, missing parent rejection, cycle rejection, invalid payload/scope rejection, immutability, anti-cloning process-local verification, strict calendar timestamps, and accessor/sparse-array rejection.
- Decision engine suite: 12 tests covering consensus-only research candidates, provenance binding, conflicts, paper-risk requirements, rejection of live intent, canonical evidence reuse, bounded inputs, graph determinism, blocked gate traces, and fail-closed behavior when the registry is absent.
- Full repository tests: 182 passed, 0 failed. Strict layer typecheck: passed. Repository static guardrails: passed with dependency-lock warnings. Release preflight: blocked by 9 environment/reproducibility issues; frontend build: blocked because `svelte-check` is unavailable. Exact environment blockers are listed below.

## Known release blockers

This workspace lacks `package-lock.json`, `bun.lock`, and `src-tauri/Cargo.lock`; `svelte-check`, Vite, and Tauri CLI are not installed; and `cargo`, `rustfmt`, and `cargo clippy` are unavailable. Do not invent lockfiles or claim frontend/native builds passed. Re-run release preflight and report current exact results after the Node test suite.

## Security limitations

Canonical SHA-256 identities protect against alias-based duplicate counting when canonical payloads are equal, and parent IDs bind derivation lineage. They do not establish provider authenticity or statistical independence of semantically related payloads. Registry verification state is process-local and must be reconstructed from descriptors after serialization. No order intent or live-execution authority is introduced.

## Application integration status

The current frontend application service does not yet receive a Rust-issued canonical dataset fingerprint and does not build this async registry as part of the analysis request. This is deliberate fail-closed behavior: the application-level decision path cannot produce an accepted research candidate until trusted dataset identity and registry construction are integrated at the ingestion/application boundary. The new registry contract is implemented and tested as a domain/decision boundary, but end-to-end app integration is not complete; this is a release blocker, not a claim of completed production rollout.
