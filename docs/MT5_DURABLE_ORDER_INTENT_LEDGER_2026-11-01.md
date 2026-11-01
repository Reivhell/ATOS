# MT5 durable order-intent ledger — implementation slice

**Status:** persistence primitive implemented on the engineering branch; not wired to broker submission and not production-approved.

## Scope

- Persist an account-scoped order intent in SQLite before any future dispatch.
- Enforce idempotency on broker server, account login, and caller-supplied idempotency key.
- Reject reuse of an idempotency key with a different payload hash.
- Store state transitions and audit events in the same SQLite transaction.
- Recover prepared, dispatching, and unknown intents after restart for reconciliation.
- Require expected-state transitions so stale callers cannot advance an intent twice.
- Keep the module isolated from the existing read-only MT5 bridge. It grants no order execution capability.

## State model

`prepared -> dispatching -> accepted | rejected | unknown`

`unknown -> accepted | rejected | reconciled`

`accepted -> reconciled`

A timeout after sending is ambiguous. It must move to `unknown`; it must not be automatically retried. The caller must reconcile broker orders/deals and account identity before closing the ambiguity. This module does not implement broker reconciliation, `OrderCheck`, a dispatch outbox worker, or the execution controller.

## Acceptance gates

- [x] SQLite schema and transaction-backed persistence primitive
- [x] Account-scoped idempotency and payload fingerprint
- [x] Explicit legal transition graph and append-only transition events
- [x] Query unresolved intents for startup recovery
- [x] Unit tests authored for duplicate intent, payload mismatch, ambiguous timeout, expected-state guard, and invalid input
- [ ] Rust tests and clippy executed against current `dev` toolchain
- [ ] Wire ledger into an execution controller without exposing a direct UI bypass
- [ ] Broker-side `OrderCheck`, ticket/deal reconciliation, and restart/fault-injection tests
- [ ] Demo-account acceptance and operator kill-switch validation

Live order submission must remain disabled until all remaining gates pass.
