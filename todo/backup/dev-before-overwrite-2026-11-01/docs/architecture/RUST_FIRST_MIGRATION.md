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

The native technical-analysis domain now calculates the six user-visible SMA, EMA, RSI, MACD, ATR, and Bollinger readings, derives bounded trend/momentum/location signal observations from those metrics, and calculates confirmed swing classifications and close-confirmed structure breaks through existing Rust SMC primitives. The Svelte view obtains indicator readings, raw research-only indicator signals, and the structure summary through typed Tauri commands. Native indicator signals are displayed separately and are not fed into the JavaScript decision verdict or treated as canonical evidence. The native boundary validates OHLCV invariants, positive/increasing timestamps, bounded candle count, and numeric overflow for indicators; structure inputs are validated before detector invocation. The JavaScript research-analysis engine still computes its own indicators, pivots, breaks, and research decision. Rust signal evidence references are diagnostic numeric observations, not yet canonical evidence-registry records. The visible indicator cards and structure summary now use native results, but decision evaluation and duplicate legacy calculations remain to be migrated. This slice does not establish the 80/20 target.

## Completion criteria

- A generated source-line report demonstrates at least 80% Rust among production business-logic lines.
- No production `.js` modules remain in application/domain/engine paths.
- TypeScript production code is limited to UI, typed IPC, and presentation glue.
- Rust implementations are used by actual app workflows, covered by native tests, and checked in CI.
- No legacy JavaScript implementation is removed until its Rust replacement passes parity tests and its production caller has switched.
