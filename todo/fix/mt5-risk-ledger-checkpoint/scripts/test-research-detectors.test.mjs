import test from 'node:test';
import assert from 'node:assert/strict';
import { detectConfirmedSwings, classifySwingStructure, detectStructureBreaks } from '../src/lib/engines/technical-analysis/research-detectors.js';

const candles = (highs, lows = highs.map((x) => x - 2), closes = highs.map((x, i) => (x + lows[i]) / 2)) => highs.map((high, i) => ({ open: closes[i], high, low: lows[i], close: closes[i] }));

test('swing pivots are timestamped at confirmation, not at the extreme', () => {
  const c = candles([1, 3, 7, 4, 2, 5, 3]);
  const pivots = detectConfirmedSwings(c, { leftBars: 1, rightBars: 1 });
  assert.ok(pivots.some((p) => p.type === 'swing_high' && p.pivotIndex === 2 && p.confirmedAt === 3));
});

test('equal highs are not arbitrarily selected as unique swing highs', () => {
  const c = candles([1, 5, 5, 2, 1]);
  const pivots = detectConfirmedSwings(c, { leftBars: 1, rightBars: 1 });
  assert.equal(pivots.filter((p) => p.type === 'swing_high').length, 0);
});

test('structure classification uses only prior pivots of the same type', () => {
  const out = classifySwingStructure([
    { type: 'swing_high', price: 10, pivotIndex: 1, confirmedAt: 2 },
    { type: 'swing_low', price: 5, pivotIndex: 2, confirmedAt: 3 },
    { type: 'swing_high', price: 12, pivotIndex: 4, confirmedAt: 5 },
    { type: 'swing_low', price: 6, pivotIndex: 5, confirmedAt: 6 },
  ]);
  assert.deepEqual(out.map((p) => p.label), [null, null, 'HH', 'HL']);
});

test('malformed candle data fails closed', () => {
  assert.throws(() => detectConfirmedSwings([{ high: 1, low: 2 }, { high: 2, low: 1 }, { high: 3, low: 2 }]), /Invalid OHLC/);
});

test('break detector does not use a swing before its confirmation index', () => {
  const c = candles([1, 4, 8, 5, 3, 9, 4], [0, 2, 5, 2, 1, 3, 2], [0.5, 3, 6, 4, 2, 8.5, 3]);
  const pivots = classifySwingStructure(detectConfirmedSwings(c, { leftBars: 1, rightBars: 1 }));
  const events = detectStructureBreaks(c, pivots);
  assert.ok(events.every((e) => e.detectedAt >= 0 && e.levelPivotIndex < e.detectedAt));
});


test('break detector rejects forged pivot prices that do not match source candle provenance', () => {
  const c = candles([1, 4, 8, 5, 3, 9, 4], [0, 2, 5, 2, 1, 3, 2], [0.5, 3, 6, 4, 2, 8.5, 3]);
  assert.throws(() => detectStructureBreaks(c, [
    { type: 'swing_high', price: 999, pivotIndex: 2, confirmedAt: 3, label: null },
  ]), /source candle provenance/);
});

test('break detector rejects non-causal and out-of-range pivot confirmation references', () => {
  const c = candles([1, 4, 8, 5, 3, 9, 4], [0, 2, 5, 2, 1, 3, 2], [0.5, 3, 6, 4, 2, 8.5, 3]);
  assert.throws(() => detectStructureBreaks(c, [
    { type: 'swing_high', price: c[2].high, pivotIndex: 2, confirmedAt: 2, label: null },
  ]), /non-causal/);
  assert.throws(() => detectStructureBreaks(c, [
    { type: 'swing_high', price: c[2].high, pivotIndex: 2, confirmedAt: 99, label: null },
  ]), /non-causal/);
});

test('break detector rejects duplicate pivot identities rather than silently overwriting provenance', () => {
  const c = candles([1, 4, 8, 5, 3, 9, 4], [0, 2, 5, 2, 1, 3, 2], [0.5, 3, 6, 4, 2, 8.5, 3]);
  const pivot = { type: 'swing_high', price: c[2].high, pivotIndex: 2, confirmedAt: 3, label: null };
  assert.throws(() => detectStructureBreaks(c, [pivot, { ...pivot }]), /Duplicate confirmed pivot identity/);
});
