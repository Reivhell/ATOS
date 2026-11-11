# OpenBB capability adoption in ATOS

**Implementation baseline:** ATOS 0.11.0 source candidate  
**Subsystem:** Rust-native market-data provider registry  
**Status:** Source implementation present; Rust and Svelte production builds have not yet been executed in this workspace.

## Why these OpenBB capabilities were selected

OpenBB's Open Data Platform separates the core runtime from provider and router extensions. Providers implement shared data contracts, the runtime selects a compatible fetcher, and results carry a common typed wrapper. The platform is designed to expose the same connected data through multiple consumption surfaces. These patterns are relevant to ATOS because its existing market inputs arrive from two sources with materially different authority: authenticated broker-terminal data and public exchange observations.

The ATOS implementation adopts the architectural capabilities, not the OpenBB Python runtime:

| OpenBB capability | ATOS implementation |
| --- | --- |
| Provider extension registry | Statically linked Rust provider registry with duplicate-ID rejection and deterministic lookup |
| Shared standard models | Versioned `MarketDataQuery`, normalized bars, volume-basis semantics and catalog contracts |
| Typed result wrapper (`OBBject`) | `UnifiedMarketDataResult` with request summary, provenance, freshness, integrity/gap information and SHA-256 dataset fingerprint |
| Provider capability discovery | Native provider catalog exposed to the Svelte Market Data page |
| Provider selection | Explicit provider IDs and deterministic routing; unknown providers fail without automatic fallback |
| Multiple data authorities | MT5 durable broker history and the existing Binance public cache, with distinct authority and allowed-purpose policies |

No OpenBB implementation source files were copied into ATOS. The Rust contracts and adapters are native implementations built against ATOS's existing domain, bridge, and cache interfaces.

## Provider authority and routing policy

### `mt5-terminal`

- Uses the existing authenticated MT5 bridge and durable SQLite closed-bar store.
- Resolves the account identity inside Rust; the UI does not supply broker login credentials to the new query command.
- The existing bridge independently revalidates that identity when the historical query is executed.
- `brokerContext` requests require a connected, non-stale terminal and revalidate both bridge sequence and authenticated broker/account identity after reading. A changed sequence or identity discards the result.
- The public result omits the account login entirely; it does not use an unkeyed hash as a privacy mechanism.
- Tick count and real volume remain distinguishable. Broker-reported volume is not mislabeled as exchange base-asset volume.

### `binance-public`

- Reads the existing validated Binance spot cache; it does not issue a new network request from the query path.
- The current normalized candle contract is restricted to BTCUSDT M5.
- Public exchange data is allowed for research and historical backtests only.
- It cannot satisfy broker-context requests and does not substitute another symbol or timeframe.

## Safety invariants

1. Provider IDs are explicit and validated. A missing provider defaults only to the public cache for ordinary research/backtest queries; broker-context requests are forcibly pinned to MT5.
2. An unknown provider fails closed. There is no hidden provider retry or cross-venue fallback.
3. Stale data is rejected by default. The operator must explicitly enable the research/backtest stale override; the decision and freshness limit are recorded in the result request summary. Broker context cannot opt into stale data.
4. Timestamps too far in the future are rejected as clock-skew violations.
5. Freshness age is calculated from the latest market-observation timestamp, never the cache-ingestion timestamp; ingestion time remains separate provenance so newly imported historical bars cannot masquerade as live market state.
6. Range, cursor, timeframe, page-size, and freshness policies are revalidated in Rust regardless of frontend validation.
7. The query API is read-only. It does not create order intents, alter risk limits, place orders, or grant execution permission.
8. The new source scope hash is an identifier, not a credential. Raw account login is not included in the unified response.

## Current supported surface

The new Tauri-native commands are `get_market_data_provider_catalog` and `query_unified_market_data`. The Svelte Market Data page displays the native catalog, lets an operator select a provider and use case, requires an explicit stale-data override, and displays request policy, dataset provenance, observation and receipt timestamps, freshness, gaps, volume semantics, and a bounded recent-bar view.

This first implementation is deliberately static-linked. It does not load third-party dynamic libraries or execute downloaded provider code. New providers must implement the Rust provider contract and be included in the signed application build. A separate authenticated Hono/Bun endpoint and MCP surface should call the same Rust-owned query service when the application has a supervised native service transport; duplicating provider logic in TypeScript would reintroduce source-of-truth divergence and is not part of this change.

## OpenBB references and licensing note

- OpenBB Platform architecture and its core/provider/extension separation: <https://github.com/openbq-org/OpenBB/blob/develop/openbb_platform/README.md>
- OpenBB core provider registry and standard-model architecture: <https://github.com/openbq-org/OpenBB/blob/develop/openbb_platform/core/README.md>
- OpenBB V5 license FAQ: <https://github.com/OpenBB-finance/openbb-docs/blob/main/content-odp/python/faqs/license.mdx>

OpenBB's V5 documentation identifies the current platform packages as Apache-2.0 and states that provider data remains subject to each provider's own terms. ATOS does not redistribute OpenBB code or third-party market data as part of this implementation. Provider data retention, display, and redistribution must still follow the applicable provider agreement.

## Verification boundary

Node source-contract tests can verify that commands, routing rules, and frontend bindings remain connected. The repository guard and release preflight also verify that Bun is present as a required runtime toolchain. These checks cannot prove Rust type correctness, SQLite behavior under concurrent account changes, Svelte compilation, Tauri packaging, terminal interoperability, or broker correctness. Those require `cargo test`, `cargo clippy`, `cargo fmt --check`, `npm run build`, MT5 MetaEditor compilation, and terminal acceptance against a real broker account.
