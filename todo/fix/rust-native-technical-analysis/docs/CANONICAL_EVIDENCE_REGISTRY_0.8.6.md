# Canonical Evidence Registry — 0.8.6

## Purpose

The v0.8.5 provenance graph explained which evidence-reference strings supported a signal, but those strings were caller-controlled aliases. v0.8.6 adds a bounded, content-addressed registry so equivalent records converge on the same SHA-256 identity and derived evidence carries an explicit parent chain.

## Canonical record contract

Each descriptor requires a unique source-reference alias, source identifier, UTC observation timestamp, dataset SHA-256, instrument, timeframe, JSON payload, and optional parent aliases. The canonical content hashed with Web Crypto SHA-256 includes schema version, source, timestamp, dataset fingerprint, instrument/timeframe scope, normalized payload, and sorted canonical parent IDs. The external alias is deliberately excluded from the hash, so changing a vendor label alone does not create a new evidence identity.

The registry enforces a maximum of 512 descriptors, 16 parent references per descriptor, 16 KiB payload per record, 1 MiB serialized registry size, bounded nesting/arrays/object keys/string lengths, plain JSON values only, finite numbers, dense arrays, getter-free data properties, valid calendar timestamps, and acyclic derivation. Missing parents, cycles, invalid scope, unsupported payload values, or unavailable Web Crypto cause registration to fail rather than silently degrade.

Registry output and nested payloads are immutable. The decision engine only accepts registry objects created by this module in the current JavaScript process, checks that each signal reference resolves to a registered canonical ID, checks content digest and snapshot scope, and rejects canonical evidence reuse across domains. Serialized registries must be reconstructed and revalidated before use; a copied object does not retain in-process verification state.

## Guarantees and limitations

- Deterministic canonical IDs and registry digest for identical canonical records.
- Different source aliases with identical canonical content converge on the same ID.
- Parent derivation lineage is included in the hash, so changing ancestry changes derived identity.
- Registry creation is async because Web Crypto is async; the decision verdict remains synchronous and side-effect free.
- Hashes prove consistency of the canonicalized content presented to this registry, not truthfulness of the source, authenticity of an exchange response, or statistical independence of distinct payloads. The upstream ingestion boundary still must authenticate providers, validate raw candles, and issue a trustworthy dataset fingerprint.
- This is an in-process application guard, not a replacement for native Rust validation, durable signed receipts, secure key management, or an external audit log.

## Decision contract

The decision contract advances to `research-decision.v4`, provenance to `decision-provenance.v2`, and policy identity to `research-decision-policy.v3`. Missing registry, unknown evidence IDs, scope mismatch, or reused canonical evidence blocks the verdict. No order intent is emitted; execution authority remains `none` and live execution remains disabled.
