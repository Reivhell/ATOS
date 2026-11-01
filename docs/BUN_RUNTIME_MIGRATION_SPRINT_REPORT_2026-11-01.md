# Bun runtime migration — sprint report

**Date:** 2026-11-01  
**Repository:** `Reivhell/ATOS`  
**Base branch:** `main`  
**Working branch:** `migration/bun-runtime-primary`  
**Sprint completion:** **70%**  
**Status:** Bun-first workflow is implemented. Frontend, Bun guardrails/tests, API integration, Rust formatting, and Rust tests pass in PR CI; Clippy remains blocked by existing lint debt.

## Baseline and migration results

- The optional local research API already used Bun and Hono; the migration extends Bun to the primary JavaScript scripts and CI workflow.
- A `bun.lock` lockfile is committed and CI successfully installs dependencies with `bun install --frozen-lockfile`.
- Frontend typecheck/build, repository guardrails, the full JavaScript source suite, deterministic agent release qualification, and Hono API integration passed on the migration branch.
- Rust formatting and Rust tests passed in CI. Clippy failed with 42 library and 44 library-test lint errors, including unused mutability, redundant pattern matching, range checks, unnecessary clones, dead code, and too-many-arguments findings across existing application and MT5 modules.
- The Clippy failures span unrelated modules and are not evidence of a Bun runtime regression. The gate remains enabled; it must not be weakened just to obtain a green run.
- The tagged desktop-build matrix is skipped for an untagged pull request. This CI run does not certify MetaEditor, a real MT5 terminal, or live-broker acceptance.

## Sprint scope

1. Make Bun the documented and automated default for JavaScript/MJS execution and test scripts.
2. Move CI's primary source-test and repository-guardrail path to Bun.
3. Align frontend build, Tauri CLI invocation, release evidence, installer staging, and attestation verification with Bun where supported.
4. Keep release preflight fail-closed and update repository contract tests for the runtime policy.
5. Retain the npm lockfile as a transition safeguard until the Bun-first tagged release path is validated.
6. Verify the complete Bun suite, API integration, frontend build, Rust checks, and tagged release path; keep MT5 acceptance separate.

## Acceptance criteria

- A clean checkout installs dependencies with `bun install --frozen-lockfile`.
- Supported scripts and JavaScript/API test suites pass under Bun in CI.
- CI and release commands no longer require Node/npm for tasks Bun can safely own; remaining toolchain requirements are documented.
- The committed Bun lockfile is checked in frozen mode.
- Repository guardrails and regression tests enforce the new runtime policy.
- Release preflight remains fail-closed.
- Reports distinguish CI results from release and broker-specific checks that have not run.

## Work log

- Created working branch `migration/bun-runtime-primary`.
- Updated package scripts, CI and tagged-release workflow commands, Tauri hooks, release preflight policy, and related repository contracts to use Bun.
- Fixed release-evidence test fixtures to use the CI revision and increased the release-preflight contract test timeout.
- Preserved request IDs on API body-timeout responses and extended the stalled-body integration-test timeout.
- Changed two mutable Rust atomic guards from `const` to `static` after Clippy exposed the interior-mutability issue.
- Latest PR CI run `41305860063`: frontend build passed; Bun guardrails/full source tests passed; Bun/Hono integration passed; Rust formatting and tests passed; Clippy failed on existing lint debt. Tagged desktop builds were skipped as expected for an untagged PR.

## Next step

Decide whether to open a separate, scoped Rust lint-cleanup change. Keep the Bun migration PR focused, preserve the Clippy gate, and run tagged-release validation before calling the migration complete.
