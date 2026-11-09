# Validation Report — Agentic Trading OS v0.5.7

## Delivered capability

This candidate adds an automatic robustness suite to the Rust-owned SMA simulator:

1. Two deterministic execution-cost sensitivity scenarios (moderate and severe), returning applied fee/slippage, cap flags, final equity, total return, return delta vs baseline, max drawdown, rough Sharpe, and trade count.
2. Chronological out-of-sample (OOS) folds. Each fold is fixed length, uses only preceding bars for indicator warm-up, executes no warm-up trades, resets starting capital/position, and reports its return, drawdown, rough Sharpe, trade count, and UTC range.
3. Up to 12 most-recent complete folds are reported; incomplete tail bars are excluded. Less than two full test windows produces an explicit warning and an empty fold list, not fabricated statistics.
4. Native regression tests were added for cost outputs, fixed chronological folds, insufficient history, and invariance of a completed fold to later candle-value mutations.
5. Svelte UI tables display the cost shocks and folds; the existing complete-result JSON export includes the added result fields. No strategy parameters are optimized and no live-trading authority is added.

## Local verification results

- `npm test`: **PASS — 110 tests passed across 18 Node test files; 0 failed, 0 skipped.** These include static source-contract tests; they do not compile or run the native Rust tests.
- `npm run verify:repo`: **PASS** static repository guardrails. It warns that `package-lock.json` and `src-tauri/Cargo.lock` are missing.
- `node --check scripts/test-backtest-robustness.test.mjs`: **PASS**.
- Application version alignment: **PASS** across `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, all `0.5.7`.
- `npm run build`: **BLOCKED**, `svelte-check: not found`; the `node_modules` directory is absent.
- `cargo fmt`, `cargo test`, and `cargo clippy`: **NOT RUN**; `cargo`, `rustc`, and `rustfmt` are unavailable. The new Rust tests are authored but remain uncompiled/unexecuted here.
- Bun compatibility lane: **NOT RUN**; Bun is unavailable. npm/Node remains the official dependency/runtime path.
- Desktop runtime/installer acceptance: **NOT RUN**.
- `npm run release:preflight`: **BLOCKED — 8 release issues**: missing `package-lock.json`, missing `src-tauri/Cargo.lock`, missing installed `svelte-check`, `vite`, and `tauri` executables, and unavailable `cargo`, `rustfmt`, and `cargo clippy`.

## Interpretation limits

Cost shocks are deterministic sensitivities, not market-specific fill forecasts; ceilings can limit the applied shock. OOS windows test period-to-period variation with fixed parameters and reset capital. They do not establish profitability, capture portfolio path dependence across folds, model market impact/spread/depth/latency, or prove future behavior. A passing Node/source-contract suite is not a substitute for native compilation and execution.

## Release decision

**NOT PRODUCTION-READY / NOT RELEASE-CERTIFIED.** Production acceptance still requires pinned npm and Cargo dependency graphs, successful frontend typecheck/build, successful native Rust formatting/tests/Clippy under the locked dependency graph, and desktop runtime/packaging acceptance in CI. The application remains local-first research and paper trading. Live order execution remains disabled.
