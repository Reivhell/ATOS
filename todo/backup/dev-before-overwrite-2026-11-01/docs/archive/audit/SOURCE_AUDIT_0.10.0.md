# Source audit — Agentic Trading OS v0.10.0

**Audit date:** 2026-10-30  
**Baseline:** supplied `Agentic-Trading-OS-v0.9.6-attested-release-supply-chain-2026-10-30.zip`  
**Scope:** new research subsystem and all directly modified integration, UI, manifest, release-policy and documentation files.

## Change inventory

Compared with the supplied v0.9.6 archive:

- 5 files added: `src-tauri/src/research_workspace.rs`, `scripts/test-research-workspace-contract.test.mjs`, `docs/RESEARCH_WORKSPACE_0.10.0.md`, `VALIDATION_REPORT_0.10.0.md`, and `SOURCE_AUDIT_0.10.0.md`.
- 17 files changed, including `src-tauri/src/engine.rs`, `src-tauri/src/lib.rs`, `src/App.svelte`, the Tauri bridge/type/runtime boundary, manifests, release-version assertions, README, changelog and security documentation.
- 0 files deleted.

The new Rust research module is approximately 1,400 lines and is wired into the existing Tauri application. Version metadata is aligned at `0.10.0` across npm, Cargo and Tauri configuration.

## Data-flow and authority review

1. The Svelte Research Lab collects hypothesis and research parameters; it does not calculate or fabricate market candles.
2. The frontend bridge requires the native Tauri runtime and invokes typed research commands.
3. Tauri dispatches to Rust `ResearchWorkspace` methods. The backend independently validates request text, identifier, market, timeframe, periods, capital and thresholds.
4. `run_workflow` reads fee/slippage assumptions through `TradingCore::research_risk_limits`, which validates the account state without calling the dashboard snapshot path or persisting valuation updates.
5. The native `TradingCore::run_backtest` produces the underlying backtest result. The research workflow validates its data fingerprint, timeframe, period, provenance fields and numeric metrics before statistical review.
6. OOS and bootstrap review run as separate scoped worker tasks against the same immutable result. A worker panic or invalid result aborts the workflow before the card is stored.
7. Research records are committed transactionally to a dedicated `research-workspace.sqlite3` database. The research subsystem does not access `EngineState`, write account state, create order intents, place orders, change risk controls or alter the kill switch.
8. Returned cards are rendered from native Tauri responses and revalidated by content hash and semantic checks when loaded.

No Python runtime was introduced. No fake candle fixtures, fabricated provider responses, demo records, or mock investment decisions were added.

## Findings corrected during this change

- **Trigger verifier false rejection:** the previous case-sensitive SQL substring approach could reject a valid SQLite trigger definition. The verifier now compares the full normalized trigger SQL definition and rejects missing or weakened guards. Regression coverage includes a weakened `RAISE(IGNORE)` trigger.
- **Statistical metric mismatch:** return percentiles were not comparable with a maximum-drawdown threshold. The gate now compares observed drawdown and the bootstrap 95th-percentile drawdown only against drawdown limits.
- **Bootstrap method ambiguity:** the implementation is a non-circular moving-block bootstrap and is labelled accordingly. Sampling now uses deterministic PCG-XSH-RR 64/32 with rejection sampling for unbiased bounded indices; the dataset fingerprint seeds the reproducible stream.
- **Unsupported hypothesis market:** hypothesis creation previously accepted arbitrary valid symbols/timeframes even though the research runner only supports BTCUSDT 5m. The Rust creation boundary now enforces the same exact native market contract as the run path.
- **Mutable hypothesis integrity gap:** the initial implementation hash-chained lifecycle events but did not cross-check the mutable hypothesis row against the event sequence. Startup and every write transaction now replay lifecycle events against persisted title/thesis/symbol/interval/status/revision/timestamps, with immutable identity and no-delete triggers. A regression test mutates status/revision without an event and expects integrity failure.
- **Stale hypothesis selection:** the UI could retain an invalidated hypothesis as selected after refresh. Selection and execution now require a non-invalidated record; the backend independently rejects invalidated hypotheses and commits against the expected revision.
- **Duplicate hypothesis submission:** the create action now has an independent in-flight guard to prevent repeated clicks from creating duplicate records.
- **Misleading registry count:** the UI now distinguishes total stored cards from the latest cards loaded into the current view.
- **Incidental account writes:** research risk context now uses a dedicated read-only Rust accessor rather than invoking the snapshot path which can mark/persist positions.

## Guardrails reviewed

- Separate database and strict storage bounds for hypotheses, run cards, audit events and serialized card size.
- `BEGIN IMMEDIATE` transactions for hypothesis mutation, invalidation and run-card persistence.
- Optimistic revision check at commit time; a hypothesis invalidated/changed while a backtest runs cannot receive a stale run card.
- Every research mutation transaction re-verifies the current event chain, hypothesis replay state, immutable trigger definitions and persisted card hashes before it writes.
- Append-only event chain with sequence continuity, payload validation, previous-hash linkage and SHA-256 verification.
- Hypothesis identity columns have immutable/update-delete protections; mutable status/revision/timestamps are replay-checked against the ordered creation, review and invalidation events. Mutating lifecycle state without a corresponding valid audit event fails integrity verification.
- Immutable run-card and event-log SQLite triggers, revalidated against exact normalized definitions on startup.
- Run-card content hash plus semantic validation of identity, native market contract, data fingerprint, metrics, role roster, DAG, algorithm version and gate thresholds.
- Bounded bootstrap observations and total resampling work; finite-value checks and positive-equity checks before calculations.
- No `unsafe` blocks or production-path `unwrap()`, `expect()`, `panic!()`, `todo!()` or `unimplemented!()` calls found in the new research module. `unwrap()` usage in that module is confined to unit tests.
- Source-contract tests exercise the backend/Tauri/frontend integration contract, data provenance boundaries, storage immutability policy, DAG and hypothesis lifecycle.

## Test evidence

The local validation suite reported 266/266 Node tests passing across 41 `npm run test:*` command groups, strict layer typecheck passing, all 85 JavaScript module files passing `node --check`, and repository static guardrails passing with lockfile warnings. A separate SQLite standard-library smoke check confirmed that the new DDL and all six exact trigger definitions are accepted by SQLite and that identity edits/deletion are rejected. This does not replace the unrun Rust SQLite integration tests. The research workspace contract suite contains seven tests.

These are not native runtime proofs. Cargo, rustfmt and Clippy are absent in this environment, and `svelte-check`/Vite/Tauri CLI are unavailable. Consequently, Rust compilation, SQLite native tests, Svelte compilation, Tauri packaging, integration behavior and cross-platform acceptance remain unverified. See `VALIDATION_REPORT_0.10.0.md`.

## Remaining design and operational gaps

- Current data support and strategy coverage remain narrow: one BTCUSDT 5m market series and the existing SMA crossover.
- Specialist reviewers are deterministic Rust validators, not live LLM agents or independent external research processes.
- The research store has hard limits of 100 immutable run cards and 1,024 events but does not yet have in-app archive rotation. This must be resolved before long-running high-volume operational use.
- Local hashes and SQLite triggers are tamper-evident controls, not cryptographic non-repudiation or defense against a privileged local attacker.
- This candidate has no live execution or MT5 integration. The no-live-order boundary is retained intentionally.

## Audit conclusion

The new research path is separated from the account/order authority boundary and is covered by runnable static/source tests. The reviewed source does not justify a production certification claim because native compilation/tests, frontend packaging, reproducible lockfiles, immutable CI action pins, signed installer attestations and platform acceptance have not passed. Keep the release status **BLOCKED — NOT PRODUCTION-CERTIFIED** until the validation report's conditions are satisfied.
