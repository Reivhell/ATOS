# AI Agent Release Qualification — v0.9.0

## Purpose

The release qualification command connects the reliability evaluator to the actual JavaScript application boundary (`evaluateAgentResearchReport`) using deterministic fixtures. It provides a repeatable offline quality gate; it does not require or accept provider credentials and does not call a live provider, exchange, or broker.

## Scenarios

The suite covers:

- unsupported provider metadata rejected by the report contract;
- provider timeout represented by a stable, sanitized adapter error code;
- attempted authority escalation and injected order intent rejected by the application boundary;
- evidence-envelope tampering rejected after the fixture integrity boundary;
- repeated runs produce stable decision fingerprints;
- changing commentary without changing evidence does not change the decision fingerprint;
- permuting equivalent evidence entries does not change the semantic decision fingerprint;
- over-limit commentary rejected before scoring.

The evaluator requires full category coverage and a 100% pass rate for this release command. The report records `providerLiveIntegrationTested: false`, execution authority `none`, live execution disabled, and null order intent. CI publishes the machine-readable report as an artifact.

## Timeout policy

A per-scenario timeout now terminates the evaluation campaign rather than retrying a potentially uncooperative adapter or starting additional scenarios. The current scenario fails with `AGENT_EVALUATION_TIMEOUT`; remaining scenarios are marked `blocked` with `EVALUATION_HALTED_AFTER_TIMEOUT`. This prevents the evaluator itself from deliberately accumulating background provider work after a timeout. It still cannot forcibly kill a promise that ignores `AbortSignal`; a real provider adapter must enforce a native transport deadline as well.

## Interpretation

A passing offline suite proves only that the JavaScript application boundary satisfies these fixtures. It is not a provider certification, statistical model-quality assessment, trading profitability claim, production certification, or signed attestation. Real provider transport, OS keychain, frontend/native builds, cross-platform packaging, and market-data acceptance tests remain separate release gates.
