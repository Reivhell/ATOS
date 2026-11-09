# Strict work-plan durability — v0.4.1

## Why this change matters

Earlier replay protection was held only in a process-local mutex. It rejected duplicate completion attempts while one process stayed alive, but the consumed marker disappeared on restart. In v0.4.1, production Tauri commands use a dedicated SQLite database adjacent to the local app database. The in-memory implementation remains only for unit-test/parity helper calls.

## Durable schema

`strict_work_plans` stores:

- `work_plan_id`: UUID primary key
- `issued_at_ms`: timestamp used for the 15-minute TTL
- `plan_json`: exact serialized Rust-issued plan
- `plan_sha256`: SHA-256 integrity digest of that serialized JSON
- `consumed_at_ms`: NULL until the one allowed completion attempt begins

The store also records every claimed completion attempt in `strict_analysis_attempts`, then finalizes that row with the decision, audit receipt digest, the complete submitted per-concept assessment bundle, and the serialized result. The store uses WAL journaling, `synchronous=FULL`, a bounded SQLite busy timeout, and an explicit schema version. An unknown newer schema or a store/open/transaction error is a hard error; callers receive no authorization payload.

## Atomicity and lifecycle

1. Plan creation validates symbol, supported timeframe, timestamp, fingerprint, and the embedded 364-entry catalog.
2. The durable registry is opened and stale records are expired inside an `IMMEDIATE` transaction.
3. The pending-plan capacity check and insert happen under that same write transaction.
4. At completion, the Rust-supplied plan must serialize byte-for-byte to the issued JSON and its SHA-256 must match the stored digest.
5. A write transaction checks expiry and consumption, then conditionally sets `consumed_at_ms` while the marker is still NULL.
6. The consumption transaction commits before evaluation continues and creates a durable `in_progress` attempt marker. A blocked analysis is still a consumed attempt; retries require a freshly issued plan.
7. The completed assessment bundle, decision, result and receipt digest are finalized atomically in the audit table before the native command returns any result. If finalization fails, the command returns an error and no handoff payload. A process crash leaves an inspectable `in_progress` record rather than silently losing the fact that the plan was consumed.

The persistent tests cover replay across fresh SQLite connections, tampering that must not burn the original plan, consumption of a plan when the assessment gate is blocked, and durable retention of the submitted assessments and blocked result. The existing strict gate remains unchanged: every concept needs a unique valid assessment; 357 `Catalog only` entries are still unsupported and block handoff.

## Security boundary and limitations

The SHA-256 field protects against accidental corruption and participates in exact request comparison; it is **not** a digital signature or keyed MAC against an attacker with write access to the local database. OS-level file permissions and device security remain relevant. The separate store currently shares the local app's single-instance lifecycle, but cross-process SQLite transaction safety is independently enforced by SQLite's writer locking. Recovery/backup and disk-full fault-injection tests must run in the Rust CI/release environment.

This change does not dispatch nine AI agents, make all 364 concepts computable, promote catalog items, authorize live trading, or certify production release. Dependency lockfiles, Rust compilation/tests/Clippy, frontend production build, and end-to-end validation remain release blockers.
