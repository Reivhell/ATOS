# Agentic Trading OS v0.9.1 — Native-Only Boundary Integrity Audit

**Assessment date:** 2026-10-29  
**Release status:** **NOT PRODUCTION-CERTIFIED**  
**Scope:** source audit, root-cause repairs, deterministic Node tests, JS/TS layer typecheck, repository guardrails, AI-agent release evaluation, and release-preflight attempt.

## Executive result

This candidate removes synthetic dashboard fallback behavior and tightens data provenance, market-data freshness, portfolio-risk gates, and financial-statistic semantics. The available automated checks pass, but frontend and native Rust builds cannot be verified in this environment. The missing tools and dependency lockfiles remain release blockers; passing JavaScript tests is not a substitute for those builds.

## Root-cause repairs

1. **Native runtime is required.** The browser-only entry path no longer synthesizes quotes, candles, screener entries, agent sessions, or audit events. If Tauri/Rust is unavailable, the app shows an explicit runtime-unavailable state instead of an illustrative trading dashboard. Fresh/recovered engine state has no market observations or placeholder agents and starts with the kill switch engaged. Startup purges specifically recognized legacy synthetic records and records that migration; this is marker-based cleanup, not a general classifier for malicious persisted data.
2. **Evidence content is bound to its digest.** Each native AI evidence record carries canonical payload JSON and SHA-256. The JS application boundary checks the payload bytes, semantic representation, evidence fields, and envelope before handing evidence to the canonical registry/decision engine.
3. **Market data is revalidated after provider processing.** Rust re-reads the current validated candle context after the model provider responds; the report is rejected if dataset fingerprint, snapshot scope, timestamp, source, candle count, or freshness differs. This closes the previously identified provider-latency time-of-check/time-of-use gap.
4. **Provider failure blocks risk-increasing paper orders.** A young cached quote is not sufficient if global provider state has left `CONNECTED`. Rust checks both quote and provider provenance. The UI uses the same freshness rule for current quote decisions.
5. **All UI quote surfaces share one freshness rule.** Provider status must be connected; quote must be live; price must be finite and positive; timestamp must not be in the future; and age must be at most 120 seconds. Stale quotes are excluded from top-performer rankings, displayed as unavailable in current quote/watchlist metrics, and cannot enable order estimation/submission.
6. **Whole-portfolio mark gate.** A risk-increasing buy is rejected if any existing position lacks a positive live mark no older than 120 seconds. Saved notional cannot silently stand in for an unmarked position in that gate. Sells are not blocked solely because an unrelated position lacks a fresh mark, while the selected sell instrument must still satisfy its own checks.
7. **Unavailable statistics are represented as unavailable.** Dashboard and backtest `winRatePct` are nullable when no closed trades exist, rather than encoding unknown as `0%`. Backtest output now exposes `grossProfit` and `grossLoss`, so the UI can distinguish a mathematically unbounded profit factor (positive gross profit with zero gross loss) from an undefined statistic (no gains and no losses).
8. **Illustrative trend plots and zero-price fallbacks removed.** No hardcoded dashboard equity/drawdown sparklines remain; chart timestamps derive from loaded candle data. Missing prices render unavailable, not `$0`.

## Verification actually run

| Check | Result | Scope/notes |
|---|---|---|
| `npm test` | **PASS — 232/232** | Node test suite, zero failures |
| `npm run typecheck:layers` | **PASS** | Strict JS/TS domain/application layer check; does not compile Svelte |
| `npm run verify:repo` | **PASS with warnings** | Three dependency lockfiles still missing |
| `node --check` sweep | **PASS** | All `.js` and `.mjs` files under `src`, `api`, and `scripts` |
| `npm run eval:agents -- --report-file validation/agent-release-evaluation-0.9.1.json` | **PASS — 10/10 scenarios** | All required categories covered; policy passed; provider live integration explicitly `false` |
| Source-pattern audit | **PASS for scanned patterns** | 90 JS/MJS/TS/Svelte/Rust code files; 18,162 lines scanned for the declared patterns |
| ZIP integrity and round-trip test | **PASS** | Final archive is checked with `unzip -t` and SHA-256, then the Node suite, layer typecheck, repo guardrails, and AI-agent evaluator are rerun from the extracted archive |

### Static audit coverage

- No `browserPreviewSnapshot`, `demo_state`, or `demo_candles` path found in the current runtime source.
- No hardcoded dashboard sparkline markers previously removed were found.
- No production-source TODO/FIXME, `todo!`, or `unimplemented!` marker was found in the scanned source scope.
- No `eval(`, `new Function`, `innerHTML =`, Svelte `{@html}`, `dangerouslySetInnerHTML`, or `Command::new` pattern was found in the scanned application/native source.
- The secret-logging search found no matching logs of API-key/secret/token/authorization/password patterns.
- Legacy marker strings such as `DEMO FIXTURE` and `fixture-v0` remain only where startup migration recognizes and removes old persisted synthetic state and inside native regression-test setup. They are not a default runtime data provider or a user-facing demo mode. Automated source-pattern scans are not a substitute for manual security review or a full SAST/dependency audit.

## Release blockers (still open)

`npm run release:preflight` remains **BLOCKED with nine issues**:

1. `package-lock.json` missing.
2. `bun.lock` missing.
3. `src-tauri/Cargo.lock` missing.
4. `svelte-check` executable unavailable.
5. `vite` executable unavailable.
6. `tauri` executable unavailable.
7. `cargo --version` unavailable.
8. `rustfmt --version` unavailable.
9. `cargo clippy --version` unavailable.

`npm run build` fails with `svelte-check: not found`. The Rust unit tests for payload/snapshot revalidation, portfolio marks, provider-failure provenance, and nullable metric behavior were added/updated as source, but **were not run** because the Rust toolchain is absent. JS source-contract tests that inspect those invariants are not native execution tests.

## Not yet verified

- Svelte compiler and Vite production build.
- Rust compile, `cargo test`, `cargo fmt --check`, and Clippy.
- Tauri desktop package, signing, SBOM/dependency audit, and cross-platform acceptance.
- Real provider HTTP behavior, real OS keychain persistence, imported-model acceptance, and restart recovery under actual desktop runtime.
- Real-provider AI analysis under injected network/provider faults. The evaluator currently runs its application-boundary scenarios deterministically and uses a simulated provider-failure scenario; it does not perform live-provider integration.

## Safety invariants

The app remains research/paper-only. No live venue execution capability was added. `executionAuthority = none`, `liveExecutionAuthorized = false`, and `orderIntent = null` remain required invariants. The fresh-account kill switch is engaged. Do not call this candidate production-ready until the above build/runtime/acceptance gates pass on a pinned CI runner.
