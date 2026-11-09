import test from 'node:test';
import assert from 'node:assert/strict';
import { evaluateResearchDecision } from '../src/lib/engines/decision/research-decision-engine.js';
import { createCanonicalEvidenceRegistry, resolveCanonicalEvidenceId } from '../src/lib/domain/decision/canonical-evidence-registry.js';

const asOf = '2026-10-27T06:00:00.000Z';
const datasetSha256 = 'a'.repeat(64);
const marketData = () => ({
  validated: true,
  source: 'fixture-provider',
  asOf,
  datasetSha256,
  symbol: 'BTCUSDT',
  timeframe: '5m',
});
const signal = (domain, direction = 'bullish', overrides = {}) => ({
  domain, direction, signalStrength: 0.72, asOf, datasetSha256,
  evidenceRefs: [`fixture:${domain}:bar-99`],
  ...overrides,
});
const consensus = () => [
  signal('trend'), signal('momentum'), signal('structure'), signal('location'),
];

async function evaluate(input) {
  const market = input.marketData ?? {};
  const rawSignals = Array.isArray(input.signals) ? input.signals : [];
  const aliases = new Map();
  for (const item of rawSignals) {
    if (!Array.isArray(item?.evidenceRefs)) continue;
    for (const ref of item.evidenceRefs) {
      if (typeof ref !== 'string' || !ref.trim() || ref.length > 256 || aliases.has(ref)) continue;
      aliases.set(ref, {
        ref,
        source: 'fixture-provider',
        observedAt: typeof item.asOf === 'string' ? item.asOf : market.asOf,
        datasetSha256: typeof market.datasetSha256 === 'string' ? market.datasetSha256.toLowerCase() : 'a'.repeat(64),
        symbol: typeof market.symbol === 'string' && /^[A-Z0-9_-]{1,30}$/.test(market.symbol) ? market.symbol : 'BTCUSDT',
        timeframe: typeof market.timeframe === 'string' && /^[A-Za-z0-9_-]{1,16}$/.test(market.timeframe) ? market.timeframe : '5m',
        payload: { sourceObservation: ref },
      });
    }
  }
  const evidenceRegistry = aliases.size ? await createCanonicalEvidenceRegistry([...aliases.values()]) : undefined;
  const signals = rawSignals.map((item) => ({
    ...item,
    evidenceRefs: Array.isArray(item?.evidenceRefs) ? item.evidenceRefs.map((ref) => resolveCanonicalEvidenceId(evidenceRegistry, ref) ?? ref) : item?.evidenceRefs,
  }));
  return evaluateResearchDecision({ ...input, signals, evidenceRegistry });
}

test('unanimous validated cross-domain evidence yields a research candidate only', async () => {
  const result = await evaluate({ marketData: marketData(), signals: consensus(), intent: 'research' });
  assert.equal(result.decision, 'RESEARCH_LONG_CANDIDATE');
  assert.equal(result.technicalBias, 'bullish');
  assert.equal(result.orderIntent, null);
  assert.equal(result.executionAuthority, 'none');
  assert.equal(result.liveExecutionAuthorized, false);
  assert.equal(result.coverage.accepted, 4);
});

test('missing provenance or dataset fingerprint blocks the decision fail-closed', async () => {
  const data = marketData();
  delete data.datasetSha256;
  const result = await evaluate({ marketData: data, signals: consensus(), intent: 'research' });
  assert.equal(result.decision, 'BLOCK');
  assert.ok(result.reasons.some((reason) => reason.includes('SHA-256 fingerprint')));
  assert.equal(result.orderIntent, null);
});

test('duplicate evidence domains and mismatched dataset snapshots are blocked', async () => {
  const duplicated = [...consensus(), signal('trend')];
  const result = await evaluate({ marketData: marketData(), signals: duplicated, intent: 'research' });
  assert.equal(result.decision, 'BLOCK');
  assert.ok(result.reasons.some((reason) => reason.includes('Duplicate signal domain')));

  const mismatch = consensus();
  mismatch[0] = signal('trend', 'bullish', { datasetSha256: 'b'.repeat(64) });
  const mismatchResult = await evaluate({ marketData: marketData(), signals: mismatch, intent: 'research' });
  assert.equal(mismatchResult.decision, 'BLOCK');
  assert.ok(mismatchResult.reasons.some((reason) => reason.includes('fingerprint does not match')));
});

test('balanced directional conflicts abstain rather than selecting the weighted winner', async () => {
  const signals = [
    signal('trend', 'bullish'), signal('structure', 'bullish'),
    signal('momentum', 'bearish'), signal('location', 'bearish'),
  ];
  const result = await evaluate({ marketData: marketData(), signals, intent: 'research' });
  assert.equal(result.decision, 'ABSTAIN_CONFLICTING_SIGNALS');
  assert.equal(result.orderIntent, null);
});

test('paper-trade intent requires a native risk result and traceable receipt', async () => {
  const result = await evaluate({
    marketData: marketData(), signals: consensus(), intent: 'paper_trade',
    risk: { passed: true, source: 'javascript', receiptId: 'made-up' },
  });
  assert.equal(result.decision, 'BLOCK');
  assert.ok(result.reasons.some((reason) => reason.includes('native Rust risk gateway')));
  assert.equal(result.liveExecutionAuthorized, false);
});

test('live execution is rejected as an unsupported intent', async () => {
  const result = await evaluate({ marketData: marketData(), signals: consensus(), intent: 'live_trade' });
  assert.equal(result.decision, 'BLOCK');
  assert.ok(result.reasons.some((reason) => reason.includes('live intent is not supported')));
  assert.equal(result.executionAuthority, 'none');
});


test('cross-domain evidence reuse is blocked to prevent one fact from counting as independent votes', async () => {
  const signals = consensus();
  signals[1] = signal('momentum', 'bullish', { evidenceRefs: ['fixture:trend:bar-99'] });
  const result = await evaluate({ marketData: marketData(), signals, intent: 'research' });
  assert.equal(result.decision, 'BLOCK');
  assert.ok(result.reasons.some((reason) => reason.includes('cross-domain evidence reuse')));
  assert.equal(result.orderIntent, null);
});

test('decision engine rejects oversized signal batches before scoring', async () => {
  const signals = Array.from({ length: 17 }, (_, index) => signal('trend', 'bullish', { evidenceRefs: [`fixture:oversized:${index}`] }));
  const result = await evaluate({ marketData: marketData(), signals, intent: 'research' });
  assert.equal(result.decision, 'BLOCK');
  assert.ok(result.reasons.some((reason) => reason.includes('hard limit of 16')));
  assert.equal(result.orderIntent, null);
});

test('decision engine bounds evidence-reference count and size', async () => {
  const tooManyRefs = signal('trend', 'bullish', { evidenceRefs: Array.from({ length: 33 }, (_, i) => `ref:${i}`) });
  const tooLongRef = signal('momentum', 'bullish', { evidenceRefs: ['x'.repeat(257)] });
  for (const candidate of [tooManyRefs, tooLongRef]) {
    const result = await evaluate({ marketData: marketData(), signals: [candidate, ...consensus().filter((item) => item.domain !== candidate.domain)], intent: 'research' });
    assert.equal(result.decision, 'BLOCK');
    assert.ok(result.reasons.some((reason) => reason.includes('evidence references')));
    assert.equal(result.orderIntent, null);
  }
});


test('decision provenance graph explains accepted evidence, weighted contributions, and gate outcomes', async () => {
  const input = { marketData: marketData(), signals: consensus(), intent: 'research' };
  const result = await evaluate(input);
  assert.equal(result.contractVersion, 'research-decision.v4');
  assert.equal(result.provenance.schemaVersion, 'decision-provenance.v2');
  assert.equal(result.provenance.graph.nodes.filter((node) => node.type === 'market_snapshot').length, 1);
  assert.equal(result.provenance.graph.nodes.filter((node) => node.type === 'validated_signal').length, 4);
  assert.equal(result.provenance.contributions.length, 4);
  assert.equal(result.provenance.contributions.find((item) => item.domain === 'trend').signedContribution, 1.44);
  assert.ok(result.provenance.gates.some((item) => item.id === 'signal_integrity' && item.status === 'passed'));
  assert.equal(result.provenance.integrity.graphIsDiagnosticNotCryptographicAttestation, true);
  assert.equal(result.provenance.integrity.canonicalRegistryVerified, true);
  assert.match(result.provenance.integrity.canonicalRegistrySha256, /^[a-f0-9]{64}$/);
  assert.equal(result.orderIntent, null);
  assert.equal(result.liveExecutionAuthorized, false);
  assert.deepEqual(result.provenance, (await evaluate(input)).provenance, 'identical inputs must produce identical provenance');
});

test('decision engine fails closed when canonical evidence registry is absent or copied', () => {
  const input = { marketData: marketData(), signals: consensus(), intent: 'research' };
  const result = evaluateResearchDecision(input);
  assert.equal(result.decision, 'BLOCK');
  assert.ok(result.reasons.some((reason) => reason.includes('canonical evidence registry')));
  assert.equal(result.executionAuthority, 'none');
  assert.equal(result.orderIntent, null);
});

test('blocked decisions preserve rejected gate reasons in provenance rather than hiding the failure', async () => {
  const signals = consensus();
  signals[1] = signal('momentum', 'bullish', { evidenceRefs: ['fixture:trend:bar-99'] });
  const result = await evaluate({ marketData: marketData(), signals, intent: 'research' });
  assert.equal(result.decision, 'BLOCK');
  assert.equal(result.provenance.integrity.crossDomainEvidenceReuseDetected, true);
  const integrityGate = result.provenance.gates.find((item) => item.id === 'signal_integrity');
  assert.equal(integrityGate.status, 'blocked');
  assert.ok(integrityGate.detail.some((detail) => detail.includes('cross-domain evidence reuse')));
  assert.equal(result.executionAuthority, 'none');
});
