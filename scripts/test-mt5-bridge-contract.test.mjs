import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');

const [native, ea, ui, types, adapter, app, docs, cargo, mt5Modules, bootstrap] = await Promise.all([
  read('src-tauri/src/infrastructure/mt5/bridge.rs'),
  read('MQL5/Experts/ATOS_Bridge.mq5'),
  read('src/lib/components/Mt5BridgePanel.svelte'),
  read('src/lib/types.ts'),
  read('src/lib/adapters/tauri/bridge.ts'),
  read('src/App.svelte'),
  read('docs/MT5_BRIDGE_SECURITY_AND_OPERATIONS.md'),
  read('src-tauri/Cargo.toml'),
  read('src-tauri/src/infrastructure/mt5/mod.rs'),
  read('src-tauri/src/adapters/tauri/bootstrap.rs'),
]);

test('native bridge is wired as a managed Tauri service with explicit commands', () => {
  assert.match(native, /pub fn start\(app_data_dir: PathBuf\)/);
  assert.match(app, /Mt5BridgePanel/);
  assert.match(adapter, /get_mt5_bridge_status/);
  assert.match(adapter, /get_mt5_bridge_setup/);
  assert.match(mt5Modules, /pub mod bridge;/);
  assert.match(bootstrap, /commands::get_mt5_bridge_status/);
});

test('listener is IPv4 loopback-only and allows a single active session', () => {
  assert.match(native, /Ipv4Addr::LOCALHOST/);
  assert.match(native, /active_connection\s*\.\s*compare_exchange\s*\(\s*false\s*,\s*true/);
  assert.match(ea, /InpBridgeHost!="127\.0\.0\.1"/);
  assert.doesNotMatch(ea, /InpBridgeHost!="localhost"/);
});

test('pairing token is OS-vault-backed and fails closed without plaintext fallback', () => {
  assert.match(native, /Entry::new\(SERVICE_NAME, CREDENTIAL_NAME\)/);
  assert.match(native, /entry\.set_password/);
  assert.match(native, /No plaintext token fallback was used/);
  assert.match(native, /Zeroizing/);
  assert.match(cargo, /keyring/);
  assert.match(cargo, /zeroize/);
});

test('protocol enforces bounded frames, strict schemas, timestamps, nonce replay and sequence order', () => {
  assert.match(native, /MAX_FRAME_BYTES: usize = 2 \* 1024 \* 1024/);
  assert.match(native, /deny_unknown_fields/);
  assert.match(native, /validate_timestamp\(frame\.sent_at_ms\)/);
  assert.match(native, /nonce_guard/);
  assert.match(native, /frame\.sequence != expected_sequence/);
  assert.match(native, /read_bounded_line/);
});

test('snapshot validation rejects invalid numerical, quote, position and cardinality states', () => {
  assert.match(native, /!value\.is_finite\(\)/);
  assert.match(native, /symbol\.bid <= 0\.0 \|\| symbol\.ask <= 0\.0/);
  assert.match(native, /symbol\.ask < symbol\.bid/);
  assert.match(native, /MAX_POSITIONS/);
  assert.match(native, /tickets\.insert/);
  assert.match(native, /position\.volume <= 0\.0/);
  assert.match(native, /negative floating P\/L and swap are valid broker states/);
});

test('EA is telemetry-only and contains no trading API or order mutation call', () => {
  assert.match(ea, /READ-ONLY/);
  assert.match(ea, /character<32/);
  assert.match(ea, /SocketConnect/);
  assert.match(ea, /SocketSend/);
  assert.match(ea, /SocketRead/);
  assert.doesNotMatch(ea, /\b(OrderSend|OrderSendAsync|OrderModify|OrderDelete|OrderClose|PositionClose|PositionModify|PositionOpen)\s*\(/);
});

test('EA checks handshake protocol, sequential acknowledgement and refuses partial positions', () => {
  assert.match(ea, /protocolVersion/);
  assert.match(ea, /maxFrameBytes/);
  assert.match(ea, /sequence.*next_sequence/);
  assert.match(ea, /CopyTicks/);
  assert.match(ea, /CopyRates/);
  assert.match(ea, /sessionId/);
  assert.match(native, /account_login: String/);
  assert.match(native, /broker_server: String/);
  assert.match(native, /symbols: Vec<String>/);
  assert.match(native, /timeframes: Vec<String>/);
  assert.match(ea, /StringArrayJson\(g_symbols\)/);
  assert.match(ea, /TimeframesJson\(\)/);
  assert.match(ea, /tickCursors/);
  assert.match(ea, /barCursors/);
  assert.match(ea, /positions_total>ATOS_MAX_POSITIONS/);
  assert.match(ea, /withheld frame/);
});

test('UI reports current telemetry only while live heartbeat is fresh and permits explicit token reveal', () => {
  assert.match(ui, /getMt5BridgeStatus/);
  assert.match(ui, /status\?\.connected && snapshot/);
  assert.match(ui, /getMt5BridgeSetup/);
  assert.match(ui, /onclick=\{closeToken\}/);
  assert.match(ui, /STALE QUOTE/);
  assert.match(ui, /No account, quote, or position values are fabricated/);
});

test('native TypeScript contract includes account, terminal, symbol, positions and status fields', () => {
  for (const field of ['balance', 'equity', 'marginLevelPct', 'tickSize', 'volumeStep', 'positionType', 'quoteStale', 'heartbeatAgeMs']) {
    assert.ok(types.includes(field), `missing type field: ${field}`);
  }
});

test('operations guide documents local-only transport and unverified MT5 integration gates', () => {
  assert.match(docs, /not encrypted in transit/i);
  assert.match(docs, /127\.0\.0\.1/);
  assert.match(docs, /MetaEditor\/MQL5 compiler/);
  assert.match(docs, /No live order route is implemented/);
  assert.match(docs, /2 MiB/);
});


test('EA is packaged at the canonical MetaTrader data-folder path', () => {
  assert.ok(ea.includes('MT5 telemetry and market-data ingestion adapter'));
  assert.match(docs, /MQL5\/Experts\/ATOS_Bridge\.mq5/);
  assert.match(native, /expert_advisor_path: "MQL5\/Experts\/ATOS_Bridge\.mq5"/);
});

test('native snapshot validation enforces terminal, permissions and broker contract invariants', () => {
  assert.match(native, /!snapshot\.terminal\.connected/);
  assert.match(native, /effective_trade_permission/);
  assert.match(native, /account\.leverage <= 0/);
  assert.match(native, /symbol\.point <= 0\.0 \|\| symbol\.tick_size <= 0\.0/);
  assert.match(native, /symbol\.volume_min\s*<=\s*0\.0\s*\|\|\s*symbol\.volume_max\s*<\s*symbol\.volume_min/);
  assert.match(native, /validate_market_data_scope/);
  assert.match(native, /snapshot\.account\.login != authenticated_login/);
  assert.match(native, /snapshot\.account\.server != authenticated_server/);
  assert.match(native, /outside the authenticated subscription/);
  assert.match(native, /frame_scope_binds_account_and_market_data_to_authenticated_subscriptions/);
});

test('authenticated account identity and series scope are enforced before market-data commit', () => {
  assert.match(native, /validate_market_data_scope\(/);
  assert.match(native, /snapshot\.account\.login != authenticated_login/);
  assert.match(native, /snapshot\.account\.server != authenticated_server/);
  assert.match(native, /tick\.symbol\.as_str\(\)/);
  assert.match(native, /bar\.timeframe\.as_str\(\)/);
  assert.match(native, /frame_scope_binds_account_and_market_data_to_authenticated_subscriptions/);
  assert.match(native, /validate_market_data_scope\(AuthenticatedMarketDataScope \{/);
});
