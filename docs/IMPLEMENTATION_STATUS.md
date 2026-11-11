## Current engineering baseline — `0.0.1-beta` (2026-11-09)

This snapshot consolidates implemented source capabilities and corrects prior non-evidence-backed release claims. It is **not production-certified** and grants **no live order/execution authority**.

ATOS includes a Rust-native, statically linked provider registry and a versioned market-data contract (provider discovery, canonical models, explicit provider selection, typed provenance). Current real adapters in source: authenticated MT5 bridge + durable account-bound closed-bar history, the MT5 account-wide risk control plane (account-wide stop-risk inventory and broker-native sizing preflight), and validated public market/cache paths where configured. Broker-context requests must be pinned to MT5 and require fresh, authenticated identity/sequence revalidation. Freshness is anchored to market-observation time (not cache-ingestion time). Research/backtest stale override must remain explicit in request/response semantics. Read-only MT5 EA (`MQL5/Experts/ATOS_Bridge.mq5`) must not submit/amend/close/cancel broker orders.

### Verified locally (2026-11-09)
- `bun run verify:repo` — pass (static guardrails)
- `cargo check --manifest-path src-tauri/Cargo.toml` — pass
- Spot Bun tests passed (strict orchestrator, release evidence, selected contract suites). `scripts/test-agent-runtime-orchestrator.test.mjs` uses nested `test()` (Bun limitation) but passes under Node; contract checks remain valid.

### Not verified / blockers
Full Bun/Node suite in a clean CI-equivalent run, Svelte/Vite production build, Rust tests/Clippy/fmt release gates, Tauri packaging/signing/attestation publication, MetaEditor EA compilation, real MT5/broker acceptance, live provider/keychain integration, and cross-platform runtime acceptance.

### Historical notes
Older per-version implementation/audit details were moved to `docs/archive/` (including `docs/archive/historical-0.11-era/`). Claims in archived docs are historical and must not be treated as current certification.
