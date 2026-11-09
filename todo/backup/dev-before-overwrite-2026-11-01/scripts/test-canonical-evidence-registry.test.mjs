import test from 'node:test';
import assert from 'node:assert/strict';
import {
  createCanonicalEvidenceRegistry,
  getCanonicalEvidenceRecord,
  isVerifiedCanonicalEvidenceRegistry,
  resolveCanonicalEvidenceId,
} from '../src/lib/domain/decision/canonical-evidence-registry.js';

const scope = {
  source: 'market-feed',
  observedAt: '2026-10-29T00:00:00.000Z',
  datasetSha256: 'a'.repeat(64),
  symbol: 'BTCUSDT',
  timeframe: '5m',
};
const descriptor = (ref, payload, extra = {}) => ({ ...scope, ref, payload, ...extra });

test('canonical IDs and registry digest are stable regardless of descriptor and object-key order', async () => {
  const left = await createCanonicalEvidenceRegistry([
    descriptor('obs-a', { close: 100, open: 99 }),
    descriptor('obs-b', { volume: 4, low: 98 }),
  ]);
  const right = await createCanonicalEvidenceRegistry([
    descriptor('obs-b', { low: 98, volume: 4 }),
    descriptor('obs-a', { open: 99, close: 100 }),
  ]);
  assert.equal(left.registrySha256, right.registrySha256);
  assert.deepEqual(left.records.map((item) => item.canonicalId), right.records.map((item) => item.canonicalId));
  assert.equal(left.recordCount, 2);
  assert.equal(isVerifiedCanonicalEvidenceRegistry(left), true);
});

test('identical canonical content under different aliases resolves to the same evidence identity', async () => {
  const registry = await createCanonicalEvidenceRegistry([
    descriptor('vendor-a:bar-12', { close: 100 }),
    descriptor('vendor-b:bar-12', { close: 100 }),
  ]);
  assert.equal(registry.recordCount, 1);
  assert.equal(resolveCanonicalEvidenceId(registry, 'vendor-a:bar-12'), resolveCanonicalEvidenceId(registry, 'vendor-b:bar-12'));
});

test('derivation parents are canonicalized and unresolved references fail closed', async () => {
  const registry = await createCanonicalEvidenceRegistry([
    descriptor('derived', { movingAverage: 100 }, { parentRefs: ['raw'] }),
    descriptor('raw', { close: 100 }),
  ]);
  const derived = getCanonicalEvidenceRecord(registry, resolveCanonicalEvidenceId(registry, 'derived'));
  assert.equal(derived.parentEvidenceIds.length, 1);
  assert.equal(derived.parentEvidenceIds[0], resolveCanonicalEvidenceId(registry, 'raw'));
  await assert.rejects(() => createCanonicalEvidenceRegistry([descriptor('derived', { x: 1 }, { parentRefs: ['missing'] })]), /unresolved/);
});

test('derivation cycles are rejected', async () => {
  await assert.rejects(() => createCanonicalEvidenceRegistry([
    descriptor('a', { x: 1 }, { parentRefs: ['b'] }),
    descriptor('b', { y: 2 }, { parentRefs: ['a'] }),
  ]), /cycle detected/);
});

test('invalid scope, non-finite payloads, and payload budget overflow are rejected', async () => {
  await assert.rejects(() => createCanonicalEvidenceRegistry([descriptor('bad', { x: Number.NaN })]), /non-finite/);
  await assert.rejects(() => createCanonicalEvidenceRegistry([descriptor('bad', { x: 'x'.repeat(17_000) })]), /exceeds 4096 characters/);
  await assert.rejects(() => createCanonicalEvidenceRegistry([descriptor('bad', { x: 1 }, { symbol: 'btc/usdt' })]), /invalid instrument scope/);
});

test('invalid calendar timestamps and accessor-backed payloads are rejected without invoking getters', async () => {
  await assert.rejects(() => createCanonicalEvidenceRegistry([descriptor('bad-time', { close: 100 }, { observedAt: '2026-02-30T00:00:00.000Z' })]), /explicit UTC observation timestamp/);
  let getterInvoked = false;
  const payload = {};
  Object.defineProperty(payload, 'secret', { enumerable: true, get() { getterInvoked = true; return 'unexpected'; } });
  await assert.rejects(() => createCanonicalEvidenceRegistry([descriptor('getter', payload)]), /accessor properties/);
  assert.equal(getterInvoked, false);
  const sparse = new Array(2);
  sparse[1] = 'x';
  await assert.rejects(() => createCanonicalEvidenceRegistry([descriptor('sparse', { values: sparse })]), /dense/);
});

test('registry records and nested payloads are immutable and cannot be forged by cloning', async () => {
  const registry = await createCanonicalEvidenceRegistry([descriptor('obs', { nested: { close: 100 } })]);
  assert.equal(Object.isFrozen(registry), true);
  assert.equal(Object.isFrozen(registry.records[0]), true);
  assert.equal(Object.isFrozen(registry.records[0].payload.nested), true);
  assert.equal(isVerifiedCanonicalEvidenceRegistry({ ...registry }), false);
});
