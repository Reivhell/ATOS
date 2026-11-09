import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src-tauri/src/domain/research_smc.rs', import.meta.url), 'utf8');
const catalog = JSON.parse(readFileSync(new URL('./fixtures/technical-analysis-master-list.v1.json', import.meta.url), 'utf8'));

test('market structure break detector is causal, close-confirmed and research-only', () => {
  assert.match(source, /pub fn detect_confirmed_structure_breaks/);
  assert.match(source, /swings\[next_swing\]\.confirmed_at_index < index/);
  assert.match(source, /active_highs\s*\.\s*range/);
  assert.match(source, /active_lows\s*\.\s*range/);
  assert.match(source, /Included\(ordered_price_key\(previous_close\)\)/);
  assert.match(source, /Excluded\(ordered_price_key\(candle\.close\)\)/);
  assert.match(source, /reversal_candidate/);
  assert.match(source, /BOS\/CHoCH\/MSS\/MSB require additional strategy/);
  assert.match(source, /structure_break_requires_close_cross_after_level_confirmation/);
  assert.match(source, /wick_only_breach_is_not_a_structure_break/);
  assert.equal(catalog.entries.find((entry) => entry.name === 'BOS (Break of Structure)')?.availability, 'Catalog only');
  assert.equal(catalog.entries.find((entry) => entry.name === 'CHoCH (Change of Character)')?.availability, 'Catalog only');
});


test('same-candle multi-level breaks are resolved as a batch from one prior-bias snapshot', () => {
  assert.match(source, /Keep deterministic batching: all events on this candle use the same pre-candle/);
  assert.match(source, /let has_bullish = eligible\.iter\(\)\.any\(\|&s\| swings\[s\]\.kind == "swing_high"\)/);
  assert.match(source, /let has_bearish = eligible\.iter\(\)\.any\(\|&s\| swings\[s\]\.kind == "swing_low"\)/);
  assert.match(source, /if !ambiguous \{[\s\S]*?if has_bullish \{[\s\S]*?bias = 1;/);
  assert.match(source, /multi_level_breaks_on_one_candle_use_one_pre_candle_bias_snapshot/);
});


test('outside-bar swing kinds retain independent lifecycle identity', () => {
  assert.match(source, /active_highs = BTreeMap::<u64, Vec<usize>>::new\(\)/);
  assert.match(source, /active_lows = BTreeMap::<u64, Vec<usize>>::new\(\)/);
  assert.match(source, /if swing\.kind == "swing_high"\s*\{\s*&mut active_highs\s*\}\s*else\s*\{\s*&mut active_lows\s*\}/);
  assert.match(source, /for key in bullish_keys\s*\{\s*active_highs\.remove\(&key\)\s*;\s*\}/);
  assert.match(source, /for key in bearish_keys\s*\{\s*active_lows\.remove\(&key\)\s*;\s*\}/);
  assert.match(source, /outside_bar_swing_high_and_low_have_independent_break_lifecycles/);
});

test('structure timeline indexes swing confirmations and breaks once', () => {
  assert.match(source, /swings_by_confirmation/);
  assert.match(source, /breaks_by_candle/);
  assert.match(source, /former implementation rescanned every swing and every break/);
});
