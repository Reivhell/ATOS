import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const strictRust = readFileSync(new URL('../src-tauri/src/application/strict_analysis.rs', import.meta.url), 'utf8');
const tauriLib = readFileSync(new URL('../src-tauri/src/adapters/tauri/commands.rs', import.meta.url), 'utf8');
const bridge = readFileSync(new URL('../src/lib/adapters/tauri/bridge.ts', import.meta.url), 'utf8');
const types = readFileSync(new URL('../src/lib/types.ts', import.meta.url), 'utf8');

test('durable audit chain binds issuance, single-use consumption and finalized outcomes', () => {
  for (const token of [
    'strict_audit_chain_events', 'strict_audit_chain_head', 'AUDIT_CHAIN_GENESIS',
    'previous_hash', 'event_sha256', 'verify_audit_chain_connection',
    'work_plan_issued', 'work_plan_consumed', 'assessment_attempt_started', 'assessment_attempt_finalized',
    'schema_migrated', 'schema_migrated_v2_to_v3', 'STRICT_STORE_SCHEMA_VERSION',
  ]) assert.ok(strictRust.includes(token), `Missing audit-chain contract token: ${token}`);
  assert.match(strictRust, /verify_audit_chain_connection\(&tx\)/);
  assert.match(strictRust, /head mismatch; tail truncation or head tampering is suspected/);
  assert.match(strictRust, /possible tampering detected/);
  assert.match(strictRust, /schema_v1_store_migration_imports_existing_plan_into_the_chain/);
  assert.match(strictRust, /\"schema_migrated_v2_to_v3\"/);
  assert.doesNotMatch(strictRust, /\"schema_migrated_v2_to_v3_specialist_dispatch_receipts\"/);
  assert.match(strictRust, /audit_chain_rejects_modified_event_and_truncated_tail_fail_closed/);
  assert.match(strictRust, /semantic cross-check must detect receipt tampering even if the row trigger is bypassed/);
});

test('audit-chain integrity is exposed through the native boundary, not browser preview', () => {
  assert.match(tauriLib, /fn verify_strict_analysis_audit_chain/);
  assert.match(tauriLib, /verify_strict_analysis_audit_chain/);
  assert.match(bridge, /verifyStrictAnalysisAuditChain/);
  assert.match(bridge, /invoke<StrictAuditChainStatus>\('verify_strict_analysis_audit_chain'\)/);
  assert.match(types, /export interface StrictAuditChainStatus/);
});


test('durable specialist receipts are required, hash-bound, append-only and never browser-self-attested', () => {
  const policy = JSON.parse(readFileSync(new URL('../docs/STRICT_DECISION_LAYER_POLICY.json', import.meta.url), 'utf8'));
  const receiptDoc = readFileSync(new URL('../docs/STRICT_SPECIALIST_DISPATCH_RECEIPTS_0.4.2.md', import.meta.url), 'utf8');
  for (const token of [
    'strict_specialist_dispatch_receipts', 'specialist_receipt_recorded',
    'schema_migrated_v2_to_v3', 'pub(crate) fn persist_specialist_dispatch_receipt',
    'fn specialist_dispatch_receipt_gaps', 'Missing durable execution receipt',
    'persistent_gate_requires_nine_durable_specialist_receipts_and_never_fakes_dispatch',
    'specialist_receipts_bind_exact_category_output_and_are_append_only',
  ]) assert.ok(strictRust.includes(token), `Missing durable specialist receipt contract token: ${token}`);
  assert.match(strictRust, /CREATE TRIGGER IF NOT EXISTS strict_dispatch_receipts_no_update/);
  assert.match(strictRust, /CREATE TRIGGER IF NOT EXISTS strict_dispatch_receipts_no_delete/);
  assert.doesNotMatch(tauriLib, /persist_specialist_dispatch_receipt/);
  assert.equal(policy.requiredSpecialistReceiptCount, 9);
  assert.equal(policy.providerDispatchConfigured, false);
  assert.ok(policy.downstreamHandoffRequires.includes('exactly_nine_durable_specialist_dispatch_receipts_for_the_current_work_plan'));
  assert.match(receiptDoc, /current runtime has no configured AI-provider dispatcher/);
  assert.match(receiptDoc, /not registered as a Tauri command/);
  for (let i = 1; i <= 9; i += 1) {
    const files = readFileSync(new URL(`../docs/agents/${String(i).padStart(2, '0')}-${[
      'smc-ict-agent', 'price-action-agent', 'support-resistance-trend-agent', 'technical-indicators-agent',
      'order-flow-market-profile-agent', 'divergence-intermarket-agent', 'other-methods-agent',
      'fundamental-macro-agent', 'risk-management-evaluation-agent',
    ][i - 1]}.md`, import.meta.url), 'utf8');
    assert.match(files, /Runtime execution proof \(strict v0\.4\.2\)/);
    assert.match(files, /must block handoff until a trusted runtime executes and persists the result/);
  }
});
