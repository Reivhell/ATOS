#!/usr/bin/env node
/**
 * Dependency-free repository guardrails. This is a preflight check, not a substitute
 * for `svelte-check`, a Rust compiler, the unit tests, or desktop acceptance testing.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const root = process.cwd();
const errors = [];
const warnings = [];

function read(relativePath) {
  const absolutePath = path.join(root, relativePath);
  try {
    return fs.readFileSync(absolutePath, 'utf8');
  } catch {
    errors.push(`Required file is missing or unreadable: ${relativePath}`);
    return '';
  }
}

function parseJson(relativePath) {
  const content = read(relativePath);
  if (!content) return null;
  try {
    return JSON.parse(content);
  } catch (error) {
    errors.push(`Invalid JSON in ${relativePath}: ${error.message}`);
    return null;
  }
}

function requireText(content, snippet, label) {
  if (snippet instanceof RegExp) {
    if (!snippet.test(content)) errors.push(`${label}: expected to match ${snippet}`);
  } else if (!content.includes(snippet)) errors.push(`${label}: expected to find ${JSON.stringify(snippet)}`);
}

function forbidText(content, snippet, label) {
  if (content.includes(snippet)) errors.push(`${label}: forbidden content ${JSON.stringify(snippet)} found`);
}

const requiredFiles = [
  'README.md', 'SECURITY.md', 'CHANGELOG.md', 'rust-toolchain.toml', 'scripts/check-release-readiness.mjs', 'docs/MARKET_DATA_SNAPSHOT_ISOLATION_0.5.2.md', 'docs/VALIDATION_REPORT_0.5.2.md', 'docs/IMPLEMENTATION_STATUS.md', 'docs/VALIDATION_REPORT_0.1.9.md', 'docs/VALIDATION_REPORT_0.1.8.md', 'docs/VALIDATION_REPORT_0.1.7.md', 'docs/14_TECHNICAL_ANALYSIS_MASTER_LIST_CONTRACT.md', 'docs/VALIDATION_REPORT_0.1.6.md', 'docs/VALIDATION_REPORT_0.1.5.md', 'docs/VALIDATION_REPORT_0.1.4.md', 'docs/VALIDATION_REPORT_0.1.3.md', 'docs/VALIDATION_REPORT_0.1.2.md',
  'docs/VALIDATION_REPORT_0.7.0.md', 'docs/LOCAL_API_REQUEST_BOUNDARY_0.7.0.md', 'api/src/request-origin-policy.ts', 'scripts/test-request-context-policy.test.mjs', 'docs/VALIDATION_REPORT_0.6.0.md', 'docs/MARKET_DATA_ASSURANCE_0.6.0.md', 'docs/ARCHITECTURE_LAYERS_0.6.0.md', 'docs/VALIDATION_REPORT_0.5.8.md', 'docs/VALIDATION_REPORT_0.5.9.md', 'docs/BUN_HONO_LOCAL_RESEARCH_API_0.5.9.md', 'docs/ENGINEERING_UPDATE_2026-10-28.md', 'docs/ARCHITECTURE_LAYERS_0.5.9.md', 'api/src/app.ts', 'api/src/server.ts', 'api/src/validation.ts', 'api/src/rate-limit.ts', 'api/src/request-rate-policy.ts', 'api/test/app.integration.test.ts', 'scripts/test-request-rate-policy.test.mjs', 'scripts/test-local-research-api.test.mjs', 'scripts/test-rate-limit.test.mjs', 'src/App.svelte', 'src/lib/types.ts', 'src/lib/domain/market-data/timeframe-contract.js', 'src/lib/domain/market-data/candle-series-audit.js', 'scripts/test-candle-series-audit.test.mjs', 'scripts/test-market-data-model.test.mjs', 'scripts/test-exports.test.mjs', 'src-tauri/Cargo.toml',
  'src-tauri/src/runtime/trading_core.rs', 'src-tauri/src/application/backtest.rs', 'src-tauri/src/application/strict_analysis.rs', 'src-tauri/src/application/strict_analysis_service.rs', 'src-tauri/src/application/market_data_query.rs', 'src-tauri/src/domain/mod.rs', 'src-tauri/src/domain/technical_analysis.rs', 'src-tauri/src/application/mod.rs', 'src-tauri/src/infrastructure/mod.rs', 'src-tauri/src/infrastructure/providers/public_market.rs', 'src-tauri/src/adapters/tauri/commands.rs', 'src-tauri/src/adapters/tauri/bootstrap.rs', 'src-tauri/src/lib.rs', 'scripts/test-candle-continuity-contract.test.mjs', 'scripts/test-rust-technical-indicators-contract.test.mjs', 'scripts/report-rust-first-ratio.mjs', 'docs/CANDLE_CONTINUITY_0.5.6.md', 'docs/VALIDATION_REPORT_0.5.6.md', 'docs/BACKTEST_ROBUSTNESS_0.5.7.md', 'docs/VALIDATION_REPORT_0.5.7.md', 'scripts/test-backtest-robustness.test.mjs', 'scripts/test-dataset-fingerprint.test.mjs', 'src/lib/technical-analysis.js', 'scripts/test-technical-analysis.test.mjs', 'scripts/fixtures/technical-analysis-master-list.v1.json',
  'scripts/release-evidence.mjs', 'scripts/test-release-evidence.test.mjs', '.github/workflows/ci.yml', 'scripts/test-decision-gate.test.mjs', 'scripts/test-agent-knowledge-packs.test.mjs',
  'docs/STRICT_DECISION_LAYER_POLICY.json', 'docs/MARKET_STRUCTURE_BATCH_SEMANTICS_0.4.3.md', 'docs/VALIDATION_REPORT_0.4.3.md', 'docs/STRICT_AUDIT_CHAIN_0.4.2.md', 'docs/STRICT_SPECIALIST_DISPATCH_RECEIPTS_0.4.2.md', 'docs/VALIDATION_REPORT_0.4.2.md', 'scripts/test-rust-audit-chain-contract.test.mjs', 'docs/agents/00-decision-orchestrator.md',
  'docs/agents/01-smc-ict-agent.md', 'docs/agents/02-price-action-agent.md',
  'docs/agents/03-support-resistance-trend-agent.md', 'docs/agents/04-technical-indicators-agent.md',
  'docs/agents/05-order-flow-market-profile-agent.md', 'docs/agents/06-divergence-intermarket-agent.md',
  'docs/agents/07-other-methods-agent.md', 'docs/agents/08-fundamental-macro-agent.md',
  'docs/agents/09-risk-management-evaluation-agent.md', 'docs/agents/strict-ta-assessment.schema.json', 'docs/agents/strict-ta-work-plan.schema.json', 'scripts/test-strict-analysis-orchestrator.test.mjs', 'scripts/test-rust-strict-contract.test.mjs',
];
for (const item of requiredFiles) read(item);
const workflow = read('.github/workflows/ci.yml');
requireText(workflow, 'npm run typecheck:layers', 'Frontend CI typecheck gate');
requireText(workflow, 'bun test api/test/app.integration.test.ts', 'Bun/Hono HTTP integration CI gate');

const packageJson = parseJson('package.json');
const cargoManifest = read('src-tauri/Cargo.toml');
const tauriConfig = parseJson('src-tauri/tauri.conf.json');
const masterListManifest = parseJson('scripts/fixtures/technical-analysis-master-list.v1.json');
const capabilityConfig = parseJson('src-tauri/capabilities/default.json');
parseJson('tsconfig.json');
read('vite.config.ts');

if (masterListManifest) {
  if (masterListManifest.masterListVersion !== '1.0.0') errors.push('Technical-analysis master-list manifest version must remain 1.0.0 unless deliberately reviewed.');
  if (masterListManifest.expectedEntryCount !== 364 || masterListManifest.entries?.length !== 364) errors.push('Technical-analysis master-list manifest must contain exactly 364 contextual entries.');
  if (masterListManifest.categories?.length !== 9) errors.push('Technical-analysis master-list manifest must contain exactly nine top-level categories.');
}

if (packageJson) {
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'svelte-check', 'Frontend build script');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'vite build', 'Frontend build script');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'typecheck:layers', 'Strict analysis-layer typecheck script');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'verify-repo.mjs', 'Repository guardrail script');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-exports.test.mjs', 'Dependency-free export tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-dataset-fingerprint.test.mjs', 'Dependency-free provenance contract tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-technical-analysis.test.mjs', 'Dependency-free technical analysis tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-dashboard-model.test.mjs', 'Dependency-free dashboard view-model tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-release-evidence.test.mjs', 'Release artifact provenance and integrity tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node scripts/release-evidence.mjs create', 'Release evidence generator command');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node scripts/release-evidence.mjs verify', 'Release evidence verification command');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-market-data-model.test.mjs', 'Dependency-free typed market-data contract tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-candle-series-audit.test.mjs', 'Bounded candle timeline and latest-bar anomaly audit tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-candle-continuity-contract.test.mjs', 'Native candle-continuity source contract tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-backtest-robustness.test.mjs', 'Backtest stress and out-of-sample source contract tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'bun test scripts/*.test.mjs', 'Bun runtime test suite');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'bun --hot run api/src/server.ts', 'Bun/Hono local research API development command');
  requireText(JSON.stringify(packageJson.dependencies ?? {}), 'hono', 'Hono HTTP adapter dependency');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'test-local-research-api.test.mjs', 'Local research API security/validation tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'test-rate-limit.test.mjs', 'Sliding-window rate limiter regression tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'test-request-rate-policy.test.mjs', 'Separate authenticated and failed-authentication quota tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'test-request-context-policy.test.mjs', 'Loopback Host/origin request-context policy regression tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-decision-gate.test.mjs', 'Strict decision gate tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-strict-analysis-orchestrator.test.mjs', 'Strict orchestrator completion-barrier tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-technical-analysis-service.test.mjs', 'Technical-analysis application boundary tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-analysis-engine.test.mjs', 'Layered technical-analysis engine tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'test-rust-technical-indicators-contract.test.mjs', 'Rust technical indicator migration contract tests');
  requireText(JSON.stringify(packageJson.scripts ?? {}), 'node --test scripts/test-research-decision-engine.test.mjs', 'Fail-closed research decision tests');
}

if (packageJson && tauriConfig) {
  if (packageJson.version !== tauriConfig.version) errors.push('package.json and Tauri application versions differ.');
  const cargoPackage = cargoManifest.match(/\[package\][\s\S]*?^version = "([^"]+)"/m);
  if (!cargoPackage || cargoPackage[1] !== packageJson.version) errors.push('package.json and Cargo package versions differ.');
}
requireText(cargoManifest, 'features = ["json", "rustls", "stream", "query"]', 'Bounded async HTTP response support');
requireText(cargoManifest, 'rust-version = "1.82"', 'Minimum Rust compiler version');
requireText(cargoManifest, 'sha2 = "0.11"', 'Stable SHA-256 dataset fingerprint implementation');

if (tauriConfig) {
  const iconPaths = tauriConfig.bundle?.icon ?? [];
  if (iconPaths.length === 0) errors.push('Tauri bundle must declare platform icon assets.');
  for (const iconPath of iconPaths) {
    const assetPath = path.join(root, 'src-tauri', iconPath);
    if (!fs.existsSync(assetPath)) {
      errors.push(`Configured Tauri icon is missing: src-tauri/${iconPath}`);
      continue;
    }
    const bytes = fs.readFileSync(assetPath);
    const extension = path.extname(iconPath).toLowerCase();
    if (extension === '.png' && !(bytes[0] === 0x89 && bytes.toString('ascii', 1, 4) === 'PNG')) {
      errors.push(`Configured PNG icon has an invalid signature: ${iconPath}`);
    }
    if (extension === '.ico' && !(bytes[0] === 0 && bytes[1] === 0 && bytes[2] === 1 && bytes[3] === 0)) {
      errors.push(`Configured ICO icon has an invalid signature: ${iconPath}`);
    }
    if (extension === '.icns' && bytes.toString('ascii', 0, 4) !== 'icns') {
      errors.push(`Configured ICNS icon has an invalid signature: ${iconPath}`);
    }
  }
  const devUrl = tauriConfig.build?.devUrl ?? '';
  if (!devUrl.startsWith('http://127.0.0.1:')) errors.push('Tauri dev server must bind to loopback, not a public interface.');
  const csp = tauriConfig.app?.security?.csp ?? '';
  requireText(csp, "default-src 'self'", 'Tauri content security policy');
  forbidText(csp, "'unsafe-eval'", 'Tauri content security policy');
  if (tauriConfig.bundle?.createUpdaterArtifacts === true) {
    warnings.push('Updater artifacts are enabled; release signing and update-channel validation must be completed.');
  }
}

if (capabilityConfig) {
  const permissions = capabilityConfig.permissions ?? [];
  if (permissions.includes('shell:allow-execute') || permissions.includes('shell:allow-spawn')) {
    errors.push('Default Tauri capability grants shell execution; the baseline must remain least-privilege.');
  }
}

const engine = read('src-tauri/src/runtime/trading_core.rs');
for (const check of [
  'Kill switch', 'Market-data provenance', 'Market-data freshness', 'Order notional',
  'Cash / inventory', 'Position cap', 'Daily loss guard', 'client_order_id',
]) requireText(engine, check, 'Rust risk gateway');
requireText(engine, 'Operation would produce an invalid engine state', 'State validation rollback');
requireText(engine, 'Cannot produce dashboard snapshot because state validation failed', 'Snapshot rollback');
requireText(engine, 'StateLoadError::UnsupportedSchema(version)', 'Future-schema fail-closed behavior');
requireText(engine, 'newer than this application supports. The database was left untouched', 'Future-schema data preservation');
requireText(engine, 'pub fn open(persistence_path: PathBuf) -> Result<Self, String>', 'Fallible persistent startup');
requireText(engine, 'Cannot preserve corrupt local database file', 'Corrupt-state backup failure handling');
requireText(engine, 'corrupt_database_and_sqlite_sidecars_are_preserved_together', 'SQLite sidecar backup regression test');
requireText(engine, 'orphaned_sqlite_sidecars_are_preserved_before_new_database_creation', 'Orphaned SQLite sidecar regression test');
requireText(engine, 'Orphaned SQLite WAL/SHM sidecar exists without its main database', 'Orphaned sidecar recovery');
requireText(engine, 'future_schema_database_is_rejected_without_mutation_or_backup', 'Future database-schema regression test');
requireText(engine, 'future_embedded_state_schema_is_rejected_without_mutation', 'Future payload-schema regression test');
requireText(engine, 'if state.schema_version > 1', 'Future payload-schema preservation');
  requireText(engine, /payload_value\s*\.\s*get\s*\(\s*"schemaVersion"\s*\)/, 'Future payload-schema detection before deserialization');
requireText(engine, 'startup_fails_when_durable_storage_cannot_be_created', 'Startup persistence regression test');
requireText(engine, 'startup_preserves_state_if_saved_quotes_make_marked_equity_overflow', 'Startup state-marking overflow regression test');
requireText(engine, 'Current quote marking would produce an invalid account state', 'Pre-mutation marking invariant');
requireText(engine, 'resetting_paper_account_does_not_release_kill_switch', 'Kill-switch reset regression test');
requireText(engine, 'fn acquire_instance_lock(path: &Path)', 'Process-lifetime exclusive account lock');
requireText(engine, 'BEGIN EXCLUSIVE;', 'Exclusive SQLite instance-lock transaction');
requireText(engine, 'second_engine_instance_is_rejected_and_lock_is_released_on_close', 'Single-instance lock regression test');

const nativeIndicators = read('src-tauri/src/domain/technical_analysis.rs');
requireText(nativeIndicators, 'pub fn calculate_readings(candles: &[Candle])', 'Rust-native technical indicator engine');
requireText(nativeIndicators, 'const MAX_CANDLES: usize = 10_000', 'Native indicator input bound');
requireText(nativeIndicators, 'malformed_ohlc_fails_closed_for_every_indicator', 'Native indicator malformed-data regression test');
requireText(nativeIndicators, 'insufficient_history_is_reported_without_fabricated_values', 'Native indicator insufficient-history regression test');
requireText(read('src-tauri/src/adapters/tauri/commands.rs'), 'calculate_technical_indicator_readings', 'Native indicator Tauri command');
requireText(read('src-tauri/src/adapters/tauri/bootstrap.rs'), 'commands::calculate_technical_indicator_readings', 'Native indicator command registration');
requireText(read('src/lib/adapters/tauri/bridge.ts'), "invoke<TechnicalIndicatorReading[]>('calculate_technical_indicator_readings'", 'Typed native indicator bridge');
requireText(nativeIndicators, 'pub fn calculate_structure(candles: &[Candle])', 'Rust-native market structure engine');
requireText(nativeIndicators, 'detect_confirmed_structure_breaks(candles, 2, 2)', 'Causal Rust structure break detector');
requireText(read('src-tauri/src/adapters/tauri/commands.rs'), 'calculate_native_market_structure', 'Native market structure Tauri command');
requireText(read('src-tauri/src/adapters/tauri/bootstrap.rs'), 'commands::calculate_native_market_structure', 'Native market structure command registration');
requireText(read('src/lib/adapters/tauri/bridge.ts'), "invoke<NativeStructureSnapshot>('calculate_native_market_structure'", 'Typed native market structure bridge');
requireText(read('src/App.svelte'), 'calculateNativeMarketStructure(series.candles)', 'UI market structure uses Rust results');
requireText(read('src/App.svelte'), 'nativeStructure.bias', 'UI structure bias is sourced from Rust');
requireText(nativeIndicators, 'pub fn calculate_signal_snapshot(candles: &[Candle])', 'Rust-native indicator signal generation');
requireText(read('src-tauri/src/adapters/tauri/commands.rs'), 'calculate_native_indicator_signals', 'Native indicator signal Tauri command');
requireText(read('src-tauri/src/adapters/tauri/bootstrap.rs'), 'commands::calculate_native_indicator_signals', 'Native indicator signal command registration');
requireText(read('src/lib/adapters/tauri/bridge.ts'), "invoke<NativeIndicatorSignalSnapshot>('calculate_native_indicator_signals'", 'Typed native indicator signal bridge');
requireText(read('src/App.svelte'), 'calculateNativeIndicatorSignals(series.candles)', 'UI requests Rust-native indicator signals');
requireText(read('src/App.svelte'), 'Rust indicator signals:', 'UI labels native indicator signals separately from the decision engine');
requireText(read('src/App.svelte'), 'calculateTechnicalIndicatorReadings(series.candles)', 'UI indicator cards use Rust results');
forbidText(read('src/App.svelte'), 'technicalReadings = technicalEnvelope.indicatorReadings', 'Frontend indicator calculation must not be the UI source of truth');

const backtest = read('src-tauri/src/application/backtest.rs');
requireText(backtest, 'for (i, candle) in candles.iter().enumerate().skip(evaluation_start_index)', 'Backtest chronological iterator preserves execution-bar identity');
requireText(backtest, 'let price = candle.close;', 'Backtest signal observation uses the current completed bar close');
requireText(backtest, 'candle.open * (1.0 + slip_rate)', 'Backtest next-bar-open buy execution');
requireText(backtest, 'candle.open * (1.0 - slip_rate)', 'Backtest next-bar-open sell execution');
requireText(backtest, 'execution_uses_next_bar_open_not_the_same_bar_close', 'Backtest regression test');
requireText(backtest, 'sharpe_annualization_tracks_observed_candle_interval', 'Backtest cadence regression test');
requireText(backtest, 'crossover_strategy_requires_an_observed_cross_up', 'True crossover regression test');
requireText(backtest, 'backtest_rejects_overflowing_prefix_sums', 'Numeric overflow regression test');
requireText(backtest, 'Cumulative close-price sum exceeds the supported numeric range', 'Backtest numeric overflow guard');
requireText(backtest, 'agentic-trading-os.ohlcv.canonical.v1', 'Canonical backtest dataset hash encoding');
requireText(backtest, 'to_le_bytes()', 'Endian-stable backtest dataset hash encoding');
requireText(backtest, 'canonical_dataset_sha256_matches_known_vector', 'Dataset SHA-256 known-vector regression test');
requireText(backtest, 'f2372778376160ebe9f4759a6a81dc8e0b76020e21dcc8dafe48d1f76134dec6', 'Backtest known SHA-256 test vector');
requireText(backtest, 'dataset_sha256_is_deterministic_and_changes_for_any_candle_field', 'Dataset SHA-256 mutation regression test');
requireText(backtest, 'backtest_result_exposes_input_range_and_fingerprint_metadata', 'Backtest provenance metadata regression test');
requireText(backtest, 'backtest_reports_cost_stress_scenarios_with_applied_costs_and_deltas', 'Backtest cost sensitivity regression test');
requireText(backtest, 'backtest_builds_fixed_chronological_out_of_sample_windows', 'Backtest out-of-sample window regression test');
requireText(backtest, 'earlier_out_of_sample_fold_is_invariant_to_future_candle_values', 'Backtest future-invariance regression test');
requireText(backtest, 'run_once(symbol, window, request.clone(), source, warmup_bars)', 'Out-of-sample indicators use warm-up without trading');
for (const field of ['dataset_sha256', 'dataset_encoding_version', 'dataset_start_ms', 'dataset_end_ms', 'median_candle_interval_ms']) {
  requireText(backtest, field, 'Backtest reproducibility metadata');
}
requireText(cargoManifest, 'sha2 = "0.11"', 'Canonical data fingerprint dependency');

const rustCommands = [read('src-tauri/src/adapters/tauri/commands.rs'), read('src-tauri/src/adapters/tauri/bootstrap.rs'), read('src-tauri/src/infrastructure/providers/public_market.rs'), read('src-tauri/src/application/strict_analysis_service.rs')].join('\n');
  requireText(rustCommands, /claim_refresh_slot\s*\(\s*&LAST_MARKET_REFRESH_MS/, 'Market-data refresh throttling');
  requireText(rustCommands, /claim_refresh_slot\s*\(\s*&LAST_SCREENER_REFRESH_MS/, 'Screener refresh throttling');
requireText(rustCommands, 'response.bytes_stream()', 'Bounded provider response reading');
requireText(rustCommands, 'parse_closed_binance_candles(rows, Utc::now().timestamp_millis())', 'Closed-candle ingestion');
requireText(rustCommands, 'close_time >= now_ms', 'In-progress candle exclusion');
requireText(rustCommands, 'binance_backtest_data_excludes_the_still_forming_candle', 'Closed-candle regression test');
requireText(rustCommands, 'validate_candle_series(&converted, CandleInterval::FiveMinutes, 30)', 'Native closed-candle continuity gate');
requireText(rustCommands, 'binance_parser_rejects_internal_gap_instead_of_returning_partial_candles', 'Provider candle-gap regression test');
requireText(rustCommands, 'binance_parser_rejects_duplicate_and_out_of_order_rows_instead_of_normalizing_them', 'Provider timestamp-order regression test');
requireText(rustCommands, 'TradingCore::open(app_data_dir.join("agentic-trading-os.sqlite3"))', 'Fail-closed app startup');
requireText(rustCommands, 'provider_body_limit_accepts_boundary_and_rejects_overflow_atomically', 'Provider response-size regression test');
requireText(rustCommands, 'https://api.binance.com/api/v3/ticker/24hr', 'Public market data provider');
requireText(rustCommands, 'https://api.dexscreener.com/token-boosts/latest/v1', 'Public screener provider');
requireText(rustCommands, `AgenticTradingOS/${packageJson?.version ?? '0.1.9'}`, 'Current versioned public-provider user agent');
requireText(rustCommands, 'get_candle_series', 'Exact typed candle-series command');
for (const endpoint of [
  'https://api.binance.com/api/v3/order',
  'https://api.binance.com/api/v3/account',
  'https://api.binance.com/sapi/v1/capital/withdraw',
]) forbidText(rustCommands, endpoint, 'Live/private Binance endpoint boundary');

const exportsModule = read('src/lib/infrastructure/serialization/exports.js');
requireText(exportsModule, 'neutralize leading spreadsheet formula markers', 'CSV spreadsheet-injection protection');
requireText(exportsModule, 'agentic-trading-os.backtest-report', 'Versioned backtest export envelope');
requireText(exportsModule, 'timestamp_utc', 'Audit CSV UTC timestamp column');
requireText(exportsModule, 'formatVersion: 2', 'Backtest report export schema v2');
const appTypes = read('src/lib/types.ts');
requireText(appTypes, 'datasetSha256: string', 'Frontend backtest fingerprint contract');
requireText(appTypes, 'datasetStartMs: number', 'Frontend backtest input-range contract');
requireText(appTypes, 'costStressScenarios: BacktestCostStressScenario[]', 'Frontend cost stress result contract');
requireText(appTypes, 'outOfSampleFolds: BacktestOutOfSampleFold[]', 'Frontend out-of-sample result contract');
requireText(read('src/App.svelte'), 'Dataset fingerprint and input range', 'Visible backtest provenance details');
requireText(read('src/App.svelte'), 'Execution-cost sensitivity', 'Visible execution-cost sensitivity panel');
requireText(read('src/App.svelte'), 'Out-of-sample stability', 'Visible out-of-sample evaluation panel');
requireText(read('src/App.svelte'), 'isoDateTimeLabel(backtestResult.datasetStartMs)', 'Safe dataset timestamp rendering');
requireText(read('src/App.svelte'), 'canonical input-data fingerprint', 'Accurate v2 export UI confirmation');
const exportTests = read('scripts/test-exports.test.mjs');
requireText(exportTests, 'neutralizes spreadsheet formula injection', 'CSV serializer regression test');
requireText(exportTests, 'correctly escapes commas, quotes, and embedded newlines', 'CSV serializer escaping regression test');
requireText(exportTests, 'preserves the complete supplied result', 'Backtest JSON export regression test');

const apiSource = read('api/src/app.ts');
const apiServer = read('api/src/server.ts');
const apiValidation = read('api/src/validation.ts');
const apiOriginPolicy = read('api/src/request-origin-policy.ts');
requireText(apiSource, 'A LOCAL_RESEARCH_API_TOKEN of at least 32 characters is required.', 'Hono API secret requirement');
requireText(apiSource, 'RATE_MAX_REQUESTS = 30', 'Hono API rate limiting');
requireText(apiSource, 'FAILED_AUTH_MAX_REQUESTS = 10', 'Separate failed-authentication rate limit');
requireText(apiSource, 'consumeAuthenticatedRequest()', 'Authenticated request budget is checked only after credential validation');
requireText(apiSource, 'consumeFailedAuthentication()', 'Failed authentication cannot consume the valid-client request budget');
requireText(apiSource, 'researchCandidateHandoffAllowed: false', 'Research API fail-closed decision handoff');
requireText(apiSource, 'validateRequestContext', 'Research API Host/origin request-context boundary');
requireText(apiSource, 'Request host or browser origin is not allowed.', 'Research API safe context rejection');
requireText(apiOriginPolicy, 'cross_site_browser_request', 'Cross-site browser request rejection');
requireText(apiOriginPolicy, '127\\.0\\.0\\.1', 'Canonical loopback Host validation');
requireText(apiOriginPolicy, 'browser_origin_not_supported', 'Explicit no-CORS browser Origin rejection');
requireText(apiServer, "host !== '127.0.0.1'", 'Hono API loopback-only bind');
requireText(apiServer, 'expectedPort: port', 'Research API validates Host port against its listener');
requireText(apiValidation, 'MAX_API_CANDLES = 2_000', 'Research API bounded candle input');
requireText(read('api/test/app.integration.test.ts'), 'researchCandidateHandoffAllowed).toBe(false)', 'Bun/Hono integration test asserts no trusted handoff');
forbidText(apiSource, '/v1/orders', 'Research API order-route boundary');
forbidText(apiSource, '/v1/execute', 'Research API execution-route boundary');

const layerContract = read('docs/ARCHITECTURE_LAYERS_0.5.8.md');
requireText(layerContract, 'domain → engines → application → adapters', 'Layering direction contract');
for (const layerPath of [
  'src/lib/domain/decision/contracts.js', 'src/lib/domain/decision/research-policy.js',
  'src/lib/domain/decision/strict-catalog-registry.js', 'src/lib/domain/market-data/market-data-model.js',
  'src/lib/domain/technical-analysis/catalog.js', 'src/lib/engines/technical-analysis/indicators.js',
  'src/lib/engines/technical-analysis/research-detectors.js', 'src/lib/engines/technical-analysis/analysis-engine.js',
  'src/lib/engines/decision/research-decision-engine.js', 'src/lib/engines/decision/strict-decision-gate.js',
  'src/lib/application/strict-analysis-orchestrator.js', 'src/lib/application/technical-analysis-service.js', 'scripts/test-technical-analysis-service.test.mjs', 'src/lib/adapters/tauri/bridge.ts',
  'src/lib/presentation/dashboard/dashboard-model.js', 'src/lib/infrastructure/serialization/exports.js',
]) read(layerPath);

const app = read('src/App.svelte');
requireText(app, 'function downloadTextFile', 'Local export download path');
requireText(app, 'Export CSV', 'Audit CSV export UI');
requireText(app, 'Export JSON', 'Backtest JSON export UI');
requireText(app, 'Installed desktop runtime required', 'Native-only application entrypoint');
forbidText(app, 'Browser preview', 'No browser preview UI or synthetic-data path');
forbidText(app, 'Research fixture data only', 'No demo screener warning/data mode');
forbidText(app, 'class=\"mini-spark', 'No static fabricated portfolio curves');
forbidText(app, 'money(quoteFor(activeCandleRequest.symbol)?.price ?? 0)', 'Missing market price is never rendered as zero');
requireText(app, 'closedTradeCount > 0', 'Win rate denominator and unavailable empty statistic');
requireText(app, 'No agent sessions configured', 'Empty agent registry is not described as running');
requireText(app, 'Technical Analysis', 'Technical analysis workspace navigation');
requireText(app, 'Available indicator readings', 'Indicator readings panel');
requireText(app, 'chart-order-mismatch', 'Explicit order/chart symbol mismatch warning');
requireText(app, 'Candle-series capabilities', 'Visible market-data capability panel');
requireText(app, 'const validatedResponse = validateCandleSeriesResponse(response, typedRequest)', 'UI consumes immutable validated candle snapshot');
requireText(app, 'createTechnicalAnalysisReport({', 'UI uses application service rather than calling analysis engine directly');
forbidText(app, './lib/engines/technical-analysis/analysis-engine.js', 'Svelte-to-engine boundary');
requireText(read('src/lib/domain/market-data/market-data-model.js'), 'time-of-check/time-of-use gap', 'Market-data snapshot isolation guard');
requireText(read('src/lib/domain/market-data/market-data-model.js'), 'candle.time % expectedIntervalMs !== 0', 'UTC interval-grid validation at the frontend boundary');
requireText(app, 'Catalog only', 'Explicit catalog-only coverage status');
requireText(app, '<option>Calculated now</option>', 'Separate calculated indicator status filter');
requireText(app, '<option>Workflow available</option>', 'Separate workflow status filter');
requireText(app, 'analysisStatusCounts.catalogOnly', 'Visible strict implementation-status counts');
const technicalAnalysis = read('src/lib/domain/technical-analysis/catalog.js') + read('src/lib/engines/technical-analysis/indicators.js');
requireText(technicalAnalysis, 'BOS (Break of Structure)', 'SMC/ICT catalog coverage');
requireText(technicalAnalysis, "TECHNICAL_ANALYSIS_EXPECTED_ENTRY_COUNT = 364", 'Strict master-list entry count');
requireText(technicalAnalysis, "'Indikator Teknikal': 52", 'Exact master-list category label and count');
requireText(technicalAnalysis, "'Metode Analisis Lain': 42", 'Exact master-list category label and count');
requireText(technicalAnalysis, "'Fundamental & Makro': 20", 'Exact master-list category label and count');
requireText(technicalAnalysis, 'FOMC (Federal Open Market Committee)', 'Macro catalog coverage');
requireText(technicalAnalysis, 'Monte Carlo Simulation', 'Risk evaluation catalog coverage');
requireText(technicalAnalysis, 'calculateIndicatorReadings', 'Deterministic indicator calculator');
requireText(technicalAnalysis, 'Wilder-smoothed true range', 'ATR calculation disclosure');
const dashboardModel = read('src/lib/presentation/dashboard/dashboard-model.js');
requireText(dashboardModel, 'summarizeLastCandle', 'Validated Home dashboard OHLC model');
requireText(dashboardModel, 'normalizedVolumeHeight', 'Normalized OHLCV histogram');
const dashboardTests = read('scripts/test-dashboard-model.test.mjs');
requireText(dashboardTests, 'rejects malformed OHLC relationships', 'Dashboard OHLC fail-closed regression tests');
requireText(dashboardTests, 'volume bars normalize', 'Dashboard volume histogram regression tests');
const technicalTests = read('scripts/test-technical-analysis.test.mjs');
requireText(technicalTests, 'malformed candles fail closed', 'Malformed OHLCV regression test');
requireText(technicalTests, 'strict catalog exactly matches the reviewed master-list inventory snapshot', 'Exhaustive catalog manifest regression test');
requireText(technicalTests, 'strict catalog preserves all nine requested top-level category names and exact counts', 'Strict top-level category/count regression test');
requireText(read('docs/14_TECHNICAL_ANALYSIS_MASTER_LIST_CONTRACT.md'), '364 contextual entries', 'Strict technical-analysis contract documentation');
requireText(technicalTests, 'insufficient history never fabricates', 'Insufficient-data regression test');
const strictOrchestrator = read('src/lib/application/strict-analysis-orchestrator.js');
requireText(strictOrchestrator, 'createStrictAnalysisWorkPlan', 'Full-list specialist work planner');
requireText(strictOrchestrator, 'evaluateStrictAnalysisCompletion', 'Strict downstream completion barrier');
requireText(strictOrchestrator, 'nativeRustGateRequired', 'JavaScript diagnostics require native Rust handoff authority');
requireText(read('docs/agents/strict-ta-assessment.schema.json'), 'strict-ta-assessment.v1', 'Strict specialist assessment schema');
const decisionGate = read('src/lib/engines/decision/strict-decision-gate.js');
requireText(decisionGate, 'code-owned registry is authoritative', 'Trusted strict catalog registry');
requireText(decisionGate, 'must remain unsupported until reviewed promotion', 'Strict catalog-only self-promotion prevention');
requireText(decisionGate, 'Market data has not passed the validated-data contract', 'Decision gate market-data prerequisite');
requireText(decisionGate, 'Risk gateway did not approve the handoff', 'Decision gate risk prerequisite');
requireText(decisionGate, 'nativeRuntimeRequired: true', 'JavaScript decision gate requires native Rust authorization');
requireText(decisionGate, 'JavaScript gate is non-authoritative', 'JavaScript gate never authorizes downstream handoff');
requireText(read('docs/agents/00-decision-orchestrator.md'), 'full 364-entry inventory', 'Agent orchestrator strict full-inventory requirement');
const strictPolicy = parseJson('docs/STRICT_DECISION_LAYER_POLICY.json');
requireText(packageJson ? JSON.stringify(packageJson.scripts ?? {}) : '', 'test:agent-packs', 'Agent pack coverage test script');
requireText(packageJson ? JSON.stringify(packageJson.scripts ?? {}) : '', 'test:rust-audit-chain-contract', 'Durable audit-chain and specialist-receipt contract tests');
const receiptContract = read('docs/STRICT_SPECIALIST_DISPATCH_RECEIPTS_0.4.2.md');
requireText(receiptContract, 'exactly one durable dispatch receipt', 'Strict specialist receipt acceptance contract');
requireText(receiptContract, 'current runtime has no configured AI-provider dispatcher', 'No fabricated specialist execution');
const auditChainRustTest = read('scripts/test-rust-audit-chain-contract.test.mjs');
requireText(auditChainRustTest, 'schema_migrated_v2_to_v3', 'SQLite schema migration regression test');
requireText(auditChainRustTest, 'durable specialist receipts are required', 'Specialist receipt contract regression test');
for (let index = 1; index <= 9; index += 1) {
  const agentPath = `docs/agents/${String(index).padStart(2, '0')}-${['smc-ict-agent', 'price-action-agent', 'support-resistance-trend-agent', 'technical-indicators-agent', 'order-flow-market-profile-agent', 'divergence-intermarket-agent', 'other-methods-agent', 'fundamental-macro-agent', 'risk-management-evaluation-agent'][index - 1]}.md`;
  requireText(read(agentPath), 'Runtime execution proof (strict v0.4.2)', `Specialist ${index} runtime receipt rule`);
}
if (strictPolicy && (strictPolicy.expectedEntryCount !== 364 || strictPolicy.liveExecutionAuthorized !== false)) errors.push('Strict decision policy must require 364 entries and never authorize live execution.');
if (strictPolicy && (strictPolicy.requiredSpecialistReceiptCount !== 9 || strictPolicy.providerDispatchConfigured !== false)) errors.push('Strict decision policy must require nine receipts and must not falsely claim provider dispatch is configured.');

const bridge = read('src/lib/adapters/tauri/bridge.ts');
const nativeGuard = read('src/lib/adapters/tauri/native-runtime-guard.js');
requireText(nativeGuard, 'NATIVE_RUNTIME_REQUIRED', 'Stable native runtime failure code');
requireText(nativeGuard, 'Browser execution is disabled', 'No synthetic browser fallback');
const engineSource = read('src-tauri/src/runtime/trading_core.rs');
requireText(engineSource, 'fn fresh_state() -> EngineState', 'Empty startup state constructor');
requireText(engineSource, 'closed_trade_count', 'Dashboard win-rate denominator');
requireText(read('src/lib/types.ts'), 'closedTradeCount: number', 'Typed dashboard win-rate denominator');
requireText(engineSource, 'purge_legacy_synthetic_state', 'Persisted legacy fixture migration');
requireText(engineSource, 'state.kill_switch_engaged = true', 'Fail-closed operator control after legacy cleanup');
forbidText(engineSource, 'fn demo_state()', 'No product-level demo account seed');
forbidText(engineSource, 'fn demo_candles(', 'No synthetic candle generator in native runtime');
requireText(bridge, 'assertNativeRuntime', 'Shared native-runtime boundary guard');
forbidText(bridge, 'browserPreviewSnapshot', 'No browser market-data fallback');
forbidText(bridge, '67_421.12', 'No synthetic price constants in runtime bridge');
forbidText(bridge, 'Bundled synthetic fixtures', 'No synthetic candle source in runtime bridge');
requireText(bridge, "invoke<CandleSeriesResponse>('get_candle_series', { request })", 'Typed candle-series bridge invocation');

const security = read('SECURITY.md');
requireText(security, 'No account/order, withdrawal, wallet-signing, or private exchange endpoints are called.', 'Security boundary documentation');
requireText(security, 'The Svelte frontend is not a security boundary.', 'Security boundary documentation');
requireText(workflow, 'tagged-release-build:', 'Tagged release build gate');
requireText(read('scripts/release-version-policy.mjs'), 'Release tag v${tagVersion} does not match application manifest version ${applicationVersion}.', 'Release tag/version identity contract');
requireText(workflow, 'npm run release:preflight', 'Fail-closed release readiness preflight');
requireText(workflow, 'cancel-in-progress:', 'Stale non-release CI cancellation');
requireText(workflow, 'timeout-minutes:', 'CI job runtime bounds');
requireText(read('docs/REQUEST_RATE_ISOLATION_0.8.3.md'), 'failed-authentication pressure', 'Separate authentication failure budget documentation');
requireText(workflow, 'npm test', 'Canonical full Node test suite in release CI');
requireText(read('scripts/check-release-readiness.mjs'), 'Cargo.lock root package version', 'Cargo lock manifest alignment check');
requireText(read('scripts/check-release-readiness.mjs'), "'bun.lock'", 'Bun/Hono lockfile release gate');
requireText(read('scripts/check-release-readiness.mjs'), "['bun', ['--version']]", 'Bun runtime release toolchain gate');
requireText(read('scripts/check-release-readiness.mjs'), 'bun install --frozen-lockfile', 'Pinned Bun install release gate');
requireText(read('scripts/check-release-readiness.mjs'), 'package-lock.json root package identity/version', 'npm lock manifest alignment check');
const guardrailJob = workflow.split('  repository-guardrails:')[1]?.split('\n  bun-compatibility:')[0] ?? '';
requireText(guardrailJob, 'npm test', 'Repository CI runs the entire source test suite');
requireText(workflow, 'npm run release:evidence', 'CI release provenance manifest generation');
requireText(workflow, 'npm run release:evidence:verify', 'CI release artifact manifest verification');
requireText(workflow, 'atos-release-staging/**', 'CI uploads the exact staged installer payload and release evidence');
requireText(workflow, 'actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6', 'Pinned signed Sigstore release provenance attestation');
requireText(workflow, 'anchore/sbom-action@66cbf4bc1f1c0d2edc94016e65bc221b6bb0ad6c', 'Pinned release SPDX SBOM generation');
requireText(read('rust-toolchain.toml'), 'channel = \"1.99.1\"', 'Pinned Rust compiler toolchain');
requireText(workflow, 'scripts/verify-release-attestation.mjs', 'Release verifies signer workflow and source tag');
requireText(workflow, 'cargo test --manifest-path src-tauri/Cargo.toml --all-targets', 'CI Rust regression tests');
requireText(workflow, 'test -f package-lock.json', 'Release npm lockfile gate');
requireText(workflow, 'test -f bun.lock', 'Release Bun lockfile gate');
requireText(workflow, 'test -f src-tauri/Cargo.lock', 'Release Cargo lockfile gate');
requireText(workflow, 'npm ci --no-audit --fund=false', 'Locked npm install for release builds');
requireText(workflow, 'cargo test --locked', 'Locked Cargo test for release builds');

const marketDataModel = read('src/lib/domain/market-data/market-data-model.js');
requireText(marketDataModel, 'No substitute series will be returned', 'No-fallback candle series contract');
requireText(marketDataModel, 'getAvailableCandleSeries', 'Provider capability selector');
requireText(marketDataModel, 'validateCandleSeriesResponse', 'Frontend candle-series response validator');
requireText(marketDataModel, 'OHLC price-range invariants', 'Frontend OHLC invariant validation');
requireText(marketDataModel, 'Candle cadence gap at row', 'Frontend interval cadence validation');
requireText(marketDataModel, 'latest fully closed bar is stale', 'Live series closed-bar recency validation');
requireText(marketDataModel, 'retrieval timestamp is implausibly far in the future', 'Market-data receipt clock validation');
requireText(marketDataModel, 'latestClosedBarAtMs !== expectedLatestClosedBarAtMs', 'Closed-bar timestamp provenance validation');
const marketDataTests = read('scripts/test-market-data-model.test.mjs');
requireText(marketDataTests, 'does not silently substitute another series', 'Exact request routing regression test');
requireText(marketDataTests, 'order chart alignment', 'Order/chart mismatch regression test');
requireText(marketDataTests, 'fails closed on request mismatch', 'Candle-series response fail-closed regression tests');
requireText(marketDataTests, 'validated candle snapshot is isolated from post-validation mutation', 'Validated market-data TOCTOU regression test');
const rustStrict = read('src-tauri/src/application/strict_analysis.rs');
requireText(rustStrict, 'include_str!("../../../scripts/fixtures/technical-analysis-master-list.v1.json")', 'Rust embeds reviewed master-list manifest');
requireText(rustStrict, 'pub fn create_work_plan', 'Rust native strict work-plan builder');
requireText(rustStrict, 'pub fn evaluate_completion', 'Rust strict completion barrier');
requireText(rustStrict, 'Catalog-only concept must remain unsupported', 'Rust strict no-self-promotion rule');
requireText(rustStrict, 'next_layer_payload', 'Rust downstream payload gate');
requireText(rustStrict, 'fn full_inventory_still_blocks_handoff_when_catalog_only_concepts_exist', 'Rust fail-closed tests');
requireText(rustStrict, 'live_execution_authorized: false', 'Rust strict gate never authorizes live execution');
requireText(read('src-tauri/src/adapters/tauri/commands.rs'), 'evaluate_strict_analysis_completion', 'Tauri registers Rust strict completion command');
requireText(bridge, "invoke<StrictAnalysisWorkPlan>('create_strict_analysis_work_plan', { request })", 'Typed Rust strict work-plan bridge');
requireText(bridge, "invoke<StrictCompletionResult>('evaluate_strict_analysis_completion', { request })", 'Typed Rust strict completion bridge');
const models = read('src-tauri/src/domain/models.rs');
requireText(models, 'pub enum CandleInterval', 'Typed Rust candle interval');
requireText(models, 'pub struct CandleSeriesRequest', 'Typed Rust candle-series request');
requireText(models, 'pub struct MarketDataCapabilities', 'Explicit market-data capabilities contract');
requireText(engine, 'typed_candle_series_returns_only_the_exact_supported_symbol_and_interval', 'Rust series routing regression test');

if (!fs.existsSync(path.join(root, 'package-lock.json'))) warnings.push('No package-lock.json is present; npm dependency resolution is not reproducible yet and tagged releases are blocked.');
if (!fs.existsSync(path.join(root, 'bun.lock'))) warnings.push('No bun.lock is present; Bun/Hono runtime dependency resolution is not reproducible yet and tagged releases are blocked.');
if (!fs.existsSync(path.join(root, 'src-tauri/Cargo.lock'))) warnings.push('No Cargo.lock is present; Rust dependency resolution is not pinned yet.');

if (errors.length > 0) {
  console.error(`Repository guardrail check failed (${errors.length} error(s)):`);
  for (const item of errors) console.error(`  - ${item}`);
  for (const item of warnings) console.warn(`  ! ${item}`);
  process.exit(1);
}

console.log('Repository guardrail check passed.');
console.log('This check is static and does not claim that Rust or Svelte builds/tests passed.');
for (const item of warnings) console.warn(`  ! ${item}`);

