import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const rust = readFileSync(new URL('../src-tauri/src/infrastructure/ai/agent_runtime.rs', import.meta.url), 'utf8');
const lib = ['adapters/tauri/commands.rs', 'adapters/tauri/bootstrap.rs']
  .map((file) => readFileSync(new URL(`../src-tauri/src/${file}`, import.meta.url), 'utf8')).join('\n');
const component = readFileSync(new URL('../src/lib/components/AgentRuntimePanel.svelte', import.meta.url), 'utf8');
const orchestrator = readFileSync(new URL('../src/lib/application/ai-agent-orchestrator.js', import.meta.url), 'utf8');
const app = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const cargo = readFileSync(new URL('../src-tauri/Cargo.toml', import.meta.url), 'utf8');
const tauri = JSON.parse(readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));

 test('native agent providers are fixed and do not accept arbitrary request URLs', () => {
  assert.match(rust, /https:\/\/api\.openai\.com\/v1\/models/);
  assert.match(rust, /https:\/\/api\.anthropic\.com\/v1\/models/);
  assert.match(rust, /https:\/\/openrouter\.ai\/api\/v1\/models/);
  assert.match(rust, /http:\/\/127\.0\.0\.1:11434\/api\/tags/);
  assert.match(rust, /redirect\(Policy::none\(\)\)/);
  assert.doesNotMatch(rust, /request\.url|user_url|custom_endpoint/);
  assert.match(rust, /connect_timeout\(Duration::from_secs\(5\)\)/);
  assert.match(rust, /timeout\(Duration::from_secs\(35\)\)/);
  assert.match(rust, /MAX_PROVIDER_OPERATIONS/);
  assert.match(rust, /MODEL_REGISTRY_OPERATIONS/);
  assert.match(rust, /Serialize keychain reads with writes\/provider requests/);
});

test('API credentials are confined to OS credential store and zeroized in memory', () => {
  assert.match(cargo, /keyring\s*=\s*\{ version = "=4\.2"/);
  assert.match(cargo, /zeroize\s*=\s*"1\.8"/);
  assert.match(rust, /Entry::new\(SERVICE_NAME, provider\)/);
  assert.match(rust, /entry\.set_password\(secret\.as_str\(\)\)/);
  assert.match(rust, /Zeroizing<String>/);
  assert.match(rust, /No plaintext fallback was used/);
  assert.doesNotMatch(rust, /api_key.*write_all|write_all.*api_key|apiKey.*INSERT INTO/i);
});

test('provider response, prompt, output and network concurrency have hard resource ceilings', () => {
  assert.match(rust, /MAX_MODEL_RESPONSE_BYTES: usize = 256 \* 1024/);
  assert.match(rust, /MAX_PROVIDER_PROBE_BYTES: usize = 32 \* 1024/);
  assert.match(rust, /MAX_ANALYSIS_PROMPT_BYTES: usize = 32 \* 1024/);
  assert.match(rust, /content\.len\(\) > 64 \* 1024/);
  assert.match(rust, /max_tokens\": 1200/);
  assert.match(rust, /num_predict\":1200/);
  assert.match(rust, /MAX_AUDIT_EVENTS/);
  assert.match(rust, /MAX_AGENT_RUNS_PER_HOUR/);
  assert.match(rust, /MAX_AGENT_RUNS_PER_DAY/);
  assert.match(rust, /Failed provider calls count toward the budget/);
});

test('model import is size-bounded, content-hashed, path-scoped, and cannot execute imported weights', () => {
  assert.match(rust, /MAX_MODEL_ARTIFACT_BYTES: u64 = 16 \* 1024 \* 1024 \* 1024/);
  assert.match(rust, /\["gguf", "onnx", "safetensors"\]/);
  assert.match(rust, /file_type\(\)\.is_symlink\(\)/);
  assert.match(rust, /hasher\.update\(&buffer\[\.\.read\]\)/);
  assert.match(rust, /IMPORTED_ONLY_NO_LOCAL_INFERENCE_RUNTIME/);
  assert.match(rust, /MAX_IMPORTED_MODELS/);
  assert.match(rust, /validate_model_artifact_format/);
  assert.match(rust, /validate_onnx_model_header/);
  assert.match(rust, /MAX_ONNX_TOP_LEVEL_FIELDS: usize = 10_000/);
  assert.match(rust, /Validate the bytes actually stored/);
  assert.match(rust, /SafeTensors tensor data offsets exceed/);
  assert.match(rust, /MAX_TOTAL_MODEL_BYTES/);
  assert.match(rust, /Imported model artifact changed after registration/);
  assert.match(rust, /local_model_inference_available: false/);
  assert.match(component, /not executable by this application yet/);
});

test('audit journal is append-only, hash-linked, and has bounded payload size', () => {
  assert.match(rust, /ai_agent_journal_no_update BEFORE UPDATE/);
  assert.match(rust, /ai_agent_journal_no_delete BEFORE DELETE/);
  assert.match(rust, /previous_hash TEXT NOT NULL/);
  assert.match(rust, /event_hash TEXT NOT NULL UNIQUE/);
  assert.match(rust, /payload_json\.len\(\) > 24 \* 1024/);
  assert.match(rust, /verify_audit_chain/);
  assert.match(rust, /require_valid_audit_chain/);
  assert.match(rust, /provider access and mutations are blocked/);
  assert.match(rust, /Permissions::from_mode\(0o700\)/);
  assert.match(rust, /Permissions::from_mode\(0o600\)/);
});

test('Tauri commands and the visible AI Agent panel are wired into the application', () => {
  for (const command of [
    'get_ai_agent_runtime_status', 'save_ai_provider_key', 'remove_ai_provider_key',
    'test_ai_provider_connection', 'list_imported_ai_models', 'import_local_ai_model',
    'verify_imported_ai_model', 'remove_imported_ai_model', 'verify_ai_agent_audit_chain', 'run_ai_research_agent',
  ]) assert.match(lib, new RegExp(command));
  assert.match(lib, /\.plugin\(tauri_plugin_dialog::init\(\)\)/);
  assert.match(tauri.app.security.csp || '', /default-src/);
  assert.match(component, /Save key securely/);
  assert.match(component, /Import model file/);
  assert.match(component, /Expected artifact SHA-256/);
  assert.match(component, /expectedSha256: expectedDigest \|\| null/);
  assert.match(component, /Remove imported model/);
  assert.match(component, /Provider abuse \/ cost budget/);
  assert.match(component, /Run research analyst/);
  assert.match(component, /Closed-bar cutoff:/);
  assert.match(component, /Snapshot received:/);
  assert.match(component, /retrievedAtMs/);
  assert.match(app, /<AgentRuntimePanel desktop=\{desktop\}/);
  assert.match(app, /Execution authority remains disabled/);
  assert.match(app, /nine-specialist, 364-item workflow remains plan-only/);
  assert.match(app, /no specialist dispatch or per-concept receipts are active/);
});

test('LLM output is not a score input and the evidence envelope is verified before deterministic scoring', () => {
  assert.match(orchestrator, /RUST_DERIVED_DETERMINISTIC_EVIDENCE_ONLY/);
  assert.match(orchestrator, /evidenceEnvelopeSha256/);
  assert.match(orchestrator, /Native scoring envelope digest mismatch/);
  assert.match(orchestrator, /payloadCanonicalJson/);
  assert.match(orchestrator, /payloadSha256/);
  assert.match(orchestrator, /Native evidence payload digest mismatch/);
  assert.match(rust, /post_provider_snapshot_revalidated|postProviderSnapshotRevalidated/);
  assert.match(rust, /market_snapshot_changed_during_provider_request/);
  assert.match(rust, /fn same_fresh_market_snapshot/);
  assert.match(rust, /fn market_series_is_fresh/);
  assert.match(rust, /series\.latest_closed_bar_at_ms/);
  assert.match(rust, /series\.retrieved_at_ms/);
  assert.match(rust, /latest_closed_bar_at_ms: series\.latest_closed_bar_at_ms/);
  assert.match(rust, /retrieved_at_ms: series\.retrieved_at_ms/);
  assert.match(rust, /confirmed\.latest_closed_bar_at_ms == initial\.latest_closed_bar_at_ms/);
  assert.match(rust, /confirmed\.retrieved_at_ms == initial\.retrieved_at_ms/);
  assert.match(orchestrator, /latestClosedBarAtMs/);
  assert.match(orchestrator, /retrievedAtMs/);
  assert.match(orchestrator, /MAX_MARKET_RECEIPT_AGE_MS = 120_000/);
  assert.match(orchestrator, /timestampsRemainFresh/);
  assert.match(rust, /unchanged_fresh_snapshot_is_accepted/);
  assert.match(rust, /stale_or_changed_snapshot_is_rejected/);
  assert.ok(rust.indexOf('call_provider_for_research(&provider_id') < rust.indexOf('same_fresh_market_snapshot(&market_context, &confirmed_context)'));
  assert.ok(rust.indexOf('same_fresh_market_snapshot(&market_context, &confirmed_context)') < rust.indexOf('let report = AgentResearchReport'));
  assert.match(rust, /AgenticTradingOS\/0\.11\.0/);
  assert.match(orchestrator, /aiNarrativeExcludedFromScoring: true/);
  assert.match(orchestrator, /verifyCommentaryDigest/);
  assert.match(orchestrator, /commentary digest mismatch/);
  assert.match(orchestrator, /cites evidence outside its assigned domain/);
  assert.match(orchestrator, /MAX_SUMMARY_CHARS/);
  assert.match(orchestrator, /executionAuthority: 'none'/);
  assert.match(orchestrator, /orderIntent: null/);
  assert.match(rust, /execution_authority: "none"/);
  assert.match(rust, /live_execution_authorized: false/);
  assert.match(rust, /AI provider calls require a fresh, connected live market snapshot/);
  assert.match(rust, /AI provider calls require a fresh, connected live market snapshot[\s\S]*?reserve_and_record_agent_run\s*\([\s\S]*?&data_dir/, 'stale input must be blocked before reserving or spending provider budget');
});
