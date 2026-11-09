# Bun/Hono Local Research API — 0.5.9

## Purpose and scope

This is an optional, local-only HTTP adapter for bounded research analysis. It does not replace Tauri IPC or the Rust backend, and it does not expose order placement, account, broker, or execution endpoints. The desktop app does not depend on this service to start.

## Runtime

- Runtime: Bun; HTTP routing: Hono.
- Bind address is restricted to literal loopback IPs `127.0.0.1` or `::1`. Hostnames, including `localhost`, are rejected so resolver configuration cannot choose the bind address.
- `LOCAL_RESEARCH_API_PORT` defaults to `8787` and must be in `1024..=65535`.
- `LOCAL_RESEARCH_API_TOKEN` is mandatory and must contain at least 32 characters. Use a randomly generated secret; do not commit it or write it to logs.
- Start explicitly with `bun run api/src/server.ts`. This service is not automatically launched by the desktop application.

## HTTP contract

- `GET /health`: process health and explicit `executionAuthority: none`.
- `POST /v1/analysis`: accepts JSON containing `symbol`, `timeframe`, and 34–2,000 OHLCV candles. Candle timestamps must be strictly increasing, aligned to the supported UTC interval grid, contiguous, and no more than 30 seconds in the future. OHLC geometry and finite numeric fields are required.
- All routes require `Authorization: Bearer <token>`. POST requires `Content-Type: application/json`.
- Request bodies are capped at 2 MiB. A fixed-memory sliding window permits at most 30 requests in any 60-second interval across the process, including failed authentication attempts. The limiter uses a monotonic production clock, rejects clock rollback fail-closed, and returns `Retry-After` with HTTP 429. Rejected requests cannot increase the limiter state size. POST accepts no charset declaration or UTF-8 only; compressed bodies are rejected.
- Unknown routes return a bounded JSON 404. Internal errors do not echo payloads or secrets.

## Trust and safety

Caller-submitted candles remain provider-untrusted. They are passed to the technical-analysis engine with `validated: false` and a server-computed canonical `datasetSha256`. The response says `datasetFingerprintComputed: true` and `datasetFingerprintVerified: false`: the hash supports repeatability/evidence binding but was not compared with a trusted external digest and does not prove data origin. The API still returns `providerAuthenticated: false`, `researchCandidateHandoffAllowed: false`, and `executionAuthority: none`.

The API intentionally has no CORS middleware and binds only to loopback. Do not reverse-proxy it or expose it on a LAN without a separately reviewed threat model, identity model, and rate-limit store. The rate limiter is process-local and suitable only for this local adapter, not a horizontally scaled public service. Do not expose this API on a LAN or reverse-proxy it without a separately reviewed identity, networking, and distributed rate-limit design.

## Release gate

The API is a candidate integration until CI executes Bun install/runtime tests and Hono route tests, the npm and Rust dependency lockfiles are committed and verified, and the normal frontend/Rust/desktop acceptance pipeline passes. Source-contract tests are not substitutes for live HTTP tests. The Bun CI job must run `bun test api/test/app.integration.test.ts`, not only the dependency-free source tests.
