# Validation report — Agentic Trading OS v0.10.1

**Date:** 2026-10-30  
**Candidate:** v0.10.1 — nested walk-forward parameter tournament  
**Release status:** **BLOCKED — NOT PRODUCTION-CERTIFIED**

## Change summary

v0.10.1 extends the Rust-native Research Lab with a bounded nested walk-forward parameter tournament. It compares up to 25 SMA parameter pairs per fold, selects parameters using training data only, and evaluates the selected pair on the next unseen chronological block. Three folds are requested by the Research Lab. Results include every training candidate score, selected parameters, per-fold OOS metrics, parameter stability, dataset provenance, blockers, and caveats.

A new semantic-validation pass verifies that stored candidate grids are unique and stable across folds, candidates are eligible only after the minimum training-trade count, test windows form chronological expanding folds, selected parameters are the deterministic training-only winner, and summary metrics exactly match the stored fold evidence. The canonical workflow DAG is also revalidated for exact dependency edges, node order, timestamps, statuses and agreement with specialist/committee evidence. Committee policy v2 refuses promotion when folds are missing, positive OOS ratio is below the recorded threshold, or worst OOS drawdown breaches the configured research ceiling. The baseline backtest and tournament now share one immutable candle/provider snapshot, eliminating a refresh race between separate data reads; the engine lock is not held during CPU-heavy simulations.

The strategy tournament is Rust-native and read-only with respect to engine account state. The Research Lab's separate SQLite workspace remains isolated from orders, balances, positions, kill-switch state and live execution authorization. No sample or synthetic market result is exposed in the UI; candidate records are rendered from the native command response.

## Local validation results

| Check | Result | Details |
|---|---|---|
| `npm test` | PASS | 269 tests passed, 0 failed across 41 npm test command groups |
| `npm run typecheck:layers` | PASS | Strict TypeScript/JavaScript layer typecheck |
| JavaScript syntax | PASS | `node --check` passed for all 85 JavaScript/ES module files in `src`, `api` and `scripts` |
| `npm run verify:repo` | PASS with warnings | Static repository guardrails passed; three missing dependency lockfiles reported |
| Focused Research Lab + backtest contract tests | PASS | 12 tests passed, 0 failed |
| Release manifest alignment | PASS | `package.json`, `Cargo.toml`, and `tauri.conf.json` all report 0.10.1 |
| AI Agent release qualification | PASS | The existing source-level qualification gate passed as part of release preflight |
| `npm run build` | BLOCKED | `svelte-check` executable is not installed; Vite and Tauri CLI are also absent |
| Rust native tests/build/format/Clippy | NOT RUN | Cargo, rustfmt and Clippy are not installed |
| Release preflight | BLOCKED | 9 local lockfile/build-toolchain blockers; the external-action SHA policy now passes |

The Rust tournament unit tests were added to the native source but could not be compiled or executed in this environment. The full JavaScript suite completed with 269/269 passing tests across 41 command groups. Node source-contract tests verify their presence and the frontend/native contract; those checks are not a substitute for executing Rust tests.

## Release blockers (9)

1. `package-lock.json` is missing.
2. `bun.lock` is missing.
3. `src-tauri/Cargo.lock` is missing.
4. `svelte-check` executable is unavailable.
5. Vite executable is unavailable.
6. Tauri CLI executable is unavailable.
7. `cargo --version` cannot run.
8. `rustfmt --version` cannot run.
9. `cargo clippy --version` cannot run.
Regular CI now requires frozen Bun/npm dependency installs instead of regenerating lockfiles. The external GitHub Actions policy passes: all eight third-party actions use upstream-reviewed, full 40-character commit SHAs. The workflow, `rust-toolchain.toml`, and Node guardrails pin Rust 1.99.1; CI no longer tracks the moving `stable` compiler channel. Workflow YAML parsed successfully and contains the five expected jobs. `npm run build` was attempted and stopped at the missing `svelte-check` executable, before Vite could run.

The workspace still cannot resolve the missing dependency metadata or execute the pinned native toolchain locally. Dependency lockfiles were not fabricated.

## Not verified

- Rust code has not been compiled; native unit tests, rustfmt and Clippy have not run.
- The Svelte component has not passed `svelte-check`; Vite/Tauri desktop build and OS packaging have not run.
- No live exchange/provider, account keychain, or broker integration was exercised.
- No Linux/Windows/macOS cross-platform acceptance was run.
- Signed GitHub Artifact Attestations have not been produced or verified against real installers.
- Platform code signing/notarization (Windows Authenticode and Apple Developer ID) has not been performed.
- The nested tournament has no broker fill model, order-book depth, variable spread, market impact, latency, or position carry between fold boundaries; these limitations are recorded in the research architecture.

## Release decision

Do not deploy or label v0.10.1 production-certified. Passing source-level JavaScript tests and TypeScript layer checks does not prove that the new Rust source compiles or that the packaged desktop application works. Production signoff requires reviewed lockfiles, a green tagged Linux/Windows/macOS CI run, native Rust tests and Clippy, Svelte/Vite/Tauri builds, provider/keychain and desktop acceptance tests, signed installer/SBOM attestations, and applicable OS signing/notarization.
