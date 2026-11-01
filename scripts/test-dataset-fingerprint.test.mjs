import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

const DOMAIN_SEPARATOR = Buffer.from('agentic-trading-os.ohlcv.canonical.v1\0', 'utf8');

/** Independent executable reference for the byte-level dataset fingerprint contract. */
function canonicalDatasetSha256(candles) {
  const chunks = [DOMAIN_SEPARATOR];
  const count = Buffer.alloc(8);
  count.writeBigUInt64LE(BigInt(candles.length));
  chunks.push(count);
  for (const candle of candles) {
    const time = Buffer.alloc(8);
    time.writeBigInt64LE(BigInt(candle.time));
    chunks.push(time);
    for (const key of ['open', 'high', 'low', 'close', 'volume']) {
      const value = Buffer.alloc(8);
      value.writeDoubleLE(candle[key]);
      chunks.push(value);
    }
  }
  return createHash('sha256').update(Buffer.concat(chunks)).digest('hex');
}

const knownCandle = { time: 1, open: 1, high: 2, low: 0.5, close: 1.5, volume: 3 };
const knownDigest = 'f2372778376160ebe9f4759a6a81dc8e0b76020e21dcc8dafe48d1f76134dec6';

test('canonical OHLCV SHA-256 reference matches the cross-language known vector', () => {
  assert.equal(canonicalDatasetSha256([knownCandle]), knownDigest);
});

test('canonical fingerprint is stable and changes when any input field changes', () => {
  const candle = { time: 10, open: 10, high: 12, low: 9, close: 11, volume: 42 };
  const baseline = canonicalDatasetSha256([candle]);
  assert.equal(canonicalDatasetSha256([{ ...candle }]), baseline);
  for (const changed of [
    { ...candle, time: 11 }, { ...candle, open: 10.5 }, { ...candle, high: 12.5 },
    { ...candle, low: 8.5 }, { ...candle, close: 11.5 }, { ...candle, volume: 43 },
  ]) {
    assert.notEqual(canonicalDatasetSha256([changed]), baseline);
  }
  assert.notEqual(canonicalDatasetSha256([candle]), canonicalDatasetSha256([candle, candle]));
});
