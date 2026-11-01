import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const rust = read('../src-tauri/src/domain/technical_analysis.rs');
const commands = read('../src-tauri/src/adapters/tauri/commands.rs');
const bootstrap = read('../src-tauri/src/adapters/tauri/bootstrap.rs');
const bridge = read('../src/lib/adapters/tauri/bridge.ts');
const app = read('../src/App.svelte');

test('native analysis shares one OHLCV and timestamp validator across all calculation paths', () => {
  assert.match(rust, /fn validate_candles\(candles: &\[Candle\], context: &str\)/);
  assert.match(rust, /validate_candles\(candles, "Indicator"\)/);
  assert.match(rust, /validate_candles\(candles, "Signal"\)\?/);
  assert.match(rust, /validate_candles\(candles, "Structure"\)\?/);
});

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
  assert.match(app, /calculateTechnicalIndicatorReadings\(series\.candles\)/);
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
  assert.match(app, /nativeStructure\.bias/);
  assert.doesNotMatch(app, /technicalEnvelope\.structure\.bias/);
});

test('strict evidence references are snapshot-bound, bounded, and fail closed in both gates', () => {
  const strictGate = read('../src/lib/engines/decision/strict-decision-gate.js');
  const strictRust = read('../src-tauri/src/application/strict_analysis.rs');
  const schema = read('../docs/agents/strict-ta-assessment.schema.json');
  assert.match(strictGate, /function canonicalEvidenceRefMatchesSnapshot/);
  assert.match(strictGate, /binance\|mt5\|rust-indicator\|rust-structure\|backtest\|fixture/);
  assert.match(strictGate, /refs\.length <= 32/);
  assert.match(strictGate, /ref\.length <= 256/);
  assert.match(strictRust, /fn canonical_evidence_ref_matches_snapshot/);
  assert.match(strictRust, /MAX_EVIDENCE_REFS_PER_ASSESSMENT: usize = 32/);
  assert.match(strictRust, /MAX_EVIDENCE_REF_LENGTH: usize = 256/);
  assert.match(strictRust, /evidence_refs_enforce_count_and_length_limits/);
  assert.match(schema, /"maxItems": 32/);
  assert.match(schema, /"maxLength": 256/);
});

test('Rust generates bounded indicator-domain signals through the native application boundary', () => {
  assert.match(rust, /pub fn calculate_signal_snapshot\(candles: &\[Candle\]\)/);
  assert.match(rust, /native_signal\("trend"/);
  assert.match(rust, /native_signal\("momentum"/);
  assert.match(rust, /native_signal\("location"/);
  assert.match(rust, /native_signal_snapshot_emits_bounded_indicator_domains/);
  assert.match(rust, /native_signal_snapshot_rejects_invalid_ohlcv/);
  assert.match(commands, /calculate_native_indicator_signals/);
  assert.match(bootstrap, /commands::calculate_native_indicator_signals/);
  assert.match(bridge, /invoke<NativeIndicatorSignalSnapshot>\('calculate_native_indicator_signals'/);
  assert.match(app, /calculateNativeIndicatorSignals\(series\.candles\)/);
  assert.match(app, /Rust indicator signals:/);
});
