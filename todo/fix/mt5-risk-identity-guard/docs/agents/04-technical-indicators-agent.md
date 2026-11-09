# Agent Knowledge Pack — Indikator Teknikal

Contract: Strict Master List v1.0.0. Category entries: **52**.

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

- Assess all 52 indicator catalog records individually. Return source series, formula/parameter version, warm-up requirements, and the latest input candle timestamp for every evaluated value.
- Only the exact reviewed calculations currently marked `Calculated now` may be marked `evaluated` when the required history and validation pass: SMA, EMA, RSI, MACD, ATR, and Bollinger Bands. The status is not permission to mark related variants (WMA, VWMA, ADX, CCI, etc.) as calculated.
- Never fill missing history with zeros, fixtures, another symbol, or another timeframe. Use `insufficient_data` for missing/short/invalid input; this blocks the strict handoff. Use `unsupported` when the formula/detector itself has not been implemented and reviewed.
- Indicator values are features, not standalone buy/sell instructions. Record all parameters, code/implementation version, snapshot SHA-256, and look-ahead-safe calculation boundaries.

## Complete category inventory

- `Indikator Teknikal::Trend::SMA (Simple Moving Average)` — **SMA (Simple Moving Average)** (group: Trend; implementation status: Calculated now)
- `Indikator Teknikal::Trend::EMA (Exponential Moving Average)` — **EMA (Exponential Moving Average)** (group: Trend; implementation status: Calculated now)
- `Indikator Teknikal::Trend::WMA (Weighted Moving Average)` — **WMA (Weighted Moving Average)** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::VWMA (Volume-Weighted Moving Average)` — **VWMA (Volume-Weighted Moving Average)** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::DEMA (Double Exponential Moving Average)` — **DEMA (Double Exponential Moving Average)** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::TEMA (Triple Exponential Moving Average)` — **TEMA (Triple Exponential Moving Average)** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::HMA (Hull Moving Average)` — **HMA (Hull Moving Average)** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::Supertrend` — **Supertrend** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::Ichimoku Cloud` — **Ichimoku Cloud** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::Parabolic SAR` — **Parabolic SAR** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::ADX (Average Directional Index)` — **ADX (Average Directional Index)** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Trend::Aroon` — **Aroon** (group: Trend; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::RSI (Relative Strength Index)` — **RSI (Relative Strength Index)** (group: Momentum; implementation status: Calculated now)
- `Indikator Teknikal::Momentum::Stochastic Oscillator` — **Stochastic Oscillator** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::Stochastic RSI` — **Stochastic RSI** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::MACD (Moving Average Convergence Divergence)` — **MACD (Moving Average Convergence Divergence)** (group: Momentum; implementation status: Calculated now)
- `Indikator Teknikal::Momentum::CCI (Commodity Channel Index)` — **CCI (Commodity Channel Index)** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::Williams %R` — **Williams %R** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::ROC (Rate of Change)` — **ROC (Rate of Change)** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::TRIX` — **TRIX** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::TSI (True Strength Index)` — **TSI (True Strength Index)** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::Ultimate Oscillator` — **Ultimate Oscillator** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::Awesome Oscillator` — **Awesome Oscillator** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::PPO (Percentage Price Oscillator)` — **PPO (Percentage Price Oscillator)** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::RVI (Relative Vigor Index)` — **RVI (Relative Vigor Index)** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::Fisher Transform` — **Fisher Transform** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::DPO (Detrended Price Oscillator)` — **DPO (Detrended Price Oscillator)** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Momentum::KST (Know Sure Thing)` — **KST (Know Sure Thing)** (group: Momentum; implementation status: Catalog only)
- `Indikator Teknikal::Volatility::ATR (Average True Range)` — **ATR (Average True Range)** (group: Volatility; implementation status: Calculated now)
- `Indikator Teknikal::Volatility::Bollinger Bands` — **Bollinger Bands** (group: Volatility; implementation status: Calculated now)
- `Indikator Teknikal::Volatility::Keltner Channels` — **Keltner Channels** (group: Volatility; implementation status: Catalog only)
- `Indikator Teknikal::Volatility::Donchian Channels` — **Donchian Channels** (group: Volatility; implementation status: Catalog only)
- `Indikator Teknikal::Volatility::Standard Deviation` — **Standard Deviation** (group: Volatility; implementation status: Catalog only)
- `Indikator Teknikal::Volatility::Historical Volatility` — **Historical Volatility** (group: Volatility; implementation status: Catalog only)
- `Indikator Teknikal::Volatility::Chaikin Volatility` — **Chaikin Volatility** (group: Volatility; implementation status: Catalog only)
- `Indikator Teknikal::Volatility::ADR (Average Daily Range)` — **ADR (Average Daily Range)** (group: Volatility; implementation status: Catalog only)
- `Indikator Teknikal::Volatility::AWR (Average Weekly Range)` — **AWR (Average Weekly Range)** (group: Volatility; implementation status: Catalog only)
- `Indikator Teknikal::Volatility::Implied Volatility` — **Implied Volatility** (group: Volatility; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Volume Bars` — **Volume Bars** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Volume Profile` — **Volume Profile** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Fixed Range Volume Profile` — **Fixed Range Volume Profile** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Visible Range Volume Profile` — **Visible Range Volume Profile** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Session Volume Profile` — **Session Volume Profile** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::VWAP (Volume-Weighted Average Price)` — **VWAP (Volume-Weighted Average Price)** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Anchored VWAP` — **Anchored VWAP** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::OBV (On-Balance Volume)` — **OBV (On-Balance Volume)** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::MFI (Money Flow Index)` — **MFI (Money Flow Index)** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::CMF (Chaikin Money Flow)` — **CMF (Chaikin Money Flow)** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Accumulation / Distribution Line` — **Accumulation / Distribution Line** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Volume Oscillator` — **Volume Oscillator** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Volume Delta` — **Volume Delta** (group: Volume; implementation status: Catalog only)
- `Indikator Teknikal::Volume::Cumulative Volume Delta (CVD)` — **Cumulative Volume Delta (CVD)** (group: Volume; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-04-technical-indicators` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.