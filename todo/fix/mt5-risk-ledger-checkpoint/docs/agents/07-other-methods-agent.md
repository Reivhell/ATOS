# Agent Knowledge Pack — Metode Analisis Lain

Contract: Strict Master List v1.0.0. Category entries: **42**.

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

- Assess every Wyckoff, Elliott Wave, harmonic, and other-method catalog ID independently; preserve competing counts/interpretations when ambiguity exists.
- Elliott/Wyckoff/harmonic concepts must define pivot selection, pattern completion, rule set, tolerances, invalidation, and when the pattern became knowable. Do not backdate a wave count or PRZ confirmation to its visual starting point.
- Statistical arbitrage, mean reversion, trend-following, and seasonal claims require explicit hypotheses, pre-registered windows, costs, out-of-sample validation, survivorship/look-ahead controls, and reproducible implementation IDs.
- Subjective or unimplemented methods remain `unsupported`; a plausible interpretation is not evidence of a tested detector or profitable edge.

## Complete category inventory

- `Metode Analisis Lain::Wyckoff::Wyckoff Accumulation` — **Wyckoff Accumulation** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::Wyckoff Distribution` — **Wyckoff Distribution** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::Spring` — **Spring** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::Upthrust` — **Upthrust** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::UTAD (Upthrust After Distribution)` — **UTAD (Upthrust After Distribution)** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::SOS (Sign of Strength)` — **SOS (Sign of Strength)** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::SOW (Sign of Weakness)` — **SOW (Sign of Weakness)** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::LPS (Last Point of Support)` — **LPS (Last Point of Support)** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::LPSY (Last Point of Supply)` — **LPSY (Last Point of Supply)** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::Selling Climax` — **Selling Climax** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::Buying Climax` — **Buying Climax** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::Automatic Rally` — **Automatic Rally** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::Automatic Reaction` — **Automatic Reaction** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Wyckoff::Secondary Test` — **Secondary Test** (group: Wyckoff; implementation status: Catalog only)
- `Metode Analisis Lain::Elliott Wave::Impulse Waves 1–5` — **Impulse Waves 1–5** (group: Elliott Wave; implementation status: Catalog only)
- `Metode Analisis Lain::Elliott Wave::Corrective Waves A–B–C` — **Corrective Waves A–B–C** (group: Elliott Wave; implementation status: Catalog only)
- `Metode Analisis Lain::Elliott Wave::Zigzag` — **Zigzag** (group: Elliott Wave; implementation status: Catalog only)
- `Metode Analisis Lain::Elliott Wave::Flat Correction` — **Flat Correction** (group: Elliott Wave; implementation status: Catalog only)
- `Metode Analisis Lain::Elliott Wave::Triangle Correction` — **Triangle Correction** (group: Elliott Wave; implementation status: Catalog only)
- `Metode Analisis Lain::Elliott Wave::Leading Diagonal` — **Leading Diagonal** (group: Elliott Wave; implementation status: Catalog only)
- `Metode Analisis Lain::Elliott Wave::Ending Diagonal` — **Ending Diagonal** (group: Elliott Wave; implementation status: Catalog only)
- `Metode Analisis Lain::Elliott Wave::Wave Extension` — **Wave Extension** (group: Elliott Wave; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Gartley` — **Gartley** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Bat` — **Bat** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Butterfly` — **Butterfly** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Crab` — **Crab** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Deep Crab` — **Deep Crab** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Shark` — **Shark** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Cypher` — **Cypher** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::ABCD Pattern` — **ABCD Pattern** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Three Drives` — **Three Drives** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Harmonic Patterns::Potential Reversal Zone (PRZ)` — **Potential Reversal Zone (PRZ)** (group: Harmonic Patterns; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Dow Theory` — **Dow Theory** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Gann Theory` — **Gann Theory** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Mean Reversion` — **Mean Reversion** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Trend Following` — **Trend Following** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Momentum Trading` — **Momentum Trading** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Breakout Trading` — **Breakout Trading** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Range Trading` — **Range Trading** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Statistical Arbitrage` — **Statistical Arbitrage** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Quantitative Analysis` — **Quantitative Analysis** (group: Teori Lain; implementation status: Catalog only)
- `Metode Analisis Lain::Teori Lain::Seasonal Analysis` — **Seasonal Analysis** (group: Teori Lain; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-07-other-methods` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.