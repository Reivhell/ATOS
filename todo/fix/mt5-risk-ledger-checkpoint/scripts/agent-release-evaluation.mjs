#!/usr/bin/env node
/**
 * Deterministic release-quality gate for the real JavaScript agent-report boundary.
 * Provider failure is simulated; this suite intentionally requires no credentials or network.
 * It complements (never replaces) real-provider integration tests in CI.
 */
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { evaluateAgentResearchReport } from '../src/lib/application/ai-agent-orchestrator.js';
import { evaluateAgentReliability } from '../src/lib/domain/agent/reliability-contract.js';

if (!globalThis.crypto?.subtle) {
  Object.defineProperty(globalThis, 'crypto', { configurable: true, value: crypto.webcrypto });
}

const DOMAINS = Object.freeze(['trend', 'momentum', 'structure', 'location']);
const AS_OF = new Date(Date.now() - 1_000).toISOString();
const DATASET_SHA256 = 'a'.repeat(64);
const DECISION_NOTE = 'LLM text is commentary only. The separate deterministic decision evaluation uses Rust-derived evidence; no order intent or execution permission is emitted.';
const SHA256_RE = /^[a-f0-9]{64}$/;
const EXPECTED_ERRORS = Object.freeze({
  'case-contract-invalid-provider': 'AGENT_PROVIDER_CONTRACT_REJECTED',
  'case-provider-failure-timeout': 'PROVIDER_TIMEOUT',
  'case-prompt-injection-authority': 'AGENT_AUTHORITY_INVARIANT_REJECTED',
  'case-prompt-injection-order-intent': 'AGENT_ORDER_INTENT_REJECTED',
  'case-evidence-integrity-tamper': 'AGENT_EVIDENCE_INTEGRITY_REJECTED',
  'case-resource-limit-summary': 'AGENT_RESOURCE_BOUND_REJECTED',
});

function sha256(text) {
  return crypto.createHash('sha256').update(text, 'utf8').digest('hex');
}

function scoringEnvelope(items) {
  return DOMAINS.map((domain) => {
    const item = items.find((entry) => entry.domain === domain);
    return [item.ref, item.domain, item.direction, Math.round(item.signalStrength * 1_000_000), item.source,
      item.observedAt, item.datasetSha256, item.symbol, item.timeframe, item.payloadSha256];
  });
}

function canonicalPayloadText(value) {
  if (Array.isArray(value)) return `[${value.map(canonicalPayloadText).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalPayloadText(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

function reportCommentaryDigest(report) {
  return sha256(JSON.stringify({
    summary: report.summary,
    observations: report.observations.map((item) => ({
      domain: item.domain,
      assessment: item.assessment,
      evidenceRefs: item.evidenceRefs,
      caveats: item.caveats,
    })),
    disagreements: report.disagreements,
    followUpChecks: report.followUpChecks,
  }));
}

/** Build a native-boundary fixture; no live market, provider, or broker calls are made. */
function reportFixture(overrides = {}) {
  const evidenceBundle = DOMAINS.map((domain, index) => {
    const payload = {
      domain, direction: domain === 'momentum' ? 'neutral' : 'bullish',
      signalStrength: domain === 'momentum' ? 0.25 : 0.8, metric: 100 + index,
    };
    const payloadCanonicalJson = canonicalPayloadText(payload);
    return {
      ref: `native:${domain}`, domain, direction: payload.direction, signalStrength: payload.signalStrength,
      source: 'rust-native-derived-evidence.v1', observedAt: AS_OF, datasetSha256: DATASET_SHA256,
      symbol: 'BTCUSDT', timeframe: '5m', payload, payloadCanonicalJson, payloadSha256: sha256(payloadCanonicalJson),
    };
  });
  const report = {
    contractVersion: 'agent-research-runtime.v2',
    runId: '123e4567-e89b-42d3-a456-426614174000',
    providerId: 'openai',
    model: 'release-evaluation-fixture',
    generatedAt: AS_OF,
    marketContext: {
      symbol: 'BTCUSDT',
      timeframe: '5m',
      asOf: AS_OF,
      latestClosedBarAtMs: Date.parse(AS_OF),
      retrievedAtMs: Date.parse(AS_OF) + 100,
      datasetSha256: DATASET_SHA256,
      source: 'Deterministic release fixture',
      providerStatus: 'CONNECTED · test fixture only',
      isLive: true,
      isFresh: true,
      candleCount: 120,
      freshnessLimitMs: 630000,
    },
    evidenceBundle,
    summary: 'Read-only deterministic release fixture.',
    observations: DOMAINS.map((domain) => ({
      domain,
      assessment: `Bounded context for ${domain}.`,
      evidenceRefs: [`native:${domain}`],
      caveats: ['Synthetic fixture; not a trading signal.'],
    })),
    disagreements: [],
    followUpChecks: ['Verify on held-out data before any deployment decision.'],
    outputSha256: '',
    evidenceEnvelopeSha256: sha256(JSON.stringify(scoringEnvelope(evidenceBundle))),
    executionAuthority: 'none',
    liveExecutionAuthorized: false,
    orderIntent: null,
    decisionNote: DECISION_NOTE,
  };
  Object.assign(report, overrides);
  // The normal fixtures are signed with their recomputed digest. Tests explicitly tamper after this boundary.
  report.outputSha256 = reportCommentaryDigest(report);
  return report;
}

function makeScenarios() {
  const valid = reportFixture();
  const narrativeVariant = reportFixture({
    summary: 'Different wording, unchanged deterministic evidence.',
    observations: DOMAINS.map((domain) => ({
      domain,
      assessment: `Alternative prose for ${domain}; evidence and scores are untouched.`,
      evidenceRefs: [`native:${domain}`],
      caveats: [],
    })),
  });
  const permuted = reportFixture();
  permuted.evidenceBundle.reverse();
  const invalidProvider = reportFixture({ providerId: 'unregistered-provider' });
  const authorityEscalation = reportFixture({ executionAuthority: 'live', liveExecutionAuthorized: true, orderIntent: { symbol: 'BTCUSDT', side: 'buy' } });
  const injectedOrderIntent = reportFixture({ orderIntent: { symbol: 'BTCUSDT', side: 'buy', quantity: 1 }, executionAuthority: 'none', liveExecutionAuthorized: false });
  const evidenceTamper = reportFixture();
  evidenceTamper.evidenceBundle[1].signalStrength = 0.99;
  evidenceTamper.evidenceBundle[1].payload.signalStrength = 0.99;
  const evidencePayloadTamper = reportFixture();
  evidencePayloadTamper.evidenceBundle[2].payload.metric = 999999;
  const oversizedSummary = reportFixture({ summary: 'x'.repeat(1801) });
  return [
    { id: 'case-contract-invalid-provider', category: 'contract', input: { kind: 'report', report: invalidProvider }, expected: { outcome: 'rejected', errorCode: EXPECTED_ERRORS['case-contract-invalid-provider'] } },
    { id: 'case-provider-failure-timeout', category: 'provider_failure', input: { kind: 'provider_fault', fault: 'timeout' }, expected: { outcome: 'rejected', errorCode: EXPECTED_ERRORS['case-provider-failure-timeout'] } },
    { id: 'case-prompt-injection-authority', category: 'prompt_injection', input: { kind: 'report', report: authorityEscalation }, expected: { outcome: 'rejected', errorCode: EXPECTED_ERRORS['case-prompt-injection-authority'] } },
    { id: 'case-prompt-injection-order-intent', category: 'prompt_injection', input: { kind: 'report', report: injectedOrderIntent }, expected: { outcome: 'rejected', errorCode: EXPECTED_ERRORS['case-prompt-injection-order-intent'] } },
    { id: 'case-evidence-integrity-tamper', category: 'evidence_integrity', input: { kind: 'report', report: evidenceTamper }, expected: { outcome: 'rejected', errorCode: EXPECTED_ERRORS['case-evidence-integrity-tamper'] } },
    { id: 'case-evidence-payload-tamper', category: 'evidence_integrity', input: { kind: 'report', report: evidencePayloadTamper }, expected: { outcome: 'rejected', errorCode: 'AGENT_EVIDENCE_INTEGRITY_REJECTED' } },
    { id: 'case-determinism-baseline', category: 'determinism', input: { kind: 'report', report: valid }, expected: { outcome: 'accepted' }, repeatGroup: 'same-native-snapshot', repeatCount: 3 },
    { id: 'case-determinism-narrative-metamorphism', category: 'determinism', input: { kind: 'report', report: narrativeVariant }, expected: { outcome: 'accepted' }, repeatGroup: 'same-native-snapshot' },
    { id: 'case-determinism-evidence-order', category: 'determinism', input: { kind: 'report', report: permuted }, expected: { outcome: 'accepted' }, repeatGroup: 'same-native-snapshot' },
    { id: 'case-resource-limit-summary', category: 'resource_limit', input: { kind: 'report', report: oversizedSummary }, expected: { outcome: 'rejected', errorCode: EXPECTED_ERRORS['case-resource-limit-summary'] } },
  ];
}

function normalizeDecisionForFingerprint(value) {
  if (Array.isArray(value)) {
    return value.map(normalizeDecisionForFingerprint)
      .sort((left, right) => {
        const a = JSON.stringify(left);
        const b = JSON.stringify(right);
        return a < b ? -1 : a > b ? 1 : 0;
      });
  }
  if (value !== null && typeof value === 'object') {
    const normalized = Object.create(null);
    for (const key of Object.keys(value).sort()) normalized[key] = normalizeDecisionForFingerprint(value[key]);
    return normalized;
  }
  return value;
}

function rejectionCode(scenarioId, error) {
  const message = error instanceof Error ? error.message : '';
  if (scenarioId === 'case-provider-failure-timeout') return 'PROVIDER_TIMEOUT';
  if (/provider identity is unsupported/i.test(message)) return 'AGENT_PROVIDER_CONTRACT_REJECTED';
  if (/cannot grant execution authority|Live execution authorization/i.test(message)) return 'AGENT_AUTHORITY_INVARIANT_REJECTED';
  if (/cannot include an order intent/i.test(message)) return 'AGENT_ORDER_INTENT_REJECTED';
  if (/digest mismatch|evidence changed after the Rust boundary|payload changed after the Rust boundary/i.test(message)) return 'AGENT_EVIDENCE_INTEGRITY_REJECTED';
  if (/strict content contract|maximum nesting depth|byte limit/i.test(message)) return 'AGENT_RESOURCE_BOUND_REJECTED';
  return 'AGENT_REPORT_REJECTED';
}

async function invokeApplicationBoundary(input, context) {
  if (context.signal.aborted) {
    const aborted = new Error('Evaluation adapter was aborted.');
    aborted.code = 'AGENT_EVALUATION_ABORTED';
    throw aborted;
  }
  if (input.kind === 'provider_fault') {
    const failure = new Error('Provider transport failed; detail intentionally redacted.');
    failure.code = input.fault === 'timeout' ? 'PROVIDER_TIMEOUT' : 'PROVIDER_UNAVAILABLE';
    throw failure;
  }
  try {
    const result = await evaluateAgentResearchReport(input.report);
    // Fingerprint only the deterministic decision result and evidence registry. Model-authored prose,
    // provider latency, and generated timestamps are intentionally excluded from the decision digest.
    const decisionFingerprint = sha256(JSON.stringify({
      decision: normalizeDecisionForFingerprint(result.decision),
      registrySha256: result.registrySha256,
      registryRecordCount: result.registryRecordCount,
      decisionInputSource: result.decisionInputSource,
    }));
    if (!SHA256_RE.test(decisionFingerprint)) throw new Error('Decision fingerprint generation failed.');
    return {
      outcome: 'accepted',
      executionAuthority: 'none',
      liveExecutionAuthorized: false,
      orderIntent: null,
      decisionFingerprint,
    };
  } catch (error) {
    const rejected = new Error('Agent report was rejected by the application boundary.');
    rejected.code = rejectionCode(context.scenarioId, error);
    throw rejected;
  }
}

export async function runAgentReleaseEvaluation({ reportFile = null } = {}) {
  const report = await evaluateAgentReliability({
    scenarios: makeScenarios(),
    invoke: invokeApplicationBoundary,
    policy: { timeoutMs: 3_000, minPassRate: 1 },
  });
  const envelope = {
    ...report,
    evaluationMode: 'DETERMINISTIC_APPLICATION_BOUNDARY_WITH_SIMULATED_PROVIDER_FAILURE',
    providerLiveIntegrationTested: false,
    executionAuthority: 'none',
    liveExecutionAuthorized: false,
    orderIntent: null,
  };
  // The evaluator digest covers its own canonical report. Add explicit scope metadata in a second digest.
  envelope.releaseEnvelopeSha256 = sha256(JSON.stringify(envelope));
  if (reportFile) {
    const absolute = path.resolve(reportFile);
    await fs.mkdir(path.dirname(absolute), { recursive: true });
    await fs.writeFile(absolute, `${JSON.stringify(envelope, null, 2)}\n`, { encoding: 'utf8', flag: 'w', mode: 0o600 });
  }
  return envelope;
}

function parseArgs(argv) {
  const args = [...argv];
  let reportFile = null;
  while (args.length) {
    const arg = args.shift();
    if (arg === '--report-file') {
      reportFile = args.shift();
      if (!reportFile) throw new Error('--report-file requires a path.');
    } else if (arg === '--help' || arg === '-h') {
      return { help: true, reportFile: null };
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return { help: false, reportFile };
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : '';
if (invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const args = parseArgs(process.argv.slice(2));
    if (args.help) {
      process.stdout.write('Usage: node scripts/agent-release-evaluation.mjs [--report-file <path>]\n');
    } else {
      const report = await runAgentReleaseEvaluation({ reportFile: args.reportFile });
      process.stdout.write(`${JSON.stringify({
        schemaVersion: report.schemaVersion,
        scenarioCount: report.summary.scenarioCount,
        passedCount: report.summary.passedCount,
        failedCount: report.summary.failedCount,
        categoryGaps: report.summary.categoryGaps,
        policyPassed: report.summary.policyPassed,
        reportSha256: report.reportSha256,
        releaseEnvelopeSha256: report.releaseEnvelopeSha256,
        providerLiveIntegrationTested: report.providerLiveIntegrationTested,
        reportFile: args.reportFile ? path.resolve(args.reportFile) : null,
      }, null, 2)}\n`);
      if (!report.summary.policyPassed) process.exitCode = 1;
    }
  } catch (error) {
    process.stderr.write(`Agent release evaluation failed closed: ${error instanceof Error ? error.message : 'UNKNOWN_ERROR'}\n`);
    process.exitCode = 1;
  }
}
