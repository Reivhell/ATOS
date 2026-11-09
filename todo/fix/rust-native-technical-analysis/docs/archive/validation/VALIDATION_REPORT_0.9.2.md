# Agentic Trading OS v0.9.2 — Provider Session Revalidation & Restart Safety

Date: 2026-10-29

## Release qualification summary

This candidate addresses state that incorrectly survived process boundaries as if it were live, as well as stale/future-timestamp acceptance. It is **not production-certified**: Svelte/Vite and native Rust build/tests could not run because the required dependencies and toolchains are unavailable in this environment.

## Root-cause fixes

1. **Persisted provider liveness is invalidated on startup.** Restored quotes no longer retain `isLive=true`; restored provider and screener sessions are marked stale (or no-data when empty), while last-known values remain historical context only.
2. **Kill switch is re-armed on every process restart.** A refresh alone cannot resume paper order entry. The operator must review the new session's data/risk state and explicitly release the switch.
3. **Freshness predicates reject future timestamps.** Rust order/risk, candle provenance and provider freshness use shared predicates requiring a positive timestamp, no future observation, bounded age (120 seconds), connected provider state, and live quote status where relevant. Frontend quote/provider/screener predicates follow the same clock and freshness policy.
4. **Stale screener cache is not displayed as current evidence.** Persisted rows remain available as historical state, but the current-evidence list is hidden until a connected, timestamp-valid, fresh snapshot arrives.
5. **Fixed-origin public HTTP clients reject redirects.** Screener and market-data clients set `reqwest::redirect::Policy::none()` so responses cannot redirect those requests to an unreviewed host.
6. **Explicit timestamp display.** Invalid timestamps show `unknown`; future timestamps show `timestamp in future` rather than being rendered as recently observed.
7. **No runtime demo data added.** Fresh state starts with no market/screener observations and no registered agents. Legacy inert bundled agents/synthetic observations are migrated out of active state; test fixtures remain confined to tests. A source warning remains to reject externally supplied datasets that are explicitly labelled synthetic; it is not a bundled data feed.

## Tests and static checks actually run

The file-by-file scan details are included in `docs/SOURCE_AUDIT_0.9.2.md`.

- `npm test`: **237 passed, 0 failed**.
- `npm run typecheck:layers`: passed.
- `npm run verify:repo`: passed with dependency pinning warnings (details below).
- `npm run eval:agents -- --report-file validation/agent-release-evaluation-0.9.2.json`: **10/10 scenarios passed**, no category gaps, policy passed. This evaluation uses deterministic scenarios/application-boundary tests; `providerLiveIntegrationTested` is `false`.
- JavaScript/MJS parser checks: **0 failures** across the scanned inventory.
- Targeted production-source scan: **90 files / 18,168 lines** across `src`, `api/src`, `src-tauri/src`, and `scripts`; zero dynamic execution/explicit HTML sink patterns (`eval`, `new Function`, `innerHTML`, `outerHTML`, `dangerouslySetInnerHTML`, `document.write`); zero TODO/FIXME/XXX/HACK markers; zero matches for the scanned secret-adjacent logging patterns.
- The broad mock/demo keyword scan found two intentional text-only matches: a UI statement explicitly saying synthetic observations are not supplied, and a warning that rejects input datasets labelled as synthetic. No runtime `browserPreviewSnapshot`, bundled synthetic candles/quotes, sample-market-data path, or mock-provider feature was found in production source.
- ZIP integrity and SHA-256 are checked after packaging; the test suite is re-run from the extracted archive.

## Known blockers — release preflight is not green

`npm run release:preflight` remains **BLOCKED by 9 issues**:

1. `package-lock.json` missing.
2. `bun.lock` missing.
3. `src-tauri/Cargo.lock` missing.
4. `svelte-check` unavailable.
5. Vite executable unavailable.
6. Tauri CLI executable unavailable.
7. Cargo unavailable.
8. rustfmt unavailable.
9. cargo-clippy unavailable.

`npm run build` fails at `svelte-check: not found` (exit 127). Rust unit tests were added for restored provider-session invalidation/kill-switch re-arm and future timestamp rejection, but **were not run** because Cargo/rustc/rustfmt/Clippy are unavailable. Bun/Hono runtime integration, native compile, packaging, provider-live integration, OS keychain validation, and cross-platform acceptance tests also remain unverified. No manually fabricated lockfiles were created.

## Safety boundary

This remains a local paper/research system. `executionAuthority` stays `none`; `liveExecutionAuthorized` stays `false`; `orderIntent` stays `null`. The changes harden data freshness and session revalidation, not live execution capabilities.

## Required work before production deployment

1. Generate and review dependency lockfiles from actual package resolution; do not hand-author them.
2. Run Svelte/Vite build with pinned dependencies and execute Cargo tests, rustfmt and Clippy in a pinned toolchain CI image.
3. Run native desktop integration tests for startup/restart, provider refresh, redirects, quote freshness, audit persistence and OS credential storage.
4. Run live-provider contract tests and cross-platform acceptance tests with secrets kept in the OS credential store.
5. Archive the actual build outputs, dependency bill of materials, scan results and test reports in the release evidence bundle.
