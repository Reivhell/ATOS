import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
const [bridge, risk, ea, types, panel, docs] = await Promise.all([
  read('src-tauri/src/infrastructure/mt5/bridge.rs'),
  read('src-tauri/src/application/mt5_risk_sizing.rs'),
  read('MQL5/Experts/ATOS_Bridge.mq5'),
  read('src/lib/types.ts'),
  read('src/lib/components/Mt5BridgePanel.svelte'),
  read('docs/MT5_PENDING_ORDER_EXPOSURE_RECONCILIATION_2026-10-31.md'),
]);

test('protocol v6 requires complete bounded pending-order inventory', () => {
  assert.match(bridge, /PROTOCOL_VERSION: u16 = 6/);
  assert.match(bridge, /pending_orders: Vec<Mt5PendingOrderSnapshot>/);
  assert.match(bridge, /MAX_PENDING_ORDERS: usize = 200/);
  assert.match(bridge, /snapshot\.pending_orders\.len\(\) > MAX_PENDING_ORDERS/);
  assert.match(bridge, /order_tickets\.insert/);
  assert.match(bridge, /order\.volume_current > order\.volume_initial/);
  assert.match(bridge, /const ORDER_STATE_PARTIAL: i64 = 3/);
  assert.match(bridge, /for terminal_state in \[2_i64, 4, 5, 6\]/);
  assert.match(bridge, /fn is_active_pending_order_state\(state: i64\)/);
  assert.match(bridge, /ORDER_STATE_PARTIAL/);
  assert.match(bridge, /ORDER_STATE_CANCELED/);
  assert.match(bridge, /terminal MQL5 order state \{terminal_state\} must not appear/);
  assert.match(ea, /#define ATOS_PROTOCOL_VERSION 6/);
  assert.match(ea, /OrdersTotal\(\)/);
  assert.match(ea, /OrderSelect\(ticket\)/);
  assert.match(ea, /ORDER_TIME_SETUP_MSC/);
  assert.match(ea, /ORDER_VOLUME_CURRENT/);
  assert.match(ea, /pendingOrders/);
  assert.match(ea, /ATOS_MAX_PENDING_ORDERS/);
});

test('directional volume limit includes remaining pending orders and open positions', () => {
  assert.match(risk, /same_direction_volume\(&snapshot\.positions/);
  assert.match(risk, /same_direction_pending_volume\(&snapshot\.pending_orders/);
  assert.match(risk, /total_directional_symbol_volume \+ normalized_volume > volume_limit/);
  assert.match(risk, /matches!\(order_type, 2 \| 4 \| 6\)/);
  assert.match(risk, /pending_order_volume_is_aggregated_by_symbol_and_direction/);
  assert.match(risk, /fn checked_volume_total\([\s\S]*?volume\.is_finite\(\) \|\| volume < 0\.0[\s\S]*?next\.is_finite\(\)/);
  assert.match(risk, /directional_volume_aggregation_rejects_non_finite_totals/);
  assert.match(risk, /pending_symbol_volume/);
});

test('frontend exposes the broker inventory and separate pending directional volume', () => {
  assert.match(types, /export interface Mt5PendingOrderSnapshot/);
  assert.match(types, /pendingOrders: Mt5PendingOrderSnapshot\[\]/);
  assert.match(panel, /pendingOrders\.length/);
  assert.match(panel, /pendingSymbolVolume/);
  assert.match(panel, /totalDirectionalSymbolVolume/);
});

test('no order mutation surface is introduced', () => {
  assert.doesNotMatch(ea, /\bOrderSend\s*\(|\bOrderModify\s*\(|\bOrderDelete\s*\(/);
  assert.match(docs, /Protocol v6 is incompatible with protocol v5/);
  assert.match(ea, /OrderCalcMargin/);
  assert.match(ea, /OrderCalcProfit/);
});
