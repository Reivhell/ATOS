# Agent Knowledge Pack — Order Flow & Market Profile

Contract: Strict Master List v1.0.0. Category entries: **21**.

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

- Assess every catalog ID individually and verify the data source actually contains the required granularity. OHLCV bars cannot substitute for DOM, Level 2, time-and-sales, bid/ask footprint, or true volume-at-price data.
- Market Profile/TPO needs an explicit session definition, price-bin/tick size, and profile-building rule. Volume Profile requires the source volume semantics and allocation method; disclose if only bar-level approximation exists.
- Absorption, exhaustion, iceberg, and stacked-imbalance claims require time-stamped order-book/trade evidence and should be `unsupported` if that feed or a reviewed detector is absent. Do not infer hidden orders from candle shapes.
- When data entitlement, sequence integrity, or feed coverage is insufficient, use `insufficient_data`; strict handoff remains blocked. Never describe synthetic fixture data as live order flow.

## Complete category inventory

- `Order Flow & Market Profile::Auction & Order Flow::Market Profile` — **Market Profile** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::TPO (Time Price Opportunity)` — **TPO (Time Price Opportunity)** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::POC (Point of Control)` — **POC (Point of Control)** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::VAH (Value Area High)` — **VAH (Value Area High)** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::VAL (Value Area Low)` — **VAL (Value Area Low)** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::HVN (High Volume Node)` — **HVN (High Volume Node)** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::LVN (Low Volume Node)` — **LVN (Low Volume Node)** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Initial Balance` — **Initial Balance** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Footprint Chart` — **Footprint Chart** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Bid-Ask Imbalance` — **Bid-Ask Imbalance** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Stacked Imbalance` — **Stacked Imbalance** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::DOM (Depth of Market)` — **DOM (Depth of Market)** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Level 2 Order Book` — **Level 2 Order Book** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Time & Sales` — **Time & Sales** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Absorption` — **Absorption** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Exhaustion` — **Exhaustion** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Iceberg Orders` — **Iceberg Orders** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Auction Market Theory` — **Auction Market Theory** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Failed Auction` — **Failed Auction** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Market Acceptance` — **Market Acceptance** (group: Auction & Order Flow; implementation status: Catalog only)
- `Order Flow & Market Profile::Auction & Order Flow::Market Rejection` — **Market Rejection** (group: Auction & Order Flow; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-05-order-flow-market-profile` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.