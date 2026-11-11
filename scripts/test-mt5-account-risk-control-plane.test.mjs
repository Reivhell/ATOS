import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
const [bridge, risk, ea, types, panel, securityDocs, designDocs, statusDocs] = await Promise.all([
  read('src-tauri/src/infrastructure/mt5/bridge.rs'),
  read('src-tauri/src/application/mt5_risk_sizing.rs'),
  read('MQL5/Experts/ATOS_Bridge.mq5'),
  read('src/lib/types.ts'),
  read('src/lib/components/Mt5BridgePanel.svelte'),
  read('docs/archive/historical-0.11-era/MT5_BRIDGE_SECURITY_AND_OPERATIONS.md'),
  read('docs/archive/historical-0.11-era/MT5_BROKER_NATIVE_RISK_SIZING_ENGINE_2026-10-31.md'),
  read('docs/IMPLEMENTATION_STATUS.md'),
]);

test('protocol v6 carries broker margin policy and per-position/per-order stop-risk state', () => {
  assert.match(bridge, /PROTOCOL_VERSION: u16 = 6/);
  for (const field of ['margin_initial', 'margin_maintenance', 'margin_call_level', 'stop_out_level', 'margin_stopout_mode', 'margin_mode', 'account_hedge_allowed', 'account_limit_orders']) {
    assert.match(bridge, new RegExp(`pub ${field}:`));
  }
  for (const struct of ['Mt5PositionSnapshot', 'Mt5PendingOrderSnapshot']) {
    const start = bridge.indexOf(`pub struct ${struct}`);
    const end = bridge.indexOf('\n}', start);
    const block = bridge.slice(start, end);
    assert.match(block, /risk_status: String/);
    assert.match(block, /risk_at_stop_loss: Option<f64>/);
  }
  assert.match(bridge, /fn validate_stop_risk/);
  assert.match(bridge, /pub fn account_risk_inventory/);
  assert.match(bridge, /current_stop_risk_percent\.is_finite\(\)/);
});

test('account-wide stop risk includes every protected position and pending order and fails closed on incomplete exposure', () => {
  assert.match(bridge, /pub fn account_risk_inventory\(snapshot: &Mt5Snapshot\)[\s\S]*?validate_snapshot\(snapshot\)\?/);
  assert.match(bridge, /add_account_stop_risk\(&mut current_stop_risk, risk, \"position\"\)/);
  assert.match(bridge, /add_account_stop_risk\(&mut current_stop_risk, risk, \"pending order\"\)/);
  assert.match(bridge, /fn add_account_stop_risk\([\s\S]*?risk\.is_finite\(\) \|\| risk <= 0\.0[\s\S]*?aggregate\.is_finite\(\)/);
  assert.match(bridge, /account_risk_inventory_validates_snapshots_at_its_own_boundary/);
  assert.match(risk, /unprotected_position_count > 0/);
  assert.match(risk, /unresolved_position_risk_count > 0/);
  assert.match(risk, /No account-wide stop-loss risk capacity remains/);
  assert.match(risk, /cap_trade_risk_budget\(risk_budget, max_account_risk_budget, account_risk\.current_stop_risk\)/);
  assert.match(risk, /requested_trade_budget\.min\(\(account_budget - current_account_risk\)\.max\(0\.0\)\)/);
  assert.match(risk, /trade_budget_is_capped_by_remaining_account_wide_risk_capacity/);
});

test('native margin preflight is correlated, serialized, expiring, and uses terminal calculations only', () => {
  assert.match(bridge, /MARGIN_PREFLIGHT_TIMEOUT: Duration = Duration::from_secs\(8\)/);
  assert.match(bridge, /MARGIN_PREFLIGHT_MAX_AGE_MS: i64 = 5_000/);
  assert.match(bridge, /pending_margin_preflight/);
  assert.match(bridge, /margin_preflight_changed\s*\.\s*wait_timeout/);
  assert.match(bridge, /result\.request_id != request\.request_id/);
  assert.match(bridge, /validate_recent_calculation_timestamp\s*\(\s*result\.calculated_at_ms\s*,\s*now_ms\(\s*\)\s*,\s*MARGIN_PREFLIGHT_MAX_AGE_MS\s*,\s*\)\s*\?/);
  assert.match(bridge, /fn native_margin_calculation_timestamp_rejects_future_and_expired_values/);
  assert.match(bridge, /Successful MT5 native calculation used an invalid, inconsistent, expired, or future-dated bid\/ask quote/);
  assert.match(ea, /OrderCalcMargin\(/);
  assert.match(ea, /OrderCalcProfit\(/);
  assert.match(ea, /position_volume,position_current,position_sl,position_risk_status/);
  assert.match(ea, /Equity already includes floating P\/L/);
  assert.match(ea, /ATOS_MARGIN_PREFLIGHT_MAX_AGE_MS 5000/);
  assert.match(ea, /marginPreflightResults/);
  assert.match(ea, /marginPreflightRequest/);
  assert.match(ea, /BuildMarginPreflightResult\(g_margin_preflight_request_json,snapshot_tick\)/);
  assert.match(ea, /BuildSnapshot\(snapshot,snapshot_tick\)/);
  assert.doesNotMatch(ea, /\bOrderSend\s*\(|\bOrderModify\s*\(|\bOrderDelete\s*\(|\bPositionClose\s*\(/);
});

test('risk decision revalidates live trigger-side stop distance and account-state drift', () => {
  assert.match(risk, /snapshot\.symbol\.bid - request\.stop_loss_price/);
  assert.match(risk, /request\.stop_loss_price - snapshot\.symbol\.ask/);
  assert.match(risk, /preflight\.bid - request\.stop_loss_price/);
  assert.match(risk, /request\.stop_loss_price - preflight\.ask/);
  assert.match(risk, /equity_drift\.max\(margin_drift\)\.max\(free_margin_drift\)/);
  assert.match(risk, /MAX_ACCOUNT_STATE_DRIFT_PERCENT: f64 = 0\.5/);
  assert.match(risk, /MIN_POST_TRADE_FREE_MARGIN_PERCENT_EQUITY: f64 = 20\.0/);
  assert.match(risk, /MIN_MARGIN_LEVEL_PCT: f64 = 300\.0/);
  assert.match(risk, /margin_stopout_mode == 0/);
  assert.match(risk, /margin_stopout_mode == 1/);
  assert.match(risk, /market_trade_permission_block_reason/);
  assert.match(risk, /snapshot\.account\.account_trade_allowed,\s*snapshot\.symbol\.trade_mode/);
  assert.match(risk, /SYMBOL_ORDER_MARKET: i64 = 1/);
  assert.match(risk, /SYMBOL_ORDER_SL: i64 = 16/);
  assert.match(risk, /account_state_drift > MAX_ACCOUNT_STATE_DRIFT_PERCENT/);
  assert.match(risk, /!margin_after\.is_finite\(\s*\)\s*\|\|\s*!free_after\.is_finite\(\s*\)\s*\|\|\s*!stressed_equity\.is_finite\(\s*\)/);
  assert.match(risk, /if margin_after > 0\.0/);
});

test('protective stop-risk cannot be zero, crossed, or profitable-side and native result is sequence-bound', () => {
  assert.match(bridge, /value\.is_finite\(\) && \*value > 0\.0/);
  assert.match(bridge, /protective_stop_is_loss_side/);
  assert.match(bridge, /stop-limit order is missing its broker-reported stop-limit activation price/);
  assert.match(bridge, /rejects_zero_loss_and_non_protective_stops_as_verified_account_protection/);
  assert.match(ea, /loss<=0\.0/);
  assert.match(ea, /stop_loss>=reference_price/);
  assert.match(ea, /stop_loss<=reference_price/);
  assert.match(bridge, /result\.snapshot_sequence = frame\.sequence/);
  assert.match(risk, /status\.last_sequence != preflight\.snapshot_sequence/);
  assert.match(risk, /native margin\/profit calculations used a different quote/);
  assert.match(risk, /the correlated account inventory contains unprotected or unresolved/);
  assert.match(risk, /snapshot_sequence: calculation_snapshot_sequence/);
});

test('EA discards non-atomic position and pending-order inventory frames', () => {
  assert.match(ea, /g_snapshot_position_tickets/);
  assert.match(ea, /g_snapshot_order_tickets/);
  assert.match(ea, /PositionsTotal\(\)!=positions_total/);
  assert.match(ea, /OrdersTotal\(\)!=orders_total/);
  assert.match(ea, /discarded a non-atomic account inventory snapshot/);
  assert.match(ea, /AccountInventoryMatchesLastSnapshot\(\)/);
  assert.match(ea, /if\(!AccountInventoryMatchesLastSnapshot\(\)\)[\s\S]*?discarded telemetry because account financial state or inventory changed before transmission/);
  assert.match(ea, /bool AccountRiskFieldsMatchSnapshot\(\)[\s\S]*?ACCOUNT_EQUITY[\s\S]*?ACCOUNT_MARGIN_FREE[\s\S]*?ACCOUNT_MARGIN_SO_MODE[\s\S]*?TERMINAL_TRADE_ALLOWED[\s\S]*?MQL_TRADE_ALLOWED/);
  assert.match(ea, /AccountInventoryMatchesLastSnapshot\(\)[\s\S]*?if\(!AccountRiskFieldsMatchSnapshot\(\)\)/);
  assert.match(ea, /bool AuthenticatedAccountIdentityMatches\(\)[\s\S]*?ACCOUNT_LOGIN[\s\S]*?ACCOUNT_SERVER[\s\S]*?ACCOUNT_CURRENCY/);
  assert.match(ea, /if\(!AuthenticatedAccountIdentityMatches\(\)\)[\s\S]*?refused connection because the active MT5 account identity changed/);
  assert.match(ea, /return AuthenticatedAccountIdentityMatches\(\)[\s\S]*?SnapshotNumberMatches\(AccountInfoDouble\(ACCOUNT_BALANCE\)/);
  assert.match(ea, /g_account_server=AccountInfoString\(ACCOUNT_SERVER\)/);
  assert.match(ea, /g_account_currency=AccountInfoString\(ACCOUNT_CURRENCY\)/);
  assert.doesNotMatch(ea, /native_preflight_in_frame\s*&&\s*!AccountInventoryMatchesLastSnapshot/);
  assert.match(ea, /g_snapshot_position_prices/);
  assert.match(ea, /g_snapshot_order_stop_losses/);
  assert.match(ea, /position whose risk-bearing fields changed during serialization/);
  assert.match(ea, /pending order whose risk-bearing fields changed during serialization/);
  assert.match(ea, /SnapshotNumberMatches\(PositionGetDouble\(POSITION_VOLUME\)/);
  assert.match(ea, /SnapshotNumberMatches\(OrderGetDouble\(ORDER_VOLUME_CURRENT\)/);
});

test('valid distressed account telemetry retains negative free margin for display and blocking', () => {
  assert.doesNotMatch(bridge, /account\.margin < 0\.0 \|\| account\.free_margin < 0\.0/);
  assert.match(bridge, /negative_free_margin_is_preserved_as_a_distressed_broker_state/);
  assert.match(risk, /free_after < min_free_margin/);
});

// These checks cover stale-result presentation, not identity changes across reconnects.
test('frontend exposes account-wide inventory, broker stop-out policy and pending-order risk detail', () => {
  for (const field of ['marginInitial', 'marginMaintenance', 'marginCallLevel', 'stopOutLevel', 'marginStopoutMode', 'accountHedgeAllowed', 'accountLimitOrders']) {
    assert.match(types, new RegExp(`${field}:`));
  }
  assert.match(types, /export interface Mt5AccountRiskInventory/);
  assert.match(types, /accountRisk: Mt5AccountRiskInventory \| null/);
  assert.match(types, /effectiveRiskBudget: number/);
  assert.match(types, /snapshotSequence: number/);
  assert.match(types, /accountAvailableRiskBudget: number/);
  assert.match(panel, /Account-wide stop-loss risk inventory/);
  assert.match(panel, /Active pending orders/);
  assert.match(panel, /Projected free margin/);
  assert.match(panel, /Stressed margin level/);
  assert.match(panel, /No active pending orders/);
  assert.match(panel, /riskSizingResultStale/);
  assert.match(panel, /STALE — RE-CALCULATE/);
  assert.match(panel, /Snapshot superseded/);
});

test('security and engineering documentation identify protocol v6 and non-authorization limits', () => {
  assert.match(securityDocs, /Protocol v6/);
  assert.match(securityDocs, /OrderCalcMargin/);
  assert.match(securityDocs, /not an `OrderCheck` approval/);
  assert.match(designDocs, /effective_trade_budget = min/);
  assert.match(designDocs, /current mark price because account equity already includes its floating P\/L/);
  assert.match(designDocs, /0\.5% of equity blocks the result/);
  assert.match(designDocs, /No live order submission/);
  assert.match(designDocs, /Terminal Algo Trading and EA live-trading flags remain visible in telemetry but do not gate this read-only calculation/);
  assert.match(statusDocs, /MT5 account-wide risk control plane/);
});

test('terminal preflight serializer has exact StringFormat placeholder parity', () => {
  const start = ea.indexOf('   return StringFormat(\n      "{\\"requestId\\"');
  const end = ea.indexOf('\n   );', start);
  assert.ok(start >= 0 && end > start, 'margin preflight serializer must exist');
  const formatMatch = ea.slice(start, end).match(/"((?:\\.|[^"\\])*)"/);
  assert.ok(formatMatch, 'margin preflight format literal must exist');
  const placeholders = [...formatMatch[1].matchAll(/%(?:I64d|s|d|u|f)/g)].length;
  const argsStart = ea.indexOf('\n      request_id,', start) + '\n      '.length;
  const argsEnd = end;
  const argsText = ea.slice(argsStart, argsEnd);
  let depth = 0;
  let inString = false;
  let escaped = false;
  let commas = 0;
  for (const char of argsText) {
    if (inString) {
      if (escaped) escaped = false;
      else if (char === '\\') escaped = true;
      else if (char === '"') inString = false;
      continue;
    }
    if (char === '"') inString = true;
    else if (char === '(') depth += 1;
    else if (char === ')') depth -= 1;
    else if (char === ',' && depth === 0) commas += 1;
  }
  assert.equal(placeholders, commas + 1, 'every margin preflight placeholder must have exactly one argument');
});
