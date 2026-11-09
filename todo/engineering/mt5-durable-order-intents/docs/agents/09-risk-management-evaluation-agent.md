# Agent Knowledge Pack — Risk Management & Evaluation

Contract: Strict Master List v1.0.0. Category entries: **31**.

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

- Assess every risk/execution/evaluation catalog ID independently. No strategy handoff is permitted without the full 364-item gate and a separately approved Rust risk gateway.
- Entry/exit prices, stop distance, risk per trade, order type, position sizing, daily loss, fees, slippage, swap, latency, and available cash/inventory must use explicit instrument specifications and timestamped account/data snapshots.
- Backtest, forward, out-of-sample, walk-forward, and Monte Carlo claims must document code version, dataset fingerprint, splits, parameter-selection boundary, transaction-cost assumptions, and reproducible result IDs. In-sample performance is not validation.
- Risk formulas must use finite values and reject invalid/zero denominators; apply daily-loss and kill-switch rules independently of strategy direction. A strategy agent cannot approve its own risk gate.
- `Workflow available` for Backtesting means the existing SMA-crossover backtest workflow exists; it does not mean every evaluation method is implemented or its evidence can be assumed. Use `unsupported` unless the assigned method itself has a reviewed result.

## Complete category inventory

- `Risk Management & Evaluation::Execution & Position Risk::Entry Trigger` — **Entry Trigger** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Confirmation Entry` — **Confirmation Entry** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Market Order` — **Market Order** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Limit Order` — **Limit Order** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Stop Order` — **Stop Order** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Stop Loss (SL)` — **Stop Loss (SL)** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Take Profit (TP)` — **Take Profit (TP)** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Risk-to-Reward Ratio (RR)` — **Risk-to-Reward Ratio (RR)** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Position Sizing` — **Position Sizing** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Fixed Fractional Risk` — **Fixed Fractional Risk** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Break-Even (BE)` — **Break-Even (BE)** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Trailing Stop` — **Trailing Stop** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Partial Close` — **Partial Close** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Scaling In` — **Scaling In** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Scaling Out` — **Scaling Out** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Maximum Daily Loss` — **Maximum Daily Loss** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Risk of Ruin` — **Risk of Ruin** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Spread` — **Spread** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Commission` — **Commission** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Swap` — **Swap** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Execution & Position Risk::Execution Latency` — **Execution Latency** (group: Execution & Position Risk; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Expectancy` — **Expectancy** (group: Strategy Validation & Journal; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Win Rate` — **Win Rate** (group: Strategy Validation & Journal; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Profit Factor` — **Profit Factor** (group: Strategy Validation & Journal; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Maximum Drawdown` — **Maximum Drawdown** (group: Strategy Validation & Journal; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Backtesting` — **Backtesting** (group: Strategy Validation & Journal; implementation status: Workflow available)
- `Risk Management & Evaluation::Strategy Validation & Journal::Forward Testing` — **Forward Testing** (group: Strategy Validation & Journal; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Out-of-Sample Testing` — **Out-of-Sample Testing** (group: Strategy Validation & Journal; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Walk-Forward Analysis` — **Walk-Forward Analysis** (group: Strategy Validation & Journal; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Monte Carlo Simulation` — **Monte Carlo Simulation** (group: Strategy Validation & Journal; implementation status: Catalog only)
- `Risk Management & Evaluation::Strategy Validation & Journal::Trading Journal` — **Trading Journal** (group: Strategy Validation & Journal; implementation status: Catalog only)

## Required assessment output — strict v1 contract

Emit exactly one assessment for every inventory ID in this file and no assessment for another specialist's category. Each record must contain all of these fields:

- `contractVersion`: `strict-ta-assessment.v1`
- `masterListVersion`: `1.0.0`
- `catalogId`, `category`, `group`, and `name`: exact copies from the inventory entry; never normalize or paraphrase them
- `agentId`: `agent-09-risk-management-evaluation` (fixed routing identity for this knowledge pack)
- `status`: exactly one of `evaluated`, `insufficient_data`, `not_applicable`, `unsupported`
- `asOf`: ISO-8601 timestamp with timezone
- For `evaluated`: non-empty `evidenceRefs`, explicit non-null `result`, `lookAheadSafe: true`, a reviewed `implementationVersion`, and `dataSnapshotId` tied to the input-data fingerprint
- For `insufficient_data` and `unsupported`: a concrete `reason`; state the missing input or missing reviewed implementation, not a generic error
- For `not_applicable`: a substantive context-specific `reason` plus non-empty auditable `evidenceRefs`

Never state `evaluated` solely because the concept is familiar or appears in this inventory. The code-owned implementation status is authoritative: `Catalog only` must be `unsupported` until deterministic implementation, tests, review, and registry promotion are complete. That state blocks all downstream handoffs. `insufficient_data` also blocks. Do not invent a neutral result to make coverage look complete.

The native Rust gate reconstructs the candle context from a fresh connected provider snapshot and derives the risk preflight from Rust-owned account state; never treat UI-supplied `validated`, source labels, candles, or `risk.passed` as authoritative. The gate additionally validates timestamp, symbol, timeframe, SHA-256 fingerprint, exact catalog identity, and zero missing/duplicate/unknown assessments. Every work plan is Rust-issued, durably registered in the production SQLite strict-analysis store, single-use, and expires after 15 minutes; modified, expired, or replayed plans block. The durable attempt ledger retains completion status, the submitted per-concept assessment bundle, decision, and audit receipt; if audit finalization fails, no completion result or handoff payload is returned. A candle must be fully closed by `asOf`. Every result remains research/paper-only; live execution is not authorized.