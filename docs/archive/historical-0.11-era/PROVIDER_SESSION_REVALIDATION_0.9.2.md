# Provider Session Revalidation — 0.9.2

## Problem
The desktop app persisted market quotes with `isLive=true` and a provider status beginning with `CONNECTED`. On a later process start, those persisted fields could be read before a new provider request succeeded. A recent timestamp alone is not a live connection proof.

## Repair
- Startup invalidates all restored quote live flags and converts cached market/screener state to explicit restart-stale status.
- Cached rows are retained for historical inspection; they cannot qualify as current observations until a successful new provider refresh replaces/updates them.
- Shared Rust predicates require a positive timestamp, a non-future observation time, a bounded age, a connected recent provider snapshot, and a live quote.
- The same market-freshness predicates are used in candle capabilities/response, order provenance, and whole-portfolio mark gating.
- Fixed-origin public provider HTTP clients disable automatic redirects. A provider redirect is rejected instead of changing the trust boundary.

## Verification boundary
Rust regression tests have been added for restart invalidation and future timestamps. This environment has no Cargo/rustc/rustfmt/Clippy toolchain, so those native tests are not claimed as executed. Node source-contract tests, full Node suite, strict JS/TS layer typecheck and source scan are the available verification methods. A pinned CI runner must run Cargo tests and build before release certification.


## Restart safety and current-evidence presentation

A process restart always re-engages the paper-trading kill switch, including when a prior persisted account had explicitly released it. Provider refresh in the new process does not release that switch; operator review and a separate release action are required. This avoids treating a new network response alone as authorization to resume a prior session's order policy.

The UI's quote freshness predicate requires a connected snapshot timestamp and a per-quote timestamp. Both must be safe integers, positive, not in the future, and no more than 120 seconds old. Screener rows are not displayed in current-evidence surfaces if the feed is partial, stale, disconnected, future-dated, or missing a valid provider timestamp. The cached rows remain stored for audit/history inspection only.

Validation includes runnable Node source-contract checks. The Rust unit regressions are included in `src-tauri/src/engine.rs` but were not executable in this workspace because the Rust toolchain is unavailable.
