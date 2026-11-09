# Agent Knowledge Pack — Price Action

Contract: Strict Master List v1.0.0. Category entries: **67**.

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

- Assess every assigned catalog ID independently with one strict assessment record per ID.
- Patterns must use explicit numerical definitions (lookback, tolerance, wick/body proportions, pivot confirmation, invalidation, and completion time). Do not claim subjective visual resemblance is a confirmed detector output.
- Candlestick patterns require completed OHLC candles. Chart patterns require confirmed pivots and must not use future bars to claim earlier recognition; store both pattern start and confirmation timestamps.
- Regime labels must state their window, thresholds, and uncertainty. Do not equate a detected pattern with an expected profitable direction.
- If the deterministic detector has not been implemented, tested on positive/negative and malformed fixtures, and promoted in the trusted registry, use `unsupported`.

## Complete category inventory

- `Price Action::Market Conditions::Trending Market` — **Trending Market** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Ranging Market` — **Ranging Market** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Consolidation` — **Consolidation** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Accumulation` — **Accumulation** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Distribution` — **Distribution** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Expansion` — **Expansion** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Contraction` — **Contraction** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Impulse` — **Impulse** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Correction` — **Correction** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Retracement` — **Retracement** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Pullback` — **Pullback** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Reversal` — **Reversal** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Continuation` — **Continuation** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Breakout` — **Breakout** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Breakdown` — **Breakdown** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::False Breakout` — **False Breakout** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Breakout Retest` — **Breakout Retest** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Acceptance` — **Acceptance** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Market Conditions::Rejection` — **Rejection** (group: Market Conditions; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Doji` — **Doji** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Dragonfly Doji` — **Dragonfly Doji** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Gravestone Doji` — **Gravestone Doji** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Hammer` — **Hammer** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Inverted Hammer` — **Inverted Hammer** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Hanging Man` — **Hanging Man** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Shooting Star` — **Shooting Star** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Pin Bar` — **Pin Bar** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Marubozu` — **Marubozu** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Spinning Top` — **Spinning Top** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Bullish Engulfing` — **Bullish Engulfing** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Bearish Engulfing` — **Bearish Engulfing** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Harami` — **Harami** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Piercing Line` — **Piercing Line** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Dark Cloud Cover` — **Dark Cloud Cover** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Tweezer Top` — **Tweezer Top** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Tweezer Bottom` — **Tweezer Bottom** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Morning Star` — **Morning Star** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Evening Star` — **Evening Star** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Three White Soldiers` — **Three White Soldiers** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Three Black Crows` — **Three Black Crows** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Inside Bar` — **Inside Bar** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Outside Bar` — **Outside Bar** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Fakey Pattern` — **Fakey Pattern** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Rising Three Methods` — **Rising Three Methods** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Falling Three Methods` — **Falling Three Methods** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Candlestick Patterns::Three-Bar Play` — **Three-Bar Play** (group: Candlestick Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Head & Shoulders` — **Head & Shoulders** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Inverse Head & Shoulders` — **Inverse Head & Shoulders** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Double Top` — **Double Top** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Double Bottom` — **Double Bottom** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Triple Top` — **Triple Top** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Triple Bottom` — **Triple Bottom** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Rounding Top` — **Rounding Top** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Rounding Bottom` — **Rounding Bottom** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Rising Wedge` — **Rising Wedge** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Falling Wedge` — **Falling Wedge** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Ascending Triangle` — **Ascending Triangle** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Descending Triangle` — **Descending Triangle** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Symmetrical Triangle` — **Symmetrical Triangle** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Bull Flag` — **Bull Flag** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Bear Flag` — **Bear Flag** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Pennant` — **Pennant** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Rectangle` — **Rectangle** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Cup and Handle` — **Cup and Handle** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Broadening Formation` — **Broadening Formation** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Diamond Pattern` — **Diamond Pattern** (group: Chart Patterns; implementation status: Catalog only)
- `Price Action::Chart Patterns::Measured Move` — **Measured Move** (group: Chart Patterns; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-02-price-action` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.