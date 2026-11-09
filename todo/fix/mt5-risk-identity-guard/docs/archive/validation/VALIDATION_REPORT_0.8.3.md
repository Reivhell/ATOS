# Validation report — Agentic Trading OS 0.8.3

**Candidate:** Authentication-aware rate-limit isolation  
**Reviewed:** 2026-10-29  
**Release status:** **BLOCKED — not production-certified**

## Scope delivered

- Separated failed bearer-authentication attempts (10 admitted failures per 60-second sliding window) from the authenticated API request budget (30 requests per 60-second sliding window).
- Invalid credentials receive 401 until the failed-auth bucket is exhausted, then 429 with `Retry-After`. Invalid credentials do not consume the authenticated request budget.
- Kept both rate-limit buckets fixed-memory and process-local. No unbounded client/IP-keyed map was introduced.
- Added dependency-free Node tests for bucket isolation, exact window expiry, capacity limits, and invalid configuration.
- Added a Bun/Hono HTTP integration test which sends 40 invalid-token requests and then checks a valid token still succeeds.
- Added CI job timeouts and cancellation of stale push/PR workflows.
- Updated version metadata, changelog, user documentation, and repository guardrails.

## Checks performed in this workspace

| Check | Result | Notes |
|---|---|---|
| `npm test` | PASS — 169/169 | 28 dependency-free Node test scripts/suites, 0 failures |
| `npm run verify:repo` | PASS with warnings | Warns that all three lockfiles are missing; static guardrails are not native-build evidence |
| `npm run typecheck:layers` | PASS | Strict JS/TS layer typecheck |
| `npm run build` | BLOCKED | `svelte-check: not found`; frontend dependencies are not installed |
| `npm run release:preflight` | BLOCKED — 9 blockers | See list below |
| Bun/Hono `api/test/app.integration.test.ts` | NOT RUN locally | Bun and project dependencies are unavailable; CI contains this suite but has not been run from this environment |
| Rust format/tests/Clippy and native desktop builds | NOT RUN | Rust toolchain unavailable |
| Desktop cross-platform acceptance | NOT RUN | No Windows/macOS/Linux release acceptance run was available |
| npm registry install | BLOCKED | Registry/network unavailable in this workspace; no lockfile was fabricated |

## Release blockers from preflight

1. Missing `package-lock.json`.
2. Missing `bun.lock`.
3. Missing `src-tauri/Cargo.lock`.
4. Missing installed `svelte-check` executable.
5. Missing installed `vite` executable.
6. Missing installed `tauri` executable.
7. `cargo --version` failed because Cargo is unavailable.
8. `rustfmt --version` failed because rustfmt is unavailable.
9. `cargo clippy --version` failed because Cargo/Clippy is unavailable.

Manifest versions were checked by the preflight and are aligned at `0.8.3`. Do not treat the dependency-free suite or static guardrails as proof that the Bun/Hono server, Svelte frontend, Rust engine, or Tauri binaries compile and run.

## Operational boundaries

- The API remains local, bearer-authenticated, browser-Origin rejecting, research-only, and has no order route/live-trading authority.
- The new failed-auth budget is **global to one process**, not per IP/user and not distributed across multiple processes. This deliberately avoids unbounded attacker-controlled maps; deployment assumptions must remain single-process loopback or be revisited before any multi-process exposure.
- A tagged release must remain blocked until the lockfiles are generated using the actual package managers, CI/runtime integration succeeds, frontend and native builds pass, and desktop acceptance evidence is recorded.
