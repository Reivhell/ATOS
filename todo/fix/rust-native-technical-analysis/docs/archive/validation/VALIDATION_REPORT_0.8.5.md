# Validation report — Agentic Trading OS 0.8.5

Date: 2026-10-29
Status: **candidate only; not production-certified**

## Architectural changes

- `research-decision.v3` emits a deterministic `decision-provenance.v1` diagnostic artifact.
- The provenance graph links the dataset snapshot, accepted signal-domain nodes, and evidence-reference nodes.
- Per-domain signed score contributions expose how the aggregate directional scores are constructed.
- Explicit gate traces report prerequisite, signal integrity, distinct-domain coverage, directional coverage, mean strength, directional agreement, and score separation.
- Evidence graph node identifiers are namespaced/encoded to avoid collisions with internal graph identifiers.
- Prior v0.8.4 protections remain: at most 16 signals, at most 32 evidence references per signal, maximum reference length 256, duplicate reference rejection, and cross-domain reference reuse blocking.
- Safety invariants remain: no order intent, execution authority `none`, and live execution disabled.

## Validation executed

| Check | Result |
|---|---|
| `npm test` | PASS — 174 tests, 0 failures |
| `npm run test:decision-engine` | PASS — 11 tests, 0 failures |
| `npm run typecheck:layers` | PASS |
| `npm run verify:repo` | PASS with lockfile warnings |
| `npm run release:preflight` | BLOCKED — 9 release-readiness blockers |
| `npm run build` | BLOCKED — `svelte-check: not found` |

## Release blockers not bypassed

- `package-lock.json` is missing.
- `bun.lock` is missing.
- `src-tauri/Cargo.lock` is missing.
- Installed `svelte-check`, `vite`, and `tauri` executables are unavailable in this environment.
- `cargo`, `rustfmt`, and `cargo clippy` are unavailable in this environment.

No lockfiles were fabricated, and no Rust/native tests, Svelte production build, Tauri package, or cross-platform acceptance test is claimed as passed.

## Known integrity boundary

The graph is deterministic and diagnostic; it is **not a cryptographic attestation**. Upstream evidence references remain caller-supplied identifiers. This layer cannot establish the authenticity of source observations, semantic independence between aliases, or correctness of upstream feature calculations. A trusted canonical evidence registry with content digests and derivation lineage remains necessary before stronger provenance claims can be made.

## Production assessment

This change improves decision explainability and auditability, but the repository remains a release candidate. Do not deploy as a production trading system until dependency lockfiles are generated through the intended package managers, the required toolchains are installed, the complete frontend/native build and tests run in CI, and runtime/security/operational acceptance criteria are met.
