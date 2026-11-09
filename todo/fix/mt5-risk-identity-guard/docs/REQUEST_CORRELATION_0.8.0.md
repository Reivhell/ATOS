# Request Correlation — 0.8.0

The local research API assigns a fresh server-generated UUID to every Hono request and emits it as `X-Request-ID`. Caller-supplied request IDs are intentionally not trusted or reflected. The identifier is for operational correlation only; it is not an authentication token, idempotency key, or authorization capability.

Do not add request bodies, bearer tokens, or provider payloads to logs when adding server-side correlation. Integration tests in `api/test/app.integration.test.ts` assert that both successful and unauthorized responses carry distinct IDs. The runtime integration suite still must execute under Bun/Hono CI before release certification.
