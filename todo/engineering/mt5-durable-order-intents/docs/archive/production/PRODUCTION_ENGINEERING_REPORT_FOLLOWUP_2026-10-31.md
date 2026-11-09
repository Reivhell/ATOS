# ATOS — Autonomous Production Engineering Continuation (follow-up)

**Audit date:** 2026-10-31  
**Application version:** 0.11.0 (unchanged)  
**MT5 bridge protocol:** v6  
**Disposition:** Source engineering continuation completed for the scoped changes. Release remains **BLOCKED**; ATOS is **NOT PRODUCTION-CERTIFIED**.

## Executive summary

This continuation made two substantive changes rather than incrementing the version:

1. Rebuilt release installer staging around a held source file descriptor. The same open file object is validated, hashed and copied; source/directory identity and metadata are rechecked; copied destination bytes are re-read, hashed, structurally revalidated and flushed. Mutation, truncation, atomic replacement and package-directory substitution fail closed.
2. Expanded strict frontend JavaScript type coverage to all 31 `src/**/*.js` modules, corrected strict decision-gate/orchestrator contracts, removed the duplicate `Candle` type export ambiguity, and included the JavaScript source tree in project `checkJs` configuration.

The application version remains 0.11.0 because this is engineering hardening of the current release line, not a version-number exercise. No mock/demo feature, wallet signing, order execution, or order mutation surface was introduced.

## Engineering changes

### Descriptor-bound release staging

The earlier staging flow validated and hashed a path, then reopened the path for copying. A concurrent replacement could therefore cause bytes other than the file previously inspected to be copied. The replacement flow now retains a single source descriptor from opening through staging and:

- rejects symlinks, non-regular files, empty/oversized payloads and unsupported installer extensions;
- uses `O_NOFOLLOW` and non-blocking flags where the platform exposes them;
- compares pre-open path metadata with descriptor metadata and repeatedly checks path/directory identity;
- validates format markers and hashes the full source through the retained descriptor;
- copies from that descriptor while hashing the copied stream and checking content stability;
- reads the staged destination back through its descriptor, verifies size and digest, validates format markers again, applies sanitized permissions and calls `sync()`;
- removes partial release output if any step fails.

Focused regressions pass **11/11**, including atomic path replacement, same-inode mutation, truncation and package-directory replacement after inventory discovery.

This is a meaningful reduction of TOCTOU exposure, not a claim of perfect race freedom on every operating system. Node's portable filesystem API does not expose directory-descriptor-relative `openat/openat2` operations consistently. Format checks remain header/structure plausibility checks, not complete parsers for every package format, code-signature verification, Apple notarization verification or vendor trust-chain validation. The tests were run on Linux only; Windows and macOS acceptance remains necessary.

### Strict frontend source typing

- Added explicit JSDoc input/output contracts to the strict decision gate and strict-analysis orchestrator.
- Clarified the `Candle` aliases so the public barrel no longer produces duplicate export ambiguity.
- Added missing existing native market-data capability names to the explicit runtime capability union.
- Included `src/**/*.js` in project `checkJs` coverage and added the strict gate/orchestrator to `typecheck:layers`.
- Added `@types/node` to development dependencies, but it is not installed in this workspace because dependency resolution could not complete.

Strict TypeScript checking of all **31 frontend JavaScript modules passes**. The configured layer typecheck also passes. The whole-project TypeScript/Svelte check does not pass in this environment because installed declarations/packages for Tauri, Svelte, Vite, the Svelte Vite plugin and Node are absent. No ambient stub declarations were added to create a false pass.

## Verification evidence

| Gate | Result | Scope / limitation |
|---|---|---|
| `npm test` | **PASS** | 375/375 tests, 51 configured command groups, 0 failures. |
| Release staging focused suite | **PASS** | 11/11 tests, including new race/mutation regressions. |
| `npm run typecheck:layers` | **PASS** | Strict configured JSDoc/TypeScript layer list. |
| All frontend JavaScript modules | **PASS** | Strict `tsc` checking of all 31 `src/**/*.js` files. |
| `npm run verify:repo` | **PASS with warnings** | Static repository guard passed; absent npm/Bun/Cargo lockfiles remain warnings and release blockers. |
| Static syntax/structure scan | **PASS** | 92 JavaScript-family files, 13 TypeScript files and 18 JSON files parsed successfully. |
| High-confidence secret-pattern scan | **PASS** | No embedded private-key, common cloud-key, GitHub-token or common service-token patterns found. This is not a full secret-scanning certification. |
| Agent release evaluation | **PASS** | 10/10 offline qualification scenarios; live provider integration not tested. |
| Full project typecheck | **BLOCKED** | Required installed dependencies/type declarations are absent. |
| `npm run build` | **BLOCKED** | `svelte-check` is not installed; Vite build did not start. |
| Release preflight | **BLOCKED** | 10 missing/unavailable prerequisites remain. |
| Rust build/tests/format/Clippy | **NOT RUN** | Cargo, rustc, rustfmt and Clippy are unavailable. |
| MT5 EA compile and broker acceptance | **NOT RUN** | MetaEditor and a broker-connected MT5 acceptance environment are unavailable. |

The full test log, focused staging log, typecheck outputs, scan evidence and preflight results are included in the evidence bundle.

## Release blockers

The release preflight reports ten blockers:

1. `package-lock.json` is missing.
2. `bun.lock` is missing.
3. `src-tauri/Cargo.lock` is missing.
4. `svelte-check` is unavailable.
5. Vite is unavailable.
6. Tauri CLI is unavailable.
7. Bun is unavailable.
8. Cargo is unavailable.
9. `rustfmt` is unavailable.
10. `cargo clippy` is unavailable.

A real npm lockfile-generation command was attempted from `package.json` using `npm install --package-lock-only --ignore-scripts --no-audit --no-fund`, but it did not complete within the 120-second execution limit. It created no lockfile. No lockfile was written by hand. Bun and Rust lockfiles cannot be generated or validated without their package managers/toolchains.

## Remaining acceptance work

- Resolve and commit authentic npm, Bun and Cargo lockfiles using controlled package-manager versions and a network-enabled build environment; enforce immutable/frozen installs.
- Install exact locked dependencies and execute whole-project Svelte/TypeScript checking, Vite production build, Tauri build, Rust tests, formatting and Clippy.
- Run staging regressions on Linux, Windows and macOS. Replace header plausibility checks with full package-format validation and independent vendor code-signature/notarization verification before publishing installers.
- Compile `MQL5/Experts/ATOS_Bridge.mq5` in MetaEditor and retain compiler output and binary hash.
- Exercise MT5 protocol v6 against a real terminal and broker acceptance account with account switching, disconnect/reconnect, duplicate/reordered frames, sequence gaps, position/order inventory changes, symbol-spec revisions and failed/slow terminal-native calculations.
- Produce signed/CI-attested build provenance, SBOM, dependency/license evidence, artifact hashes and broker acceptance results.

## Risk and product boundaries

MT5 account-wide risk sizing and the bridge remain read-only. A sizing result is not an order reservation, execution authorization or guarantee of realized loss. Broker execution, slippage, gaps, commissions, swaps, liquidation behavior and broker-specific margin tiering are not established by source tests. Meme Coin Hunter remains a research/screening capability; no wallet signing or trade execution was added. Strict specialist dispatch and live provider integration remain unverified.

## Certification decision

**NOT PRODUCTION-CERTIFIED.** The source-level changes and test results are evidenced, but dependency reproducibility, the whole frontend/native build, Rust compilation, MQL5 compilation and broker acceptance remain unverified. Release and deployment must remain blocked until the above gates are completed with evidence.
