# Strict-analysis durable audit chain — v0.4.2

## State transitions

The native SQLite store appends an ordered chain event in the same `BEGIN IMMEDIATE` transaction as each state transition:

1. `work_plan_issued` binds the plan ID, issue timestamp, and SHA-256 of the serialized Rust-issued plan.
2. `work_plan_consumed` binds the plan digest to the single-use consume timestamp.
3. `assessment_attempt_started` records that completion began; the plan is consumed even when the eventual decision is `BLOCK`.
4. `assessment_attempt_finalized` binds the serialized assessment bundle, final result, receipt digest, status, decision, and completion timestamp through a payload digest.

Each event includes an increasing sequence number and previous-event hash. A separate `strict_audit_chain_head` row stores the final sequence and digest. A successful store open verifies chain continuity, every event digest, the stored head, payload hashes, and semantic links between events and work-plan/attempt rows. Failure to verify makes the persistence operation return an error; no downstream payload is returned.

## Migration

Schema-v1 work-plan and attempt rows are imported transactionally before the store advances to schema v2, then schema v2-to-v3 migration appends the specialist-receipt migration event. Rows that still exist receive issuance/consume/start/finalize events. If an earlier build had already pruned a plan row while leaving its attempt, the attempt is preserved and bound by a `legacy_orphan_attempt_imported` event. That legacy gap is explicitly recorded rather than misrepresented as a complete original lifecycle.

## Retention and capacity

The 15-minute work-plan TTL controls whether a plan is eligible for consumption. The 256-plan bound counts only currently pending, non-expired plans. Historical plan/attempt rows and chain events are not deleted during routine issuance; this preserves chain semantics but means the store grows over time. Production operations still need a reviewed archival/checkpoint policy before sustained long-running use.

## Security limits

The chain detects accidental or partial edits, event deletion/reordering, broken hashes, inconsistent row updates, and ordinary tail truncation because the stored head no longer matches. SHA-256 alone is not authentication: an actor with write access to the complete database can rewrite the event chain and head together. Strong protection against that attacker requires an independently protected signature key or remote/WORM anchor, which is not implemented here.

## Strict trading policy remains unchanged

The Rust code-owned catalog still contains exactly 364 concepts in nine categories. Current availability remains 6 calculated, 1 workflow, and 357 catalog-only concepts. Unsupported, missing, malformed, insufficient-data, or unreviewed concepts block handoff. Knowledge packs are not proof that specialist runtime dispatch occurred. Live trading is not authorized.


## Specialist dispatch receipts (schema v3)

Schema v3 adds a durable append-only table for specialist dispatch receipts. Each receipt is cross-linked to a `specialist_receipt_recorded` event in this hash chain. Audit-chain verification recounts receipt rows and events, recomputes each receipt digest, reconstructs the stored canonical work plan, and recomputes task-input SHA-256 from the embedded knowledge pack, trusted category IDs, specialist route, dataset scope, and contract versions. Once an attempt is finalized, the verifier also recomputes each category output digest from the persisted assessment bundle and compares it to the dispatch receipt. The native tests include bypassing the normal row trigger and verifying that a semantic receipt mutation is still rejected. These Rust tests are authored but have not been executed in this environment because `cargo`/`rustc` are unavailable. The strict gate requires nine category/agent receipts for the current plan; the current provider dispatcher is unconfigured, so missing receipts intentionally force `BLOCK`. See [`STRICT_SPECIALIST_DISPATCH_RECEIPTS_0.4.2.md`](STRICT_SPECIALIST_DISPATCH_RECEIPTS_0.4.2.md).
