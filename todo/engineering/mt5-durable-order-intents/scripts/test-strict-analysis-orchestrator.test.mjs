import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createStrictAnalysisWorkPlan, evaluateStrictAnalysisCompletion } from '../src/lib/application/strict-analysis-orchestrator.js';
import { STRICT_CATALOG_REGISTRY, STRICT_MASTER_LIST_VERSION } from '../src/lib/domain/decision/strict-catalog-registry.js';
import { REVIEWED_IMPLEMENTATION_VERSION_BY_CATALOG_ID } from '../src/lib/domain/decision/contracts.js';

const manifest = JSON.parse(readFileSync(new URL('./fixtures/technical-analysis-master-list.v1.json', import.meta.url), 'utf8'));
const schema = JSON.parse(readFileSync(new URL('../docs/agents/strict-ta-assessment.schema.json', import.meta.url), 'utf8'));
const asOf = '2026-10-27T08:00:00.000Z';
const fingerprint = 'a'.repeat(64);
const plan = () => createStrictAnalysisWorkPlan({ symbol: 'BTC-USD', timeframe: '1h', asOf, datasetSha256: fingerprint });
const agentByCategory = {
  'SMC & ICT': 'agent-01-smc-ict',
  'Price Action': 'agent-02-price-action',
  'Support, Resistance & Trend': 'agent-03-support-resistance-trend',
  'Indikator Teknikal': 'agent-04-technical-indicators',
  'Order Flow & Market Profile': 'agent-05-order-flow-market-profile',
  'Divergence & Intermarket Analysis': 'agent-06-divergence-intermarket',
  'Metode Analisis Lain': 'agent-07-other-methods',
  'Fundamental & Makro': 'agent-08-fundamental-macro',
  'Risk Management & Evaluation': 'agent-09-risk-management-evaluation',
};
const assessmentsFor = (workPlan) => workPlan.entries.map((entry) => ({
  contractVersion: 'strict-ta-assessment.v1',
  masterListVersion: STRICT_MASTER_LIST_VERSION,
  catalogId: entry.id,
  category: entry.category,
  group: entry.group,
  name: entry.name,
  agentId: agentByCategory[entry.category],
  status: entry.availability === 'Catalog only' ? 'unsupported' : 'evaluated',
  asOf,
  ...(entry.availability === 'Catalog only'
    ? { reason: 'No reviewed implementation is registered for this concept.' }
    : {
        evidenceRefs: [`snapshot:${fingerprint}:binance:catalog-${workPlan.entries.indexOf(entry)}`],
        result: { fixtureOnly: true },
        lookAheadSafe: true,
        implementationVersion: REVIEWED_IMPLEMENTATION_VERSION_BY_CATALOG_ID[entry.id] ?? 'unregistered-fixture-version',
        dataSnapshotId: fingerprint,
      }),
}));
const marketDataFor = (workPlan) => ({
  validated: true,
  source: 'deterministic-test-fixture',
  asOf: workPlan.asOf,
  datasetSha256: workPlan.datasetSha256,
  symbol: workPlan.symbol,
  timeframe: workPlan.timeframe,
});

test('work plan dispatches all 364 exact IDs to nine category-owned specialists', () => {
  const workPlan = plan();
  assert.equal(workPlan.expectedAssessmentCount, 364);
  assert.equal(workPlan.entries.length, 364);
  assert.equal(workPlan.tasks.length, 9);
  assert.equal(workPlan.dispatchStatus, 'work_plan_created_not_dispatched');
  assert.equal(workPlan.liveExecutionAuthorized, false);
  assert.deepEqual(workPlan.entries.map((entry) => entry.id), STRICT_CATALOG_REGISTRY.map((entry) => entry.id));
  assert.equal(new Set(workPlan.tasks.flatMap((task) => task.catalogIds)).size, 364);
  for (const task of workPlan.tasks) {
    assert.equal(task.agentId, agentByCategory[task.category]);
    assert.equal(task.requiredAssessmentCount, manifest.categoryCounts[task.category]);
    assert.equal(task.catalogIds.length, task.requiredAssessmentCount);
  }
});

test('work plan rejects missing instrument scope, bad timestamp, and invalid dataset fingerprint', () => {
  assert.throws(() => createStrictAnalysisWorkPlan({ timeframe: '1h', asOf, datasetSha256: fingerprint }), /symbol is required/);
  assert.throws(() => createStrictAnalysisWorkPlan({ symbol: 'BTC-USD', timeframe: '1h', asOf: 'bad', datasetSha256: fingerprint }), /valid ISO-8601/);
  assert.throws(() => createStrictAnalysisWorkPlan({ symbol: 'BTC-USD', timeframe: '1h', asOf: '2026-10-27T08:00:00', datasetSha256: fingerprint }), /valid ISO-8601/);
  assert.throws(() => createStrictAnalysisWorkPlan({ symbol: 'BTC-USD', timeframe: '1h', asOf, datasetSha256: 'abc' }), /64-character SHA-256/);
});

test('completion barrier processes the full inventory but blocks on every catalog-only concept', () => {
  const workPlan = plan();
  const result = evaluateStrictAnalysisCompletion({
    workPlan,
    assessments: assessmentsFor(workPlan),
    marketData: marketDataFor(workPlan),
    risk: { passed: true },
    intent: 'paper_trade',
  });
  assert.equal(result.allowed, false);
  assert.equal(result.decision, 'BLOCK');
  assert.equal(result.coverage.required, 364);
  assert.equal(result.coverage.assessed, 364);
  assert.equal(result.coverage.catalogOnly, 357);
  assert.equal(result.coverage.unsupported, 357);
  assert.equal(result.nextLayerPayload, undefined);
  assert.equal(result.nativeRustGateRequired, true);
  assert.equal(result.liveExecutionAuthorized, false);
  assert.equal(Object.keys(result.coverage.byCategory).length, 9);
});

test('completion diagnostics treat SHA-256 hex case as the same snapshot identity', () => {
  const workPlan = plan();
  const assessments = assessmentsFor(workPlan).map((assessment) => assessment.status === 'evaluated'
    ? { ...assessment, dataSnapshotId: assessment.dataSnapshotId.toUpperCase() }
    : assessment);
  const result = evaluateStrictAnalysisCompletion({
    workPlan,
    assessments,
    marketData: marketDataFor(workPlan),
    risk: { passed: true },
    intent: 'research',
  });
  assert.equal(result.allowed, false);
  assert.ok(!result.reasons.some((reason) => reason.includes('dataSnapshotId does not match the work-plan snapshot')));
});

test('completion barrier blocks snapshot mismatch and specialist identity tampering', () => {
  const workPlan = plan();
  const assessments = assessmentsFor(workPlan);
  assessments[1] = { ...assessments[1], agentId: 'agent-09-risk-management-evaluation' };
  const badMarket = { ...marketDataFor(workPlan), datasetSha256: 'b'.repeat(64) };
  const result = evaluateStrictAnalysisCompletion({ workPlan, assessments, marketData: badMarket, risk: { passed: true }, intent: 'research' });
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('fingerprint does not match')));
  assert.ok(result.reasons.some((reason) => reason.includes('identity/contract/agent routing mismatch')));
  assert.equal(result.nextLayerPayload, undefined);
  assert.equal(result.nativeRustGateRequired, true);
});

test('completion barrier blocks incomplete plans and missing assessments', () => {
  const workPlan = plan();
  const incompletePlan = { ...workPlan, tasks: [] };
  const result = evaluateStrictAnalysisCompletion({
    workPlan: incompletePlan,
    assessments: assessmentsFor(workPlan).slice(0, 363),
    marketData: marketDataFor(workPlan),
    risk: { passed: true },
    intent: 'research',
  });
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('specialist routing')));
  assert.ok(result.reasons.some((reason) => reason.includes('Assessment count')));
  assert.equal(result.nextLayerPayload, undefined);
  assert.equal(result.nativeRustGateRequired, true);
});

test('assessment schema captures strict identity and status-specific evidence requirements', () => {
  assert.equal(schema.properties.contractVersion.const, 'strict-ta-assessment.v1');
  assert.equal(schema.properties.masterListVersion.const, '1.0.0');
  assert.ok(schema.required.includes('catalogId'));
  assert.ok(schema.required.includes('group'));
  assert.ok(schema.required.includes('agentId'));
  assert.equal(schema.allOf.length, 4);
  assert.equal(schema.properties.evidenceRefs.maxItems, 32);
  assert.equal(schema.properties.evidenceRefs.items.maxLength, 256);
  assert.match(schema.properties.evidenceRefs.items.pattern, /snapshot:/);
  const evidencePattern = new RegExp(schema.properties.evidenceRefs.items.pattern);
  for (const locator of ['BTCUSDT/1h', 'candle-1', 'snapshot-record']) {
    assert.equal(evidencePattern.test(`snapshot:${'a'.repeat(64)}:binance:${locator}`), true);
  }
  for (const locator of ['', '.', '..', 'BTCUSDT/', 'BTCUSDT//1h', 'BTCUSDT/./1h', 'BTCUSDT/.private/1h', 'BTCUSDT/../1h']) {
    assert.equal(evidencePattern.test(`snapshot:${'a'.repeat(64)}:binance:${locator}`), false, `unsafe schema locator accepted: ${locator}`);
  }
  assert.equal(evidencePattern.test(`snapshot:${'a'.repeat(64)}:fixture:catalog-1`), false);
});
