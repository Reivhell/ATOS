import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const rust = readFileSync(new URL('../src-tauri/src/application/backtest.rs', import.meta.url), 'utf8');
const models = readFileSync(new URL('../src-tauri/src/domain/models.rs', import.meta.url), 'utf8');
const types = readFileSync(new URL('../src/lib/types.ts', import.meta.url), 'utf8');
const app = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const packageJson = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
const cargo = readFileSync(new URL('../src-tauri/Cargo.toml', import.meta.url), 'utf8');
const tauri = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));

test('backtest always returns two deterministic cost-shock scenarios with applied cost metadata', () => {
  assert.match(rust, /\("Moderate cost shock", 1\.5_f64, 2\.0_f64, 1\.5_f64, 5\.0_f64\)/);
  assert.match(rust, /\("Severe cost shock", 2\.0_f64, 5\.0_f64, 2\.0_f64, 10\.0_f64\)/);
  assert.match(rust, /fee_cap_reached: raw_fee > 500\.0/);
  assert.match(rust, /slippage_cap_reached: raw_slippage > 2_000\.0/);
  assert.match(rust, /return_delta_percentage_points:\s*stressed\.total_return_pct\s*-\s*baseline\.total_return_pct/);
  assert.match(models, /pub cost_stress_scenarios: Vec<BacktestCostStressScenario>/);
  assert.match(types, /costStressScenarios: BacktestCostStressScenario\[\]/);
  assert.match(app, /Execution-cost sensitivity/);
});

test('out-of-sample windows are fixed-size, chronological, use explicit indicator warm-up, and cap report size', () => {
  assert.match(rust, /let warmup_bars\s*=\s*request[\s\S]*?\.slow_period[\s\S]*?\.checked_add\s*\(\s*2\s*\)/);
  assert.match(rust, /let minimum_test_bars = request\.slow_period\.saturating_mul\(2\)\.max\(30\)/);
  assert.match(rust, /let fold_count = complete_window_count\.min\(12\)/);
  assert.match(rust, /run_once\(symbol, window, request\.clone\(\), source, warmup_bars\)/);
  assert.match(rust, /for i in evaluation_start_index\.\.candles\.len\(\)/);
  assert.match(models, /pub out_of_sample_folds: Vec<BacktestOutOfSampleFold>/);
  assert.match(types, /outOfSampleFolds: BacktestOutOfSampleFold\[\]/);
  assert.match(app, /Out-of-sample stability/);
});

test('native regression coverage addresses cost stress, short history and future-value invariance', () => {
  assert.match(rust, /fn backtest_reports_cost_stress_scenarios_with_applied_costs_and_deltas\(\)/);
  assert.match(rust, /fn backtest_builds_fixed_chronological_out_of_sample_windows\(\)/);
  assert.match(rust, /fn earlier_out_of_sample_fold_is_invariant_to_future_candle_values\(\)/);
  assert.match(rust, /fn out_of_sample_validation_fails_closed_when_history_is_too_short\(\)/);
});

test('application manifests remain version-aligned at 0.11.0', () => {
  assert.equal(packageJson.version, '0.11.0');
  assert.equal(tauri.version, '0.11.0');
  assert.match(cargo, /^version = "0\.11\.0"/m);
});
