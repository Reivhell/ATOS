# Technical Analysis Master List — Strict Contract

Contract version: `1.0.0`  
Application revision: `0.1.7`  
Inventory size: `364` contextual entries  
Top-level categories: `9`

## Source of truth and drift protection

The supplied master list is represented by the ordered catalog in `src/lib/technical-analysis.js`. The checked-in JSON snapshot at `scripts/fixtures/technical-analysis-master-list.v1.json` is the reviewed contract for each row's `id`, category, subgroup, name, implementation status, and kind. The test suite checks the complete ordered record list, not a sample or a minimum count.

A missing, added, renamed, reordered, recategorized, or relabeled entry makes `npm run test:technical-analysis` fail until the change is intentionally reviewed and the contract snapshot is updated. This is deliberate: list drift must never be silent.

## Exact category names and counts

| Category | Entries |
|---|---:|
| SMC & ICT | 97 |
| Price Action | 67 |
| Support, Resistance & Trend | 19 |
| Indikator Teknikal | 52 |
| Order Flow & Market Profile | 21 |
| Divergence & Intermarket Analysis | 15 |
| Metode Analisis Lain | 42 |
| Fundamental & Makro | 20 |
| Risk Management & Evaluation | 31 |
| **Total** | **364** |

Combined concepts from the requested master list are expanded into separately searchable entries where useful (for example HH/HL/LH/LL, bullish/bearish patterns, and market/limit/stop orders). SMT Divergence intentionally appears in two contexts: SMC & ICT and Divergence & Intermarket Analysis. That yields 364 contextual entries and 363 unique displayed names; catalog IDs remain unique because they include category and group.

## Implementation status is a separate contract

| Status | Count | Meaning |
|---|---:|---|
| Calculated now | 6 | SMA 9/21, EMA 20, RSI 14, MACD 12/26/9, ATR 14, Bollinger Bands 20/2σ. |
| Workflow available | 1 | The existing SMA-crossover backtest workflow. This does not mean each concept is used by that workflow. |
| Catalog only | 357 | Searchable taxonomy entries without an implemented detector/calculator or tested signal logic. |

The UI filter exposes all three statuses independently. Code review, unit tests, known-vector fixtures, data requirements, and no-look-ahead checks are required before a catalog-only entry can be promoted. Presence in the master list alone does not imply a trading signal, profitability, or permission to execute an order.

## Updating the contract

1. Map the requested terminology into the correct category and subgroup; avoid deleting synonyms just to make counts fit.
2. Implement the behavior and its data/uncertainty contract before changing status from `Catalog only`.
3. Update expected total/category counts and the reviewed manifest only after the taxonomy diff has been checked against the source master list.
4. Add deterministic tests, including missing/malformed input and anti-look-ahead behavior where relevant.
5. Run `npm test` and `npm run verify:repo`; then complete Svelte/Rust compilation and cross-platform validation before release.

Never mark a detector as available because a name is in the catalog or because its UI row exists. Live trading remains outside this contract and unavailable in the current build.


## Decision contract v3 supplement

The 364-entry taxonomy remains Master List v1.0.0. Gate v3 binds every result to the exact category, group, name and category-owned agent ID; evaluated records additionally require a reviewed implementation version and data snapshot ID. The market-data envelope requires source, timezone-aware timestamp, exact symbol, timeframe, and a SHA-256 dataset fingerprint. `not_applicable` requires contextual evidence. These are decision-contract changes, not changes to the taxonomy version.


## Rust runtime enforcement (v0.3.1)

`src-tauri/src/application/strict_analysis.rs` embeds the reviewed manifest with `include_str!`, validates the canonical IDs and category counts at runtime initialization, and owns the native work-plan/completion commands. The gate validates all 364 item-level assessments, exact category/group/name and specialist routing, timestamp format, evidence, implementation version, SHA-256 data snapshot binding, instrument/timeframe scope, risk approval, and explicit research/paper-trade intent. Its `nextLayerPayload` is absent unless every barrier passes. `liveExecutionAuthorized` is always `false`.

The JavaScript gate and planner remain deterministic mirrors used for frontend contract regression tests; they are not a replacement for the Rust runtime. Creating a work plan does not execute or dispatch LLM agents. No LLM/provider adapter is configured by this change, so the system must not claim that nine agents ran merely because nine tasks were prepared. Native Rust tests and Clippy must pass in CI before this module can be called build-verified.


### Native boundary hardening in v0.3.1

- The Tauri command does not trust UI-supplied candles, source label, validated flag, or risk approval. It reconstructs the analysis market context from the fresh connected Rust-owned provider cache.
- Work plans have a random Rust-issued ID stored in a bounded in-process registry for 15 minutes. Altered, unknown, or expired plans are blocked.
- A bar must be fully closed by `asOf`, not merely have an opening timestamp before the cutoff.
- Account preflight reads the native paper-account kill switch and maximum-daily-loss policy. Order-specific sizing, market-data freshness, cash/inventory, fee/slippage, and position limits are still checked independently when a paper order is submitted.
- Only BTCUSDT 5m is currently a supported native candle series. Other symbol/timeframe pairs fail closed; they are not silently substituted.
