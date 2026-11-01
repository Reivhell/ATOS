# Validation Report — Agentic Trading OS v0.5.8

## Delivery summary

This candidate performs a structural refactor of the analytical and decision path, with compatibility facades retained at old import locations.

- **Domain** now owns decision contracts and policy, catalog/strict-registry data, and market-data validation/snapshot identity.
- **Engines** now own numeric indicators, causal structure research detectors, a structured bounded technical-analysis envelope, the strict decision gate, and a separate deterministic research-decision engine.
- **Application** coordinates trusted chart snapshots, scope/cutoff checks, engine invocations, readings, and boundary diagnostics. Svelte uses this service rather than importing the analysis engine directly.
- **Adapters** contains the typed Tauri bridge; **presentation** contains pure dashboard helpers; **infrastructure** owns serialization/export implementations. Old root-level imports are thin compatibility re-exports rather than second implementations.
- Technical-analysis catalog data is separated from calculation logic. Shared numerical metrics drive both indicator readings and machine analysis to reduce calculation drift.
- The research decision engine requires unique independent domains, matching UTC cutoff and canonical SHA-256 fingerprint, explicit instrument/timeframe scope, acceptable signal agreement/strength, and risk receipt preconditions for paper-trade intent. Its outputs are research labels only; `orderIntent` is always null and execution authority is always none.
- The current UI chart-series path does **not** provide a canonical Rust-issued dataset fingerprint or native risk receipt, so the decision candidate remains intentionally blocked. No synthetic fingerprint is minted in JavaScript.
- A strict `npm run typecheck:layers` command is now wired into the frontend CI job before the Svelte build.

## Local verification results

- `npm test`: **PASS — 131 tests passed, 0 failed, 0 skipped** across the Node.js suite. This includes dependency-free tests and source-contract guards; Rust source-contract tests do not compile or execute the Rust test suite.
- `npm run verify:repo`: **PASS** — static repository guardrails. It warns that `package-lock.json` and `src-tauri/Cargo.lock` are missing.
- `npm run typecheck:layers`: **PASS** — TypeScript strict/checkJs validation of the changed domain, technical-analysis engines, decision engine, application service, dashboard helper, serializer, catalog, and shared DTO types.
- Final ZIP archive integrity (`unzip -t`): **PASS** after packaging. SHA-256 is recorded beside the archive.
- `npm run build`: **BLOCKED** — `svelte-check: not found`; dependencies are not installed in this workspace and package registry access was unavailable.
- `npm run release:preflight`: **BLOCKED — 8 issues**: missing `package-lock.json`; missing `src-tauri/Cargo.lock`; missing installed `svelte-check`, `vite`, and `tauri` executables; unavailable `cargo`, `rustfmt`, and `cargo clippy`.
- `cargo fmt`, `cargo test`, and `cargo clippy`: **NOT RUN** because the Rust toolchain is unavailable. The native Rust implementation paths and new native test cases remain uncompiled/unexecuted here.
- Bun compatibility lane: **NOT RUN locally** because Bun is unavailable. Node/npm remains the supported installation/build path. A Bun CI compatibility job remains staged; no migration is claimed complete.
- Desktop runtime/installer acceptance: **NOT RUN**.

## Engineering limitations and known blockers

1. The UI still cannot provide the canonical dataset fingerprint expected by the research decision engine, because the chart-series contract currently exposes no Rust-issued fingerprint for that exact immutable dataset.
2. The frontend `WeakSet` identity guard prevents arbitrary cloned objects from being treated as validated in the normal application path, but it is not a security boundary against compromised frontend code. Native Rust remains authoritative.
3. Indicator strength is heuristic and not calibrated probability. Structural observations are research evidence, not universal ICT/SMC semantics or a claim that all catalog concepts are implemented.
4. The cross-domain research verdict does not optimize parameters, estimate future profitability, or authorize any order. `paper_trade` requires native risk receipt prerequisites; the UI path cannot mint that receipt. Live trading remains disabled.
5. Reproducible dependency resolution, full Svelte/Vite build, native Rust test/Clippy results, and desktop packaging/runtime acceptance are still release gates. Lockfiles are deliberately not fabricated because the dependency graph could not be resolved in this environment.
6. Hono was not added: this Tauri desktop product has no standalone JavaScript HTTP API requiring it. Bun is staged for compatibility assessment but is not promoted to package-manager authority without a committed lockfile and successful frozen frontend/Tauri builds.

## Release decision

**NOT PRODUCTION-READY / NOT RELEASE-CERTIFIED.** The structural candidate and its dependency-free/targeted type checks are real improvements, but they do not substitute for the missing locked build, native Rust execution, and desktop acceptance. The next release gate must run in CI with committed dependency lockfiles and a supported Rust/frontend toolchain before certification.
