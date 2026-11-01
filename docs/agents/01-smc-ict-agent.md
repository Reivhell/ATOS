# Agent Knowledge Pack — SMC & ICT

Contract: Strict Master List v1.0.0. Category entries: **97**.

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

- Assess every assigned catalog ID independently; produce exactly one `strict-ta-assessment.v1` record per ID. Never replace item-level results with a narrative summary.
- Confirmed structure must use only completed candles and record the time a pivot became knowable. Distinguish observed levels from hypotheses about institutional intent; OHLCV alone cannot prove that an order block, stop hunt, or iceberg order was placed by a particular participant.
- Session concepts require an explicit timezone, venue/session calendar, daylight-saving rule where relevant, and the exact session boundary. Do not infer London/New York session labels from local machine time.
- Sweep/raid and FVG/block claims require deterministic definitions, source candle references, mitigation/fill state, and no-look-ahead evidence. If the detector is not reviewed and registered, set `unsupported`.
- Do not promote any catalog item based on chart intuition. Current `Catalog only` concepts must remain `unsupported`, with a concrete reason.

## Complete category inventory

- `SMC & ICT::Market Structure::BOS (Break of Structure)` — **BOS (Break of Structure)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::CHoCH (Change of Character)` — **CHoCH (Change of Character)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::MSS (Market Structure Shift)` — **MSS (Market Structure Shift)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::MSB (Market Structure Break)` — **MSB (Market Structure Break)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::CISD (Change in State of Delivery)` — **CISD (Change in State of Delivery)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::HH (Higher High)` — **HH (Higher High)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::HL (Higher Low)` — **HL (Higher Low)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::LH (Lower High)` — **LH (Lower High)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::LL (Lower Low)` — **LL (Lower Low)** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::Internal Structure` — **Internal Structure** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::External Structure` — **External Structure** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::Protected High` — **Protected High** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::Protected Low` — **Protected Low** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::Swing High` — **Swing High** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::Swing Low` — **Swing Low** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Market Structure::Dealing Range` — **Dealing Range** (group: Market Structure; implementation status: Catalog only)
- `SMC & ICT::Liquidity::BSL (Buy-Side Liquidity)` — **BSL (Buy-Side Liquidity)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::SSL (Sell-Side Liquidity)` — **SSL (Sell-Side Liquidity)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::EQH (Equal Highs)` — **EQH (Equal Highs)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::EQL (Equal Lows)` — **EQL (Equal Lows)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::Liquidity Sweep` — **Liquidity Sweep** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::Liquidity Grab` — **Liquidity Grab** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::Liquidity Raid` — **Liquidity Raid** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::Stop Hunt` — **Stop Hunt** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::Stop Run` — **Stop Run** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::Inducement (IDM)` — **Inducement (IDM)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::IRL (Internal Range Liquidity)` — **IRL (Internal Range Liquidity)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::ERL (External Range Liquidity)` — **ERL (External Range Liquidity)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::DOL (Draw on Liquidity)` — **DOL (Draw on Liquidity)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::Trendline Liquidity` — **Trendline Liquidity** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::Session Liquidity` — **Session Liquidity** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::PDH (Previous Day High)` — **PDH (Previous Day High)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::PDL (Previous Day Low)` — **PDL (Previous Day Low)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::PWH (Previous Week High)` — **PWH (Previous Week High)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::PWL (Previous Week Low)` — **PWL (Previous Week Low)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::PMH (Previous Month High)` — **PMH (Previous Month High)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Liquidity::PML (Previous Month Low)` — **PML (Previous Month Low)** (group: Liquidity; implementation status: Catalog only)
- `SMC & ICT::Blocks::OB (Order Block)` — **OB (Order Block)** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Blocks::BB (Breaker Block)` — **BB (Breaker Block)** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Blocks::MB (Mitigation Block)` — **MB (Mitigation Block)** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Blocks::RB (Rejection Block)` — **RB (Rejection Block)** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Blocks::Propulsion Block` — **Propulsion Block** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Blocks::Bullish Order Block` — **Bullish Order Block** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Blocks::Bearish Order Block` — **Bearish Order Block** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Blocks::Internal Order Block` — **Internal Order Block** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Blocks::External Order Block` — **External Order Block** (group: Blocks; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::FVG (Fair Value Gap)` — **FVG (Fair Value Gap)** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::IFVG (Inverse Fair Value Gap)` — **IFVG (Inverse Fair Value Gap)** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::BPR (Balanced Price Range)` — **BPR (Balanced Price Range)** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::VI (Volume Imbalance)` — **VI (Volume Imbalance)** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Liquidity Void` — **Liquidity Void** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Displacement` — **Displacement** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Inefficiency` — **Inefficiency** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Consequent Encroachment (CE)` — **Consequent Encroachment (CE)** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Rebalancing` — **Rebalancing** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Repricing` — **Repricing** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Partial Fill` — **Partial Fill** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Full Fill` — **Full Fill** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Imbalance & Price Delivery::Reclaimed FVG` — **Reclaimed FVG** (group: Imbalance & Price Delivery; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::Premium Zone` — **Premium Zone** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::Discount Zone` — **Discount Zone** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::Equilibrium (EQ)` — **Equilibrium (EQ)** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::OTE (Optimal Trade Entry)` — **OTE (Optimal Trade Entry)** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::Fibonacci Retracement` — **Fibonacci Retracement** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::Fibonacci Extension` — **Fibonacci Extension** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::Fibonacci Projection` — **Fibonacci Projection** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::Mean Threshold` — **Mean Threshold** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Premium, Discount & Fibonacci::Midpoint` — **Midpoint** (group: Premium, Discount & Fibonacci; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Asian Range` — **Asian Range** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::London Kill Zone` — **London Kill Zone** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::New York Kill Zone` — **New York Kill Zone** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Judas Swing` — **Judas Swing** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Power of Three (PO3 / AMD)` — **Power of Three (PO3 / AMD)** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Silver Bullet` — **Silver Bullet** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::ICT Macros` — **ICT Macros** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Daily Open` — **Daily Open** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Weekly Open` — **Weekly Open** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Monthly Open` — **Monthly Open** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::True Day Open` — **True Day Open** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Midnight Open` — **Midnight Open** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::NDOG (New Day Opening Gap)` — **NDOG (New Day Opening Gap)** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::NWOG (New Week Opening Gap)` — **NWOG (New Week Opening Gap)** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Session High` — **Session High** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Session Low` — **Session Low** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Opening Range` — **Opening Range** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::CBDR (Central Bank Dealers Range)` — **CBDR (Central Bank Dealers Range)** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::MMBM (Market Maker Buy Model)` — **MMBM (Market Maker Buy Model)** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::MMSM (Market Maker Sell Model)` — **MMSM (Market Maker Sell Model)** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Time & Session::Quarterly Theory` — **Quarterly Theory** (group: Time & Session; implementation status: Catalog only)
- `SMC & ICT::Model & Setup ICT::Unicorn Model` — **Unicorn Model** (group: Model & Setup ICT; implementation status: Catalog only)
- `SMC & ICT::Model & Setup ICT::SMT Divergence` — **SMT Divergence** (group: Model & Setup ICT; implementation status: Catalog only)
- `SMC & ICT::Model & Setup ICT::Turtle Soup` — **Turtle Soup** (group: Model & Setup ICT; implementation status: Catalog only)
- `SMC & ICT::Model & Setup ICT::Candle Range Theory (CRT)` — **Candle Range Theory (CRT)** (group: Model & Setup ICT; implementation status: Catalog only)
- `SMC & ICT::Model & Setup ICT::Market Maker Buy Model` — **Market Maker Buy Model** (group: Model & Setup ICT; implementation status: Catalog only)
- `SMC & ICT::Model & Setup ICT::Market Maker Sell Model` — **Market Maker Sell Model** (group: Model & Setup ICT; implementation status: Catalog only)
- `SMC & ICT::Model & Setup ICT::Daily Bias` — **Daily Bias** (group: Model & Setup ICT; implementation status: Catalog only)
- `SMC & ICT::Model & Setup ICT::Weekly Bias` — **Weekly Bias** (group: Model & Setup ICT; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-01-smc-ict` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.
## Research detector implementation boundary (v0.2.2)

The deterministic module `src/lib/research-detectors.js` currently provides confirmed swing pivots, HH/HL/LH/LL labels, and close-based BOS/CHoCH research events. Pivots are timestamped at `confirmedAt`, not at the earlier extreme, and break events can only use pivots after confirmation. These outputs are research-only and are **not** promoted to `Calculated now` in the authoritative 364-entry registry; independent review, broader fixtures, and gate integration are required first. This module does not implement MSS, MSB, CISD, protected swings, liquidity, blocks, FVG, or the rest of the SMC/ICT catalog.

## Additional native research capability (0.4.0)

The Rust research module includes `build_structure_state_timeline()` for point-in-time snapshots over confirmed swings and close-confirmed breaks. Treat `active_swing_high/low`, `bias`, `transition`, and `protected_high/low` as auditable research observations only. A same-candle bullish/bearish conflict is explicitly ambiguous. Do not relabel this output as BOS, CHoCH, MSS, MSB, CISD, internal/external structure, or a trading signal. The 0.4.0 implementation has not been promoted into the strict catalog registry; its associated catalog entries remain `Catalog only` until native compilation, fixture validation, and independent review pass.
