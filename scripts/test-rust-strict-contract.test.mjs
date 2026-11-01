import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const rustGate = readFileSync(new URL('../src-tauri/src/application/strict_analysis.rs', import.meta.url), 'utf8');
const commands = readFileSync(new URL('../src-tauri/src/adapters/tauri/commands.rs', import.meta.url), 'utf8');
const rustLib = [
  commands,
  readFileSync(new URL('../src-tauri/src/adapters/tauri/bootstrap.rs', import.meta.url), 'utf8'),
  readFileSync(new URL('../src-tauri/src/application/strict_analysis_service.rs', import.meta.url), 'utf8'),
].join('\n');
const engine = readFileSync(new URL('../src-tauri/src/runtime/trading_core.rs', import.meta.url), 'utf8');
const manifest = JSON.parse(readFileSync(new URL('./fixtures/technical-analysis-master-list.v1.json', import.meta.url), 'utf8'));
const bridge = readFileSync(new URL('../src/lib/adapters/tauri/bridge.ts', import.meta.url), 'utf8');
const workPlanSchema = JSON.parse(readFileSync(new URL('../docs/agents/strict-ta-work-plan.schema.json', import.meta.url), 'utf8'));
const auditReceiptSchema = JSON.parse(readFileSync(new URL('../docs/agents/strict-ta-audit-receipt.schema.json', import.meta.url), 'utf8'));

test('Rust strict gate embeds the reviewed exact 364-entry catalog and all nine specialist routes', () => {
  assert.match(rustGate, /include_str!\("\.\.\/\.\.\/\.\.\/scripts\/fixtures\/technical-analysis-master-list\.v1\.json"\)/);
  assert.match(rustGate, /pub const REQUIRED_CONCEPT_COUNT: usize = 364/);
  assert.match(rustGate, /pub fn specialist_for_category/);
  assert.match(rustGate, /pub fn reviewed_implementation_version/);
  assert.match(rustGate, /Rust-owned reviewed version/);
  assert.match(rustGate, /pub fn knowledge_pack_for_category|fn knowledge_pack_for_category/);
  assert.equal(manifest.entries.length, 364);
  assert.equal(manifest.categories.length, 9);
  for (const entry of manifest.entries) assert.ok(entry.id === `${entry.category}::${entry.group}::${entry.name}`);
  for (const agentId of [
    'agent-01-smc-ict', 'agent-02-price-action', 'agent-03-support-resistance-trend',
    'agent-04-technical-indicators', 'agent-05-order-flow-market-profile',
    'agent-06-divergence-intermarket', 'agent-07-other-methods', 'agent-08-fundamental-macro',
    'agent-09-risk-management-evaluation',
  ]) assert.ok(rustGate.includes(agentId), `Missing Rust specialist routing: ${agentId}`);
});

test('Rust gate refuses handoff on unsupported catalog items and exposes no live execution authority', () => {
  assert.match(rustGate, /Catalog-only concept must remain unsupported/);
  assert.match(rustGate, /Unsupported concept blocks downstream decisions/);
  assert.match(rustGate, /Insufficient data blocks downstream decisions/);
  assert.match(rustGate, /let next_layer_payload = if allowed/);
  assert.match(rustGate, /else\s*\{\s*None\s*\}/);
  assert.match(rustGate, /live_execution_authorized: false/);
  assert.match(rustGate, /fn full_inventory_still_blocks_handoff_when_catalog_only_concepts_exist/);
});

test('Tauri command surface exposes Rust work planning and completion; browser does not fake it', () => {
  assert.match(rustLib, /create_strict_analysis_work_plan/);
  assert.match(rustLib, /evaluate_strict_analysis_completion/);
  assert.match(bridge, /createStrictAnalysisWorkPlan[\s\S]*?requireDesktopRuntime\('strict analysis'\)[\s\S]*?invoke<StrictAnalysisWorkPlan>\('create_strict_analysis_work_plan'/);
  assert.match(bridge, /evaluateStrictAnalysisCompletion[\s\S]*?requireDesktopRuntime\('strict analysis'\)[\s\S]*?invoke<StrictCompletionResult>\('evaluate_strict_analysis_completion'/);
  assert.match(bridge, /assertNativeRuntime\(isDesktopRuntime\(\), capability\)/);
});

// This static test checks wiring only. It is explicitly not a substitute for `cargo test`.


test('Native Tauri boundary does not trust UI-supplied data or risk approvals', () => {
  assert.match(rustLib, /pub\(crate\) fn authoritative_strict_series\(/);
  assert.match(rustLib, /series\.is_live/);
  assert.match(rustLib, /series\.provider_status\.starts_with\("CONNECTED"\)/);
  assert.match(rustLib, /series\.source\.starts_with\("Binance public REST"\)/);
  assert.match(rustLib, /Strict analysis requires a fresh connected Rust-owned Binance public-data snapshot/);
  assert.match(rustGate, /Strict completion requires trusted Rust-owned Binance public REST provenance/);
  assert.match(commands, /request\.market_data = strict_analysis::StrictMarketDataContext/);
  assert.match(commands, /request\.risk = match core\.strict_analysis_risk_preflight\(\)/);
  assert.match(rustLib, /request\.market_data = strict_analysis::StrictMarketDataContext/);
  assert.match(rustLib, /request\.risk = match core\.strict_analysis_risk_preflight\(\)/);
  assert.match(engine, /pub fn strict_analysis_risk_preflight\(/);
  assert.match(engine, /state\.kill_switch_engaged/);
  assert.match(engine, /state\.risk_limits\.max_daily_loss/);
});

test('Issued work-plan registry storage tuple matches the consumed-state implementation', () => {
  assert.match(rustGate, /type IssuedWorkPlanRegistry = Mutex<HashMap<String,/);
  assert.match(rustGate, /static ISSUED_WORK_PLANS: OnceLock<IssuedWorkPlanRegistry>/);
  assert.match(rustGate, /Some\(\(_, issued, consumed\)\)/);
});

test('Strict native work plans are process-issued, bounded, expire, and reject tampering', () => {
  assert.match(rustGate, /work_plan_id: Uuid::new_v4\(\)\.to_string\(\)/);
  assert.match(rustGate, /WORK_PLAN_TTL_MS/);
  assert.match(rustGate, /MAX_ISSUED_WORK_PLANS/);
  assert.match(rustGate, /work_plan_is_unmodified_and_unexpired/);
  assert.match(rustGate, /unknown, modified, expired, or already consumed/);
  assert.match(rustGate, /not fully closed at the asOf cutoff/);
});


test('Work-plan schema pins versions, exact inventory size, issued-plan identity, and non-dispatch status', () => {
  assert.equal(workPlanSchema.properties.schemaVersion.const, 'strict-ta-work-plan.v1');
  assert.equal(workPlanSchema.properties.decisionGateContractVersion.const, 'strict-ta-decision-gate.v4');
  assert.equal(workPlanSchema.properties.expectedAssessmentCount.const, 364);
  assert.equal(workPlanSchema.properties.entries.minItems, 364);
  assert.equal(workPlanSchema.properties.entries.maxItems, 364);
  assert.equal(workPlanSchema.properties.dispatchStatus.const, 'work_plan_created_not_dispatched');
  assert.equal(workPlanSchema.properties.liveExecutionAuthorized.const, false);
  assert.ok(workPlanSchema.required.includes('workPlanId'));
  assert.equal(workPlanSchema.properties.tasks.minItems, 9);
  assert.equal(workPlanSchema.properties.tasks.maxItems, 9);
});


test('Native strict completion emits a content-bound audit receipt without weakening the gate', () => {
  assert.match(rustGate, /pub struct StrictAuditReceipt/);
  assert.match(rustGate, /assessment_bundle_sha256/);
  assert.match(rustGate, /category_assessment_sha256/);
  assert.match(rustGate, /reasons_sha256/);
  assert.match(rustGate, /receipt_sha256 = sha256_serializable\(&audit_receipt\)/);
  assert.match(rustGate, /audit_receipt_exists_for_blocked_result_and_binds_all_categories_and_input_bundles/);
  assert.match(rustGate, /audit_assessment_bundle_hash_is_independent_of_input_assessment_order/);
  assert.match(rustGate, /let allowed = reasons\.is_empty\(\)\s*&&\s*assessed == REQUIRED_CONCEPT_COUNT\s*&&\s*catalog_only == 0\s*&&\s*unsupported == 0\s*&&\s*insufficient_data == 0/);
  assert.match(rustGate, /live_execution_authorized: false/);
});


test('Audit receipt schema fixes identity, digest shapes, category coverage, and disabled live authority', () => {
  assert.equal(auditReceiptSchema.properties.schemaVersion.const, 'strict-analysis-audit.v1');
  assert.equal(auditReceiptSchema.properties.masterListVersion.const, '1.0.0');
  assert.equal(auditReceiptSchema.properties.liveExecutionAuthorized.const, false);
  assert.equal(auditReceiptSchema.properties.categoryAssessmentSha256.minProperties, 9);
  assert.equal(auditReceiptSchema.properties.categoryAssessmentSha256.maxProperties, 9);
  for (const field of ['datasetSha256', 'assessmentBundleSha256', 'reasonsSha256', 'receiptSha256']) {
    assert.equal(auditReceiptSchema.properties[field].pattern, '^[a-f0-9]{64}$');
  }
  assert.match(auditReceiptSchema.description, /not a digital signature/);
});


test('Audit receipt verifier enforces self-integrity, schema, catalog version, specialist digests, and no-live invariant', () => {
  assert.match(rustGate, /pub fn verify_audit_receipt\(/);
  assert.match(rustGate, /Audit receipt self-hash mismatch/);
  assert.match(rustGate, /cannot authorize live execution/);
  assert.match(rustGate, /all nine specialist categories/);
  assert.match(rustGate, /verify_audit_receipt\(&audit_receipt\)/);
  assert.match(rustGate, /audit_receipt_verifier_accepts_original_and_rejects_tampering/);
  assert.match(rustGate, /audit_receipt_verifier_rejects_missing_specialist_digest/);
});


test('Native work plans are atomically single-use and replay attempts fail closed', () => {
  assert.match(rustGate, /consumed: bool|StrictAnalysisWorkPlan, bool/);
  assert.match(rustGate, /fn consume_work_plan_once/);
  assert.match(rustGate, /each plan permits exactly one completion attempt/);
  assert.match(rustGate, /work_plan_is_single_use_and_replay_is_fail_closed/);
  assert.match(rustGate, /tampered_plan_cannot_consume_the_original_issued_plan/);
});


test('Production Tauri path uses the durable SQLite work-plan registry, not process-memory replay state', () => {
  assert.match(rustLib, /create_work_plan_persisted\(request, &core\.strict_analysis_store_path\(\)\)/);
  assert.match(rustLib, /evaluate_completion_persisted\(request, &core\.strict_analysis_store_path\(\)\)/);
  assert.match(engine, /pub fn strict_analysis_store_path\(&self\)/);
  assert.match(rustGate, /BEGIN|TransactionBehavior::Immediate/);
  assert.match(rustGate, /CREATE TABLE IF NOT EXISTS strict_work_plans/);
  assert.match(rustGate, /CREATE TABLE IF NOT EXISTS strict_analysis_attempts/);
  assert.match(rustGate, /assessment_bundle_json TEXT NULL/);
  assert.match(rustGate, /fn persist_completion_result/);
  assert.match(rustGate, /verify_audit_receipt\(&result\.audit_receipt\)/);
  assert.match(rustGate, /consumed_at_ms INTEGER NULL/);
  assert.match(rustGate, /persistent_work_plan_consumption_survives_connection_reopen_and_rejects_replay/);
  assert.match(rustGate, /persistent_tampering_does_not_consume_the_authentic_plan/);
  assert.match(rustGate, /persistent_completion_consumes_even_when_assessment_gate_blocks/);
});
