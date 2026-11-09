# Architecture boundaries — 0.6.0

The core app remains `Svelte presentation → application service → deterministic analysis/decision engines → Tauri IPC → Rust authority`. The release adds a market-data assurance boundary inside the shared JavaScript domain layer without granting that layer execution authority.

## Timeframe contract and data assurance

`domain/market-data/timeframe-contract.js` owns supported timeframe widths. Both the frontend validator and optional local research API import it. The analysis engine separately invokes `domain/market-data/candle-series-audit.js` so a direct call cannot bypass UTC grid, timestamp, or exact-cadence checks merely by setting a `validated` flag in metadata.

The audit distinguishes hard temporal/OHLCV integrity failures from soft latest-bar outlier warnings. Hard failures suppress indicators and directional signals. Soft return/volume/flatline warnings remain advisory and visible in the analysis envelope; they change status to `partial` and block candidate handoff until a warning-free snapshot is supplied, without asserting that a real shock is corrupted data.

## Dependency direction

- `domain`: pure timeframe and OHLCV invariants, robust statistics, and value contracts; no framework/provider runtime imports.
- `engines`: deterministic indicator calculations, causal structure observations, assurance orchestration, and research-only decision evaluation.
- `application`: immutable-snapshot and request-scope boundary.
- `adapters`: Tauri IPC and optional Hono HTTP transport.
- `presentation`: Svelte/dashboard diagnostics.
- `infrastructure`: serialization and release evidence.

Hono stays out of the domain and deterministic-engine implementation. The local Bun/Hono API remains opt-in, loopback-bound, research-only, and separate from the desktop/native risk path.

## Release status

This is a validated source increment, not a production certificate. Dependency lockfiles, Bun/Hono runtime integration in this local workspace, Svelte/Vite build, native Rust format/tests/Clippy, signed/platform installers, and desktop acceptance remain release gates. See `VALIDATION_REPORT_0.6.0.md` and `MARKET_DATA_ASSURANCE_0.6.0.md`.
