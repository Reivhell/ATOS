# Validation Report — Agentic Trading OS 0.6.0

Status: **CANDIDATE — NOT PRODUCTION-CERTIFIED**

## Implemented in this increment

- Added one canonical JavaScript timeframe contract shared by the market-data validation boundary, analysis engine and opt-in local research API.
- Added bounded candle-series assurance: finite/valid OHLCV geometry, supported timeframe, positive safe-integer timestamps, UTC-grid alignment, strict ordering, and exact interval continuity.
- The analysis engine now enforces cadence independently of adapter metadata. A bad interval/time grid suppresses metrics and directional signals and fails closed as `invalid_data`.
- Added causal latest-bar outlier diagnostics for robust price-return deviations, abnormal volume relative to the preceding 20-bar median, and identical flat-price bars. These are explicit advisory warnings, not automatic data rejection; while a warning is present, the envelope becomes partial and candidate handoff is blocked.
- Promoted the technical-analysis envelope to `technical-analysis-envelope.v2`; the UI displays audit status, integrity faults, and advisory-warning counts.
- Added 7 focused tests for the timeframe contract, clean series, timestamp faults, gaps, direct-engine fail-closed behavior, anomaly warnings, flatline warnings, and bounded audit work.
- Aligned package, Tauri, Cargo and public-provider user-agent versions to 0.6.0.

## Verification performed in this workspace

- `npm test`: **PASS — 149 tests, 25 Node test suites, 0 failures**.
- `npm run typecheck:layers`: **PASS** using strict TypeScript checking of JavaScript domain/application/engine layers.
- `npm run verify:repo`: **PASS**, with explicit warnings that `package-lock.json`, `bun.lock`, and `src-tauri/Cargo.lock` are absent.
- The new audit tests prove that cadence/grid violations produce no numeric indicator metrics or directional signals, while statistical outliers remain visible as warnings.

## Not verified / release blockers

- Bun/Hono HTTP runtime integration: not run locally because Bun and the installed Hono dependency are unavailable.
- Svelte/Vite/Tauri build: attempted; `npm run build` is blocked at `svelte-check: not found` because dependencies are not installed in this workspace. Vite and native Tauri build have therefore not run.
- Rust formatting, compilation, unit tests, and Clippy: not run because the Rust toolchain is unavailable.
- Release preflight fails closed with 9 environment/reproducibility blockers: three missing lockfiles (`package-lock.json`, `bun.lock`, `src-tauri/Cargo.lock`), unavailable frontend executables (`svelte-check`, `vite`, `tauri`), and unavailable Rust tooling (`cargo`, `rustfmt`, `cargo clippy`). Dependency installation previously failed due unavailable registry DNS in this environment; no fake lockfiles were created.
- Platform-specific installer signing, startup/upgrade tests, and desktop end-to-end acceptance remain unverified.

## Release decision

Do not call this artifact production-ready or release-certified. `npm test`, strict source typecheck and repository guardrails are green, but they do not substitute for frozen dependency installation, Bun/Hono runtime acceptance, the Svelte/Tauri build, native Rust verification, and OS-level desktop acceptance. The optional API remains local-only and research-only. Live trading remains disabled.
