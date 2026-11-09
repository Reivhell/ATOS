# Local Research API — Security Hardening Notes

Status: implemented in the 0.5.9 working candidate; runtime acceptance remains incomplete.

## Changes

1. **Streaming body cap.** `readBoundedBody` reads the request stream incrementally and grows one bounded buffer geometrically. It checks each incoming chunk against the remaining 2 MiB budget, cancels the stream on overflow, and does not trust `Content-Length` as the enforcement mechanism.
2. **Strict decoding.** UTF-8 decoding is fatal on malformed sequences. The route distinguishes oversized input, invalid encoding, body read failure, and invalid JSON without echoing request content.
3. **Authentication abuse budget.** A fixed-memory sliding-window limiter is consumed before authentication, so invalid bearer guesses are not exempt from the 30-request/60-second budget. A preallocated ring buffer stores at most 30 admitted timestamps; rejected requests do not grow memory. The limiter uses a monotonic clock in production and clamps clock rollback fail-closed. `429` responses include `Retry-After`. It is deliberately process-wide because the service is loopback-only and forwarding headers are untrusted.
4. **Request representation.** POST accepts only `application/json` with no charset or an explicit UTF-8 charset. Other charsets and unsupported content encodings are rejected instead of risking ambiguous decoding or compressed-body expansion.
5. **Response hardening.** API responses set `Cache-Control: no-store`, `Pragma: no-cache`, `X-Content-Type-Options: nosniff`, `X-Frame-Options: DENY`, and `Referrer-Policy: no-referrer`.
6. **Canonical dataset identity.** The API computes the cross-runtime OHLCV SHA-256 fingerprint using the same domain-separated, little-endian byte contract as the native Rust engine. This allows reproducible analysis and evidence binding without confusing byte identity with provider authenticity. The response explicitly marks the fingerprint as computed while provider authentication remains false.
7. **Trust semantics stay explicit.** The response marks `datasetFingerprintComputed: true` and `datasetFingerprintVerified: false`: the service computed a fingerprint, but did not compare it with a caller-supplied trusted digest or provider signature. Caller-supplied OHLCV remains untrusted; provider authentication, trusted candidate handoff, and execution authority remain disabled.
8. **Literal loopback binding.** The server accepts only `127.0.0.1` or `::1`; the `localhost` hostname is rejected to avoid relying on resolver configuration for the security boundary.

## Verification performed

- `node --test scripts/test-local-research-api.test.mjs`: 6/6 passed, including canonical cross-runtime fingerprint known-vector verification and actual bounded-stream behavior for a body over 2 MiB without `Content-Length`, fragmented chunks, and malformed UTF-8.
- The initial hardening baseline recorded 137 passing Node tests; follow-up validation below includes the new limiter regression suite.
- `npm run typecheck:layers`: passed.
- `npm run verify:repo`: passed, with expected lockfile warnings.
- `npm run release:preflight`: blocked by missing `package-lock.json`, `src-tauri/Cargo.lock`, installed Svelte/Vite/Tauri executables, and Rust/Cargo/rustfmt/Clippy toolchain.

## Follow-up engineering — 2026-10-28

- Replaced the fixed-window counter with a fixed-capacity, fixed-memory sliding-window limiter. Added deterministic tests for boundary bursts, exact event expiry, wall-clock rollback, invalid configuration, and 50,000 rejected requests without growing limiter storage. `Retry-After` is now returned on `429`.
- Rejected hostname-based binding; only literal loopback IP addresses are accepted. Restricted JSON charset declarations to UTF-8 and clarified that a server-computed dataset fingerprint is **computed, not verified**. The response now reports `datasetFingerprintComputed: true` and `datasetFingerprintVerified: false`.
- Corrected CI wiring so the Bun job runs `bun test api/test/app.integration.test.ts` in addition to source tests. Added integration coverage for rolling-window boundaries, retry timing, fingerprint trust labels, and rejected non-UTF-8 charsets. The integration tests are now wired to run in CI but still cannot be executed in this local workspace because Bun and installed Hono dependencies are unavailable.
- Local verification after the changes: `npm run test:rate-limit` 5/5 passed; `npm run test:api` 6/6 passed; `npm run verify:repo` passed; `npm run typecheck:layers` passed; complete `npm test` passed.
- `npm install --package-lock-only` was attempted and failed with registry DNS error `EAI_AGAIN`; no fake lockfiles were created. Release preflight remains blocked.

## Not yet verified

Bun/Hono HTTP integration, Svelte build, native Rust compilation/tests/Clippy, and desktop package acceptance have not been run locally. Dependency installation timed out; no lockfile was fabricated. This artifact is **not production-certified** until these release gates pass in a pinned CI environment.
