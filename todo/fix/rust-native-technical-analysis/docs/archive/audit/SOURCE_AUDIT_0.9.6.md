# Source audit — Agentic Trading OS v0.9.6

**Date:** 2026-10-30  
**Disposition:** Source checks pass; release is **BLOCKED / NOT PRODUCTION-CERTIFIED**.

## Scope and inventory

- 105 JavaScript, TypeScript, Svelte, and Rust logic files; 20,650 source lines in the scanned extensions and project tree.
- `node --check` applied to all 85 `.js`, `.mjs`, and `.cjs` files.
- GitHub Actions workflow parsed as YAML and its five declared jobs were checked: `repository-guardrails`, `bun-compatibility`, `frontend`, `rust`, and `tagged-release-build`.
- Static scan patterns were evaluated line by line, so a logger on one line does not combine with unrelated text on a later line.

## Source-level safety and release-boundary findings

- 0 matches for TODO/FIXME/XXX/HACK markers.
- 0 matches for direct `eval(...)` / `new Function(...)` construction patterns.
- 0 matches for direct `.innerHTML =` or `dangerouslySetInnerHTML` patterns.
- 0 matches for the audited credential-adjacent logging pattern.
- Strict installer staging accepts only platform-required installer formats, rejects symlinks, unsafe or empty payloads, filename collisions and reused/overlapping staging roots; staging emits SHA-256 inventory data.
- Release attestation verification requires both SLSA provenance (`https://slsa.dev/provenance/v1`) and SPDX 2.3 (`https://spdx.dev/Document/v2.3`) for each installer, constraining exact repo, source tag, signer workflow, OIDC issuer and hosted-runner policy.
- Added fail-closed policy and tests for GitHub Actions immutable pins: full 40-character commit SHAs are accepted; moving tags, branches and abbreviated hashes are rejected. Because the local environment could not independently resolve full commit IDs for every third-party action, existing moving refs are reported as a release blocker instead of guessed hashes being introduced.
- Existing native state-commit decision layer and tamper-evident local receipt chain remain in the source. A local SHA-256 chain is tamper-evident, not tamper-proof against an administrator who can rewrite the database and recompute the chain.
- Agent release qualification reports `executionAuthority: none`, `liveExecutionAuthorized: false`, and `orderIntent: null`; live venue execution is not enabled by this work.

## Executed checks

| Check | Result |
|---|---|
| `npm test` | PASS — 259/259 tests across 40 npm test command groups |
| `npm run typecheck:layers` | PASS |
| `npm run verify:repo` | PASS with warnings for missing dependency lockfiles |
| `npm run eval:agents -- --report-file validation/agent-evaluation-0.9.6.json` | PASS — 10/10 scenarios, no category gaps, policy passed |
| JavaScript syntax | PASS — all 85 JS/MJS/CJS files checked |
| CI YAML parse and job contract | PASS — 5 jobs parsed |
| Automated source-pattern scan | PASS for the limited patterns above; not a proof of security or manual line-by-line human review |
| `npm run release:preflight` | BLOCKED — 10 issues, intentionally fail-closed |

## Exact distribution archive check

The final ZIP was extracted into a clean directory and the full test suite, strict layer typecheck, and repository guardrails were rerun against the extracted files. Results were identical: 259/259 tests passed, typecheck passed, guardrails passed with missing-lockfile warnings, and release preflight remained blocked on the same 10 issues. Archive integrity and the external SHA-256 manifest were verified.

## Not executed / not certified

This workspace has no npm, Bun, or Cargo lockfiles, no installed `svelte-check` / Vite / Tauri CLI executables, and no `cargo`, `rustfmt`, or `cargo clippy` commands. Therefore this audit does **not** claim that Rust tests/Clippy, Svelte/Tauri production build, Bun/Hono integration on a real Bun runtime, cross-platform installer packaging, real OS keychain/provider integration, or cryptographic verification of an actual GitHub-issued Sigstore attestation has executed successfully. Unit tests of the attestation verifier test the policy and failure paths with controlled command behavior; they are not a real signing ceremony.

The third-party GitHub Actions used in `.github/workflows/ci.yml` are still tag-pinned, not immutable commit-pinned. The new release preflight rejects those refs until every external action is reviewed and pinned to a full 40-character commit SHA. No guessed hashes have been inserted.
