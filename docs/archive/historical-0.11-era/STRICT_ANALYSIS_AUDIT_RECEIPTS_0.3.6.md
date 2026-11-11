# Strict Analysis Audit Receipts — v0.3.6

## Purpose

The Rust completion boundary returns a digest-based receipt for every evaluation, including blocked decisions, so operators can identify the exact input bundle, catalog revision, snapshot, coverage state, and blocker set associated with a decision. The receipt does not authorize an otherwise blocked handoff.

## Digest definitions

- `assessmentBundleSha256`: SHA-256 of JSON serialization of the complete assessment list sorted by `catalogId`. Assessment struct field order follows the Rust serde schema. The original list order is normalized for distinct IDs; duplicate IDs remain in the serialization and are separately rejected by the gate.
- `categoryAssessmentSha256`: for each of the nine trusted categories, SHA-256 of the assessment subset whose IDs map to that category, sorted by `catalogId`. Unknown IDs do not map to a category but remain included in the overall digest.
- `reasonsSha256`: SHA-256 of the sorted, deduplicated blocker-reason vector produced by the native gate.
- `receiptSha256`: SHA-256 of the serialized `StrictAuditReceipt` with `receiptSha256` cleared to the empty string. Use the same schema and serializer to verify.

## Security boundaries

A hash detects content changes only when the expected hash is trusted. It is not a digital signature, does not provide non-repudiation, and does not prove an AI provider executed a task. Because this iteration does not add persistent signed storage, operators must preserve the receipt and independently obtained hash in a controlled audit sink if they need later verification.

## Fail-closed policy unchanged

A receipt is emitted for diagnostic and auditability even when a request is blocked. `nextLayerPayload` remains absent unless the existing native gate passes every strict condition. No agent may promote catalog status, and live execution remains disabled.
