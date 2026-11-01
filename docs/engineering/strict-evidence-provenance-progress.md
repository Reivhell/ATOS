# Strict evidence provenance sprint

Last updated: 2026-11-01

## Overall progress: 61%

This percentage tracks the strict-evidence provenance workstream only, not the full ATOS product. The score is a weighted delivery estimate; authored code is not counted as validated until the relevant tests run successfully.

| Milestone | Weight | Completion | Weighted points | Evidence / remaining work |
|---|---:|---:|---:|---|
| Canonical reference format, snapshot digest binding, source allowlist, duplicate and size limits | 20% | 100% | 20.0 | Rust and JavaScript gates plus schema constraints exist. |
| Locator path safety and rejection of malformed segments | 15% | 100% | 15.0 | Regression cases cover traversal, empty segments, dot-prefixed segments, and unregistered source labels. |
| Resolve Binance evidence to the exact market symbol, timeframe, snapshot digest, and candle timestamp | 25% | 70% | 17.5 | Rust completion gate now resolves locators against the actual candle array. Rust regression tests are authored but have not been executed in this environment. |
| Trusted registries for MT5, Rust indicator, Rust structure, and backtest evidence | 20% | 0% | 0.0 | These source types are not yet backed by resolvable producer-owned evidence registries; do not treat them as verified provenance. |
| Rust/JavaScript/schema contract parity and regression coverage | 10% | 60% | 6.0 | Static contract assertions and JavaScript rejection tests are present; full suite has not been run against this branch state. |
| CI evidence and end-to-end verification | 10% | 20% | 2.0 | CI workflow exists, but no successful run for the latest commits has been confirmed. |

Weighted total: 60.5%, rounded to **61%**.

## Recent implementation

- Rust now accepts a Binance evidence locator only when its snapshot digest matches and the locator's symbol, timeframe, and timestamp resolve to a candle in the same market snapshot.
- Locators using another evidence source, a different instrument or timeframe, an unknown timestamp, or a catalog-only placeholder fail closed at the Rust completion gate.
- Added Rust regression coverage for valid and unresolved locators and a static contract assertion to protect the new behavior.
- Existing JavaScript validation remains a parity/early-rejection layer; Rust remains authoritative for completion.

## Validation status

- Rust tests: **not run / not verified**.
- Full JavaScript test suite: **not run / not verified against the latest branch state**.
- GitHub Actions for latest commit: **no confirmed successful run available**.
- Live trading authorization: **not enabled**.

## Next measurable milestones

1. Run `cargo test --manifest-path src-tauri/Cargo.toml` and resolve any compile or test failures.
2. Run `npm test` and `npm run verify:repo` against the same commit.
3. Add producer-owned registries for Rust indicator and structure outputs, then MT5 and backtest outputs; each locator must resolve to immutable, snapshot-bound evidence rather than a caller-provided label.
4. Add integration tests that tamper with each registry entry and prove the Rust gate blocks the completion.
5. Record successful CI run URLs and update this percentage only when each milestone's acceptance evidence is available.

## Latest related commits

- [Evidence resolution against market candles](https://github.com/Reivhell/ATOS/commit/faf89213734dc8ebc70dca1a8dfe162020d1e0fe)
- [Unresolved-locator regression test](https://github.com/Reivhell/ATOS/commit/fb0a9da19020e28f7897e1d7c9894d726dc9de47)
- [Static evidence-resolution contract](https://github.com/Reivhell/ATOS/commit/89890a45980e05d0df98b7603486cb6d730267d7)