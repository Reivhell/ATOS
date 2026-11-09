import test from 'node:test';
import assert from 'node:assert/strict';
import {
  buildCandleSeriesRequest,
  getAvailableCandleSeries,
  isOrderChartAligned,
  isValidatedCandleSeriesSnapshot,
  isCurrentLiveCandleSeriesSnapshot,
  validateCandleSeriesResponse,
} from '../src/lib/domain/market-data/market-data-model.js';

const baseTime = Math.floor(Date.now() / 300_000) * 300_000 - 30 * 300_000;
function makeCandles(count = 30, intervalMs = 300_000) {
  let previous = 100;
  return Array.from({ length: count }, (_, index) => {
    const close = 100 + index * 0.25;
    const candle = {
      time: baseTime + index * intervalMs,
      open: previous,
      high: Math.max(previous, close) + 0.5,
      low: Math.min(previous, close) - 0.5,
      close,
      volume: index + 1,
    };
    previous = close;
    return candle;
  });
}

const capabilities = {
  quoteSymbols: ['BTCUSDT', 'ETHUSDT', 'SOLUSDT'],
  candleSeries: [
    { symbol: 'BTCUSDT', interval: '5m', supported: true, available: true, candleCount: 120, latestCandleTimeMs: baseTime + 29 * 300_000, latestClosedBarAtMs: baseTime + 30 * 300_000 - 1, retrievedAtMs: Date.now(), source: 'Public provider', note: 'validated fixture series' },
    { symbol: 'ETHUSDT', interval: '5m', supported: false, available: false, candleCount: 0, latestCandleTimeMs: null, latestClosedBarAtMs: null, retrievedAtMs: 0, source: 'Public provider', note: 'quote only' },
    { symbol: 'BTCUSDT', interval: '1h', supported: true, available: false, candleCount: 0, latestCandleTimeMs: null, latestClosedBarAtMs: null, retrievedAtMs: 0, source: 'Public provider', note: 'hourly candles have not been loaded' },
  ],
};

const validResponse = (overrides = {}) => {
  const retrievedAtMs = Date.now();
  const candles = makeCandles();
  return {
    request: { symbol: 'BTCUSDT', interval: '5m' },
    source: 'Public provider',
    providerStatus: 'CONNECTED · PUBLIC MARKET DATA',
    updatedAtMs: retrievedAtMs,
    retrievedAtMs,
    latestClosedBarAtMs: candles.at(-1).time + 299_999,
    datasetSha256: 'a'.repeat(64),
    isLive: true,
    candles,
    ...overrides,
  };
};

test('available candle-series list includes only supported, loaded and metadata-valid pairs', () => {
  assert.deepEqual(getAvailableCandleSeries(capabilities).map(({ symbol, interval }) => ({ symbol, interval })), [
    { symbol: 'BTCUSDT', interval: '5m' },
  ]);
  assert.deepEqual(getAvailableCandleSeries(null), []);
  assert.deepEqual(getAvailableCandleSeries({ candleSeries: 'invalid' }), []);
  assert.deepEqual(getAvailableCandleSeries({ candleSeries: [{ ...capabilities.candleSeries[0], candleCount: 2 }] }), []);
  assert.deepEqual(getAvailableCandleSeries({ candleSeries: [{ ...capabilities.candleSeries[0], latestCandleTimeMs: null }] }), []);
  assert.deepEqual(getAvailableCandleSeries({ candleSeries: [{ ...capabilities.candleSeries[0], latestClosedBarAtMs: null }] }), []);
  assert.deepEqual(getAvailableCandleSeries({ candleSeries: [{ ...capabilities.candleSeries[0], retrievedAtMs: -1 }] }), []);
});

test('typed series request normalizes symbol but does not silently substitute another series', () => {
  assert.deepEqual(buildCandleSeriesRequest(capabilities, ' btcusdt ', '5m'), { symbol: 'BTCUSDT', interval: '5m' });
  assert.throws(() => buildCandleSeriesRequest(capabilities, 'ETHUSDT', '5m'), /Unsupported candle series ETHUSDT 5m/);
  assert.throws(() => buildCandleSeriesRequest(capabilities, 'BTCUSDT', '1h'), /hourly candles have not been loaded/);
  assert.throws(() => buildCandleSeriesRequest({ candleSeries: [{ ...capabilities.candleSeries[0], candleCount: 3 }] }, 'BTCUSDT', '5m'), /not currently validated as available/);
  assert.throws(() => buildCandleSeriesRequest(capabilities, 'BTCUSDT', '7m'), /Unsupported candle interval/);
  assert.throws(() => buildCandleSeriesRequest(capabilities, 'BTC/USDT', '5m'), /symbol is malformed/);
});

test('order chart alignment is an explicit relationship, not inferred from quote coverage', () => {
  const series = { symbol: 'BTCUSDT', interval: '5m' };
  assert.equal(isOrderChartAligned('BTCUSDT', series), true);
  assert.equal(isOrderChartAligned('ETHUSDT', series), false);
  assert.equal(isOrderChartAligned(' btcusdt ', series), true);
  assert.equal(isOrderChartAligned('BTCUSDT', null), false);
});

test('candle response accepts an exact request with validated source, OHLC and cadence', () => {
  const response = validResponse();
  const validated = validateCandleSeriesResponse(response, { symbol: ' btcusdt ', interval: '5m' });
  assert.notEqual(validated, response, 'validation must detach the trusted snapshot from the bridge object');
  assert.deepEqual(validated.request, { symbol: 'BTCUSDT', interval: '5m' });
  assert.equal(Object.isFrozen(validated), true);
  assert.equal(Object.isFrozen(validated.request), true);
  assert.equal(Object.isFrozen(validated.candles), true);
  assert.equal(Object.isFrozen(validated.candles[0]), true);
  assert.equal(isValidatedCandleSeriesSnapshot(validated), true);
  assert.equal(isValidatedCandleSeriesSnapshot({ ...validated }), false);
  assert.equal(validated.candles[0].close, response.candles[0].close);
  assert.equal(validated.datasetSha256, response.datasetSha256);
});




test('validated live snapshot freshness is re-evaluated at consumption time', () => {
  const validated = validateCandleSeriesResponse(validResponse(), { symbol: 'BTCUSDT', interval: '5m' });
  const now = Date.now();
  assert.equal(isCurrentLiveCandleSeriesSnapshot(validated, now), true);
  assert.equal(isCurrentLiveCandleSeriesSnapshot(validated, now + 120_001), false);
  assert.equal(isCurrentLiveCandleSeriesSnapshot({ ...validated }, now), false);
  assert.equal(isCurrentLiveCandleSeriesSnapshot(validated, validated.latestClosedBarAtMs - 1), false);
});

test('candle response requires a canonical lowercase SHA-256 dataset fingerprint', () => {
  assert.throws(() => validateCandleSeriesResponse(validResponse({ datasetSha256: undefined }), { symbol: 'BTCUSDT', interval: '5m' }), /dataset fingerprint/);
  assert.throws(() => validateCandleSeriesResponse(validResponse({ datasetSha256: 'A'.repeat(64) }), { symbol: 'BTCUSDT', interval: '5m' }), /dataset fingerprint/);
  assert.throws(() => validateCandleSeriesResponse(validResponse({ datasetSha256: 'f'.repeat(63) }), { symbol: 'BTCUSDT', interval: '5m' }), /dataset fingerprint/);
});

test('validated candle snapshot is isolated from post-validation mutation of bridge payload', () => {
  const response = validResponse();
  const validated = validateCandleSeriesResponse(response, { symbol: 'BTCUSDT', interval: '5m' });
  const originalClose = validated.candles[0].close;
  response.candles[0].close = 0.001;
  response.candles.push({ time: Date.now(), open: 1, high: 2, low: 1, close: 1, volume: 1 });
  response.request.symbol = 'ETHUSDT';
  response.source = 'untrusted replacement';
  assert.equal(validated.candles[0].close, originalClose);
  assert.equal(validated.candles.length, 30);
  assert.equal(validated.request.symbol, 'BTCUSDT');
  assert.equal(validated.source, 'Public provider');
  assert.throws(() => { validated.candles[0].close = 0.001; }, TypeError);
  assert.throws(() => { validated.candles.push({}); }, TypeError);
});

test('candle response fails closed on request mismatch, short series, malformed OHLC, duplicate times or wrong cadence', () => {
  assert.throws(() => validateCandleSeriesResponse(validResponse({ request: { symbol: 'ETHUSDT', interval: '5m' } }), { symbol: 'BTCUSDT', interval: '5m' }), /did not match requested series/);
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: makeCandles(29) }), { symbol: 'BTCUSDT', interval: '5m' }), /fewer than 30 candles/);
  const malformed = makeCandles(); malformed[10].high = 1;
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: malformed }), { symbol: 'BTCUSDT', interval: '5m' }), /OHLC price-range invariants/);
  const unsorted = makeCandles(); unsorted[10].time = unsorted[9].time;
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: unsorted }), { symbol: 'BTCUSDT', interval: '5m' }), /strictly increasing/);
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: makeCandles(30, 600_000) }), { symbol: 'BTCUSDT', interval: '5m' }), /cadence gap/);
  const missingInterval = makeCandles();
  for (let index = 11; index < missingInterval.length; index += 1) missingInterval[index].time += 300_000;
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: missingInterval }), { symbol: 'BTCUSDT', interval: '5m' }), /cadence gap.*1 candle interval\(s\) missing/);
  const irregularInterval = makeCandles();
  for (let index = 11; index < irregularInterval.length; index += 1) irregularInterval[index].time += 90_000;
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: irregularInterval }), { symbol: 'BTCUSDT', interval: '5m' }), /not aligned to the 5m UTC interval grid/);
});

test('candle response rejects timestamps that are strictly increasing but off the UTC interval grid', () => {
  const candles = makeCandles();
  candles[0].time += 1;
  assert.throws(
    () => validateCandleSeriesResponse(validResponse({ candles }), { symbol: 'BTCUSDT', interval: '5m' }),
    /not aligned to the 5m UTC interval grid/,
  );
});

test('candle response rejects stale or disconnected claims of live data', () => {
  assert.throws(() => validateCandleSeriesResponse(validResponse({ providerStatus: 'PARTIAL · provider warning' }), { symbol: 'BTCUSDT', interval: '5m' }), /without a connected and fresh provider receipt/);
  const staleReceipt = Date.now() - 180_000;
  const futureReceipt = Date.now() + 60_000;
  assert.throws(() => validateCandleSeriesResponse(validResponse({ updatedAtMs: staleReceipt, retrievedAtMs: staleReceipt }), { symbol: 'BTCUSDT', interval: '5m' }), /without a connected and fresh provider receipt/);
  assert.throws(() => validateCandleSeriesResponse(validResponse({ updatedAtMs: futureReceipt, retrievedAtMs: futureReceipt }), { symbol: 'BTCUSDT', interval: '5m' }), /far in the future/);
  const oldCandles = makeCandles().map((candle) => ({ ...candle, time: candle.time - 50 * 300_000 }));
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: oldCandles, latestClosedBarAtMs: oldCandles.at(-1).time + 299_999 }), { symbol: 'BTCUSDT', interval: '5m' }), /latest fully closed bar is stale/);
});

test('recent retrieval cannot make a replayed historical candle series live', () => {
  const candles = makeCandles().map((candle) => ({ ...candle, time: candle.time - 50 * 300_000 }));
  const latestClosedBarAtMs = candles.at(-1).time + 299_999;
  assert.throws(
    () => validateCandleSeriesResponse(validResponse({ candles, latestClosedBarAtMs }), { symbol: 'BTCUSDT', interval: '5m' }),
    /latest fully closed bar is stale/,
  );
});

test('freshly closed bar is measured from close time, not from open time', () => {
  const now = Date.now();
  const latestOpen = Math.floor(now / 300_000) * 300_000 - 300_000;
  const candles = makeCandles().map((candle, index, values) => ({
    ...candle, time: latestOpen - (values.length - 1 - index) * 300_000,
  }));
  const retrievedAtMs = now;
  const latestClosedBarAtMs = candles.at(-1).time + 299_999;
  const validated = validateCandleSeriesResponse(validResponse({ candles, updatedAtMs: retrievedAtMs, retrievedAtMs, latestClosedBarAtMs }), { symbol: 'BTCUSDT', interval: '5m' });
  assert.equal(validated.latestClosedBarAtMs, latestClosedBarAtMs);
});

test('forming bars and conflicting close or retrieval metadata are rejected', () => {
  const candles = makeCandles();
  const forming = candles.map((candle) => ({ ...candle }));
  const intervalStart = Math.floor(Date.now() / 300_000) * 300_000;
  const shift = intervalStart - forming.at(-1).time;
  for (const candle of forming) candle.time += shift;
  const retrieval = Date.now();
  assert.throws(() => validateCandleSeriesResponse(validResponse({
    candles: forming, updatedAtMs: retrieval, retrievedAtMs: retrieval, latestClosedBarAtMs: forming.at(-1).time + 299_999,
  }), { symbol: 'BTCUSDT', interval: '5m' }), /forming bar or a future close/);
  assert.throws(() => validateCandleSeriesResponse(validResponse({ latestClosedBarAtMs: Date.now() }), { symbol: 'BTCUSDT', interval: '5m' }), /does not match/);
  assert.throws(() => validateCandleSeriesResponse(validResponse({ updatedAtMs: Date.now(), retrievedAtMs: Date.now() - 5_000 }), { symbol: 'BTCUSDT', interval: '5m' }), /missing valid source/);
});

test('candle response requires valid provenance metadata and finite values', () => {
  assert.throws(() => validateCandleSeriesResponse(validResponse({ source: '' }), { symbol: 'BTCUSDT', interval: '5m' }), /missing valid source/);
  assert.throws(() => validateCandleSeriesResponse(validResponse({ providerStatus: ' ' }), { symbol: 'BTCUSDT', interval: '5m' }), /missing valid source/);
  const invalidValue = makeCandles(); invalidValue[3].close = Number.NaN;
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: invalidValue }), { symbol: 'BTCUSDT', interval: '5m' }), /non-finite OHLCV field/);
  const fractionalTime = makeCandles(); fractionalTime[4].time += 0.5;
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: fractionalTime }), { symbol: 'BTCUSDT', interval: '5m' }), /non-finite OHLCV field/);
  const futureCandle = makeCandles(); futureCandle[futureCandle.length - 1].time = Date.now() + 90_000;
  assert.throws(() => validateCandleSeriesResponse(validResponse({ candles: futureCandle }), { symbol: 'BTCUSDT', interval: '5m' }), /far in the future/);
});
