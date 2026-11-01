import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import path from 'node:path';

const root = path.resolve(new URL('..', import.meta.url).pathname);
const read = (relative) => readFile(path.join(root, relative), 'utf8');

test('legacy imports remain stable while implementation resides in explicit layers', async () => {
  const legacy = await import(pathToFileURL(path.join(root, 'src/lib/technical-analysis.js')));
  const layered = await import(pathToFileURL(path.join(root, 'src/lib/domain/technical-analysis/catalog.js')));
  const indicators = await import(pathToFileURL(path.join(root, 'src/lib/engines/technical-analysis/indicators.js')));
  assert.equal(legacy.TECHNICAL_ANALYSIS_CATALOG, layered.TECHNICAL_ANALYSIS_CATALOG);
  assert.equal(legacy.calculateIndicatorSnapshot, indicators.calculateIndicatorSnapshot);
  assert.equal(legacy.TECHNICAL_ANALYSIS_CATALOG.length, 364);

  const service = await import(pathToFileURL(path.join(root, 'src/lib/application/technical-analysis-service.js')));
  assert.equal(typeof service.createTechnicalAnalysisReport, 'function');
});

test('domain dependencies point inward and exclude runtime, UI, persistence, and provider APIs', async () => {
  const domainFiles = [
    'src/lib/domain/decision/contracts.js',
    'src/lib/domain/decision/research-policy.js',
    'src/lib/domain/decision/canonical-evidence-registry.js',
    'src/lib/domain/decision/strict-catalog-registry.js',
    'src/lib/domain/market-data/market-data-model.js',
    'src/lib/domain/market-data/timeframe-contract.js',
    'src/lib/domain/market-data/candle-series-audit.js',
    'src/lib/domain/technical-analysis/catalog.js',
  ];
  for (const file of domainFiles) {
    const source = await read(file);
    assert.doesNotMatch(source, /from ['"].*\/(?:engines|application|adapters|presentation|infrastructure)\//, `${file} must not depend on outer layers`);
    assert.doesNotMatch(source, /@tauri-apps|node:sqlite|reqwest|fetch\(/, `${file} must remain framework/provider free`);
  }
});

test('compatibility files are shims rather than second implementations', async () => {
  const expected = [
    ['src/lib/technical-analysis.js', 'domain/technical-analysis/catalog.js'],
  ];
  for (const [file, target] of expected) {
    const source = await read(file);
    assert.ok(source.includes(target), `${file} must delegate to ${target}`);
    assert.ok(source.length < 400, `${file} should not contain duplicated implementation logic`);
  }
});


test('Svelte reaches the analysis engine through the application service boundary', async () => {
  const appSource = await read('src/App.svelte');
  assert.match(appSource, /createTechnicalAnalysisReport\(/);
  assert.match(appSource, /application\/technical-analysis-service\.js/);
  assert.doesNotMatch(appSource, /engines\/technical-analysis\/analysis-engine\.js/);
});


test('Rust backend uses explicit domain, application, runtime, infrastructure and adapter layers', async () => {
  const required = [
    'src-tauri/src/domain/models.rs', 'src-tauri/src/domain/state_decision.rs',
    'src-tauri/src/application/backtest.rs', 'src-tauri/src/application/strict_analysis.rs',
    'src-tauri/src/runtime/trading_core.rs', 'src-tauri/src/infrastructure/mt5/bridge.rs',
    'src-tauri/src/infrastructure/mt5/market_data.rs', 'src-tauri/src/infrastructure/providers/public_market.rs',
    'src-tauri/src/adapters/tauri/commands.rs', 'src-tauri/src/adapters/tauri/bootstrap.rs',
  ];
  for (const file of required) assert.ok((await read(file)).length > 0, `${file} must exist`);
  const root = await read('src-tauri/src/lib.rs');
  const commands = await read('src-tauri/src/adapters/tauri/commands.rs');
  const provider = await read('src-tauri/src/infrastructure/providers/public_market.rs');
  const bootstrap = await read('src-tauri/src/adapters/tauri/bootstrap.rs');
  const domain = await read('src-tauri/src/domain/models.rs');
  assert.ok(root.length < 1_000, 'composition root must remain small');
  assert.doesNotMatch(root, /#\[tauri::command\]|reqwest::|sqlite|https:\/\//, 'composition root must not own transport or provider behavior');
  assert.match(commands, /#\[tauri::command\]/);
  assert.doesNotMatch(commands, /reqwest::|Client::builder|Connection::open|TransactionBehavior::/, 'IPC adapter must delegate provider and persistence work');
  assert.doesNotMatch(provider, /tauri::|State<|#\[tauri::command\]/, 'provider integration must be independent of Tauri IPC');
  assert.match(bootstrap, /commands::get_dashboard_state/);
  assert.doesNotMatch(domain, /tauri::|reqwest::|rusqlite::|keyring::|fetch\(/, 'domain contracts must remain infrastructure-free');
});


test('Tauri IPC command declarations and bootstrap registrations are one-to-one', async () => {
  const commands = await read('src-tauri/src/adapters/tauri/commands.rs');
  const bootstrap = await read('src-tauri/src/adapters/tauri/bootstrap.rs');
  const declarations = [...commands.matchAll(/#\[tauri::command\]\s*pub\s+(?:async\s+)?fn\s+([A-Za-z0-9_]+)/g)].map((match) => match[1]);
  const registrations = [...bootstrap.matchAll(/commands::([A-Za-z0-9_]+)/g)].map((match) => match[1]);
  assert.ok(declarations.length > 0, 'Tauri commands must be explicitly declared');
  assert.equal(new Set(declarations).size, declarations.length, 'command declarations must be unique');
  assert.equal(new Set(registrations).size, registrations.length, 'command registrations must be unique');
  assert.deepEqual([...registrations].sort(), [...declarations].sort(), 'every command must be registered exactly once and no stale registration may remain');
  assert.doesNotMatch(commands, /\bpub\s+pub\b|\basync\s+pub\b/, 'command visibility and async modifiers must be valid Rust syntax');
});

test('Rust compile-time embedded assets resolve after the layer reorganization', async () => {
  const { access } = await import('node:fs/promises');
  const strictAnalysis = await read('src-tauri/src/application/strict_analysis.rs');
  const expressions = [...strictAnalysis.matchAll(/include_str!\s*\(\s*"([^"]+)"\s*\)/g)].map((match) => match[1]);
  assert.ok(expressions.length >= 9, 'strict analysis must retain its reviewed catalogue and knowledge packs');
  for (const relative of expressions) {
    const resolved = path.resolve(root, 'src-tauri/src/application', relative);
    await access(resolved);
  }
});


test('whole-project TypeScript configuration includes checked JavaScript application modules', async () => {
  const config = JSON.parse(await read('tsconfig.json'));
  assert.equal(config.compilerOptions.allowJs, true);
  assert.equal(config.compilerOptions.checkJs, true);
  assert.ok(config.include.includes('src/**/*.js'));
});
