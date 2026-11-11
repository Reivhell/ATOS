# Changelog

## 0.0.1-beta — 2026-11-09

### Status
- **Beta engineering snapshot, not production-certified.** Local-first research/paper-trading desktop application (Tauri + Rust + Svelte). No live order submission/cancellation/amendment is enabled in the application or MT5 EA.

### Implemented / present in repository (verified by source audit)
- **Desktop app stack:** Tauri 2 + Rust backend, Svelte 5/Vite frontend, Bun/Node scripts and local API scaffolding (`api/`), TypeScript config, CI workflow with pinned action SHAs (`.github/workflows/ci.yml`).
- **Rust modules (compiled locally with `cargo check`):** domain/application/adapters/infrastructure/runtime layers including market-data platform/query, strict analysis/service, research workspace, MT5 risk sizing, AI agent runtime, MT5 bridge/market-data, and provider registry.
- **Frontend/UI components:** `AgentRuntimePanel`, `MarketDataPlatformPanel`, `MemeCoinHunterPanel`, `Mt5BridgePanel`; app version label updated to `v0.0.1 Beta`.
- **MT5 bridge EA:** `MQL5/Experts/ATOS_Bridge.mq5` (1517 lines). Source scan found no order-submission/trade-mutation API calls (`OrderSend`/`trade.Buy`/`trade.Sell` patterns absent); intended as read-only bridge per project docs/code intent.
- **Technical analysis / strict gate assets:** 364-entry master-list fixture reference in strict analysis, 9 specialist agent docs + schemas under `docs/agents/`, strict/research/market-data/decision/evidence contract tests in `scripts/*.test.mjs`.
- **Release/repo tooling:** `verify-repo`, release evidence/artifact/attestation/version-policy scripts, lockfiles present (`package-lock.json`, `bun.lock`, `src-tauri/Cargo.lock`), `package.json`/`tauri.conf.json`/`Cargo.toml` versions aligned to `0.0.1-beta`.
- **Verification run in this environment (2026-11-09):** `bun run verify:repo` passed (static guardrails); `cargo check --manifest-path src-tauri/Cargo.toml` passed (`dev` profile finished); sample Bun tests passed (strict orchestrator 6/6, release evidence 7/7, selected contract suites 33/33).

### Not verified / not claimed
- Full Bun/Node test suite, Svelte/Vite production build, Rust unit tests/Clippy/fmt gates in CI-equivalent clean environment, Tauri packaging, signed release attestations/SBOM publication, MetaEditor compilation of EA, real MT5 broker acceptance, live provider integration, and cross-platform runtime acceptance are **not certified by this changelog**.
- Historical changelog entries claiming `0.11.0`–`0.1.x` as sequential released milestones were removed because they were not evidence-backed as shipped releases in this repository state; implemented capabilities are consolidated above. Reintroduce versioned release notes only when backed by tagged CI artifacts, passing gates, and reviewed evidence.
