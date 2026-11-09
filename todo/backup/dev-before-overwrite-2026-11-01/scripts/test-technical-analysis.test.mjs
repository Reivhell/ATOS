import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  calculateIndicatorReadings,
  TECHNICAL_ANALYSIS_CATALOG,
  TECHNICAL_ANALYSIS_CATEGORIES,
  TECHNICAL_ANALYSIS_EXPECTED_CATEGORY_COUNTS,
  TECHNICAL_ANALYSIS_EXPECTED_ENTRY_COUNT,
  TECHNICAL_ANALYSIS_MASTER_LIST_VERSION,
} from '../src/lib/technical-analysis.js';

const masterListManifest = JSON.parse(readFileSync(new URL('./fixtures/technical-analysis-master-list.v1.json', import.meta.url), 'utf8'));
const manifestRecords = masterListManifest.entries.map(({ id, category, group, name, availability, kind }) => ({ id, category, group, name, availability, kind }));

function makeCandles(count, closeAt = (i) => 100 + i) {
  return Array.from({ length: count }, (_, i) => {
    const close = closeAt(i);
    return {
      time: 1_700_000_000_000 + i * 60_000,
      open: close - 0.5,
      high: close + 2,
      low: close - 2,
      close,
      volume: 100 + i,
    };
  });
}

test('strict catalog exactly matches the reviewed master-list inventory snapshot', () => {
  assert.equal(TECHNICAL_ANALYSIS_MASTER_LIST_VERSION, '1.0.0');
  assert.equal(masterListManifest.masterListVersion, TECHNICAL_ANALYSIS_MASTER_LIST_VERSION);
  assert.equal(TECHNICAL_ANALYSIS_EXPECTED_ENTRY_COUNT, 364);
  assert.equal(TECHNICAL_ANALYSIS_CATALOG.length, TECHNICAL_ANALYSIS_EXPECTED_ENTRY_COUNT);
  assert.equal(masterListManifest.expectedEntryCount, TECHNICAL_ANALYSIS_EXPECTED_ENTRY_COUNT);
  const actualRecords = TECHNICAL_ANALYSIS_CATALOG.map(({ id, category, group, name, availability, kind }) => ({ id, category, group, name, availability, kind }));
  assert.deepEqual(actualRecords, manifestRecords, 'any add/remove/rename/reorder/status change requires a reviewed manifest update');
});

test('strict catalog preserves all nine requested top-level category names and exact counts', () => {
  assert.deepEqual(TECHNICAL_ANALYSIS_CATEGORIES, [
    'SMC & ICT', 'Price Action', 'Support, Resistance & Trend', 'Indikator Teknikal',
    'Order Flow & Market Profile', 'Divergence & Intermarket Analysis',
    'Metode Analisis Lain', 'Fundamental & Makro', 'Risk Management & Evaluation',
  ]);
  const categoryCounts = Object.fromEntries(TECHNICAL_ANALYSIS_CATEGORIES.map((category) => [
    category, TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.category === category).length,
  ]));
  assert.deepEqual(categoryCounts, TECHNICAL_ANALYSIS_EXPECTED_CATEGORY_COUNTS);
  assert.deepEqual(categoryCounts, masterListManifest.categoryCounts);
  assert.deepEqual(masterListManifest.categories, TECHNICAL_ANALYSIS_CATEGORIES);
  const names = new Set(TECHNICAL_ANALYSIS_CATALOG.map((item) => item.name));
  for (const required of [
    'BOS (Break of Structure)', 'CHoCH (Change of Character)', 'FVG (Fair Value Gap)',
    'SMT Divergence', 'RSI (Relative Strength Index)', 'MACD (Moving Average Convergence Divergence)',
    'Footprint Chart', 'Spring', 'Gartley', 'FOMC (Federal Open Market Committee)',
    'Monte Carlo Simulation', 'Maximum Daily Loss', 'Market Order', 'Limit Order', 'Stop Order',
  ]) assert.ok(names.has(required), `missing master-list concept: ${required}`);
});

test('catalog identifiers are unique and only the requested SMT Divergence cross-listing repeats a name', () => {
  const ids = TECHNICAL_ANALYSIS_CATALOG.map((item) => item.id);
  assert.equal(new Set(ids).size, ids.length);
  const byName = new Map();
  for (const item of TECHNICAL_ANALYSIS_CATALOG) byName.set(item.name, [...(byName.get(item.name) ?? []), item]);
  const duplicates = [...byName].filter(([, entries]) => entries.length > 1);
  assert.deepEqual(duplicates.map(([name]) => name), ['SMT Divergence']);
  assert.deepEqual(duplicates[0][1].map(({ category }) => category), ['SMC & ICT', 'Divergence & Intermarket Analysis']);
});

test('implementation coverage is explicit and totals six calculations, one workflow, and 357 catalog-only entries', () => {
  const counts = Object.fromEntries(['Calculated now', 'Workflow available', 'Catalog only'].map((status) => [
    status, TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === status).length,
  ]));
  assert.deepEqual(counts, { 'Calculated now': 6, 'Workflow available': 1, 'Catalog only': 357 });
  assert.ok(TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Calculated now').every((item) => item.available && item.kind === 'indicator'));
  assert.ok(TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Workflow available').every((item) => item.name === 'Backtesting' && item.available && item.kind === 'workflow'));
  assert.ok(TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Catalog only').every((item) => !item.available && item.kind === 'research concept'));
});

test('indicator calculator produces finite readings for a valid 60-candle rising series', () => {
  const readings = calculateIndicatorReadings(makeCandles(60));
  assert.equal(readings.length, 6);
  assert.ok(readings.every((reading) => reading.available), 'all six readings should be available');
  assert.match(readings.find((reading) => reading.key === 'rsi').value, /100/);
  assert.match(readings.find((reading) => reading.key === 'atr').value, /4/);
  assert.match(readings.find((reading) => reading.key === 'bollinger').value, /–/);
  for (const reading of readings) assert.ok(reading.detail.length > 0);
});

test('SMA and Bollinger outputs match a known rising-series reference', () => {
  const readings = calculateIndicatorReadings(makeCandles(60));
  assert.equal(readings.find((reading) => reading.key === 'sma').value, '155 / 149');
  assert.equal(readings.find((reading) => reading.key === 'bollinger').value, '137.9674 – 161.0326');
});

test('insufficient history never fabricates a complete indicator reading', () => {
  const readings = calculateIndicatorReadings(makeCandles(10));
  assert.equal(readings.length, 6);
  assert.ok(readings.every((reading) => !reading.available));
  assert.ok(readings.every((reading) => reading.value === 'Unavailable'));
  assert.ok(readings.every((reading) => reading.detail.includes('Need at least')));
});

test('malformed candles fail closed instead of displaying indicator output', () => {
  const candles = makeCandles(60);
  candles[32] = { ...candles[32], high: Number.NaN };
  const readings = calculateIndicatorReadings(candles);
  assert.ok(readings.every((reading) => !reading.available));
  assert.ok(readings.every((reading) => reading.value === 'Unavailable'));
});

test('flat-price RSI is neutral, while ATR and MACD remain finite', () => {
  const readings = calculateIndicatorReadings(makeCandles(60, () => 100));
  assert.equal(readings.find((reading) => reading.key === 'rsi').value, '50');
  assert.equal(readings.find((reading) => reading.key === 'atr').value, '4');
  assert.equal(readings.find((reading) => reading.key === 'macd').value, '0');
});
