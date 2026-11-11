# Local Research API — Streaming Boundary Hardening

## Finding

The incremental request-body reader previously awaited `reader.cancel()` after detecting an oversized chunk and after a stream read failure. Cancellation belongs to the underlying stream source. A source whose cancellation promise never settles could therefore hold the HTTP request open after the 2 MiB byte limit had already been exceeded. The timeout path also attempted to release the reader lock while a `read()` request could still be pending; WHATWG streams reject that operation, which could replace the intended bounded timeout response with an internal error.

The failure was reproduced against the repository implementation on Node.js 22 using a `ReadableStream` whose `cancel()` returned a never-settling promise. The pre-fix oversized-body path did not return within 100 ms.

## Corrective implementation

- Overflow calls cancellation on a best-effort basis and returns `too_large` without awaiting the source.
- Read failure similarly initiates cancellation without awaiting it and returns `read_error`.
- Timeout initiates cancellation without awaiting it and returns `timed_out`.
- Timeout deliberately skips `releaseLock()` while a read request remains pending. Releasing a reader in that state is not a valid cleanup operation and may itself throw.
- Normal completion, overflow, and read-error paths still release the reader lock when there is no pending read.
- The fixed body cap remains 2 MiB and the default body-read deadline remains 10 seconds.

## Regression coverage

`test-local-research-api.test.mjs` now exercises oversized and stalled streams whose `cancel()` never resolves. Both paths must return their bounded result without waiting for cancellation. Static contract coverage prevents reintroducing an awaited `reader.cancel()` into the request reader.

## Scope and residual limitation

This change bounds how long application request handling waits; it cannot force an arbitrary JavaScript stream source to complete its own cancellation routine. The API remains loopback-only, bearer-authenticated, research-only, and has no order route. It does not establish provider authenticity for caller-supplied OHLCV and does not grant execution authority.

## Verification

- Focused API validation suite: passed on Node.js 22.
- Full repository Node test suite: 295 passed, 0 failed.
- `typecheck:layers`: passed.
- `verify:repo`: passed with expected missing-lockfile warnings.
- Native Rust, Tauri, Svelte production build, Bun integration tests, and MetaEditor compilation were not available in this execution environment and are not claimed as passed.
