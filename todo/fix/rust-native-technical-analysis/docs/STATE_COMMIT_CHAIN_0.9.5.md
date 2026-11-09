# Durable State Commit Chain — v0.9.5

## Purpose

The SQLite state snapshot and embedded audit events already pass through the native state-decision layer. v0.9.5 adds a separate, append-only receipt for each accepted durable state commit. This makes ordinary payload corruption, accidental ledger edits, missing rows, reordered rows, and mismatched receipt hashes detectable before another state transition is accepted.

## Transaction contract

Each production `TradingCore::persist` call performs the following inside one SQLite transaction:

1. Read the exact predecessor payload from `app_state`.
2. Evaluate the candidate using the native state-decision policy.
3. Verify every existing receipt: sequence starts at 1 and is contiguous, the first previous hash is all zeroes, every later previous hash equals the prior receipt hash, and each record hash recomputes correctly.
4. Verify the chain head's state digest matches the exact predecessor payload bytes.
5. For a legacy schema-v1 database without receipts, append one `LEGACY_BASELINE_UNVERIFIED` receipt for the predecessor payload. This marks the start of verification; it does **not** assert that the historical database was untampered before this point.
6. Write the candidate state, append its receipt, and append audit rows.
7. Commit the transaction. A failure in any write rolls the transaction back.

The receipt digest is SHA-256 over a fixed-order, newline-delimited representation of sequence, commit timestamp, transition name, state-payload SHA-256, and previous receipt hash. The state-payload digest covers the exact serialized payload stored in SQLite.

## Startup behavior

If a database already has a commit chain, startup verifies the chain and payload binding before the general corrupt-state recovery path runs. A mismatch returns an error and refuses startup; it is not silently converted into a new paper account. Databases created before this feature have no chain table and continue through the existing compatibility path; the next accepted commit anchors their then-current predecessor state as explicitly unverified legacy history.

## Database enforcement

SQLite `BEFORE UPDATE` and `BEFORE DELETE` triggers reject ordinary modifications to receipt rows. The native writer only inserts new rows. The application continues to use the existing exclusive account-instance lock and full-synchronous WAL configuration.

## Threat model and limits

This is **tamper-evident**, not tamper-proof. It detects corruption or ordinary database edits while the established chain and triggers remain intact. A local administrator or attacker able to replace the entire database, remove triggers, rewrite all rows, and recompute every hash can forge a new local chain because the chain has no external cryptographic anchor or hardware-backed signing key. The database is not an externally notarized ledger. Backup retention, OS access control, and endpoint security remain required.

## Validation

A Rust unit test covers append-only receipt triggers and valid-JSON payload tampering. The repository's Node source-contract test verifies that chain validation is wired before startup recovery and within the persistence transaction. The Rust test has not been executed in this workspace because `cargo`/`rustc` are unavailable; CI must run `cargo test --locked --manifest-path src-tauri/Cargo.toml --all-targets`, formatting, Clippy, and the desktop build before release certification.
