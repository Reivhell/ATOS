import test from 'node:test';
import assert from 'node:assert/strict';
import os from 'node:os';
import crypto from 'node:crypto';
import path from 'node:path';
import fs from 'node:fs/promises';
import { runAgentReleaseEvaluation } from './agent-release-evaluation.mjs';

test('release gate exercises the real application boundary and all adversarial categories', async () => {
  const report = await runAgentReleaseEvaluation();
  assert.equal(report.schemaVersion, 'agent-reliability-evaluation.v2');
  assert.equal(report.summary.policyPassed, true);
  assert.equal(report.summary.scenarioCount, 10);
  assert.equal(report.summary.failedCount, 0);
  assert.equal(report.summary.haltedAfterTimeout, false);
  assert.deepEqual(report.summary.categoryGaps, []);
  assert.match(report.reportSha256, /^[a-f0-9]{64}$/);
  assert.match(report.releaseEnvelopeSha256, /^[a-f0-9]{64}$/);
  const { releaseEnvelopeSha256, ...unsignedEnvelope } = report;
  assert.equal(crypto.createHash('sha256').update(JSON.stringify(unsignedEnvelope), 'utf8').digest('hex'), releaseEnvelopeSha256);
  assert.equal(report.providerLiveIntegrationTested, false);
  assert.equal(report.executionAuthority, 'none');
  assert.equal(report.liveExecutionAuthorized, false);
  assert.equal(report.orderIntent, null);

  for (const [id, errorCode] of Object.entries({
    'case-contract-invalid-provider': 'AGENT_PROVIDER_CONTRACT_REJECTED',
    'case-provider-failure-timeout': 'PROVIDER_TIMEOUT',
    'case-prompt-injection-authority': 'AGENT_AUTHORITY_INVARIANT_REJECTED',
    'case-prompt-injection-order-intent': 'AGENT_ORDER_INTENT_REJECTED',
    'case-evidence-integrity-tamper': 'AGENT_EVIDENCE_INTEGRITY_REJECTED',
    'case-evidence-payload-tamper': 'AGENT_EVIDENCE_INTEGRITY_REJECTED',
    'case-resource-limit-summary': 'AGENT_RESOURCE_BOUND_REJECTED',
  })) {
    const result = report.results.find((item) => item.scenarioId === id);
    assert.equal(result?.status, 'pass', `${id} must pass by observing the expected rejection`);
    assert.equal(result?.errorCode, errorCode);
  }
});

test('same snapshot stays decision-equivalent across prose changes and evidence ordering', async () => {
  const report = await runAgentReleaseEvaluation();
  const equivalent = report.results.filter((item) => item.scenarioId.startsWith('case-determinism-'));
  assert.equal(equivalent.length, 3);
  assert.ok(equivalent.every((item) => item.status === 'pass'));
  assert.equal(report.summary.equivalenceFailures.length, 0);
  assert.equal(new Set(equivalent.map((item) => item.decisionFingerprint)).size, 1);
});

test('release gate writes a restricted machine-readable report when requested', async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'agent-release-eval-'));
  t.after(async () => fs.rm(dir, { recursive: true, force: true }));
  const file = path.join(dir, 'nested', 'report.json');
  const report = await runAgentReleaseEvaluation({ reportFile: file });
  const parsed = JSON.parse(await fs.readFile(file, 'utf8'));
  assert.equal(parsed.releaseEnvelopeSha256, report.releaseEnvelopeSha256);
  const { releaseEnvelopeSha256, ...unsignedEnvelope } = parsed;
  assert.equal(crypto.createHash('sha256').update(JSON.stringify(unsignedEnvelope), 'utf8').digest('hex'), releaseEnvelopeSha256);
  assert.equal(parsed.summary.policyPassed, true);
  const stat = await fs.stat(file);
  if (process.platform !== 'win32') assert.equal(stat.mode & 0o077, 0);
});
