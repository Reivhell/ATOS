import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const source = readFileSync(new URL('../src-tauri/src/domain/research_smc.rs', import.meta.url), 'utf8');
const catalog = JSON.parse(readFileSync(new URL('./fixtures/technical-analysis-master-list.v1.json', import.meta.url), 'utf8'));

test('point-in-time structure state machine exists and exposes auditable snapshots', () => {
  assert.match(source, /pub struct StructureStateSnapshot/);
  assert.match(source, /pub fn build_structure_state_timeline/);
  for (const field of ['at_index', 'at_ms', 'bias', 'transition', 'active_swing_high', 'active_swing_low', 'protected_high', 'protected_low', 'ambiguous']) {
    assert.match(source, new RegExp(`pub ${field}:`));
  }
  assert.match(source, /swings_by_confirmation\[index\]/);
  assert.match(source, /has_bullish && has_bearish/);
  assert.match(source, /transition = "ambiguous_break"/);
  assert.match(source, /bias is frozen/);
  assert.match(source, /not a trade signal/);
});

test('market-structure concepts are not promoted merely because the state machine exists', () => {
  for (const name of ['BOS (Break of Structure)', 'CHoCH (Change of Character)', 'MSS (Market Structure Shift)', 'MSB (Market Structure Break)', 'Protected High', 'Protected Low']) {
    assert.equal(catalog.entries.find((entry) => entry.name === name)?.availability, 'Catalog only', `${name} must remain blocked until reviewed integration`);
  }
});
