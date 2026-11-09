import test from 'node:test';
import assert from 'node:assert/strict';
import { createTechnicalAnalysisReport } from '../src/lib/application/technical-analysis-service.js';
import { validateCandleSeriesResponse } from '../src/lib/domain/market-data/market-data-model.js';

function makeSeries(count = 80, isLive = true) {
  const intervalMs = 300_000;
  const now = Date.now();
  const latestClosedBarOpen = Math.floor(now / intervalMs) * intervalMs - intervalMs;
  const start = latestClosedBarOpen - (count - 1) * intervalMs;
  let previous = 100;
  const candles = Array.from({ length: count }, (_, index) => {
    const close = 100 + index * 0.1 + Math.sin(index * 0.4);
    const open = previous;
    previous = close;
    return { time: start + index * intervalMs, open, high: Math.max(open, close) + 0.5, low: Math.min(open, close) - 0.5, close, volume: 10 + index % 7 };
  });
  return validateCandleSeriesResponse({
    request: { symbol: 'BTCUSDT', interval: '5m' }, source: 'fixture-provider', providerStatus: 'CONNECTED · FIXTURE',
    updatedAtMs: now, retrievedAtMs: now, latestClosedBarAtMs: candles.at(-1).time + intervalMs - 1,
    datasetSha256: 'a'.repeat(64), isLive, candles,
  }, { symbol: 'BTCUSDT', interval: '5m' });
}

test('application service accepts only the exact validated snapshot and returns a report', () => {
  const validatedSeries = makeSeries();
  const report = createTechnicalAnalysisReport({ validatedSeries, requestedSymbol: 'BTCUSDT', requestedTimeframe: '5m' });
  assert.equal(report.applicationContractVersion, 'technical-analysis-application.v1');
  assert.equal(report.validationBoundary.passed, true);
  assert.equal(report.dataQuality.valid, true);
  assert.equal(report.identity.asOf, new Date(validatedSeries.latestClosedBarAtMs).toISOString());
  assert.equal(report.identity.datasetSha256, validatedSeries.datasetSha256);
  assert.equal(report.validationBoundary.liveSnapshot, true);
  assert.equal(report.indicatorReadings.length, 6);
  assert.equal(report.authority.orderIntent, null);
  assert.equal(report.authority.liveExecutionAuthorized, false);
  assert.ok(!report.decision.reasons.some((reason) => reason.includes('SHA-256 fingerprint')));
});



test('a snapshot that expires during analysis is blocked at the final decision boundary', () => {
  const validatedSeries = makeSeries();
  const originalNow = Date.now;
  const validationTime = originalNow();
  let clockReads = 0;
  Date.now = () => (++clockReads === 1 ? validationTime : validationTime + 120_001);
  try {
    const report = createTechnicalAnalysisReport({ validatedSeries, requestedSymbol: 'BTCUSDT', requestedTimeframe: '5m' });
    assert.equal(report.validationBoundary.passed, true);
    assert.equal(report.validationBoundary.liveSnapshot, false);
    assert.equal(report.status, 'blocked');
    assert.equal(report.dataQuality.valid, false);
    assert.equal(report.decision.decision, 'BLOCK');
    assert.ok(report.dataQuality.issues.some((issue) => issue.includes('expired')));
  } finally {
    Date.now = originalNow;
  }
});

test('historical or stale snapshots may be inspected but cannot enter current research decisioning', () => {
  const validatedSeries = makeSeries(80, false);
  const report = createTechnicalAnalysisReport({ validatedSeries, requestedSymbol: 'BTCUSDT', requestedTimeframe: '5m' });
  assert.equal(report.validationBoundary.passed, true);
  assert.equal(report.validationBoundary.liveSnapshot, false);
  assert.equal(report.status, 'blocked');
  assert.equal(report.dataQuality.valid, false);
  assert.equal(report.decision.decision, 'BLOCK');
  assert.equal(report.identity.datasetSha256, validatedSeries.datasetSha256);
  assert.equal(report.authority.executionAuthority, 'none');
});

test('object-shaped but unvalidated market data cannot be promoted by the application service', () => {
  const spoofed = makeSeries();
  const fake = { ...spoofed, candles: [...spoofed.candles] };
  const report = createTechnicalAnalysisReport({ validatedSeries: fake, requestedSymbol: 'BTCUSDT', requestedTimeframe: '5m' });
  assert.equal(report.status, 'blocked');
  assert.equal(report.validationBoundary.trustedSnapshot, false);
  assert.equal(report.dataQuality.valid, false);
  assert.equal(report.decision.decision, 'BLOCK');
  assert.ok(report.dataQuality.issues.some((issue) => issue.includes('immutable candle snapshot')));
});

test('requested symbol or timeframe mismatch blocks the report', () => {
  const validatedSeries = makeSeries();
  const report = createTechnicalAnalysisReport({ validatedSeries, requestedSymbol: 'ETHUSDT', requestedTimeframe: '1h' });
  assert.equal(report.status, 'blocked');
  assert.equal(report.validationBoundary.scopeMatches, false);
  assert.equal(report.decision.decision, 'BLOCK');
  assert.ok(report.dataQuality.issues.some((issue) => issue.includes('does not match')));
});

test('empty input produces an explicit blocked report rather than pretending analysis ran', () => {
  const report = createTechnicalAnalysisReport({ validatedSeries: null, requestedSymbol: 'BTCUSDT', requestedTimeframe: '5m' });
  assert.equal(report.status, 'blocked');
  assert.equal(report.indicatorReadings.length, 6);
  assert.equal(report.dataQuality.valid, false);
  assert.equal(report.decision.orderIntent, null);
});
