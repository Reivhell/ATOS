import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const [nativeModule, nativeCommands, bootstrap, bridge, runtimeGuard, app, types, backtest, engine] = await Promise.all([
  readFile(new URL('../src-tauri/src/application/research_workspace.rs', import.meta.url), 'utf8'),
  readFile(new URL('../src-tauri/src/adapters/tauri/commands.rs', import.meta.url), 'utf8'),
  readFile(new URL('../src-tauri/src/adapters/tauri/bootstrap.rs', import.meta.url), 'utf8'),
  readFile(new URL('../src/lib/adapters/tauri/bridge.ts', import.meta.url), 'utf8'),
  readFile(new URL('../src/lib/adapters/tauri/native-runtime-guard.js', import.meta.url), 'utf8'),
  readFile(new URL('../src/App.svelte', import.meta.url), 'utf8'),
  readFile(new URL('../src/lib/types.ts', import.meta.url), 'utf8'),
  readFile(new URL('../src-tauri/src/application/backtest.rs', import.meta.url), 'utf8'),
  readFile(new URL('../src-tauri/src/runtime/trading_core.rs', import.meta.url), 'utf8'),
]);

test('research workspace uses a separate durable database and preserves the account ledger boundary', () => {
  assert.match(nativeCommands, /research_workspace::ResearchWorkspace/);
  assert.match(bootstrap, /ResearchWorkspace::open\(app_data_dir\.join\("research-workspace\.sqlite3"\)\)/);
  assert.doesNotMatch(nativeModule, /EngineState|place_order\(|toggle_kill_switch\(|persist\(/);
  assert.match(nativeModule, /core\.run_research_backtest_bundle\(/);
  assert.match(nativeModule, /core\.research_risk_limits\(\)/);
  assert.doesNotMatch(nativeModule, /core\.snapshot\(\)/);
});

test('hypothesis and run-card persistence is bounded, transactional and tamper-evident', () => {
  assert.match(nativeModule, /MAX_HYPOTHESES: usize = 200/);
  assert.match(nativeModule, /MAX_RUN_CARDS: usize = 100/);
  assert.match(nativeModule, /MAX_RESEARCH_EVENTS: i64 = 1_024/);
  assert.match(nativeModule, /MAX_RUN_CARD_BYTES: usize = 1_000_000/);
  assert.match(nativeModule, /TransactionBehavior::Immediate/);
  assert.match(nativeModule, /research_hypotheses_identity_no_update/);
  assert.match(nativeModule, /research_hypotheses_no_delete/);
  assert.match(nativeModule, /verify_hypotheses/);
  assert.match(nativeModule, /hypothesis_state_tampering_without_a_matching_event_fails_integrity_check/);
  assert.match(nativeModule, /research_event_chain_no_update/);
  assert.match(nativeModule, /research_event_chain_no_delete/);
  assert.match(nativeModule, /research_run_cards_no_update/);
  assert.match(nativeModule, /research_run_cards_no_delete/);
  assert.match(nativeModule, /does not exactly match the required immutable-table policy/);
  assert.match(nativeModule, /persisted_card_count >= MAX_RUN_CARDS as i64/);
  assert.match(nativeModule, /verify_schema_contract/);
  assert.match(nativeModule, /validate_run_card_semantics/);
  assert.match(nativeModule, /weakened_immutability_trigger_fails_closed_on_reopen/);
  assert.match(nativeModule, /sha256_hex\(card_json\.as_bytes\(\)\)/);
  assert.match(nativeModule, /event_hash\s*\(\s*sequence\s*,\s*timestamp_ms/);
  assert.match(nativeModule, /revision=revision\+1/);
  assert.match(nativeModule, /Research hypotheses currently accept only the validated BTCUSDT 5m native candle series/);
});

test('research rejects unsupported market data and reuses only native validated backtests', () => {
  assert.match(nativeModule, /symbol != "BTCUSDT" \|\| request\.interval != CandleInterval::FiveMinutes/);
  assert.match(nativeModule, /core\.run_research_backtest_bundle\(/);
  assert.match(nativeModule, /core\.research_risk_limits\(\)/);
  assert.doesNotMatch(nativeModule, /core\.snapshot\(\)/);
  assert.match(nativeModule, /result\.median_candle_interval_ms != interval\.duration_ms\(\)/);
  assert.match(nativeModule, /result\.dataset_encoding_version != 1/);
  assert.match(nativeModule, /!valid_sha256\(&result\.dataset_sha256\)/);
  assert.match(nativeModule, /MAX_BOOTSTRAP_OBSERVATIONS: usize = 500_000/);
  assert.match(nativeModule, /BOOTSTRAP_WORK_BUDGET: u64 = 25_000_000/);
  assert.match(nativeModule, /moving-block bootstrap of strategy equity returns; PCG32 v1/);
  assert.match(nativeModule, /struct Pcg32/);
  assert.match(nativeModule, /let threshold = bound\.wrapping_neg\(\) % bound/);
  assert.match(nativeModule, /pcg32_is_reproducible_and_bounded_sampling_stays_in_range/);
  assert.doesNotMatch(nativeModule, /circular moving-block bootstrap/);
  assert.doesNotMatch(nativeModule, /return_p05_pct\.is_none_or\(\|value\| value <= -maximum_drawdown_pct\)/);
});

test('multi-specialist workflow is a validated DAG with independent robustness branches', () => {
  assert.match(nativeModule, /fn validate_workflow_graph/);
  assert.match(nativeModule, /fn validate_run_card_graph_contract/);
  assert.match(nativeModule, /canonical identity, dependency, timestamp, or detail contract/);
  assert.match(nativeModule, /DAG status does not match its authoritative review evidence/);
  assert.match(nativeModule, /run_card_graph_contract_rejects_missing_or_rewired_dependencies/);
  assert.match(nativeModule, /dependency cycle/);
  assert.match(nativeModule, /scope\.spawn\(\|\| analyze_out_of_sample/);
  assert.match(nativeModule, /scope\.spawn\(\|\| block_bootstrap/);
  for (const role of ['Data Steward', 'Quant Researcher', 'Independent Robustness Reviewer', 'Risk Officer', 'Investment Committee']) {
    assert.ok(nativeModule.includes(role), `Missing deterministic role: ${role}`);
  }
  assert.match(nativeModule, /eligible_for_paper_validation/);
  assert.match(nativeModule, /no execution authority is granted/i);
});

test('Tauri exposes guarded native commands and the UI consumes returned records rather than local fixtures', () => {
  for (const command of [
    'create_research_hypothesis', 'list_research_hypotheses', 'invalidate_research_hypothesis',
    'execute_research_workflow', 'list_research_run_cards', 'get_research_run_card', 'verify_research_workspace',
  ]) {
    assert.match(nativeCommands, new RegExp(`\\b${command}\\b`));
  }
  assert.match(bridge, /requireDesktopRuntime\('research workspace'\)/);
  assert.match(bridge, /invoke<ResearchRunCardRecord>\('execute_research_workflow'/);
  assert.match(runtimeGuard, /'research workspace'/);
  assert.match(app, /label: 'Research Lab'/);
  assert.match(app, /await executeResearchWorkflow\(/);
  assert.match(app, /await getCandleSeries\(requested\)/);
  assert.match(app, /validateCandleSeriesResponse\(rawSeries, requested\)/);
  assert.match(app, /verifyResearchWorkspace\(\)/);
  assert.match(app, /record\.card\.decision\.status/);
  assert.match(nativeModule, /maximum_drawdown_pct: request\.max_drawdown_pct/);
  assert.match(nativeModule, /minimum_positive_oos_ratio: request\.minimum_positive_oos_ratio/);
  assert.match(nativeModule, /minimum_sharpe_ratio: request\.minimum_sharpe_ratio/);
  assert.doesNotMatch(app, /const\s+researchRunCards\s*=\s*\[/);
});

test('frontend contract includes native research run-card, statistical and provenance records', () => {
  for (const contract of [
    'ResearchHypothesis', 'ResearchWorkflowRequest', 'ResearchWorkflowNode',
    'ResearchOutOfSampleReview', 'ResearchBlockBootstrapSummary', 'ResearchSpecialistReview',
    'ResearchBacktestEvidence', 'ResearchGateDecision', 'ResearchRunCardRecord',
    'ResearchIntegrityStatus', 'ResearchGateParameters', 'gateParameters', 'contentSha256', 'datasetSha256',
  ]) assert.ok(types.includes(contract), `Missing TypeScript contract ${contract}`);
});

test('hypothesis lifecycle is revision-checked and invalidated selections are rejected', () => {
  assert.match(app, /invalidateResearchHypothesis\(selected\.id, reason, selected\.revision\)/);
  assert.match(app, /hypothesis\.id === selectedResearchHypothesisId && hypothesis\.status !== 'INVALIDATED'/);
  assert.match(app, /Reason for invalidation/);
  assert.match(app, /if \(researchHypothesisCreateBusy\) return/);
  assert.match(app, /researchIntegrity\?\.runCardCount \?\? researchRunCards\.length/);
  assert.match(bridge, /invoke<ResearchHypothesis>\('invalidate_research_hypothesis'/);
});


test('nested parameter tournament is training-only, bounded, native, and represented in the dossier', () => {
  assert.match(backtest, /pub fn run_walk_forward_parameter_tournament/);
  assert.match(backtest, /TOURNAMENT_MAX_CANDIDATES: usize = 25/);
  assert.match(backtest, /TOURNAMENT_MAX_FOLDS: usize = 5/);
  assert.match(backtest, /TOURNAMENT_WORK_BUDGET: u64 = 50_000_000/);
  assert.match(backtest, /compare_tournament_candidate_priority/);
  assert.match(backtest, /tournament_priority_uses_one_exact_order_for_selection_and_verification/);
  assert.match(backtest, /training_result = run_once/);
  assert.match(backtest, /out_of_sample = run_once/);
  assert.match(backtest, /evaluation_start_index|train_end/);
  assert.match(backtest, /nested_walk_forward_selection_does_not_peek_into_its_first_test_block/);
  assert.match(engine, /pub fn run_research_backtest_bundle/);
  assert.match(engine, /one.*immutable candle\/provider snapshot/s);
  assert.match(engine, /let \(candles, source\)/);
  assert.match(engine, /run_walk_forward_parameter_tournament/);
  assert.match(nativeModule, /run_research_backtest_bundle/);
  assert.match(engine, /validate_candle_series\(&candles, CandleInterval::FiveMinutes, 150\)/);
  assert.match(nativeModule, /validate_strategy_tournament/);
  assert.match(nativeModule, /deterministic training-only winner/);
  assert.match(nativeModule, /compare_tournament_candidate_priority/);
  assert.match(nativeModule, /expanding windows/);
  assert.match(nativeModule, /strategy_tournament: bool/);
  assert.match(nativeModule, /nested_parameter_tournament/);
  assert.match(nativeModule, /atos-research-gate-v2-nested-walk-forward/);
  assert.match(types, /interface StrategyParameterTournament/);
  assert.match(types, /strategyTournament\?: StrategyParameterTournament/);
  assert.match(app, /Run nested walk-forward parameter tournament/);
  assert.match(app, /Training candidate scores/);
  assert.match(app, /strategyTournament: researchStrategyTournament/);
});
