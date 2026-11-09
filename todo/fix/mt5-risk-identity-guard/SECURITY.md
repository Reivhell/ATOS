# Security policy

## Current execution boundary

This application is local-first. The configured external providers are public, unauthenticated read-only data sources: Binance Spot REST for a fixed set of market quotes/candles and DEX Screener for a bounded latest-boosted-token discovery sample. No account/order, withdrawal, wallet-signing, or private exchange endpoints are called. The order panel submits only to the local Rust paper broker.

The Svelte frontend is not a security boundary. All supported paper-trading constraints are revalidated by Rust. Agent status is observe-only. Tools & MCP is disabled because arbitrary shell/file/network execution is not acceptable without a separate capability and threat model.

## Report a vulnerability

Do not include real trading keys, seed phrases, personal account exports, or private financial data in bug reports. Until a dedicated security contact is configured, report reproducible issues through the private channel used by the repository maintainers; do not publish exploit details before maintainers have a chance to assess the issue.

## Security principles

- Default to paper execution and explicit live-trading lockout. Account reset must not release the kill switch; safety locks require a separate explicit operator action.
- Never put secrets into `VITE_*` environment variables, frontend bundles, screenshots, or committed configuration.
- Keep exchange permission scopes minimal and never grant withdrawal permission to a future trading adapter.
- Fail closed when quote freshness, numeric validation, persistence, or risk checks fail.
- Prevent concurrent processes from mutating the same paper-account snapshot; the native engine acquires a process-lifetime exclusive SQLite lock before loading state. Preserve audit evidence and corrupt-state backups rather than silently treating a failure as success.
- Treat external market data and token metadata as untrusted inputs; cap streamed response bodies, validate schema, bounds, OHLCV integrity, timestamps, addresses, URL origins, and symbol identity before use.
- The optional Bun/Hono research API binds only to literal loopback IPs, requires a bearer token of at least 32 characters, limits the body to 2 MiB, and applies a fixed-memory 30-request/60-second sliding window before authentication. The body reader has a 10-second deadline and does not await an underlying stream's cancellation hook on overflow, read failure, or timeout; a hostile/non-settling cancellation hook must not convert a bounded 413/408 path into an unbounded wait. On timeout, a reader with a pending read is deliberately not unlocked because WHATWG streams reject `releaseLock()` while read requests remain pending. It exposes no order routes. Caller-supplied OHLCV remains provider-untrusted: a computed SHA-256 identifies the submitted bytes but is not a verified provider digest or permission to hand off a decision. Do not proxy or expose this API beyond loopback without a separate identity/network threat model.
- Audit CSV export quotes cells and prefixes common spreadsheet formula markers in text fields. Export files can still contain sensitive operational details; review before sharing. The browser runtime does not return audit rows. Audit history is exposed only from the native SQLite journal.
- The DEX Screener boosted-token feed is promotion-biased and not a security scan; missing contract, holder, liquidity-lock, wash-trading, and depth evidence remains unresolved.

## AI Agent runtime (0.8.7 candidate)

- Provider API keys are persisted only through the operating-system credential store. The runtime does not accept arbitrary provider URLs; remote providers use fixed HTTPS endpoints and Ollama is loopback-only. Never paste exchange account, wallet, withdrawal, or trading credentials into the AI provider field.
- Market snapshots, candle values, symbols, deterministic technical evidence, and a bounded request prompt are sent to the selected external model provider when you invoke a cloud research run. Do not use this feature with datasets whose disclosure to that provider is not permitted. Read the provider's own retention and data-processing terms before enabling it.
- Provider output is untrusted commentary. Native Rust-derived evidence and a cross-layer digest are passed to the strict decision engine; model prose cannot set score-bearing signals or grant execution authority. Reports do not submit orders and do not authorize live trading.
- Imported GGUF, ONNX and SafeTensors files are size-capped, hashed and structurally checked, but are registered only: this app does not execute imported artifacts. Model import is not proof that weights are safe, reputable, correctly trained, or compatible. Run local inference through a separately installed loopback Ollama instance and keep that runtime updated.
- Provider calls have bounded network timeouts/response sizes, a single-flight gate, output-token cap, and a persistent per-user run-count budget. The count budget is not a guaranteed monetary billing cap; configure provider-side spend limits independently.
- The SQLite agent journal is append-only by schema and hash-chained. It is tamper-evident, not digitally signed or externally anchored; an attacker with control of the user's application data and process can potentially rewrite both events and hashes. Audit verification failures block provider use and most mutations until the operator investigates the local store.
- Release certification remains blocked until dependency lockfiles, Svelte/Tauri build, Rust tests/Clippy, platform-specific credential-store checks, mock-provider integration tests, and signed package acceptance pass in a pinned CI environment.


## Research workspace isolation and nested parameter tournament (v0.10.1)

Research hypotheses, run cards, and research events are held in a distinct SQLite database and are not part of the account-state payload. Research commands have no order-authority API. Run cards are content-hashed; the event log is hash-chained; critical tables have immutable triggers checked against their exact normalized SQL at startup. A failed integrity check blocks the workspace. These controls are tamper-evident local protections, not remote notarisation or protection against a privileged local attacker. The research gate is not a trade approval and cannot change the paper-account kill switch or submit orders.

## Release artifact provenance (v0.9.6)

- Use only installers from the staged release payload; do not distribute raw build directories or un-attested app-bundle internals. Platform staging requires the declared installer formats and rejects empty, symlinked, colliding, or unexpected release-stage entries.
- Release CI generates an SPDX SBOM and creates GitHub Artifact Attestations for both SLSA build provenance and the SBOM. The consumer verifier binds the signature to the expected repository, `.github/workflows/ci.yml`, exact `refs/tags/vX.Y.Z` source ref, GitHub OIDC issuer, and hosted-runner policy.
- A manifest containing hashes is useful for corruption detection but is not publisher authentication. Do not accept an installer solely because `checksums.json` matches; perform `gh attestation verify` through `npm run release:attestation:verify` with the expected repository and tag.
- GitHub Artifact Attestations may be unavailable for private repositories on some plans. The tagged release fails closed when provenance cannot be created or verified; do not disable the gate to make a build pass.
- Attestation does not prove the software is vulnerability-free or that the build workflow itself is uncompromised. Review workflow changes, lockfile changes, SBOM components, vulnerability findings, and trusted workflow identities separately.

- Release CI rejects mutable third-party GitHub Actions references. Every external action must use a reviewed full 40-character commit SHA; major tags and branches are not adequate release pins. Check `docs/archive/validation/VALIDATION_REPORT_0.10.1.md` for the current fail-closed release blockers and verified action/toolchain pins.


The v0.10.1 nested walk-forward tournament is read-only with respect to the account ledger. It accepts only the validated BTCUSDT 5m series, caps candidate/fold/work counts, selects candidate parameters from each fold's training prefix, and evaluates them only on the next unseen block. Run-card validation verifies that each selected pair is the deterministic training-only winner and recomputes the fold aggregates. This research output cannot create order intent or authorize paper/live execution. Native Rust compile/test and full desktop build evidence remain required before production deployment.
