# System Architecture

## Release boundary

The application is a local-first modular monolith built with Tauri 2, Svelte 5, and a Rust backend. It is not a distributed agent platform and it is not certified for live execution. The Rust backend owns persisted paper-account state, deterministic risk checks, backtesting, research persistence, AI-provider boundaries, and MT5 telemetry ingestion. The frontend is a presentation and application-client layer; it does not own account authority or broker credentials.

The current Rust source layout is intentionally layered:

```text
src-tauri/src/
  lib.rs                         # composition-root export only
  main.rs                        # desktop entrypoint
  adapters/tauri/
    bootstrap.rs                 # dependency construction and IPC registration
    commands.rs                  # typed IPC adapters; no direct SQL or HTTP client
  application/
    backtest.rs                  # research/backtest use cases
    market_data_query.rs         # MT5 market-data query facade
    research_workspace.rs        # research workflows, evidence and run-card rules
    strict_analysis.rs           # strict-analysis contracts and persisted gate
    strict_analysis_service.rs   # authoritative snapshot assembly for commands
  domain/
    models.rs                    # shared business contracts and invariants
    research_smc.rs              # deterministic SMC research primitives
    state_decision.rs            # persisted account-state transition policy
  runtime/
    trading_core.rs              # account aggregate, state lifecycle and paper risk runtime
  infrastructure/
    ai/agent_runtime.rs          # provider/keyring/model-artifact adapter and agent runtime
    mt5/bridge.rs                # authenticated local MT5 telemetry transport
    mt5/market_data.rs           # durable MT5 ingestion and dataset query store
    providers/public_market.rs  # bounded public market/screener providers
```

Frontend source follows the same dependency direction:

```text
src/lib/
  domain/          # framework/provider-independent contracts and policies
  engines/         # deterministic calculations and decision evaluators
  application/     # use-case orchestration
  adapters/tauri/  # IPC client and native-runtime guard
  infrastructure/ # serialization and external-format support
  presentation/    # view models
  components/      # Svelte panels
```

The root-level `src/lib/*.js` and `src/lib/bridge.ts` compatibility surfaces remain small re-export shims for existing imports. They must not become second implementations.

## Dependency rules

1. Domain code does not import Tauri, HTTP clients, SQLite, keyring, Svelte, or provider APIs.
2. Engines depend on typed domain contracts and deterministic inputs.
3. Application services coordinate use cases and call runtime/infrastructure services; transport frameworks do not own business decisions.
4. Tauri commands are adapters only. They deserialize typed requests, delegate, and return typed results. They do not open databases or construct HTTP clients.
5. Infrastructure adapters do not depend on Tauri IPC. They expose Rust APIs to application/runtime code.
6. `lib.rs` remains a small composition-root export. `bootstrap.rs` owns the Tauri builder, managed state, plugins, and registered command list.
7. The browser presentation path cannot synthesize account or market data and cannot authorize order execution.

The Node architecture suite checks the Rust source tree, command-registration parity, compile-time embedded asset paths, frontend layer boundaries, and compatibility shims. These source-contract checks are not a substitute for `cargo check`, `cargo test`, Clippy, or a real Tauri build.

## Runtime ownership and data boundaries

- `TradingCore` owns the local paper-account snapshot, risk enforcement, paper-order lifecycle, provider freshness gates, and the account persistence/commit ledger.
- `ResearchWorkspace` owns separate research persistence, hypotheses, workflow evidence, run cards, and research committee validation.
- `Mt5Bridge` owns local authenticated telemetry transport. `Mt5MarketDataStore` owns durable tick/candle receipts, per-series cursors, quality analysis and query manifests. MT5 data is kept separate from paper-account and research databases.
- `AgentRuntime` uses allowlisted providers and OS credential storage. Agent results are bounded research commentary, not trade instructions or execution authority.
- Public provider refreshes are bounded and read-only. They update Rust-owned state through `TradingCore`.
- `MQL5/Experts/ATOS_Bridge.mq5` is a read-only terminal adapter. The current EA does not submit, modify, close, or delete orders.

## Trust and failure behavior

- Invalid, stale, replayed, out-of-sequence, unauthenticated, or out-of-subscription MT5 frames fail closed before durable acknowledgement.
- Market-data persistence acknowledges a frame only after its SQLite transaction commits. A reconnect obtains durable cursors and resumes bounded backfill.
- Dataset page requests are bound to a deterministic manifest. A changed dataset across pagination invalidates continuation rather than mixing snapshots.
- Account writes are gated by native state-transition policy and durable commit evidence. Storage errors block state-changing operations.
- Missing or stale market observations block risk-increasing paper orders. The kill switch is re-engaged at startup.
- AI-provider failure or malformed output produces a bounded error/degraded result; it cannot grant execution authority.
- An unknown live-broker execution result must be reconciled against broker state before any retry. Live order execution is not currently implemented.

## Known architectural debt (not hidden)

The folder boundaries are now explicit, but this is not yet a fully port-driven hexagonal architecture:

- `runtime/trading_core.rs` is still a large aggregate with account policy, SQLite schema/lifecycle, persistence, and state transition methods in one file. Splitting it requires preserving transaction boundaries, instance locking, crash recovery and the hash-chained commit ledger; it should be done with native Rust test coverage available.
- `application/research_workspace.rs` still combines workflow policy, validation, statistical evaluation and SQLite persistence. Repository ports and focused service modules remain unfinished.
- `application/strict_analysis.rs` still combines contracts, work-plan lifecycle, validation and audit persistence.
- `infrastructure/ai/agent_runtime.rs` still combines provider orchestration, credential lifecycle, artifact registry and audit journaling.
- `infrastructure/providers/public_market.rs` owns bounded provider parsing and refresh orchestration; a provider-client/normalizer split remains desirable.
- The application currently uses concrete runtime/infrastructure services in several places. Formal traits/ports and dependency injection should be added where they remove real coupling, not as unused abstractions.
- Market-data time provenance is not yet a broker session calendar. Broker timezone, DST, holidays and per-symbol trading sessions must be modeled before classifying closed-session intervals as missing bars.
- MT5 loopback transport is not encrypted. It is limited to local-machine use and must not be exposed on a network interface. A reviewed authenticated local IPC transport or mutually authenticated encrypted channel is required before remote-terminal support.
- `MqlTick` does not expose a globally unique tick identifier; exact reconstruction cannot be guaranteed for pathological high-density same-millisecond pagination windows.

## Deliberately excluded capabilities

There is no live MT5 order adapter, no AI-owned risk-limit mutation, no LLM order signing, no synthetic production market feed, and no automatic transition from backtest approval to live trading. The current execution surface is paper-only.

## Deployment posture

Keep the product as a local modular monolith until profiling or operational requirements justify process separation. Do not split into microservices just to satisfy a diagram. Release certification requires reviewed lockfiles, reproducible frozen dependency installs, native/frontend builds, Rust tests and Clippy, MQL5 compilation, real MT5 terminal acceptance, persistence/crash testing, OS credential-store acceptance, platform packaging, signed release attestations and cross-platform acceptance.
