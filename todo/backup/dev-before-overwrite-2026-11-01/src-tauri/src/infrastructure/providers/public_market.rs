//! Bounded, read-only public market-data provider integration.
//! Tauri commands call this service; provider HTTP details do not live in the composition root.

use crate::{
    domain::models::{
        validate_candle_series, Candle, CandleInterval, DashboardSnapshot, Quote, ScreenerToken,
        MARKET_DATA_OBSERVATION_MAX_AGE_MS,
    },
    runtime::trading_core::TradingCore,
};
use chrono::Utc;
use futures_util::{future::join_all, StreamExt};
use reqwest::{redirect::Policy, Client, Response};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicBool, AtomicI64, Ordering},
    time::Duration,
};

static LAST_SCREENER_REFRESH_MS: AtomicI64 = AtomicI64::new(0);
static LAST_MARKET_REFRESH_MS: AtomicI64 = AtomicI64::new(0);
static SCREENER_REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);
static MARKET_REFRESH_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

/// Exclusive lease held for the entire async provider refresh. Drop releases it on
/// every return path, including parse/network errors and cancellation. A time-window
/// throttle alone is insufficient because a slow request can outlive that window.
struct RefreshLease<'a> {
    in_flight: &'a AtomicBool,
}

impl Drop for RefreshLease<'_> {
    fn drop(&mut self) {
        self.in_flight.store(false, Ordering::Release);
    }
}

fn claim_refresh_slot<'a>(
    last_attempt: &AtomicI64,
    in_flight: &'a AtomicBool,
    attempt_ms: i64,
    minimum_interval_ms: i64,
    label: &str,
) -> Result<RefreshLease<'a>, String> {
    in_flight
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| {
            format!("{label} refresh is already in progress; concurrent refreshes are rejected.")
        })?;
    let lease = RefreshLease { in_flight };
    let previous_attempt = last_attempt.load(Ordering::Acquire);
    if attempt_ms.saturating_sub(previous_attempt) < minimum_interval_ms {
        return Err(format!(
            "{label} refresh is throttled; retry after the configured interval."
        ));
    }
    last_attempt
        .compare_exchange(
            previous_attempt,
            attempt_ms,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .map_err(|_| {
            format!("Another {label} refresh changed the rate-limit state. Retry shortly.")
        })?;
    Ok(lease)
}

fn append_bounded_chunk(body: &mut Vec<u8>, chunk: &[u8], max_bytes: usize) -> Result<(), String> {
    if body.len().saturating_add(chunk.len()) > max_bytes {
        return Err(format!(
            "Provider response exceeded the {max_bytes}-byte safety limit."
        ));
    }
    body.extend_from_slice(chunk);
    Ok(())
}

async fn parse_json_bounded<T: DeserializeOwned>(
    response: Response,
    max_bytes: usize,
) -> Result<T, String> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes as u64)
    {
        return Err(format!(
            "Provider response exceeds the {max_bytes}-byte safety limit."
        ));
    }
    let capacity = response.content_length().unwrap_or(0).min(max_bytes as u64) as usize;
    let mut body = Vec::with_capacity(capacity);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|error| format!("Unable to read provider response body: {error}"))?;
        append_bounded_chunk(&mut body, &chunk, max_bytes)?;
    }
    serde_json::from_slice(&body)
        .map_err(|error| format!("Provider returned invalid JSON: {error}"))
}

#[derive(Debug, Deserialize)]
struct BinanceTicker {
    symbol: String,
    #[serde(rename = "lastPrice")]
    last_price: String,
    #[serde(rename = "priceChangePercent")]
    price_change_percent: String,
    #[serde(rename = "quoteVolume")]
    quote_volume: String,
    #[serde(rename = "closeTime")]
    close_time: i64,
}

pub async fn refresh_public_screener(core: &TradingCore) -> Result<DashboardSnapshot, String> {
    let attempt_ms = Utc::now().timestamp_millis();
    let _refresh_lease = claim_refresh_slot(
        &LAST_SCREENER_REFRESH_MS,
        &SCREENER_REFRESH_IN_FLIGHT,
        attempt_ms,
        10_000,
        "Screener",
    )?;
    let client = match Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(9))
        .connect_timeout(Duration::from_secs(4))
        .user_agent("AgenticTradingOS/0.11.0 (public-screener; read-only; no-order-permissions)")
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return core.record_screener_failure(format!(
                "Unable to initialize public screener client: {error}"
            ))
        }
    };

    let boosts_response = match client
        .get("https://api.dexscreener.com/token-boosts/latest/v1")
        .send()
        .await
    {
        Ok(response) => match response.error_for_status() {
            Ok(response) => match parse_json_bounded::<Value>(response, 1_048_576).await {
                Ok(value) => value,
                Err(error) => {
                    return core.record_screener_failure(format!(
                        "DEX Screener boost response was rejected: {error}"
                    ))
                }
            },
            Err(error) => {
                return core.record_screener_failure(format!(
                    "DEX Screener boost endpoint returned an unsuccessful status: {error}"
                ))
            }
        },
        Err(error) => {
            return core
                .record_screener_failure(format!("DEX Screener boost request failed: {error}"))
        }
    };
    let Some(boost_rows) = boosts_response.as_array() else {
        return core
            .record_screener_failure("DEX Screener boost response was not a JSON array.".into());
    };

    let allowed_chains: BTreeSet<&'static str> = [
        "solana",
        "ethereum",
        "base",
        "bsc",
        "arbitrum",
        "polygon",
        "optimism",
        "avalanche",
    ]
    .into_iter()
    .collect();
    let mut grouped: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut candidate_tokens = BTreeSet::new();
    for row in boost_rows {
        let Some(chain) = row
            .get("chainId")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase)
        else {
            continue;
        };
        let Some(address) = row
            .get("tokenAddress")
            .and_then(Value::as_str)
            .map(str::to_owned)
        else {
            continue;
        };
        if !allowed_chains.contains(chain.as_str())
            || !(20..=64).contains(&address.len())
            || !address
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            continue;
        }
        let identity = screener_identity(&chain, &address);
        if !seen.insert(identity.clone()) {
            continue;
        }
        candidate_tokens.insert(identity);
        let entries = grouped.entry(chain).or_default();
        // Tokens endpoint documents at most 30 addresses per request. This cap also bounds memory and provider load.
        if entries.len() < 30 {
            entries.push(address);
        }
    }
    if grouped.is_empty() {
        return core.record_screener_failure("Latest boost feed contained no supported token addresses after validation. Previous records retained.".into());
    }

    let requests = grouped.into_iter().map(|(chain, addresses)| {
        let client = client.clone();
        async move {
            let address_path = addresses.join(",");
            let url = format!("https://api.dexscreener.com/tokens/v1/{chain}/{address_path}");
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|error| format!("{chain}: network error: {error}"))?
                .error_for_status()
                .map_err(|error| format!("{chain}: unsuccessful provider status: {error}"))?;
            let result = parse_json_bounded::<Value>(response, 4 * 1_048_576)
                .await
                .map_err(|error| format!("{chain}: rejected pool response: {error}"))?;
            Ok::<(String, Value), String>((chain, result))
        }
    });
    let results = join_all(requests).await;
    let mut errors = Vec::new();
    let mut best_by_token: BTreeMap<String, (f64, Value, String)> = BTreeMap::new();
    let now = Utc::now().timestamp_millis();
    for result in results {
        let (chain, payload) = match result {
            Ok(value) => value,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        let Some(pairs) = payload.as_array() else {
            errors.push(format!("{chain}: provider payload was not a pool array"));
            continue;
        };
        for pair in pairs {
            let pair_chain = pair
                .get("chainId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if pair_chain != chain {
                continue;
            }
            let Some(base) = pair.get("baseToken") else {
                continue;
            };
            let Some(address) = base.get("address").and_then(Value::as_str) else {
                continue;
            };
            let identity = screener_identity(&chain, address);
            // Keep only actual boost-feed candidates. Pool lookup may return pools where a requested token is merely the quote asset.
            if !candidate_tokens.contains(&identity) {
                continue;
            }
            // Select only pools where the discovered token is the base asset so priceUsd denotes that token.
            let liquidity = pair
                .pointer("/liquidity/usd")
                .and_then(value_to_f64)
                .filter(|v| v.is_finite() && *v >= 0.0)
                .unwrap_or(0.0);
            match best_by_token.get(&identity) {
                Some((current_liquidity, _, _)) if *current_liquidity >= liquidity => {}
                _ => {
                    best_by_token.insert(identity, (liquidity, pair.clone(), chain.clone()));
                }
            }
        }
    }

    let mut tokens = Vec::new();
    for (_, (_, pair, chain)) in best_by_token {
        let Some(base) = pair.get("baseToken") else {
            continue;
        };
        let Some(address) = base.get("address").and_then(Value::as_str) else {
            continue;
        };
        let price = pair
            .get("priceUsd")
            .and_then(value_to_f64)
            .filter(|value| value.is_finite() && *value > 0.0);
        let Some(price) = price else {
            continue;
        };
        let symbol = base
            .get("symbol")
            .and_then(Value::as_str)
            .unwrap_or("UNKNOWN")
            .trim();
        let name = base
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(symbol)
            .trim();
        let change_5m = pair
            .pointer("/priceChange/m5")
            .and_then(value_to_f64)
            .filter(|v| v.is_finite());
        let change_1h = pair
            .pointer("/priceChange/h1")
            .and_then(value_to_f64)
            .filter(|v| v.is_finite());
        let change_24h = pair
            .pointer("/priceChange/h24")
            .and_then(value_to_f64)
            .filter(|v| v.is_finite());
        let volume = pair
            .pointer("/volume/h24")
            .and_then(value_to_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .unwrap_or(0.0);
        let liquidity = pair
            .pointer("/liquidity/usd")
            .and_then(value_to_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .unwrap_or(0.0);
        let market_cap = pair
            .get("marketCap")
            .and_then(value_to_f64)
            .filter(|v| v.is_finite() && *v >= 0.0);
        let fdv = pair
            .get("fdv")
            .and_then(value_to_f64)
            .filter(|v| v.is_finite() && *v >= 0.0);
        let created = pair
            .get("pairCreatedAt")
            .and_then(Value::as_i64)
            .filter(|t| *t > 0);
        let age_minutes = created.map(|created_ms| ((now - created_ms).max(0) / 60_000) as u64);
        let buys = pair.pointer("/txns/h24/buys").and_then(Value::as_u64);
        let sells = pair.pointer("/txns/h24/sells").and_then(Value::as_u64);
        let active_boosts = pair.pointer("/boosts/active").and_then(Value::as_u64);
        let pair_address = pair
            .get("pairAddress")
            .and_then(Value::as_str)
            .filter(|value| value.len() <= 80 && value.chars().all(|c| c.is_ascii_alphanumeric()))
            .map(str::to_owned);
        let pair_url = pair
            .get("url")
            .and_then(Value::as_str)
            .filter(|url| url.starts_with("https://dexscreener.com/") && url.len() <= 500)
            .map(str::to_owned);
        let mut risk_flags = vec![
            "Latest boost feed is promotion-biased; paid visibility is not organic demand"
                .to_string(),
            "Contract permissions, holder concentration and liquidity-lock status are not verified"
                .to_string(),
            "Pool-level price impact and executable depth are not modeled".to_string(),
        ];
        if liquidity < 25_000.0 {
            risk_flags.push("Low pool liquidity (< $25k); exit capacity may be poor".into());
        } else if liquidity < 100_000.0 {
            risk_flags.push("Limited pool liquidity (< $100k)".into());
        }
        match age_minutes {
            Some(age) if age < 60 => risk_flags
                .push("Pool is less than one hour old; price and liquidity are unstable".into()),
            None => risk_flags
                .push("Pool creation time unavailable; token age cannot be established".into()),
            _ => {}
        }
        if market_cap.is_none() {
            risk_flags.push(
                "Market cap unavailable; FDV is not a substitute for circulating value".into(),
            );
        }
        if buys.is_none() || sells.is_none() {
            risk_flags.push("24h buy/sell transaction counts unavailable".into());
        }
        if change_1h.is_none() || change_24h.is_none() {
            risk_flags.push("One or more momentum windows are missing".into());
        }
        let turnover = if liquidity > 0.0 {
            volume / liquidity
        } else {
            f64::INFINITY
        };
        if turnover > 20.0 {
            risk_flags.push(
                "Extreme 24h turnover versus displayed liquidity; verify wash-trading risk".into(),
            );
        }
        if let (Some(b), Some(s)) = (buys, sells) {
            if b.saturating_add(s) > 100 && (b as f64) / (s.max(1) as f64) > 8.0 {
                risk_flags.push("Buy/sell count is unusually one-sided; transaction counts do not prove organic demand".into());
            }
        }

        // Transparent heuristic v1 (not a probability): bounded price acceleration plus liquidity-adjusted activity.
        let m5 = change_5m.unwrap_or(0.0);
        let h1 = change_1h.unwrap_or(0.0);
        let h24 = change_24h.unwrap_or(0.0);
        let activity = if liquidity > 0.0 {
            (turnover.max(0.0).ln_1p() * 8.0).clamp(0.0, 20.0)
        } else {
            0.0
        };
        let transaction_flow = match (buys, sells) {
            (Some(buy_count), Some(sell_count)) if buy_count.saturating_add(sell_count) >= 20 => {
                let total = buy_count.saturating_add(sell_count).max(1) as f64;
                (((buy_count as f64 - sell_count as f64) / total) * 8.0).clamp(-8.0, 8.0)
            }
            _ => 0.0,
        };
        let raw_momentum = 35.0
            + m5.clamp(-20.0, 20.0) * 0.65
            + h1.clamp(-50.0, 50.0) * 0.45
            + h24.clamp(-100.0, 100.0) * 0.12
            + activity
            + transaction_flow;
        let momentum_score = raw_momentum.round().clamp(0.0, 100.0) as u8;
        let mut risk: f64 = 20.0;
        risk += if liquidity < 10_000.0 {
            35.0
        } else if liquidity < 25_000.0 {
            27.0
        } else if liquidity < 100_000.0 {
            16.0
        } else if liquidity < 500_000.0 {
            8.0
        } else {
            0.0
        };
        risk += match age_minutes {
            Some(age) if age < 60 => 28.0,
            Some(age) if age < 24 * 60 => 18.0,
            Some(age) if age < 7 * 24 * 60 => 8.0,
            Some(_) => 0.0,
            None => 12.0,
        };
        if turnover > 20.0 {
            risk += 12.0;
        } else if turnover > 8.0 {
            risk += 6.0;
        }
        if market_cap.is_none() {
            risk += 5.0;
        }
        if change_1h.is_none() || change_24h.is_none() {
            risk += 10.0;
        }
        if buys.is_none() || sells.is_none() {
            risk += 5.0;
        }
        let risk_score = risk.round().clamp(0.0, 100.0) as u8;
        let insufficient =
            liquidity <= 0.0 || change_1h.is_none() || change_24h.is_none() || volume <= 0.0;
        let status = if insufficient {
            "Insufficient Data"
        } else if risk_score >= 65 {
            "High Risk"
        } else if momentum_score >= 65 {
            "Momentum"
        } else {
            "Watch"
        };
        let display_chain = match chain.as_str() {
            "solana" => "SOL",
            "ethereum" => "ETH",
            "base" => "BASE",
            "bsc" => "BNB",
            "arbitrum" => "ARB",
            "polygon" => "POLYGON",
            "optimism" => "OP",
            "avalanche" => "AVAX",
            _ => chain.as_str(),
        };
        tokens.push(ScreenerToken {
            symbol: if symbol.is_empty() {
                "UNKNOWN".into()
            } else {
                symbol.chars().take(32).collect()
            },
            name: if name.is_empty() {
                symbol.to_string()
            } else {
                name.chars().take(100).collect()
            },
            chain: display_chain.into(),
            price,
            change_1h_pct: h1,
            change_24h_pct: h24,
            volume_24h: volume,
            liquidity,
            momentum_score,
            risk_score,
            status: status.into(),
            source: "DEX Screener public API · latest boost feed".into(),
            updated_at_ms: now,
            risk_flags,
            token_address: Some(address.to_string()),
            pair_address,
            pair_url,
            price_change_5m_pct: change_5m,
            market_cap_usd: market_cap,
            fdv_usd: fdv,
            age_minutes,
            buys_24h: buys,
            sells_24h: sells,
            boost_active: active_boosts,
            ruleset_version: "momentum-risk-v1".into(),
        });
    }
    if tokens.is_empty() {
        let detail = if errors.is_empty() {
            "No valid token pools were returned for the current boosted-token sample.".to_string()
        } else {
            errors.join("; ")
        };
        return core
            .record_screener_failure(format!("{detail} Previous screener rows were retained."));
    }
    tokens.sort_by(|a, b| {
        b.momentum_score
            .cmp(&a.momentum_score)
            .then_with(|| b.liquidity.total_cmp(&a.liquidity))
    });
    if tokens.len() > 150 {
        tokens.truncate(150);
    }
    let snapshot = core.update_screener_data(tokens)?;
    if !errors.is_empty() {
        core.record_screener_partial(errors.len(), errors.join("; "))?;
        return core.snapshot();
    }
    Ok(snapshot)
}

pub async fn refresh_public_market_data(core: &TradingCore) -> Result<DashboardSnapshot, String> {
    let _refresh_lease = claim_refresh_slot(
        &LAST_MARKET_REFRESH_MS,
        &MARKET_REFRESH_IN_FLIGHT,
        Utc::now().timestamp_millis(),
        3_000,
        "Market-data",
    )?;
    let client = match Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_secs(8))
        .connect_timeout(Duration::from_secs(4))
        .user_agent("AgenticTradingOS/0.11.0 (public-market-data; no-order-permissions)")
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return core.record_market_data_failure(format!(
                "Unable to initialize market-data client: {error}"
            ))
        }
    };
    let symbols = ["BTCUSDT", "ETHUSDT", "SOLUSDT", "BNBUSDT", "DOGEUSDT"];
    let mut quotes: Vec<Quote> = Vec::with_capacity(symbols.len());
    let mut failures: Vec<String> = Vec::new();
    // One bounded request for the explicitly selected symbols avoids five serial requests
    // and reduces latency/rate-limit pressure compared with refreshing each ticker separately.
    let symbols_json = serde_json::to_string(&symbols)
        .map_err(|e| format!("Unable to encode public market-data symbol list: {e}"))?;
    let ticker_response = client
        .get("https://api.binance.com/api/v3/ticker/24hr")
        .query(&[("symbols", symbols_json.as_str())])
        .send()
        .await;
    let tickers: Vec<BinanceTicker> = match ticker_response {
        Ok(response) => match response.error_for_status() {
            Ok(response) => {
                match parse_json_bounded::<Vec<BinanceTicker>>(response, 1_048_576).await {
                    Ok(parsed) => parsed,
                    Err(error) => {
                        return core.record_market_data_failure(format!(
                            "Binance ticker payload was rejected: {error}"
                        ));
                    }
                }
            }
            Err(error) => {
                return core.record_market_data_failure(format!(
                    "Binance ticker endpoint returned an unsuccessful status: {error}"
                ))
            }
        },
        Err(error) => {
            return core
                .record_market_data_failure(format!("Binance ticker request failed: {error}"))
        }
    };
    // Compare exchange event time against response receipt, not request start time.
    let response_received_at_ms = Utc::now().timestamp_millis();

    for symbol in symbols {
        let Some(ticker) = tickers.iter().find(|ticker| ticker.symbol == symbol) else {
            failures.push(format!("{symbol}: provider omitted requested symbol"));
            continue;
        };
        let parsed = (
            ticker.last_price.parse::<f64>(),
            ticker.price_change_percent.parse::<f64>(),
            ticker.quote_volume.parse::<f64>(),
        );
        let (Ok(price), Ok(change), Ok(volume)) = parsed else {
            failures.push(format!(
                "{symbol}: provider included a non-numeric ticker value"
            ));
            continue;
        };
        if !price.is_finite()
            || price <= 0.0
            || !change.is_finite()
            || !volume.is_finite()
            || volume < 0.0
        {
            failures.push(format!(
                "{symbol}: provider included a non-finite or out-of-range ticker value"
            ));
            continue;
        }
        if ticker.close_time <= 0
            || ticker.close_time > response_received_at_ms
            || response_received_at_ms.saturating_sub(ticker.close_time)
                > MARKET_DATA_OBSERVATION_MAX_AGE_MS
        {
            failures.push(format!(
                "{symbol}: exchange observation timestamp is stale, future-dated, or invalid"
            ));
            continue;
        }
        quotes.push(Quote {
            symbol: symbol.into(),
            price,
            change_24h_pct: change,
            volume_24h: volume,
            source: "Binance public REST".into(),
            updated_at_ms: ticker.close_time,
            retrieved_at_ms: 0,
            is_live: true,
        });
    }

    let mut btc_candles: Option<Vec<Candle>> = None;
    if quotes.iter().any(|q| q.symbol == "BTCUSDT") {
        let candle_result = async {
            let response = client
                .get("https://api.binance.com/api/v3/klines")
                .query(&[("symbol", "BTCUSDT"), ("interval", "5m"), ("limit", "120")])
                .send()
                .await
                .map_err(|e| format!("network error: {e}"))?
                .error_for_status()
                .map_err(|e| format!("provider returned an unsuccessful status: {e}"))?;
            let rows: Vec<Vec<Value>> = parse_json_bounded(response, 2 * 1_048_576)
                .await
                .map_err(|e| format!("invalid candle response: {e}"))?;
            parse_closed_binance_candles(rows, Utc::now().timestamp_millis())
        }
        .await;
        match candle_result {
            Ok(candles) => btc_candles = Some(candles),
            Err(error) => failures.push(format!("BTCUSDT klines: {error}")),
        }
    }

    if quotes.is_empty() {
        let reason = if failures.is_empty() {
            "No market quotes were returned.".to_string()
        } else {
            failures.join("; ")
        };
        return core.record_market_data_failure(reason);
    }
    let status = if failures.is_empty() {
        "CONNECTED · PUBLIC MARKET DATA".to_string()
    } else {
        format!("PARTIAL · {} provider warning(s)", failures.len())
    };
    let source = "Binance public REST · spot tickers and BTCUSDT 5m candles".to_string();
    let snapshot = core.update_market_data(quotes, btc_candles, status, source)?;
    if !failures.is_empty() {
        let _ = core.record_event(
            "WARN",
            "market-data",
            "Some market data requests failed",
            failures.join("; "),
        );
        return core.snapshot();
    }
    Ok(snapshot)
}

fn parse_closed_binance_candles(rows: Vec<Vec<Value>>, now_ms: i64) -> Result<Vec<Candle>, String> {
    const INTERVAL_MS: i64 = 300_000;
    let row_count = rows.len();
    let mut converted = Vec::with_capacity(row_count);
    for (index, row) in rows.into_iter().enumerate() {
        // Malformed rows must invalidate the candle batch rather than be skipped and
        // potentially disguise an upstream omission as a complete historical series.
        if row.len() < 7 {
            return Err(format!("Binance candle row {} is truncated.", index + 1));
        }
        let time = row[0].as_i64().ok_or_else(|| {
            format!(
                "Binance candle row {} has an invalid open timestamp.",
                index + 1
            )
        })?;
        let close_time = row[6].as_i64().ok_or_else(|| {
            format!(
                "Binance candle row {} has an invalid close timestamp.",
                index + 1
            )
        })?;
        if time <= 0
            || time % INTERVAL_MS != 0
            || time.checked_add(INTERVAL_MS - 1) != Some(close_time)
        {
            return Err(format!(
                "Binance candle row {} has an invalid 5m timestamp grid or close time.",
                index + 1
            ));
        }
        if close_time >= now_ms {
            // Exactly one still-forming candle may appear, and only at the end of the provider
            // response. The caller never receives it as historical data.
            if index + 1 == row_count && time <= now_ms {
                continue;
            }
            return Err(format!("Unclosed or future Binance candle appeared before the end of the response at row {}.", index + 1));
        }
        let parse_num = |column: usize| row.get(column).and_then(value_to_f64);
        let (Some(open), Some(high), Some(low), Some(close), Some(volume)) = (
            parse_num(1),
            parse_num(2),
            parse_num(3),
            parse_num(4),
            parse_num(5),
        ) else {
            return Err(format!(
                "Binance candle row {} contains an invalid OHLCV number.",
                index + 1
            ));
        };
        converted.push(Candle {
            time,
            open,
            high,
            low,
            close,
            volume,
        });
    }
    validate_candle_series(&converted, CandleInterval::FiveMinutes, 30).map_err(|error| {
        format!("Binance BTCUSDT candle series failed integrity validation: {error}")
    })?;
    Ok(converted)
}

fn value_to_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()))
}

fn screener_identity(chain: &str, address: &str) -> String {
    let normalized_address = match chain.to_ascii_lowercase().as_str() {
        "ethereum" | "base" | "bsc" | "arbitrum" | "polygon" | "optimism" | "avalanche" => {
            address.to_ascii_lowercase()
        }
        _ => address.to_string(),
    };
    format!("{}:{normalized_address}", chain.to_ascii_lowercase())
}

#[cfg(test)]
mod refresh_throttle_tests {
    use super::{append_bounded_chunk, claim_refresh_slot};
    use serde_json::Value;
    use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

    #[test]
    fn refresh_slot_enforces_minimum_interval_and_releases_after_throttle_rejection() {
        let last = AtomicI64::new(0);
        let in_flight = AtomicBool::new(false);
        let first = claim_refresh_slot(&last, &in_flight, 20_000, 3_000, "market-data").unwrap();
        drop(first);
        assert!(claim_refresh_slot(&last, &in_flight, 22_999, 3_000, "market-data").is_err());
        assert!(
            !in_flight.load(Ordering::Acquire),
            "a throttled attempt must release its in-flight lease"
        );
        assert!(claim_refresh_slot(&last, &in_flight, 23_000, 3_000, "market-data").is_ok());
    }

    #[test]
    fn refresh_slot_rejects_overlapping_requests_even_after_rate_window_elapsed() {
        let last = AtomicI64::new(0);
        let in_flight = AtomicBool::new(false);
        let first = claim_refresh_slot(&last, &in_flight, 20_000, 3_000, "market-data").unwrap();
        assert!(claim_refresh_slot(&last, &in_flight, 30_000, 3_000, "market-data").is_err());
        drop(first);
        assert!(claim_refresh_slot(&last, &in_flight, 30_000, 3_000, "market-data").is_ok());
    }

    #[test]
    fn refresh_slot_fails_closed_when_clock_moves_backwards_and_releases_lease() {
        let last = AtomicI64::new(50_000);
        let in_flight = AtomicBool::new(false);
        assert!(claim_refresh_slot(&last, &in_flight, 49_999, 3_000, "market-data").is_err());
        assert_eq!(last.load(Ordering::Acquire), 50_000);
        assert!(!in_flight.load(Ordering::Acquire));
    }

    #[test]
    fn provider_body_limit_accepts_boundary_and_rejects_overflow_atomically() {
        let mut body = Vec::new();
        assert!(append_bounded_chunk(&mut body, b"1234", 6).is_ok());
        assert!(append_bounded_chunk(&mut body, b"56", 6).is_ok());
        assert_eq!(body, b"123456");
        assert!(append_bounded_chunk(&mut body, b"7", 6).is_err());
        assert_eq!(
            body, b"123456",
            "over-limit chunk must not be appended partially"
        );
    }

    fn closed_binance_rows(start: i64, count: usize) -> Vec<Vec<Value>> {
        (0..count)
            .map(|index| {
                let time = start + index as i64 * 300_000;
                serde_json::json!([
                    time,
                    "100.0",
                    "102.0",
                    "99.0",
                    "101.0",
                    "42.0",
                    time + 299_999
                ])
                .as_array()
                .unwrap()
                .clone()
            })
            .collect()
    }

    #[test]
    fn binance_parser_rejects_internal_gap_instead_of_returning_partial_candles() {
        let start = 1_700_000_100_000_i64;
        let mut rows = closed_binance_rows(start, 31);
        rows.remove(12);
        let error = super::parse_closed_binance_candles(rows, start + 40 * 300_000).unwrap_err();
        assert!(error.contains("Candle cadence violation"));
        assert!(error.contains("1 candle interval(s) missing"));
    }

    #[test]
    fn binance_parser_rejects_duplicate_and_out_of_order_rows_instead_of_normalizing_them() {
        let start = 1_700_000_100_000_i64;
        let now_ms = start + 60 * 300_000;

        let mut duplicate = closed_binance_rows(start, 30);
        let duplicate_time = duplicate[14][0].as_i64().unwrap();
        duplicate[15][0] = serde_json::json!(duplicate_time);
        duplicate[15][6] = serde_json::json!(duplicate_time + 299_999);
        assert!(super::parse_closed_binance_candles(duplicate, now_ms)
            .unwrap_err()
            .contains("Candle cadence violation"));

        let mut out_of_order = closed_binance_rows(start, 30);
        out_of_order.swap(10, 11);
        assert!(super::parse_closed_binance_candles(out_of_order, now_ms)
            .unwrap_err()
            .contains("Candle cadence violation"));
    }

    #[test]
    fn binance_parser_rejects_malformed_ohlcv_instead_of_skipping_rows() {
        let start = 1_700_000_100_000_i64;
        let mut rows = closed_binance_rows(start, 30);
        rows[7][1] = serde_json::json!("not-a-price");
        let error = super::parse_closed_binance_candles(rows, start + 60 * 300_000).unwrap_err();
        assert!(error.contains("invalid OHLCV number"));
    }

    #[test]
    fn binance_backtest_data_excludes_the_still_forming_candle() {
        let start = 1_700_000_100_000_i64;
        let mut rows = (0..30)
            .map(|index| {
                let time = start + index * 300_000;
                serde_json::json!([
                    time,
                    "100.0",
                    "102.0",
                    "99.0",
                    "101.0",
                    "42.0",
                    time + 299_999
                ])
                .as_array()
                .unwrap()
                .clone()
            })
            .collect::<Vec<_>>();
        let forming_time = start + 30 * 300_000;
        rows.push(
            serde_json::json!([
                forming_time,
                "101.0",
                "103.0",
                "100.0",
                "102.0",
                "12.0",
                forming_time + 299_999
            ])
            .as_array()
            .unwrap()
            .clone(),
        );
        let now_ms = forming_time + 100_000;
        let candles = super::parse_closed_binance_candles(rows, now_ms).unwrap();
        assert_eq!(candles.len(), 30);
        assert_eq!(candles.last().unwrap().time, start + 29 * 300_000);
        assert!(candles.iter().all(|candle| candle.time < forming_time));
    }
}
