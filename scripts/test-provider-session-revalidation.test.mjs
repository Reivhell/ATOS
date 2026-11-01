import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const root = new URL('../', import.meta.url);
const read = (path) => readFile(new URL(path, root), 'utf8');

test('startup invalidates persisted market and screener live claims before exposing state', async () => {
  const source = await read('src-tauri/src/runtime/trading_core.rs');
  const openBody = source.slice(source.indexOf('pub fn open('), source.indexOf('/// Dedicated durable store'));
  assert.match(openBody, /invalidate_restored_provider_sessions\(&mut state\)/);
  assert.ok(openBody.indexOf('invalidate_restored_provider_sessions(&mut state)') < openBody.indexOf('Self::mark_positions(&mut state)'));
  assert.match(source, /quote\.is_live = false/);
  assert.match(source, /STALE · RESTART REVALIDATION REQUIRED/);
  assert.match(source, /Persisted provider session invalidated/);
  assert.match(openBody, /if !state\.kill_switch_engaged[\s\S]*?state\.kill_switch_engaged = true[\s\S]*?Kill switch re-engaged on startup/);
});

test('single timestamp predicate rejects zero, future, and expired observations', async () => {
  const source = await read('src-tauri/src/runtime/trading_core.rs');
  assert.match(source, /fn timestamp_is_fresh\(now_ms: i64, observed_at_ms: i64, maximum_age_ms: i64\) -> bool/);
  assert.match(source, /now_ms >= observed_at_ms/);
  assert.match(source, /Self::timestamp_is_fresh\(now, quote\.updated_at_ms, MARKET_DATA_OBSERVATION_MAX_AGE_MS\)/);
  assert.match(source, /Self::timestamp_is_fresh\s*\(\s*now_ms\s*,\s*quote\.retrieved_at_ms\s*,\s*MARKET_DATA_OBSERVATION_MAX_AGE_MS[\s,]*\)/);
  assert.match(source, /previous\.retrieved_at_ms > 0 && previous\.updated_at_ms > quote\.updated_at_ms/);
  assert.match(source, /Self::provider_observation_is_fresh\(state, now_ms\)/);
  assert.match(source, /Self::quote_observation_is_fresh\(state, quote, snapshot_now_ms\)/);
});

test('portfolio risk and order provenance require a current provider observation', async () => {
  const source = await read('src-tauri/src/runtime/trading_core.rs');
  assert.match(source, /passed: quote\.is_live && provider_observation_fresh/);
  assert.match(source, /Self::quote_observation_is_fresh\(state, position_quote, now\)/);
  assert.match(source, /quote\.retrieved_at_ms = now/);
  assert.match(source, /Portfolio mark freshness/);
  assert.match(source, /restarted process must require explicit operator re-arm/);
});

test('quote freshness preserves exchange observation time separately from local receipt time', async () => {
  const [models, provider, app, statePolicy] = await Promise.all([
    read('src-tauri/src/domain/models.rs'),
    read('src-tauri/src/infrastructure/providers/public_market.rs'),
    read('src/App.svelte'),
    read('src-tauri/src/domain/state_decision.rs'),
  ]);
  assert.match(models, /pub updated_at_ms: i64,[\s\S]*?pub retrieved_at_ms: i64/);
  assert.match(provider, /#\[serde\(rename = \"closeTime\"\)\][\s\S]*?close_time: i64/);
  assert.match(provider, /updated_at_ms:\s*ticker\.close_time,\s*retrieved_at_ms:\s*0/);
  assert.match(provider, /let response_received_at_ms = Utc::now\(\)\.timestamp_millis\(\)/);
  assert.match(provider, /ticker\.close_time\s*<=\s*0\s*\|\|\s*ticker\.close_time\s*>\s*response_received_at_ms/);
  assert.match(provider, /response_received_at_ms\.saturating_sub\(ticker\.close_time\)\s*>\s*MARKET_DATA_OBSERVATION_MAX_AGE_MS/);
  assert.match(app, /quoteReceiptFresh/);
  assert.match(app, /quote\.retrievedAtMs/);
  assert.match(statePolicy, /\[\"updatedAtMs\", \"retrievedAtMs\"\]/);
  assert.match(statePolicy, /quote_observation_is_fresh\(quote, now_ms\)/);
  assert.match(statePolicy, /use crate::domain::models::MARKET_DATA_OBSERVATION_MAX_AGE_MS/);
});

test('fixed public market-data endpoints reject HTTP redirects', async () => {
  const source = await read('src-tauri/src/infrastructure/providers/public_market.rs');
  assert.match(source, /use reqwest::\{redirect::Policy, Client, Response\}/);
  assert.equal((source.match(/\.redirect\(Policy::none\(\)\)/g) ?? []).length, 2);
});

test('stale or disconnected screener snapshots cannot be presented as current UI evidence', async () => {
  const source = await read('src/App.svelte');
  assert.match(source, /screenerProviderStatus\.startsWith\('CONNECTED'\)/);
  assert.match(source, /isFreshMarketTimestamp\(snapshot\.screenerProviderUpdatedAtMs, clockNow\)/);
  assert.match(source, /\[\.\.\.\(screenerSnapshotFresh \? \(snapshot\?\.screener \?\? \[\]\) : \[\]\)\]/);
  assert.match(source, /Stored rows are retained for audit\/history only and are not presented as current evidence/);
  assert.match(source, /quoteReceiptFresh/);
  assert.match(source, /quote\.retrievedAtMs/);
  assert.match(source, /isFreshMarketTimestamp\(snapshot\.providerUpdatedAtMs, clockNow\)/);

});


test('market and screener refreshes hold exclusive in-flight leases across async provider work', async () => {
  const source = await read('src-tauri/src/infrastructure/providers/public_market.rs');
  assert.match(source, /static SCREENER_REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new\(false\)/);
  assert.match(source, /static MARKET_REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new\(false\)/);
  assert.match(source, /struct RefreshLease<'a>/);
  assert.match(source, /impl Drop for RefreshLease<'_>/);
  assert.match(source, /self\.in_flight\.store\(false, Ordering::Release\)/);
  assert.match(source, /claim_refresh_slot\s*\(\s*&LAST_SCREENER_REFRESH_MS\s*,\s*&SCREENER_REFRESH_IN_FLIGHT/);
  assert.match(source, /claim_refresh_slot\s*\(\s*&LAST_MARKET_REFRESH_MS\s*,\s*&MARKET_REFRESH_IN_FLIGHT/);
  assert.match(source, /refresh_slot_rejects_overlapping_requests_even_after_rate_window_elapsed/);
  assert.match(source, /refresh_slot_enforces_minimum_interval_and_releases_after_throttle_rejection/);
});


test('market-data freshness policy uses one bounded timestamp predicate across UI and candle validation', async () => {
  const policy = await read('src/lib/domain/market-data/market-data-policy.js');
  const app = await read('src/App.svelte');
  const model = await read('src/lib/domain/market-data/market-data-model.js');
  const nativeModels = await read('src-tauri/src/domain/models.rs');
  const nativeCore = await read('src-tauri/src/runtime/trading_core.rs');
  const nativeProvider = await read('src-tauri/src/infrastructure/providers/public_market.rs');
  const nativeDecision = await read('src-tauri/src/domain/state_decision.rs');
  assert.match(policy, /MARKET_DATA_OBSERVATION_MAX_AGE_MS = 120_000/);
  const nativeMaximumAge = nativeModels.match(/MARKET_DATA_OBSERVATION_MAX_AGE_MS: i64 = ([0-9_]+);/)?.[1]?.replaceAll('_', '');
  assert.equal(Number(nativeMaximumAge), 120_000, 'Native and frontend maximum market-data age must remain identical.');
  assert.match(nativeModels, /CANDLE_PROVIDER_RETRIEVAL_MAX_AGE_MS: i64 = MARKET_DATA_OBSERVATION_MAX_AGE_MS/);
  assert.match(nativeCore, /MARKET_DATA_OBSERVATION_MAX_AGE_MS/);
  assert.match(nativeProvider, /MARKET_DATA_OBSERVATION_MAX_AGE_MS/);
  assert.match(nativeDecision, /MARKET_DATA_OBSERVATION_MAX_AGE_MS/);
  assert.match(app, /isFreshMarketTimestamp\(quoteObservedAtMs, now\)/);
  assert.match(app, /isFreshMarketTimestamp\(quoteRetrievedAtMs, now\)/);
  assert.match(model, /isFreshMarketTimestamp\(response\.retrievedAtMs, now, 30_000\)/);
  const { isFreshMarketTimestamp, MARKET_DATA_OBSERVATION_MAX_AGE_MS } = await import('../src/lib/domain/market-data/market-data-policy.js');
  const now = 1_800_000_000_000;
  assert.equal(MARKET_DATA_OBSERVATION_MAX_AGE_MS, 120_000);
  assert.equal(isFreshMarketTimestamp(now, now), true);
  assert.equal(isFreshMarketTimestamp(now - 120_000, now), true);
  assert.equal(isFreshMarketTimestamp(now - 120_001, now), false);
  assert.equal(isFreshMarketTimestamp(now + 1, now), false);
  assert.equal(isFreshMarketTimestamp(now + 30_000, now, 30_000), true);
  assert.equal(isFreshMarketTimestamp(now + 30_001, now, 30_000), false);
  assert.equal(isFreshMarketTimestamp(0, now), false);
  assert.equal(isFreshMarketTimestamp(Number.NaN, now), false);
  assert.equal(isFreshMarketTimestamp(now, Number.NaN), false);
});
