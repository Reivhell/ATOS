import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { assertNativeRuntime, NativeRuntimeRequiredError } from '../src/lib/adapters/tauri/native-runtime-guard.js';

const bridge = readFileSync(new URL('../src/lib/adapters/tauri/bridge.ts', import.meta.url), 'utf8');
const app = readFileSync(new URL('../src/App.svelte', import.meta.url), 'utf8');
const engine = readFileSync(new URL('../src-tauri/src/runtime/trading_core.rs', import.meta.url), 'utf8');
const models = readFileSync(new URL('../src-tauri/src/domain/models.rs', import.meta.url), 'utf8');

test('native runtime boundary allows native commands only when the runtime probe returns true', () => {
  assert.doesNotThrow(() => assertNativeRuntime(true, 'dashboard'));
  for (const unavailable of [false, undefined, null, 1, 'true']) {
    assert.throws(
      () => assertNativeRuntime(unavailable, 'dashboard'),
      (error) => error instanceof NativeRuntimeRequiredError
        && error.code === 'NATIVE_RUNTIME_REQUIRED'
        && error.message.includes('no synthetic market data or audit records are available'),
    );
  }
});

test('bridge does not synthesize dashboard, candle or audit data for browser execution', () => {
  assert.doesNotMatch(bridge, /browserPreviewSnapshot|Bundled synthetic fixtures|DEMO FIXTURE|preview-event|67_421\.12/);
  assert.match(bridge, /export async function getDashboardState[\s\S]*?requireDesktopRuntime\('dashboard'\)[\s\S]*?invoke<DashboardSnapshot>\('get_dashboard_state'\)/);
  assert.match(bridge, /export async function getCandleSeries[\s\S]*?requireDesktopRuntime\('market data'\)[\s\S]*?invoke<CandleSeriesResponse>\('get_candle_series'/);
  assert.match(bridge, /export async function getAuditHistory[\s\S]*?requireDesktopRuntime\('audit history'\)[\s\S]*?invoke<AuditEvent\[]>\('get_audit_history'/);
});

test('browser entrypoint never mounts dashboard UI or starts a native command', () => {
  assert.match(app, /onMount\(\(\) => \{[\s\S]*?if \(!desktop\) return;[\s\S]*?void loadSnapshot\(\)/);
  assert.match(app, /window\.setInterval\(\(\) => \{ clockNow = Date\.now\(\); \}, 1000\)/, 'Quote freshness must age while the dashboard is open.');
  assert.match(app, /\{#if !desktop\}[\s\S]*?Installed desktop runtime required[\s\S]*?\{:else\}\s*<div class="app-shell">/);
  assert.match(app, /No synthetic quotes, candles, token rows, or audit events are supplied\./);
  assert.doesNotMatch(app, /Browser preview|PREVIEW MODE|Research fixture data only|DEMO FIXTURE|Fixture or stale quote/);
});

test('fresh native account contains no synthetic observations and starts with its kill switch engaged', () => {
  assert.doesNotMatch(engine, /fn demo_state\(|fn demo_candles\(/);
  const freshState = engine.match(/fn fresh_state\(\) -> EngineState \{[\s\S]*?\n    \}/)?.[0];
  assert.ok(freshState, 'fresh_state must exist');
  assert.match(freshState, /quotes: BTreeMap::new\(\)/);
  assert.match(freshState, /candles: BTreeMap::new\(\)/);
  assert.match(freshState, /screener: Vec::new\(\)/);
  assert.match(freshState, /agents: Vec::new\(\)/);
  assert.match(freshState, /kill_switch_engaged: true/);
  assert.match(engine, /fn purge_legacy_synthetic_state\(state: &mut EngineState\)/);
  assert.match(engine, /Legacy synthetic observations removed/);
  assert.match(engine, /fresh_account_kill_switch_blocks_orders_even_after_a_quote_arrives/);
  assert.doesNotMatch(models, /DEMO FIXTURE · NOT LIVE|Bundled screener fixtures|fixture-v0/);
  assert.match(models, /pub dataset_sha256: String/, 'Native candle responses must carry their canonical OHLCV fingerprint.');
  assert.match(engine, /let dataset_sha256 = crate::application::backtest::dataset_sha256\(&candles\)[\s\S]*?dataset_sha256,/, 'The fingerprint must be computed from the exact candle vector returned by the native response.');
  assert.match(engine, /assert_eq!\s*\(\s*response\.dataset_sha256,\s*crate::application::backtest::dataset_sha256\(&response\.candles\)\s*\)/, 'Native response tests must bind the digest to returned OHLCV.');
});

test('dashboard never renders fabricated trend plots or zero-price fallbacks when evidence is absent', () => {
  assert.doesNotMatch(app, /class="mini-spark|metric-ring|M0,27 L10,24|M0,4 L10,9/, 'No hardcoded portfolio/equity trend graphics are allowed.');
  assert.match(app, /closedTradeCount > 0 && snapshot\.winRatePct !== null \? number\(snapshot\.winRatePct\) : '—'/, 'Win rate must be unavailable in both the native data contract and UI before a closed trade exists.');
  assert.match(engine, /let win_rate_pct = if closed_trade_count == 0\s*\{\s*None\s*\}\s*else\s*\{\s*Some\(/, 'Unknown win rate must be represented as None, not a fabricated zero-percent value.');
  assert.match(readFileSync(new URL('../src-tauri/src/domain/models.rs', import.meta.url), 'utf8'), /pub win_rate_pct: Option<f64>/, 'Native snapshot must represent unavailable win rate as null.');
  assert.match(app, /\$:\s*currentQuoteFresh = quoteIsFresh\(currentQuote, clockNow\)/, 'Order entry must reuse the shared quote freshness policy.');
  assert.match(app, /No agent sessions configured/, 'No agent session must not be presented as running.');
  assert.match(app, /portfolioMarksFresh = Number\.isFinite\(clockNow\)[\s\S]*?every\(\(position\) => positionMarkFresh\(position\.symbol, clockNow\)\)/, 'Portfolio metrics must age reactively and become unavailable when any position mark goes stale.');
  assert.match(app, /Unavailable \u00b7 stale mark/, 'Stale position marks must not be presented as current exposure or PnL.');
  assert.match(app, /Exposure unavailable[\s\S]*?Refresh market data for every open position/, 'Exposure ratios must not be shown against stale portfolio equity.');
  assert.match(app, /activeChartQuote = quoteFor\(activeCandleRequest\.symbol\)[\s\S]*?activeChartQuoteFresh = positionMarkFresh\(activeCandleRequest\.symbol, clockNow\)/, 'Chart quote must age reactively and never display a stale quote as current.');
  assert.doesNotMatch(app, /money\(quoteFor\(activeCandleRequest\.symbol\)\?\.price \?\? 0\)/, 'Missing quote cannot display as a zero price.');
  assert.match(app, /No fresh validated quote is available/, 'Stale/absent quote explanation must be explicit.');
  assert.match(app, /function ageLabel\(value: number, now = clockNow\)[\s\S]*?if \(value > now\) return 'timestamp in future'/, 'Future timestamps must not be formatted as if they were observed moments ago.');
  assert.match(app, /function quoteIsFresh\(quote: Quote \| undefined[\s\S]*?isFreshMarketTimestamp\(providerObservedAtMs, now\)[\s\S]*?isFreshMarketTimestamp\(quoteObservedAtMs, now\)[\s\S]*?isFreshMarketTimestamp\(quoteRetrievedAtMs, now\)[\s\S]*?providerObservationFresh && quoteObservationFresh && quoteReceiptFresh/, 'Every UI surface must require provider, exchange-observation and local-receipt freshness.');
  assert.match(app, /\[\.\.\.freshQuotes\]\.sort/, 'Top performers must not rank stale quotes as current market leaders.');
  assert.match(app, /Unavailable · stale quote/, 'Market-data tables must not display stale marks as current prices.');
  assert.match(app, /\$: currentQuote = selectedSymbol \? quoteFor\(selectedSymbol\) : undefined;/, 'Order requests must not silently fall back to a quote for another instrument.');
  assert.match(app, /disabled=\{orderBusy \|\| snapshot\.killSwitchEngaged \|\| !desktop \|\| orderType !== 'Market' \|\| !currentQuoteFresh \|\| !Number\.isFinite\(orderQuantity\) \|\| orderQuantity <= 0\}/, 'Paper-order CTA requires selected fresh quote and positive finite quantity.');
  assert.match(readFileSync(new URL('../src/lib/types.ts', import.meta.url), 'utf8'), /closedTradeCount: number/, 'Snapshot contract carries win-rate denominator.');
});


test('risk gateway blocks risk-increasing buys when any open position has an unpriced or stale mark', () => {
  assert.match(engine, /let portfolio_marks_fresh = state\.positions\.iter\(\)\.all\(/);
  assert.match(engine, /position_quote\.price\.is_finite\(\) && position_quote\.price > 0\.0/);
  assert.match(engine, /Self::quote_observation_is_fresh\(state, position_quote, now\)/, 'Portfolio checks must reuse the shared provider-aware freshness predicate.');
  assert.match(engine, /fn timestamp_is_fresh\(now_ms: i64, observed_at_ms: i64, maximum_age_ms: i64\) -> bool[\s\S]*?now_ms >= observed_at_ms[\s\S]*?now_ms\.saturating_sub\(observed_at_ms\) <= maximum_age_ms/, 'Future timestamps must never satisfy market-data freshness.');
  assert.match(engine, /rule: "Portfolio mark freshness"\.into\(\), passed: side == "SELL" \|\| portfolio_marks_fresh/);
  assert.match(engine, /risk_increasing_buy_requires_fresh_marks_for_every_open_position_but_sell_remains_available/);
  assert.match(engine, /quote\.is_live && provider_observation_fresh/, 'Cached quote freshness cannot authorize an order after provider failure.');
  assert.match(engine, /fn quote_observation_is_fresh\(state: &EngineState, quote: &Quote, now_ms: i64\) -> bool[\s\S]*?quote\.is_live[\s\S]*?Self::provider_observation_is_fresh\(state, now_ms\)/, 'The shared freshness predicate must require a quote and a current provider observation.');
  assert.match(engine, /failed_provider_refresh_blocks_orders_even_when_cached_quote_is_young/);
  assert.match(app, /\$:\s*currentQuoteFresh = quoteIsFresh\(currentQuote, clockNow\)/, 'UI and native risk gateway must share one provider-aware quote boundary.');
});
