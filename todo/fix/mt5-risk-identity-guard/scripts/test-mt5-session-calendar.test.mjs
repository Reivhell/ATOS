import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
const [store, bridge, ea, commands, bootstrap, facade, adapter, types] = await Promise.all([
  read('src-tauri/src/infrastructure/mt5/market_data.rs'),
  read('src-tauri/src/infrastructure/mt5/bridge.rs'),
  read('MQL5/Experts/ATOS_Bridge.mq5'),
  read('src-tauri/src/adapters/tauri/commands.rs'),
  read('src-tauri/src/adapters/tauri/bootstrap.rs'),
  read('src-tauri/src/application/market_data_query.rs'),
  read('src/lib/adapters/tauri/bridge.ts'),
  read('src/lib/types.ts'),
]);

test('protocol v6 transports complete per-symbol weekly broker schedules', () => {
  assert.match(bridge, /PROTOCOL_VERSION: u16 = 6/);
  assert.match(ea, /#define ATOS_PROTOCOL_VERSION 6/);
  assert.match(ea, /bool BuildSessionSchedules\(string &json,bool &include_schedule\)/);
  assert.match(ea, /SymbolInfoSessionTrade\(watched/);
  assert.match(ea, /for\(int day=0;day<7;day\+\+\)/);
  assert.match(ea, /sessionSchedules/);
  assert.match(ea, /InpSessionCalendarRefreshSeconds = 300/);
  assert.match(ea, /g_session_schedules_cache/);
  assert.match(ea, /g_session_calendar_needs_send/);
  assert.match(ea, /include_schedule=false/);
  assert.match(store, /verify_existing_session_calendars/);
  assert.match(store, /MAX_SESSION_CALENDAR_AGE_MS/);
  assert.match(ea, /InpSessionCalendarRefreshSeconds<30/);
  assert.match(bridge, /session_schedules: Vec<Mt5SessionSchedule>/);
  assert.match(ea, /g_session_calendar_needs_send=true/);
});

test('session calendars are bounded, validated, account scoped, and transactionally persisted', () => {
  assert.match(store, /const MARKET_DATA_SCHEMA_VERSION: i64 = 3/);
  assert.match(store, /validate_session_schedules\(session_schedules\)/);
  assert.match(store, /schedule\.days\.len\(\) != 7/);
  assert.match(store, /day\.sessions\.len\(\) > 16/);
  assert.match(store, /captured_at_ms/);
  assert.match(store, /stable_schedule\.captured_at_ms = 0/);
  assert.match(bridge, /MAX_SESSION_CALENDAR_AGE_MS/);
  assert.match(store, /CREATE TABLE IF NOT EXISTS mt5_session_calendars/);
  assert.match(store, /CREATE TABLE IF NOT EXISTS mt5_session_calendar_revisions/);
  assert.match(store, /schema_v2_upgrade_preserves_instrument_projection_and_adds_session_history/);
  assert.match(store, /trg_mt5_session_calendar_revisions_no_update/);
  assert.match(store, /trg_mt5_session_calendar_revisions_no_delete/);
  assert.match(bridge, /schedule_scope != symbol_scope/);
});

test('dataset provenance binds the broker calendar without inventing UTC, DST, or holiday certainty', () => {
  assert.match(store, /ATOS-MT5-SESSION-CALENDAR-CONTEXT-V1/);
  assert.match(store, /SESSION-CALENDAR-UNKNOWN/);
  assert.match(store, /broker_server_wall_clock; no UTC offset or DST inference/);
  assert.match(store, /gap candidates remain unclassified/);
  assert.match(store, /verify_session_calendar_history/);
});

test('calendar query crosses the application and Tauri boundary with authenticated identity checks', () => {
  assert.match(store, /pub fn session_calendar\(/);
  assert.match(bridge, /pub fn session_calendar\(/);
  assert.match(facade, /pub fn session_calendar\(/);
  assert.match(commands, /pub fn get_mt5_session_calendar\(/);
  assert.match(bootstrap, /commands::get_mt5_session_calendar/);
  assert.match(adapter, /getMt5SessionCalendar/);
  assert.match(adapter, /get_mt5_session_calendar/);
  assert.match(types, /export interface Mt5SessionCalendarRecord/);
});

test('MQL5 session serializer retains exact StringFormat placeholder parity', () => {
  assert.match(ea, /StringFormat\("\{\\"symbol\\":\\"%s\\",\\"capturedAtMs\\":%I64d/);
  assert.match(ea, /JsonEscape\(watched\),\(long\)TimeGMT\(\)\*1000/);
  assert.match(ea, /StringFormat\("\{\\"dayOfWeek\\":%d,\\"sessions\\":\[",day\)/);
  assert.match(ea, /StringFormat\("\{\\"index\\":%d,\\"fromSeconds\\":%I64u,\\"toSeconds\\":%I64u\}",/);
});
