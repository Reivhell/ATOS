# Agent Knowledge Pack — Divergence & Intermarket Analysis

Contract: Strict Master List v1.0.0. Category entries: **15**.

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

- Assess every assigned divergence/intermarket catalog ID individually. Bind each comparison to exact instruments, venues, timeframes, synchronized timestamps, and the same as-of cutoff.
- Divergence detection must specify pivot rules, oscillator/price source, tolerance, minimum separation, and confirmation timestamp. Do not retrospectively select pivots using future data.
- Correlation and lead-lag claims require a declared rolling window, return transformation, missing-bar policy, significance/uncertainty description, and proof the series were aligned without look-ahead.
- SMT and related cross-market comparisons require both instruments' validated snapshots. If either market or a reviewed implementation is absent, report `insufficient_data` or `unsupported`—never fabricate a proxy.

## Complete category inventory

- `Divergence & Intermarket Analysis::Divergence & Relationships::Regular Bullish Divergence` — **Regular Bullish Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Regular Bearish Divergence` — **Regular Bearish Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Hidden Bullish Divergence` — **Hidden Bullish Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Hidden Bearish Divergence` — **Hidden Bearish Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::RSI Divergence` — **RSI Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::MACD Divergence` — **MACD Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Stochastic Divergence` — **Stochastic Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Volume Divergence` — **Volume Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::CVD Divergence` — **CVD Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::SMT Divergence` — **SMT Divergence** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Relative Strength` — **Relative Strength** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Relative Weakness` — **Relative Weakness** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Correlation Analysis` — **Correlation Analysis** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Correlation Breakdown` — **Correlation Breakdown** (group: Divergence & Relationships; implementation status: Catalog only)
- `Divergence & Intermarket Analysis::Divergence & Relationships::Lead-Lag Relationship` — **Lead-Lag Relationship** (group: Divergence & Relationships; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-06-divergence-intermarket` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.