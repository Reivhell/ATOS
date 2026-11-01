//! Application service that derives strict-analysis inputs from Rust-owned provider state.
use crate::{
    domain::models::{
        maximum_live_candle_age_ms, validate_candle_freshness, CandleInterval, CandleSeriesRequest,
        CandleSeriesResponse, CANDLE_PROVIDER_RETRIEVAL_MAX_AGE_MS,
    },
    runtime::trading_core::TradingCore,
};
use chrono::Utc;

fn strict_interval_from_text(value: &str) -> Result<CandleInterval, String> {
    match value.trim() {
        "1m" => Ok(CandleInterval::OneMinute),
        "5m" => Ok(CandleInterval::FiveMinutes),
        "15m" => Ok(CandleInterval::FifteenMinutes),
        "1h" => Ok(CandleInterval::OneHour),
        "4h" => Ok(CandleInterval::FourHours),
        "1d" => Ok(CandleInterval::OneDay),
        _ => Err("Strict analysis timeframe must be one of 1m, 5m, 15m, 1h, 4h, or 1d.".into()),
    }
}

/// Only a fresh, code-owned provider response can seed the strict analysis contract.
/// Caller-provided source labels, `validated` booleans, hashes, and candle arrays are not authority.
pub(crate) fn authoritative_strict_series(
    core: &TradingCore,
    symbol: &str,
    timeframe: &str,
) -> Result<CandleSeriesResponse, String> {
    let interval = strict_interval_from_text(timeframe)?;
    let series = core.get_candle_series(CandleSeriesRequest {
        symbol: symbol.to_string(),
        interval,
    })?;
    let now = Utc::now().timestamp_millis();
    if !series.is_live
        || !series.provider_status.starts_with("CONNECTED")
        || !(series.source == "Binance public REST" || series.source.starts_with("Binance public REST · "))
        || series.retrieved_at_ms <= 0
        || series.retrieved_at_ms > now.saturating_add(30_000)
        || now.saturating_sub(series.retrieved_at_ms) > CANDLE_PROVIDER_RETRIEVAL_MAX_AGE_MS
        || validate_candle_freshness(
            &series.candles,
            interval,
            now,
            maximum_live_candle_age_ms(interval),
        )
        .is_err()
    {
        return Err("Strict analysis requires a fresh connected Rust-owned Binance public-data snapshot; supplied or browser-preview metadata is not accepted.".into());
    }
    Ok(series)
}
