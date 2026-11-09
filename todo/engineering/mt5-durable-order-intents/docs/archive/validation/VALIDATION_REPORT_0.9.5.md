# Validation report — Agentic Trading OS v0.9.5

**Date:** 2026-10-29  
**Status:** **RELEASE BLOCKED — NOT PRODUCTION-CERTIFIED**

## Summary

v0.9.5 adds a durable SHA-256 state-commit receipt chain to the native SQLite persistence path. Each accepted state transition is recorded atomically with the state payload and embedded audit rows. Existing chains are checked on startup before general corruption recovery, and before every subsequent durable write. A mismatch fails closed rather than silently overwriting or resetting an established account.

Legacy databases without a receipt chain are compatible: the next accepted commit records their then-current state as `LEGACY_BASELINE_UNVERIFIED`. This intentionally does not claim that historical state before the anchor was verified.

## Verified in this workspace

| Check | Result | Details |
|---|---|---|
| `npm test` | PASS | 242 tests across 35 npm test-script blocks; 0 failures |
| State-decision source-contract tests | PASS | Includes receipt-chain persistence, pre-recovery verification wiring, append-only triggers, and tamper regression test presence |
| `npm run typecheck:layers` | PASS | Strict TypeScript checking for declared application/domain layers |
| `npm run verify:repo` | PASS with warnings | Static repository guardrails pass; lockfiles are missing |
| Agent qualification | PASS | 10/10 scenarios; no category gaps; `policyPassed=true` |
| Live AI provider integration | NOT TESTED | Evaluation reports `providerLiveIntegrationTested=false` |
| JavaScript syntax scan | PASS | 75 `.js`, `.mjs`, `.cjs` files pass `node --check` |
| Source-pattern scan | PASS, limited scope | 93 files / 19,872 lines; no matches for TODO/FIXME/XXX/HACK, dynamic eval, direct `innerHTML` assignment, or credential-adjacent logging patterns |
| Version alignment | PASS | `package.json`, Tauri config, Cargo manifest, UI version, and runtime User-Agent tests align to `0.9.5` |

## Native Rust test added but not executed

`src-tauri/src/engine.rs` adds `durable_state_commit_chain_detects_payload_tampering_and_rejects_receipt_mutation` and `schema_v2_refuses_startup_when_the_receipt_chain_is_removed`. They cover immutable receipt triggers, valid-JSON payload tampering, and failure-closed startup if the chain is emptied after schema-v2 activation. The existing 12 native state-decision tests also remain in source.

**Important:** neither the new Rust test nor the other native tests were executed locally. There is no `cargo`/`rustc`/`rustfmt`/Clippy toolchain in this workspace. Source-contract tests confirm wiring, not compilation or runtime correctness.

## Release preflight — blocked by 9 issues

1. Missing `package-lock.json`.
2. Missing `bun.lock`.
3. Missing `src-tauri/Cargo.lock`.
4. Missing installed `svelte-check` executable.
5. Missing installed `vite` executable.
6. Missing installed `tauri` executable.
7. `cargo --version` unavailable.
8. `rustfmt --version` unavailable.
9. `cargo clippy --version` unavailable.

`npm run release:preflight` exits nonzero by design. `npm run verify:repo` passes only its static guardrails and explicitly warns about lockfiles. Registry access was unavailable in this workspace, so no lockfiles were fabricated.

## Required before production deployment

- Generate and commit genuine dependency lockfiles using the supported package managers in a network-enabled, pinned CI environment.
- Run `cargo fmt --check`, `cargo test --locked --all-targets`, and `cargo clippy --locked --all-targets --all-features -- -D warnings`.
- Run frozen dependency installs, Bun/Hono integration, strict frontend typecheck/build, and Tauri packaging.
- Run native integration tests for existing-database migration, interrupted commits, payload/receipt tampering, process restart, and platform-specific keychain behavior.
- Verify packaged artifact inventory and checksums, and exercise the release workflow on Linux, Windows, and macOS.

## Threat model

The receipt chain detects accidental corruption and ordinary mutation while the chain and triggers remain intact. It is not a cryptographically tamper-proof ledger against a local administrator who can rewrite the whole database and recompute hashes. External notarization or signed anchoring would be required for that stronger guarantee.

No live-trading or order-execution authority was added. **Do not deploy this release as production-certified until the blocked native, dependency, build, and platform gates pass.**
