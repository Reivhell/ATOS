# Request rate-limit isolation — 0.8.3

## Risk addressed

The previous local API applied the shared request budget before bearer authentication. Any local process able to reach the loopback listener could create failed-authentication pressure by spending that budget with invalid credentials and denying service to a client that knows the token.

## Contract

- Request-context validation still precedes rate accounting; unsupported browser origins do not spend either budget.
- Failed bearer authentication uses a separate fixed-memory sliding-window budget: 10 attempts per 60 seconds by default. After the budget is exhausted, responses are HTTP 429 with `Retry-After`.
- Only successfully authenticated requests consume the operational budget: 30 requests per 60 seconds by default.
- The budgets are independent and use a monotonic clock in the server configuration. No client-controlled IP header or unbounded per-client map is introduced.
- Both budgets are process-local. They are not a distributed rate limiter and do not provide cross-process coordination.
- The API remains loopback-only, browser-Origin rejecting, authenticated, research-only, and unable to execute orders.

## Validation

Dependency-free Node tests verify independent quotas, exact window expiry, rejected-request memory bounds inherited from the generic limiter, and invalid configuration behavior. A Bun/Hono integration test covers a failed-authentication flood followed by a valid request; that test must run in CI with Bun and dependencies installed before release certification.
