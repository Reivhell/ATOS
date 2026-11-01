import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const rust = read('../src-tauri/src/domain/technical_analysis.rs');
const commands = read('../src-tauri/src/adapters/tauri/commands.rs');
const bootstrap = read('../src-tauri/src/adapters/tauri/bootstrap.rs');
const bridge = read('../src/lib/adapters/tauri/bridge.ts');
const app = read('../src/App.svelte');

test('indicator calculations live in the Rust domain and enforce bounded valid OHLCV input', () => {
  assert.match(rust, /pub fn calculate_readings\(candles: &\[Candle\]\)/);
  assert.match(rust, /const MAX_CANDLES: usize = 10_000/);
  assert.match(rust, /malformed_ohlc_fails_closed_for_every_indicator/);
  assert.match(rust, /insufficient_history_is_reported_without_fabricated_values/);
  assert.match(rust, /excessive_input_is_rejected/);
});

test('production UI receives indicator readings through a registered typed native command', () => {
  assert.match(commands, /calculate_technical_indicator_readings/);
  assert.match(bootstrap, /commands::calculate_technical_indicator_readings/);
  assert.match(bridge, /invoke<TechnicalIndicatorReading\[]>\('calculate_technical_indicator_readings'/);
  assert.match(app, /await calculateTechnicalIndicatorReadings\(series\.candles\)/);
  assert.doesNotMatch(app, /technicalReadings = technicalEnvelope\.indicatorReadings/);
  assert.match(app, /indicatorRequestEpoch/);
});

test('market structure summaries are calculated in Rust and rendered from the native response', () => {
  assert.match(rust, /pub fn calculate_structure\(candles: &\[Candle\]\)/);
  assert.match(rust, /classify_confirmed_swings\(candles, 2, 2\)/);
  assert.match(rust, /detect_confirmed_structure_breaks\(candles, 2, 2\)/);
  assert.match(commands, /calculate_native_market_structure/);
  assert.match(bootstrap, /commands::calculate_native_market_structure/);
  assert.match(bridge, /invoke<NativeStructureSnapshot>\('calculate_native_market_structure'/);
  assert.match(app, /calculateNativeMarketStructure\(series\.candles\)/);
  assert.match(app, /nativeStructure\?\.bias/);
  assert.doesNotMatch(app, /<strong>\{technicalEnvelope\.structure\.bias\}<\/strong>/);
});
