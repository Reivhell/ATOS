# Validation report — v0.3.8

## Change under test

Rust-issued strict-analysis work plans are now atomically single-use at completion. The first request that supplies the authentic, unexpired plan consumes it. Subsequent or concurrent submissions using the same plan fail closed. A tampered request is rejected without consuming the authentic plan. A blocked attempt also consumes the plan; a retry requires a new plan.

## Checks actually run

- `npm test`: **74 tests passed, 0 failed**. These include static Rust source-contract checks; they do not execute the Rust compiler or native unit tests.
- `npm run verify:repo`: **passed**, with warnings that `package-lock.json` and `Cargo.lock` are missing.
- `npm run build`: **failed with exit code 127** because `svelte-check` is not installed in this environment.
- `cargo test`, `cargo clippy`, and native compile: **not run** because neither `cargo` nor `rustc` is available.
- ZIP integrity and SHA-256: checked after packaging.

## Important limitation

Single-use state is stored in an in-process memory registry. It protects against duplicate completion attempts within one running process, including races serialized by the registry mutex, but it is not durable across process restarts or multiple application instances. A production deployment must use a persistent transactional store with an atomic consume operation before treating replay protection as restart-safe.

## Catalog and agent status

The trusted inventory remains 364 concepts across nine categories. Status remains 6 `Calculated now`, 1 `Workflow available`, and 357 `Catalog only`; no concept is promoted by this increment. The nine knowledge packs remain available, but runtime agent dispatch is not configured and work plans still report `work_plan_created_not_dispatched`. Live execution remains disabled.

## Release status

**Not production-ready or release-certified.** Native compilation and Rust unit tests must be run in a toolchain-equipped environment; dependency lockfiles and build reproducibility must be addressed. The frontend build must pass, and process-local replay protection must be replaced or backed by a durable transactional consume mechanism for production multi-process/restart guarantees.
