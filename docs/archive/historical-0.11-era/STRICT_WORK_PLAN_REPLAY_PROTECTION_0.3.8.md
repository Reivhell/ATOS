# Strict work-plan replay protection — v0.3.8

## Contract

- A plan is created by the Rust process, has a TTL, and is stored in a bounded in-memory registry.
- The first exact, unmodified plan submitted to completion atomically transitions from `issued` to `consumed` under the registry mutex.
- Any later submission with the same ID is rejected.
- A modified plan does not consume the genuine issued plan.
- A blocked completion attempt still consumes the plan. The caller must request a new plan to retry, which avoids ambiguous partial retries against a mutable decision boundary.
- Unknown, expired, or already consumed plan IDs fail closed.

## Concurrency

Consumption and identity comparison occur while holding the same mutex, so two simultaneous submissions cannot both successfully consume the same in-process plan.

## Limitations and production requirements

The registry is process-local and volatile. Restarting the app resets it; multiple app instances do not share it. This implementation is a meaningful in-process replay defense, not a complete distributed replay defense. Production hardening requires a durable database with a unique plan ID and an atomic conditional update/transaction, retention/TTL policy, and operational recovery tests.
