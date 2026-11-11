# Bun runtime migration — sprint report

**Date:** 2026-11-01  
**Repository:** `Reivhell/ATOS`  
**Target branch:** `dev`  
**Working branch:** `migration/bun-runtime-primary`  
**Sprint completion:** **90%**  
**Status:** The migration branch was deliberately installed as the `dev` branch head after creating a backup of the previous `dev` head. The branch reconciliation blocker is resolved by replacement, not by a semantic three-way merge. Applicable CI checks pass; tagged desktop release validation remains pending.

## Baseline and migration results

- The optional local research API already used Bun and Hono; this migration extends Bun to primary JavaScript scripts and CI workflows.
- The committed Bun lockfile installs successfully in frozen mode with `bun install --frozen-lockfile`.
- Repository guardrails, the full Bun source suite, deterministic agent release qualification, Hono API integration, and frontend typecheck/build passed.
- Rust formatting, `cargo test --all-targets`, and `cargo clippy --all-targets --all-features -- -D warnings` passed in the latest PR CI run.
- Latest CI run: https://github.com/Reivhell/ATOS/actions/runs/41309680127
- The tagged desktop-build matrix was skipped for an untagged pull request. This run does not certify the tagged release pipeline, MetaEditor, a real MT5 terminal, or live-broker acceptance.
- The previous `dev` head is preserved at `backup/dev-before-overwrite-2026-11-01`.

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
- CI and release commands use Bun for tasks Bun can safely own, with remaining toolchain requirements documented.
- The committed Bun lockfile is checked in frozen mode.
- Repository guardrails and regression tests enforce the new runtime policy.
- Release preflight remains fail-closed.
- Reports distinguish CI results from release and broker-specific checks that have not run.
- Tagged desktop release validation completes successfully before declaring 100%.

## Work log

- Updated package scripts, CI and tagged-release workflow commands, Tauri hooks, release preflight policy, and repository contracts to use Bun.
- Updated release-evidence fixtures to use the CI revision and increased the release-preflight contract test timeout.
- Preserved request IDs on API body-timeout responses and extended the stalled-body integration-test timeout.
- Changed Rust atomic guards from `const` to `static` to respect interior mutability.
- Resolved Clippy findings without weakening the `-D warnings` gate.
- Latest CI run `41309680127`: frontend, Bun guardrails/source tests, deterministic release qualification, API integration, Rust formatting, Rust tests, and Clippy all passed. Tagged desktop builds were skipped as expected for an untagged PR.
- Replaced `dev` with commit `300c5c53cb28327c77355ae53146dc4a81028343`; the previous `dev` head remains recoverable from the backup branch.

## Next step

Validate the tagged desktop release path and review its release evidence and artifacts before declaring 100%. MT5/MetaEditor and live-broker acceptance remain separate operational gates.
