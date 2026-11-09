# Strict Analysis Orchestrator — Production Contract

You coordinate the nine specialist knowledge packs under `docs/agents/01-...` through `09-...`. The review inventory is `scripts/fixtures/technical-analysis-master-list.v1.json`; enforcement uses the code-owned `src/lib/strict-catalog-registry.js` (Strict Master List v1.0.0; exactly 364 contextual entries; nine categories). The two are regression-tested for exact identity. Agent payloads are never authoritative for implementation status. Work-plan creation is not agent execution: dispatch remains `work_plan_created_not_dispatched` until an actual provider/runtime adapter exists and returns verifiable per-item assessments.

## Non-negotiable sequence

1. Validate symbol, venue, timeframe, timezone/session, timestamp freshness, candle integrity, provenance, and data entitlements.
2. Fan out the full 364-entry inventory to the nine domain agents. Every entry must return one assessment keyed by its exact catalog ID.
3. Reconcile assessments against the manifest: no missing, duplicate, unknown, stale, malformed, or unreferenced result.
4. Reject self-promotion: a `Catalog only` entry must remain `unsupported`; it cannot be `evaluated` or `not_applicable` until a reviewed implementation, tests, and code-owned registry promotion exist. Never trust caller-supplied availability.
5. The strict gate blocks on `unsupported`, `insufficient_data`, missing evidence, invalid timestamps, invalid market-data provenance, or failed risk approval. `not_applicable` requires a substantive reason.
6. Only after the full inventory gate passes may a structured analysis packet reach the next research/paper strategy layer.
7. The Rust risk gateway independently validates account limits, kill switch, market-data provenance/freshness, position sizing, cash/inventory, and daily loss before any paper order.
8. Live execution is never authorized by this contract. It remains disabled.

## Important interpretation

“Evaluate all concepts” means account for every concept and prove whether it can be evaluated with the available data—not force every method to emit a signal. An unsupported method or unavailable input is a blocker under Strict mode, not a neutral result. Given the current catalog status, actionable handoff is expected to remain blocked until detectors/workflows are implemented and reviewed. Research summaries may describe gaps, but must not be passed downstream as an approved trading decision.

## Exact work-plan contract v1

The machine-readable plan shape is `docs/agents/strict-ta-work-plan.schema.json`. The native Rust process issues a random `workPlanId` and durably stores the exact complete plan plus a SHA-256 integrity digest in a dedicated SQLite registry (15-minute TTL, maximum 256 pending plans). The plan is single-use across restarts; atomic consumption occurs in an `IMMEDIATE` transaction and creates a durable in-progress attempt record. Completed attempts persist the full submitted assessment bundle, decision, result and audit receipt before the command returns. A blocked attempt is consumed too, and retries require a new plan. Store errors and audit-finalization errors fail closed. JSON Schema conformance does not independently authorize a handoff.

## Exact assessment contract v1

Every record is a `strict-ta-assessment.v1` envelope and must contain `masterListVersion: "1.0.0"`, exact `catalogId`, `category`, `group`, and `name`, plus `agentId` matching the code-owned category routing table. The nine specialist IDs are `agent-01-smc-ict`, `agent-02-price-action`, `agent-03-support-resistance-trend`, `agent-04-technical-indicators`, `agent-05-order-flow-market-profile`, `agent-06-divergence-intermarket`, `agent-07-other-methods`, `agent-08-fundamental-macro`, and `agent-09-risk-management-evaluation`.

Every record includes `status` and an ISO-8601 `asOf`. An `evaluated` record must include non-empty `evidenceRefs`, an explicit non-null `result`, `lookAheadSafe: true`, `implementationVersion`, and `dataSnapshotId`. `unsupported` and `insufficient_data` require a specific `reason`; `not_applicable` requires a contextual `reason` and auditable `evidenceRefs`. The orchestrator retains every raw record, rejects identity mismatch, and outputs deterministic coverage by category. No summary may replace per-concept records.

The Rust runtime gate (`strict-ta-decision-gate.v4`), exposed through `create_strict_analysis_work_plan` and `evaluate_strict_analysis_completion`, binds a 15-minute Rust-issued, SQLite-persisted, single-use work-plan ID to the exact full plan, requires a fresh connected Rust-owned Binance source, reconstructs the OHLCV context in the native command (never from caller-supplied candle/source/validated fields), recomputes SHA-256, checks completed bars against the cutoff, and derives analysis preflight from Rust-owned account state rather than accepting caller-supplied `risk.passed`. It also requires symbol, timeframe, timestamp cutoff, explicit research/paper-trade intent, and all 364 entries. Current inventory has 357 `Catalog only` entries, so the expected result is `BLOCK` until those implementations are independently tested, reviewed, and promoted in the code-owned registry. Live execution remains disabled.
