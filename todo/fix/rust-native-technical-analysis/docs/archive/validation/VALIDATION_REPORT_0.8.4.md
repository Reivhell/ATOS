# Validation report — Agentic Trading OS 0.8.4

Date: 2026-10-29
Focus: Decision-layer evidence independence and bounded decision inputs.

## Implemented

- Decision contract upgraded to `research-decision.v2`; policy identifier upgraded to `research-decision-policy.v2`.
- The same evidence reference cannot be accepted under multiple technical domains in one decision evaluation. Cross-domain reuse blocks the verdict, preventing a single observation from being counted as independent votes.
- Input guardrails cap the supplied signal array at 16, evidence references at 32 per signal, and each reference at 256 characters.
- Regression tests cover cross-domain evidence reuse, oversized signal batches, excessive reference counts, and overlong references.
- Application manifests are aligned to 0.8.4.

## Verification results

- `npm test`: PASS, 172 tests across the repository's Node test suites, 0 failures.
- `node --test scripts/test-research-decision-engine.test.mjs`: PASS, 9/9 tests.
- `npm run typecheck:layers`: PASS.
- `npm run verify:repo`: PASS with explicit warnings for missing lockfiles.
- `npm run release:preflight`: BLOCKED, 9 issues.
- `npm run build`: BLOCKED, `svelte-check: not found`.

## Release blockers

1. `package-lock.json` is missing.
2. `bun.lock` is missing.
3. `src-tauri/Cargo.lock` is missing.
4. Frontend executable `svelte-check` is unavailable.
5. Frontend executable `vite` is unavailable.
6. Frontend executable `tauri` is unavailable.
7. `cargo --version` fails because Cargo is unavailable.
8. `rustfmt --version` fails because rustfmt is unavailable.
9. `cargo clippy --version` fails because Cargo/Clippy is unavailable.

Bun/Hono integration tests, Rust unit tests/Clippy, full frontend build, Tauri packaging, and cross-platform desktop acceptance were not run in this environment. Lockfiles were not fabricated. This artifact is **not production-certified**.

## Decision-layer limitations

Unique reference strings prevent direct reuse but do not prove statistical independence. Upstream producers must assign canonical evidence IDs from trusted observations; semantic aliases can still refer to the same underlying fact unless normalized upstream. This is a research-only decision engine and never grants order or live execution authority.
