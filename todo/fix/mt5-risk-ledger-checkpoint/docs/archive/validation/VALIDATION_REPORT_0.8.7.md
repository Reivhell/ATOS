# Validation report — Agentic Trading OS v0.8.7

**Review date:** 2026-10-29  
**Outcome:** **NOT PRODUCTION-CERTIFIED — release preflight blocked**

## Scope delivered

This candidate adds an AI-agent research runtime integrated with the existing native Tauri/Rust market-data boundary, canonical evidence registry, and strict decision engine. It includes secure provider credential handling, bounded cloud/loopback provider requests, import/verification/removal of model artifacts, persisted request budgets, and a tamper-evident SQLite audit journal.

## Checks performed in this workspace

| Check | Result | Notes |
|---|---|---|
| `npm test` | **PASS — 199/199** | All Node-based repository suites passed, including AI-agent orchestrator tests and static native-contract checks. This is not a Rust runtime build. |
| `npm run typecheck:layers` | **PASS** | Strict TypeScript checking for the configured JS/TS application and domain-layer files. Svelte component compilation is not included in this command. |
| `npm run verify:repo` | **PASS WITH WARNINGS** | Static repository guardrails passed; dependency lockfiles are absent. |
| `npm run build` | **BLOCKED** | Exited 127: `svelte-check: not found`; frontend dependencies are not installed. |
| `npm run release:preflight` | **BLOCKED — 9 blockers** | See list below. |
| `npm install --package-lock-only --ignore-scripts --no-audit --no-fund` | **NOT COMPLETED** | Dependency resolution timed out and no `package-lock.json` was produced. No synthetic lockfile was created. |
| Rust `cargo test` / `cargo clippy` / `cargo fmt --check` | **NOT RUN** | `cargo`, `rustfmt`, and Clippy are absent in the workspace. |
| Bun/Hono integration tests | **NOT RUN** | Bun executable is absent. |
| Live provider smoke tests | **NOT RUN** | No provider credentials were supplied, and no real API call was made. |
| Tauri packaging / target OS keychain acceptance | **NOT RUN** | No native build toolchain or signed packaging environment is present. |
| ZIP round-trip verification | **PASS** | Archive integrity test passed; extracted source repeated `npm test` (199/199), `npm run typecheck:layers`, and `npm run verify:repo`. |

## Release preflight blockers

1. `package-lock.json` missing; npm dependency graph is not pinned.
2. `bun.lock` missing; Bun dependency graph is not pinned.
3. `src-tauri/Cargo.lock` missing; Rust dependency graph is not pinned.
4. `svelte-check` executable missing.
5. `vite` executable missing.
6. `tauri` CLI executable missing.
7. `cargo --version` fails because Cargo is unavailable.
8. `rustfmt --version` fails because rustfmt is unavailable.
9. `cargo clippy --version` fails because Clippy is unavailable.

The report intentionally does not treat source-contract tests as native compilation proof. Run release CI in an environment with frozen dependency installs and pinned Node/Bun/Rust toolchains before deployment.

## Runtime security contract

- Providers: fixed OpenAI/Anthropic/OpenRouter HTTPS paths and loopback-only Ollama. No user-supplied remote URL.
- API keys: OS credential store only; no plaintext file/environment fallback; secret values are not returned to the UI or included in journal payloads.
- Resource boundaries: bounded HTTP timeouts and response bytes, disabled redirects, bounded prompt/output, single-flight provider and model-registry operations, 12 runs/hour and 50 runs/24 hours, and capped imported model sizes/count/combined bytes.
- Model files: GGUF, ONNX, and SafeTensors receive structural checks before and after copy plus SHA-256 verification. They are **registered only and never executed by this application**. Imported model inference is unavailable; local inference can use an already-installed Ollama model.
- Integrity: the native scoring envelope is digest-checked by the JS orchestrator; audit events use append-only SQLite triggers and a hash chain. The digest is not a signature, and the local hash chain is not externally anchored.
- Decision boundary: the LLM returns commentary only; Rust-derived evidence is routed through the canonical evidence registry and strict decision engine. Fresh live connected data is required before any external analysis call. There is no order intent, `executionAuthority` remains `none`, and live execution stays disabled.

## Required next gates before production

1. Generate and commit lockfiles from successful package resolution in a network-enabled, reviewed environment; use frozen installs in CI.
2. Compile the complete Svelte app and Tauri native target; run Rust tests, format checks, and Clippy with warnings treated as errors.
3. Add/execute mock-server HTTP integration tests for each provider schema, status failure, redirect, timeout, truncated response, and budget exhaustion path.
4. Test actual OS keyring behavior on supported Windows/macOS/Linux targets, including locked/unavailable credential stores.
5. Test model import under disk-full conditions, concurrent requests, malformed adversarial files, process interruption, and removal cleanup failures.
6. Run end-to-end desktop acceptance and signed packaging on each supported OS. Keep live trading disabled until a separate reviewed execution architecture, risk controls, and operational approval exist.
