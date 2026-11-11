# Architecture Layers — v0.5.8

## Intent

This release makes implementation boundaries explicit without turning the local-first desktop app into premature microservices. The target dependency direction is **domain → engines → application → adapters**, with presentation consuming application/engine outputs and infrastructure implementing persistence/export concerns. Runtime integration may call inward contracts, never the reverse.

## Layer map

| Layer | Current home | Responsibility | Must not own |
|---|---|---|---|
| Domain | `src/lib/domain/` | Trusted catalogs, versioned contracts/policy, pure market-data validation and selection contracts | Tauri APIs, UI state, provider requests, SQLite |
| Engines | `src/lib/engines/` | Numerical indicator calculations, causal market-structure observations, strict decision gate, cross-domain research verdict | Network I/O, local persistence, UI framework, order placement |
| Application | `src/lib/application/` | Technical-analysis request boundary, work-plan creation/diagnostics, and coordination of engine contracts | Provider-specific parsing, presentation formatting, execution credentials |
| Adapters | `src/lib/adapters/tauri/` | Typed command bridge from Svelte to Rust | Owning business rules or overriding Rust decisions |
| Presentation | `src/lib/presentation/` | Pure dashboard/view-model helpers | Market-data authenticity, risk authorization |
| Infrastructure | `src/lib/infrastructure/serialization/` | Safe CSV/JSON serialization and reporting formats | Decision policy or order authority |
| Native core | `src-tauri/src/` | Authoritative state, paper broker, risk checks, database, provider access, native strict completion and backtest | Trusting the UI as security boundary |

`src/lib/types.ts` remains the shared frontend DTO contract for now; splitting it into domain-specific generated contracts would require an explicit cross-language schema generation step and Rust/TypeScript compile verification. The old `src/lib/*.js` and `src/lib/bridge.ts` paths are compatibility facades only, not alternate implementations.

## Implemented flows

### Technical analysis

1. The market-data domain validates provider response shape, requested scope, timestamp cadence, freshness metadata, and returns an immutable snapshot registered in a private in-memory `WeakSet`; copying the object shape does not reproduce trusted identity. This is an application-integrity guard, not a security boundary against compromised frontend code.
2. `application/technical-analysis-service.js` is the only frontend application entry point: it accepts that exact snapshot, checks request scope, derives cutoff metadata, gathers human-readable readings and the machine envelope, and returns boundary diagnostics. Svelte never marks arbitrary object-shaped data as validated.
3. `engines/technical-analysis/indicators.js` computes numerical outputs and signal domains using one shared indicator-metric calculation path. Invalid OHLCV blocks numerical output; partial history is labelled rather than filled with synthetic readings.
4. `engines/technical-analysis/research-detectors.js` generates confirmed swing and close-based structure observations. It remains causal and research-only.
5. `engines/technical-analysis/analysis-engine.js` combines metrics and structure into `technical-analysis-envelope.v1`, pins the final-candle cutoff, fails closed on supplied cutoff/scope mismatch, limits input to 10,000 candles, and bounds returned structure history.
6. `engines/decision/research-decision-engine.js` checks provenance, cutoff, dataset fingerprint, unique evidence domains, minimum coverage, directional agreement, and evidence strength. The verdict is a research label only. The native candle response now supplies a Rust-computed canonical fingerprint for the exact OHLCV vector; the application boundary preserves it only inside the validated immutable snapshot. Candidate eligibility still requires a live provider receipt and matching cutoff/scope.
7. Svelte renders the application report but cannot mint a dataset hash, create a native risk receipt, authorize downstream handoff, or place a live order.

### Strict full-catalog analysis

The strict 364-entry catalog and reviewed implementation map are domain-owned. The JavaScript strict gate is a parity/diagnostic implementation. The application-service wrapper cannot authorize handoff: native Rust completion and persisted audit receipts remain required. Catalog-only concepts do not become supported merely because a research detector exists.

## Dependency rules enforced by tests

- Domain modules do not import engines, application, adapters, presentation, or infrastructure.
- The technical-analysis engine does not import Tauri, network clients, or persistence APIs.
- Decision engines are pure functions of versioned input contracts; they do not dispatch orders.
- Compatibility facades remain short re-exports and do not duplicate calculations or policy. The Svelte entry point imports the application service for analysis rather than calling the engine directly.
- Data/provenance failures block candidate handoff; unknown/neutral evidence is not silently transformed into a directional vote.

## Explicit non-goals

- No microservice or Hono server is added: the current desktop control plane uses Tauri IPC into Rust and has no independent JavaScript HTTP API.
- Bun is not promoted to package-manager authority until a lockfile, frozen install, test suite, Svelte build, and Tauri build pass under Bun. Node/npm remain the supported default.
- This refactor does not configure an LLM provider, execute the nine specialist agents, certify all 364 catalog concepts, or enable live trading.

## Repeatable layer type-check

Run `npm run typecheck:layers` to perform strict TypeScript checking of the changed JavaScript engine/domain/application modules and the shared DTO definitions. The same command is run in the frontend CI job before the full Svelte/Vite build. This targeted check supplements, but does not replace, `svelte-check` or the native Rust build.

## Required production acceptance

A release is not production-certified until CI checks in a locked dependency graph; performs frontend typecheck/build; executes `cargo fmt -- --check`, `cargo test --locked --all-targets`, and Clippy with warnings denied; builds/packages the Tauri app; verifies release evidence; and completes desktop smoke and persistence/restart acceptance on supported platforms. Source-level regex/contract tests do not substitute for compiled Rust or end-to-end runtime results.
