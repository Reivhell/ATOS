import test from 'node:test';
import assert from 'node:assert/strict';
import { getMaximumVolume, normalizedVolumeHeight, summarizeLastCandle } from '../src/lib/presentation/dashboard/dashboard-model.js';

const candle = { time: 1_700_000_000_000, open: 100, high: 112, low: 98, close: 110, volume: 25 };

test('summarizes the latest validated candle without inventing values', () => {
  assert.deepEqual(summarizeLastCandle([candle]), {
    time: candle.time, open: 100, high: 112, low: 98, close: 110, volume: 25,
    changePct: 10.000000000000009, bullish: true,
  });
  assert.equal(summarizeLastCandle([]), null);
  assert.equal(summarizeLastCandle(null), null);
});

test('rejects malformed OHLC relationships', () => {
  assert.equal(summarizeLastCandle([{ ...candle, high: 105 }]), null);
  assert.equal(summarizeLastCandle([{ ...candle, low: 111 }]), null);
  assert.equal(summarizeLastCandle([{ ...candle, volume: -1 }]), null);
  assert.equal(summarizeLastCandle([{ ...candle, close: Number.NaN }]), null);
});

test('maximum volume ignores missing and invalid values', () => {
  assert.equal(getMaximumVolume([candle, { volume: 50 }, { volume: -1 }, { volume: Infinity }, null]), 50);
  assert.equal(getMaximumVolume(undefined), 0);
});

test('volume bars normalize into the specified display range', () => {
  assert.equal(normalizedVolumeHeight(50, 50), 34);
  assert.equal(normalizedVolumeHeight(25, 50), 18);
  assert.equal(normalizedVolumeHeight(1, 50), 2.64);
  assert.equal(normalizedVolumeHeight(0, 50), 0);
  assert.equal(normalizedVolumeHeight(25, 0), 0);
  assert.equal(normalizedVolumeHeight(100, 50, 20, 3), 20);
});
