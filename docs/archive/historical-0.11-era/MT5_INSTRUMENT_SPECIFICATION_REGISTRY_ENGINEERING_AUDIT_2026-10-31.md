# ATOS MT5 Instrument Specification Registry — Engineering Audit

Date: 2026-10-31  
Application version: `0.10.1` (unchanged)  
MT5 telemetry protocol: `3`  
Market-data SQLite schema: `2`

## Executive result

This tranche implements a broker-reported instrument specification registry for the primary MT5 quote symbol. It extends the EA/Rust wire contract, stores current specifications and append-only revision history in the existing dedicated MT5 SQLite database, verifies the revision chain on reads and before extending it, and exposes an authenticated Tauri query to the MT5 workbench.

This is a completed source implementation, not a certified production release. Native Rust compilation/tests, Svelte production build, MetaEditor compilation, and real broker-terminal integration remain unverified because the required toolchains and terminal are not present in this environment.

## Architecture and data flow

`MQL5/Experts/ATOS_Bridge.mq5` collects broker properties through `SymbolInfoDouble`, `SymbolInfoInteger`, and `SymbolInfoString`. Protocol v3 carries the metadata in the authenticated snapshot. Rust validates the contract fields and persists them in the same `BEGIN IMMEDIATE` transaction as market-data rows and the durable frame receipt. The EA receives an ACK only after that transaction commits.

The registry key is `(broker_server, account_login, symbol)`. It is account- and broker-scoped; one broker's symbol definition is never reused for another account/server.

The current projection is stored in `mt5_instrument_specs`. Material contract changes append to `mt5_instrument_spec_revisions`; SQLite triggers reject UPDATE and DELETE operations on that history. Reads verify contiguous revision numbers, predecessor hashes, each revision payload fingerprint, the current projection fingerprint, and agreement between the projection and the final history row. Ingestion also verifies the existing projection/history before extending it, so a detected corruption blocks the next frame rather than extending a broken chain.

## Captured broker properties

- Symbol digits, point, contract size, and calculation mode.
- Trade mode, execution mode, allowed filling flags, and order flags.
- Tick size, tick value, profit tick value, and loss tick value.
- Minimum, maximum, step, and directional volume limit.
- Stop and freeze levels in points.
- Base, profit, and margin currencies.
- Swap mode and long/short swap values.

The official MQL5 Symbol Properties reference documents these values and the `SymbolInfoDouble`, `SymbolInfoInteger`, and `SymbolInfoString` accessors: <https://www.mql5.com/en/docs/constants/environment_state/marketinfoconstants>.

### Dynamic valuation vs static contract identity

`SYMBOL_TRADE_TICK_VALUE`, `SYMBOL_TRADE_TICK_VALUE_PROFIT`, and `SYMBOL_TRADE_TICK_VALUE_LOSS` are broker-calculated account/deposit-currency valuations and can change as conversion rates move. They are refreshed in the current projection but intentionally excluded from the static contract fingerprint. This prevents ordinary FX conversion changes from creating a new instrument-contract revision on every heartbeat. The static specification fields remain fingerprinted and revisioned.

## Migration and compatibility

- The MT5 market-data database is upgraded additively from schema v1 to v2. Existing tick/bar rows are not rewritten.
- The wire protocol moves from v2 to v3 because the snapshot now requires the new broker metadata fields under strict `deny_unknown_fields` decoding.
- The repository EA has been updated to protocol v3. A protocol-v2 EA is not compatible with this source build; operators must compile/install the EA from `MQL5/Experts/ATOS_Bridge.mq5` before reconnecting.
- Application package version remains `0.10.1`; no cosmetic version bump was made.

## Security and correctness boundaries

- The Tauri query is bound to the last authenticated account/server identity; caller-supplied foreign identities are rejected.
- Specification payloads are validated for finite numbers, positive contract/tick/volume constraints, bounded enum/flag ranges, and restricted currency identifiers.
- The revision history is append-only at the SQLite trigger boundary and is checked for integrity when read and before a new revision is committed.
- The EA remains read-only. No order submit, amend, close, or delete API was added.
- The current registry captures the primary quote symbol only. Other subscribed symbols may stream ticks/bars but do not receive registry entries until they become the primary symbol.
- The returned record is explicitly labelled `broker_snapshot_v1_partial`. It is not a complete margin/profit model and must not be used alone for live position sizing.
- Session calendars, per-day session windows, holidays, timezone/DST normalization, start/expiration dates, margin rates, and broker-specific contract overrides are not included yet.

## Files changed

- `src-tauri/src/infrastructure/mt5/market_data.rs`
- `src-tauri/src/infrastructure/mt5/bridge.rs`
- `src-tauri/src/application/market_data_query.rs`
- `src-tauri/src/adapters/tauri/commands.rs`
- `src-tauri/src/adapters/tauri/bootstrap.rs`
- `MQL5/Experts/ATOS_Bridge.mq5`
- `src/lib/types.ts`
- `src/lib/adapters/tauri/bridge.ts`
- `src/lib/adapters/tauri/native-runtime-guard.js`
- `src/lib/components/Mt5BridgePanel.svelte`
- `scripts/test-mt5-instrument-specification-registry.test.mjs`
- `package.json`
- `docs/MT5_BRIDGE_SECURITY_AND_OPERATIONS.md`
- `docs/MT5_INSTRUMENT_SPECIFICATION_REGISTRY_2026-10-31.md`

## Verification results

| Check | Result |
|---|---:|
| Node regression suite | 300 passed, 0 failed |
| TypeScript layer typecheck | Passed |
| Tauri adapter/type contract check with temporary `invoke` declaration | Passed |
| Repository guardrails | Passed with 3 lockfile warnings |
| JavaScript syntax scan | 90 files passed |
| JSON parse validation | 18 files passed |
| Embedded Rust asset path audit | 10/10 paths resolved |
| Tauri command declaration/registration parity | 36/36 exact match |
| MQL5 `StringFormat` placeholder/argument parity | 48/48 |
| MQL5 trade mutation API scan | No order mutation API found |
| SQLite schema/trigger smoke check | Passed using SQLite runtime |
| Rust compile, Rust unit tests, Clippy | Not run; Cargo/Rust toolchain unavailable |
| Svelte production build | Not run; `svelte-check` and Vite unavailable |
| MetaEditor/MQL5 compile | Not run; MetaEditor unavailable |
| Live MT5 broker integration | Not run; no terminal/account available |

Five new Rust unit tests cover additive schema migration, specification revision semantics, dynamic tick valuation updates, corrupted projection rejection, append-only history, and metadata validation. They are source additions only; they were not executed because Cargo is unavailable.

## Release blockers

`release:preflight` remains blocked by nine items:

1. Missing `package-lock.json`.
2. Missing `bun.lock`.
3. Missing `src-tauri/Cargo.lock`.
4. Missing `svelte-check` executable.
5. Missing Vite executable.
6. Missing Tauri CLI executable.
7. Cargo unavailable.
8. Rustfmt unavailable.
9. Cargo Clippy unavailable.

No lockfile was fabricated and no release gate was weakened. The repository is not certified as a production binary until these gates and real-terminal tests pass.
