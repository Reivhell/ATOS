# Decision provenance graph — 0.8.5

## What changed

The research decision contract now returns a deterministic, read-only provenance artifact alongside the verdict. The artifact includes:

- A market-snapshot root bound to the supplied dataset fingerprint and snapshot scope.
- One node per accepted signal domain and one node per unique evidence reference, with explicit support/evaluation edges. Evidence node IDs are namespaced and URI-encoded to prevent reference strings from colliding with internal node IDs.
- Per-domain configured weight and signed score contribution, so the aggregate score can be independently reconstructed.
- Explicit status/details for prerequisite, signal-integrity, coverage, strength, agreement, and score-separation gates.
- Integrity flags for cross-domain evidence reuse and input-bound violations.

## Safety and scope

This is an explainability/provenance graph, not a cryptographic attestation. Evidence references are still identifiers supplied by upstream analysis; this layer cannot prove that they refer to authentic market observations, that two semantically different identifiers are independent, or that the upstream feature calculation is correct. The `graphIsDiagnosticNotCryptographicAttestation` marker is deliberately explicit. A future trusted evidence registry must resolve references to canonical records with content digests and derivation lineage before stronger integrity claims are justified.

The graph contains only accepted signal records. Rejected signals remain represented in gate details and top-level reasons; they do not become score contributors. Output stays deterministic for identical inputs. No timestamps are generated at decision time, no order intent is emitted, execution authority remains `none`, and live execution remains disabled.

## Verification requirements

The tests assert graph shape, score-contribution reconstruction fields, stable output for identical inputs, explicit failure traces, evidence-reference reuse blocking, and the continuing no-order/no-live-execution invariants. These tests do not establish exchange integration, production performance, or correctness of upstream evidence generation.
