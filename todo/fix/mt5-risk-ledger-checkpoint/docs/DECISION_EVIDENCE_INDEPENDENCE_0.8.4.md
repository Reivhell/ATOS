# Decision-layer evidence independence — 0.8.4

## Problem

The research decision engine already limited each registered technical domain to one signal, but it did not prevent two different domains from citing the same evidence reference. A caller could therefore reuse one underlying observation as if it were independent trend, momentum, structure, and location confirmation.

## Controls

- A reference can be counted by at most one accepted domain within one decision evaluation. Reuse across domains causes a fail-closed `BLOCK`.
- Signal arrays are capped at 16 items before per-item scoring.
- Each signal must provide 1–32 evidence references.
- Each reference is capped at 256 characters.
- The engine remains deterministic, binds every accepted signal to the canonical timestamp and dataset SHA-256, and never emits order intent or execution authority.
- The public contract and policy identifiers are now `research-decision.v2` and `research-decision-policy.v2` to make the tightened contract explicit.

## Regression coverage

The decision-engine suite checks evidence reuse across domains, oversized signal arrays, too many evidence references, and overlong references in addition to existing provenance, duplicate-domain, consensus, paper-risk, and live-intent tests.

## Limitations

Evidence-reference uniqueness is a necessary integrity guard, not proof that evidence is statistically independent. Reference IDs must be generated from canonical source observations by trusted code; callers can still mislabel distinct observations unless upstream provenance is enforced. This layer is research-only and does not grant order or live-execution authority. Full production certification still requires actual frontend/native builds and runtime acceptance.
