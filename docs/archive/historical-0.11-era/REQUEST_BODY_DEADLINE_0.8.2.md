# Request-body deadline contract — 0.8.2

## Threat addressed

The analysis API already caps request bodies at 2 MiB, but a peer could send a small body very slowly or stop mid-stream and keep a handler waiting indefinitely. A byte cap alone does not bound wall-clock resource occupancy.

## Contract

- `readBoundedBody` enforces a default 10,000 ms deadline, independently of the 2 MiB byte cap.
- The deadline can be overridden for tests; `timeoutMs` must be finite and between 1 and 120,000 ms. Invalid values throw rather than silently disabling the deadline.
- Expiry returns `{ kind: 'timed_out' }`, triggers best-effort stream cancellation without awaiting a potentially hostile cancellation promise, and the API responds with HTTP 408 / `request_timeout`.
- Timeout is included in the finite observability outcome taxonomy. Request bodies, query strings, credentials, and exception details are not logged.
- The route remains research-only; timeout handling does not grant provider authenticity, handoff, or execution authority.

## Validation

Node regression tests exercise a never-resolving stream, bounded completion, and invalid timeout configuration. A Bun/Hono integration test asserts the HTTP 408 contract and request correlation. The integration test is wired into CI, but was not locally executed because Bun and installed Hono dependencies are unavailable in this workspace.

## Limitations

This handler deadline is defense in depth and does not replace server-level header/read timeouts, connection limits, or OS resource controls. Native/desktop release certification remains subject to the existing lockfile, build, and acceptance gates.
