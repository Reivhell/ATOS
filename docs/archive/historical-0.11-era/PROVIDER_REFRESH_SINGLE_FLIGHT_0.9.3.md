# Provider refresh single-flight contract — v0.9.3

## Failure mode

The earlier guard only enforced a minimum interval between refresh starts. Market-data requests have an 8-second request deadline but a 3-second start interval; therefore, a second refresh could start while the first request was still running. The older request could then finish last and overwrite the state produced by the newer request.

## Correction

Each provider class has an independent `AtomicBool` in-flight flag. `claim_refresh_slot` acquires the flag with compare-exchange before checking the existing minimum interval. A `RefreshLease` holds the flag for the entire asynchronous command and releases it in `Drop`. The guard therefore releases on normal completion, early return, provider/parse failure, and future cancellation. Rate limiting remains in place; single-flight does not replace throttling.

Market data and screener use separate flags because they have independent providers, refresh policies, and state targets. Concurrent refreshes of the same class are rejected; one class does not block the other.

## Validation

- Node source-contract test checks the guard wiring and required native test cases.
- Rust unit tests cover in-flight overlap even after the rate window elapses, minimum interval enforcement, lease release after a throttled attempt, and fail-closed handling of wall-clock rollback.
- In this workspace, `npm test` and `npm run typecheck:layers` pass. The Rust unit tests have **not** been executed because Cargo and rustfmt are unavailable. This limitation is not waived by the source-contract tests.

## Safety

This change affects public read-only market-data refresh coordination only. It adds no demo/mock data, no broker integration, no order intent, and no live execution permission.
