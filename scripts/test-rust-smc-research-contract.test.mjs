import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const moduleSource = readFileSync(new URL('../src-tauri/src/domain/research_smc.rs', import.meta.url), 'utf8');
const rustLib = readFileSync(new URL('../src-tauri/src/domain/mod.rs', import.meta.url), 'utf8');
const catalog = JSON.parse(readFileSync(new URL('./fixtures/technical-analysis-master-list.v1.json', import.meta.url), 'utf8'));

test('Rust research SMC module is compiled into the native crate but remains research-only', () => {
  assert.match(rustLib, /pub mod research_smc;/);
  assert.match(moduleSource, /research-only SMC primitives/);
  assert.match(moduleSource, /do not promote catalog entries or authorize a decision/);
  assert.equal(catalog.entries.find((entry) => entry.name === 'FVG (Fair Value Gap)')?.availability, 'Catalog only');
  assert.equal(catalog.entries.find((entry) => entry.name === 'Liquidity Sweep')?.availability, 'Catalog only');
});

test('FVG detector uses three-candle geometry and reports formation on the third candle', () => {
  assert.match(moduleSource, /pub fn detect_fair_value_gaps/);
  assert.match(moduleSource, /for index in 2\.\.candles\.len\(\)/);
  assert.match(moduleSource, /right\.low > left\.high/);
  assert.match(moduleSource, /right\.high < left\.low/);
  assert.match(moduleSource, /formed_at_index: index/);
  assert.match(moduleSource, /does not infer later fill state/);
});

test('swing detection uses a linear-time monotonic-deque scan and bounded output', () => {
  assert.match(moduleSource, /VecDeque/);
  assert.match(moduleSource, /push_max_index/);
  assert.match(moduleSource, /push_min_index/);
  assert.match(moduleSource, /Detect unique, confirmed fractal swing points in O\(n\) time/);
  assert.match(moduleSource, /fn push_max_index/);
  assert.match(moduleSource, /fn push_min_index/);
  assert.match(moduleSource, /linear_time_swing_detector_matches_naive_reference_on_generated_candles/);
  assert.match(moduleSource, /research_detector_outputs_are_prefix_invariant_and_never_rewrite_past_events/);
  const naiveHelper = moduleSource.match(/fn naive_confirmed_swings\([\s\S]*?\n    }/);
  assert.ok(naiveHelper, 'naive swing reference helper must exist');
  assert.doesNotMatch(naiveHelper[0], /return Err\(/, 'Vec-returning reference helper must not return Result errors');
  assert.match(moduleSource, /MAX_RESEARCH_EVENTS: usize = 1_000_000/);
  assert.match(moduleSource, /no partial result returned/);
});

test('liquidity sweeps and structure breaks use ordered price indexes instead of all-pivot rescans', () => {
  assert.match(moduleSource, /pub fn detect_confirmed_liquidity_sweeps/);
  assert.match(moduleSource, /BTreeMap::<u64, Vec<usize>>/);
  assert.match(moduleSource, /active_buy_side\s*\.\s*range/);
  assert.match(moduleSource, /active_sell_side[\s\S]*?\.range/);
  assert.match(moduleSource, /active_highs[\s\S]*?\.range/);
  assert.match(moduleSource, /active_lows[\s\S]*?\.range/);
  assert.match(moduleSource, /ordered_price_key/);
  assert.match(moduleSource, /Liquidity sweep output exceeds the research event safety limit/);
  assert.match(moduleSource, /Structure-break output exceeds the research event safety limit/);
  assert.match(moduleSource, /Repeated valid sweeps keep/);
});

test('liquidity sweep detector is causal and validates wick-through/close-back-inside price intervals', () => {
  assert.match(moduleSource, /pub fn detect_confirmed_liquidity_sweeps/);
  assert.match(moduleSource, /swings\[next_swing\]\.confirmed_at_index < index/);
  assert.match(moduleSource, /active_buy_side[\s\S]*?\.range/);
  assert.match(moduleSource, /active_sell_side[\s\S]*?\.range/);
  assert.match(moduleSource, /ordered_price_key\(candle\.close\)/);
  assert.match(moduleSource, /ordered_price_key\(candle\.high\)/);
  assert.match(moduleSource, /ordered_price_key\(candle\.low\)/);
  assert.match(moduleSource, /indexed_sweep_detector_preserves_confirmation_and_deterministic_event_order/);
});

test('research detectors validate finite OHLCV, candle ordering, and bounded input size', () => {
  assert.match(moduleSource, /is_finite\(\)/);
  assert.match(moduleSource, /candle\.volume < 0\.0/);
  assert.match(moduleSource, /candle\.time <= candles\[index - 1\]\.time/);
  assert.match(moduleSource, /2_000_000/);
  assert.match(moduleSource, /malformed_or_non_monotonic_candles_fail_closed/);
});

test('FVG lifecycle emits chronological partial/full-fill and inversion-candidate observations without promotion', () => {
  assert.match(moduleSource, /pub fn assess_fvg_lifecycle/);
  assert.match(moduleSource, /event_type: "partial_fill"/);
  assert.match(moduleSource, /event_type: "full_fill"/);
  assert.match(moduleSource, /event_type: "inversion_candidate"/);
  assert.match(moduleSource, /candles\.iter\(\)\.enumerate\(\)\.skip\(gap\.formed_at_index \+ 1\)/);
  assert.match(moduleSource, /FVG input contains an invalid zone or formation reference/);
  assert.match(moduleSource, /fvg_lifecycle_emits_partial_then_full_fill_once_and_never_on_formation_bar/);
  assert.match(moduleSource, /fvg_close_beyond_far_boundary_is_only_an_inversion_candidate/);
  assert.match(moduleSource, /fvg_lifecycle_rejects_forged_or_misaligned_gap_references/);
  assert.match(moduleSource, /fvg_lifecycle_rejects_fabricated_zone_geometry_even_with_valid_timestamp/);
  assert.match(moduleSource, /FVG zone geometry does not match the source candles/);
  assert.match(moduleSource, /FVG width is not representable at candle index/);
  assert.match(moduleSource, /FVG reference geometry is not representable in finite precision/);
  assert.match(moduleSource, /fvg_detector_rejects_finite_prices_whose_derived_width_overflows/);
  assert.equal(catalog.entries.find((entry) => entry.name === 'FVG (Fair Value Gap)')?.availability, 'Catalog only');
});

test('confirmed swing detector uses strict unique pivots and records confirmation time without promoting catalog entries', () => {
  assert.match(moduleSource, /pub struct ConfirmedSwing/);
  assert.match(moduleSource, /pub fn detect_confirmed_swings/);
  assert.match(moduleSource, /candle\.high > other\.high/);
  assert.match(moduleSource, /candle\.low < other\.low/);
  assert.match(moduleSource, /confirmed_at_index: confirmed_at/);
  assert.match(moduleSource, /Swing confirmation windows must both be greater than zero/);
  assert.match(moduleSource, /Swing confirmation window exceeds the safety limit/);
  assert.match(moduleSource, /swing_points_are_emitted_only_after_right_side_confirmation/);
  assert.match(moduleSource, /swing_detector_rejects_tied_pivots_and_invalid_windows/);
  assert.match(moduleSource, /swing_detector_fails_closed_on_invalid_candle_data/);
  assert.equal(catalog.entries.find((entry) => entry.name === 'Swing High')?.availability, 'Catalog only');
  assert.equal(catalog.entries.find((entry) => entry.name === 'Swing Low')?.availability, 'Catalog only');
});


test('HH/HL/LH/LL classification compares each swing to the prior same-kind confirmed pivot', () => {
  assert.match(moduleSource, /pub struct ClassifiedSwing/);
  assert.match(moduleSource, /pub fn classify_confirmed_swings/);
  assert.match(moduleSource, /=> "higher_high"/);
  assert.match(moduleSource, /=> "lower_high"/);
  assert.match(moduleSource, /=> "higher_low"/);
  assert.match(moduleSource, /=> "lower_low"/);
  assert.match(moduleSource, /=> "equal_high"/);
  assert.match(moduleSource, /=> "equal_low"/);
  assert.match(moduleSource, /compared_to_pivot_index: previous\.map/);
  assert.match(moduleSource, /swing_classification_labels_higher_and_lower_swings_with_comparison_provenance/);
  assert.match(moduleSource, /equal_price_swings_are_not_mislabeled_as_higher_or_lower/);
  assert.match(moduleSource, /not a trend\/bias\/BOS signal/);
  for (const name of ['Swing High', 'Swing Low']) {
    assert.equal(catalog.entries.find((entry) => entry.name === name)?.availability, 'Catalog only');
  }
});
