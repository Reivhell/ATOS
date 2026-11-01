# Validation report — Agentic Trading OS v0.8.8

**Review date:** 2026-10-29  
**Outcome:** **NOT PRODUCTION-CERTIFIED — release preflight blocked**

## Scope delivered

This release hardens the cross-layer AI report boundary. Before deterministic research evaluation, the application now independently checks provider/model identity, UTC generation time, fixed safety note, bounded summary and list fields, exact four-domain observation coverage, domain-specific evidence references, and a recomputed SHA-256 over the normalized commentary document. Changes to report text after the Rust validation boundary fail closed. The independent dataset-bound evidence envelope validation remains in place, and model-authored prose remains excluded from scoring.

## Checks performed in this workspace

| Check | Result | Notes |
|---|---|---|
| `npm test` | **PASS — 206 tests** | Full Node-based repository suite passed, including new AI report integrity regression cases. |
| `npm run typecheck:layers` | **PASS** | Strict configured JS/TS layer check passed. |
| `npm run verify:repo` | **PASS WITH WARNINGS** | Static guardrails passed; dependency lockfiles are absent. |
| `npm run build` | **BLOCKED** | Exit 127: `svelte-check: not found`; frontend dependencies are not installed. |
| `npm run release:preflight` | **BLOCKED — 9 issues** | See blocker list below. |
| Rust `cargo test` / `cargo clippy` / `cargo fmt --check` | **NOT RUN** | Cargo, rustfmt, and Clippy are not available in this workspace. |
| Bun/Hono integration tests | **NOT RUN** | Bun executable is not available in this workspace. |
| Real provider smoke tests | **NOT RUN** | No credentials were supplied and no real provider request was made. |
| Cross-platform keychain/model-import acceptance | **NOT RUN** | Requires supported OS test runners and built application. |
| ZIP integrity and round-trip | **PASS** | Archive integrity and checksum verified; tests (206) and configured layer typecheck passed again from extracted ZIP. |

## Release preflight blockers

1. `package-lock.json` missing; npm dependency graph is not pinned.
2. `bun.lock` missing; Bun dependency graph is not pinned.
3. `src-tauri/Cargo.lock` missing; Rust dependency graph is not pinned.
4. `svelte-check` executable missing.
5. `vite` executable missing.
6. `tauri` CLI executable missing.
7. `cargo --version` fails because Cargo is unavailable.
8. `rustfmt --version` fails because rustfmt is unavailable.
9. `cargo clippy --version` fails because Clippy is unavailable.

## Runtime security semantics

- The app accepts only the fixed provider identifiers already configured in the native runtime.
- AI commentary is strictly bounded and each observation must cite only its corresponding domain reference.
- The application recomputes the native commentary digest and separately validates the score-bearing evidence envelope.
- SHA-256 detects content changes relative to a trusted expected digest; it is **not a signature**, does not authenticate a provider, and does not protect against a compromised native process that can rewrite both content and digest.
- LLM narrative does not affect scoring. `executionAuthority` remains `none`, `liveExecutionAuthorized` remains `false`, and `orderIntent` remains `null`.

## Required gates before production

1. Generate and review all dependency lockfiles in a network-enabled environment, commit them, and use frozen installs.
2. Pass the full frontend build and Rust formatting, test, and Clippy jobs in CI.
3. Add/execute mock-server provider tests for schema errors, status failures, redirects, timeouts, truncated/oversized responses, and budget exhaustion.
4. Test keychain behavior and model import/removal failures on supported operating systems.
5. Complete desktop acceptance, signed packaging, and artifact provenance verification on each supported OS.

Do not deploy with live account credentials or enable live trading on the basis of these Node tests alone.
