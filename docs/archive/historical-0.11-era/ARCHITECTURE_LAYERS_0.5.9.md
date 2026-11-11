# Architecture boundaries — 0.5.9

The core desktop architecture remains `Svelte presentation → application services → deterministic analysis/decision engines → Tauri IPC → Rust authority`. Domain modules remain runtime-independent. Compatibility facades preserve existing import paths while implementations live in domain, engines, application, adapters, presentation, and infrastructure layers.

## Bun/Hono integration boundary

`api/src/server.ts` is an opt-in Bun process and `api/src/app.ts` is its Hono HTTP adapter. This process is deliberately separate from the Tauri desktop process. It must not be auto-started by the desktop app and must bind to loopback only. It is an integration surface for local read-only research, not a replacement for the Rust risk gate or a broker adapter.

The API validates candle shape/cadence and can calculate research observations, but a request body is not provider provenance. It therefore supplies `validated: false` and no trusted dataset fingerprint to the analysis engine. The API must not issue an execution grant. There are no order, account, broker, or execution routes.

## Dependency direction

- `domain`: value contracts and policy; no runtime or framework imports.
- `engines`: deterministic calculations and decision evaluation; no I/O authority.
- `application`: coordinates use cases and enforces trust boundaries.
- `adapters`: Tauri IPC or HTTP transport.
- `presentation`: Svelte/dashboard mapping.
- `infrastructure`: serialization and artifact/report concerns.

Hono is restricted to the HTTP adapter. It must not leak into domain or deterministic engine modules. Bun is the target runtime for the optional API and is under CI compatibility validation; Node/npm remains a fallback for the existing desktop build until frozen lockfiles and complete cross-runtime CI are proven.
