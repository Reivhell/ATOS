use chrono::Utc;
use futures_util::{future::join_all, StreamExt};
use keyring::Entry;
use reqwest::{
    header::{HeaderValue, ACCEPT, AUTHORIZATION},
    redirect::Policy,
    Client, Response,
};
use rusqlite::{OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use serde_json::json;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use uuid::Uuid;
use zeroize::Zeroizing;

const SECURITY_SERVICE: &str = "dev.agentictrading.os.security-provider";
const SECURITY_ACCOUNT: &str = "goplus-access-token";
const REPORT_DB: &str = "meme-coin-hunter.sqlite3";
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
const MAX_CANDIDATES: usize = 150;
const MAX_PER_CHAIN: usize = 30;
const MIN_SCAN_INTERVAL_MS: i64 = 60_000;
const MAX_HISTORY: i64 = 100;
static LAST_SCAN_AT: Mutex<Option<Instant>> = Mutex::new(None);
static SCAN_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

struct ScanLease;
impl Drop for ScanLease {
    fn drop(&mut self) {
        SCAN_IN_FLIGHT.store(false, Ordering::Release);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemeCoinHunterSecurityStatus {
    pub credential_configured: bool,
    pub secure_store_available: bool,
    pub provider: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemeCoinHunterSourceStatus {
    pub provider: String,
    pub state: String,
    pub discovered_count: usize,
    pub received_at_ms: Option<i64>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemeCoinHunterCandidate {
    pub chain: String,
    pub token_address: String,
    pub pair_address: String,
    pub pair_url: Option<String>,
    pub symbol: String,
    pub name: String,
    pub price_usd: f64,
    pub market_cap_usd: Option<f64>,
    pub fdv_usd: Option<f64>,
    pub liquidity_usd: f64,
    pub volume_5m_usd: Option<f64>,
    pub volume_1h_usd: Option<f64>,
    pub volume_24h_usd: f64,
    pub price_change_5m_pct: Option<f64>,
    pub price_change_1h_pct: Option<f64>,
    pub price_change_24h_pct: Option<f64>,
    pub buys_5m: Option<u64>,
    pub sells_5m: Option<u64>,
    pub buys_1h: Option<u64>,
    pub sells_1h: Option<u64>,
    pub buy_pressure_5m_pct: Option<f64>,
    pub volume_velocity_5m: Option<f64>,
    pub pool_age_minutes: Option<u64>,
    pub holder_top10_pct: Option<f64>,
    pub liquidity_locked_top10_pct: Option<f64>,
    pub buy_tax_pct: Option<f64>,
    pub sell_tax_pct: Option<f64>,
    pub security_state: String,
    pub security_checked_at_ms: Option<i64>,
    pub security_flags: Vec<String>,
    pub discovery_sources: Vec<String>,
    pub risk_score: u8,
    pub scalp_score: u8,
    pub scalp_state: String,
    pub scalp_reasons: Vec<String>,
    pub received_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemeCoinHunterReport {
    pub contract_version: String,
    pub run_id: String,
    pub scanned_at_ms: i64,
    pub provider_state: String,
    pub security_provider_configured: bool,
    pub discovered_count: usize,
    pub candidate_count: usize,
    pub scalp_setup_count: usize,
    pub blocked_count: usize,
    pub dataset_fingerprint_sha256: String,
    pub sources: Vec<MemeCoinHunterSourceStatus>,
    pub candidates: Vec<MemeCoinHunterCandidate>,
    pub execution_authority: String,
    pub decision_limitations: Vec<String>,
}

#[derive(Debug, Clone)]
struct DiscoveryToken {
    chain: String,
    address: String,
    source: String,
    rank: usize,
}

#[derive(Debug, Clone)]
struct SecurityAssessment {
    state: String,
    checked_at_ms: Option<i64>,
    flags: Vec<String>,
    hard_block: bool,
    holder_top10_pct: Option<f64>,
    liquidity_locked_top10_pct: Option<f64>,
    buy_tax_pct: Option<f64>,
    sell_tax_pct: Option<f64>,
    risk_penalty: f64,
}

fn keyring_entry() -> Result<Entry, String> {
    Entry::new(SECURITY_SERVICE, SECURITY_ACCOUNT)
        .map_err(|_| "The operating-system credential store could not be initialized.".to_string())
}

pub fn security_status() -> MemeCoinHunterSecurityStatus {
    match keyring_entry().and_then(|entry| entry.get_password().map(Zeroizing::new).map_err(|error| {
        if matches!(error, keyring::Error::NoEntry) { "not_configured".to_string() }
        else { "secure_store_unavailable".to_string() }
    })) {
        Ok(_secret) => MemeCoinHunterSecurityStatus {
            credential_configured: true, secure_store_available: true, provider: "GoPlus Security".into(),
            note: "Bearer credential is stored in the operating-system credential vault.".into(),
        },
        Err(error) if error == "not_configured" => MemeCoinHunterSecurityStatus {
            credential_configured: false, secure_store_available: true, provider: "GoPlus Security".into(),
            note: "Security verification is not configured; candidates cannot become scalp setups.".into(),
        },
        Err(_) => MemeCoinHunterSecurityStatus {
            credential_configured: false, secure_store_available: false, provider: "GoPlus Security".into(),
            note: "The operating-system credential vault is unavailable; security verification fails closed.".into(),
        },
    }
}

fn valid_security_token(token: &str) -> bool {
    (32..=4096).contains(&token.len())
        && token
            .as_bytes()
            .iter()
            .all(|byte| (0x21..=0x7e).contains(byte))
}

pub fn save_security_token(token: String) -> Result<MemeCoinHunterSecurityStatus, String> {
    let secret = Zeroizing::new(token);
    if !valid_security_token(secret.as_str()) {
        return Err("GoPlus access token must contain 32–4096 printable ASCII characters without whitespace.".into());
    }
    keyring_entry()?.set_password(secret.as_str())
        .map_err(|_| "GoPlus token could not be saved to the operating-system credential vault. No plaintext fallback was used.".to_string())?;
    Ok(security_status())
}

pub fn remove_security_token() -> Result<MemeCoinHunterSecurityStatus, String> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(security_status()),
        Err(_) => Err(
            "GoPlus token could not be removed from the operating-system credential vault.".into(),
        ),
    }
}

fn load_security_token() -> Result<Option<Zeroizing<String>>, String> {
    match keyring_entry()?.get_password() {
        Ok(token) => Ok(Some(Zeroizing::new(token))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("The operating-system credential vault is unavailable; token security checks are blocked.".into()),
    }
}

fn persist_scan_attempt(data_dir: &Path, now_ms: i64) -> Result<(), String> {
    if now_ms <= 0 {
        return Err("System clock is invalid; Meme Coin Hunter scan is blocked.".into());
    }
    fs::create_dir_all(data_dir)
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    let connection = rusqlite::Connection::open(data_dir.join(REPORT_DB))
        .map_err(|_| "Persistent scan limiter could not open its control database.".to_string())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|_| {
            "Persistent scan limiter could not configure its database timeout.".to_string()
        })?;
    connection
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
        .map_err(|_| {
            "Persistent scan limiter could not enable durable database commits.".to_string()
        })?;
    let mut connection = connection;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| "Persistent scan limiter could not acquire its database lock.".to_string())?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS scan_runs (run_id TEXT PRIMARY KEY, scanned_at_ms INTEGER NOT NULL, dataset_fingerprint_sha256 TEXT NOT NULL, report_json TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS idx_scan_runs_time ON scan_runs(scanned_at_ms DESC);
         CREATE TABLE IF NOT EXISTS scan_control (singleton INTEGER PRIMARY KEY CHECK (singleton = 1), last_attempt_ms INTEGER NOT NULL);"
    ).map_err(|_| "Persistent scan limiter schema could not be initialized.".to_string())?;
    let previous: Option<i64> = transaction
        .query_row(
            "SELECT last_attempt_ms FROM scan_control WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "Persistent scan limiter state could not be read.".to_string())?;
    if let Some(previous) = previous {
        if previous <= 0 || now_ms < previous {
            return Err("System clock moved backwards relative to the last scan; refresh is blocked until the clock is corrected.".into());
        }
        if now_ms.saturating_sub(previous) < MIN_SCAN_INTERVAL_MS {
            return Err("Meme Coin Hunter refresh is rate-limited to one scan per 60 seconds, including across application restarts.".into());
        }
    }
    transaction.execute(
        "INSERT INTO scan_control (singleton, last_attempt_ms) VALUES (1, ?1) ON CONFLICT(singleton) DO UPDATE SET last_attempt_ms = excluded.last_attempt_ms",
        rusqlite::params![now_ms],
    ).map_err(|_| "Persistent scan limiter could not commit the scan reservation.".to_string())?;
    transaction
        .commit()
        .map_err(|_| "Persistent scan limiter transaction could not commit.".to_string())
}

fn claim_scan_slot(now_ms: i64, data_dir: &Path) -> Result<ScanLease, String> {
    SCAN_IN_FLIGHT
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| {
            "Meme Coin Hunter scan is already running; concurrent scans are rejected.".to_string()
        })?;
    let lease = ScanLease;
    let mut last_scan = LAST_SCAN_AT.lock().map_err(|_| {
        "In-process scan limiter state is unavailable; scan is blocked.".to_string()
    })?;
    if last_scan.as_ref().is_some_and(|previous| {
        previous.elapsed() < Duration::from_millis(MIN_SCAN_INTERVAL_MS as u64)
    }) {
        return Err("Meme Coin Hunter refresh is rate-limited to one scan per 60 seconds.".into());
    }
    persist_scan_attempt(data_dir, now_ms)?;
    *last_scan = Some(Instant::now());
    Ok(lease)
}

fn provider_client() -> Result<Client, String> {
    Client::builder()
        .redirect(Policy::none())
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(9))
        .user_agent(
            "AgenticTradingOS/0.11.0 (meme-coin-hunter; read-only; no-transaction-authority)",
        )
        .build()
        .map_err(|_| "Unable to initialize the bounded market-data client.".into())
}

async fn bounded_json(response: Response) -> Result<Value, String> {
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES as u64)
    {
        return Err("Provider response exceeds the 4 MiB safety limit.".into());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| "Provider response stream failed.".to_string())?;
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err("Provider response exceeded the 4 MiB safety limit.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "Provider returned malformed JSON.".to_string())
}

async fn get_json(client: &Client, url: &str, gecko: bool) -> Result<Value, String> {
    let request = client.get(url).header(
        ACCEPT,
        if gecko {
            HeaderValue::from_static("application/json;version=20230203")
        } else {
            HeaderValue::from_static("application/json")
        },
    );
    let response = request
        .send()
        .await
        .map_err(|_| "Provider request failed before a response was received.".to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Provider returned HTTP {}.", status.as_u16()));
    }
    bounded_json(response).await
}

fn supported_dex_chain(chain: &str) -> bool {
    matches!(
        chain,
        "solana" | "ethereum" | "base" | "bsc" | "arbitrum" | "polygon" | "optimism" | "avalanche"
    )
}

fn normalize_dex_chain(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    let normalized = match value.as_str() {
        "eth" | "ethereum" => "ethereum",
        "sol" | "solana" => "solana",
        "base" => "base",
        "bsc" | "bnb" => "bsc",
        "arbitrum" | "arbitrum-one" => "arbitrum",
        "polygon" | "polygon_pos" => "polygon",
        "optimism" => "optimism",
        "avax" | "avalanche" => "avalanche",
        _ => return None,
    };
    Some(normalized.into())
}

fn valid_solana_address(address: &str) -> bool {
    if !(32..=44).contains(&address.len()) || !address.is_ascii() {
        return false;
    }
    const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut decoded = [0u8; 32];
    for character in address.bytes() {
        let Some(value) = BASE58.iter().position(|candidate| *candidate == character) else {
            return false;
        };
        let mut carry = value as u32;
        for byte in decoded.iter_mut().rev() {
            let accumulated = (*byte as u32) * 58 + carry;
            *byte = accumulated as u8;
            carry = accumulated >> 8;
        }
        if carry != 0 {
            return false;
        }
    }
    let leading_zeroes = address.bytes().take_while(|byte| *byte == b'1').count();
    let significant_length = decoded
        .iter()
        .position(|byte| *byte != 0)
        .map_or(0, |index| 32 - index);
    leading_zeroes + significant_length == 32
}

fn valid_token_address(chain: &str, address: &str) -> bool {
    if !(20..=64).contains(&address.len()) || !address.is_ascii() {
        return false;
    }
    if chain == "solana" {
        valid_solana_address(address)
    } else {
        address.len() == 42
            && address.starts_with("0x")
            && address[2..]
                .bytes()
                .all(|character| character.is_ascii_hexdigit())
            && address[2..].bytes().any(|character| character != b'0')
    }
}

fn identity(chain: &str, address: &str) -> String {
    if chain == "solana" {
        format!("{chain}:{address}")
    } else {
        format!("{chain}:{}", address.to_ascii_lowercase())
    }
}

fn is_major_quote_asset(chain: &str, address: &str) -> bool {
    if chain == "solana" {
        return matches!(
            address,
            "So11111111111111111111111111111111111111112"
                | "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
                | "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"
        );
    }
    let address = address.to_ascii_lowercase();
    let known: &[&str] = match chain {
        "ethereum" => &[
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
            "0xdac17f958d2ee523a2206206994597c13d831ec7",
            "0x6b175474e89094c44da98b954eedeac495271d0f",
        ],
        "base" => &[
            "0x4200000000000000000000000000000000000006",
            "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        ],
        "optimism" => &[
            "0x4200000000000000000000000000000000000006",
            "0x0b2c639c533813f4aa9d7837caf62653d097ff85",
        ],
        "bsc" => &[
            "0xbb4cdb9cbd36b01bd1cbaebf2de08d9173bc095c",
            "0x55d398326f99059ff775485246999027b3197955",
            "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d",
        ],
        "arbitrum" => &[
            "0x82af49447d8a07e3bd95bd0d56f35241523fbab1",
            "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
            "0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9",
            "0xda10009cbd5d07dd0cecc66161fc93d7c9000da1",
        ],
        "polygon" => &[
            "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",
            "0xc2132d05d31c914a87c6611c10748aacba0a1d3",
            "0x7ceb23fd6bc0add59e62ac25578270cff1b9f619",
        ],
        "avalanche" => &[
            "0xb31f66aa3c1e785363f0875a1b74e27b85fd66c7",
            "0xa7d7079b0fead91f3e65f86e8915cb59c1a4c664",
            "0xb97ef9ef8734c71904d8002f8b6bc66dd9c48a6e",
        ],
        _ => &[],
    };
    known.iter().any(|known_address| address == *known_address)
}

fn push_discovery(
    rows: &mut BTreeMap<String, DiscoveryToken>,
    chain: &str,
    address: &str,
    source: &str,
    next_rank: &mut usize,
) -> bool {
    if !supported_dex_chain(chain)
        || !valid_token_address(chain, address)
        || is_major_quote_asset(chain, address)
    {
        return false;
    }
    let key = identity(chain, address);
    if let Some(existing) = rows.get_mut(&key) {
        if !existing.source.split(" | ").any(|value| value == source) {
            existing.source.push_str(" | ");
            existing.source.push_str(source);
        }
    } else {
        rows.insert(
            key,
            DiscoveryToken {
                chain: chain.into(),
                address: address.into(),
                source: source.into(),
                rank: *next_rank,
            },
        );
        *next_rank = (*next_rank).saturating_add(1);
    }
    true
}

fn split_gecko_resource_id(value: &str) -> Option<(String, &str)> {
    for network in [
        "polygon_pos",
        "arbitrum",
        "optimism",
        "avalanche",
        "ethereum",
        "solana",
        "polygon",
        "base",
        "bsc",
        "eth",
        "sol",
        "avax",
    ] {
        let Some(remainder) = value.strip_prefix(network) else {
            continue;
        };
        let Some(address) = remainder.strip_prefix('_') else {
            continue;
        };
        let chain = normalize_dex_chain(network)?;
        return Some((chain, address));
    }
    None
}

fn parse_dex_discovery(
    payload: &Value,
    source: &str,
    rows: &mut BTreeMap<String, DiscoveryToken>,
    next_rank: &mut usize,
) -> usize {
    let Some(items) = payload.as_array() else {
        return 0;
    };
    let mut seen = BTreeSet::new();
    let mut added = 0;
    for item in items {
        let chain = item
            .get("chainId")
            .and_then(Value::as_str)
            .and_then(normalize_dex_chain);
        let address = item.get("tokenAddress").and_then(Value::as_str);
        if let (Some(chain), Some(address)) = (chain, address) {
            if push_discovery(rows, &chain, address, source, next_rank)
                && seen.insert(identity(&chain, address))
            {
                added += 1;
            }
        }
    }
    added
}

fn parse_gecko_discovery(
    payload: &Value,
    source: &str,
    rows: &mut BTreeMap<String, DiscoveryToken>,
    next_rank: &mut usize,
) -> usize {
    let Some(items) = payload.get("data").and_then(Value::as_array) else {
        return 0;
    };
    let mut seen = BTreeSet::new();
    let mut added = 0;
    for item in items {
        let pool_id = item.get("id").and_then(Value::as_str).unwrap_or_default();
        let Some((chain, _)) = split_gecko_resource_id(pool_id) else {
            continue;
        };
        for relationship in ["base_token", "quote_token"] {
            let token_id = item
                .pointer(&format!("/relationships/{relationship}/data/id"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some((token_chain, address)) = split_gecko_resource_id(token_id) else {
                continue;
            };
            if token_chain != chain {
                continue;
            }
            if push_discovery(rows, &chain, address, source, next_rank)
                && seen.insert(identity(&chain, address))
            {
                added += 1;
            }
        }
    }
    added
}

fn finite_number(value: Option<&Value>) -> Option<f64> {
    let parsed = match value? {
        Value::String(text) => text.parse::<f64>().ok(),
        Value::Number(number) => number.as_f64(),
        _ => None,
    }?;
    parsed.is_finite().then_some(parsed)
}

fn unsigned(value: Option<&Value>) -> Option<u64> {
    let parsed = match value? {
        Value::String(text) => text.parse::<u64>().ok(),
        Value::Number(number) => number.as_u64(),
        _ => None,
    }?;
    Some(parsed)
}

fn boolean_flag(value: Option<&Value>) -> Option<bool> {
    match value? {
        Value::String(text) if text == "1" => Some(true),
        Value::String(text) if text == "0" => Some(false),
        Value::Number(number) if number.as_i64() == Some(1) => Some(true),
        Value::Number(number) if number.as_i64() == Some(0) => Some(false),
        Value::Bool(flag) => Some(*flag),
        _ => None,
    }
}

fn security_chain_id(chain: &str) -> Option<&'static str> {
    match chain {
        "ethereum" => Some("1"),
        "bsc" => Some("56"),
        "base" => Some("8453"),
        "arbitrum" => Some("42161"),
        "polygon" => Some("137"),
        "optimism" => Some("10"),
        "avalanche" => Some("43114"),
        "solana" => Some("solana"),
        _ => None,
    }
}

fn find_security_record<'a>(result: &'a Value, chain: &str, address: &str) -> Option<&'a Value> {
    let records = result.as_object()?;
    if chain == "solana" {
        records.get(address)
    } else {
        records
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(address))
            .map(|(_, value)| value)
    }
}

async fn fetch_security_for_chain(
    client: Client,
    token: Zeroizing<String>,
    chain: String,
    addresses: Vec<String>,
) -> Result<(String, Value), String> {
    let chain_id = security_chain_id(&chain)
        .ok_or_else(|| "Security provider does not support this chain.".to_string())?;
    let endpoint = if chain == "solana" {
        "https://api.gopluslabs.io/api/v1/solana/token_security".to_string()
    } else {
        format!("https://api.gopluslabs.io/api/v1/token_security/{chain_id}")
    };
    let address_list = if chain == "solana" {
        addresses.join(",")
    } else {
        addresses
            .iter()
            .map(|address| address.to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(",")
    };
    let authorization = Zeroizing::new(format!("Bearer {}", token.as_str()));
    let mut authorization_header = HeaderValue::from_bytes(authorization.as_bytes())
        .map_err(|_| "GoPlus authorization token could not be encoded safely.".to_string())?;
    authorization_header.set_sensitive(true);
    let response = client
        .get(endpoint)
        .query(&[("contract_addresses", address_list)])
        .header(AUTHORIZATION, authorization_header)
        .send()
        .await
        .map_err(|_| format!("GoPlus security request failed for {chain}."))?;
    if !response.status().is_success() {
        return Err(format!(
            "GoPlus security returned HTTP {} for {chain}.",
            response.status().as_u16()
        ));
    }
    let payload = bounded_json(response).await?;
    if unsigned(payload.get("code")) != Some(1) {
        return Err(format!(
            "GoPlus security did not return a complete-success response for {chain}."
        ));
    }
    Ok((chain, payload.get("result").cloned().unwrap_or(Value::Null)))
}

fn percent_from_holder_array(
    value: Option<&Value>,
    chain: &str,
    holder_count: Option<u64>,
) -> Option<f64> {
    let rows = value?.as_array()?;
    let visible_count = rows.len().min(10);
    if visible_count == 0 || (visible_count < 10 && holder_count != Some(visible_count as u64)) {
        return None;
    }
    let mut total = 0.0;
    let mut identities = BTreeSet::new();
    for holder in rows.iter().take(10) {
        let address = holder
            .get("address")
            .or_else(|| holder.get("token_account"))?
            .as_str()?;
        if address.is_empty() {
            return None;
        }
        let key = if chain == "solana" {
            address.to_owned()
        } else {
            address.to_ascii_lowercase()
        };
        if !identities.insert(key) {
            return None;
        }
        let percent = finite_number(holder.get("percent"))?;
        if !(0.0..=1.0).contains(&percent) {
            return None;
        }
        total += percent;
    }
    if total > 1.000_001 {
        return None;
    }
    Some((total * 100.0).min(100.0))
}

fn locked_holder_percent(
    value: Option<&Value>,
    chain: &str,
    holder_count: Option<u64>,
) -> Option<f64> {
    let rows = value?.as_array()?;
    let visible_count = rows.len().min(10);
    if visible_count == 0 || (visible_count < 10 && holder_count != Some(visible_count as u64)) {
        return None;
    }
    let mut total = 0.0;
    let mut identities = BTreeSet::new();
    for holder in rows.iter().take(10) {
        let address = holder
            .get("address")
            .or_else(|| holder.get("token_account"))?
            .as_str()?;
        if address.is_empty() {
            return None;
        }
        let key = if chain == "solana" {
            address.to_owned()
        } else {
            address.to_ascii_lowercase()
        };
        if !identities.insert(key) {
            return None;
        }
        let percent = finite_number(holder.get("percent"))?;
        if !(0.0..=1.0).contains(&percent) {
            return None;
        }
        if boolean_flag(holder.get("is_locked"))? {
            total += percent;
        }
    }
    if total > 1.000_001 {
        return None;
    }
    Some((total * 100.0).min(100.0))
}

fn same_pool_address(chain: &str, left: &str, right: &str) -> bool {
    if chain == "solana" {
        left == right
    } else {
        left.eq_ignore_ascii_case(right)
    }
}

fn pool_locked_percent(chain: &str, data: &Value, pair_address: &str) -> Option<f64> {
    let dex_rows = data.get("dex")?.as_array()?;
    if chain == "solana" {
        let pool = dex_rows.iter().find(|dex| {
            dex.get("id")
                .and_then(Value::as_str)
                .is_some_and(|value| same_pool_address(chain, value, pair_address))
        })?;
        locked_holder_percent(
            pool.get("lp_holders"),
            chain,
            unsigned(pool.get("lp_holder_count")),
        )
    } else {
        let matching_pools = dex_rows
            .iter()
            .filter(|dex| {
                dex.get("pair")
                    .and_then(Value::as_str)
                    .is_some_and(|value| same_pool_address(chain, value, pair_address))
            })
            .count();
        if dex_rows.len() != 1 || matching_pools != 1 {
            return None;
        }
        locked_holder_percent(
            data.get("lp_holders"),
            chain,
            unsigned(data.get("lp_holder_count")),
        )
    }
}

fn assess_security(
    chain: &str,
    data: Option<&Value>,
    pair_address: &str,
    checked_at_ms: Option<i64>,
) -> SecurityAssessment {
    let Some(data) = data else {
        return SecurityAssessment {
            state: "unavailable".into(),
            checked_at_ms,
            flags: vec![
                "GoPlus returned no security record for this token; safety is unknown".into(),
            ],
            hard_block: false,
            holder_top10_pct: None,
            liquidity_locked_top10_pct: None,
            buy_tax_pct: None,
            sell_tax_pct: None,
            risk_penalty: 40.0,
        };
    };
    let mut flags = Vec::new();
    let mut hard_block = false;
    let mut risk_penalty: f64 = 0.0;
    let mut mandatory_known = true;
    let mut review_required = false;
    macro_rules! check_flag_inline {
        ($path:expr, $label:expr, $critical:expr, $penalty:expr) => {
            match boolean_flag(data.pointer($path)) {
                Some(true) => {
                    flags.push(format!("{}: enabled or detected", $label));
                    risk_penalty += $penalty;
                    if $critical {
                        hard_block = true;
                    }
                }
                Some(false) => {}
                None => {
                    mandatory_known = false;
                    flags.push(format!("{}: provider could not verify", $label));
                }
            }
        };
    }

    let (mut buy_tax_pct, mut sell_tax_pct, holder_top10_pct, liquidity_locked_top10_pct);
    if chain == "solana" {
        check_flag_inline!("/non_transferable", "Non-transferable token", true, 100.0);
        check_flag_inline!("/mintable/status", "Mint authority", true, 22.0);
        check_flag_inline!("/freezable/status", "Freeze authority", true, 22.0);
        check_flag_inline!("/closable/status", "Token closure authority", true, 25.0);
        check_flag_inline!(
            "/transfer_hook/malicious_address",
            "Malicious transfer hook",
            true,
            100.0
        );
        if data.pointer("/creator/malicious_address").is_some() {
            check_flag_inline!(
                "/creator/malicious_address",
                "Malicious creator address",
                true,
                100.0
            );
        } else {
            check_flag_inline!(
                "/creator/0/malicious_address",
                "Malicious creator address",
                true,
                100.0
            );
        }
        match boolean_flag(data.pointer("/metadata_mutable/status")) {
            Some(true) => {
                flags.push("Metadata is mutable".into());
                risk_penalty += 10.0;
                mandatory_known = false;
            }
            Some(false) => {}
            None => {
                flags.push("Metadata mutability could not be verified".into());
                mandatory_known = false;
            }
        }
        check_flag_inline!(
            "/balance_mutable_authority/status",
            "Balance mutation authority",
            true,
            100.0
        );
        if boolean_flag(data.pointer("/transfer_fee_upgradable/status")) == Some(true) {
            flags.push("Transfer fee can be upgraded".into());
            risk_penalty += 25.0;
            mandatory_known = false;
        } else if boolean_flag(data.pointer("/transfer_fee_upgradable/status")).is_none() {
            mandatory_known = false;
            flags.push("Transfer-fee upgrade status could not be verified".into());
        }
        match boolean_flag(data.pointer("/default_account_state_upgradable/status")) {
            Some(true) => {
                flags.push("Default account state can be upgraded".into());
                risk_penalty += 20.0;
                mandatory_known = false;
            }
            Some(false) => {}
            None => {
                flags.push("Default-account-state upgrade status could not be verified".into());
                mandatory_known = false;
            }
        }
        match boolean_flag(data.pointer("/transfer_hook_upgradable/status")) {
            Some(true) => {
                flags.push("Transfer hook can be upgraded".into());
                risk_penalty += 20.0;
                mandatory_known = false;
            }
            Some(false) => {}
            None => {
                flags.push("Transfer-hook upgrade status could not be verified".into());
                mandatory_known = false;
            }
        }
        let default_account_state = data.get("default_account_state").and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_i64().map(|number| number.to_string()))
        });
        match default_account_state.as_deref() {
            Some("1") => {}
            Some("0") | Some("2") => {
                flags.push("Token account state is not safely initialized or is frozen".into());
                hard_block = true;
            }
            _ => {
                mandatory_known = false;
                flags.push("Default account state could not be verified".into());
            }
        }
        let current_fee = finite_number(data.pointer("/transfer_fee/current_fee_rate/fee_rate"));
        let scheduled_fee =
            finite_number(data.pointer("/transfer_fee/scheduled_fee_rate/fee_rate"));
        let invalid_fee = current_fee.is_some_and(|value| value < 0.0)
            || scheduled_fee.is_some_and(|value| value < 0.0);
        if invalid_fee {
            flags.push("Provider returned an invalid negative transfer-fee rate".into());
            risk_penalty += 25.0;
            mandatory_known = false;
        }
        let transfer_fee = match (
            current_fee.filter(|value| *value >= 0.0),
            scheduled_fee.filter(|value| *value >= 0.0),
        ) {
            (Some(current), Some(scheduled)) => Some(current.max(scheduled)),
            (Some(current), None) => Some(current),
            (None, Some(scheduled)) => {
                flags.push("Current transfer-fee rate is unavailable; scheduled rate is used conservatively".into());
                mandatory_known = false;
                Some(scheduled)
            }
            (None, None) => None,
        };
        buy_tax_pct = transfer_fee.map(|value| value / 100.0);
        sell_tax_pct = buy_tax_pct;
        if let Some(tax) = buy_tax_pct {
            if scheduled_fee.is_some_and(|value| current_fee.is_some_and(|current| value > current))
            {
                flags.push("A higher scheduled transfer fee is configured".into());
                risk_penalty += 20.0;
                mandatory_known = false;
            }
            if tax > 5.0 {
                flags.push(format!("Current or scheduled transfer fee is {tax:.2}%"));
                risk_penalty += 15.0;
                mandatory_known = false;
            }
            if tax >= 10.0 {
                hard_block = true;
            }
        } else {
            mandatory_known = false;
            flags.push("Current transfer fee could not be verified".into());
        }
        if data
            .pointer("/transfer_hook/address")
            .and_then(Value::as_str)
            .is_some_and(|address| !address.is_empty())
        {
            flags.push("Custom transfer hook is configured and requires manual review".into());
            mandatory_known = false;
        }
        holder_top10_pct = percent_from_holder_array(
            data.get("holders"),
            chain,
            unsigned(data.get("holder_count")),
        );
        liquidity_locked_top10_pct = pool_locked_percent(chain, data, pair_address);
    } else {
        match boolean_flag(data.get("is_open_source")) {
            Some(true) => {}
            Some(false) => {
                flags.push("Contract is closed source; hidden behavior cannot be ruled out".into());
                risk_penalty += 30.0;
                mandatory_known = false;
            }
            None => {
                flags.push("Open-source status could not be verified".into());
                mandatory_known = false;
            }
        }
        check_flag_inline!("/is_proxy", "Proxy contract", false, 20.0);
        if boolean_flag(data.get("is_proxy")) == Some(true) {
            review_required = true;
        }
        check_flag_inline!("/is_honeypot", "Honeypot detection", true, 100.0);
        check_flag_inline!("/cannot_buy", "Buy restriction detection", true, 100.0);
        check_flag_inline!("/cannot_sell_all", "Partial-sell restriction", false, 35.0);
        if boolean_flag(data.get("cannot_sell_all")) == Some(true) {
            review_required = true;
        }
        check_flag_inline!("/is_mintable", "Mint function", true, 25.0);
        check_flag_inline!(
            "/can_take_back_ownership",
            "Ownership can be reclaimed",
            true,
            35.0
        );
        check_flag_inline!("/hidden_owner", "Hidden owner", true, 25.0);
        check_flag_inline!(
            "/owner_change_balance",
            "Owner can change balances",
            true,
            40.0
        );
        check_flag_inline!("/selfdestruct", "Self-destruct function", true, 40.0);
        check_flag_inline!("/slippage_modifiable", "Modifiable tax", true, 30.0);
        check_flag_inline!("/transfer_pausable", "Pausable transfers", true, 30.0);
        check_flag_inline!("/is_blacklisted", "Blacklist function", true, 25.0);
        check_flag_inline!("/is_whitelisted", "Whitelist restriction", false, 15.0);
        if boolean_flag(data.get("is_whitelisted")) == Some(true) {
            review_required = true;
        }
        check_flag_inline!("/trading_cooldown", "Trading cooldown", false, 15.0);
        if boolean_flag(data.get("trading_cooldown")) == Some(true) {
            review_required = true;
        }
        check_flag_inline!("/is_anti_whale", "Anti-whale restriction", false, 12.0);
        if boolean_flag(data.get("is_anti_whale")) == Some(true) {
            review_required = true;
        }
        check_flag_inline!(
            "/anti_whale_modifiable",
            "Modifiable anti-whale restriction",
            false,
            20.0
        );
        if boolean_flag(data.get("anti_whale_modifiable")) == Some(true) {
            review_required = true;
        }
        check_flag_inline!(
            "/personal_slippage_modifiable",
            "Per-address modifiable tax",
            true,
            35.0
        );
        check_flag_inline!("/is_airdrop_scam", "Airdrop scam detection", true, 100.0);
        if data.get("fake_token").is_some() {
            check_flag_inline!(
                "/fake_token/value",
                "Counterfeit-token detection",
                true,
                100.0
            );
        }
        if boolean_flag(data.get("external_call")) == Some(true) {
            flags.push("Contract performs external calls during token operations".into());
            risk_penalty += 15.0;
            review_required = true;
        }
        if boolean_flag(data.get("gas_abuse")) == Some(true) {
            flags.push("GoPlus detected gas-abuse behavior".into());
            risk_penalty += 100.0;
            hard_block = true;
        }
        if data
            .get("other_potential_risks")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        {
            flags.push("GoPlus reported additional potential contract risks".into());
            risk_penalty += 25.0;
            review_required = true;
        }
        mandatory_known &= !review_required;
        buy_tax_pct = finite_number(data.get("buy_tax")).map(|value| value * 100.0);
        sell_tax_pct = finite_number(data.get("sell_tax")).map(|value| value * 100.0);
        for (label, tax) in [("Buy", &mut buy_tax_pct), ("Sell", &mut sell_tax_pct)] {
            match *tax {
                Some(value) if value < 0.0 => {
                    flags.push(format!("{label} tax is an invalid negative value"));
                    risk_penalty += 25.0;
                    mandatory_known = false;
                    *tax = None;
                }
                Some(value) if value > 100.0 => {
                    flags.push(format!("{label} tax exceeds 100% and is invalid"));
                    risk_penalty += 100.0;
                    mandatory_known = false;
                    hard_block = true;
                    *tax = None;
                }
                Some(value) if value >= 99.0 => {
                    flags.push(format!(
                        "{label} tax is effectively 100% or token is not tradable"
                    ));
                    hard_block = true;
                }
                Some(value) if value > 10.0 => {
                    flags.push(format!("{label} tax is {value:.2}%"));
                    risk_penalty += 30.0;
                    mandatory_known = false;
                }
                Some(value) if value > 5.0 => {
                    flags.push(format!("{label} tax is {value:.2}%"));
                    risk_penalty += 15.0;
                    mandatory_known = false;
                }
                Some(_) => {}
                None => {
                    mandatory_known = false;
                    flags.push(format!("{label} tax could not be verified"));
                }
            }
        }
        holder_top10_pct = percent_from_holder_array(
            data.get("holders"),
            chain,
            unsigned(data.get("holder_count")),
        );
        liquidity_locked_top10_pct = pool_locked_percent(chain, data, pair_address);
    }
    if holder_top10_pct.is_none() {
        mandatory_known = false;
        flags.push("Top-ten holder concentration is unavailable".into());
    } else if holder_top10_pct.unwrap_or(0.0) >= 70.0 {
        flags.push("Top-ten holders control at least 70% of token supply".into());
        hard_block = true;
        risk_penalty += 40.0;
    } else if holder_top10_pct.unwrap_or(0.0) >= 40.0 {
        flags.push("Top-ten holder concentration exceeds 40%".into());
        risk_penalty += 20.0;
        mandatory_known = false;
    }
    if liquidity_locked_top10_pct.is_none() {
        mandatory_known = false;
        flags.push("Liquidity-lock evidence for the selected pool is unavailable".into());
    } else if liquidity_locked_top10_pct.unwrap_or(0.0) < 50.0 {
        flags.push("Less than 50% of selected-pool LP ownership is locked".into());
        risk_penalty += 18.0;
        mandatory_known = false;
    }
    if flags.is_empty() {
        flags.push(
            "No configured GoPlus risk flags detected; this is not a safety guarantee".into(),
        );
    }
    SecurityAssessment {
        state: if hard_block {
            "blocked"
        } else if mandatory_known {
            "verified"
        } else {
            "partial"
        }
        .into(),
        checked_at_ms,
        flags,
        hard_block,
        holder_top10_pct,
        liquidity_locked_top10_pct,
        buy_tax_pct,
        sell_tax_pct,
        risk_penalty: risk_penalty.clamp(0.0, 100.0),
    }
}

fn candidate_key(chain: &str, address: &str) -> String {
    identity(chain, address)
}

fn score_candidate(
    chain: &str,
    token_address: &str,
    pair: &Value,
    discovery_sources: String,
    security: SecurityAssessment,
    now_ms: i64,
) -> Option<MemeCoinHunterCandidate> {
    let base = pair.get("baseToken")?;
    let response_chain = normalize_dex_chain(pair.get("chainId")?.as_str()?)?;
    if response_chain != chain {
        return None;
    }
    let base_address = base.get("address")?.as_str()?;
    if candidate_key(chain, base_address) != candidate_key(chain, token_address) {
        return None;
    }
    let token = base;
    let price_usd = finite_number(pair.get("priceUsd"))?;
    if price_usd <= 0.0 {
        return None;
    }
    let symbol = token
        .get("symbol")
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN")
        .trim();
    let name = token
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(symbol)
        .trim();
    if symbol.is_empty() || symbol.len() > 32 || name.is_empty() || name.len() > 100 {
        return None;
    }
    let liquidity = finite_number(pair.pointer("/liquidity/usd"))?;
    let volume_24h = finite_number(pair.pointer("/volume/h24"))?;
    if liquidity < 0.0 || volume_24h < 0.0 {
        return None;
    }
    let volume_5m = finite_number(pair.pointer("/volume/m5"));
    let volume_1h = finite_number(pair.pointer("/volume/h1"));
    if volume_5m.is_some_and(|value| value < 0.0) || volume_1h.is_some_and(|value| value < 0.0) {
        return None;
    }
    let raw_change_5m = finite_number(pair.pointer("/priceChange/m5"));
    let raw_change_1h = finite_number(pair.pointer("/priceChange/h1"));
    let raw_change_24h = finite_number(pair.pointer("/priceChange/h24"));
    let change_5m = raw_change_5m;
    let change_1h = raw_change_1h;
    let change_24h = raw_change_24h;
    let buys_5m = unsigned(pair.pointer("/txns/m5/buys"));
    let sells_5m = unsigned(pair.pointer("/txns/m5/sells"));
    let buys_1h = unsigned(pair.pointer("/txns/h1/buys"));
    let sells_1h = unsigned(pair.pointer("/txns/h1/sells"));
    let buy_pressure = match (buys_5m, sells_5m) {
        (Some(buy), Some(sell)) if buy.saturating_add(sell) > 0 => {
            Some(buy as f64 / buy.saturating_add(sell) as f64 * 100.0)
        }
        _ => None,
    };
    let volume_velocity = match (volume_5m, volume_1h) {
        (Some(short), Some(hour)) if hour > 0.0 => Some(short / (hour / 12.0)),
        _ => None,
    };
    let created = pair
        .get("pairCreatedAt")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0);
    let pool_age =
        created.map(|created_at| now_ms.saturating_sub(created_at).max(0) as u64 / 60_000);
    let market_cap = finite_number(pair.get("marketCap")).filter(|value| *value >= 0.0);
    let fdv = finite_number(pair.get("fdv")).filter(|value| *value >= 0.0);
    let pair_address = pair.get("pairAddress").and_then(Value::as_str)?;
    if !valid_token_address(chain, pair_address) {
        return None;
    }
    let pair_url = pair
        .get("url")
        .and_then(Value::as_str)
        .filter(|url| url.starts_with("https://dexscreener.com/") && url.len() <= 500)
        .map(str::to_owned);

    let mut risk = 15.0;
    let mut reasons = Vec::new();
    if liquidity < 25_000.0 {
        risk += 30.0;
        reasons.push("Pool liquidity below $25k".into());
    } else if liquidity < 100_000.0 {
        risk += 18.0;
        reasons.push("Pool liquidity below $100k".into());
    } else if liquidity < 250_000.0 {
        risk += 8.0;
    }
    if volume_5m.is_none() || volume_1h.is_none() {
        risk += 15.0;
        reasons.push("Short-window volume evidence is missing".into());
    }
    if buys_5m.is_none() || sells_5m.is_none() {
        risk += 12.0;
        reasons.push("Five-minute buy/sell counts are missing".into());
    }
    if change_5m.is_none() || change_1h.is_none() {
        risk += 10.0;
        reasons.push("Short-window price-change evidence is missing".into());
    }
    match pool_age {
        Some(age) if age < 60 => {
            risk += 20.0;
            reasons.push("Pool is younger than one hour".into());
        }
        Some(age) if age > 30 * 24 * 60 => {
            risk += 8.0;
            reasons.push("Pool is older than the configured scalp window".into());
        }
        None => {
            risk += 12.0;
            reasons.push("Pool creation timestamp is unavailable".into());
        }
        _ => {}
    }
    if liquidity > 0.0 && volume_24h / liquidity > 20.0 {
        risk += 14.0;
        reasons.push("24h turnover/liquidity ratio is extreme".into());
    }
    if volume_velocity.is_some_and(|value| value > 12.0) {
        risk += 8.0;
        reasons.push("Five-minute volume velocity is unusually high".into());
    }
    risk += security.risk_penalty;
    if security.state != "verified" {
        risk += 25.0;
        reasons.push("Contract and liquidity security evidence is incomplete".into());
    }
    risk = risk.clamp(0.0, 100.0);

    let mut scalp_reasons = Vec::new();
    let mut hard_block = security.hard_block;
    if liquidity < 100_000.0 {
        scalp_reasons.push("Minimum liquidity is $100k".into());
    }
    if !volume_5m.is_some_and(|value| value >= 15_000.0) {
        scalp_reasons.push("Five-minute quote volume must be at least $15k".into());
    }
    if !volume_velocity.is_some_and(|value| (1.25..=12.0).contains(&value)) {
        scalp_reasons.push("Five-minute volume velocity is outside 1.25x–12x".into());
    }
    let transaction_count = buys_5m
        .zip(sells_5m)
        .map(|(buy, sell)| buy.saturating_add(sell));
    if !transaction_count.is_some_and(|count| count >= 40) {
        scalp_reasons.push("At least 40 five-minute transactions are required".into());
    }
    if !buy_pressure.is_some_and(|value| (56.0..=82.0).contains(&value)) {
        scalp_reasons.push("Buy pressure must be between 56% and 82%".into());
    }
    if !change_5m.is_some_and(|value| (0.25..=7.0).contains(&value)) {
        scalp_reasons.push("Five-minute price change must be between +0.25% and +7%".into());
    }
    if !change_1h.is_some_and(|value| (-5.0..=20.0).contains(&value)) {
        scalp_reasons.push("One-hour move must be between -5% and +20%".into());
    }
    if !pool_age.is_some_and(|value| (60..=30 * 24 * 60).contains(&value)) {
        scalp_reasons.push("Pool age must be between one hour and 30 days".into());
    }
    if security.state != "verified" {
        scalp_reasons
            .push("GoPlus security evidence must be complete before a setup is eligible".into());
    }
    if security.risk_penalty >= 45.0 {
        scalp_reasons.push("Security risk penalty exceeds the eligibility ceiling".into());
    }
    if hard_block {
        scalp_reasons.push("A critical contract/security risk was detected".into());
    }
    hard_block |= security.state == "blocked";
    let positive = [
        if liquidity >= 250_000.0 {
            20.0
        } else if liquidity >= 100_000.0 {
            14.0
        } else {
            0.0
        },
        if volume_5m.is_some_and(|value| value >= 15_000.0) {
            18.0
        } else {
            0.0
        },
        if volume_velocity.is_some_and(|value| (1.25..=12.0).contains(&value)) {
            15.0
        } else {
            0.0
        },
        if buy_pressure.is_some_and(|value| (56.0..=82.0).contains(&value)) {
            15.0
        } else {
            0.0
        },
        if change_5m.is_some_and(|value| (0.25..=7.0).contains(&value)) {
            12.0
        } else {
            0.0
        },
        if transaction_count.is_some_and(|value| value >= 40) {
            10.0
        } else {
            0.0
        },
        if security.state == "verified" {
            10.0
        } else {
            0.0
        },
    ]
    .iter()
    .sum::<f64>();
    let raw_scalp_score = (positive - risk * 0.25).round().clamp(0.0, 100.0) as u8;
    let scalp_score = if security.state == "blocked" {
        0
    } else if security.state != "verified" {
        raw_scalp_score.min(40)
    } else {
        raw_scalp_score
    };
    let scalp_state = if hard_block {
        "BLOCKED"
    } else if scalp_reasons.is_empty() && risk <= 35.0 {
        "SETUP"
    } else {
        "WATCH"
    };
    if scalp_state == "SETUP" {
        scalp_reasons.push("All configured research gates passed; execution still requires a route quote, simulation and operator authorization".into());
    }
    if reasons.is_empty() {
        reasons.push("No market-structure warning triggered by the configured heuristic".into());
    }
    if security
        .flags
        .iter()
        .any(|flag| flag.contains("could not verify") || flag.contains("unavailable"))
    {
        reasons.push("Missing security evidence is not interpreted as a clean result".into());
    }

    Some(MemeCoinHunterCandidate {
        chain: chain.into(),
        token_address: token_address.into(),
        pair_address: pair_address.into(),
        pair_url,
        symbol: symbol.chars().take(32).collect(),
        name: name.chars().take(100).collect(),
        price_usd,
        market_cap_usd: market_cap,
        fdv_usd: fdv,
        liquidity_usd: liquidity,
        volume_5m_usd: volume_5m,
        volume_1h_usd: volume_1h,
        volume_24h_usd: volume_24h,
        price_change_5m_pct: change_5m,
        price_change_1h_pct: change_1h,
        price_change_24h_pct: change_24h,
        buys_5m,
        sells_5m,
        buys_1h,
        sells_1h,
        buy_pressure_5m_pct: buy_pressure,
        volume_velocity_5m: volume_velocity,
        pool_age_minutes: pool_age,
        holder_top10_pct: security.holder_top10_pct,
        liquidity_locked_top10_pct: security.liquidity_locked_top10_pct,
        buy_tax_pct: security.buy_tax_pct,
        sell_tax_pct: security.sell_tax_pct,
        security_state: security.state,
        security_checked_at_ms: security.checked_at_ms,
        security_flags: security.flags,
        discovery_sources: discovery_sources.split(" | ").map(str::to_owned).collect(),
        risk_score: risk.round() as u8,
        scalp_score,
        scalp_state: scalp_state.into(),
        scalp_reasons: if scalp_reasons.is_empty() {
            reasons
        } else {
            scalp_reasons
        },
        received_at_ms: now_ms,
    })
}

fn candidate_dataset_fingerprint(candidates: &[MemeCoinHunterCandidate]) -> Result<String, String> {
    let canonical = serde_json::to_vec(candidates)
        .map_err(|_| "Candidate report could not be fingerprinted.".to_string())?;
    Ok(Sha256::digest(canonical)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>())
}

fn validate_report_integrity(report: &MemeCoinHunterReport) -> Result<(), String> {
    if report.contract_version != "meme-coin-hunter.v1"
        || Uuid::parse_str(&report.run_id).is_err()
        || report.scanned_at_ms <= 0
        || report.candidate_count != report.candidates.len()
        || report.discovered_count < report.candidate_count
        || report.candidate_count == 0
        || report.candidate_count > MAX_CANDIDATES
        || report.scalp_setup_count > report.candidate_count
        || report.blocked_count > report.candidate_count
        || report.execution_authority != "NONE · RESEARCH AND ALERTS ONLY"
        || !matches!(report.provider_state.as_str(), "CONNECTED" | "PARTIAL")
    {
        return Err(
            "Stored Meme Coin Hunter report failed its structural integrity checks.".into(),
        );
    }
    if report.sources.is_empty() || report.decision_limitations.is_empty() {
        return Err(
            "Stored Meme Coin Hunter report is missing provider provenance or decision limits."
                .into(),
        );
    }
    let mut source_names = BTreeSet::new();
    if report.sources.iter().any(|source| {
        source.provider.trim().is_empty()
            || !source_names.insert(source.provider.as_str())
            || !matches!(
                source.state.as_str(),
                "connected" | "empty" | "degraded" | "not_configured"
            )
            || source
                .received_at_ms
                .is_some_and(|timestamp| timestamp <= 0)
            || source.state == "connected" && source.received_at_ms.is_none()
            || matches!(source.state.as_str(), "empty" | "not_configured")
                && source.discovered_count != 0
    }) {
        return Err(
            "Stored Meme Coin Hunter report contains invalid or duplicated provider provenance."
                .into(),
        );
    }
    let setup_count = report
        .candidates
        .iter()
        .filter(|candidate| candidate.scalp_state == "SETUP")
        .count();
    let blocked_count = report
        .candidates
        .iter()
        .filter(|candidate| candidate.scalp_state == "BLOCKED")
        .count();
    if setup_count != report.scalp_setup_count || blocked_count != report.blocked_count {
        return Err(
            "Stored Meme Coin Hunter candidate totals do not match the report summary.".into(),
        );
    }
    let mut identities = BTreeSet::new();
    for candidate in &report.candidates {
        if !supported_dex_chain(&candidate.chain)
            || !valid_token_address(&candidate.chain, &candidate.token_address)
            || !valid_token_address(&candidate.chain, &candidate.pair_address)
            || !identities.insert(candidate_key(&candidate.chain, &candidate.token_address))
            || !candidate.price_usd.is_finite()
            || candidate.price_usd <= 0.0
            || !candidate.liquidity_usd.is_finite()
            || candidate.liquidity_usd < 0.0
            || !candidate.volume_24h_usd.is_finite()
            || candidate.volume_24h_usd < 0.0
            || [
                candidate.market_cap_usd,
                candidate.fdv_usd,
                candidate.volume_5m_usd,
                candidate.volume_1h_usd,
            ]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite() || value < 0.0)
            || [
                candidate.price_change_5m_pct,
                candidate.price_change_1h_pct,
                candidate.price_change_24h_pct,
            ]
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite())
            || candidate
                .buy_pressure_5m_pct
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || candidate
                .volume_velocity_5m
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || !matches!(
                candidate.security_state.as_str(),
                "verified" | "partial" | "unavailable" | "blocked"
            )
            || !matches!(
                candidate.scalp_state.as_str(),
                "SETUP" | "WATCH" | "BLOCKED"
            )
            || candidate.symbol.is_empty()
            || candidate.symbol.len() > 32
            || candidate.name.is_empty()
            || candidate.name.len() > 100
            || candidate
                .security_checked_at_ms
                .is_some_and(|timestamp| timestamp <= 0)
            || candidate.security_state != "unavailable"
                && candidate.security_checked_at_ms.is_none()
            || candidate.received_at_ms <= 0
            || candidate.risk_score > 100
            || candidate.scalp_score > 100
            || candidate.discovery_sources.is_empty()
            || candidate
                .discovery_sources
                .iter()
                .any(|source| source.trim().is_empty())
            || candidate.scalp_reasons.is_empty()
            || candidate
                .scalp_reasons
                .iter()
                .any(|reason| reason.trim().is_empty())
            || candidate
                .holder_top10_pct
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || candidate
                .liquidity_locked_top10_pct
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || candidate
                .buy_tax_pct
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || candidate
                .sell_tax_pct
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || candidate.scalp_state == "SETUP"
                && (candidate.security_state != "verified" || candidate.risk_score > 35)
            || candidate.security_state == "blocked"
                && (candidate.scalp_state != "BLOCKED" || candidate.scalp_score != 0)
            || candidate.scalp_state == "BLOCKED" && candidate.security_state != "blocked"
        {
            return Err(
                "Stored Meme Coin Hunter candidate failed its data-integrity checks.".into(),
            );
        }
        if let Some(url) = candidate.pair_url.as_deref() {
            if !url.starts_with("https://dexscreener.com/") || url.len() > 500 {
                return Err(
                    "Stored Meme Coin Hunter report contains an untrusted pair URL.".into(),
                );
            }
        }
    }
    if report.candidates.iter().any(|candidate| {
        report.security_provider_configured != candidate.security_checked_at_ms.is_some()
    }) {
        return Err(
            "Stored report security timestamps do not match whether a GoPlus check was configured."
                .into(),
        );
    }
    if report.provider_state == "CONNECTED"
        && (!report.security_provider_configured
            || report
                .sources
                .iter()
                .any(|source| source.state != "connected")
            || report
                .candidates
                .iter()
                .any(|candidate| candidate.security_state != "verified"))
    {
        return Err("Stored report claims full connectivity despite incomplete provider or security coverage.".into());
    }
    if candidate_dataset_fingerprint(&report.candidates)? != report.dataset_fingerprint_sha256 {
        return Err(
            "Stored Meme Coin Hunter dataset fingerprint does not match its candidates.".into(),
        );
    }
    Ok(())
}

fn save_report(path: &Path, report: &MemeCoinHunterReport) -> Result<(), String> {
    validate_report_integrity(report)?;
    let connection = rusqlite::Connection::open(path.join(REPORT_DB))
        .map_err(|_| "Meme Coin Hunter history database could not be opened.".to_string())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|_| "Meme Coin Hunter history database is busy.".to_string())?;
    connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; CREATE TABLE IF NOT EXISTS scan_runs (run_id TEXT PRIMARY KEY, scanned_at_ms INTEGER NOT NULL, dataset_fingerprint_sha256 TEXT NOT NULL, report_json TEXT NOT NULL); CREATE INDEX IF NOT EXISTS idx_scan_runs_time ON scan_runs(scanned_at_ms DESC);")
        .map_err(|_| "Meme Coin Hunter history schema could not be initialized.".to_string())?;
    let payload = serde_json::to_string(report)
        .map_err(|_| "Meme Coin Hunter report could not be serialized.".to_string())?;
    let transaction = connection
        .unchecked_transaction()
        .map_err(|_| "Meme Coin Hunter history transaction could not begin.".to_string())?;
    transaction.execute("INSERT INTO scan_runs (run_id, scanned_at_ms, dataset_fingerprint_sha256, report_json) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![report.run_id, report.scanned_at_ms, report.dataset_fingerprint_sha256, payload])
        .map_err(|_| "Meme Coin Hunter report could not be committed to history.".to_string())?;
    transaction.execute("DELETE FROM scan_runs WHERE run_id NOT IN (SELECT run_id FROM scan_runs ORDER BY scanned_at_ms DESC LIMIT ?1)", rusqlite::params![MAX_HISTORY])
        .map_err(|_| "Meme Coin Hunter history retention could not be applied.".to_string())?;
    transaction
        .commit()
        .map_err(|_| "Meme Coin Hunter history transaction could not commit.".to_string())
}

pub fn latest_report(data_dir: PathBuf) -> Result<Option<MemeCoinHunterReport>, String> {
    let db_path = data_dir.join(REPORT_DB);
    if !db_path.exists() {
        return Ok(None);
    }
    let connection = rusqlite::Connection::open(db_path)
        .map_err(|_| "Meme Coin Hunter history database could not be opened.".to_string())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|_| "Meme Coin Hunter history database is busy.".to_string())?;
    let payload: Option<String> = connection
        .query_row(
            "SELECT report_json FROM scan_runs ORDER BY scanned_at_ms DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| "Meme Coin Hunter history could not be read.".to_string())?;
    payload
        .map(|value| {
            let report: MemeCoinHunterReport = serde_json::from_str(&value).map_err(|_| {
                "Stored Meme Coin Hunter report failed schema validation.".to_string()
            })?;
            validate_report_integrity(&report)?;
            Ok(report)
        })
        .transpose()
}

pub async fn run_scan(data_dir: PathBuf) -> Result<MemeCoinHunterReport, String> {
    let now_ms = Utc::now().timestamp_millis();
    let _lease = claim_scan_slot(now_ms, &data_dir)?;
    let client = provider_client()?;
    let security_token = load_security_token()?;
    let mut discovered = BTreeMap::<String, DiscoveryToken>::new();
    let mut sources = Vec::new();
    let mut next_discovery_rank = 0usize;
    let discovery_requests = vec![
        (
            "GeckoTerminal new pools",
            "https://api.geckoterminal.com/api/v2/networks/new_pools?page=1".to_string(),
            true,
        ),
        (
            "DEX Screener token profiles",
            "https://api.dexscreener.com/token-profiles/latest/v1".to_string(),
            false,
        ),
        (
            "DEX Screener latest boosts",
            "https://api.dexscreener.com/token-boosts/latest/v1".to_string(),
            false,
        ),
        (
            "GeckoTerminal trending pools",
            "https://api.geckoterminal.com/api/v2/networks/trending_pools?page=1".to_string(),
            true,
        ),
    ];
    let requests = discovery_requests.into_iter().map(|(name, url, gecko)| {
        let client = client.clone();
        async move { (name, get_json(&client, &url, gecko).await) }
    });
    let discovery_results = join_all(requests).await;
    for (name, result) in discovery_results {
        match result {
            Ok(payload) => {
                let count = if name == "GeckoTerminal new pools"
                    || name == "GeckoTerminal trending pools"
                {
                    parse_gecko_discovery(&payload, name, &mut discovered, &mut next_discovery_rank)
                } else {
                    parse_dex_discovery(&payload, name, &mut discovered, &mut next_discovery_rank)
                };
                sources.push(MemeCoinHunterSourceStatus {
                    provider: name.into(),
                    state: if count > 0 { "connected" } else { "empty" }.into(),
                    discovered_count: count,
                    received_at_ms: Some(Utc::now().timestamp_millis()),
                    detail: if count > 0 {
                        "Supported token identities validated; discovery is not a safety verdict"
                            .into()
                    } else {
                        "Provider responded but returned no supported, validated token identities"
                            .into()
                    },
                });
            }
            Err(error) => sources.push(MemeCoinHunterSourceStatus {
                provider: name.into(),
                state: "degraded".into(),
                discovered_count: 0,
                received_at_ms: None,
                detail: error,
            }),
        }
    }
    if discovered.is_empty() {
        return Err("Discovery providers returned no supported token identities. Last report remains available.".into());
    }

    let mut grouped = BTreeMap::<String, Vec<String>>::new();
    let mut ranked_discovery = discovered.values().collect::<Vec<_>>();
    ranked_discovery.sort_by_key(|token| token.rank);
    for token in ranked_discovery {
        let group = grouped.entry(token.chain.clone()).or_default();
        if group.len() < MAX_PER_CHAIN {
            group.push(token.address.clone());
        }
    }
    let market_requests = grouped.into_iter().map(|(chain, addresses)| {
        let client = client.clone();
        async move {
            let joined = addresses.join(",");
            let url = format!("https://api.dexscreener.com/tokens/v1/{chain}/{joined}");
            (
                chain.clone(),
                addresses,
                get_json(&client, &url, false).await,
            )
        }
    });
    let market_results = join_all(market_requests).await;
    let mut pairs_by_token = BTreeMap::<String, Value>::new();
    let mut market_errors = Vec::new();
    for (chain, _addresses, result) in market_results {
        match result {
            Ok(payload) => {
                if let Some(pairs) = payload.as_array() {
                    for pair in pairs {
                        let pair_chain = pair
                            .get("chainId")
                            .and_then(Value::as_str)
                            .and_then(normalize_dex_chain);
                        if pair_chain.as_deref() == Some(chain.as_str()) {
                            let liquidity =
                                finite_number(pair.pointer("/liquidity/usd")).unwrap_or(0.0);
                            let Some(base_address) =
                                pair.pointer("/baseToken/address").and_then(Value::as_str)
                            else {
                                continue;
                            };
                            let key = candidate_key(&chain, base_address);
                            if !discovered.contains_key(&key) {
                                continue;
                            }
                            let existing_liquidity = pairs_by_token
                                .get(&key)
                                .and_then(|value| finite_number(value.pointer("/liquidity/usd")))
                                .unwrap_or(-1.0);
                            if liquidity > existing_liquidity {
                                pairs_by_token.insert(key, pair.clone());
                            }
                        }
                    }
                } else {
                    market_errors.push(format!(
                        "{chain}: DEX Screener token response was not an array"
                    ));
                }
            }
            Err(error) => market_errors.push(format!("{chain}: {error}")),
        }
    }
    if pairs_by_token.is_empty() {
        return Err(format!(
            "No discovered token had a valid DEX market pair. {}",
            market_errors.join("; ")
        ));
    }

    let mut security_by_chain = BTreeMap::<String, Value>::new();
    let mut security_errors = Vec::new();
    if let Some(token) = security_token.as_ref() {
        let addresses_by_chain = pairs_by_token.iter().fold(
            BTreeMap::<String, Vec<String>>::new(),
            |mut groups, (key, _pair)| {
                let chain = key.split(':').next().unwrap_or_default();
                let address = discovered
                    .get(key)
                    .map(|token| token.address.as_str())
                    .unwrap_or_default();
                let addresses = groups.entry(chain.to_string()).or_default();
                if !address.is_empty()
                    && addresses.len() < MAX_PER_CHAIN
                    && !addresses.contains(&address.to_string())
                {
                    addresses.push(address.into());
                }
                groups
            },
        );
        let security_requests = addresses_by_chain
            .into_iter()
            .filter(|(chain, _)| security_chain_id(chain).is_some())
            .map(|(chain, addresses)| {
                let client = client.clone();
                let token = Zeroizing::new(token.to_string());
                async move {
                    (
                        chain.clone(),
                        fetch_security_for_chain(client, token, chain, addresses).await,
                    )
                }
            });
        for (_, result) in join_all(security_requests).await {
            match result {
                Ok((chain, result)) => {
                    security_by_chain.insert(chain, result);
                }
                Err(error) => security_errors.push(error),
            }
        }
    }
    let security_checked_at_ms = security_token
        .as_ref()
        .map(|_| Utc::now().timestamp_millis());
    let mut candidates = Vec::new();
    for (key, pair) in &pairs_by_token {
        let chain = key.split(':').next().unwrap_or_default();
        let discovered_token = discovered.get(key);
        let address = discovered_token
            .map(|token| token.address.as_str())
            .unwrap_or_default();
        let sources = discovered_token
            .map(|value| value.source.clone())
            .unwrap_or_else(|| "DEX Screener pair enrichment".into());
        let pair_address = pair
            .get("pairAddress")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let security_record = security_by_chain
            .get(chain)
            .and_then(|result| find_security_record(result, chain, address));
        let security =
            assess_security(chain, security_record, pair_address, security_checked_at_ms);
        if let Some(candidate) = score_candidate(chain, address, pair, sources, security, now_ms) {
            candidates.push(candidate);
        }
    }
    candidates.sort_by(|left, right| {
        right
            .scalp_score
            .cmp(&left.scalp_score)
            .then_with(|| left.risk_score.cmp(&right.risk_score))
            .then_with(|| right.liquidity_usd.total_cmp(&left.liquidity_usd))
    });
    candidates.truncate(MAX_CANDIDATES);
    if candidates.is_empty() {
        return Err(
            "No candidate passed the market-data integrity checks. Last report remains available."
                .into(),
        );
    }

    if !market_errors.is_empty() {
        sources.push(MemeCoinHunterSourceStatus {
            provider: "DEX Screener pair enrichment".into(),
            state: "degraded".into(),
            discovered_count: pairs_by_token.len(),
            received_at_ms: Some(now_ms),
            detail: market_errors.join("; "),
        });
    } else {
        sources.push(MemeCoinHunterSourceStatus {
            provider: "DEX Screener pair enrichment".into(),
            state: "connected".into(),
            discovered_count: pairs_by_token.len(),
            received_at_ms: Some(now_ms),
            detail: "Five-minute and one-hour price, volume and transaction metrics loaded".into(),
        });
    }
    if !security_errors.is_empty() {
        sources.push(MemeCoinHunterSourceStatus {
            provider: "GoPlus Security".into(),
            state: "degraded".into(),
            discovered_count: 0,
            received_at_ms: security_checked_at_ms,
            detail: security_errors.join("; "),
        });
    } else if security_token.is_some() {
        sources.push(MemeCoinHunterSourceStatus { provider: "GoPlus Security".into(), state: "connected".into(), discovered_count: security_by_chain.values().map(|value| value.as_object().map_or(0, |object| object.len())).sum(), received_at_ms: security_checked_at_ms, detail: "Token security records loaded; missing subfields remain unknown and block setup eligibility".into() });
    } else {
        sources.push(MemeCoinHunterSourceStatus { provider: "GoPlus Security".into(), state: "not_configured".into(), discovered_count: 0, received_at_ms: None, detail: "Configure a GoPlus access token in the operating-system credential vault to enable security gating".into() });
    }

    let fingerprint = candidate_dataset_fingerprint(&candidates)?;
    let setup_count = candidates
        .iter()
        .filter(|candidate| candidate.scalp_state == "SETUP")
        .count();
    let blocked_count = candidates
        .iter()
        .filter(|candidate| candidate.scalp_state == "BLOCKED")
        .count();
    let failed_sources = sources
        .iter()
        .filter(|source| {
            matches!(
                source.state.as_str(),
                "degraded" | "empty" | "not_configured"
            )
        })
        .count();
    let security_coverage_incomplete = candidates
        .iter()
        .any(|candidate| candidate.security_state != "verified");
    let report = MemeCoinHunterReport {
        contract_version: "meme-coin-hunter.v1".into(), run_id: Uuid::new_v4().to_string(), scanned_at_ms: now_ms,
        provider_state: if failed_sources == 0 && !security_coverage_incomplete { "CONNECTED" } else { "PARTIAL" }.into(),
        security_provider_configured: security_token.is_some(), discovered_count: discovered.len(), candidate_count: candidates.len(),
        scalp_setup_count: setup_count, blocked_count, dataset_fingerprint_sha256: fingerprint,
        sources, candidates, execution_authority: "NONE · RESEARCH AND ALERTS ONLY".into(),
        decision_limitations: vec![
            "DEX Screener is REST market data; this is not a sub-second websocket feed and is not suitable for latency-sensitive execution".into(),
            "A SETUP is a deterministic heuristic shortlist, not a probability of profit or a buy/sell instruction".into(),
            "No route quote, transaction simulation, wallet state, gas estimate, or executable price-impact check is performed".into(),
            "A complete provider response cannot guarantee that a token is safe; new contract behavior and liquidity can change after scanning".into(),
        ],
    };
    save_report(&data_dir, &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVM_TOKEN: &str = "0x1111111111111111111111111111111111111111";
    const EVM_PAIR: &str = "0x2222222222222222222222222222222222222222";
    const EVM_QUOTE: &str = "0x3333333333333333333333333333333333333333";
    const SOL_TOKEN: &str = "So11111111111111111111111111111111111111112";
    const SOL_PAIR: &str = "7xKXtg2CW87d97TXJSDpbD5jBkheTQ5UXt5j6q5Q9p3";

    fn verified_evm_record(pair: &str) -> Value {
        json!({
            "is_open_source":"1", "is_proxy":"0", "is_honeypot":"0", "cannot_buy":"0",
            "cannot_sell_all":"0", "is_mintable":"0", "can_take_back_ownership":"0",
            "hidden_owner":"0", "owner_change_balance":"0", "selfdestruct":"0",
            "slippage_modifiable":"0", "transfer_pausable":"0", "is_blacklisted":"0",
            "is_whitelisted":"0", "trading_cooldown":"0", "is_anti_whale":"0",
            "anti_whale_modifiable":"0", "personal_slippage_modifiable":"0",
            "is_airdrop_scam":"0", "fake_token":{"value":"0"},
            "buy_tax":"0.01", "sell_tax":"0.01", "holder_count":"2", "lp_holder_count":"2",
            "holders":[{"address":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","percent":"0.05"},{"address":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","percent":"0.04"}],
            "dex":[{"pair":pair}],
            "lp_holders":[{"address":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","percent":"0.80","is_locked":"1"},{"address":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","percent":"0.20","is_locked":"0"}]
        })
    }

    #[test]
    fn quote_side_candidate_is_rejected_when_provider_price_semantics_are_ambiguous() {
        let now_ms = 1_700_000_000_000;
        let pair = json!({
            "chainId":"ethereum",
            "baseToken":{"address":EVM_TOKEN,"name":"Base token","symbol":"BASE"},
            "quoteToken":{"address":EVM_QUOTE,"name":"Quote token","symbol":"QUOTE"},
            "priceUsd":"2.0","priceNative":"4.0",
            "liquidity":{"usd":150000},
            "volume":{"m5":30000,"h1":300000,"h24":2000000},
            "priceChange":{"m5":10.0,"h1":-10.0,"h24":20.0},
            "txns":{"m5":{"buys":80,"sells":20},"h1":{"buys":300,"sells":100}},
            "pairCreatedAt":now_ms - 7_200_000,
            "pairAddress":EVM_PAIR,
            "url":"https://dexscreener.com/ethereum/test"
        });
        let security = SecurityAssessment {
            state: "verified".into(),
            checked_at_ms: Some(now_ms),
            flags: vec![],
            hard_block: false,
            holder_top10_pct: Some(10.0),
            liquidity_locked_top10_pct: Some(80.0),
            buy_tax_pct: Some(0.0),
            sell_tax_pct: Some(0.0),
            risk_penalty: 0.0,
        };
        assert!(score_candidate(
            "ethereum",
            EVM_QUOTE,
            &pair,
            "new pool".into(),
            security,
            now_ms
        )
        .is_none());
    }

    #[test]
    fn security_token_accepts_opaque_printable_bearer_values_only() {
        assert!(valid_security_token("AbC123+/=".repeat(5).as_str()));
        assert!(!valid_security_token("short"));
        assert!(!valid_security_token(&format!("{}\n", "A".repeat(32))));
        assert!(!valid_security_token(&"A".repeat(4097)));
    }

    #[test]
    fn holder_snapshot_must_match_reported_count_when_fewer_than_ten_exist() {
        let holders = json!([
            {"address":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","percent":"0.1"},
            {"address":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","percent":"0.1"}
        ]);
        assert_eq!(
            percent_from_holder_array(Some(&holders), "ethereum", Some(2)),
            Some(20.0)
        );
        assert_eq!(
            percent_from_holder_array(Some(&holders), "ethereum", Some(1)),
            None
        );
        assert_eq!(
            percent_from_holder_array(Some(&holders), "ethereum", Some(3)),
            None
        );
    }

    #[test]
    fn address_validation_is_chain_specific() {
        assert!(valid_token_address("ethereum", EVM_TOKEN));
        assert!(!valid_token_address(
            "ethereum",
            "1111111111111111111111111111111111111111"
        ));
        assert!(valid_token_address("solana", SOL_TOKEN));
        assert!(valid_token_address(
            "solana",
            "11111111111111111111111111111111"
        ));
        assert!(!valid_token_address(
            "solana",
            "1111111111111111111111111111111111111111111"
        ));
        assert!(!valid_token_address(
            "solana",
            "0x1111111111111111111111111111111111111111"
        ));
        assert!(!valid_token_address(
            "solana",
            "So11111111111111111111111111111111111111110"
        ));
        assert!(!valid_token_address(
            "ethereum",
            "0x0000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn major_quote_assets_are_not_ranked_as_meme_coin_candidates() {
        assert!(is_major_quote_asset(
            "ethereum",
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        ));
        assert!(is_major_quote_asset(
            "ethereum",
            "0xA0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
        ));
        assert!(is_major_quote_asset(
            "solana",
            "So11111111111111111111111111111111111111112"
        ));
        assert!(is_major_quote_asset(
            "solana",
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        ));
        assert!(!is_major_quote_asset("ethereum", EVM_TOKEN));
        let mut discovered = BTreeMap::new();
        let mut rank = 0;
        assert!(!push_discovery(
            &mut discovered,
            "ethereum",
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
            "trending",
            &mut rank
        ));
        assert!(discovered.is_empty());
    }

    #[test]
    fn gecko_resource_ids_preserve_multisegment_network_names() {
        assert_eq!(
            split_gecko_resource_id("polygon_pos_0x1111111111111111111111111111111111111111")
                .unwrap()
                .0,
            "polygon"
        );
        assert_eq!(
            split_gecko_resource_id("eth_0x1111111111111111111111111111111111111111")
                .unwrap()
                .0,
            "ethereum"
        );
        assert!(
            split_gecko_resource_id("unknown_0x1111111111111111111111111111111111111111").is_none()
        );
    }

    #[test]
    fn gecko_discovery_keeps_only_valid_matching_pool_and_token_networks() {
        let payload = json!({"data":[
            {"id":"polygon_pos_0x2222222222222222222222222222222222222222","relationships":{"base_token":{"data":{"id":"polygon_pos_0x1111111111111111111111111111111111111111"}}}},
            {"id":"eth_0x3333333333333333333333333333333333333333","relationships":{"base_token":{"data":{"id":"base_0x1111111111111111111111111111111111111111"}}}}
        ]});
        let mut discovered = BTreeMap::new();
        let mut rank = 0;
        let count = parse_gecko_discovery(&payload, "new pools", &mut discovered, &mut rank);
        assert_eq!(count, 1);
        assert!(discovered.contains_key(&identity(
            "polygon",
            "0x1111111111111111111111111111111111111111"
        )));
    }

    #[test]
    fn unknown_security_never_becomes_a_setup() {
        let unknown = assess_security("ethereum", None, EVM_PAIR, Some(1_700_000_000_000));
        assert_eq!(unknown.state, "unavailable");
        assert!(!unknown.hard_block);
        assert!(unknown.checked_at_ms.is_some());
        assert!(unknown.risk_penalty >= 40.0);
    }

    #[test]
    fn evm_security_uses_documented_root_lp_holders_and_selected_pair_match() {
        let data = verified_evm_record(EVM_PAIR);
        let assessment =
            assess_security("ethereum", Some(&data), EVM_PAIR, Some(1_700_000_000_000));
        assert_eq!(assessment.state, "verified");
        assert_eq!(assessment.holder_top10_pct, Some(9.0));
        assert_eq!(assessment.liquidity_locked_top10_pct, Some(80.0));

        let wrong_pool = assess_security(
            "ethereum",
            Some(&data),
            "0x3333333333333333333333333333333333333333",
            Some(1_700_000_000_000),
        );
        assert_eq!(wrong_pool.state, "partial");
        assert!(wrong_pool.liquidity_locked_top10_pct.is_none());
    }

    #[test]
    fn missing_security_credentials_do_not_fabricate_a_security_check_timestamp() {
        let assessment = assess_security("ethereum", None, EVM_PAIR, None);
        assert_eq!(assessment.state, "unavailable");
        assert_eq!(assessment.checked_at_ms, None);
    }

    #[test]
    fn absent_fake_token_field_means_no_counterfeit_evidence_per_goplus_contract() {
        let mut data = verified_evm_record(EVM_PAIR);
        data.as_object_mut().unwrap().remove("fake_token");
        let assessment =
            assess_security("ethereum", Some(&data), EVM_PAIR, Some(1_700_000_000_000));
        assert_eq!(assessment.state, "verified");
        data["fake_token"] = json!({"value":"1"});
        let counterfeit =
            assess_security("ethereum", Some(&data), EVM_PAIR, Some(1_700_000_000_000));
        assert_eq!(counterfeit.state, "blocked");
    }

    #[test]
    fn negative_evm_tax_is_invalid_and_cannot_verify_security() {
        let mut data = verified_evm_record(EVM_PAIR);
        data["buy_tax"] = json!("-0.01");
        let assessment =
            assess_security("ethereum", Some(&data), EVM_PAIR, Some(1_700_000_000_000));
        assert_eq!(assessment.state, "partial");
        assert_eq!(assessment.buy_tax_pct, None);
        assert!(assessment
            .flags
            .iter()
            .any(|flag| flag.contains("invalid negative")));

        data["buy_tax"] = json!("1.01");
        let impossible_tax =
            assess_security("ethereum", Some(&data), EVM_PAIR, Some(1_700_000_000_000));
        assert_eq!(impossible_tax.state, "blocked");
        assert_eq!(impossible_tax.buy_tax_pct, None);
    }

    #[test]
    fn evm_high_tax_and_holder_concentration_fail_setup_eligibility() {
        let mut high_tax = verified_evm_record(EVM_PAIR);
        high_tax["buy_tax"] = json!("0.08");
        assert_eq!(
            assess_security(
                "ethereum",
                Some(&high_tax),
                EVM_PAIR,
                Some(1_700_000_000_000)
            )
            .state,
            "partial"
        );

        let mut concentrated = verified_evm_record(EVM_PAIR);
        concentrated["holders"] = json!([
            {"address":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","percent":"0.35"},
            {"address":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","percent":"0.10"}
        ]);
        assert_eq!(
            assess_security(
                "ethereum",
                Some(&concentrated),
                EVM_PAIR,
                Some(1_700_000_000_000)
            )
            .state,
            "partial"
        );

        let mut incomplete = verified_evm_record(EVM_PAIR);
        incomplete.as_object_mut().unwrap().remove("holder_count");
        incomplete["holders"] = json!([
            {"address":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","percent":"0.10"},
            {"address":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","percent":"0.05"}
        ]);
        assert!(percent_from_holder_array(incomplete.get("holders"), "ethereum", None).is_none());
    }

    #[test]
    fn solana_security_uses_documented_field_names_and_pool_scoped_lp_evidence() {
        let data = json!({
            "non_transferable":"0", "default_account_state":"1",
            "creator":{"malicious_address":"0"}, "transfer_hook":{"malicious_address":"0"},
            "mintable":{"status":"0"}, "freezable":{"status":"0"}, "closable":{"status":"0"},
            "metadata_mutable":{"status":"0"}, "balance_mutable_authority":{"status":"0"},
            "transfer_fee_upgradable":{"status":"0"}, "default_account_state_upgradable":{"status":"0"},
            "transfer_hook_upgradable":{"status":"0"},
            "transfer_fee":{"current_fee_rate":{"fee_rate":"0"}},
            "holders":[
                {"token_account":"Holder001","percent":"0.01"},{"token_account":"Holder002","percent":"0.01"},
                {"token_account":"Holder003","percent":"0.01"},{"token_account":"Holder004","percent":"0.01"},
                {"token_account":"Holder005","percent":"0.01"},{"token_account":"Holder006","percent":"0.01"},
                {"token_account":"Holder007","percent":"0.01"},{"token_account":"Holder008","percent":"0.01"},
                {"token_account":"Holder009","percent":"0.01"},{"token_account":"Holder010","percent":"0.01"}
            ],
            "dex":[{"id":SOL_PAIR,"lp_holders":[
                {"token_account":"Lp001","percent":"0.75","is_locked":"1"},
                {"token_account":"Lp002","percent":"0.0277777778","is_locked":"0"},
                {"token_account":"Lp003","percent":"0.0277777778","is_locked":"0"},
                {"token_account":"Lp004","percent":"0.0277777778","is_locked":"0"},
                {"token_account":"Lp005","percent":"0.0277777778","is_locked":"0"},
                {"token_account":"Lp006","percent":"0.0277777778","is_locked":"0"},
                {"token_account":"Lp007","percent":"0.0277777778","is_locked":"0"},
                {"token_account":"Lp008","percent":"0.0277777778","is_locked":"0"},
                {"token_account":"Lp009","percent":"0.0277777778","is_locked":"0"},
                {"token_account":"Lp010","percent":"0.0277777778","is_locked":"0"}
            ]}]
        });
        let assessment = assess_security("solana", Some(&data), SOL_PAIR, Some(1_700_000_000_000));
        assert_eq!(assessment.state, "verified");
        assert_eq!(assessment.holder_top10_pct, Some(10.0));
        assert_eq!(assessment.liquidity_locked_top10_pct, Some(75.0));

        let wrong_pool = assess_security(
            "solana",
            Some(&data),
            "9zKXtg2CW87d97TXJSDpbD5jBkheTQ5UXt5j6q5Q9p3",
            Some(1_700_000_000_000),
        );
        assert_eq!(wrong_pool.state, "partial");
    }

    #[test]
    fn explicit_honeypot_is_a_hard_block() {
        let mut data = verified_evm_record(EVM_PAIR);
        data["is_honeypot"] = json!("1");
        let result = assess_security("ethereum", Some(&data), EVM_PAIR, Some(1_700_000_000_000));
        assert!(result.hard_block);
        assert_eq!(result.state, "blocked");
    }

    #[test]
    fn evm_lp_holder_data_is_rejected_when_multiple_pools_make_its_scope_ambiguous() {
        let mut data = verified_evm_record(EVM_PAIR);
        data["dex"] =
            json!([{"pair":EVM_PAIR},{"pair":"0x3333333333333333333333333333333333333333"}]);
        let assessment =
            assess_security("ethereum", Some(&data), EVM_PAIR, Some(1_700_000_000_000));
        assert_eq!(assessment.state, "partial");
        assert!(assessment.liquidity_locked_top10_pct.is_none());
    }

    #[test]
    fn partial_sell_restriction_requires_review_and_cannot_be_a_setup() {
        let mut data = verified_evm_record(EVM_PAIR);
        data["cannot_sell_all"] = json!("1");
        let assessment =
            assess_security("ethereum", Some(&data), EVM_PAIR, Some(1_700_000_000_000));
        assert_eq!(assessment.state, "partial");
    }

    #[test]
    fn solana_scheduled_transfer_fee_is_included_in_the_risk_ceiling() {
        let data = json!({
            "non_transferable":"0", "default_account_state":"1",
            "creator":{"malicious_address":"0"}, "transfer_hook":{"malicious_address":"0"},
            "mintable":{"status":"0"}, "freezable":{"status":"0"}, "closable":{"status":"0"},
            "metadata_mutable":{"status":"0"}, "balance_mutable_authority":{"status":"0"},
            "transfer_fee_upgradable":{"status":"0"}, "default_account_state_upgradable":{"status":"0"},
            "transfer_hook_upgradable":{"status":"0"},
            "transfer_fee":{"current_fee_rate":{"fee_rate":"0"},"scheduled_fee_rate":{"fee_rate":"1500","epoch":"999999"}},
            "holders":[], "dex":[{"id":SOL_PAIR,"lp_holders":[{"token_account":"Lp001","percent":"1.0","is_locked":"1"}]}]
        });
        let result = assess_security("solana", Some(&data), SOL_PAIR, Some(1_700_000_000_000));
        assert_eq!(result.buy_tax_pct, Some(15.0));
        assert_eq!(result.state, "blocked");
    }

    #[test]
    fn scheduled_solana_fee_is_still_enforced_when_current_rate_is_missing() {
        let data = json!({
            "non_transferable":"0", "default_account_state":"1",
            "creator":{"malicious_address":"0"}, "transfer_hook":{"malicious_address":"0"},
            "mintable":{"status":"0"}, "freezable":{"status":"0"}, "closable":{"status":"0"},
            "metadata_mutable":{"status":"0"}, "balance_mutable_authority":{"status":"0"},
            "transfer_fee_upgradable":{"status":"0"}, "default_account_state_upgradable":{"status":"0"},
            "transfer_hook_upgradable":{"status":"0"},
            "transfer_fee":{"scheduled_fee_rate":{"fee_rate":"1200","epoch":"999999"}},
            "holders":[], "dex":[{"id":SOL_PAIR,"lp_holders":[{"token_account":"Lp001","percent":"1.0","is_locked":"1"}]}]
        });
        let result = assess_security("solana", Some(&data), SOL_PAIR, Some(1_700_000_000_000));
        assert_eq!(result.buy_tax_pct, Some(12.0));
        assert_eq!(result.state, "blocked");
    }

    #[test]
    fn gecko_discovery_excludes_major_quote_assets() {
        let payload = json!({"data":[{
            "id":"solana_7xKXtg2CW87d97TXJSDpbD5jBkheTQ5UXt5j6q5Q9p3",
            "relationships":{
                "base_token":{"data":{"id":"solana_So11111111111111111111111111111111111111112"}},
                "quote_token":{"data":{"id":"solana_EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"}}
            }
        }]});
        let mut discovered = BTreeMap::new();
        let mut rank = 0;
        let count = parse_gecko_discovery(&payload, "new pools", &mut discovered, &mut rank);
        assert_eq!(count, 0);
        assert!(!discovered.contains_key(&identity(
            "solana",
            "So11111111111111111111111111111111111111112"
        )));
        assert!(!discovered.contains_key(&identity(
            "solana",
            "EPjFWdd5AufqSSqe2M2qN1xzybapC8G4wEGGkZwyTDt1v"
        )));
    }

    #[test]
    fn quote_side_token_is_not_assigned_an_inferred_usd_price() {
        let pair = json!({
            "chainId":"ethereum", "pairAddress":EVM_PAIR, "priceUsd":"2000", "priceNative":"1000",
            "baseToken":{"address":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","symbol":"WETH","name":"Wrapped Ether"},
            "quoteToken":{"address":EVM_TOKEN,"symbol":"MEME","name":"Meme Asset"},
            "liquidity":{"usd":"500000"}, "volume":{"m5":"25000","h1":"300000","h24":"2000000"},
            "txns":{"m5":{"buys":30,"sells":20},"h1":{"buys":300,"sells":200}},
            "priceChange":{"m5":"2","h1":"4","h24":"15"}, "pairCreatedAt":1699990000000i64
        });
        let security = assess_security("ethereum", None, EVM_PAIR, Some(1_700_000_000_000));
        assert!(score_candidate(
            "ethereum",
            EVM_TOKEN,
            &pair,
            "test source".into(),
            security,
            1_700_000_000_000
        )
        .is_none());
    }

    #[test]
    fn latest_report_returns_empty_after_a_scan_reservation_without_a_completed_scan() {
        let data_dir =
            std::env::temp_dir().join(format!("atos-meme-hunter-empty-history-{}", Uuid::new_v4()));
        assert!(persist_scan_attempt(&data_dir, 1_700_000_000_000).is_ok());
        assert!(latest_report(data_dir.clone()).unwrap().is_none());
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn persistent_scan_limit_survives_restarts_and_rejects_clock_rollback() {
        let data_dir =
            std::env::temp_dir().join(format!("atos-meme-hunter-rate-limit-{}", Uuid::new_v4()));
        assert!(persist_scan_attempt(&data_dir, 1_700_000_000_000).is_ok());
        assert!(persist_scan_attempt(&data_dir, 1_700_000_059_999).is_err());
        assert!(persist_scan_attempt(&data_dir, 1_699_999_999_999).is_err());
        assert!(persist_scan_attempt(&data_dir, 1_700_000_060_000).is_ok());
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn partial_lp_holder_snapshots_cannot_prove_locked_liquidity() {
        let holders = json!([
            {"address":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","percent":"1.0","is_locked":"1"}
        ]);
        assert_eq!(
            locked_holder_percent(Some(&holders), "ethereum", None),
            None
        );
        assert_eq!(
            locked_holder_percent(Some(&holders), "ethereum", Some(1)),
            Some(100.0)
        );
    }

    #[test]
    fn locked_liquidity_percent_uses_locked_holder_weights() {
        let data = json!([
            { "address":"0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "percent":"0.3", "is_locked":"1" },
            { "address":"0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "percent":"0.4", "is_locked":"0" },
            { "address":"0xcccccccccccccccccccccccccccccccccccccccc", "percent":"0.2", "is_locked":"1" }
        ]);
        assert_eq!(
            locked_holder_percent(Some(&data), "ethereum", Some(3)),
            Some(50.0)
        );
        assert_eq!(locked_holder_percent(Some(&data), "ethereum", None), None);
    }
}
