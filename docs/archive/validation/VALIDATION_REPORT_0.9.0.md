# Validation Report — Agentic Trading OS v0.9.0

Date: 2026-10-29

## Summary

This release adds an integrated, credential-free AI Agent release-qualification gate and tightens timeout containment and order-intent rejection at the JavaScript application boundary. The repository is an engineering candidate, **not production-certified**.

## Implemented

- Added `npm run eval:agents`, a machine-readable qualification runner that executes the existing reliability evaluator against the real `evaluateAgentResearchReport` application boundary.
- Added nine deterministic scenarios spanning all six required categories: provider contract validation; simulated provider timeout; attempted execution-authority escalation; injected non-null `orderIntent`; post-boundary evidence-envelope tampering; repeated-run determinism; narrative-only metamorphism; evidence-order permutation; and resource bounds.
- Required 100% pass rate and complete required category coverage. The report records a stable decision fingerprint, report integrity digest, a release-envelope SHA-256, `providerLiveIntegrationTested: false`, `executionAuthority: none`, `liveExecutionAuthorized: false`, and `orderIntent: null`.
- Added the report-producing command to repository guardrails and tagged-release CI; the guardrail job uploads the machine-readable evaluation report.
- Hardened evaluator timeout behavior: after a deadline breach it stops repeating the timed-out scenario, marks remaining scenarios `blocked`, and fails the quality gate. This avoids intentionally accumulating additional promises from an adapter that ignores cancellation.
- Tightened the application report boundary to reject a supplied non-null `orderIntent`, even if other authority flags remain disabled.
- Updated package, Tauri, Cargo, User-Agent and version-contract references to 0.9.0.

## Validation results

- `npm test`: **222 tests passed, 0 failed** across 32 suite invocations.
- `npm run typecheck:layers`: **passed**.
- `npm run verify:repo`: **passed**, with warnings that npm, Bun, and Cargo lockfiles are missing.
- `npm run eval:agents -- --report-file validation/agent-release-evaluation-0.9.0.json`: **9/9 scenarios passed**, zero failed, zero category gaps, policy passed.
- Release evaluation report SHA-256: `e8cf0f138ade6351c05fcd332ae7880feeb99f7ade857f506780b7aba06fa9c4`.
- Release evaluation envelope SHA-256: `5ac466617d9576870e7a3e35d31dd82356d21aea64d80408625930a1d8a2cc63`.
- `npm run release:preflight`: **blocked with 9 issues**, while confirming manifest version alignment and the new agent qualification gate passes.
- `npm run build`: **blocked** because `svelte-check` is unavailable (`svelte-check: not found`).

## Unresolved release blockers

1. `package-lock.json` is missing.
2. `bun.lock` is missing.
3. `src-tauri/Cargo.lock` is missing.
4. The `svelte-check` executable is not installed.
5. The Vite executable is not installed.
6. The Tauri CLI executable is not installed.
7. `cargo --version` is unavailable.
8. `rustfmt --version` is unavailable.
9. `cargo clippy --version` is unavailable.

The lockfiles must be generated from reviewed, actual dependency resolution and committed; they must not be fabricated. Native Rust compilation/tests/Clippy, full frontend build, Tauri packaging, live provider transport tests, OS keychain acceptance, signed artifacts, and cross-platform acceptance were not run in this environment.

## Scope and limitations

The qualification suite uses deterministic fixtures and a simulated provider timeout; it makes no network calls and uses no API keys. A pass proves only that this JS boundary satisfies these cases. It does not prove provider authenticity, model quality, statistical robustness, trading profitability, or production readiness. SHA-256 digests are integrity checks, not digital signatures. A JavaScript timeout cannot forcibly terminate a promise that ignores `AbortSignal`; provider adapters must also enforce transport deadlines.

No live trading authority, broker execution, or order generation was enabled. The application continues to require `executionAuthority: none`, `liveExecutionAuthorized: false`, and null order intent.
