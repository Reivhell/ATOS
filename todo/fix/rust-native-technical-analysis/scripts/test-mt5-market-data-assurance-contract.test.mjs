import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
const [store, bridge, commands, adapter, types, ui, nativeGuard] = await Promise.all([
  read('src-tauri/src/infrastructure/mt5/market_data.rs'),
  read('src-tauri/src/infrastructure/mt5/bridge.rs'),
  read('src-tauri/src/adapters/tauri/commands.rs'),
  read('src/lib/adapters/tauri/bridge.ts'),
  read('src/lib/types.ts'),
  read('src/lib/components/Mt5BridgePanel.svelte'),
  read('src/lib/adapters/tauri/native-runtime-guard.js'),
]);

test('series query is bounded, identity-scoped, ordered, and cursor paginated', () => {
  assert.match(store, /pub struct Mt5SeriesQuery/);
  assert.match(store, /request\.to_ms\.saturating_sub\(request\.from_ms\) > 366_i64/);
  assert.match(store, /MAX_SAFE_JSON_INTEGER/);
  assert.match(store, /from_ms > MAX_SAFE_JSON_INTEGER/);
  assert.match(store, /\(1\.\.=5_000\)\.contains\(&request\.limit\)/);
  assert.match(store, /ORDER BY time_ms ASC LIMIT \?8/);
  assert.match(store, /next_after_ms/);
  assert.match(store, /ATOS-MT5-SERIES-MANIFEST-V1/);
  assert.match(store, /TransactionBehavior::Deferred/);
  assert.match(store, /dataset_fingerprint_sha256/);
  assert.match(bridge, /market_data_series_query/);
  assert.match(bridge, /does not match the last authenticated terminal account/);
  assert.match(commands, /query_mt5_market_data_series/);
  assert.match(adapter, /query_mt5_market_data_series/);
  assert.match(adapter, /Number\.isSafeInteger/);
});

test('series assurance includes full-window revisions, interval analysis, and explicit session caveat', () => {
  assert.match(store, /mt5_bar_revisions/);
  assert.match(store, /LAG\(time_ms\) OVER \(ORDER BY time_ms\)/);
  assert.match(store, /unclassified candidates/);
  assert.match(store, /gap candidates remain unclassified/);
  assert.match(store, /dated holiday\/exception calendars/);
  assert.match(store, /payload fingerprint is corrupt/);
  assert.match(store, /refusing to export corrupted data/);
});

test('typed UI exposes authenticated broker series inspection without guessed identity', () => {
  assert.match(types, /interface Mt5SeriesQuery/);
  assert.match(types, /interface Mt5SeriesQueryResponse/);
  assert.match(ui, /Stored market-data assurance/);
  assert.match(ui, /Inspect stored series/);
  assert.match(ui, /Inspect broker sessions/);
  assert.match(ui, /fingerprintSha256/);
  assert.match(ui, /Load next page/);
  assert.match(ui, /No broker identity is guessed/);
  assert.match(ui, /seriesActiveQuery/);
  assert.match(ui, /query-result-identity/);
  assert.match(ui, /datasetFingerprintSha256 !== seriesResult\.datasetFingerprintSha256/);
  assert.match(ui, /stored MT5 dataset changed while pages were loading/);
  assert.match(ui, /terminal\.serverTimeMs/);
  assert.match(nativeGuard, /MT5 market-data series/);
  assert.match(ui, /Payload SHA-256/);
  assert.match(ui, /Dataset manifest SHA-256/);
});
