# Validation report — Agentic Trading OS v0.9.6

**Date:** 2026-10-30  
**Release status:** **BLOCKED — NOT PRODUCTION-CERTIFIED**

## What changed

v0.9.6 advances release engineering from a local SHA-256 inventory to a platform-aware, fail-closed release pipeline. For each OS, CI stages only real installable formats into a new flat directory, rejects missing required package formats and unsafe/empty/symlink/colliding artifacts, creates a content inventory, generates an SPDX SBOM, creates signed GitHub/Sigstore SLSA provenance and SBOM attestations, verifies both predicate types for every staged installer, and uploads only staged installer/evidence outputs. The consumer verifier constrains the exact repository, version tag, signer workflow, GitHub OIDC issuer and hosted-runner policy.

A root-cause regression was fixed in the release verifier: checking the default SLSA predicate alone did not prove the SBOM attestation was present. Verification now runs independently for both SLSA provenance and SPDX 2.3, and tests reject the case where provenance succeeds but SBOM verification fails.

A second release-policy gap is now fail-closed: external GitHub Actions must use reviewed full 40-character commit SHAs. Version tags, branches and abbreviated hashes are not accepted. The workspace could not independently resolve official full commit IDs for every action, so the source still contains eight moving refs and the preflight blocks release rather than guessing at immutable identifiers.

## Local verification results

| Check | Result | Details |
|---|---|---|
| `npm test` | PASS | 259 tests passed, 0 failed across 40 npm test command groups |
| `npm run typecheck:layers` | PASS | Strict layer typecheck |
| `npm run verify:repo` | PASS with warnings | Static repository guardrails passed; missing lockfiles are reported |
| AI Agent qualification | PASS | 10/10, 0 failed, no category gaps, `policyPassed=true`; provider live integration not tested |
| JavaScript syntax | PASS | All 85 `.js`, `.mjs`, `.cjs` files checked with `node --check` |
| Workflow YAML | PASS | Five expected jobs parsed and identified |
| Source scan | PASS, limited scope | 105 code files / 20,650 lines; no matches for the documented scan patterns |
| Immutable action pin policy | PASS as a guard | Tests recognize full 40-character SHAs and reject mutable refs; current workflow is reported blocked |
| Release preflight | BLOCKED | 10 blockers listed below, by design |

## Final package verification

The exact candidate ZIP was extracted into a clean directory and its `npm test`, `npm run typecheck:layers`, and `npm run verify:repo` commands were rerun successfully: 259/259 tests passed, typecheck passed, and repository guardrails passed with the same missing-lockfile warnings. The extracted-package preflight returned the expected fail-closed result with 10 blockers. ZIP integrity and its SHA-256 manifest were also verified.

## Release blockers (10)

1. `package-lock.json` is missing.
2. `bun.lock` is missing.
3. `src-tauri/Cargo.lock` is missing.
4. `svelte-check` executable is unavailable.
5. Vite executable is unavailable.
6. Tauri CLI executable is unavailable.
7. `cargo --version` cannot run.
8. `rustfmt --version` cannot run.
9. `cargo clippy --version` cannot run.
10. Eight third-party GitHub Actions in `.github/workflows/ci.yml` still use moving refs: `Swatinem/rust-cache@v2`, `actions/attest@v4`, `actions/checkout@v4`, `actions/setup-node@v4`, `actions/upload-artifact@v4`, `anchore/sbom-action@v0`, `dtolnay/rust-toolchain@stable`, and `oven-sh/setup-bun@v2`. All must be reviewed and pinned to full 40-character commit SHAs.

An attempt to obtain external registry/GitHub metadata from this workspace is blocked by DNS/network resolution. Lockfiles and action hashes were not fabricated.

## Not verified

- Native Rust tests, formatting and Clippy have not run locally.
- Svelte/Vite/Tauri production build and packaged desktop application have not run locally.
- Bun/Hono integration, actual provider and OS keychain integration, and cross-platform end-to-end acceptance have not been verified in this environment.
- Signed GitHub Artifact Attestations were not produced or verified against real installers; the workflow and verifier implementation have source-level tests only.
- Platform code-signing/notarization (such as Windows Authenticode and Apple Developer ID notarization) is outside this change and remains required where applicable.

## Release decision

Do not label or deploy this package as production-certified yet. The source-level tests and typecheck are green, but release preflight deliberately fails on missing reproducibility inputs/toolchains and mutable action references. A production signoff requires reviewed lockfiles, immutable action pins, a green tagged CI run on Linux/Windows/macOS, real Rust/frontend builds, signed attestation verification, and platform acceptance tests.
