# Agent Knowledge Pack — Support, Resistance & Trend

Contract: Strict Master List v1.0.0. Category entries: **19**.

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

- Assess every assigned catalog ID independently; levels cannot be bundled into one assessment.
- Support/resistance, channels, and trendlines must disclose the formula, pivot/anchor timestamps, tolerance, timeframe, and invalidation rule. Do not re-fit a historical line using bars that occur after the assessment timestamp.
- Psychological levels and round-number grids must declare instrument tick size and quoting convention. Multi-timeframe confluence must bind each timeframe to its own validated snapshot and as-of boundary.
- A level is an observed zone or calculation, not a guaranteed barrier or trade recommendation. If the method has no reviewed deterministic implementation, use `unsupported`.

## Complete category inventory

- `Support, Resistance & Trend::Levels & Channels::Horizontal Support` — **Horizontal Support** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Horizontal Resistance` — **Horizontal Resistance** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Dynamic Support` — **Dynamic Support** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Dynamic Resistance` — **Dynamic Resistance** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Trendline` — **Trendline** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Trend Channel` — **Trend Channel** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Parallel Channel` — **Parallel Channel** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Regression Channel` — **Regression Channel** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Donchian Channel` — **Donchian Channel** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Andrews' Pitchfork` — **Andrews' Pitchfork** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Median Line` — **Median Line** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Role Reversal` — **Role Reversal** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Polarity Change` — **Polarity Change** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Flip Zone` — **Flip Zone** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Psychological Levels` — **Psychological Levels** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Round Numbers` — **Round Numbers** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Pivot Points` — **Pivot Points** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Multi-Timeframe Support / Resistance` — **Multi-Timeframe Support / Resistance** (group: Levels & Channels; implementation status: Catalog only)
- `Support, Resistance & Trend::Levels & Channels::Confluence Zone` — **Confluence Zone** (group: Levels & Channels; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-03-support-resistance-trend` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.