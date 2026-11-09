# Local Research API Request Boundary — 0.7.0

Status: implemented candidate; Bun/Hono wire-level acceptance must run in CI before release.

## Threat model addressed

The optional research API binds to a literal loopback interface and requires a high-entropy bearer token. Loopback binding alone is not a complete HTTP boundary: DNS rebinding can manipulate the HTTP `Host` authority, and cross-site browser probes can consume a process-wide rate limiter if request context is evaluated too late.

## Controls

- A dependency-free policy module validates canonical `Host` authorities: `localhost`, `127.0.0.1`, or bracketed `::1`, optionally followed by a canonical valid TCP port. The running server also requires an explicit Host port to match its configured listener port. Alternate numeric loopback spellings, userinfo, attacker-controlled suffixes, malformed ports, whitespace, and non-loopback hostnames are rejected.
- The API explicitly remains **non-browser/CLI-native only** until a reviewed CORS contract is implemented. Every request carrying an `Origin` header is rejected. This avoids accidentally suggesting that an Origin allowlist grants browser access when the service intentionally has no CORS middleware or preflight handler.
- Fetch Metadata rejects `cross-site` and `same-site` browser requests even when the browser omits `Origin`, blocking common cross-site subresource probes. `same-origin`/`none` metadata do not imply authentication; accepted requests still require the bearer token.
- Request-context rejection happens before the shared sliding-window limiter, so rejected browser traffic and forged Host authorities do not consume legitimate clients' authentication budget.
- CLI/native clients without `Origin` and Fetch Metadata may connect, but only through a canonical loopback Host and still require the bearer token. Host and Fetch Metadata checks are defense in depth, not a substitute for authentication.

## Compatibility and limits

- The API remains research-only: no order, trade, withdrawal, or execution route is introduced. Client-supplied candles remain provider-untrusted; fingerprints prove byte identity only.
- Do not reverse-proxy the API, expose it on a LAN, or enable browser access until a separately reviewed CORS/preflight policy, identity model, and distributed rate-limit design exist.
- Node unit tests exercise the pure request policy. Real Hono request/response tests are wired into the Bun CI job and must pass before production acceptance.
