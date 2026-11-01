# Rust-first architecture migration

## Target

Rust is the source of truth for deterministic business behavior. The long-term target is at least 80% of production business-logic lines in Rust, with the remaining code primarily TypeScript presentation, typed IPC adapters, and UI state. This target is measured on production source lines, excluding generated files, tests, documentation, and build tooling.

## Boundary rules

- Rust owns market-data validation, numerical indicators, market-structure detection, research decisions, risk sizing and gates, backtesting, persistence, audit integrity, provider normalization, and agent orchestration decisions.
- TypeScript owns Svelte UI behavior, presentation formatting, form state, typed request/response contracts, and calls to Tauri commands.
- JavaScript is not an accepted language for new production application modules. `.mjs` remains acceptable for repository tooling and tests.
- A frontend validation check may improve error messages, but it must never be the sole authority for a business decision, integrity claim, risk gate, or execution permission.
- A migrated Rust implementation must have Rust unit tests and be invoked by the production UI/API path before the legacy implementation is removed. Do not count a parallel or unused Rust implementation as migration progress.
- Avoid broad rewrites that change trading behavior. Preserve existing contracts and use parity tests against known vectors before switching callers.

## Migration order

1. Inventory source lines and establish a repeatable Rust/TypeScript/JavaScript baseline.
2. Move the technical-analysis numerical engine and candle-series assurance to Rust; retain TypeScript only for typed IPC and rendering.
3. Move research evidence validation, canonical registry checks, and research-decision evaluation to Rust.
4. Move remaining deterministic domain/application logic out of JavaScript modules into Rust, keeping provider/UI adapters thin.
5. Remove migrated JavaScript implementations and duplicate validation paths only after parity and native tests pass.
6. Require native Rust formatting, tests, linting, frontend typecheck/build, and desktop runtime acceptance before declaring the migration complete.

## Current gap

The first migration slice now calculates the six user-visible SMA, EMA, RSI, MACD, ATR, and Bollinger readings in `src-tauri/src/domain/technical_analysis.rs`; the Svelte view requests those readings through a typed Tauri command. Native tests cover valid data, insufficient history, malformed OHLCV, timestamp order, arithmetic overflow, and oversized input. The legacy JavaScript indicator implementation still runs inside the research-analysis engine and the application service still computes its compatibility reading array, so duplicate calculations and core decision logic remain to be migrated. This slice does not establish the 80/20 target.

## Completion criteria

- A generated source-line report demonstrates at least 80% Rust among production business-logic lines.
- No production `.js` modules remain in application/domain/engine paths.
- TypeScript production code is limited to UI, typed IPC, and presentation glue.
- Rust implementations are used by actual app workflows, covered by native tests, and checked in CI.
- No legacy JavaScript implementation is removed until its Rust replacement passes parity tests and its production caller has switched.
