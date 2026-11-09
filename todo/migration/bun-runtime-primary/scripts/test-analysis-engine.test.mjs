import test from 'node:test';
import assert from 'node:assert/strict';
import { buildTechnicalAnalysisSnapshot, MAX_ANALYSIS_CANDLES } from '../src/lib/engines/technical-analysis/analysis-engine.js';
import { calculateIndicatorSnapshot } from '../src/lib/engines/technical-analysis/indicators.js';
import { getLatestClosedBarAtMs } from '../src/lib/domain/market-data/timeframe-contract.js';

function makeCandles(count = 100, slope = 0.35) {
  const candles = [];
  let previous = 100;
  const start = 1_760_000_100_000;
  for (let index = 0; index < count; index += 1) {
    const close = 100 + index * slope + Math.sin(index * 0.45) * 1.2;
    const open = previous;
    candles.push({
      time: start + index * 300_000,
      open,
      high: Math.max(open, close) + 0.7,
      low: Math.min(open, close) - 0.6,
      close,
      volume: 10 + (index % 9),
    });
    previous = close;
  }
  return candles;
}

const snapshotContext = (candles, extra = {}) => ({
  candles,
  symbol: 'BTCUSDT',
  timeframe: '5m',
  marketData: {
    validated: true,
    source: 'test-fixture',
    asOf: new Date(getLatestClosedBarAtMs(candles, '5m') ?? 0).toISOString(),
    latestClosedBarAtMs: getLatestClosedBarAtMs(candles, '5m'),
    datasetSha256: 'c'.repeat(64),
  },
  intent: 'research',
  ...extra,
});

test('technical snapshot emits numeric metrics, bounded evidence and a pinned cutoff', () => {
  const candles = makeCandles();
  const result = buildTechnicalAnalysisSnapshot(snapshotContext(candles));
  assert.equal(result.contractVersion, 'technical-analysis-envelope.v2');
  assert.equal(result.engineVersion, 'layered-ta-engine.v2');
  assert.equal(result.status, 'ready');
  assert.equal(result.identity.asOf, new Date(candles.at(-1).time + 300_000 - 1).toISOString());
  assert.equal(result.dataQuality.valid, true);
  assert.ok(Number.isFinite(result.indicators.metrics.smaFast));
  assert.ok(result.signals.some((entry) => entry.domain === 'trend'));
  assert.ok(result.signals.some((entry) => entry.domain === 'momentum'));
  assert.ok(result.signals.some((entry) => entry.domain === 'structure'));
  assert.equal(result.authority.orderIntent, null);
  assert.equal(result.authority.liveExecutionAuthorized, false);
});

test('indicator engine rejects malformed OHLCV and never emits directional signals', () => {
  const candles = makeCandles();
  candles[12] = { ...candles[12], high: candles[12].low - 10 };
  const result = calculateIndicatorSnapshot(candles);
  assert.equal(result.status, 'invalid_data');
  assert.equal(result.metrics, null);
  assert.deepEqual(result.signals, []);
  const envelope = buildTechnicalAnalysisSnapshot(snapshotContext(candles));
  assert.equal(envelope.status, 'invalid_data');
  assert.deepEqual(envelope.signals, []);
  assert.equal(envelope.decision.decision, 'BLOCK');
});

test('analytical output remains tied to the requested chronological prefix', () => {
  const candles = makeCandles(110);
  const cutoff = 70;
  const prefix = candles.slice(0, cutoff);
  const prefixResult = buildTechnicalAnalysisSnapshot(snapshotContext(prefix));
  const mutatedFuture = candles.map((candle, index) => index < cutoff ? candle : {
    ...candle, open: candle.open * 4, high: candle.high * 4, low: candle.low * 4, close: candle.close * 4,
  });
  const samePrefixResult = buildTechnicalAnalysisSnapshot(snapshotContext(mutatedFuture.slice(0, cutoff)));
  assert.deepEqual(samePrefixResult.indicators.metrics, prefixResult.indicators.metrics);
  assert.deepEqual(samePrefixResult.structure.breaks, prefixResult.structure.breaks);
  assert.equal(prefixResult.identity.asOf, new Date(prefix.at(-1).time + 300_000 - 1).toISOString());
});

test('missing fingerprint blocks research candidate handoff even when indicators are available', () => {
  const result = buildTechnicalAnalysisSnapshot(snapshotContext(makeCandles(), {
    marketData: { validated: true, source: 'fixture-provider', asOf: new Date(makeCandles().at(-1).time + 300_000 - 1).toISOString(), latestClosedBarAtMs: makeCandles().at(-1).time + 300_000 - 1 },
  }));
  assert.equal(result.status, 'ready');
  assert.equal(result.decision.decision, 'BLOCK');
  assert.equal(result.authority.executionAuthority, 'none');
  assert.ok(result.decision.reasons.some((reason) => reason.includes('SHA-256 fingerprint')));
});

test('analysis input has a hard memory/CPU bound', () => {
  const candles = makeCandles(MAX_ANALYSIS_CANDLES + 1);
  const result = buildTechnicalAnalysisSnapshot(snapshotContext(candles));
  assert.equal(result.status, 'blocked');
  assert.ok(result.dataQuality.issues[0].includes('safety limit'));
  assert.equal(result.decision.orderIntent, null);
});


test('provider cutoff mismatch fails closed rather than silently changing snapshot identity', () => {
  const candles = makeCandles();
  const result = buildTechnicalAnalysisSnapshot(snapshotContext(candles, {
    marketData: {
      validated: true, source: 'fixture-provider', datasetSha256: 'c'.repeat(64),
      asOf: new Date(candles.at(-2).time + 300_000 - 1).toISOString(), latestClosedBarAtMs: candles.at(-2).time + 300_000 - 1,
    },
  }));
  assert.equal(result.identity.asOf, new Date(candles.at(-1).time + 300_000 - 1).toISOString());
  assert.equal(result.decision.decision, 'BLOCK');
  assert.ok(result.dataQuality.issues.some((issue) => issue.includes('cutoff differs')));
});

test('provider symbol or timeframe mismatch fails closed', () => {
  const candles = makeCandles();
  const result = buildTechnicalAnalysisSnapshot(snapshotContext(candles, {
    marketData: {
      validated: true, source: 'fixture-provider', datasetSha256: 'c'.repeat(64),
      asOf: new Date(candles.at(-1).time + 300_000 - 1).toISOString(), latestClosedBarAtMs: candles.at(-1).time + 300_000 - 1, symbol: 'ETHUSDT', timeframe: '1h',
    },
  }));
  assert.equal(result.decision.decision, 'BLOCK');
  assert.ok(result.dataQuality.issues.some((issue) => issue.includes('symbol or timeframe scope differs')));
});
