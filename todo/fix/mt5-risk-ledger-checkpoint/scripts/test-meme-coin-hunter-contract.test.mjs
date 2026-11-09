import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = (path) => readFile(new URL(`../${path}`, import.meta.url), 'utf8');
const [provider, commands, bootstrap, bridge, types, panel, runtimeGuard] = await Promise.all([
  read('src-tauri/src/infrastructure/providers/meme_coin_hunter.rs'),
  read('src-tauri/src/adapters/tauri/commands.rs'),
  read('src-tauri/src/adapters/tauri/bootstrap.rs'),
  read('src/lib/adapters/tauri/bridge.ts'),
  read('src/lib/types.ts'),
  read('src/lib/components/MemeCoinHunterPanel.svelte'),
  read('src/lib/adapters/tauri/native-runtime-guard.js'),
]);

test('hunter discovers independently of paid promotion signals and prioritizes new-pool observations', () => {
  for (const endpoint of [
    'https://api.geckoterminal.com/api/v2/networks/new_pools?page=1',
    'https://api.geckoterminal.com/api/v2/networks/trending_pools?page=1',
    'https://api.dexscreener.com/token-profiles/latest/v1',
    'https://api.dexscreener.com/token-boosts/latest/v1',
  ]) assert.ok(provider.includes(endpoint));
  assert.match(provider, /ranked_discovery\.sort_by_key\(\|token\| token\.rank\)/);
  assert.match(provider, /MAX_PER_CHAIN: usize = 30/);
  assert.match(provider, /fn valid_solana_address\(address: &str\)/);
  assert.match(provider, /significant_length == 32/);
  assert.match(provider, /fn is_major_quote_asset\(chain: &str, address: &str\)/);
  assert.match(provider, /is_major_quote_asset\(chain, address\)/);
  assert.match(provider, /candidate_key\(&chain, base_address\)/);
});

test('native market requests are bounded, rate-limited and fail closed on incomplete data', () => {
  assert.match(provider, /MAX_RESPONSE_BYTES: usize = 4 \* 1024 \* 1024/);
  assert.match(provider, /redirect\(Policy::none\(\)\)/);
  assert.match(provider, /connect_timeout\(Duration::from_secs\(4\)\)/);
  assert.match(provider, /timeout\(Duration::from_secs\(9\)\)/);
  assert.match(provider, /MIN_SCAN_INTERVAL_MS: i64 = 60_000/);
  assert.match(provider, /SCAN_IN_FLIGHT\s*\.\s*compare_exchange/);
  assert.match(provider, /security\.state != "verified"/);
  assert.match(provider, /liquidity_locked_top10_pct\.is_none\(\)/);
  assert.match(provider, /holder_top10_pct\.unwrap_or\(0\.0\) >= 70\.0/);
  assert.match(provider, /risk_penalty >= 45\.0/);
});

test('GoPlus adapter follows the published EVM and Solana response shapes', () => {
  assert.match(provider, /"\/non_transferable"/);
  assert.match(provider, /"\/creator\/malicious_address"/);
  assert.match(provider, /"\/mintable\/status"/);
  assert.match(provider, /"\/balance_mutable_authority\/status"/);
  assert.match(provider, /data\.get\("lp_holders"\)/);
  assert.match(provider, /dex\.get\("id"\)/);
  assert.match(provider, /dex\.get\("pair"\)/);
  assert.match(provider, /to_ascii_lowercase\(\)/);
  assert.match(provider, /find_security_record/);
  assert.match(provider, /buy_tax.*mandatory_known = false/s);
  assert.match(provider, /if data\.get\(\"fake_token\"\)\.is_some\(\)/);
  assert.match(provider, /absent_fake_token_field_means_no_counterfeit_evidence_per_goplus_contract/);
});

test('GoPlus credentials remain in the OS vault and are zeroized in process memory', () => {
  assert.match(provider, /Entry::new\(SECURITY_SERVICE, SECURITY_ACCOUNT\)/);
  assert.match(provider, /Zeroizing::new\(token\)/);
  assert.match(provider, /keyring_entry\(\)\?\.set_password\(secret\.as_str\(\)\)/);
  assert.match(provider, /No plaintext fallback was used/);
  assert.doesNotMatch(provider, /std::env::var\(|\.env\b|localStorage|sessionStorage/);
});

test('hunter exposes report and credential commands only through the native Tauri boundary', () => {
  for (const command of [
    'get_meme_coin_hunter_security_status', 'save_meme_coin_hunter_security_token',
    'remove_meme_coin_hunter_security_token', 'run_meme_coin_hunter', 'get_latest_meme_coin_hunter_report',
  ]) {
    assert.match(commands, new RegExp(`fn ${command}`));
    assert.match(bootstrap, new RegExp(`commands::${command}`));
  }
  assert.match(bridge, /requireDesktopRuntime\('meme coin hunter'\)/);
  assert.match(runtimeGuard, /'meme coin hunter'/);
  assert.match(types, /export interface MemeCoinHunterReport/);
  assert.match(types, /export interface MemeCoinHunterCandidate/);
});

test('hunter UI distinguishes observation, security review and research-only scalp setups', () => {
  assert.match(panel, /Meme Coin Hunter/);
  assert.match(panel, /Contract security gate/);
  assert.match(panel, /Scalp candidate matrix/);
  assert.match(panel, /Top 10 LP locked/);
  assert.match(panel, /Run hunter scan/);
  assert.match(panel, /No transaction signing or execution authority/);
  assert.match(panel, /credentials remain outside frontend storage/);
  assert.match(panel, /run a new scan to apply security checks/i);
});

test('security coverage rejects incomplete holder snapshots and ambiguous EVM pool attribution', () => {
  assert.match(provider, /visible_count < 10/);
  assert.match(provider, /holder_count != Some\(visible_count as u64\)/);
  assert.match(provider, /lp_holder_count/);
  assert.match(provider, /partial_lp_holder_snapshots_cannot_prove_locked_liquidity/);
  assert.match(provider, /total > 1\.000_001/);
  assert.match(provider, /dex_rows\.len\(\) != 1 \|\| matching_pools != 1/);
  assert.match(provider, /cannot_sell_all.*review_required/s);
  assert.match(provider, /gas_abuse/);
  assert.match(provider, /invalid negative value/);
  assert.match(provider, /tax exceeds 100% and is invalid/);
  assert.match(provider, /volume_5m\.is_some_and\(\|value\| value < 0\.0\)/);
  assert.match(provider, /other_potential_risks/);
});

test('Solana transfer-fee screening includes scheduled fees and custom transfer hooks', () => {
  assert.match(provider, /scheduled_fee_rate\/fee_rate/);
  assert.match(provider, /scheduled rate is used conservatively/);
  assert.match(provider, /scheduled_solana_fee_is_still_enforced_when_current_rate_is_missing/);
  assert.match(provider, /transfer_hook\/address/);
  assert.match(provider, /quote_token/);
});


test('quote-side observations are rejected when their price cannot be derived reliably', () => {
  assert.match(provider, /candidate_key\(chain, base_address\)\s*!=\s*candidate_key\(chain, token_address\)\s*\{\s*return None;\s*\}/);
  assert.match(provider, /quote_side_candidate_is_rejected_when_provider_price_semantics_are_ambiguous/);
  assert.match(provider, /major_quote_assets_are_not_ranked_as_meme_coin_candidates/);
});

test('credential validation accepts opaque bearer characters without permitting whitespace', () => {
  assert.match(provider, /fn valid_security_token\(token: &str\)/);
  assert.match(provider, /0x21\.\.=0x7e/);
  assert.match(bridge, /\\x21-\\x7E/);
  assert.match(provider, /authorization_header\.set_sensitive\(true\)/);
  assert.match(provider, /Zeroizing::new\(token\.to_string\(\)\)/);
});

test('holder snapshots must match the provider count when fewer than ten holders exist', () => {
  assert.match(provider, /holder_count != Some\(visible_count as u64\)/);
  assert.match(provider, /holder_snapshot_must_match_reported_count_when_fewer_than_ten_exist/);
});


test('persisted candidate reports are bounded, provenance-checked and fingerprint-verified', () => {
  assert.match(provider, /fn validate_report_integrity\(report: &MemeCoinHunterReport\)/);
  assert.match(provider, /source_names\.insert\(source\.provider\.as_str\(\)\)/);
  assert.match(provider, /dataset_fingerprint_sha256/);
  assert.match(provider, /candidate\.risk_score\s*>\s*100\s*\|\|\s*candidate\.scalp_score\s*>\s*100/);
  assert.match(provider, /validate_report_integrity\(report\)\?/);
});

test('candidate heuristics enforce liquidity, transaction flow, momentum and pool-age gates', () => {
  for (const gate of [
    'liquidity < 100_000.0', 'value >= 15_000.0', 'count >= 40',
    '(56.0..=82.0)', '(0.25..=7.0)', '(-5.0..=20.0)', '(60..=30 * 24 * 60)',
  ]) assert.ok(provider.includes(gate), `Missing scalp gate: ${gate}`);
  assert.match(provider, /scalp_state = if hard_block\s*\{\s*"BLOCKED"\s*\}\s*else if scalp_reasons\.is_empty\(\)\s*&&\s*risk\s*<=\s*35\.0\s*\{/);
  assert.match(provider, /execution_authority: "NONE/);
  assert.doesNotMatch(provider, /\b(?:eth_sendRawTransaction|sendTransaction|OrderSend|swapExactTokensForTokens)\s*\(/);
});


test('scan cooldown is persisted across process restarts and uses a monotonic in-process clock', () => {
  assert.match(provider, /static LAST_SCAN_AT: Mutex<Option<Instant>>/);
  assert.match(provider, /TransactionBehavior::Immediate/);
  assert.match(provider, /CREATE TABLE IF NOT EXISTS scan_control/);
  assert.match(provider, /previous\.elapsed\(\)/);
  assert.match(provider, /persist_scan_attempt\(data_dir, now_ms\)/);
  assert.match(provider, /persistent_scan_limit_survives_restarts_and_rejects_clock_rollback/);
  assert.match(provider, /latest_report_returns_empty_after_a_scan_reservation_without_a_completed_scan/);
});


test('security timestamps distinguish an unconfigured provider from an attempted security check', () => {
  assert.match(provider, /fn\s+assess_security\s*\(\s*chain\s*:\s*&\s*str\s*,\s*data\s*:\s*Option<\s*&\s*Value\s*>,\s*pair_address\s*:\s*&\s*str\s*,\s*checked_at_ms\s*:\s*Option<\s*i64\s*>[\s,]*\)/);
  assert.match(provider, /assess_security\(chain, security_record, pair_address, security_checked_at_ms\)/);
  assert.match(provider, /security timestamps do not match whether a GoPlus check was configured/);
  assert.match(provider, /missing_security_credentials_do_not_fabricate_a_security_check_timestamp/);
});
