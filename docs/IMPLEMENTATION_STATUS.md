## Current engineering worktree — unified market-data platform (2026-10-31)

ATOS v0.11.0 adds a Rust-native, statically linked provider registry and a versioned market-data contract adapted from the useful OpenBB platform patterns: provider discovery, canonical models, explicit provider selection, and typed result provenance. The current real data adapters are the authenticated MT5 bridge plus durable account-bound closed-bar history and the existing validated Binance BTCUSDT M5 cache. Public exchange data cannot satisfy broker-context requests; broker context is pinned to MT5 and revalidates both bridge sequence and authenticated server/account identity after reading. Freshness is anchored to market-observation time rather than cache-ingestion time; missing timestamps cannot be overridden as merely stale. Research/backtest stale override is an explicit operator choice and is included in the returned request summary. Dataset fingerprints, volume basis, revisions, gaps, and latest receipt/observation timestamps remain attached to the result.

The feature is read-only and grants no order or execution authority. The Svelte Market Data page uses real Tauri-native commands. The API remains Bun/Hono; provider calculations remain Rust-owned rather than being duplicated in TypeScript. A separate Hono route/MCP surface is deferred until a supervised transport can safely share the Rust-owned provider service and authenticated MT5 identity. See [`OPENBB_NATIVE_DATA_PLATFORM_ADOPTION.md`](OPENBB_NATIVE_DATA_PLATFORM_ADOPTION.md).

Updated 2026-11-01: Svelte/Vite build now succeeds (`npm run build`, 3434 modules), `cargo check --all-targets` and `cargo test --all-targets` pass (239 tests), `npm test` passes (15/15), `npx svelte-check` reports 0 errors, and `npm run verify:repo` passes. MetaEditor compilation and real MT5/broker acceptance remain inherently unavailable.

## Current engineering worktree — MT5 account-wide risk control plane (2026-10-31)

The latest work advances the MT5 bridge to protocol v6 and adds a snapshot-integrity fence to the account-wide risk control plane. The authenticated account snapshot includes broker margin-call/stop-out policy and initial/maintenance margin; every open position and pending order carries terminal-calculated stop-loss risk state/value. Open-position risk is incremental from current mark (avoiding double-counting floating P/L already included in equity), while pending-order risk uses its executable activation/entry reference. Rust aggregates cross-symbol account exposure, blocks sizing when any item is unprotected or unresolved, and caps a candidate by both the requested trade-risk budget and remaining capacity beneath ATOS's 5% account-wide stop-risk ceiling. Stops must be strictly loss-side and have positive native loss; zero/crossed/profitable-side stops cannot qualify as protected.

The EA now captures one terminal tick for both the telemetry frame and native `OrderCalcMargin`/`OrderCalcProfit` candidate calculations. Rust binds each native result to the exact authenticated frame sequence and rejects a result after the accepted snapshot advances. The EA rechecks the complete position/order ticket set and risk-bearing fields after inventory enumeration, and again before transmitting a preflight result; any change discards the frame. UI sizing outputs are retained as history but cease to display an active volume as soon as the snapshot changes, disconnects, or becomes stale. The API's runtime helpers for validation, loopback-origin policy, rate limits, and observability have also been migrated to strict TypeScript; the API remains Bun/Hono. The EA remains strictly read-only. These are source-level hardening changes, not a production certification; native builds and broker acceptance gates remain outstanding. See [`docs/MT5_BROKER_NATIVE_RISK_SIZING_ENGINE_2026-10-31.md`](MT5_BROKER_NATIVE_RISK_SIZING_ENGINE_2026-10-31.md) and [`docs/ENGINEERING_UPDATE_2026-10-31_ACCOUNT_RISK_INVENTORY_FENCE.md`](ENGINEERING_UPDATE_2026-10-31_ACCOUNT_RISK_INVENTORY_FENCE.md).

## Structural audit (2026-11-01)
- Root `src/lib/*.js|ts` shims (bridge.ts, dashboard-model.js, decision-gate.js, exports.js, index.js, market-data-model.js, research-detectors.js, strict-analysis-orchestrator.js, strict-catalog-registry.js, technical-analysis.js, types.ts) remain because they are imported by `src/App.svelte`, layered modules, and `scripts/test-architecture-layers.test.mjs`; removing them breaks the build and 15 JS tests. Consolidation deferred.
- Doc claims verified: 364 concepts (`REQUIRED_CONCEPT_COUNT=364`), protocol v6 (`PROTOCOL_VERSION=6`), freshness window 120s (`MARKET_DATA_OBSERVATION_MAX_AGE_MS=120000`). All match code.
- Evidence: `cargo test --all-targets` 239 passed; `npm test` 15 passed; `npx svelte-check` 0 errors/0 warnings; `npm run build` succeeds; `npm run verify:repo` passes.

## Shim inventory (evidence-backed, 2026-11-01)
| Shim | Load-bearing? | Evidence (imported by) |
|---|---|---|
| bridge.ts | yes | App.svelte, MarketDataPlatformPanel, Mt5BridgePanel, MemeCoinHunterPanel, native-runtime-guard, tests |
| dashboard-model.js | yes | App.svelte, index.js, tests |
| decision-gate.js | yes | index.js, contracts.js, strict-analysis-orchestrator.js, tests |
| exports.js | yes | App.svelte, index.js, tests |
| index.js | yes | 20+ layered modules + tests |
| market-data-model.js | yes | App.svelte, technical-analysis-service.js, tests |
| research-detectors.js | yes | analysis-engine.js, engines/technical-analysis/index.js, tests |
| strict-analysis-orchestrator.js | yes | index.js, tests |
| strict-catalog-registry.js | yes | strict-analysis-orchestrator.js, strict-decision-gate.js, tests |
| technical-analysis.js | yes | App.svelte, research-detectors.js, index.js, catalog.js, contracts.js, technical-analysis-service.js, analysis-engine.js, tests, api/src/app.ts |
| types.ts | yes | canonical-evidence-registry.js, reliability-contract.js, candle-series-audit.js, market-data-model.js, ai-agent-orchestrator.js, indicators.js, research-detectors.js, research-decision-engine.js, Mt5BridgePanel.svelte, tests |

## .md claim audit (2026-11-01)
| Claim | Source doc | Code evidence | Status |
|---|---|---|---|
| 364 concepts | IMPLEMENTATION_STATUS / strict_analysis | `REQUIRED_CONCEPT_COUNT = 364` (strict_analysis.rs:20) | verified |
| Protocol v6 | README / bridge | `PROTOCOL_VERSION: u16 = 6` (bridge.rs:30) | verified |
| Freshness window 120s | docs | `MARKET_DATA_OBSERVATION_MAX_AGE_MS = 120_000` (models.rs:58) | verified |
| Svelte/Vite build | README (old: unverified) | `npm run build` succeeds (3434 modules) | updated |
| Rust tests | README (old: unverified) | `cargo test --all-targets` 239 passed | updated |
| JS tests | README (old: unverified) | `npm test` 15 passed | updated |

## MT5 durable order-intent engineering slice (2026-11-01)

Branch `engineering/mt5-durable-order-intents` adds a standalone SQLite persistence primitive for a future execution controller. It persists account-scoped intents before dispatch, enforces idempotency and full-intent payload fingerprints, records transactional state-transition events, and exposes unresolved intents for restart recovery. Ambiguous post-send timeouts are represented as `unknown` and cannot transition directly back to `dispatching`; reconciliation must resolve them first.

This is not live execution. The module is registered under the MT5 infrastructure namespace but is not connected to the read-only bridge, UI, or any broker submission path. Tests are authored but must run in CI before merging. Broker reconciliation, `OrderCheck`, outbox dispatch, and demo-account acceptance remain outstanding. See [MT5 durable order-intent ledger](MT5_DURABLE_ORDER_INTENT_LEDGER_2026-11-01.md).
