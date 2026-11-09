import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
const [engine, commands, bootstrap, adapter, ui, types, guard] = await Promise.all([
  read('src-tauri/src/application/mt5_risk_sizing.rs'),
  read('src-tauri/src/adapters/tauri/commands.rs'),
  read('src-tauri/src/adapters/tauri/bootstrap.rs'),
  read('src/lib/adapters/tauri/bridge.ts'),
  read('src/lib/components/Mt5BridgePanel.svelte'),
  read('src/lib/types.ts'),
  read('src/lib/adapters/tauri/native-runtime-guard.js'),
]);

test('MT5 risk sizing is based on authenticated fresh terminal state and broker contract', () => {
  assert.match(engine, /status\.connected/);
  assert.match(engine, /MAX_QUOTE_AGE_MS/);
  assert.match(engine, /MAX_HEARTBEAT_AGE_MS/);
  assert.match(engine, /bridge\.instrument_specification/);
  assert.match(engine, /Live quote snapshot and durable broker contract projection disagree/);
  assert.match(engine, /tick_value_loss/);
  assert.match(engine, /MAX_SPECIFICATION_AGE_MS/);
  assert.match(engine, /specification\.last_observed_at_ms/);
  assert.match(engine, /snapshot\.account\.equity/);
  assert.match(engine, /stops_level_points/);
  assert.match(engine, /specification_fingerprint_sha256/);
  assert.match(engine, /correlated_preflight_inventory/);
  assert.match(engine, /snapshot_contract_matches/);
  assert.match(engine, /snapshot_sequence: calculation_snapshot_sequence/);
});

test('risk sizing rounds volume down and blocks minimum-volume risk overruns', () => {
  assert.match(engine, /normalize_volume_down/);
  assert.match(engine, /normalized_volume < volume_min/);
  assert.match(engine, /minimum lot would exceed the available risk capacity/);
  assert.match(engine, /volume_limit_aggregation_is_symbol_and_direction_specific/);
  assert.match(engine, /expected_position_type/);
  assert.match(engine, /position\.position_type == position_type/);
  assert.match(engine, /volume_limit/);
  assert.match(engine, /volume_step/);
});

test('unknown margin and commission are explicitly not estimated', () => {
  assert.match(engine, /NOT_COMPUTED/);
  assert.match(engine, /excludes commission, swap, slippage and gap risk/);
  assert.doesNotMatch(engine, /OrderSend|OrderModify|OrderClose/);
});

test('sizing request is wired through Rust, Tauri, adapter and UI', () => {
  assert.match(commands, /pub fn calculate_mt5_risk_sizing/);
  assert.match(bootstrap, /commands::calculate_mt5_risk_sizing/);
  assert.match(adapter, /calculateMt5RiskSizing/);
  assert.match(adapter, /calculate_mt5_risk_sizing/);
  assert.match(ui, /calculateRiskSizing/);
  assert.match(ui, /Broker-native position risk sizing/);
  assert.match(types, /export interface Mt5RiskSizingResult/);
  assert.match(guard, /MT5 risk sizing/);
});

test('Rust unit tests cover rounding, caps and invalid numerical inputs', () => {
  assert.match(engine, /live_snapshot_and_durable_contract_comparison_is_tolerant_but_strict/);
  assert.match(engine, /loss_per_lot_uses_broker_tick_loss_value/);
  assert.match(engine, /volume_is_rounded_down_not_nearest/);
  assert.match(engine, /volume_is_capped_at_broker_maximum/);
  assert.match(engine, /approved_volume = if reasons\.is_empty\(\)/);
  assert.match(engine, /invalid_inputs_never_produce_volume/);
});

test('volume normalization never returns a broker volume above the maximum or off the step grid', async () => {
  const source = await readFile(new URL('../src-tauri/src/application/mt5_risk_sizing.rs', import.meta.url), 'utf8');
  assert.match(source, /let maximum_steps = \(volume_max \/ volume_step\)\.floor\(\)/);
  assert.match(source, /volume <= raw_volume && volume <= volume_max/);
  assert.match(source, /volume_cap_is_rounded_down_to_a_broker_valid_step/);
});


test('read-only risk sizing uses broker account permission without requiring local Algo Trading enablement', async () => {
  assert.equal((engine.match(/snapshot\.account\.account_trade_allowed[\s\S]*?snapshot\.symbol\.trade_mode[\s\S]*?snapshot\.symbol\.order_mode[\s\S]*?&side/g) ?? []).length, 2,
    'initial and correlated-snapshot gates must both recheck broker account-level permission');
  assert.match(engine, /fn market_trade_permission_block_reason\(\s*broker_account_trade_allowed: bool/);
  assert.match(engine, /Broker account-level trading permission is disabled/);
  assert.match(engine, /actionable_preflight_obeys_broker_account_symbol_and_order_permissions/);
  assert.doesNotMatch(engine, /snapshot\.account\.trade_allowed, snapshot\.symbol\.trade_mode/,
    'terminal and EA Algo Trading toggles remain telemetry-only for read-only calculations');
});

test('final sizing output is bound to the unchanged account frame and completion-time freshness', () => {
  assert.match(engine, /let completed_at_ms = chrono::Utc::now\(\)\.timestamp_millis\(\)/);
  assert.match(engine, /final_status\.last_sequence != calculation_snapshot_sequence/);
  assert.match(engine, /final_status\.quote_stale/);
  assert.match(engine, /current\.symbol\.quote_time_ms == snapshot\.symbol\.quote_time_ms/);
  assert.match(engine, /quote_age = completed_at_ms\.saturating_sub\(snapshot\.symbol\.quote_time_ms\)/);
  assert.match(engine, /calculated_at_ms: completed_at_ms/);
  assert.match(engine, /quote_age_is_evaluated_against_the_current_completion_time/);
});
