import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const models = readFileSync(new URL('../src-tauri/src/domain/models.rs', import.meta.url), 'utf8');
const engine = readFileSync(new URL('../src-tauri/src/runtime/trading_core.rs', import.meta.url), 'utf8');
const native = readFileSync(new URL('../src-tauri/src/infrastructure/providers/public_market.rs', import.meta.url), 'utf8');
const frontend = readFileSync(new URL('../src/lib/domain/market-data/market-data-model.js', import.meta.url), 'utf8');
const strictService = readFileSync(new URL('../src-tauri/src/application/strict_analysis_service.rs', import.meta.url), 'utf8');
const registry = readFileSync(new URL('../src-tauri/src/infrastructure/providers/registry.rs', import.meta.url), 'utf8');

 test('Rust has one canonical fail-closed OHLCV interval validator', () => {
  assert.match(models, /pub\(crate\) fn validate_candle_series\(/);
  assert.match(models, /candle\.time % interval_ms != 0/);
  assert.match(models, /candle\.time\.checked_sub\(previous\)/);
  assert.match(models, /observed != interval_ms/);
  assert.match(models, /Candle cadence violation/);
  assert.match(models, /one_internal_gap_is_rejected_even_when_most_intervals_are_valid/);
  assert.match(models, /duplicate_out_of_order_and_unaligned_timestamps_are_rejected/);
});

test('Native market data refuses malformed candles instead of sorting or deduplicating them', () => {
  assert.match(native, /validate_candle_series\(&converted, CandleInterval::FiveMinutes, 30\)/);
  assert.match(native, /Malformed rows must invalidate the candle batch/);
  assert.doesNotMatch(native.slice(native.indexOf('fn parse_closed_binance_candles'), native.indexOf('fn value_to_f64')), /sort_by_key|dedup_by_key/);
  assert.match(engine, /validate_candle_series\(&candles, CandleInterval::FiveMinutes, 30\)/);
  assert.match(engine, /BTCUSDT candle series rejected/);
  assert.match(engine, /validate_candle_series\(&candles, request\.interval, 30\)/);
  assert.match(engine, /let candle_series_available\s*=\s*validate_candle_series\s*\(/);
});

test('Frontend independently enforces the exact UTC candle interval grid', () => {
  assert.match(frontend, /candle\.time % expectedIntervalMs !== 0/);
  assert.match(frontend, /not aligned to the \$\{interval\} UTC interval grid/);
});

// Static source contracts complement, but never replace, native Rust compilation/tests.

test('Rust shares closed-bar freshness and receipt provenance across ingestion, analysis and queries', () => {
  assert.match(models, /pub fn candle_close_at_ms\(/);
  assert.match(models, /pub fn validate_candle_freshness\(/);
  assert.match(models, /has an invalid close timestamp/);
  assert.match(models, /CANDLE_CLOSE_FRESHNESS_GRACE_MS: i64 = 30_000/);
  assert.match(models, /recent_retrieval_does_not_make_a_replayed_bar_fresh/);
  assert.match(engine, /candle_retrieved_at_ms/);
  assert.match(engine, /validate_candle_freshness\(/);
  assert.match(strictService, /validate_candle_freshness\(/);
  assert.match(registry, /latest_closed_bar_at_ms\(&dataset, CandleInterval::FiveMinutes\)/);
  assert.match(registry, /latest_received_at_ms: Some\(series.retrieved_at_ms\)/);
  assert.match(frontend, /response.latestClosedBarAtMs !== expectedLatestClosedBarAtMs/);
  assert.match(frontend, /response.updatedAtMs !== response.retrievedAtMs/);
});
