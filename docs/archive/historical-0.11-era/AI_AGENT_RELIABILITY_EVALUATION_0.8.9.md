# AI Agent Reliability Evaluation Framework — v0.8.9

## Purpose

The reliability evaluator is a deterministic, bounded harness for exercising the actual AI-agent adapter against controlled adversarial and failure scenarios. It is not an LLM benchmark leaderboard and does not claim that passing a small scenario suite proves model correctness. The harness produces evidence for release gates and exposes missing coverage.

## Required scenario categories

- `contract`: malformed or out-of-contract inputs and outputs.
- `provider_failure`: timeout, unavailable provider, throttling, and malformed transport responses.
- `prompt_injection`: attempts to override system constraints or inject unsupported instructions.
- `evidence_integrity`: changed fingerprints, unknown references, or evidence scope mismatch.
- `determinism`: repeated evaluation of identical inputs.
- `resource_limit`: oversized output/input, excessive latency, and bounded-resource behavior.

The default policy requires every category. Missing categories block the policy even if every supplied scenario passes. Use `requiredCategories` only for explicitly scoped development runs; release CI should use the default complete policy.

## Scenario and adapter contract

Each scenario has a unique bounded ID, one supported category, a JSON-like `input`, and an expected outcome (`accepted`, `rejected`, or `degraded`). The evaluator invokes the real adapter with an `AbortSignal`, scenario ID, and attempt number. Adapter implementations must also enforce their own HTTP/RPC deadlines because JavaScript cannot forcibly terminate a promise that ignores cancellation.

Every observed adapter output must be a plain object with `executionAuthority: "none"`, `liveExecutionAuthorized: false`, and `orderIntent: null`. Accepted outputs must provide a 64-character lowercase hexadecimal `decisionFingerprint`. This fingerprint must be produced by the deterministic decision/evidence layer, not by model-authored prose.

Expected provider errors must use stable, sanitized error codes. Raw error messages are never copied into the report. The evaluator hashes canonical input and output representations, checks repeat-run determinism, and optionally checks equivalence groups where semantically equivalent scenarios are expected to preserve a decision fingerprint.

## Resource and data safety

- Maximum 128 scenarios per evaluation.
- Maximum 10 seconds per scenario attempt (default 3 seconds).
- Maximum five repeated attempts per scenario.
- Maximum 64 KiB per canonical input/output object.
- Maximum nesting depth 24.
- Rejects accessors, cycles, sparse arrays, non-finite numbers, unsupported prototypes, and prototype-pollution property names.
- Output report includes no API key, provider response body, or raw exception message.
- The report is hashed with Web Crypto SHA-256. This hash detects accidental changes; it is not a digital signature or proof of who created the report.

## Release policy

A policy passes only when all required categories are covered, every scenario passes, repeat runs agree, equivalence groups agree, and the configured pass rate is met. Release CI must execute this harness against the actual adapter with pinned fixtures and should archive the report alongside the build evidence.

Passing this harness does not establish profitability, model truthfulness, statistical evidence independence, or live-trading readiness. Live trading remains unauthorized and outside the harness.

## Minimal integration shape

```js
import { evaluateAgentReliability } from '../domain/agent/reliability-contract.js';

const report = await evaluateAgentReliability({
  scenarios: releaseScenarios, // reviewed, version-controlled fixtures
  invoke: (input, context) => actualAgentAdapter.run(input, context),
});
if (!report.summary.policyPassed) {
  throw new Error('AI Agent reliability release gate failed');
}
```

The example is an integration shape, not a standalone provider implementation. `releaseScenarios` must include adversarial/fault fixtures for all required categories and the adapter must return the fixed safety fields described above.
