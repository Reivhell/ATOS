import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = (path) => readFileSync(new URL(path, import.meta.url), 'utf8');
const rust = read('../src-tauri/src/infrastructure/mt5/market_data.rs');
const bridge = read('../src-tauri/src/infrastructure/mt5/bridge.rs');
const commands = read('../src-tauri/src/adapters/tauri/commands.rs');
const bootstrap = read('../src-tauri/src/adapters/tauri/bootstrap.rs');
const ea = read('../MQL5/Experts/ATOS_Bridge.mq5');
const types = read('../src/lib/types.ts');
const frontendAdapter = read('../src/lib/adapters/tauri/bridge.ts');
const panel = read('../src/lib/components/Mt5BridgePanel.svelte');

test('MT5 instrument registry persists immutable revisions and stable fingerprints', () => {
  assert.match(rust, /CREATE TABLE IF NOT EXISTS mt5_instrument_specs/);
  assert.match(rust, /CREATE TABLE IF NOT EXISTS mt5_instrument_spec_revisions/);
  assert.match(rust, /instrument_specification_fingerprint/);
  assert.match(rust, /previous_sha256/);
  assert.match(rust, /trg_mt5_instrument_spec_revisions_no_update/);
  assert.match(rust, /trg_mt5_instrument_spec_revisions_no_delete/);
  assert.match(rust, /verify_instrument_specification_history/);
  assert.match(rust, /revision=revision\+1/);
  assert.match(rust, /completeness: "broker_snapshot_v1_partial"/);
});

test('MT5 protocol v6 carries broker contract, execution and currency metadata', () => {
  assert.match(bridge, /PROTOCOL_VERSION: u16 = 6/);
  for (const field of ['contract_size', 'calculation_mode', 'execution_mode', 'filling_mode', 'order_mode', 'tick_value_profit', 'tick_value_loss', 'volume_limit', 'stops_level_points', 'freeze_level_points', 'currency_base', 'currency_profit', 'currency_margin', 'swap_mode', 'swap_long', 'swap_short']) {
    assert.match(bridge, new RegExp(`pub ${field}:`));
  }
  for (const field of ['contractSize', 'calculationMode', 'executionMode', 'fillingMode', 'orderMode', 'tickValueProfit', 'tickValueLoss', 'volumeLimit', 'stopsLevelPoints', 'freezeLevelPoints', 'currencyBase', 'currencyProfit', 'currencyMargin', 'swapMode', 'swapLong', 'swapShort']) {
    assert.ok(ea.includes(`\\"${field}\\"`), `EA must emit ${field}`);
    assert.match(types, new RegExp(`${field}:`));
  }
  assert.match(ea, /#define ATOS_PROTOCOL_VERSION 6/);
});

test('instrument registry is exposed through application and Tauri command layers', () => {
  assert.match(commands, /pub fn get_mt5_instrument_specification/);
  assert.match(bootstrap, /commands::get_mt5_instrument_specification/);
  assert.match(bridge, /pub fn instrument_specification\(/);
});

test('frontend exposes authenticated instrument lookup and labels partial completeness', () => {
  assert.match(frontendAdapter, /getMt5InstrumentSpecification/);
  assert.match(frontendAdapter, /get_mt5_instrument_specification/);
  assert.match(panel, /Inspect broker contract/);
  assert.match(panel, /partial broker snapshot/);
  assert.match(panel, /not a complete margin model/);
});


test('MQL5 snapshot StringFormat placeholder count matches supplied arguments', () => {
  const start = ea.indexOf('   string format=');
  const end = ea.indexOf('   json=StringFormat(format,', start);
  assert.ok(start >= 0 && end > start, 'EA snapshot StringFormat block must exist');
  const literals = [...ea.slice(start, end).matchAll(/"((?:\\.|[^"\\])*)"/g)].map((match) => match[1]);
  const format = literals.join('');
  const placeholders = [...format.matchAll(/%(?:I64d|s|d|u|f)/g)].length;
  const argsStart = end + '   json=StringFormat(format,'.length;
  let depth = 1;
  let inString = false;
  let escaped = false;
  let commas = 0;
  for (let index = argsStart; index < ea.length; index += 1) {
    const char = ea[index];
    if (inString) {
      if (escaped) escaped = false;
      else if (char === '\\') escaped = true;
      else if (char === '"') inString = false;
      continue;
    }
    if (char === '"') inString = true;
    else if (char === '(') depth += 1;
    else if (char === ')') {
      depth -= 1;
      if (depth === 0) break;
    } else if (char === ',' && depth === 1) commas += 1;
  }
  assert.equal(placeholders, commas + 1, 'every StringFormat placeholder must have exactly one argument');
});
