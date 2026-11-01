# Strict evidence provenance sprint tracker

Scope: strict-analysis evidence validation only. This percentage does not represent overall ATOS product completion or readiness for live trading.

Last updated: 2026-11-01
Working branch: `dev`

## Overall progress: 73%

Progress is a weighted checklist, not a subjective estimate of production readiness. A completed source-level check is not counted as a passing test unless the test has actually run.

| Work package | Weight | Progress | Evidence / exit criteria |
|---|---:|---:|---|
| Canonical evidence reference format and SHA-256 snapshot binding | 20% | 100% | Rust and JS gates require canonical `snapshot:<sha256>:<source>:<locator>` references bound to the dataset digest. |
| Trusted source allowlist and path safety | 15% | 100% | Rejects unregistered source IDs, path traversal, empty/dot-prefixed path segments, duplicate references, and over-limit counts/lengths. |
| Native market-provider source boundary | 15% | 100% | Rust snapshot service, Rust completion gate, and JS parity gate accept the exact Binance source label or its documented ` · ` detail suffix; spoofed suffixes are rejected by the JS regression case. |
| Adversarial regression coverage | 10% | 70% | Added Rust cases for exact market snapshot resolution, source spoofing, wrong symbol/timeframe, missing candle timestamps, cross-snapshot references, and noncanonical timestamps. Full Node and Rust test runs on the current head remain unverified. |
| Runtime evidence locator registry | 65% | 25% | Rust now resolves Binance references against the exact symbol, timeframe, snapshot digest, and candle timestamp, and rejects noncanonical timestamp encodings. Registry resolution for Rust indicator, structure, and backtest outputs is still missing. |
| Producer-issued evidence attestations | 10% | 0% | Not implemented: assessment references are not yet authenticated as producer-issued attestations tied to Rust calculation outputs. |
| CI verification on current head | 5% | 0% | No passing CI result has been observed for the current sprint head. |

## Current sprint changes

- Tightened native provider-source checks to require either the exact `Binance public REST` label or a label beginning `Binance public REST · `.
- Applied the same boundary to the Rust service, Rust completion gate, and JavaScript parity gate.
- Added regression coverage for a spoofed `Binance public REST spoof` label and valid provider labels.
- Expanded evidence locator rejection tests and corrected static contract assertions in earlier commits.
- Added a Rust regression test for exact snapshot resolution and hardened the resolver against timestamps with leading zeros. These latest Rust changes are source-reviewed only; Rust compilation and test execution have not been observed.

## Next milestones

1. Extend the Rust-owned resolver beyond candle timestamps to registered indicator, structure, and backtest outputs; do not treat an agent-authored locator as a producer attestation.
2. Require each evaluated assessment reference to resolve to an entry in that registry and match its dataset digest, source type, and output identity.
3. Add adversarial Rust unit tests and JS/Rust contract tests for missing, forged, stale, cross-snapshot, and duplicated registry entries.
4. Run the complete Node and Rust suites and capture CI results for the exact `dev` head.

## Safety status

Strict completion remains fail-closed when catalog coverage, evidence, market-data provenance, or risk approval is invalid. This sprint does not enable live order execution. The remaining calculation-output registry and producer-attestation gaps mean evidence provenance is not yet fully proven.
