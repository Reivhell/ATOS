import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
const [store, bridge, ea, mt5Modules, bootstrap, ui, types] = await Promise.all([
  read('src-tauri/src/infrastructure/mt5/market_data.rs'),
  read('src-tauri/src/infrastructure/mt5/bridge.rs'),
  read('MQL5/Experts/ATOS_Bridge.mq5'),
  read('src-tauri/src/infrastructure/mt5/mod.rs'),
  read('src-tauri/src/adapters/tauri/bootstrap.rs'),
  read('src/lib/components/Mt5BridgePanel.svelte'),
  read('src/lib/types.ts'),
]);

test('market data is stored separately from account and research databases', () => {
  assert.match(bridge, /mt5-market-data\.sqlite3/);
  assert.match(mt5Modules, /pub mod market_data;/);
  assert.match(bootstrap, /Mt5Bridge::start\(app_data_dir\.clone\(\)\)/);
  assert.doesNotMatch(store, /agentic-trading-os\.sqlite3|research-workspace\.sqlite3/);
});

test('frame receipts, market rows, and revisions share one durable transaction', () => {
  assert.match(store, /TransactionBehavior::Immediate/);
  assert.match(store, /PRAGMA journal_mode=WAL/);
  assert.match(store, /PRAGMA synchronous=FULL/);
  assert.match(store, /mt5_frame_receipts/);
  assert.match(store, /mt5_bar_revisions/);
  assert.match(store, /tx\.commit\(\)/);
  assert.match(bridge, /last_committed_sequence/);
  assert.match(bridge, /lastCommittedSequence/);
  assert.match(bridge, /tickCursors/);
  assert.match(bridge, /barCursors/);
  assert.match(bridge, /valid_identifier\(&auth\.client_id, 192\)/);
  assert.match(store, /valid_identity\(client_id, 192\)/);
  assert.match(store, /synchronization_cursors/);
  assert.match(store, /MARKET_DATA_SCHEMA_VERSION/);
  assert.match(store, /storage_available: true/);
  assert.match(store, /durable tick counter is negative or corrupt/);
  assert.match(store, /valid_symbol/);
  assert.match(store, /schema version .*newer than this application supports/);
});

test('replay protection rejects sequence reuse with different payloads and suppresses exact replays', () => {
  assert.match(store, /already committed with a different frame payload/);
  assert.match(store, /duplicate_frame: true/);
  assert.match(store, /tick_fingerprint/);
  assert.match(store, /INSERT OR IGNORE INTO mt5_ticks/);
});

test('tick and closed-bar validation enforce broker data integrity', () => {
  for (const invariant of ['MAX_TICKS_PER_FRAME', 'MAX_BARS_PER_FRAME', 'MAX_SYMBOLS_PER_FRAME', 'f64::is_finite', 'tick.time_ms <= 0', 'tick.time_ms < previous', '!bar.closed', 'valid_timeframe']) {
    assert.ok(store.includes(invariant), `missing data invariant: ${invariant}`);
  }
});

test('EA streams multi-symbol ticks and completed bars, then advances cursors only on durable ACK', () => {
  assert.match(ea, /input string InpWatchSymbols/);
  assert.match(ea, /input string InpTimeframes/);
  assert.match(ea, /CopyTicks/);
  assert.match(ea, /CopyRates/);
  assert.match(ea, /candidate_tick_times/);
  assert.match(ea, /candidate_bar_times/);
  assert.match(ea, /g_last_tick_time,candidate_tick_times/);
  assert.match(ea, /g_last_bar_time,candidate_bar_times/);
  assert.match(ea, /lastCommittedSequence/);
  assert.match(ea, /ReadMapCursor/);
  assert.match(ea, /IsSafeSymbolName/);
  assert.match(ea, /iBarShift/);
  assert.match(ea, /cursor_probe/);
  assert.match(ea, /latest_closed_probe/);
  assert.match(ea, /ATOS_BAR_AUDIT_INTERVAL_SECONDS/);
  assert.match(ea, /audit_recent_bars/);
  assert.match(ea, /durable_bar_cursor/);
  assert.match(ea, /durable bar cursor is unavailable/);
  assert.match(ea, /cursor_boundary_found/);
  assert.match(ea, /potentially uncommitted same-millisecond ticks/);
});

test('MT5 UI reports durable ingestion totals without fabricating broker observations', () => {
  assert.match(types, /interface Mt5MarketDataStatus/);
  assert.match(types, /marketDataStorage: Mt5MarketDataStatus \| null/);
  assert.match(ui, /Durable market data/);
  assert.match(ui, /marketStorage\.ticksTotal/);
  assert.match(ui, /closed bars/);
});

test('read-only EA still contains no trade or position mutation APIs', () => {
  assert.doesNotMatch(ea, /\b(OrderSend|OrderSendAsync|OrderModify|OrderDelete|OrderClose|PositionClose|PositionModify|PositionOpen)\s*\(/);
});
