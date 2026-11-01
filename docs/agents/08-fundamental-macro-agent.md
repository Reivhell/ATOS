# Agent Knowledge Pack — Fundamental & Makro

Contract: Strict Master List v1.0.0. Category entries: **20**.

## Mandatory operating rules

- Treat this file as domain vocabulary and coverage specification, not proof that a detector exists.
- Assess only from validated, timestamped evidence and cite source/candle/event identifiers.
- Use `unsupported` for any concept without a reviewed implementation; never infer or fabricate a result.
- Use `insufficient_data` when the required market, timeframe, session, order-book, volume, cross-market, or macro input is unavailable.
- Use `not_applicable` only with a specific rationale tied to instrument, venue, timeframe, or strategy context.
- No look-ahead: only use information available at the assessment timestamp; distinguish confirmed from provisional structures.
- Report neutral/ambiguous results honestly. Do not turn a concept label into a buy/sell recommendation.
- Never bypass the central strict decision gate or the Rust risk gateway. Live execution is not authorized.

## Runtime execution proof (strict v0.4.2)

- This knowledge pack is a specification, not evidence that this specialist was executed. A work plan, route, prompt, or populated assessment object is not a dispatch receipt.
- The current release has no configured trusted AI-provider dispatcher. Therefore this agent must not claim that runtime execution occurred and the strict Rust gate must block handoff until a trusted runtime executes and persists the result.
- A valid durable receipt must be created only by native Rust after the real specialist call and output validation. It binds the exact issued work-plan ID, this category and routed agent ID, a hash of the task input, a canonical hash of the category assessment output, complete category coverage, and completion time.
- Missing, duplicate, stale, mismatched, fabricated, or integrity-invalid receipts are blockers. Never create, copy, or infer a receipt from a previous run. A fresh run requires a fresh issued work plan.
- Even a valid receipt proves only that the trusted native dispatcher recorded a matching category result; the central gate separately validates all 364 concepts, market-data provenance, implementation status, risk preflight, and no-look-ahead requirements. It does not authorize live execution.

## Mandatory assessment protocol

- Assess each macro/fundamental catalog ID independently and use only traceable provider/source records with publication time, period covered, retrieval time, timezone, and revision/vintage where applicable.
- Never use revised macroeconomic history as if it were known at the original release. Align event time to the instrument session and distinguish scheduled releases from headlines and realized data.
- DXY, yields, ETF flows, central-bank purchases, geopolitical risk, and risk-on/off proxies need source-specific definitions and synchronized timestamps. Qualitative interpretation must be labelled as such and cannot masquerade as a quantitative detector.
- If licensed/validated source data or a reviewed workflow is absent, record `insufficient_data` or `unsupported`; never invent a current macro state from memory.

## Complete category inventory

- `Fundamental & Makro::Macro Events & Cross-Market Context::Federal Reserve (Fed)` — **Federal Reserve (Fed)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::FOMC (Federal Open Market Committee)` — **FOMC (Federal Open Market Committee)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Interest Rate` — **Interest Rate** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::CPI (Consumer Price Index)` — **CPI (Consumer Price Index)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::PCE (Personal Consumption Expenditures)` — **PCE (Personal Consumption Expenditures)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::NFP (Non-Farm Payrolls)` — **NFP (Non-Farm Payrolls)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Unemployment Rate` — **Unemployment Rate** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::GDP (Gross Domestic Product)` — **GDP (Gross Domestic Product)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::PMI (Purchasing Managers' Index)` — **PMI (Purchasing Managers' Index)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::ISM (Institute for Supply Management)` — **ISM (Institute for Supply Management)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::DXY (US Dollar Index)` — **DXY (US Dollar Index)** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::US Treasury Yields` — **US Treasury Yields** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Real Yields` — **Real Yields** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Geopolitical Risk` — **Geopolitical Risk** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Central Bank Gold Purchases` — **Central Bank Gold Purchases** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Gold ETF Flows` — **Gold ETF Flows** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Risk-On / Risk-Off` — **Risk-On / Risk-Off** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Economic Calendar` — **Economic Calendar** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::News Volatility` — **News Volatility** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)
- `Fundamental & Makro::Macro Events & Cross-Market Context::Slippage Risk` — **Slippage Risk** (group: Macro Events & Cross-Market Context; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-08-fundamental-macro` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.