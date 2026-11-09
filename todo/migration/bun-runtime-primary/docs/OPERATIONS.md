# Operations and validation

## First run

1. Install Node.js 20+ and a current Rust stable toolchain.
2. Install Tauri's native build prerequisites for the host OS: <https://v2.tauri.app/start/prerequisites/>.
3. From the repository root, run `npm install`.
4. Run `npm run tauri dev` to open the native desktop application.
5. Use **Sync data** to request public Binance Spot market data. Until it succeeds, no market quote/candle observation is available and price-dependent actions remain blocked. Verify provider status, exact instrument, and update age before paper order entry.
6. Open **Meme Coin Screener** and refresh the DEX Screener boosted-token feed. This is an unrepresentative, promotion-biased discovery subset. Inspect the source timestamp and unresolved risk flags; the integration is not a contract-security, holder-distribution, or liquidity-lock scanner.

Public provider refreshes are throttled (3 seconds for market data, 10 seconds for the screener). Response bodies are capped while streaming: 1 MiB for ticker/boost discovery, 2 MiB for klines, and 4 MiB per chain pool response. A size violation is treated as a provider failure and must not update the last-good data timestamp.

The browser runtime is disabled rather than populated with synthetic data. Dashboard access, order submission, risk writes, backtests, and agent status changes require the Rust desktop runtime; no browser action can create a paper fill.

## State and recovery

The state database is `agentic-trading-os.sqlite3` under the Tauri application data directory. A separate SQLite guard database holds an exclusive transaction for the engine lifetime, so a second process cannot open and overwrite the same paper account. The operating system releases this lock if the process exits unexpectedly. Close the existing app before opening the same account in another window/process. SQLite transactions commit the serialized engine state and audit rows together. The audit table is append-only from the application command surface; no UI control deletes it. State schema version is stored in SQLite `user_version` and `app_state.schema_version`.

If startup reports that the exclusive instance lock is already held, close the other running copy and retry; do not manually delete the lock database while an app process is open. If existing state cannot be decoded or validated, the engine moves the database and any available SQLite `-wal` / `-shm` sidecars to a timestamped `.corrupt-...` backup and creates a new local paper account. If backup preservation fails, startup fails closed. If either the SQLite schema or embedded state-payload schema is newer than this binary supports, startup refuses to open it and leaves it untouched. Startup also re-marks restored positions against their saved quote set and refuses to persist a state whose resulting equity or invariants overflow. If durable storage cannot be initialized, startup fails instead of silently operating without persistence. Orphaned `-wal` / `-shm` files without a main database are also preserved under a timestamped backup before a clean database is created. Review the backup before proceeding. A state recovery event is recorded in the new account when possible. Keep periodic closed-app backups for any data that must be retained.

## Risk behavior

- Order type: paper market orders only. A submitted `limitPrice` is rejected, not silently converted to a market order.
- Quote age: new order intents require a quote no older than 120 seconds, with a future timestamp rejected.
- Sizing: max order notional, per-symbol marked position notional, available cash or inventory, fees, and configured adverse slippage are checked in Rust.
- Daily loss: the baseline rolls at UTC date boundaries. New risk-increasing orders are gated after the configured loss; sells that reduce existing paper exposure remain allowed unless the kill switch is engaged.
- Kill switch: blocks all new paper order intents. Resetting the paper account does not release it; a separate explicit operator action is required. Emergency liquidation is not implemented.
- Persistence: a failed state/audit commit returns an error and rolls in-memory state back; do not treat a UI notification as a fill unless the Rust command returned `FILLED (PAPER)`.
- Agent switches: toggle observe-only status and emit events; they do not initiate order placement.

## Checks to run before distributing a build

```bash
npm run verify:repo
npm run test:exports
npm run test:provenance
npm run test:technical-analysis
cargo fmt --manifest-path src-tauri/Cargo.toml --all
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --all-features
npm run build
npm run tauri build
```

`verify:repo` is a dependency-free static preflight only; it is not a Rust/Svelte compiler or a substitute for the full build and tests. The repository currently has no npm, Bun, or Cargo lockfile, so dependency resolution still needs to be pinned before a reproducible release. Tagged-release CI is deliberately configured to refuse a release until all three lockfiles are generated by their package managers and committed; placeholder lockfiles are not acceptable.

Also test manual acceptance flows on each supported OS:

- Launch offline; confirm quote/candle/screener data stays unavailable rather than being populated with examples, and verify orders remain blocked without an exact-symbol fresh quote.
- Refresh with network unavailable, a partial symbol response, and malformed data; ensure stale quotes keep their original timestamp and become unusable for orders after the freshness limit.
- Try invalid quantity, unknown symbol, over-cap order, insufficient cash/inventory, duplicate client-order ID, reused ID with a different order, stale quote, daily loss, and kill switch.
- Restart the application after a filled paper order; verify balances, position, order history, idempotency, and audit history persist.
- Feed an extreme finite quote into a copied test state with an open position; verify mark-to-market overflow is rejected and the prior state is retained.
- Corrupt a copied test database and verify it is preserved before paper-state recovery.
- Run acceptance backtests only against provider-fetched candles and verify source/provenance disclosure, fees/slippage, observed completed-bar crossover, next-bar execution, and fail-closed extreme-input behavior. Unit-test fixtures are test-only and must never be exposed through runtime UI or provider state.
- Exercise DEX Screener empty/partial payloads, rate limits, timeout and malformed tokens; confirm the previous rows and last-success timestamp are retained and the provider is marked stale.
- Verify the screener UI clearly states promotion bias and does not present unknown contract/holder/liquidity-lock status as safe.
- Verify the Live Trading page cannot submit orders and no credentials are requested or stored.


## Exporting research and audit data

- In **Audit Log**, the CSV export includes the currently loaded, currently filtered events only. It does not include events outside the loaded page/window; the UI shows how many rows are being exported. Fields are quoted, timestamps are serialized in UTC, and text cells beginning with common spreadsheet formula markers are prefixed to reduce CSV formula-injection risk.
- In **Backtesting**, export a completed result as a versioned JSON report. It captures result metrics, the runner parameters, the equity curve, data-source label, warnings/caveats, canonical OHLCV SHA-256 fingerprint, timestamp range, encoding version, and median candle interval. The fingerprint detects whether two reports used the same encoded candles, but it is not a signature of provider authenticity. The export does **not** embed raw candles, so it remains a saved report, not a full replay package.
- Treat exported files as local research records. They may contain detailed operational/audit text; share them only after reviewing their contents.
