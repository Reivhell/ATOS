# Validation report — Agentic Trading OS v0.11.0

**Candidate:** v0.11.0 — Rust-native unified market-data platform
**Date:** 2026-10-31
**Status:** Source milestone; not production-certified

## Implemented scope

- Rust-native, statically linked provider registry with validated identifiers, duplicate-registration rejection, deterministic lookup, and no automatic cross-provider fallback.
- Versioned canonical market-data request, catalog, normalized bar, quality, provenance, and result contracts.
- Real MT5 adapter through the existing authenticated bridge and durable SQLite closed-bar query service. Broker-context requests are pinned to MT5 and require a connected, non-stale terminal; the bridge sequence and authenticated broker/account identity are revalidated after the query.
- Existing Binance public cache adapter for BTCUSDT M5 research/backtest only. It cannot satisfy broker context or substitute another symbol/timeframe.
- Observation-time freshness rather than cache-ingestion freshness. Unknown observations fail closed even when stale data is allowed. Research/backtest stale override is an explicit operator choice and is echoed with the freshness limit in the result request summary.
- Stable SHA-256 fingerprint of the complete requested public dataset, independent of pagination page. MT5 uses the existing account-bound SQLite dataset manifest fingerprint.
- Explicit distinction between exchange base-asset volume, broker tick count, and broker real volume.
- Native Tauri commands and a Svelte Market Data panel with provider discovery, query selection, stale policy, freshness, gap/revision and provenance displays.
- Release preflight now checks `bun --version` as a required runtime toolchain command.

## Executed verification

| Check | Result |
| --- | ---: |
| Full `npm test` | 335/335 subtests passed; 50 command groups; exit 0 |
| `npm run verify:repo` | PASS |
| `npm run typecheck:layers` | PASS; selected strict layer subset only |
| JavaScript syntax scan | 89 files, 0 errors |
| TypeScript syntax scan | 12 files, 0 errors |
| New OpenBB/native-market-data source-contract checks | 8/8 passed |
| Release preflight | BLOCKED; 10 issues |

The `typecheck:layers` command does not constitute a complete Svelte/frontend typecheck. An additional direct Tauri bridge typecheck could not resolve `@tauri-apps/api/core` and the imported JavaScript guard because package dependencies are not installed in the workspace.

## Production blockers

The release preflight is fail-closed and reports:

1. Missing `package-lock.json`.
2. Missing `bun.lock`.
3. Missing `src-tauri/Cargo.lock`.
4. Missing installed `svelte-check`.
5. Missing installed `vite`.
6. Missing installed Tauri CLI executable.
7. `bun --version` unavailable.
8. `cargo --version` unavailable.
9. `rustfmt --version` unavailable.
10. `cargo clippy --version` unavailable.

`npm run build` was attempted and stopped because `svelte-check` is not installed. Rust compilation, Rust unit tests, Clippy, rustfmt, Bun/Hono runtime integration, MetaEditor compilation, terminal testing, and real-broker acceptance were not executed. The embedded Rust unit tests are present but unverified at runtime.

## Architecture and licensing boundary

The implementation adopts OpenBB's provider/core and standard-model/result-wrapper concepts but does not copy or bundle OpenBB source code or run its Python runtime. The provider registry is statically linked into ATOS for a smaller, signed, auditable native execution surface. A separate Hono/MCP API surface is not added because no supervised transport currently exposes the Rust-owned authenticated provider service to the Bun process; duplicating MT5 identity and provider logic in TypeScript would weaken the trust boundary.

OpenBB V5 software licensing does not grant rights to third-party provider data. Any future additional provider must undergo source reliability, data-license, retention, redistribution, credential, freshness and schema review before registration.

## Release decision

Version manifests are aligned at `0.11.0` to record the substantial new capability. This is not a release approval. Do not deploy or describe the application as production-ready until all ten preflight blockers are resolved, full Svelte/Vite/Tauri and Rust builds/tests pass, and MetaEditor plus real terminal/broker acceptance evidence is attached to the release record.
