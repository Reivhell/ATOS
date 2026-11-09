import test from 'node:test';
import assert from 'node:assert/strict';
import { auditEventsToCsv, backtestReportToJson } from '../src/lib/infrastructure/serialization/exports.js';

const event = (overrides = {}) => ({
  id: 'event-123', timestampMs: Date.UTC(2026, 9, 27, 1, 2, 3),
  level: 'WARN', category: 'risk', title: 'Rejected, safely',
  detail: 'The operator said "no"\nand the action stayed blocked.',
  ...overrides,
});

test('audit CSV includes a UTF-8 BOM, stable header, UTC timestamp, and CRLF rows', () => {
  const csv = auditEventsToCsv([event()]);
  assert.ok(csv.startsWith('\uFEFF"timestamp_utc","event_id","level","category","title","detail"\r\n'));
  assert.ok(csv.includes('"2026-10-27T01:02:03.000Z"'));
  assert.ok(csv.endsWith('\r\n'));
});

test('audit CSV correctly escapes commas, quotes, and embedded newlines', () => {
  const csv = auditEventsToCsv([event()]);
  assert.ok(csv.includes('"Rejected, safely"'));
  assert.ok(csv.includes('"The operator said ""no""\nand the action stayed blocked."'));
});

test('audit CSV neutralizes spreadsheet formula injection in text fields', () => {
  for (const injected of ['=HYPERLINK("https://example.invalid")', ' +CMD', '-1+1', '@SUM(A1:A2)']) {
    const csv = auditEventsToCsv([event({ detail: injected })]);
    assert.ok(csv.includes(`"'${injected.replace(/"/g, '""')}"`), `expected formula marker to be escaped: ${injected}`);
  }
});

test('audit CSV handles empty input and invalid timestamps without throwing', () => {
  const empty = auditEventsToCsv([]);
  assert.ok(empty.includes('"timestamp_utc"'));
  assert.equal(empty.split('\r\n').length, 2);
  assert.ok(auditEventsToCsv([event({ timestampMs: Number.NaN })]).split('\r\n')[1].startsWith('""'));
  assert.throws(() => auditEventsToCsv(/** @type {any} */ (null)), /array/);
});

test('backtest JSON export is versioned and preserves the complete supplied result', () => {
  const result = {
    id: 'bt-42', symbol: 'BTCUSDT', strategyName: 'SMA crossover', fastPeriod: 9,
    slowPeriod: 21, initialCapital: 10000, finalEquity: 10125, totalReturnPct: 1.25,
    maxDrawdownPct: 2.5, winRatePct: 50, profitFactor: 1.4, sharpeRatio: 0.7,
    tradeCount: 4, feeBps: 10, slippageBps: 5, candlesUsed: 120,
    dataSource: 'Fixture data', datasetSha256: 'a'.repeat(64), datasetEncodingVersion: 1,
    datasetStartMs: 1000, datasetEndMs: 35695000, medianCandleIntervalMs: 300000,
    equityCurve: [{ time: 1, equity: 10000, benchmark: 10000 }],
    warnings: ['Research only'],
  };
  const parsed = JSON.parse(backtestReportToJson(result, '2026-10-27T00:00:00.000Z'));
  assert.equal(parsed.format, 'agentic-trading-os.backtest-report');
  assert.equal(parsed.formatVersion, 2);
  assert.equal(parsed.exportedAtUtc, '2026-10-27T00:00:00.000Z');
  assert.deepEqual(parsed.result, result);
  assert.throws(() => backtestReportToJson(/** @type {any} */ (null)), /completed backtest/);
});
