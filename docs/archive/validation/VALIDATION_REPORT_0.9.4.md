# Validation report — Agentic Trading OS v0.9.4

**Date:** 2026-10-29  
**Status:** **RELEASE BLOCKED — NOT PRODUCTION-CERTIFIED**

## Summary

The v0.9.4 source adds a fail-closed native state-commit decision layer and fixes paper-account reset so every reset re-engages the kill switch. Existing provider single-flight refresh leases remain in place. The JavaScript and TypeScript checks run in this workspace pass, but the native Rust toolchain and the application build dependencies are unavailable. Therefore this report does not claim native compilation, native unit-test success, or a shippable desktop build.

## Verified in this workspace

| Check | Result | Details |
|---|---|---|
| `npm test` | PASS | 35 npm test script blocks; 241 assertions passed, 0 failed |
| `node --test scripts/test-state-decision-layer.test.mjs` | PASS | 3 tests passed, 0 failed; checks write-path wiring and reset kill-switch contract |
| `npm run typecheck:layers` | PASS | Strict TypeScript checking of the declared application/domain layers |
| `npm run verify:repo` | PASS with warnings | Static repository guardrails passed; it explicitly warns that dependency lockfiles are missing |
| `npm run eval:agents -- --report-file validation/agent-evaluation-0.9.4.json` | PASS | 10/10 scenarios; 0 failures; no category gaps; `policyPassed=true` |
| Agent provider acceptance | NOT TESTED | Evaluation report records `providerLiveIntegrationTested=false` |
| JavaScript syntax scan | PASS | 75 `.js`, `.mjs`, and `.cjs` files checked with `node --check`; 0 syntax failures |
| Automated source-pattern scan | PASS, limited scope | 93 code files / 19,697 lines; no matches for TODO/FIXME/XXX/HACK, `eval(`/`new Function`, direct `.innerHTML =`, or credential-adjacent logging patterns |
| Version manifest alignment | PASS | Release preflight reports all declared manifests aligned to `0.9.4` |
| Release ZIP integrity | PASS | `unzip -t` reports no archive errors; external SHA-256 manifest verifies; tests and typecheck rerun against an extracted copy of the exact ZIP with the same results |

## Native policy tests added but NOT executed

`src-tauri/src/state_decision.rs` contains 12 Rust unit tests, including bootstrap rejection, risk policy audit evidence, safe kill-switch release, safe reset, order/cash/position atomicity, ledger immutability, provider event evidence, unknown field transitions, append-only event history, and UTC daily-baseline rollover. These test cases are source code only until a Rust compiler/test runner executes them.

## Release preflight — blocked by 9 issues

1. Missing `package-lock.json`.
2. Missing `bun.lock`.
3. Missing `src-tauri/Cargo.lock`.
4. Missing executable `svelte-check`.
5. Missing executable `vite`.
6. Missing executable `tauri`.
7. `cargo --version` unavailable.
8. `rustfmt --version` unavailable.
9. `cargo clippy --version` unavailable.

An attempt to query the npm registry failed with DNS resolution error `EAI_AGAIN`, so a dependency lockfile was not fabricated and missing build tools could not be installed from the registry. The preflight exits with code 1 by design.

## Not verified / prohibited claims

- No Rust compile, `cargo test`, `rustfmt`, or Clippy run was possible.
- No Svelte `svelte-check`, Vite production build, or Tauri desktop bundle was produced.
- No live public-provider integration acceptance or OS-keychain integration acceptance was performed in this session.
- No signed or cross-platform release artifact was built.
- Static tests and scans do not prove memory safety, all state-path correctness, security, or production readiness.

## Release decision

**Do not deploy or label v0.9.4 production-ready yet.** Before approval, run the full CI matrix in a network-enabled, pinned environment: generate and review dependency lockfiles from real resolution, install pinned Node/Bun/Rust toolchains, run Rust unit tests and Clippy, run `svelte-check` and Vite build, execute provider/keychain acceptance tests on supported operating systems, then produce and verify signed platform artifacts. The application remains local paper/research tooling and this change adds no live execution authority.
