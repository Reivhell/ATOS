import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const root = new URL('../', import.meta.url);
const read = (file) => readFile(new URL(file, root), 'utf8');

test('all durable engine-state writes are gated against the exact SQLite predecessor', async () => {
  const engine = await read('src-tauri/src/runtime/trading_core.rs');
  const native = await read('src-tauri/src/domain/mod.rs');
  const persist = engine.slice(engine.indexOf('fn persist('), engine.indexOf('fn persist_test_fixture('));
  assert.match(native, /^pub mod state_decision;/m);
  assert.match(engine, /use crate::domain::state_decision::evaluate_state_commit/);
  assert.match(persist, /SELECT payload FROM app_state WHERE singleton = 1/);
  assert.match(persist, /serde_json::from_str::<serde_json::Value>/);
  assert.match(persist, /evaluate_state_commit\(/);
  assert.ok(persist.indexOf('evaluate_state_commit(') < persist.indexOf('INSERT INTO app_state'));
  assert.match(persist, /if !decision\.allowed/);
  assert.match(persist, /State commit rejected by strict native decision layer/);
  assert.match(persist, /transaction\s*\.\s*commit\s*\(/);
  assert.match(engine, /CREATE TRIGGER IF NOT EXISTS audit_events_no_update/);
  assert.match(engine, /CREATE TRIGGER IF NOT EXISTS audit_events_no_delete/);
  assert.match(engine, /fn durable_audit_rows_reject_update_and_delete\(\)/);
  assert.match(persist, /Self::state_is_valid\(state\)/);
});

test('state policy explicitly classifies changes and preserves append-only audit history', async () => {
  const policy = await read('src-tauri/src/domain/state_decision.rs');
  assert.match(policy, /fn order_risk_checks_match_state\(/);
  assert.match(policy, /fn paper_fill_economics_are_consistent\(/);
  assert.match(policy, /new_orders\.iter\(\)\.skip\(1\)\.ne\(old_orders\.iter\(\)\)/);
  for (const contract of [
    'CLASSIFIED_FIELDS', 'event_history_is_append_only', 'immutable', 'risk_limit_changes_require_a_new_risk_policy_audit_event',
    'market_observation_changes_require_a_provider_or_startup_invalidation_audit_event',
    'screener_changes_require_a_provider_or_startup_cleanup_audit_event',
    'agent_state_changes_require_an_operator_control_or_migration_audit_event',
    'order_ledger_may_only_append_one_complete_risk_checked_fill_or_rejection_without_rewriting_prior_orders',
    'paper_fill_must_commit_order_cash_and_position_exposure_atomically',
    'paper_fill_cash_quantity_cost_basis_fee_and_realized_pnl_must_match_the_order_and_prior_state',
    'equity_peak_drawdown_and_daily_baseline_must_follow_their_deterministic_state_equations',
    'kill_switch_release_requires_a_fresh_connected_provider_and_fresh_portfolio_marks',
  ]) assert.ok(policy.includes(contract), `Missing native state-decision contract: ${contract}`);
  for (const testName of [
    'bootstrap_fails_closed_until_account_is_empty_and_locked',
    'risk_policy_cannot_change_without_new_audit_evidence',
    'kill_switch_release_requires_fresh_provider_and_portfolio_quotes',
    'reset_requires_empty_account_initial_cash_and_engaged_lock',
    'order_fill_requires_atomic_cash_position_order_and_risk_evidence',
    'order_append_cannot_rewrite_a_previous_ledger_record',
    'market_observations_require_new_provider_audit_evidence',
    'unknown_state_fields_are_blocked_until_explicitly_classified',
    'audit_history_cannot_be_rewritten_or_cleared',
    'daily_baseline_rollover_is_valid_when_equity_is_unchanged',
    'order_risk_check_uses_the_candidate_daily_baseline_after_utc_rollover',
    'removed_unclassified_state_fields_are_rejected',
  ]) assert.ok(policy.includes(`fn ${testName}()`), `Missing Rust policy test: ${testName}`);
});

test('paper-account reset always re-engages the kill switch', async () => {
  const engine = await read('src-tauri/src/runtime/trading_core.rs');
  const reset = engine.slice(engine.indexOf('pub fn reset_paper_account('), engine.indexOf('pub fn refresh_market_data('));
  assert.match(reset, /state\.orders\.clear\(\)/);
  assert.match(reset, /state\.positions\.clear\(\)/);
  assert.match(reset, /state\.kill_switch_engaged = true/);
  assert.match(reset, /Paper account reset/);
  assert.ok(reset.indexOf('state.kill_switch_engaged = true') < reset.indexOf('Self::push_event('));
  assert.match(engine, /fn resetting_after_kill_switch_release_reengages_it_and_requires_explicit_rearm\(\)/);
});


test('durable state commits are hash-chained and verified before recovery or mutation', async () => {
  const engine = await read('src-tauri/src/runtime/trading_core.rs');
  const persist = engine.slice(engine.indexOf('fn persist('), engine.indexOf('fn persist_test_fixture('));
  const open = engine.slice(engine.indexOf('pub fn open('), engine.indexOf('pub fn strict_analysis_store_path('));
  assert.match(engine, /fn state_commit_record_hash\(/);
  assert.match(engine, /fn read_state_commit_chain_head\(/);
  assert.match(engine, /fn append_state_commit_record\(/);
  assert.match(engine, /CREATE TABLE IF NOT EXISTS state_commit_ledger/);
  assert.match(engine, /CREATE TRIGGER IF NOT EXISTS state_commit_ledger_no_update/);
  assert.match(engine, /CREATE TRIGGER IF NOT EXISTS state_commit_ledger_no_delete/);
  assert.match(open, /verify_existing_state_commit_chain\(&persistence_path\)/);
  assert.match(persist, /read_state_commit_chain_head\(&transaction, previous_payload\.as_deref\(\)\)/);
  assert.match(persist, /append_state_commit_record\(/);
  assert.match(persist, /decision\.transition/);
  assert.match(engine, /fn durable_state_commit_chain_detects_payload_tampering_and_rejects_receipt_mutation\(\)/);
  assert.match(engine, /fn schema_v2_refuses_startup_when_the_receipt_chain_is_removed\(\)/);
  assert.match(engine, /database_version >= 2 && head\.is_none\(\)/);
  assert.match(engine, /schema_version_before_migration >= 2 && chain_head\.is_none\(\)/);
  assert.match(engine, /PRAGMA user_version = 2/);
  assert.match(engine, /LEGACY_BASELINE_UNVERIFIED/);
});
