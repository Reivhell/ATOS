# Bun runtime migration — sprint report

**Date:** 2026-11-01  
**Repository:** `Reivhell/ATOS`  
**Target branch:** `dev`  
**Working branch:** `migration/bun-runtime-primary`  
**Sprint completion:** **85%**  
**Status:** Bun-first migration checks are passing on the working branch. The branch is not yet merge-ready for `dev`: GitHub reports the PR as non-mergeable and the branch has diverged from `dev`. Tagged desktop release validation also remains pending because PR runs skip the tagged build matrix.

## Baseline and migration results

- The optional local research API already used Bun and Hono; this migration extends Bun to primary JavaScript scripts and CI workflows.
- The committed Bun lockfile installs successfully in frozen mode with `bun install --frozen-lockfile`.
- Frontend typecheck/build, repository guardrails, the full Bun source suite, deterministic agent release qualification, and Hono API integration passed.
- Rust formatting, `cargo test --all-targets`, and `cargo clippy --all-targets --all-features -- -D warnings` passed in the latest PR CI run.
- Latest PR CI run: https://github.com/Reivhell/ATOS/actions/runs/41308834420
- The tagged desktop-build matrix was skipped for an untagged pull request. This run does not certify the tagged release pipeline, MetaEditor, a real MT5 terminal, or live-broker acceptance.

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
- Latest PR CI run `41308834420`: frontend, Bun guardrails/source tests, deterministic release qualification, API integration, Rust formatting, Rust tests, and Clippy all passed. Tagged desktop builds were skipped as expected for an untagged PR.

## Next step

First reconcile the working branch with the current `dev` tree without dropping changes unique to either side, then rerun CI. After the branch is mergeable and CI passes, validate the tagged desktop release path and review its release evidence and artifacts. MT5/MetaEditor and live-broker acceptance remain separate operational gates.
