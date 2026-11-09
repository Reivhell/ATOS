import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');

test('native domain exposes a versioned normalized market-data contract with lineage and quality', async () => {
  const source = await read('src-tauri/src/domain/market_data_platform.rs');
  for (const token of [
    'MARKET_DATA_PLATFORM_SCHEMA_VERSION', 'MarketDataQuery', 'MarketDataProviderCatalogEntry',
    'MarketDataRequestSummary', 'MarketDataProvenance', 'MarketDataQuality', 'dataset_fingerprint_sha256',
    'scope_fingerprint_sha256', 'MarketDataVolumeBasis', 'MarketDataFreshness', 'gap_candidates'
  ]) assert.ok(source.includes(token), `Missing native market-data contract field: ${token}`);
  assert.match(source, /deny_unknown_fields/);
  assert.match(source, /MAX_MARKET_DATA_RANGE_MS/);
  assert.match(source, /MAX_MARKET_DATA_PAGE_SIZE/);
  assert.match(source, /pub allow_stale: bool/);
  assert.match(source, /pub max_data_age_ms: i64/);
});

test('broker-context requests are pinned to MT5 and stale-data override is prohibited', async () => {
  const source = await read('src-tauri/src/domain/market_data_platform.rs');
  assert.match(source, /self\.purpose == MarketDataPurpose::BrokerContext/);
  assert.match(source, /provider != "mt5-terminal"/);
  assert.match(source, /Stale data cannot be explicitly allowed for broker context/);
  assert.match(source, /self\.provider_id = Some\("mt5-terminal"\.into\(\)\)/);
});

test('registry has explicit built-in providers and rejects duplicate or unknown identifiers without fallback', async () => {
  const source = await read('src-tauri/src/infrastructure/providers/registry.rs');
  assert.match(source, /MT5_PROVIDER_ID: &str = "mt5-terminal"/);
  assert.match(source, /BINANCE_PROVIDER_ID: &str = "binance-public"/);
  assert.match(source, /fn register\(/);
  assert.match(source, /already registered/);
  assert.match(source, /no fallback provider was selected/);
  assert.match(source, /unwrap_or\(BINANCE_PROVIDER_ID\)/);
});

test('MT5 queries are bound to authenticated account identity and a stable broker snapshot', async () => {
  const registry = await read('src-tauri/src/infrastructure/providers/registry.rs');
  const bridge = await read('src-tauri/src/infrastructure/mt5/bridge.rs');
  assert.match(bridge, /pub fn authenticated_account_identity/);
  assert.match(registry, /authenticated_account_identity\(\)/);
  assert.match(registry, /market_data_series_query\(Mt5SeriesQuery/);
  assert.match(registry, /confirmed\.last_sequence != sequence/);
  assert.match(registry, /confirmed_identity\.0 != broker_server\s*\|\|\s*confirmed_identity\.1 != account_login/);
  assert.match(registry, /Broker context requires a connected, non-stale MT5 terminal heartbeat/);
  assert.match(registry, /scope_fingerprint_sha256: None/);
  assert.match(registry, /authenticated account identity is validated inside the native bridge/);
});

test('public Binance cache cannot be used for broker context or substitute another symbol/timeframe', async () => {
  const source = await read('src-tauri/src/infrastructure/providers/registry.rs');
  assert.match(source, /Public exchange data cannot satisfy a broker-context query/);
  assert.match(source, /request\.symbol != "BTCUSDT" \|\| request\.timeframe != "M5"/);
  assert.match(source, /no substitute dataset was returned/);
  assert.match(source, /MarketDataVolumeBasis::BaseAsset/);
  assert.match(source, /ATOS-BINANCE-QUERY-DATASET-V1/);
  assert.match(source, /let dataset: Vec<_> = series\s*\.\s*candles\s*\.\s*into_iter\s*\(\s*\)/);
  assert.match(source, /let fingerprint = dataset_hasher\s*\.\s*finalize\s*\(\s*\)\s*\.\s*iter\s*\(\s*\)\s*\.\s*map/);
});

test('Tauri registers the provider catalog and unified query as native commands', async () => {
  const commands = await read('src-tauri/src/adapters/tauri/commands.rs');
  const bootstrap = await read('src-tauri/src/adapters/tauri/bootstrap.rs');
  assert.match(commands, /pub fn get_market_data_provider_catalog/);
  assert.match(commands, /pub fn query_unified_market_data/);
  assert.match(bootstrap, /commands::get_market_data_provider_catalog/);
  assert.match(bootstrap, /commands::query_unified_market_data/);
});

test('Svelte Market Data view consumes the native provider registry and renders provenance and freshness', async () => {
  const app = await read('src/App.svelte');
  const panel = await read('src/lib/components/MarketDataPlatformPanel.svelte');
  const bridge = await read('src/lib/adapters/tauri/bridge.ts');
  assert.match(app, /MarketDataPlatformPanel/);
  assert.match(panel, /getMarketDataProviderCatalog/);
  assert.match(panel, /queryUnifiedMarketData/);
  assert.match(panel, /datasetFingerprintSha256/);
  assert.match(panel, /quality\.freshness/);
  assert.match(panel, /bind:checked=\{allowStale\}/);
  assert.match(panel, /result\.request\.allowStale/);
  assert.match(panel, /purpose === 'brokerContext' \? 'mt5-terminal'/);
  assert.match(bridge, /query_unified_market_data/);
  assert.match(bridge, /maxDataAgeMs/);
});

test('provider freshness is anchored to observed market time, not cache ingestion time', async () => {
  const registry = await read('src-tauri/src/infrastructure/providers/registry.rs');
  assert.match(registry, /freshness\(latest_observation_at_ms, now, request\.max_data_age_ms\)/);
  assert.match(registry, /let latest_observation_at_ms\s*=\s*latest_closed_bar_at_ms\(&dataset, CandleInterval::FiveMinutes\)\.ok\(\)/);
  assert.match(registry, /freshness\(Some\(old_market_observation\), now, 600_000\)/);
  assert.match(registry, /freshness_is_derived_from_market_observation_not_recent_cache_ingestion/);
  assert.match(registry, /state == MarketDataFreshness::Stale\s*&&\s*allow_stale/);
  assert.match(registry, /MarketDataFreshness::Unavailable\s*,\s*true\s*,\s*MarketDataPurpose::Research/);
});
