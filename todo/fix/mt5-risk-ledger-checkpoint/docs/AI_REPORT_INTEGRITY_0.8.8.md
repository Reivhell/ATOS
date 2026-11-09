# AI report integrity boundary — v0.8.8

## What changed

The application-side `evaluateAgentResearchReport` now independently validates commentary returned by the native Rust runtime before deterministic decision evaluation:

- provider is one of the fixed supported identifiers; model ID and generated timestamp are bounded and validated;
- summary, assessment, caveats, disagreements, and follow-up lists have strict size/count ceilings;
- exactly one observation is required for each registered domain;
- each observation may cite only its assigned `native:<domain>` evidence reference;
- the fixed safety note must match the runtime contract;
- the application recomputes SHA-256 over the ordered, normalized commentary document and compares it with the native `outputSha256`.

The evidence scoring-envelope digest continues to be checked independently. The LLM narrative remains excluded from scoring, and no order intent or execution authority is produced.

## Security semantics and limitations

SHA-256 detects accidental or post-boundary content changes when the expected digest is trusted. It is not a digital signature, does not authenticate the provider, and does not protect against a compromised native process that can alter both content and digest. This check is not a substitute for Rust compilation, integration tests, signed build provenance, or a reviewed trust boundary.

## Required production gates

See `VALIDATION_REPORT_0.8.8.md`. Frontend/native builds, lockfile-backed installation, real provider mock-server tests, keychain acceptance on supported operating systems, and desktop packaging must pass before production certification.
