import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src-tauri/src/application/backtest.rs', import.meta.url), 'utf8');
const cargo = readFileSync(new URL('../src-tauri/Cargo.toml', import.meta.url), 'utf8');

test('backtest cadence validation rejects gaps instead of hiding them behind a median', () => {
  assert.match(source, /expected_interval_ms: Option<i64>/);
  assert.match(source, /checked_sub\(prev\)/);
  assert.match(source, /observed_interval_ms != expected/);
  assert.match(source, /continuous, uniform candle series/);
  assert.doesNotMatch(source, /intervals_ms\.select_nth_unstable/);
});

test('native regression tests cover missing intervals, irregular cadence, and timestamp overflow', () => {
  assert.match(source, /fn backtest_rejects_missing_or_irregular_candle_intervals\(\)/);
  assert.match(source, /fn backtest_rejects_timestamp_delta_overflow\(\)/);
  assert.match(source, /approximately 1 interval\(s\) missing/);
});

test('backtest cadence policy documents the continuous-market limitation', () => {
  assert.match(source, /session-aware instruments need a/);
  assert.match(cargo, /version = "0\.0\.1-beta"/);
});


test('benchmark liquidation costs are unconditional on strategy position state', () => {
  assert.match(source, /fn benchmark_liquidation_costs_apply_when_strategy_is_flat\(\)/);
  assert.match(source, /Benchmark final liquidation exceeded the supported numeric range/);
  const strategyLiquidation = source.indexOf('if in_position {');
  const benchmarkLiquidation = source.indexOf('// The buy-and-hold benchmark is liquidated at the final close regardless');
  assert.ok(strategyLiquidation >= 0 && benchmarkLiquidation > strategyLiquidation,
    'benchmark liquidation must be performed after the strategy-position conditional');
});


test('backtest unavailable statistics are not serialized or displayed as false zeros/infinities', () => {
  assert.match(source, /let win_rate = if trade_pnls\.is_empty\(\) \{\s*None\s*\}\s*else\s*\{\s*Some\(/);
  assert.match(source, /gross_profit:\s*gains,\s*gross_loss:\s*losses/);
  assert.match(source, /no closed strategy trades exist/);
  assert.match(source, /break-even trades as non-winners/);
  const models = readFileSync(new URL('../src-tauri/src/domain/models.rs', import.meta.url), 'utf8');
  assert.match(models, /pub win_rate_pct: Option<f64>/);
  assert.match(models, /pub gross_profit: f64,[\s\S]*?pub gross_loss: f64/);
  const app = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
  assert.match(app, /backtestResult\.tradeCount > 0 && backtestResult\.winRatePct !== null/);
  assert.match(app, /backtestResult\.grossProfit > 0 && backtestResult\.grossLoss === 0 \? '∞' : backtestResult\.profitFactor == null \? '—'/);
});
