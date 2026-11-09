use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candle {
    /// UTC timestamp of the bar open. Close time is derived from the requested interval.
    pub time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandleInterval {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "4h")]
    FourHours,
    #[serde(rename = "1d")]
    OneDay,
}

impl CandleInterval {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::FifteenMinutes => "15m",
            Self::OneHour => "1h",
            Self::FourHours => "4h",
            Self::OneDay => "1d",
        }
    }

    pub fn duration_ms(self) -> i64 {
        match self {
            Self::OneMinute => 60_000,
            Self::FiveMinutes => 300_000,
            Self::FifteenMinutes => 900_000,
            Self::OneHour => 3_600_000,
            Self::FourHours => 14_400_000,
            Self::OneDay => 86_400_000,
        }
    }
}

pub const CANDLE_CLOSE_FRESHNESS_GRACE_MS: i64 = 30_000;
pub const MARKET_DATA_OBSERVATION_MAX_AGE_MS: i64 = 120_000;
pub const CANDLE_PROVIDER_RETRIEVAL_MAX_AGE_MS: i64 = MARKET_DATA_OBSERVATION_MAX_AGE_MS;

pub fn candle_close_at_ms(bar_open_at_ms: i64, interval: CandleInterval) -> Result<i64, String> {
    if bar_open_at_ms <= 0 {
        return Err("Candle open timestamp must be positive.".into());
    }
    bar_open_at_ms
        .checked_add(interval.duration_ms().saturating_sub(1))
        .ok_or_else(|| "Candle close timestamp overflowed.".into())
}

pub fn latest_closed_bar_at_ms(
    candles: &[Candle],
    interval: CandleInterval,
) -> Result<i64, String> {
    let latest = candles
        .last()
        .ok_or_else(|| "Candle series is empty.".to_string())?;
    candle_close_at_ms(latest.time, interval)
}

pub fn validate_candle_freshness(
    candles: &[Candle],
    interval: CandleInterval,
    now_ms: i64,
    maximum_age_ms: i64,
) -> Result<(i64, i64), String> {
    if now_ms <= 0 || maximum_age_ms < 0 {
        return Err("Candle freshness validation received an invalid clock or age limit.".into());
    }
    let close_at_ms = latest_closed_bar_at_ms(candles, interval)?;
    if close_at_ms >= now_ms {
        return Err("Latest candle is still forming or has a future close timestamp.".into());
    }
    let age_ms = now_ms.saturating_sub(close_at_ms);
    if age_ms > maximum_age_ms {
        return Err(format!(
            "Latest fully closed candle is stale: age {age_ms}ms exceeds the {maximum_age_ms}ms limit."
        ));
    }
    Ok((close_at_ms, age_ms))
}

pub fn maximum_live_candle_age_ms(interval: CandleInterval) -> i64 {
    interval
        .duration_ms()
        .saturating_add(CANDLE_CLOSE_FRESHNESS_GRACE_MS)
}

/// Validates a canonical OHLCV series before the native engine persists or serves it.
/// Continuous markets must have aligned timestamps and exactly one interval between bars;
/// sorting or de-duplicating malformed provider data would conceal upstream integrity failures.
pub(crate) fn validate_candle_series(
    candles: &[Candle],
    interval: CandleInterval,
    minimum_count: usize,
) -> Result<(), String> {
    if candles.len() < minimum_count {
        return Err(format!(
            "Only {} candle(s) supplied; at least {minimum_count} are required.",
            candles.len()
        ));
    }
    let interval_ms = interval.duration_ms();
    let mut previous_time: Option<i64> = None;
    for (index, candle) in candles.iter().enumerate() {
        if candle.time <= 0
            || !candle.open.is_finite()
            || !candle.high.is_finite()
            || !candle.low.is_finite()
            || !candle.close.is_finite()
            || !candle.volume.is_finite()
            || candle.open <= 0.0
            || candle.high <= 0.0
            || candle.low <= 0.0
            || candle.close <= 0.0
            || candle.volume < 0.0
            || candle.high < candle.open.max(candle.close).max(candle.low)
            || candle.low > candle.open.min(candle.close).min(candle.high)
        {
            return Err(format!(
                "Candle row {} violates OHLCV invariants.",
                index + 1
            ));
        }
        if candle.time % interval_ms != 0 {
            return Err(format!(
                "Candle row {} timestamp is not aligned to the {} UTC interval grid.",
                index + 1,
                interval.as_str()
            ));
        }
        candle_close_at_ms(candle.time, interval).map_err(|reason| {
            format!(
                "Candle row {} has an invalid close timestamp: {reason}",
                index + 1
            )
        })?;
        if let Some(previous) = previous_time {
            let observed = candle.time.checked_sub(previous).ok_or_else(|| {
                format!("Candle timestamp delta overflowed at row {}.", index + 1)
            })?;
            if observed != interval_ms {
                let missing = if observed > interval_ms && observed % interval_ms == 0 {
                    format!(
                        " ({} candle interval(s) missing)",
                        observed / interval_ms - 1
                    )
                } else {
                    String::new()
                };
                return Err(format!(
                    "Candle cadence violation at row {}: expected {interval_ms}ms, received {observed}ms{missing}.",
                    index + 1
                ));
            }
        }
        previous_time = Some(candle.time);
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandleSeriesRequest {
    pub symbol: String,
    pub interval: CandleInterval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandleSeriesResponse {
    pub request: CandleSeriesRequest,
    pub source: String,
    pub provider_status: String,
    /// Legacy alias for the exact candle-series retrieval timestamp.
    pub updated_at_ms: i64,
    pub retrieved_at_ms: i64,
    pub latest_closed_bar_at_ms: i64,
    pub dataset_sha256: String,
    pub is_live: bool,
    pub candles: Vec<Candle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandleSeriesCapability {
    pub symbol: String,
    pub interval: CandleInterval,
    pub supported: bool,
    pub available: bool,
    pub is_live: bool,
    pub candle_count: usize,
    pub latest_candle_time_ms: Option<i64>,
    pub latest_closed_bar_at_ms: Option<i64>,
    pub retrieved_at_ms: i64,
    pub source: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataCapabilities {
    pub quote_symbols: Vec<String>,
    pub candle_series: Vec<CandleSeriesCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    pub symbol: String,
    pub price: f64,
    pub change_24h_pct: f64,
    pub volume_24h: f64,
    pub source: String,
    pub updated_at_ms: i64,
    #[serde(default)]
    pub retrieved_at_ms: i64,
    pub is_live: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub average_entry: f64,
    pub mark_price: f64,
    pub notional: f64,
    pub unrealized_pnl: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskCheck {
    pub rule: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperOrder {
    pub id: String,
    #[serde(default)]
    pub client_order_id: Option<String>,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub requested_price: f64,
    #[serde(default)]
    pub requested_limit_price: Option<f64>,
    pub fill_price: Option<f64>,
    pub notional: f64,
    pub fee: f64,
    pub realized_pnl: f64,
    pub status: String,
    pub created_at_ms: i64,
    pub risk_checks: Vec<RiskCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub status: String,
    pub mode: String,
    pub objective: String,
    pub last_decision: String,
    pub pnl: f64,
    pub tools: Vec<String>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub id: String,
    pub timestamp_ms: i64,
    pub level: String,
    pub category: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenerToken {
    pub symbol: String,
    pub name: String,
    pub chain: String,
    pub price: f64,
    pub change_1h_pct: f64,
    pub change_24h_pct: f64,
    pub volume_24h: f64,
    pub liquidity: f64,
    pub momentum_score: u8,
    pub risk_score: u8,
    pub status: String,
    pub source: String,
    pub updated_at_ms: i64,
    pub risk_flags: Vec<String>,
    #[serde(default)]
    pub token_address: Option<String>,
    #[serde(default)]
    pub pair_address: Option<String>,
    #[serde(default)]
    pub pair_url: Option<String>,
    #[serde(default)]
    pub price_change_5m_pct: Option<f64>,
    #[serde(default)]
    pub market_cap_usd: Option<f64>,
    #[serde(default)]
    pub fdv_usd: Option<f64>,
    #[serde(default)]
    pub age_minutes: Option<u64>,
    #[serde(default)]
    pub buys_24h: Option<u64>,
    #[serde(default)]
    pub sells_24h: Option<u64>,
    #[serde(default)]
    pub boost_active: Option<u64>,
    #[serde(default = "default_screener_ruleset")]
    pub ruleset_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskLimits {
    pub max_order_notional: f64,
    pub max_position_notional: f64,
    pub max_daily_loss: f64,
    pub fee_bps: f64,
    pub slippage_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub mode: String,
    pub initial_balance: f64,
    pub cash_balance: f64,
    pub equity: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub day_change_pct: f64,
    pub max_drawdown_pct: f64,
    /// `None` until at least one closed paper trade exists; avoid encoding "unknown" as 0%.
    pub win_rate_pct: Option<f64>,
    /// Number of closed paper trades used to calculate the win-rate statistic.
    pub closed_trade_count: usize,
    pub positions: Vec<Position>,
    pub orders: Vec<PaperOrder>,
    pub agents: Vec<Agent>,
    pub events: Vec<AuditEvent>,
    pub quotes: Vec<Quote>,
    pub candles: Vec<Candle>,
    pub market_data_capabilities: MarketDataCapabilities,
    pub screener: Vec<ScreenerToken>,
    #[serde(default = "default_screener_status")]
    pub screener_provider_status: String,
    #[serde(default = "default_screener_source")]
    pub screener_provider_source: String,
    #[serde(default)]
    pub screener_provider_updated_at_ms: i64,
    pub kill_switch_engaged: bool,
    pub provider_status: String,
    pub provider_source: String,
    pub provider_updated_at_ms: i64,
    pub risk_limits: RiskLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOrderRequest {
    pub symbol: String,
    #[serde(default)]
    pub client_order_id: Option<String>,
    pub side: String,
    pub quantity: f64,
    pub limit_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestRequest {
    pub symbol: String,
    pub fast_period: usize,
    pub slow_period: usize,
    pub initial_capital: f64,
    pub fee_bps: f64,
    pub slippage_bps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestPoint {
    pub time: i64,
    pub equity: f64,
    pub benchmark: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestCostStressScenario {
    pub name: String,
    pub fee_bps: f64,
    pub slippage_bps: f64,
    pub fee_cap_reached: bool,
    pub slippage_cap_reached: bool,
    pub final_equity: f64,
    pub total_return_pct: f64,
    pub return_delta_percentage_points: f64,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub trade_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestOutOfSampleFold {
    pub fold_index: usize,
    pub start_ms: i64,
    pub end_ms: i64,
    pub candles_evaluated: usize,
    pub final_equity: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub trade_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BacktestResult {
    pub id: String,
    pub symbol: String,
    pub strategy_name: String,
    pub fast_period: usize,
    pub slow_period: usize,
    pub initial_capital: f64,
    pub final_equity: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    /// `None` when no completed strategy trades exist.
    pub win_rate_pct: Option<f64>,
    /// `None` when there are no losing trades; interpret with gross_profit/gross_loss.
    pub profit_factor: Option<f64>,
    pub gross_profit: f64,
    pub gross_loss: f64,
    pub sharpe_ratio: f64,
    pub trade_count: usize,
    pub fee_bps: f64,
    pub slippage_bps: f64,
    pub candles_used: usize,
    pub data_source: String,
    /// SHA-256 of the canonical candle encoding documented by dataset_encoding_version.
    pub dataset_sha256: String,
    /// Version 1 encodes candle count, timestamp, and OHLCV IEEE-754 bits in little-endian order.
    pub dataset_encoding_version: u32,
    pub dataset_start_ms: i64,
    pub dataset_end_ms: i64,
    pub median_candle_interval_ms: i64,
    pub equity_curve: Vec<BacktestPoint>,
    /// Deterministic fee/slippage shock scenarios; these are sensitivity tests, not liquidity forecasts.
    pub cost_stress_scenarios: Vec<BacktestCostStressScenario>,
    /// Chronological, independent out-of-sample windows; each uses prior bars only for indicator warm-up.
    pub out_of_sample_folds: Vec<BacktestOutOfSampleFold>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineState {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    pub cash_balance: f64,
    #[serde(default = "default_daily_start_equity")]
    pub daily_start_equity: f64,
    #[serde(default)]
    pub daily_start_date: String,
    #[serde(default)]
    pub max_drawdown_pct: f64,
    pub initial_balance: f64,
    pub orders: Vec<PaperOrder>,
    pub positions: BTreeMap<String, Position>,
    pub agents: Vec<Agent>,
    pub events: Vec<AuditEvent>,
    pub quotes: BTreeMap<String, Quote>,
    pub candles: BTreeMap<String, Vec<Candle>>,
    #[serde(default)]
    pub candle_retrieved_at_ms: i64,
    pub screener: Vec<ScreenerToken>,
    #[serde(default = "default_screener_status")]
    pub screener_provider_status: String,
    #[serde(default = "default_screener_source")]
    pub screener_provider_source: String,
    #[serde(default)]
    pub screener_provider_updated_at_ms: i64,
    pub kill_switch_engaged: bool,
    pub provider_status: String,
    pub provider_source: String,
    pub provider_updated_at_ms: i64,
    pub risk_limits: RiskLimits,
    pub equity_peak: f64,
}

fn default_schema_version() -> u32 {
    1
}
fn default_daily_start_equity() -> f64 {
    10_000.0
}
fn default_screener_status() -> String {
    "NOT CONNECTED · NO OBSERVATIONS".into()
}
fn default_screener_source() -> String {
    "No screener provider observation loaded".into()
}
fn default_screener_ruleset() -> String {
    "unavailable".into()
}

#[cfg(test)]
mod candle_series_validation_tests {
    use super::{
        latest_closed_bar_at_ms, maximum_live_candle_age_ms, validate_candle_freshness,
        validate_candle_series, Candle, CandleInterval,
    };

    fn valid_series(count: usize) -> Vec<Candle> {
        let start = 1_700_000_100_000_i64;
        (0..count)
            .map(|index| {
                let time = start + index as i64 * 300_000;
                Candle {
                    time,
                    open: 100.0,
                    high: 102.0,
                    low: 99.0,
                    close: 101.0,
                    volume: 10.0,
                }
            })
            .collect()
    }

    #[test]
    fn canonical_continuous_series_is_accepted() {
        assert!(validate_candle_series(&valid_series(30), CandleInterval::FiveMinutes, 30).is_ok());
    }

    #[test]
    fn one_internal_gap_is_rejected_even_when_most_intervals_are_valid() {
        let mut candles = valid_series(40);
        for candle in candles.iter_mut().skip(20) {
            candle.time += 300_000;
        }
        let error = validate_candle_series(&candles, CandleInterval::FiveMinutes, 30).unwrap_err();
        assert!(error.contains("Candle cadence violation"));
        assert!(error.contains("1 candle interval(s) missing"));
    }

    #[test]
    fn duplicate_out_of_order_and_unaligned_timestamps_are_rejected() {
        let mut duplicate = valid_series(30);
        duplicate[10].time = duplicate[9].time;
        assert!(validate_candle_series(&duplicate, CandleInterval::FiveMinutes, 30).is_err());

        let mut unordered = valid_series(30);
        unordered.swap(10, 11);
        assert!(validate_candle_series(&unordered, CandleInterval::FiveMinutes, 30).is_err());

        let mut unaligned = valid_series(30);
        unaligned[0].time += 1;
        assert!(
            validate_candle_series(&unaligned, CandleInterval::FiveMinutes, 30)
                .unwrap_err()
                .contains("not aligned")
        );
    }

    #[test]
    fn malformed_ohlcv_and_short_history_fail_closed() {
        assert!(
            validate_candle_series(&valid_series(29), CandleInterval::FiveMinutes, 30)
                .unwrap_err()
                .contains("at least 30")
        );
        let mut malformed = valid_series(30);
        malformed[4].high = malformed[4].low - 1.0;
        assert!(
            validate_candle_series(&malformed, CandleInterval::FiveMinutes, 30)
                .unwrap_err()
                .contains("OHLCV invariants")
        );
    }

    #[test]
    fn candle_close_time_is_distinct_from_bar_open_and_retrieval_time() {
        let candles = valid_series(30);
        let expected_close = candles.last().unwrap().time + 299_999;
        assert_eq!(
            latest_closed_bar_at_ms(&candles, CandleInterval::FiveMinutes).unwrap(),
            expected_close
        );
        let now = expected_close + 5_000;
        assert_eq!(
            validate_candle_freshness(
                &candles,
                CandleInterval::FiveMinutes,
                now,
                maximum_live_candle_age_ms(CandleInterval::FiveMinutes)
            )
            .unwrap(),
            (expected_close, 5_000),
        );
    }

    #[test]
    fn recent_retrieval_does_not_make_a_replayed_bar_fresh() {
        let candles = valid_series(30);
        let latest_close = latest_closed_bar_at_ms(&candles, CandleInterval::FiveMinutes).unwrap();
        let now = latest_close + maximum_live_candle_age_ms(CandleInterval::FiveMinutes) + 1;
        let recent_retrieval_at_ms = now - 1_000;
        assert!(recent_retrieval_at_ms > latest_close);
        assert!(validate_candle_freshness(
            &candles,
            CandleInterval::FiveMinutes,
            now,
            maximum_live_candle_age_ms(CandleInterval::FiveMinutes)
        )
        .unwrap_err()
        .contains("stale"));
    }

    #[test]
    fn forming_future_and_overflowing_candle_close_times_fail_closed() {
        let candles = valid_series(30);
        let close = latest_closed_bar_at_ms(&candles, CandleInterval::FiveMinutes).unwrap();
        assert!(
            validate_candle_freshness(&candles, CandleInterval::FiveMinutes, close, i64::MAX)
                .unwrap_err()
                .contains("still forming")
        );
        assert!(super::candle_close_at_ms(i64::MAX, CandleInterval::FiveMinutes).is_err());
        assert!(validate_candle_freshness(&[], CandleInterval::FiveMinutes, 1, 1).is_err());

        let interval_ms = CandleInterval::FiveMinutes.duration_ms();
        let overflowing_open = i64::MAX.div_euclid(interval_ms) * interval_ms;
        let overflow_series = (0..30)
            .map(|index| Candle {
                time: overflowing_open - (29 - index) * interval_ms,
                open: 100.0,
                high: 102.0,
                low: 99.0,
                close: 101.0,
                volume: 10.0,
            })
            .collect::<Vec<_>>();
        assert!(
            validate_candle_series(&overflow_series, CandleInterval::FiveMinutes, 30)
                .unwrap_err()
                .contains("invalid close timestamp")
        );
    }
}
