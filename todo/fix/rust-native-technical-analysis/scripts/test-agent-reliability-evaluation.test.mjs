import test from 'node:test';
import assert from 'node:assert/strict';
import {
  AGENT_RELIABILITY_SCHEMA,
  canonicalizeAgentEvaluationValue,
  evaluateAgentReliability,
} from '../src/lib/domain/agent/reliability-contract.js';

const FINGERPRINT = 'a'.repeat(64);
const CATEGORIES = ['contract', 'provider_failure', 'prompt_injection', 'evidence_integrity', 'determinism', 'resource_limit'];

function output(outcome = 'accepted', extra = {}) {
  return {
    outcome,
    executionAuthority: 'none',
    liveExecutionAuthorized: false,
    orderIntent: null,
    ...(outcome === 'accepted' ? { decisionFingerprint: FINGERPRINT } : {}),
    ...extra,
  };
}

function completeScenarios() {
  return CATEGORIES.map((category) => ({
    id: `case-${category}`,
    category,
    input: { category, prompt: 'bounded test input' },
    expected: { outcome: category === 'provider_failure' || category === 'prompt_injection' ? 'rejected' : 'accepted' },
    ...(category === 'determinism' ? { repeatCount: 3 } : {}),
  }));
}

test('canonicalization sorts object keys and normalizes negative zero', () => {
  assert.equal(canonicalizeAgentEvaluationValue({ z: -0, a: 1 }), '{"a":1,"z":0}');
});

test('canonicalization rejects cycles, accessors, sparse arrays, non-finite numbers and exotic prototypes', () => {
  const cyclic = {}; cyclic.self = cyclic;
  assert.throws(() => canonicalizeAgentEvaluationValue(cyclic), /cycle/);
  const accessor = {}; Object.defineProperty(accessor, 'secret', { enumerable: true, get() { throw new Error('should not execute'); } });
  assert.throws(() => canonicalizeAgentEvaluationValue(accessor), /accessor/);
  const sparse = []; sparse.length = 2; sparse[1] = 'x';
  assert.throws(() => canonicalizeAgentEvaluationValue(sparse), /sparse array/);
  assert.throws(() => canonicalizeAgentEvaluationValue({ n: Number.NaN }), /non-finite/);
  assert.throws(() => canonicalizeAgentEvaluationValue(new Date()), /prototype/);
});

test('reliability evaluation enforces full risk-category coverage and hashes report', async () => {
  const report = await evaluateAgentReliability({
    scenarios: completeScenarios(),
    invoke: async (_input, context) => context.scenarioId.includes('provider_failure') || context.scenarioId.includes('prompt_injection')
      ? output('rejected') : output('accepted'),
  });
  assert.equal(report.schemaVersion, AGENT_RELIABILITY_SCHEMA);
  assert.equal(report.summary.policyPassed, true);
  assert.equal(report.summary.failedCount, 0);
  assert.equal(report.summary.passRate, 1);
  assert.deepEqual(report.summary.categoryGaps, []);
  assert.match(report.reportSha256, /^[a-f0-9]{64}$/);
  assert.equal(report.summary.executionAuthority, 'none');
  assert.equal(report.summary.liveExecutionAuthorized, false);
  assert.equal(report.summary.orderIntent, null);
});

test('missing critical categories blocks policy even if all supplied scenarios pass', async () => {
  const report = await evaluateAgentReliability({
    scenarios: [{ id: 'only-contract', category: 'contract', input: {}, expected: { outcome: 'accepted' } }],
    invoke: async () => output(),
  });
  assert.equal(report.summary.policyPassed, false);
  assert.ok(report.summary.categoryGaps.includes('prompt_injection'));
});

test('agent output that attempts to grant authority or emit an order is rejected', async () => {
  const scenarios = completeScenarios();
  const report = await evaluateAgentReliability({
    scenarios,
    invoke: async (_input, context) => context.scenarioId === 'case-contract'
      ? output('accepted', { executionAuthority: 'live' })
      : (context.scenarioId.includes('provider_failure') || context.scenarioId.includes('prompt_injection') ? output('rejected') : output()),
  });
  assert.equal(report.summary.policyPassed, false);
  assert.equal(report.results.find((item) => item.scenarioId === 'case-contract').status, 'fail');
  assert.equal(report.results.find((item) => item.scenarioId === 'case-contract').errorCode, 'AGENT_EVALUATION_ERROR');
});

test('expected stable provider error is accepted as a fail-closed rejection', async () => {
  const scenarios = completeScenarios();
  scenarios.find((item) => item.category === 'provider_failure').expected.errorCode = 'PROVIDER_TIMEOUT';
  const report = await evaluateAgentReliability({
    scenarios,
    invoke: async (_input, context) => {
      if (context.scenarioId === 'case-provider_failure') { const error = new Error('private provider detail'); error.code = 'PROVIDER_TIMEOUT'; throw error; }
      if (context.scenarioId === 'case-prompt_injection') return output('rejected');
      return output();
    },
  });
  assert.equal(report.results.find((item) => item.scenarioId === 'case-provider_failure').status, 'pass');
  assert.equal(report.results.find((item) => item.scenarioId === 'case-provider_failure').errorCode, 'PROVIDER_TIMEOUT');
});

test('repeat scenarios detect nondeterministic output fingerprints', async () => {
  let calls = 0;
  const scenarios = completeScenarios();
  const report = await evaluateAgentReliability({
    scenarios,
    invoke: async (_input, context) => {
      if (context.scenarioId === 'case-provider_failure') return output('rejected');
      if (context.scenarioId === 'case-prompt_injection') return output('rejected');
      if (context.scenarioId === 'case-determinism') return output('accepted', { decisionFingerprint: String(++calls).padStart(64, '0') });
      return output();
    },
  });
  assert.equal(report.results.find((item) => item.scenarioId === 'case-determinism').status, 'fail');
  assert.equal(report.summary.policyPassed, false);
});

test('equivalence groups detect same-class cases that produce different decision fingerprints', async () => {
  const scenarios = completeScenarios();
  scenarios[0].repeatGroup = 'same-market-snapshot';
  scenarios[1].repeatGroup = 'same-market-snapshot';
  const report = await evaluateAgentReliability({
    scenarios,
    invoke: async (_input, context) => {
      if (context.scenarioId === 'case-provider_failure' || context.scenarioId === 'case-prompt_injection') return output('rejected');
      return output('accepted', { decisionFingerprint: context.scenarioId === 'case-contract' ? 'b'.repeat(64) : 'c'.repeat(64) });
    },
  });
  assert.equal(report.summary.equivalenceFailures.length, 1);
  assert.equal(report.summary.policyPassed, false);
});

test('timeouts abort the adapter signal and fail the evaluation closed', async () => {
  const scenarios = completeScenarios();
  let signalAborted = false;
  const report = await evaluateAgentReliability({
    scenarios,
    policy: { timeoutMs: 50, requiredCategories: [] },
    invoke: async (_input, context) => {
      if (context.scenarioId === 'case-contract') {
        await new Promise((resolve) => context.signal.addEventListener('abort', () => { signalAborted = true; resolve(); }, { once: true }));
        return output();
      }
      if (context.scenarioId === 'case-provider_failure' || context.scenarioId === 'case-prompt_injection') return output('rejected');
      return output();
    },
  });
  assert.equal(report.results.find((item) => item.scenarioId === 'case-contract').status, 'fail');
  assert.equal(report.results.find((item) => item.scenarioId === 'case-contract').errorCode, 'AGENT_EVALUATION_TIMEOUT');
  assert.equal(signalAborted, true);
  assert.equal(report.summary.policyPassed, false);
});

test('scenario count, duplicate IDs, and unsafe scenario fields are rejected before invocation', async () => {
  let calls = 0;
  await assert.rejects(() => evaluateAgentReliability({ scenarios: [], invoke: async () => { calls += 1; return output(); } }), /between 1 and 128/);
  const duplicate = completeScenarios(); duplicate[1].id = duplicate[0].id;
  await assert.rejects(() => evaluateAgentReliability({ scenarios: duplicate, invoke: async () => { calls += 1; return output(); } }), /Duplicate scenario ID/);
  await assert.rejects(() => evaluateAgentReliability({ scenarios: [{ id: 'unsafe', category: 'contract', input: { payload: 'x'.repeat(70_000) }, expected: { outcome: 'accepted' } }], invoke: async () => { calls += 1; return output(); } }), /byte limit/);
  const accessorScenario = { category: 'contract', input: {}, expected: { outcome: 'accepted' } };
  Object.defineProperty(accessorScenario, 'id', { enumerable: true, get() { throw new Error('must not execute'); } });
  await assert.rejects(() => evaluateAgentReliability({ scenarios: [accessorScenario], invoke: async () => { calls += 1; return output(); } }), /accessor/);
  assert.equal(calls, 0);
});

test('minimum pass-rate and policy bounds are strictly validated', async () => {
  await assert.rejects(() => evaluateAgentReliability({ scenarios: completeScenarios(), invoke: async () => output(), policy: { minPassRate: 0.5 } }), /between 0.90 and 1.00/);
  await assert.rejects(() => evaluateAgentReliability({ scenarios: completeScenarios(), invoke: async () => output(), policy: { timeoutMs: 20_000 } }), /between 50 and 10000/);
});

test('a deadline breach does not retry a hung adapter and halts later scenarios fail-closed', async () => {
  const scenarios = completeScenarios();
  scenarios[0].repeatCount = 5;
  let invocationCount = 0;
  const report = await evaluateAgentReliability({
    scenarios,
    policy: { timeoutMs: 50 },
    invoke: async (_input, context) => {
      invocationCount += 1;
      await new Promise((resolve) => context.signal.addEventListener('abort', resolve, { once: true }));
      // The adapter may settle after abort; this must not trigger another attempt.
      return output();
    },
  });
  assert.equal(invocationCount, 1);
  assert.equal(report.summary.haltedAfterTimeout, true);
  assert.equal(report.results[0].status, 'fail');
  assert.equal(report.results[0].errorCode, 'AGENT_EVALUATION_TIMEOUT');
  assert.ok(report.results.slice(1).every((result) => result.status === 'blocked'));
  assert.equal(report.summary.policyPassed, false);
});
