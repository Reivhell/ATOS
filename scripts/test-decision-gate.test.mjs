import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { evaluateStrictDecisionGate } from '../src/lib/engines/decision/strict-decision-gate.js';
import { REVIEWED_IMPLEMENTATION_VERSION_BY_CATALOG_ID } from '../src/lib/domain/decision/contracts.js';
import { STRICT_CATALOG_REGISTRY, STRICT_MASTER_LIST_VERSION } from '../src/lib/domain/decision/strict-catalog-registry.js';

const manifest = JSON.parse(readFileSync(new URL('./fixtures/technical-analysis-master-list.v1.json', import.meta.url), 'utf8'));
const catalog = manifest.entries;
const now = '2026-10-27T06:00:00.000Z';
const AGENT_BY_CATEGORY = {
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
const assessmentFor = (item, status, extra = {}) => ({
  contractVersion: 'strict-ta-assessment.v1',
  masterListVersion: STRICT_MASTER_LIST_VERSION,
  catalogId: item.id,
  category: item.category,
  group: item.group,
  name: item.name,
  agentId: AGENT_BY_CATEGORY[item.category],
  status,
  asOf: now,
  ...extra,
});
const goodAssessment = (item) => item.availability === 'Catalog only'
  ? assessmentFor(item, 'unsupported', { reason: 'Reviewed detector implementation is not available yet.' })
  : assessmentFor(item, 'evaluated', {
      evidenceRefs: [`snapshot:${'a'.repeat(64)}:binance:catalog-${catalog.indexOf(item)}`],
      result: { value: 'fixture-only' },
      lookAheadSafe: true,
      implementationVersion: REVIEWED_IMPLEMENTATION_VERSION_BY_CATALOG_ID[item.id] ?? 'unregistered-test-version',
      dataSnapshotId: 'a'.repeat(64),
    });
const validInput = () => ({
  catalog,
  assessments: catalog.map(goodAssessment),
  marketData: {
    validated: true,
    source: 'test-fixture',
    asOf: now,
    datasetSha256: 'a'.repeat(64),
    symbol: 'BTC-USD',
    timeframe: '1h',
  },
  risk: { passed: true },
  intent: 'paper_trade',
});

test('strict gate blocks while any catalog-only concepts remain unsupported', () => {
  const result = evaluateStrictDecisionGate(validInput());
  assert.equal(result.allowed, false);
  assert.equal(result.decision, 'BLOCK');
  assert.equal(result.coverage.required, 364);
  assert.equal(result.coverage.assessed, 364);
  assert.equal(result.coverage.catalogOnly, 357);
  assert.equal(result.coverage.unsupported, 357);
  assert.equal(result.handoff, undefined);
});

test('missing one assessment blocks handoff', () => {
  const input = validInput(); input.assessments.pop();
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('Assessment count')));
  assert.ok(result.reasons.some((reason) => reason.includes('Missing assessment')));
});

test('caller cannot self-promote catalog-only concepts by changing payload availability', () => {
  const input = validInput();
  input.catalog = catalog.map((item) => ({ ...item, availability: 'Calculated now' }));
  input.assessments = catalog.map((item) => assessmentFor(item, 'evaluated', { evidenceRefs: [`snapshot:${'a'.repeat(64)}:binance:catalog-${catalog.indexOf(item)}`], result: {}, lookAheadSafe: true, implementationVersion: 'agent-assertion', dataSnapshotId: 'agent-assertion' }));
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('code-owned Strict Master List')));
  assert.ok(result.reasons.some((reason) => reason.includes('must remain unsupported')));
});

test('catalog-only concept cannot be marked evaluated even when assessment has evidence', () => {
  const input = validInput();
  const index = catalog.findIndex((item) => item.availability === 'Catalog only');
  input.assessments[index] = assessmentFor(catalog[index], 'evaluated', { evidenceRefs: [`snapshot:${'a'.repeat(64)}:binance:catalog-${index}`], result: {}, lookAheadSafe: true, implementationVersion: 'test', dataSnapshotId: 'fixture' });
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('must remain unsupported')));
});

test('insufficient data blocks handoff and requires a reason', () => {
  const input = validInput();
  const item = catalog.find((entry) => entry.availability !== 'Catalog only');
  input.assessments[catalog.findIndex((entry) => entry.id === item.id)] = assessmentFor(item, 'insufficient_data', { reason: 'Higher timeframe candles are missing.' });
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.equal(result.coverage.insufficientData, 1);
});

test('duplicate or unknown assessments block handoff', () => {
  const input = validInput();
  input.assessments.push(input.assessments[0], { catalogId: 'not-in-contract', status: 'evaluated', asOf: now, evidenceRefs: ['x'], result: {}, lookAheadSafe: true });
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.equal(result.coverage.duplicate, 1);
  assert.ok(result.reasons.some((reason) => reason.includes('unknown or malformed')));
});

test('evaluated status requires valid timestamp, evidence, explicit result and look-ahead flag', () => {
  const input = validInput();
  const index = catalog.findIndex((item) => item.availability === 'Calculated now');
  input.assessments[index] = assessmentFor(catalog[index], 'evaluated', { asOf: 'bad-date', evidenceRefs: [], result: undefined });
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('timestamp')));
  assert.ok(result.reasons.some((reason) => reason.includes('evidence references')));
  assert.ok(result.reasons.some((reason) => reason.includes('explicit result')));
  assert.ok(result.reasons.some((reason) => reason.includes('look-ahead safety')));
  assert.ok(result.reasons.some((reason) => reason.includes('implementation version')));
  assert.ok(result.reasons.some((reason) => reason.includes('data snapshot identifier')));
});

test('not-applicable requires a meaningful contextual rationale', () => {
  const input = validInput();
  const index = catalog.findIndex((item) => item.availability !== 'Catalog only');
  input.assessments[index] = assessmentFor(catalog[index], 'not_applicable', { reason: 'n/a', evidenceRefs: [] });
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('meaningful rationale')));
});

test('unvalidated market data or failed risk gateway blocks handoff', () => {
  const input = validInput(); input.marketData.validated = false; input.risk.passed = false;
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('Market data')));
  assert.ok(result.reasons.some((reason) => reason.includes('Risk gateway')));
});

test('intent must be explicit and live execution is never authorized', () => {
  const input = validInput(); input.intent = 'live_trade';
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('Intent must be explicitly')));
  assert.equal(result.handoff, undefined);
});

test('master-list identity, category and names are protected by code-owned registry', () => {
  const input = validInput();
  input.catalog = catalog.map((item, index) => index === 0 ? { ...item, category: 'Fake Category' } : item);
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('code-owned Strict Master List')));
});

test('code-owned registry exactly matches reviewed fixture identity and availability', () => {
  assert.equal(STRICT_MASTER_LIST_VERSION, '1.0.0');
  assert.deepEqual(STRICT_CATALOG_REGISTRY, catalog.map(({ id, category, group, name, availability, kind }) => ({ id, category, group, name, availability, kind })));
});

test('assessment identity must match master version, taxonomy and category owner', () => {
  const input = validInput();
  input.assessments[0] = { ...input.assessments[0], group: 'Wrong Group', agentId: 'wrong-agent' };
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('identity/contract/agent routing mismatch')));
});

test('market-data gate requires an auditable SHA-256 snapshot and exact scope', () => {
  const input = validInput();
  delete input.marketData.datasetSha256;
  delete input.marketData.timeframe;
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('SHA-256 snapshot fingerprint')));
  assert.ok(result.reasons.some((reason) => reason.includes('timeframe scope')));
});


test('assessment snapshot digest must be valid SHA-256 and compare independently of hex case', () => {
  const input = validInput();
  const index = input.assessments.findIndex((entry) => entry.status === 'evaluated');
  input.assessments[index] = { ...input.assessments[index], dataSnapshotId: 'not-a-digest' };
  let result = evaluateStrictDecisionGate(input);
  assert.ok(result.reasons.some((reason) => reason.includes('valid SHA-256 data snapshot identifier')));

  const uppercase = validInput();
  const evaluatedIndex = uppercase.assessments.findIndex((entry) => entry.status === 'evaluated');
  uppercase.assessments[evaluatedIndex] = {
    ...uppercase.assessments[evaluatedIndex],
    dataSnapshotId: 'A'.repeat(64),
    evidenceRefs: [`snapshot:${'A'.repeat(64)}:binance:catalog-${catalog.indexOf(catalog.find((item) => item.id === uppercase.assessments[evaluatedIndex].catalogId))}`],
  };
  result = evaluateStrictDecisionGate(uppercase);
  assert.ok(!result.reasons.some((reason) => reason.includes('does not match the validated market-data dataset')));
});

test('assessment timestamps must include a timezone and evaluated evidence must bind to the exact dataset fingerprint', () => {
  const input = validInput();
  const index = catalog.findIndex((entry) => entry.availability === 'Calculated now');
  input.assessments[index] = { ...input.assessments[index], asOf: '2026-10-27T06:00:00', dataSnapshotId: 'b'.repeat(64) };
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('timestamp')));
  assert.ok(result.reasons.some((reason) => reason.includes('does not match the validated market-data dataset')));
});


test('assessment timestamp must match the exact snapshot cutoff and evidence references must be unique', () => {
  const input = validInput();
  input.assessments[0] = { ...input.assessments[0], asOf: '2026-10-28T06:00:00.000Z' };
  const result = evaluateStrictDecisionGate(input);
  assert.equal(result.allowed, false);
  assert.ok(result.reasons.some((reason) => reason.includes('exact market-data snapshot cutoff')));

  const second = validInput();
  const evaluated = second.assessments.find((entry) => entry.status === 'evaluated');
  evaluated.evidenceRefs = [`snapshot:${'a'.repeat(64)}:binance:catalog-${catalog.indexOf(catalog.find((entry) => entry.id === evaluated.catalogId))}`, `snapshot:${'a'.repeat(64)}:binance:catalog-${catalog.indexOf(catalog.find((entry) => entry.id === evaluated.catalogId))}`];
  const duplicateEvidence = evaluateStrictDecisionGate(second);
  assert.equal(duplicateEvidence.allowed, false);
  assert.ok(duplicateEvidence.reasons.some((reason) => reason.includes('evidence references')));
});


test('evidence references enforce count and length bounds', () => {
  const input = validInput();
  const index = input.assessments.findIndex((entry) => entry.status === 'evaluated');
  const original = input.assessments[index];
  input.assessments[index] = { ...original, evidenceRefs: Array.from({ length: 33 }, (_, i) => `snapshot:${'a'.repeat(64)}:binance:catalog-${i}`) };
  let result = evaluateStrictDecisionGate(input);
  assert.ok(result.reasons.some((reason) => reason.includes('canonical')));

  input.assessments[index] = { ...original, evidenceRefs: [`snapshot:${'a'.repeat(64)}:binance:${'x'.repeat(257)}`] };
  result = evaluateStrictDecisionGate(input);
  assert.ok(result.reasons.some((reason) => reason.includes('canonical')));
});

test('evidence references must bind to the exact snapshot and use canonical safe locators', () => {
  const input = validInput();
  const index = input.assessments.findIndex((entry) => entry.status === 'evaluated');
  const original = input.assessments[index];
  const reference = original.evidenceRefs[0];
  input.assessments[index] = { ...original, evidenceRefs: [reference.replace('a'.repeat(64), 'b'.repeat(64))] };
  let result = evaluateStrictDecisionGate(input);
  assert.ok(result.reasons.some((reason) => reason.includes('bound to the market-data snapshot')));

  input.assessments[index] = { ...original, evidenceRefs: ['agent-assertion'] };
  result = evaluateStrictDecisionGate(input);
  assert.ok(result.reasons.some((reason) => reason.includes('canonical')));

  input.assessments[index] = { ...original, evidenceRefs: [`snapshot:${'a'.repeat(64)}:agent:assertion`] };
  input.assessments[index] = { ...original, evidenceRefs: [`snapshot:${'a'.repeat(64)}:fixture:catalog-${index}`] };
  result = evaluateStrictDecisionGate(input);
  assert.ok(result.reasons.some((reason) => reason.includes('canonical, and bound')));
  result = evaluateStrictDecisionGate(input);
  assert.ok(result.reasons.some((reason) => reason.includes('canonical')));

  input.assessments[index] = { ...original, evidenceRefs: [`snapshot:${'a'.repeat(64)}:binance:../private`] };
  result = evaluateStrictDecisionGate(input);
  assert.ok(result.reasons.some((reason) => reason.includes('canonical')));
});

test('JavaScript parity gate never authorizes downstream handoff; Rust is authoritative', () => {
  const result = evaluateStrictDecisionGate(validInput());
  assert.equal(result.allowed, false);
  assert.equal(result.handoff, undefined);
  assert.equal(result.nativeRuntimeRequired, true);
  assert.equal(result.decision, 'BLOCK');
});
