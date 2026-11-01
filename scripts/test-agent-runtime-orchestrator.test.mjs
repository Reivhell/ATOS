import test from 'node:test';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { evaluateAgentResearchReport } from '../src/lib/application/ai-agent-orchestrator.js';

if (!globalThis.crypto?.subtle) Object.defineProperty(globalThis, 'crypto', { configurable: true, value: webcrypto });

const AS_OF = new Date(Date.now() - 1_000).toISOString();
const LATEST_CLOSED_BAR_AT_MS = Date.parse(AS_OF);
const RETRIEVED_AT_MS = LATEST_CLOSED_BAR_AT_MS + 100;
const DATASET = 'a'.repeat(64);
const DOMAINS = ['trend', 'momentum', 'structure', 'location'];

async function hash(value) {
  const bytes = new TextEncoder().encode(JSON.stringify(value));
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
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

async function hashText(text) {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

async function reportFixture(overrides = {}) {
  const evidenceBundle = await Promise.all(DOMAINS.map(async (domain, index) => {
    const payload = { domain, direction: 'bullish', signalStrength: 0.8, metric: 100 + index };
    const payloadCanonicalJson = canonicalPayloadText(payload);
    return {
      ref: `native:${domain}`, domain, direction: 'bullish', signalStrength: 0.8,
      source: 'rust-native-derived-evidence.v1', observedAt: AS_OF, datasetSha256: DATASET,
      symbol: 'BTCUSDT', timeframe: '5m', payload, payloadCanonicalJson, payloadSha256: await hashText(payloadCanonicalJson),
    };
  }));
  const report = {
    contractVersion: 'agent-research-runtime.v2',
    runId: '123e4567-e89b-42d3-a456-426614174000',
    providerId: 'openai',
    model: 'test-model',
    generatedAt: AS_OF,
    marketContext: {
      symbol: 'BTCUSDT', timeframe: '5m', asOf: AS_OF, latestClosedBarAtMs: LATEST_CLOSED_BAR_AT_MS, retrievedAtMs: RETRIEVED_AT_MS, datasetSha256: DATASET,
      source: 'Exchange public spot candles', providerStatus: 'CONNECTED · public spot feed',
      isLive: true, isFresh: true, candleCount: 120, freshnessLimitMs: 630000,
    },
    evidenceBundle,
    summary: 'Read-only context summary.',
    observations: DOMAINS.map((domain) => ({ domain, assessment: `Context for ${domain}.`, evidenceRefs: [`native:${domain}`], caveats: ['Heuristic, not probability.'] })),
    disagreements: [],
    followUpChecks: ['Validate on out-of-sample data.'],
    outputSha256: 'b'.repeat(64),
    evidenceEnvelopeSha256: await hash(scoringEnvelope(evidenceBundle)),
    executionAuthority: 'none',
    liveExecutionAuthorized: false,
    decisionNote: 'LLM text is commentary only. The separate deterministic decision evaluation uses Rust-derived evidence; no order intent or execution permission is emitted.',
    ...overrides,
  };
  if (!Object.hasOwn(overrides, 'outputSha256')) {
    report.outputSha256 = await hash({
      summary: report.summary,
      observations: report.observations.map((item) => ({ domain: item.domain, assessment: item.assessment, evidenceRefs: item.evidenceRefs, caveats: item.caveats })),
      disagreements: report.disagreements,
      followUpChecks: report.followUpChecks,
    });
  }
  return report;
}

test('native envelope yields a reproducible research candidate without creating an order intent', async () => {
  const report = await reportFixture();
  const result = await evaluateAgentResearchReport(report);
  assert.equal(result.decision.decision, 'RESEARCH_LONG_CANDIDATE');
  assert.equal(result.registryRecordCount, 4);
  assert.match(result.registrySha256, /^[a-f0-9]{64}$/);
  assert.equal(result.executionAuthority, 'none');
  assert.equal(result.liveExecutionAuthorized, false);
  assert.equal(result.orderIntent, null);
  assert.equal(result.aiNarrativeExcludedFromScoring, true);
});

test('model-authored narrative cannot change deterministic score or canonical registry digest', async () => {
  const first = await reportFixture();
  const second = await reportFixture({
    summary: 'A completely different LLM narrative.',
    observations: DOMAINS.map((domain) => ({ domain, assessment: 'Different prose and no extra certainty.', evidenceRefs: [`native:${domain}`], caveats: [] })),
    disagreements: ['The model disagrees with a previous response.'],
  });
  const a = await evaluateAgentResearchReport(first);
  const b = await evaluateAgentResearchReport(second);
  assert.equal(a.decision.decision, b.decision.decision);
  assert.equal(a.decision.bullishScore, b.decision.bullishScore);
  assert.equal(a.registrySha256, b.registrySha256);
});

test('altering score-bearing fields or display-only payload after the native boundary fails integrity verification', async (t) => {
  await t.test('score-bearing evidence', async () => {
    const report = await reportFixture();
    report.evidenceBundle[1].signalStrength = 1;
    report.evidenceBundle[1].payload.signalStrength = 1;
    await assert.rejects(() => evaluateAgentResearchReport(report), /payload digest mismatch|scoring envelope digest mismatch/);
  });
  await t.test('display-only payload metric', async () => {
    const report = await reportFixture();
    report.evidenceBundle[2].payload.metric = 999_999;
    await assert.rejects(() => evaluateAgentResearchReport(report), /payload digest mismatch/);
  });
  await t.test('canonical payload and digest substitution', async () => {
    const report = await reportFixture();
    const changed = { ...report.evidenceBundle[0].payload, metric: 777 };
    report.evidenceBundle[0].payloadCanonicalJson = canonicalPayloadText(changed);
    await assert.rejects(() => evaluateAgentResearchReport(report), /payload digest mismatch/);
  });
});

test('stale or fixture market data fails closed even when all four domains agree', async () => {
  const report = await reportFixture({ marketContext: {
    symbol: 'BTCUSDT', timeframe: '5m', asOf: AS_OF, latestClosedBarAtMs: LATEST_CLOSED_BAR_AT_MS, retrievedAtMs: RETRIEVED_AT_MS, datasetSha256: DATASET,
    source: 'Fixture data', providerStatus: 'STALE', isLive: false, isFresh: false,
    candleCount: 120, freshnessLimitMs: 630000,
  } });
  const result = await evaluateAgentResearchReport(report);
  assert.equal(result.decision.decision, 'BLOCK');
  assert.equal(result.executionAuthority, 'none');
});

test('duplicate domains, wrong aliases, malformed payload binding and execution authority are rejected', async (t) => {
  await t.test('duplicate domain', async () => {
    const report = await reportFixture();
    report.evidenceBundle[3].domain = 'trend';
    report.evidenceBundle[3].ref = 'native:trend';
    report.evidenceBundle[3].payload.domain = 'trend';
    report.evidenceBundle[3].payloadCanonicalJson = canonicalPayloadText(report.evidenceBundle[3].payload);
    report.evidenceBundle[3].payloadSha256 = await hashText(report.evidenceBundle[3].payloadCanonicalJson);
    await assert.rejects(() => evaluateAgentResearchReport(report), /duplicate or unregistered domain/);
  });
  await t.test('wrong native alias', async () => {
    const report = await reportFixture();
    report.evidenceBundle[0].ref = 'model:trend';
    await assert.rejects(() => evaluateAgentResearchReport(report), /reference does not match/);
  });
  await t.test('payload mismatch', async () => {
    const report = await reportFixture();
    report.evidenceBundle[0].payload.direction = 'bearish';
    await assert.rejects(() => evaluateAgentResearchReport(report), /payload changed after the Rust boundary|payload fields do not match/);
  });
  await t.test('authority override', async () => {
    const report = await reportFixture({ executionAuthority: 'live' });
    await assert.rejects(() => evaluateAgentResearchReport(report), /cannot grant execution authority/);
  });
  await t.test('injected order intent', async () => {
    const report = await reportFixture({ orderIntent: { symbol: 'BTCUSDT', side: 'buy', quantity: 1 } });
    await assert.rejects(() => evaluateAgentResearchReport(report), /cannot include an order intent/);
  });
});

test('commentary is bounded and domain references cannot be cross-wired', async (t) => {
  await t.test('oversized summary rejected', async () => {
    const report = await reportFixture({ summary: 'x'.repeat(1801) });
    await assert.rejects(() => evaluateAgentResearchReport(report), /summary exceeds/);
  });
  await t.test('cross-domain commentary reference rejected', async () => {
    const base = await reportFixture();
    const observations = base.observations.map((item) => ({ ...item, evidenceRefs: [...item.evidenceRefs] }));
    observations[0].evidenceRefs[0] = 'native:momentum';
    const report = { ...base, observations };
    report.outputSha256 = await hash({ summary: report.summary, observations, disagreements: report.disagreements, followUpChecks: report.followUpChecks });
    await assert.rejects(() => evaluateAgentResearchReport(report), /cites evidence outside its assigned domain/);
  });
  await t.test('modified narrative rejected by native output digest', async () => {
    const base = await reportFixture();
    const report = { ...base, summary: 'Edited after validation' };
    await assert.rejects(() => evaluateAgentResearchReport(report), /commentary digest mismatch/);
  });
  await t.test('unknown provider rejected', async () => {
    const report = await reportFixture({ providerId: 'arbitrary-provider' });
    await assert.rejects(() => evaluateAgentResearchReport(report), /provider identity is unsupported/);
  });
  await t.test('malformed generation timestamp rejected', async () => {
    const report = await reportFixture({ generatedAt: 'yesterday' });
    await assert.rejects(() => evaluateAgentResearchReport(report), /generation timestamp is malformed/);
  });
  await t.test('fixed safety note cannot be overridden', async () => {
    const report = await reportFixture({ decisionNote: 'Live trading approved.' });
    await assert.rejects(() => evaluateAgentResearchReport(report), /fixed safety contract/);
  });
});

 test('market context close and receipt provenance must remain bound and fresh', async (t) => {
  await t.test('missing close timestamp is rejected', async () => {
    const report = await reportFixture();
    delete report.marketContext.latestClosedBarAtMs;
    await assert.rejects(() => evaluateAgentResearchReport(report), /latest-closed-bar timestamp/);
  });
  await t.test('cutoff substitution is rejected', async () => {
    const report = await reportFixture();
    report.marketContext.latestClosedBarAtMs += 1;
    await assert.rejects(() => evaluateAgentResearchReport(report), /does not match the native latest-closed-bar timestamp/);
  });
  await t.test('receipt before the candle close is rejected', async () => {
    const report = await reportFixture();
    report.marketContext.retrievedAtMs = report.marketContext.latestClosedBarAtMs - 1;
    await assert.rejects(() => evaluateAgentResearchReport(report), /receipt timestamp is malformed/);
  });
  await t.test('future receipt is blocked', async () => {
    const report = await reportFixture();
    report.marketContext.retrievedAtMs = Date.now() + 5_000;
    const result = await evaluateAgentResearchReport(report);
    assert.equal(result.decision.decision, 'BLOCK');
  });
  await t.test('closed candle that expires during processing is blocked', async () => {
    const report = await reportFixture();
    report.marketContext.latestClosedBarAtMs = Date.now() - report.marketContext.freshnessLimitMs - 1;
    report.marketContext.asOf = new Date(report.marketContext.latestClosedBarAtMs).toISOString();
    report.marketContext.retrievedAtMs = Date.now();
    for (const item of report.evidenceBundle) item.observedAt = report.marketContext.asOf;
    report.evidenceEnvelopeSha256 = await hash(scoringEnvelope(report.evidenceBundle));
    const result = await evaluateAgentResearchReport(report);
    assert.equal(result.decision.decision, 'BLOCK');
  });
  await t.test('receipt older than the native retrieval bound is blocked', async () => {
    const report = await reportFixture();
    report.marketContext.retrievedAtMs = Date.now() - 120_001;
    report.marketContext.latestClosedBarAtMs = report.marketContext.retrievedAtMs - 100;
    report.marketContext.asOf = new Date(report.marketContext.latestClosedBarAtMs).toISOString();
    for (const item of report.evidenceBundle) item.observedAt = report.marketContext.asOf;
    report.evidenceEnvelopeSha256 = await hash(scoringEnvelope(report.evidenceBundle));
    const result = await evaluateAgentResearchReport(report);
    assert.equal(result.decision.decision, 'BLOCK');
  });
});

test('reordering provider evidence does not change canonical evaluation', async () => {
  const report = await reportFixture();
  const expected = await evaluateAgentResearchReport(report);
  report.evidenceBundle.reverse();
  const actual = await evaluateAgentResearchReport(report);
  assert.equal(actual.decision.decision, expected.decision.decision);
  assert.equal(actual.registrySha256, expected.registrySha256);
});
