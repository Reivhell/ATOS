import test from 'node:test';
import assert from 'node:assert/strict';
import { auditCandleSeries, CANDLE_SERIES_AUDIT_CONTRACT, MAX_CANDLE_SERIES_AUDIT_CANDLES } from '../src/lib/domain/market-data/candle-series-audit.js';
import { CANDLE_INTERVALS, CANDLE_INTERVAL_MS, getCandleIntervalMs, getLatestClosedBarAtMs } from '../src/lib/domain/market-data/timeframe-contract.js';
import { buildTechnicalAnalysisSnapshot } from '../src/lib/engines/technical-analysis/analysis-engine.js';

const START = Math.floor(1_760_000_100_000 / 300_000) * 300_000;

function makeCandles(count = 100, { start = START, interval = 300_000, flat = false } = {}) {
  const rows = [];
  let previous = 100;
  for (let index = 0; index < count; index += 1) {
    const close = flat ? 100 : 100 + Math.sin(index * 0.37) * 0.4 + index * 0.03;
    const open = flat ? 100 : previous;
    rows.push({
      time: start + index * interval,
      open,
      high: Math.max(open, close) + (flat ? 0 : 0.12),
      low: Math.min(open, close) - (flat ? 0 : 0.12),
      close,
      volume: 10 + (index % 5),
    });
    previous = close;
  }
  return rows;
}

function errorCodes(audit) {
  return audit.hardIssues.map((issue) => issue.code);
}

test('canonical timeframe table is one-to-one and retains UTC candle intervals', () => {
  assert.deepEqual(CANDLE_INTERVALS, Object.keys(CANDLE_INTERVAL_MS));
  assert.equal(new Set(CANDLE_INTERVALS).size, CANDLE_INTERVALS.length);
  assert.deepEqual(CANDLE_INTERVALS.map(getCandleIntervalMs), CANDLE_INTERVALS.map((interval) => CANDLE_INTERVAL_MS[interval]));
  assert.equal(getCandleIntervalMs('2m'), null);
  assert.equal(getCandleIntervalMs(null), null);
  const finalOpen = START + 99 * 300_000;
  assert.equal(getLatestClosedBarAtMs(makeCandles(), '5m'), finalOpen + 299_999);
  assert.equal(getLatestClosedBarAtMs(makeCandles(), '2m'), null);
  assert.equal(getLatestClosedBarAtMs([], '5m'), null);
});

test('a valid contiguous UTC-grid-aligned series receives a clean audit', () => {
  const candles = makeCandles();
  const audit = auditCandleSeries(candles, '5m');
  assert.equal(audit.contractVersion, CANDLE_SERIES_AUDIT_CONTRACT);
  assert.equal(audit.status, 'clean');
  assert.deepEqual(audit.hardIssues, []);
  assert.deepEqual(audit.warnings, []);
  assert.equal(audit.metrics.firstCandleTimeMs, candles[0].time);
  assert.equal(audit.metrics.lastCandleTimeMs, candles.at(-1).time);
  assert.equal(audit.metrics.expectedSpanMs, (candles.length - 1) * 300_000);
});

test('audit fails closed on unsupported timeframe, missing timestamps, grid drift, gaps and order errors', () => {
  assert.ok(errorCodes(auditCandleSeries(makeCandles(), '2m')).includes('unsupported_timeframe'));
  const missing = makeCandles();
  delete missing[7].time;
  assert.ok(errorCodes(auditCandleSeries(missing, '5m')).includes('invalid_timestamp'));
  const unaligned = makeCandles();
  unaligned[8].time += 1_000;
  assert.ok(errorCodes(auditCandleSeries(unaligned, '5m')).includes('unaligned_timestamp'));
  const gap = makeCandles();
  gap[50].time += 300_000;
  for (let index = 51; index < gap.length; index += 1) gap[index].time += 300_000;
  assert.ok(errorCodes(auditCandleSeries(gap, '5m')).includes('cadence_gap'));
  const duplicate = makeCandles();
  duplicate[50].time = duplicate[49].time;
  assert.ok(errorCodes(auditCandleSeries(duplicate, '5m')).includes('non_increasing_timestamp'));
});

test('series structural errors disable all engine metrics and directional signals', () => {
  const candles = makeCandles();
  candles[55].time += 300_000;
  for (let index = 56; index < candles.length; index += 1) candles[index].time += 300_000;
  const result = buildTechnicalAnalysisSnapshot({
    candles,
    symbol: 'BTCUSDT',
    timeframe: '5m',
    marketData: { validated: true, source: 'test-fixture', asOf: new Date(candles.at(-1).time + 299_999).toISOString(), latestClosedBarAtMs: candles.at(-1).time + 299_999, datasetSha256: 'd'.repeat(64) },
    intent: 'research',
  });
  assert.equal(result.status, 'invalid_data');
  assert.equal(result.dataQuality.valid, false);
  assert.equal(result.dataQuality.audit.status, 'invalid');
  assert.equal(result.indicators.metrics, null);
  assert.deepEqual(result.signals, []);
  assert.equal(result.decision.decision, 'BLOCK');
  assert.equal(result.authority.executionAuthority, 'none');
});

test('latest-bar return and volume outliers are visible advisory warnings, not automatic rejection', () => {
  const candles = makeCandles(100);
  const previous = candles.at(-1).close;
  const close = previous * 1.12;
  candles.push({
    time: candles.at(-1).time + 300_000,
    open: previous,
    high: close * 1.002,
    low: previous * 0.998,
    close,
    volume: 10_000,
  });
  const audit = auditCandleSeries(candles, '5m');
  assert.equal(audit.status, 'warning');
  assert.deepEqual(audit.hardIssues, []);
  assert.ok(audit.warnings.some((warning) => warning.code === 'latest_price_return_outlier'));
  assert.ok(audit.warnings.some((warning) => warning.code === 'latest_volume_outlier'));
  assert.ok(Number.isFinite(audit.metrics.latestCloseReturnPct));
  assert.ok(audit.metrics.latestVolumeToMedianMultiple > 8);

  const result = buildTechnicalAnalysisSnapshot({
    candles,
    symbol: 'BTCUSDT',
    timeframe: '5m',
    marketData: { validated: true, source: 'test-fixture', asOf: new Date(candles.at(-1).time + 299_999).toISOString(), latestClosedBarAtMs: candles.at(-1).time + 299_999, datasetSha256: 'e'.repeat(64) },
    intent: 'research',
  });
  assert.equal(result.status, 'partial');
  assert.equal(result.dataQuality.valid, true);
  assert.equal(result.dataQuality.audit.status, 'warning');
  assert.equal(result.decision.decision, 'BLOCK');
  assert.ok(result.decision.reasons.some((reason) => reason.includes('candidate handoff is blocked')));
  assert.equal(result.dataQuality.audit.warnings.length, 2);
  assert.ok(result.dataQuality.issues.some((issue) => issue.includes('possible genuine shock')));
  assert.equal(result.authority.liveExecutionAuthorized, false);
});

test('20 identical flat-price bars are flagged without fabricating a trend signal', () => {
  const candles = makeCandles(40, { flat: true });
  const audit = auditCandleSeries(candles, '5m');
  assert.equal(audit.status, 'warning');
  assert.ok(audit.warnings.some((warning) => warning.code === 'latest_price_flatline'));
});

test('audit work and recorded fault state are bounded on excessive and malformed input', () => {
  const oversized = makeCandles(MAX_CANDLE_SERIES_AUDIT_CANDLES + 1);
  const audit = auditCandleSeries(oversized, '5m');
  assert.equal(audit.status, 'invalid');
  assert.ok(errorCodes(audit).includes('series_too_large'));

  const malformed = makeCandles(100);
  for (let index = 0; index < malformed.length; index += 1) malformed[index].time += index * 1_000;
  const denseAudit = auditCandleSeries(malformed, '5m');
  assert.equal(denseAudit.hardIssues.length, 64);
});
