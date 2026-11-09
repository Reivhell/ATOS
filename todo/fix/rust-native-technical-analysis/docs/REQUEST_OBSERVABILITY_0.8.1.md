# Request observability contract — 0.8.1

## Purpose

The optional loopback Bun/Hono research API emits one best-effort completion observation for each request that reaches the Hono middleware. Its purpose is operational correlation and coarse failure analysis; it is not an audit record of trading decisions and it does not authorize decision handoff or execution.

## Event schema

Each JSON line uses schema `local-research-api.request.v1` and these fields only:

- `event`: stable event name (`http.request.completed`)
- `requestId`: server-generated UUID also returned in `X-Request-ID`
- `method`: allowlisted HTTP method or `OTHER`
- `route`: `/health`, `/v1/analysis`, or `/other`; arbitrary path segments are never emitted
- `statusCode`: HTTP status class outcome as the observed status code
- `durationMs`: rounded nonnegative duration, capped at 300,000 ms
- `outcome`: finite taxonomy maintained in `api/src/request-observability.ts`

Never add request/response bodies, query strings, raw URL paths, arbitrary headers, authorization values, cookies, IP addresses, provider data, raw exception messages, or user-controlled metadata without a new threat/privacy review.

## Log volume and failure policy

The production server uses an O(1)-memory observer capped at 120 emitted records per 60-second window. Additional events are counted and the suppression count is attached to a subsequent emitted record. The sink does not allocate buckets by client identity. Output writer failures are swallowed by design: observability must not turn an otherwise valid response into a failed request. This is best-effort process telemetry, not durable storage; events can be lost on process termination or while output is failing.

## Correlation and security

`X-Request-ID` is generated on the server. A caller-supplied request ID is never trusted or reflected. The request ID is a correlation handle, not an authentication token and not proof of identity. The API remains loopback-only, bearer-authenticated, no-CORS, research-only, and without order routes. Request context checks still happen before consuming the shared limiter.

## Validation

`npm run test:request-observability` tests schema allowlisting, route/query redaction, clock bounds, finite outcomes, fixed-memory sampling behavior, configuration rejection, integration wiring, and the production observer limit. Bun/Hono end-to-end tests and the native/frontend build remain mandatory CI release gates and were not executed solely by running Node source tests.
