# AI Agent runtime v0.8.8

## Purpose and trust boundary

The AI Agent is a bounded research assistant, not a privileged controller. It receives a native-generated evidence bundle and may return only text observations attached to those native evidence references. Provider output cannot create signal strength, direction, canonical IDs, tools, order intents, or execution authority. The strict decision engine consumes the independently constructed native evidence and canonical registry output—not model prose.

## Providers and credentials

- `openai`, `anthropic`, and `openrouter` use fixed HTTPS endpoints. No user-provided URL is accepted, which avoids arbitrary-endpoint SSRF through the provider setting.
- `ollama` is restricted to `127.0.0.1:11434`; non-loopback host overrides are not supported.
- API keys are stored in the OS credential manager. If the credential store is unavailable, credential setup fails; keys are not persisted to JSON, environment files, SQLite, or UI state.
- Requests use bounded connection/total timeouts, disable redirects, cap provider response bytes, restrict provider output tokens, and allow only one in-flight provider operation in a process.
- Run budget is 12 starts per hour and 50 per day. Failed requests consume budget deliberately to limit provider abuse and surprise spend; token caps are not a billing estimate.

## Model import semantics

GGUF, ONNX and SafeTensors import supports registration and integrity checking only. Import copies into an application-managed directory using a generated storage name, checks file size, hashes content while streaming, validates a structural header, and optionally compares a user-supplied SHA-256. Model count and combined disk budget are capped. Removal requires user confirmation and is journaled. File deletion is best effort; on copy-on-write/SSD storage it must not be treated as secure physical erasure.

Imported artifacts are **not loaded or executed** by this runtime. `localModelInferenceAvailable` therefore remains false. Local inference requires a pre-installed Ollama model, accessible only through loopback. No automatic code/model plugin execution occurs during import.

## Evidence and decision flow

1. The native market-data path supplies a validated candle series for the requested symbol and timeframe.
2. Rust builds bounded trend, momentum, structure and location evidence and binds them to a canonical dataset fingerprint, latest fully closed candle cutoff, provider receipt timestamp, symbol and timeframe. The candle cutoff and local receipt time are separate provenance fields; the cutoff must match the final candle interval exactly, and the receipt must still be within the 120-second provider retrieval window.
3. Rust creates a digest over the score-bearing envelope fields. The application orchestrator reconstructs that envelope using a deterministic representation and Web Crypto SHA-256; a mismatch blocks the run from entering the decision engine.
4. The orchestrator creates/verifies canonical evidence registry entries and calls the same strict research decision engine used by other analysis routes. Model summary, caveats and disagreement prose do not contribute to a score.
5. Before accepting the report, the application layer independently checks that the ISO cutoff equals `latestClosedBarAtMs`, the receipt timestamp is not earlier than the candle close, and both timestamps remain fresh at evaluation time. A report that expires while a slow model request is in flight is blocked.
6. Output is explicitly research-only (`executionAuthority: none`, `liveExecutionAuthorized: false`, `orderIntent: null`). Stale or non-live input cannot be upgraded by the model’s output.

The provider receipt timestamp is displayed separately from the latest closed-bar cutoff so operators can distinguish observation time from ingestion time. The envelope digest is a consistency check across layers, not a cryptographic signature or a trust anchor against a compromised process. The audit chain is hash-chained and append-only at the SQLite schema level, but it is not externally anchored or signed. A privileged attacker who can rewrite all application files can potentially rewrite the journal too.

## Operational limits and release gates

- Maximum supported market history and payload limits are enforced in native code. Provider results are schema-checked and size-limited.
- Local imported model file size is capped at 16 GiB per file, 64 artifacts, and 64 GiB combined by registry metadata. Import requires enough free disk space in practice; disk availability is not guaranteed by those logical caps.
- Before deployment, run a pinned/frozen dependency installation, frontend Svelte check and Vite build, Rust `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`, Tauri packaging, automated tests against mock provider servers, and manual acceptance on each supported OS. Validate OS keychain behavior on each target. Real-provider smoke tests should use dedicated low-privilege keys and strict spend limits.
- This release candidate is not production-certified until those gates pass. It does not submit or authorize orders.
