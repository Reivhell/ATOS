use serde::{Deserialize, Serialize};

use crate::domain::{
    models::Candle,
    research_smc::{self, ClassifiedSwing, StructureBreak},
};

const MAX_CANDLES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndicatorReading {
    pub key: String,
    pub title: String,
    pub value: String,
    pub detail: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeStructurePivot {
    pub kind: String,
    pub classification: String,
    pub pivot_index: usize,
    pub confirmed_at_index: usize,
    pub pivot_at_ms: i64,
    pub confirmed_at_ms: i64,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeStructureBreak {
    pub direction: String,
    pub classification: String,
    pub level_kind: String,
    pub level: f64,
    pub pivot_index: usize,
    pub level_confirmed_at_index: usize,
    pub broken_at_index: usize,
    pub broken_at_ms: i64,
    pub close: f64,
    pub prior_bias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeStructureSnapshot {
    pub pivots: Vec<NativeStructurePivot>,
    pub breaks: Vec<NativeStructureBreak>,
    pub bias: String,
}

impl From<ClassifiedSwing> for NativeStructurePivot {
    fn from(swing: ClassifiedSwing) -> Self {
        Self {
            kind: swing.kind.into(),
            classification: swing.classification.into(),
            pivot_index: swing.pivot_index,
            confirmed_at_index: swing.confirmed_at_index,
            pivot_at_ms: swing.pivot_at_ms,
            confirmed_at_ms: swing.confirmed_at_ms,
            price: swing.price,
        }
    }
}

impl From<StructureBreak> for NativeStructureBreak {
    fn from(event: StructureBreak) -> Self {
        Self {
            direction: event.direction.into(),
            classification: event.classification.into(),
            level_kind: event.level_kind.into(),
            level: event.level,
            pivot_index: event.pivot_index,
            level_confirmed_at_index: event.level_confirmed_at_index,
            broken_at_index: event.broken_at_index,
            broken_at_ms: event.broken_at_ms,
            close: event.close,
            prior_bias: event.prior_bias.into(),
        }
    }
}

fn format_value(value: f64, digits: usize) -> String {
    if !value.is_finite() {
        return "Unavailable".into();
    }
    let digits = if value.abs() < 1.0 { digits.max(6).min(8) } else { digits.min(4) };
    format!("{value:.digits$}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn sma(values: &[f64], period: usize) -> Option<f64> {
    if values.len() < period {
        return None;
    }
    let sample = &values[values.len() - period..];
    if sample.iter().any(|value| !value.is_finite()) {
        return None;
    }
    Some(sample.iter().sum::<f64>() / period as f64)
}

fn ema_series(values: &[f64], period: usize) -> Vec<Option<f64>> {
    if values.len() < period || values.iter().any(|value| !value.is_finite()) {
        return Vec::new();
    }
    let mut output = vec![None; values.len()];
    let mut current = values[..period].iter().sum::<f64>() / period as f64;
    output[period - 1] = Some(current);
    let alpha = 2.0 / (period as f64 + 1.0);
    for index in period..values.len() {
        current = values[index] * alpha + current * (1.0 - alpha);
        output[index] = Some(current);
    }
    output
}

fn rsi(values: &[f64], period: usize) -> Option<f64> {
    if values.len() <= period || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let (mut gains, mut losses) = (0.0, 0.0);
    for index in 1..=period {
        let delta = values[index] - values[index - 1];
        gains += delta.max(0.0);
        losses += (-delta).max(0.0);
    }
    let (mut average_gain, mut average_loss) = (gains / period as f64, losses / period as f64);
    for index in period + 1..values.len() {
        let delta = values[index] - values[index - 1];
        average_gain = (average_gain * (period - 1) as f64 + delta.max(0.0)) / period as f64;
        average_loss = (average_loss * (period - 1) as f64 + (-delta).max(0.0)) / period as f64;
    }
    if average_loss == 0.0 {
        return Some(if average_gain == 0.0 { 50.0 } else { 100.0 });
    }
    Some(100.0 - 100.0 / (1.0 + average_gain / average_loss))
}

fn atr(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() <= period {
        return None;
    }
    let mut ranges = Vec::with_capacity(candles.len());
    for (index, candle) in candles.iter().enumerate() {
        if index == 0 {
            ranges.push(candle.high - candle.low);
        } else {
            let previous_close = candles[index - 1].close;
            ranges.push(
                (candle.high - candle.low)
                    .max((candle.high - previous_close).abs())
                    .max((candle.low - previous_close).abs()),
            );
        }
    }
    if ranges.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut current = ranges[..period].iter().sum::<f64>() / period as f64;
    for value in ranges.iter().skip(period) {
        current = (current * (period - 1) as f64 + value) / period as f64;
    }
    Some(current)
}

fn bollinger(values: &[f64], period: usize) -> Option<(f64, f64, f64)> {
    if values.len() < period || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let sample = &values[values.len() - period..];
    let mean = sample.iter().sum::<f64>() / period as f64;
    let variance = sample.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / period as f64;
    let deviation = variance.sqrt();
    Some((mean + 2.0 * deviation, mean, mean - 2.0 * deviation))
}

fn unavailable_readings(count: usize, reason: &str) -> Vec<IndicatorReading> {
    [
        ("sma", "SMA · 9 / 21", 21),
        ("ema", "EMA · 20", 20),
        ("rsi", "RSI · 14", 15),
        ("macd", "MACD · 12 / 26 / 9", 34),
        ("atr", "ATR · 14", 15),
        ("bollinger", "Bollinger Bands · 20 / 2σ", 20),
    ]
    .into_iter()
    .map(|(key, title, needed)| IndicatorReading {
        key: key.into(),
        title: title.into(),
        value: "Unavailable".into(),
        detail: if reason.is_empty() {
            format!("Need at least {needed} valid candles; {count} loaded.")
        } else {
            reason.into()
        },
        available: false,
    })
    .collect()
}

pub fn calculate_readings(candles: &[Candle]) -> Result<Vec<IndicatorReading>, String> {
    if candles.len() > MAX_CANDLES {
        return Err(format!("Indicator input exceeds the {MAX_CANDLES}-candle safety limit."));
    }
    let mut previous_time = None;
    for (index, candle) in candles.iter().enumerate() {
        if candle.time <= 0 || previous_time.is_some_and(|previous| candle.time <= previous) {
            return Ok(unavailable_readings(candles.len(), &format!(
                "Candle {} timestamp must be positive and strictly increasing.", index + 1
            )));
        }
        previous_time = Some(candle.time);
        if ![candle.open, candle.high, candle.low, candle.close, candle.volume]
            .iter()
            .all(|value| value.is_finite())
            || candle.open <= 0.0
            || candle.high <= 0.0
            || candle.low <= 0.0
            || candle.close <= 0.0
            || candle.volume < 0.0
            || candle.high < candle.open.max(candle.close).max(candle.low)
            || candle.low > candle.open.min(candle.close).min(candle.high)
        {
            return Ok(unavailable_readings(candles.len(), &format!(
                "Candle {} violates positive-price, non-negative-volume, or OHLC range invariants.",
                index + 1
            )));
        }
    }

    let closes: Vec<f64> = candles.iter().map(|candle| candle.close).collect();
    let sma9 = sma(&closes, 9);
    let sma21 = sma(&closes, 21);
    let ema20 = ema_series(&closes, 20).last().copied().flatten();
    let rsi14 = rsi(&closes, 14);
    let atr14 = atr(candles, 14);
    let macd = if closes.len() >= 34 {
        let fast = ema_series(&closes, 12);
        let slow = ema_series(&closes, 26);
        let values: Vec<f64> = (25..closes.len())
            .filter_map(|index| Some(fast.get(index).copied().flatten()? - slow.get(index).copied().flatten()?))
            .collect();
        let signal = ema_series(&values, 9).last().copied().flatten();
        values.last().copied().zip(signal).map(|(line, signal)| (line, signal, line - signal))
    } else {
        None
    };
    let bands = bollinger(&closes, 20);
    let count = candles.len();
    let finite_scalars = [sma9, sma21, ema20, rsi14, atr14]
        .into_iter()
        .flatten()
        .all(f64::is_finite);
    let finite_macd = macd.is_none_or(|values| values.0.is_finite() && values.1.is_finite() && values.2.is_finite());
    let finite_bands = bands.is_none_or(|values| values.0.is_finite() && values.1.is_finite() && values.2.is_finite());
    if !finite_scalars || !finite_macd || !finite_bands {
        return Ok(unavailable_readings(count, "Indicator arithmetic exceeded the supported numeric range."));
    }

    Ok(vec![
        IndicatorReading {
            key: "sma".into(), title: "SMA · 9 / 21".into(),
            value: sma9.zip(sma21).map(|(fast, slow)| format!("{} / {}", format_value(fast, 4), format_value(slow, 4))).unwrap_or_else(|| "Unavailable".into()),
            detail: sma9.zip(sma21).map(|(fast, slow)| format!("Fast {} slow · close-price simple averages.", if fast >= slow { "above" } else { "below" })).unwrap_or_else(|| format!("Need at least 21 valid candles; {count} loaded.")),
            available: sma9.is_some() && sma21.is_some(),
        },
        IndicatorReading {
            key: "ema".into(), title: "EMA · 20".into(),
            value: ema20.map(|value| format_value(value, 4)).unwrap_or_else(|| "Unavailable".into()),
            detail: if ema20.is_some() { "Exponential average seeded with the first 20-bar mean.".into() } else { format!("Need at least 20 valid candles; {count} loaded.") },
            available: ema20.is_some(),
        },
        IndicatorReading {
            key: "rsi".into(), title: "RSI · 14".into(),
            value: rsi14.map(|value| format_value(value, 2)).unwrap_or_else(|| "Unavailable".into()),
            detail: rsi14.map(|value| if value >= 70.0 { "Above the conventional 70 threshold; not a standalone sell signal." } else if value <= 30.0 { "Below the conventional 30 threshold; not a standalone buy signal." } else { "Between conventional 30 / 70 reference levels." }.into()).unwrap_or_else(|| format!("Need at least 15 valid candles; {count} loaded.")),
            available: rsi14.is_some(),
        },
        IndicatorReading {
            key: "macd".into(), title: "MACD · 12 / 26 / 9".into(),
            value: macd.map(|(_, _, histogram)| format_value(histogram, 4)).unwrap_or_else(|| "Unavailable".into()),
            detail: macd.map(|(line, signal, _)| format!("Line {} · signal {} · histogram shown above.", format_value(line, 4), format_value(signal, 4))).unwrap_or_else(|| format!("Need at least 34 valid candles; {count} loaded.")),
            available: macd.is_some(),
        },
        IndicatorReading {
            key: "atr".into(), title: "ATR · 14".into(),
            value: atr14.map(|value| format_value(value, 4)).unwrap_or_else(|| "Unavailable".into()),
            detail: if atr14.is_some() { "Wilder-smoothed true range; measures volatility, not direction.".into() } else { format!("Need at least 15 valid candles; {count} loaded.") },
            available: atr14.is_some(),
        },
        IndicatorReading {
            key: "bollinger".into(), title: "Bollinger Bands · 20 / 2σ".into(),
            value: bands.map(|(upper, _, lower)| format!("{} – {}", format_value(lower, 4), format_value(upper, 4))).unwrap_or_else(|| "Unavailable".into()),
            detail: bands.map(|(upper, middle, lower)| { let _ = (upper, lower); format!("Middle {} · population standard deviation over 20 closes.", format_value(middle, 4)) }).unwrap_or_else(|| format!("Need at least 20 valid candles; {count} loaded.")),
            available: bands.is_some(),
        },
    ])
}

pub fn calculate_structure(candles: &[Candle]) -> Result<NativeStructureSnapshot, String> {
    if candles.len() > MAX_CANDLES {
        return Err(format!("Structure input exceeds the {MAX_CANDLES}-candle safety limit."));
    }
    let mut previous_time = None;
    for (index, candle) in candles.iter().enumerate() {
        if candle.time <= 0 || previous_time.is_some_and(|previous| candle.time <= previous) {
            return Err(format!("Structure candle {index} has a non-positive or non-increasing timestamp."));
        }
        previous_time = Some(candle.time);
        if ![candle.open, candle.high, candle.low, candle.close, candle.volume].iter().all(|value| value.is_finite())
            || candle.open <= 0.0 || candle.high <= 0.0 || candle.low <= 0.0 || candle.close <= 0.0
            || candle.volume < 0.0
            || candle.high < candle.open.max(candle.close).max(candle.low)
            || candle.low > candle.open.min(candle.close).min(candle.high)
        {
            return Err(format!("Structure candle {index} violates positive-price, non-negative-volume, or OHLC invariants."));
        }
    }
    let pivots = research_smc::classify_confirmed_swings(candles, 2, 2)?
        .into_iter()
        .map(NativeStructurePivot::from)
        .collect::<Vec<_>>();
    let breaks = research_smc::detect_confirmed_structure_breaks(candles, 2, 2)?
        .into_iter()
        .map(NativeStructureBreak::from)
        .collect::<Vec<_>>();
    let bias = breaks.last().map(|event| event.direction.clone()).unwrap_or_else(|| "unknown".into());
    Ok(NativeStructureSnapshot {
        pivots: pivots.into_iter().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect(),
        breaks: breaks.into_iter().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect(),
        bias,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candles(count: usize) -> Vec<Candle> {
        (0..count).map(|index| {
            let close = 100.0 + index as f64 * 0.5 + (index % 3) as f64 * 0.1;
            Candle { time: index as i64 + 1, open: close - 0.2, high: close + 0.4, low: close - 0.4, close, volume: 10.0 }
        }).collect()
    }

    #[test]
    fn computes_indicator_readings_from_valid_candles() {
        let result = calculate_readings(&candles(100)).unwrap();
        assert_eq!(result.len(), 6);
        assert!(result.iter().all(|reading| reading.available));
        assert_ne!(result[0].value, "Unavailable");
    }

    #[test]
    fn insufficient_history_is_reported_without_fabricated_values() {
        let result = calculate_readings(&candles(10)).unwrap();
        assert_eq!(result.len(), 6);
        assert!(result.iter().all(|reading| !reading.available));
    }

    #[test]
    fn malformed_ohlc_fails_closed_for_every_indicator() {
        let mut rows = candles(100);
        rows[10].high = rows[10].low - 1.0;
        let result = calculate_readings(&rows).unwrap();
        assert!(result.iter().all(|reading| !reading.available));
        assert!(result[0].detail.contains("Candle 11"));
    }

    #[test]
    fn out_of_order_timestamps_fail_closed() {
        let mut rows = candles(40);
        rows[20].time = rows[19].time;
        let result = calculate_readings(&rows).unwrap();
        assert!(result.iter().all(|reading| !reading.available));
        assert!(result[0].detail.contains("strictly increasing"));
    }

    #[test]
    fn arithmetic_overflow_does_not_emit_available_indicators() {
        let rows: Vec<Candle> = (0..40).map(|index| Candle {
            time: index + 1,
            open: f64::MAX * 0.9,
            high: f64::MAX * 0.9,
            low: f64::MAX * 0.9,
            close: f64::MAX * 0.9,
            volume: 1.0,
        }).collect();
        let result = calculate_readings(&rows).unwrap();
        assert!(result.iter().all(|reading| !reading.available));
        assert!(result[0].detail.contains("numeric range"));
    }

    #[test]
    fn excessive_input_is_rejected() {
        assert!(calculate_readings(&candles(MAX_CANDLES + 1)).is_err());
    }

    #[test]
    fn native_structure_rejects_malformed_input() {
        let mut rows = candles(40);
        rows[12].low = rows[12].high + 1.0;
        assert!(calculate_structure(&rows).unwrap_err().contains("OHLC invariants"));
    }

    #[test]
    fn native_structure_is_causal_and_bounded() {
        let rows = candles(100);
        let result = calculate_structure(&rows).unwrap();
        assert!(result.pivots.len() <= 40);
        assert!(result.breaks.len() <= 40);
    }
}
