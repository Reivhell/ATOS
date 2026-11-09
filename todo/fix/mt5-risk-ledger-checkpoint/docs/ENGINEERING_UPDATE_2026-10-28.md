# Engineering update — 2026-10-28

## Scope and release position

This is a hardening continuation of the existing Agentic Trading OS `0.5.9` candidate, not a version-only release. The candidate remains **NOT PRODUCTION-CERTIFIED**. The work closes concrete API abuse and release-validation gaps but does not claim that the Svelte/Tauri/Rust product passed an end-to-end production acceptance run.

## Implemented

### Bounded sliding-window request budget

- Replaced a fixed-window request counter with a fixed-capacity ring buffer sliding window: 30 requests per any 60-second interval, process-wide, including failed authentication attempts.
- Rate-limit storage is bounded by the configured capacity (30 timestamp slots in production), and rejected attempts do not allocate new entries or per-client buckets.
- Production rate limiting uses `performance.now()` rather than wall-clock time. Backwards clock movement is clamped to the last observed instant instead of resetting the budget.
- HTTP 429 responses include `Retry-After` derived from the oldest live event.
- Added five deterministic Node tests, including a sustained 50,000-rejection case to assert the memory-bounded path remains constant in shape.

### Stronger research API trust boundaries

- Bind configuration now accepts only literal loopback IPs (`127.0.0.1` and `::1`); hostnames such as `localhost` are rejected to avoid making the security boundary depend on resolver configuration.
- JSON POST requests accept no charset parameter or an explicit UTF-8 charset only. Other charset declarations are rejected with HTTP 415.
- Bearer comparison iterates over the configured expected token length instead of scaling the comparison loop to an attacker-supplied header length. This reduces a controllable work factor but is not advertised as a cryptographic constant-time primitive.
- Fingerprint provenance is stated precisely: `datasetFingerprintComputed: true`, `datasetFingerprintVerified: false`. A server-computed hash helps identify/reproduce input bytes; it does not authenticate a provider or verify an independently trusted digest.
- Provider-authenticated handoff, research-candidate handoff, and execution authority remain disabled.

### Release and CI integrity

- The Bun job now executes `bun test api/test/app.integration.test.ts`; the previous workflow only ran the dependency-free `.mjs` suite despite the Hono integration suite existing.
- Tagged-release acceptance now requires actual `package-lock.json`, `bun.lock`, and `src-tauri/Cargo.lock` files, a frozen Bun install, Bun source tests and Hono API integration tests, plus the existing npm, Rust, desktop-build, and artifact-evidence gates.
- The repository guardrail and release preflight check the expanded contract. Placeholder lockfiles are explicitly not created.

## Verification evidence collected in this workspace

| Check | Result |
|---|---|
| `npm run test:rate-limit` | PASS — 5/5 |
| `npm run test:api` | PASS — 6/6 |
| `npm run verify:repo` | PASS, with three expected missing-lockfile warnings |
| `npm run typecheck:layers` | PASS |
| `npm test` | PASS — 142/142 tests across 24 Node suites |
| TypeScript syntax parse | PASS — API app, Bun server, and integration tests parsed |
| CI YAML parse | PASS — 5 jobs parsed |
| `npm run release:preflight` | BLOCKED — 9 prerequisites |

## Blockers not concealed

- `package-lock.json`, `bun.lock`, and `src-tauri/Cargo.lock` are still absent. Generating them requires their real package managers and dependency resolution; placeholder files would not make installs reproducible.
- An attempt to generate the npm lockfile failed because `registry.npmjs.org` resolution returned `EAI_AGAIN` in this environment.
- Bun is not installed locally, so `api/test/app.integration.test.ts` has not been executed here. It is now a mandatory CI step; the local source-contract suite is not represented as a substitute for it.
- `svelte-check`, Vite and Tauri CLI are not installed, and Cargo/rustfmt/Clippy are unavailable. Consequently the Svelte production build, native Rust formatting/tests/Clippy, desktop packaging and OS-level acceptance remain unverified.

**Release decision:** keep the candidate blocked until all three lockfiles are generated and committed and every runtime/native/desktop CI gate passes. This update increases verifiability and security; it does not itself satisfy production certification.
