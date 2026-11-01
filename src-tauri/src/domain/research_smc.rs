//! Deterministic, research-only SMC primitives.
//!
//! These functions intentionally do not promote catalog entries or authorize a decision.
//! They emit observations with explicit confirmation indices and never inspect future bars
//! when assigning an event to a candle. Independent review and broader market fixtures are
//! required before any output can be registered as a reviewed implementation.

use crate::domain::models::Candle;
use std::collections::{BTreeMap, VecDeque};
use std::ops::Bound::{Excluded, Included};

/// A detector must fail closed rather than return a truncated research result.
const MAX_RESEARCH_EVENTS: usize = 1_000_000;

/// Convert a finite IEEE-754 price to a monotonically ordered integer key.
/// Normalize signed zero because market-price comparisons treat -0.0 and +0.0 as equal.
fn ordered_price_key(price: f64) -> u64 {
    let normalized = if price == 0.0 { 0.0 } else { price };
    let bits = normalized.to_bits();
    if bits & (1_u64 << 63) != 0 {
        !bits
    } else {
        bits ^ (1_u64 << 63)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmedSwing {
    /// `swing_high` or `swing_low`; equal/multiple-tie pivots are intentionally excluded.
    pub kind: &'static str,
    pub pivot_index: usize,
    /// Earliest index at which the pivot is knowable without looking ahead.
    pub confirmed_at_index: usize,
    pub pivot_at_ms: i64,
    pub confirmed_at_ms: i64,
    pub price: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassifiedSwing {
    /// Source pivot type: `swing_high` or `swing_low`.
    pub kind: &'static str,
    /// `initial_high`, `higher_high`, `lower_high`, `equal_high`,
    /// `initial_low`, `higher_low`, `lower_low`, or `equal_low`.
    pub classification: &'static str,
    pub pivot_index: usize,
    pub confirmed_at_index: usize,
    pub pivot_at_ms: i64,
    pub confirmed_at_ms: i64,
    pub price: f64,
    /// Previous same-kind pivot used for this comparison; absent for the first pivot.
    pub compared_to_pivot_index: Option<usize>,
    pub compared_to_price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FairValueGap {
    pub direction: &'static str,
    pub formed_at_index: usize,
    pub formed_at_ms: i64,
    pub lower: f64,
    pub upper: f64,
    pub midpoint: f64,
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiquiditySweep {
    pub side: &'static str,
    pub level: f64,
    pub pivot_index: usize,
    pub pivot_confirmed_at_index: usize,
    pub swept_at_index: usize,
    pub swept_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FvgLifecycleEvent {
    pub direction: &'static str,
    /// `partial_fill`, `full_fill`, or `inversion_candidate`; never a trade signal.
    pub event_type: &'static str,
    pub formed_at_index: usize,
    pub observed_at_index: usize,
    pub observed_at_ms: i64,
    pub lower: f64,
    pub upper: f64,
}

fn validate_candles(candles: &[Candle]) -> Result<(), String> {
    if candles.len() < 3 {
        return Err("At least three candles are required for SMC research detectors.".into());
    }
    if candles.len() > 2_000_000 {
        return Err("Candle input exceeds the SMC detector safety limit.".into());
    }
    for (index, candle) in candles.iter().enumerate() {
        if !candle.open.is_finite()
            || !candle.high.is_finite()
            || !candle.low.is_finite()
            || !candle.close.is_finite()
            || !candle.volume.is_finite()
            || candle.volume < 0.0
            || candle.high < candle.low
            || candle.high < candle.open
            || candle.high < candle.close
            || candle.low > candle.open
            || candle.low > candle.close
        {
            return Err(format!("Malformed OHLCV candle at index {index}."));
        }
        if index > 0 && candle.time <= candles[index - 1].time {
            return Err(format!(
                "Candle timestamps must be strictly increasing (index {index})."
            ));
        }
    }
    Ok(())
}

/// Keep indices in a monotonic deque ordered by descending candle high.
/// The caller expires stale indices from the front as its window advances.
fn push_max_index(candles: &[Candle], deque: &mut VecDeque<usize>, index: usize) {
    while let Some(&back) = deque.back() {
        if candles[back].high <= candles[index].high {
            deque.pop_back();
        } else {
            break;
        }
    }
    deque.push_back(index);
}

/// Keep indices in a monotonic deque ordered by ascending candle low.
fn push_min_index(candles: &[Candle], deque: &mut VecDeque<usize>, index: usize) {
    while let Some(&back) = deque.back() {
        if candles[back].low >= candles[index].low {
            deque.pop_back();
        } else {
            break;
        }
    }
    deque.push_back(index);
}

/// Detect unique, confirmed fractal swing points in O(n) time. A pivot at `p` is emitted
/// only after `right_bars` complete candles have closed. Strict inequalities intentionally
/// reject tied highs/lows; equal-price plateaus do not select an arbitrary pivot. This is a
/// geometric primitive only, not a complete BOS/CHoCH/MSS interpretation.
pub fn detect_confirmed_swings(
    candles: &[Candle],
    left_bars: usize,
    right_bars: usize,
) -> Result<Vec<ConfirmedSwing>, String> {
    validate_candles(candles)?;
    if left_bars == 0 || right_bars == 0 {
        return Err("Swing confirmation windows must both be greater than zero.".into());
    }
    if left_bars > 10_000 || right_bars > 10_000 {
        return Err("Swing confirmation window exceeds the safety limit.".into());
    }
    if candles.len() <= left_bars.saturating_add(right_bars) {
        return Ok(Vec::new());
    }

    let first_pivot = left_bars;
    let pivot_end = candles.len() - right_bars; // exclusive
    let mut left_max = VecDeque::<usize>::new();
    let mut left_min = VecDeque::<usize>::new();
    let mut right_max = VecDeque::<usize>::new();
    let mut right_min = VecDeque::<usize>::new();

    for index in 0..first_pivot {
        push_max_index(candles, &mut left_max, index);
        push_min_index(candles, &mut left_min, index);
    }
    // For the first pivot, the right window is [pivot + 1, pivot + right_bars].
    for index in (first_pivot + 1)..=(first_pivot + right_bars) {
        push_max_index(candles, &mut right_max, index);
        push_min_index(candles, &mut right_min, index);
    }

    let mut swings = Vec::new();
    for pivot in first_pivot..pivot_end {
        let candle = &candles[pivot];
        let is_high = left_max
            .front()
            .is_some_and(|&entry| candle.high > candles[entry].high)
            && right_max
                .front()
                .is_some_and(|&entry| candle.high > candles[entry].high);
        let is_low = left_min
            .front()
            .is_some_and(|&entry| candle.low < candles[entry].low)
            && right_min
                .front()
                .is_some_and(|&entry| candle.low < candles[entry].low);
        let confirmed_at = pivot + right_bars;

        if is_high {
            if swings.len() >= MAX_RESEARCH_EVENTS {
                return Err("Swing output exceeds the research event safety limit; no partial result returned.".into());
            }
            swings.push(ConfirmedSwing {
                kind: "swing_high",
                pivot_index: pivot,
                confirmed_at_index: confirmed_at,
                pivot_at_ms: candle.time,
                confirmed_at_ms: candles[confirmed_at].time,
                price: candle.high,
            });
        }
        if is_low {
            if swings.len() >= MAX_RESEARCH_EVENTS {
                return Err("Swing output exceeds the research event safety limit; no partial result returned.".into());
            }
            swings.push(ConfirmedSwing {
                kind: "swing_low",
                pivot_index: pivot,
                confirmed_at_index: confirmed_at,
                pivot_at_ms: candle.time,
                confirmed_at_ms: candles[confirmed_at].time,
                price: candle.low,
            });
        }

        if pivot + 1 < pivot_end {
            let next_pivot = pivot + 1;
            let left_min_valid = next_pivot - left_bars;
            while left_max
                .front()
                .is_some_and(|&entry| entry < left_min_valid)
            {
                left_max.pop_front();
            }
            while left_min
                .front()
                .is_some_and(|&entry| entry < left_min_valid)
            {
                left_min.pop_front();
            }
            push_max_index(candles, &mut left_max, pivot);
            push_min_index(candles, &mut left_min, pivot);

            // Advance right window from [pivot + 1, pivot + right] to
            // [pivot + 2, pivot + right + 1]. Indices remain increasing in each deque.
            let right_min_valid = next_pivot + 1;
            while right_max
                .front()
                .is_some_and(|&entry| entry < right_min_valid)
            {
                right_max.pop_front();
            }
            while right_min
                .front()
                .is_some_and(|&entry| entry < right_min_valid)
            {
                right_min.pop_front();
            }
            let entering = pivot + right_bars + 1;
            push_max_index(candles, &mut right_max, entering);
            push_min_index(candles, &mut right_min, entering);
        }
    }

    swings.sort_by_key(|swing| (swing.confirmed_at_index, swing.pivot_index, swing.kind));
    Ok(swings)
}

/// Classify confirmed swing points as HH/HL/LH/LL relative to the prior pivot of the
/// same kind. Classification becomes available at the swing's confirmation candle, not at
/// its historical pivot candle. Equal-price comparisons are explicit and are not coerced
/// into HH/LH or HL/LL. This is a research primitive, not a trend/bias/BOS signal.
pub fn classify_confirmed_swings(
    candles: &[Candle],
    left_bars: usize,
    right_bars: usize,
) -> Result<Vec<ClassifiedSwing>, String> {
    let swings = detect_confirmed_swings(candles, left_bars, right_bars)?;
    let mut ordered = swings;
    ordered.sort_by_key(|swing| (swing.pivot_index, swing.kind));

    let mut previous_high: Option<ConfirmedSwing> = None;
    let mut previous_low: Option<ConfirmedSwing> = None;
    let mut classified = Vec::with_capacity(ordered.len());
    for swing in ordered {
        let (classification, previous) = if swing.kind == "swing_high" {
            let previous = previous_high.as_ref();
            let label = match previous {
                None => "initial_high",
                Some(prev) if swing.price > prev.price => "higher_high",
                Some(prev) if swing.price < prev.price => "lower_high",
                Some(_) => "equal_high",
            };
            (label, previous)
        } else {
            let previous = previous_low.as_ref();
            let label = match previous {
                None => "initial_low",
                Some(prev) if swing.price > prev.price => "higher_low",
                Some(prev) if swing.price < prev.price => "lower_low",
                Some(_) => "equal_low",
            };
            (label, previous)
        };
        classified.push(ClassifiedSwing {
            kind: swing.kind,
            classification,
            pivot_index: swing.pivot_index,
            confirmed_at_index: swing.confirmed_at_index,
            pivot_at_ms: swing.pivot_at_ms,
            confirmed_at_ms: swing.confirmed_at_ms,
            price: swing.price,
            compared_to_pivot_index: previous.map(|prev| prev.pivot_index),
            compared_to_price: previous.map(|prev| prev.price),
        });
        if swing.kind == "swing_high" {
            previous_high = Some(swing);
        } else {
            previous_low = Some(swing);
        }
    }
    classified.sort_by_key(|swing| (swing.confirmed_at_index, swing.pivot_index, swing.kind));
    Ok(classified)
}

/// Detect classic three-candle FVG geometry. A gap is knowable only when the third candle
/// closes; this function reports the third candle index and does not infer later fill state.
pub fn detect_fair_value_gaps(candles: &[Candle]) -> Result<Vec<FairValueGap>, String> {
    validate_candles(candles)?;
    let mut gaps = Vec::new();
    for index in 2..candles.len() {
        let left = &candles[index - 2];
        let right = &candles[index];
        if right.low > left.high {
            let lower = left.high;
            let upper = right.low;
            let width = upper - lower;
            // Finite source prices do not guarantee finite derived geometry: subtraction
            // can overflow for extreme but finite f64 values. Never emit an invalid zone.
            if !width.is_finite() || width <= 0.0 {
                return Err(format!(
                    "FVG width is not representable at candle index {index}."
                ));
            }
            let midpoint = lower / 2.0 + upper / 2.0;
            if !midpoint.is_finite() {
                return Err(format!(
                    "FVG midpoint is not representable at candle index {index}."
                ));
            }
            if gaps.len() >= MAX_RESEARCH_EVENTS {
                return Err("FVG output exceeds the research event safety limit; no partial result returned.".into());
            }
            gaps.push(FairValueGap {
                direction: "bullish",
                formed_at_index: index,
                formed_at_ms: right.time,
                lower,
                upper,
                midpoint,
                width,
            });
        } else if right.high < left.low {
            let lower = right.high;
            let upper = left.low;
            let width = upper - lower;
            if !width.is_finite() || width <= 0.0 {
                return Err(format!(
                    "FVG width is not representable at candle index {index}."
                ));
            }
            let midpoint = lower / 2.0 + upper / 2.0;
            if !midpoint.is_finite() {
                return Err(format!(
                    "FVG midpoint is not representable at candle index {index}."
                ));
            }
            if gaps.len() >= MAX_RESEARCH_EVENTS {
                return Err("FVG output exceeds the research event safety limit; no partial result returned.".into());
            }
            gaps.push(FairValueGap {
                direction: "bearish",
                formed_at_index: index,
                formed_at_ms: right.time,
                lower,
                upper,
                midpoint,
                width,
            });
        }
    }
    Ok(gaps)
}

/// Emit FVG lifecycle observations in chronological order. This function evaluates each
/// post-formation candle independently and never labels a gap from data before formation.
/// `inversion_candidate` means a candle closes beyond the far boundary; it is not proof of
/// a strategy-valid IFVG or a reclaimed setup.
pub fn assess_fvg_lifecycle(
    candles: &[Candle],
    gaps: &[FairValueGap],
) -> Result<Vec<FvgLifecycleEvent>, String> {
    validate_candles(candles)?;
    for gap in gaps {
        if gap.formed_at_index < 2
            || gap.formed_at_index >= candles.len()
            || !gap.lower.is_finite()
            || !gap.upper.is_finite()
            || gap.lower >= gap.upper
            || !gap.midpoint.is_finite()
            || !gap.width.is_finite()
            || gap.width <= 0.0
            || !matches!(gap.direction, "bullish" | "bearish")
            || gap.formed_at_ms != candles[gap.formed_at_index].time
        {
            return Err("FVG input contains an invalid zone or formation reference.".into());
        }
        let left = &candles[gap.formed_at_index - 2];
        let right = &candles[gap.formed_at_index];
        let expected = if right.low > left.high {
            Some(("bullish", left.high, right.low))
        } else if right.high < left.low {
            Some(("bearish", right.high, left.low))
        } else {
            None
        };
        let Some((direction, lower, upper)) = expected else {
            return Err(
                "FVG reference does not match a three-candle gap in the supplied candles.".into(),
            );
        };
        let midpoint = lower / 2.0 + upper / 2.0;
        let width = upper - lower;
        if !width.is_finite() || width <= 0.0 || !midpoint.is_finite() {
            return Err("FVG reference geometry is not representable in finite precision.".into());
        }
        let tolerance = f64::EPSILON * lower.abs().max(upper.abs()).max(1.0) * 8.0;
        if gap.direction != direction
            || (gap.lower - lower).abs() > tolerance
            || (gap.upper - upper).abs() > tolerance
            || (gap.midpoint - midpoint).abs() > tolerance
            || (gap.width - width).abs() > tolerance
        {
            return Err("FVG zone geometry does not match the source candles.".into());
        }
    }

    let mut events = Vec::new();
    for gap in gaps {
        let mut partial_emitted = false;
        let mut full_emitted = false;
        let mut inversion_emitted = false;
        for (index, candle) in candles.iter().enumerate().skip(gap.formed_at_index + 1) {
            let (touches_zone, fully_filled, closes_beyond) = if gap.direction == "bullish" {
                (
                    candle.low <= gap.upper && candle.high >= gap.lower,
                    candle.low <= gap.lower,
                    candle.close < gap.lower,
                )
            } else {
                (
                    candle.high >= gap.lower && candle.low <= gap.upper,
                    candle.high >= gap.upper,
                    candle.close > gap.upper,
                )
            };
            if fully_filled && !full_emitted {
                if events.len() >= MAX_RESEARCH_EVENTS {
                    return Err("FVG lifecycle output exceeds the research event safety limit; no partial result returned.".into());
                }
                events.push(FvgLifecycleEvent {
                    direction: gap.direction,
                    event_type: "full_fill",
                    formed_at_index: gap.formed_at_index,
                    observed_at_index: index,
                    observed_at_ms: candle.time,
                    lower: gap.lower,
                    upper: gap.upper,
                });
                full_emitted = true;
                partial_emitted = true;
            } else if touches_zone && !partial_emitted {
                if events.len() >= MAX_RESEARCH_EVENTS {
                    return Err("FVG lifecycle output exceeds the research event safety limit; no partial result returned.".into());
                }
                events.push(FvgLifecycleEvent {
                    direction: gap.direction,
                    event_type: "partial_fill",
                    formed_at_index: gap.formed_at_index,
                    observed_at_index: index,
                    observed_at_ms: candle.time,
                    lower: gap.lower,
                    upper: gap.upper,
                });
                partial_emitted = true;
            }
            if closes_beyond && !inversion_emitted {
                if events.len() >= MAX_RESEARCH_EVENTS {
                    return Err("FVG lifecycle output exceeds the research event safety limit; no partial result returned.".into());
                }
                events.push(FvgLifecycleEvent {
                    direction: gap.direction,
                    event_type: "inversion_candidate",
                    formed_at_index: gap.formed_at_index,
                    observed_at_index: index,
                    observed_at_ms: candle.time,
                    lower: gap.lower,
                    upper: gap.upper,
                });
                inversion_emitted = true;
            }
        }
    }
    events.sort_by_key(|event| {
        (
            event.observed_at_index,
            event.formed_at_index,
            event.event_type,
        )
    });
    Ok(events)
}

/// Detect wick-through/close-back-inside sweeps of unique confirmed pivots.
/// `left_bars` and `right_bars` must be positive. A pivot is not eligible until its
/// confirmation index (`pivot + right_bars`) and only later candles can sweep it.
pub fn detect_confirmed_liquidity_sweeps(
    candles: &[Candle],
    left_bars: usize,
    right_bars: usize,
) -> Result<Vec<LiquiditySweep>, String> {
    validate_candles(candles)?;
    // Reuse the single source of truth for strict pivot uniqueness and its window limits.
    let swings = detect_confirmed_swings(candles, left_bars, right_bars)?;
    let mut active_buy_side = BTreeMap::<u64, Vec<usize>>::new();
    let mut active_sell_side = BTreeMap::<u64, Vec<usize>>::new();
    let mut next_swing = 0usize;
    let mut sweeps = Vec::new();

    for (index, candle) in candles.iter().enumerate() {
        // A pivot is eligible only strictly after its confirmation candle.
        while next_swing < swings.len() && swings[next_swing].confirmed_at_index < index {
            let swing_index = next_swing;
            let swing = &swings[swing_index];
            let levels = if swing.kind == "swing_high" {
                &mut active_buy_side
            } else {
                &mut active_sell_side
            };
            levels
                .entry(ordered_price_key(swing.price))
                .or_default()
                .push(swing_index);
            next_swing += 1;
        }

        // Range lookup visits only levels actually crossed by this candle, instead of
        // comparing every candle with every historical pivot. Repeated valid sweeps keep
        // their original event semantics; total work is O(n log p + emitted events).
        let buy_keys: Vec<u64> = if candle.high > candle.close {
            active_buy_side
                .range((
                    Excluded(ordered_price_key(candle.close)),
                    Excluded(ordered_price_key(candle.high)),
                ))
                .map(|(&key, _)| key)
                .collect()
        } else {
            Vec::new()
        };
        for key in buy_keys {
            if let Some(pivots) = active_buy_side.get(&key) {
                for &pivot_index in pivots {
                    let swing = &swings[pivot_index];
                    if sweeps.len() >= MAX_RESEARCH_EVENTS {
                        return Err("Liquidity sweep output exceeds the research event safety limit; no partial result returned.".into());
                    }
                    sweeps.push(LiquiditySweep {
                        side: "buy_side",
                        level: swing.price,
                        pivot_index: swing.pivot_index,
                        pivot_confirmed_at_index: swing.confirmed_at_index,
                        swept_at_index: index,
                        swept_at_ms: candle.time,
                    });
                }
            }
        }

        let sell_keys: Vec<u64> = if candle.low < candle.close {
            active_sell_side
                .range((
                    Excluded(ordered_price_key(candle.low)),
                    Excluded(ordered_price_key(candle.close)),
                ))
                .map(|(&key, _)| key)
                .collect()
        } else {
            Vec::new()
        };
        for key in sell_keys {
            if let Some(pivots) = active_sell_side.get(&key) {
                for &pivot_index in pivots {
                    let swing = &swings[pivot_index];
                    if sweeps.len() >= MAX_RESEARCH_EVENTS {
                        return Err("Liquidity sweep output exceeds the research event safety limit; no partial result returned.".into());
                    }
                    sweeps.push(LiquiditySweep {
                        side: "sell_side",
                        level: swing.price,
                        pivot_index: swing.pivot_index,
                        pivot_confirmed_at_index: swing.confirmed_at_index,
                        swept_at_index: index,
                        swept_at_ms: candle.time,
                    });
                }
            }
        }
    }
    sweeps.sort_by_key(|sweep| (sweep.swept_at_index, sweep.pivot_index, sweep.side));
    Ok(sweeps)
}

/// A close-confirmed break of a previously confirmed swing level. Research-only: the
/// `classification` values are deliberately conservative and are not universal ICT labels.
#[derive(Debug, Clone, PartialEq)]
pub struct StructureBreak {
    pub direction: &'static str,
    /// `initial_break`, `continuation_break`, `reversal_candidate`, or `ambiguous_break`.
    pub classification: &'static str,
    pub level_kind: &'static str,
    pub level: f64,
    pub pivot_index: usize,
    pub level_confirmed_at_index: usize,
    pub broken_at_index: usize,
    pub broken_at_ms: i64,
    pub close: f64,
    pub prior_bias: &'static str,
}

/// Detect close-confirmed breaks of confirmed swing levels without look-ahead.
/// A level cannot be broken on its own confirmation candle; only the first crossing
/// close for each pivot is emitted. Wick-only breaches are intentionally ignored.
/// Reversal labels are candidates only; BOS/CHoCH/MSS/MSB require additional strategy
/// definitions and must not be inferred from this primitive alone.
pub fn detect_confirmed_structure_breaks(
    candles: &[Candle],
    left_bars: usize,
    right_bars: usize,
) -> Result<Vec<StructureBreak>, String> {
    validate_candles(candles)?;
    let swings = detect_confirmed_swings(candles, left_bars, right_bars)?;
    let mut active_highs = BTreeMap::<u64, Vec<usize>>::new();
    let mut active_lows = BTreeMap::<u64, Vec<usize>>::new();
    let mut next_swing = 0usize;
    let mut events = Vec::new();
    let mut bias: i8 = 0; // 0 unknown, 1 bullish, -1 bearish

    for index in 1..candles.len() {
        // Load only levels confirmed before this candle opened. Both maps are ordered by
        // price so a close crossing queries only the interval that can actually break.
        while next_swing < swings.len() && swings[next_swing].confirmed_at_index < index {
            let swing_index = next_swing;
            let swing = &swings[swing_index];
            let levels = if swing.kind == "swing_high" {
                &mut active_highs
            } else {
                &mut active_lows
            };
            levels
                .entry(ordered_price_key(swing.price))
                .or_default()
                .push(swing_index);
            next_swing += 1;
        }

        let candle = &candles[index];
        let previous_close = candles[index - 1].close;
        let bullish_keys: Vec<u64> = if candle.close > previous_close {
            active_highs
                .range((
                    Included(ordered_price_key(previous_close)),
                    Excluded(ordered_price_key(candle.close)),
                ))
                .map(|(&key, _)| key)
                .collect()
        } else {
            Vec::new()
        };
        let bearish_keys: Vec<u64> = if candle.close < previous_close {
            active_lows
                .range((
                    Excluded(ordered_price_key(candle.close)),
                    Included(ordered_price_key(previous_close)),
                ))
                .map(|(&key, _)| key)
                .collect()
        } else {
            Vec::new()
        };

        let mut eligible = Vec::<usize>::new();
        for key in &bullish_keys {
            if let Some(pivots) = active_highs.get(key) {
                eligible.extend(pivots.iter().copied());
            }
        }
        for key in &bearish_keys {
            if let Some(pivots) = active_lows.get(key) {
                eligible.extend(pivots.iter().copied());
            }
        }
        for key in bullish_keys {
            active_highs.remove(&key);
        }
        for key in bearish_keys {
            active_lows.remove(&key);
        }
        // Keep deterministic batching: all events on this candle use the same pre-candle
        // bias snapshot, regardless of BTreeMap order or the number of crossed levels.
        eligible.sort_by_key(|&swing_index| {
            (
                swings[swing_index].confirmed_at_index,
                swings[swing_index].pivot_index,
                swings[swing_index].kind,
            )
        });
        let has_bullish = eligible.iter().any(|&s| swings[s].kind == "swing_high");
        let has_bearish = eligible.iter().any(|&s| swings[s].kind == "swing_low");
        let ambiguous = has_bullish && has_bearish;
        let prior_bias: &'static str = match bias {
            1 => "bullish",
            -1 => "bearish",
            _ => "unknown",
        };
        for swing_index in eligible {
            let swing = &swings[swing_index];
            let direction: &'static str = if swing.kind == "swing_high" {
                "bullish"
            } else {
                "bearish"
            };
            let direction_sign: i8 = if direction == "bullish" { 1 } else { -1 };
            let classification: &'static str = if ambiguous {
                "ambiguous_break"
            } else if bias == 0 {
                "initial_break"
            } else if bias == direction_sign {
                "continuation_break"
            } else {
                "reversal_candidate"
            };
            if events.len() >= MAX_RESEARCH_EVENTS {
                return Err("Structure-break output exceeds the research event safety limit; no partial result returned.".into());
            }
            events.push(StructureBreak {
                direction,
                classification,
                level_kind: swing.kind,
                level: swing.price,
                pivot_index: swing.pivot_index,
                level_confirmed_at_index: swing.confirmed_at_index,
                broken_at_index: index,
                broken_at_ms: candle.time,
                close: candle.close,
                prior_bias,
            });
        }
        // Ambiguous batches consume levels but freeze directional bias.
        if !ambiguous {
            if has_bullish {
                bias = 1;
            } else if has_bearish {
                bias = -1;
            }
        }
    }
    Ok(events)
}

/// Point-in-time market-structure snapshot. This is a research observation, not a trade signal.
#[derive(Debug, Clone, PartialEq)]
pub struct StructureStateSnapshot {
    pub at_index: usize,
    pub at_ms: i64,
    /// `unknown`, `bullish`, or `bearish` based only on accepted close-confirmed breaks.
    pub bias: &'static str,
    /// `none`, `initial_break`, `continuation_break`, `reversal_candidate`, or `ambiguous_break`.
    pub transition: &'static str,
    pub active_swing_high: Option<f64>,
    pub active_swing_low: Option<f64>,
    /// The latest confirmed opposing swing when a directional break establishes a bias.
    pub protected_high: Option<f64>,
    pub protected_low: Option<f64>,
    /// True when one candle closes through eligible levels in both directions; bias is frozen.
    pub ambiguous: bool,
}

/// Build a causal structure-state timeline from confirmed swings and close-confirmed breaks.
/// If a single candle crosses both bullish and bearish levels, the state machine marks that
/// candle ambiguous and freezes bias/protected levels rather than choosing an arbitrary order.
/// Protected levels are operational definitions for this detector, not universal ICT semantics.
pub fn build_structure_state_timeline(
    candles: &[Candle],
    left_bars: usize,
    right_bars: usize,
) -> Result<Vec<StructureStateSnapshot>, String> {
    validate_candles(candles)?;
    let swings = detect_confirmed_swings(candles, left_bars, right_bars)?;
    let breaks = detect_confirmed_structure_breaks(candles, left_bars, right_bars)?;

    // Index events once. The former implementation rescanned every swing and every break
    // for every candle, making timeline construction quadratic even when few events existed.
    let mut swings_by_confirmation: Vec<Vec<&ConfirmedSwing>> =
        (0..candles.len()).map(|_| Vec::new()).collect();
    for swing in &swings {
        swings_by_confirmation[swing.confirmed_at_index].push(swing);
    }
    let mut breaks_by_candle: Vec<Vec<&StructureBreak>> =
        (0..candles.len()).map(|_| Vec::new()).collect();
    for event in &breaks {
        breaks_by_candle[event.broken_at_index].push(event);
    }

    let mut bias: i8 = 0;
    let mut active_high: Option<f64> = None;
    let mut active_low: Option<f64> = None;
    let mut protected_high: Option<f64> = None;
    let mut protected_low: Option<f64> = None;
    let mut latest_confirmed_high: Option<&ConfirmedSwing> = None;
    let mut latest_confirmed_low: Option<&ConfirmedSwing> = None;
    let mut snapshots = Vec::with_capacity(candles.len());

    for index in 0..candles.len() {
        let at_index = &breaks_by_candle[index];
        let has_bullish = at_index.iter().any(|event| event.direction == "bullish");
        let has_bearish = at_index.iter().any(|event| event.direction == "bearish");
        let mut transition = "none";
        let mut ambiguous = false;

        if has_bullish && has_bearish {
            transition = "ambiguous_break";
            ambiguous = true;
        } else if has_bullish || has_bearish {
            let next_bias = if has_bullish { 1 } else { -1 };
            transition = if bias == 0 {
                "initial_break"
            } else if bias == next_bias {
                "continuation_break"
            } else {
                "reversal_candidate"
            };
            // These pointers only include swings confirmed strictly before this candle.
            if next_bias == 1 {
                if let Some(swing) = latest_confirmed_low {
                    protected_low = Some(swing.price);
                }
            } else if let Some(swing) = latest_confirmed_high {
                protected_high = Some(swing.price);
            }
            bias = next_bias;
        }

        // A pivot becomes active in this snapshot exactly at its confirmation index.
        for swing in &swings_by_confirmation[index] {
            if swing.kind == "swing_high" {
                active_high = Some(swing.price);
                latest_confirmed_high = Some(swing);
            } else {
                active_low = Some(swing.price);
                latest_confirmed_low = Some(swing);
            }
        }

        snapshots.push(StructureStateSnapshot {
            at_index: index,
            at_ms: candles[index].time,
            bias: match bias {
                1 => "bullish",
                -1 => "bearish",
                _ => "unknown",
            },
            transition,
            active_swing_high: active_high,
            active_swing_low: active_low,
            protected_high,
            protected_low,
            ambiguous,
        });
    }
    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(time: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            time,
            open,
            high,
            low,
            close,
            volume: 10.0,
        }
    }

    fn naive_confirmed_swings(
        candles: &[Candle],
        left_bars: usize,
        right_bars: usize,
    ) -> Vec<ConfirmedSwing> {
        let mut swings = Vec::new();
        for pivot in left_bars..candles.len().saturating_sub(right_bars) {
            let candle = &candles[pivot];
            let left = &candles[pivot - left_bars..pivot];
            let right = &candles[pivot + 1..=pivot + right_bars];
            let is_high = left.iter().all(|other| candle.high > other.high)
                && right.iter().all(|other| candle.high > other.high);
            let is_low = left.iter().all(|other| candle.low < other.low)
                && right.iter().all(|other| candle.low < other.low);
            let confirmed_at = pivot + right_bars;
            if is_high {
                swings.push(ConfirmedSwing {
                    kind: "swing_high",
                    pivot_index: pivot,
                    confirmed_at_index: confirmed_at,
                    pivot_at_ms: candle.time,
                    confirmed_at_ms: candles[confirmed_at].time,
                    price: candle.high,
                });
            }
            if is_low {
                swings.push(ConfirmedSwing {
                    kind: "swing_low",
                    pivot_index: pivot,
                    confirmed_at_index: confirmed_at,
                    pivot_at_ms: candle.time,
                    confirmed_at_ms: candles[confirmed_at].time,
                    price: candle.low,
                });
            }
        }
        swings.sort_by_key(|swing| (swing.confirmed_at_index, swing.pivot_index, swing.kind));
        swings
    }

    #[test]
    fn linear_time_swing_detector_matches_naive_reference_on_generated_candles() {
        let mut seed = 0x4d59_5df4_d0f3_3173_u64;
        let mut candles = Vec::new();
        for index in 0..400_i64 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let low = ((seed >> 32) % 150) as f64 - 50.0;
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let high = low + ((seed >> 32) % 11) as f64 + 1.0;
            let span = high - low;
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let open = low + ((seed >> 32) % 101) as f64 / 100.0 * span;
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let close = low + ((seed >> 32) % 101) as f64 / 100.0 * span;
            candles.push(c(index + 1, open, high, low, close));
        }
        for left in 1..=7 {
            for right in 1..=7 {
                assert_eq!(
                    detect_confirmed_swings(&candles, left, right).unwrap(),
                    naive_confirmed_swings(&candles, left, right),
                    "optimized output differs for left={left}, right={right}"
                );
            }
        }
    }

    #[test]
    fn research_detector_outputs_are_prefix_invariant_and_never_rewrite_past_events() {
        // Deterministic generated OHLCV data: every prefix must agree with the full-series
        // run for events that were already observable at that prefix. This catches accidental
        // look-ahead and future-data-dependent reclassification in the research primitives.
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut candles = Vec::new();
        for index in 0..96_i64 {
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let center = 100.0 + ((seed >> 32) % 61) as f64 - 30.0;
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let high = center + ((seed >> 32) % 9) as f64 + 0.5;
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let low = center - ((seed >> 32) % 9) as f64 - 0.5;
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let open = low + ((seed >> 32) % 10_001) as f64 / 10_001.0 * (high - low);
            seed = seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let close = low + ((seed >> 32) % 10_001) as f64 / 10_001.0 * (high - low);
            candles.push(Candle {
                time: index + 1,
                open,
                high,
                low,
                close,
                volume: (seed % 10_000) as f64,
            });
        }

        let full_swings = detect_confirmed_swings(&candles, 2, 2).unwrap();
        let full_gaps = detect_fair_value_gaps(&candles).unwrap();
        let full_lifecycle = assess_fvg_lifecycle(&candles, &full_gaps).unwrap();
        let full_sweeps = detect_confirmed_liquidity_sweeps(&candles, 2, 2).unwrap();
        let full_breaks = detect_confirmed_structure_breaks(&candles, 2, 2).unwrap();

        for prefix_len in 3..=candles.len() {
            let prefix = &candles[..prefix_len];
            let expected_swings: Vec<_> = full_swings
                .iter()
                .filter(|event| event.confirmed_at_index < prefix_len)
                .cloned()
                .collect();
            assert_eq!(
                detect_confirmed_swings(prefix, 2, 2).unwrap(),
                expected_swings,
                "swing output changed under prefix length {prefix_len}"
            );

            let expected_gaps: Vec<_> = full_gaps
                .iter()
                .filter(|event| event.formed_at_index < prefix_len)
                .cloned()
                .collect();
            let prefix_gaps = detect_fair_value_gaps(prefix).unwrap();
            assert_eq!(
                prefix_gaps, expected_gaps,
                "FVG formation output changed under prefix length {prefix_len}"
            );

            let expected_lifecycle: Vec<_> = full_lifecycle
                .iter()
                .filter(|event| event.observed_at_index < prefix_len)
                .cloned()
                .collect();
            assert_eq!(
                assess_fvg_lifecycle(prefix, &prefix_gaps).unwrap(),
                expected_lifecycle,
                "FVG lifecycle rewrote an already-observed event under prefix length {prefix_len}"
            );

            let expected_sweeps: Vec<_> = full_sweeps
                .iter()
                .filter(|event| event.swept_at_index < prefix_len)
                .cloned()
                .collect();
            assert_eq!(
                detect_confirmed_liquidity_sweeps(prefix, 2, 2).unwrap(),
                expected_sweeps,
                "liquidity-sweep output changed under prefix length {prefix_len}"
            );

            let expected_breaks: Vec<_> = full_breaks
                .iter()
                .filter(|event| event.broken_at_index < prefix_len)
                .cloned()
                .collect();
            assert_eq!(
                detect_confirmed_structure_breaks(prefix, 2, 2).unwrap(),
                expected_breaks,
                "structure-break output changed under prefix length {prefix_len}"
            );
        }
    }

    #[test]
    fn sorted_price_keys_follow_numeric_order_and_normalize_signed_zero() {
        let values = [-1.0e308, -10.0, -0.0, 0.0, 0.5, 10.0, 1.0e308];
        let keys: Vec<_> = values
            .iter()
            .map(|&value| ordered_price_key(value))
            .collect();
        assert!(keys.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(ordered_price_key(-0.0), ordered_price_key(0.0));
    }

    #[test]
    fn swing_points_are_emitted_only_after_right_side_confirmation() {
        let candles = vec![
            c(1, 9.0, 10.0, 8.0, 9.0),
            c(2, 10.0, 12.0, 9.0, 11.0),
            c(3, 11.0, 11.0, 8.5, 9.0),
            c(4, 9.0, 10.0, 7.0, 8.0),
            c(5, 8.0, 9.0, 7.5, 8.5),
        ];
        let swings = detect_confirmed_swings(&candles, 1, 1).unwrap();
        assert!(swings.iter().any(|s| s.kind == "swing_high"
            && s.pivot_index == 1
            && s.confirmed_at_index == 2
            && s.pivot_at_ms == 2
            && s.confirmed_at_ms == 3));
        assert!(swings
            .iter()
            .any(|s| s.kind == "swing_low" && s.pivot_index == 3 && s.confirmed_at_index == 4));
        assert!(swings.iter().all(|s| s.confirmed_at_index > s.pivot_index));
    }

    #[test]
    fn monotonic_deque_swings_match_reference_scan_across_window_sizes() {
        let candles: Vec<Candle> = (0..73)
            .map(|index| {
                let center = 30.0 + ((index * 17) % 19) as f64;
                let high = center + ((index * 7) % 5) as f64;
                let low = center - 1.0 - ((index * 11) % 7) as f64;
                c(
                    index as i64 + 1,
                    low + (high - low) * 0.4,
                    high,
                    low,
                    low + (high - low) * 0.6,
                )
            })
            .collect();

        for left_bars in 1..=5 {
            for right_bars in 1..=5 {
                let actual = detect_confirmed_swings(&candles, left_bars, right_bars).unwrap();
                let mut expected = Vec::<ConfirmedSwing>::new();
                if candles.len() > left_bars + right_bars {
                    for pivot in left_bars..(candles.len() - right_bars) {
                        let current = &candles[pivot];
                        let window = &candles[pivot - left_bars..=pivot + right_bars];
                        let unique_high = window.iter().enumerate().all(|(offset, other)| {
                            let absolute = pivot - left_bars + offset;
                            absolute == pivot || current.high > other.high
                        });
                        let unique_low = window.iter().enumerate().all(|(offset, other)| {
                            let absolute = pivot - left_bars + offset;
                            absolute == pivot || current.low < other.low
                        });
                        let confirmed = pivot + right_bars;
                        if unique_high {
                            expected.push(ConfirmedSwing {
                                kind: "swing_high",
                                pivot_index: pivot,
                                confirmed_at_index: confirmed,
                                pivot_at_ms: current.time,
                                confirmed_at_ms: candles[confirmed].time,
                                price: current.high,
                            });
                        }
                        if unique_low {
                            expected.push(ConfirmedSwing {
                                kind: "swing_low",
                                pivot_index: pivot,
                                confirmed_at_index: confirmed,
                                pivot_at_ms: current.time,
                                confirmed_at_ms: candles[confirmed].time,
                                price: current.low,
                            });
                        }
                    }
                }
                expected
                    .sort_by_key(|swing| (swing.confirmed_at_index, swing.pivot_index, swing.kind));
                assert_eq!(
                    actual, expected,
                    "window left={left_bars}, right={right_bars}"
                );
            }
        }
    }

    #[test]
    fn swing_detector_rejects_tied_pivots_and_invalid_windows() {
        let tied = vec![
            c(1, 9.0, 10.0, 8.0, 9.0),
            c(2, 9.0, 12.0, 8.0, 11.0),
            c(3, 11.0, 12.0, 8.5, 9.0),
            c(4, 9.0, 10.0, 7.0, 8.0),
        ];
        let swings = detect_confirmed_swings(&tied, 1, 1).unwrap();
        assert!(!swings.iter().any(|s| s.kind == "swing_high"));
        assert!(detect_confirmed_swings(&tied, 0, 1).is_err());
        assert!(detect_confirmed_swings(&tied, 1, 0).is_err());
        assert!(detect_confirmed_swings(&tied, 10_001, 1).is_err());
    }

    #[test]
    fn swing_detector_fails_closed_on_invalid_candle_data() {
        let invalid = vec![
            c(1, 10.0, 11.0, 9.0, 10.0),
            c(2, 10.0, 11.0, 9.0, 10.0),
            c(3, 10.0, 9.0, 8.0, 9.0),
        ];
        assert!(detect_confirmed_swings(&invalid, 1, 1).is_err());
    }

    #[test]
    fn swing_classification_labels_higher_and_lower_swings_with_comparison_provenance() {
        let candles = vec![
            c(1, 8.0, 10.0, 5.0, 8.0),
            c(2, 10.0, 15.0, 7.0, 14.0),
            c(3, 9.0, 11.0, 6.0, 8.0),
            c(4, 8.0, 13.0, 4.0, 9.0),
            c(5, 9.0, 12.0, 6.0, 10.0),
            c(6, 10.0, 14.0, 5.0, 11.0),
            c(7, 11.0, 11.0, 7.0, 9.0),
            c(8, 9.0, 12.0, 3.0, 7.0),
            c(9, 7.0, 10.0, 5.0, 8.0),
            c(10, 8.0, 11.0, 4.0, 9.0),
            c(11, 9.0, 10.0, 6.0, 9.0),
        ];
        let labels = classify_confirmed_swings(&candles, 1, 1).unwrap();
        assert!(labels.iter().any(|s| s.pivot_index == 1
            && s.classification == "initial_high"
            && s.confirmed_at_index == 2));
        assert!(labels.iter().any(|s| s.pivot_index == 3
            && s.classification == "lower_high"
            && s.compared_to_pivot_index == Some(1)));
        assert!(labels.iter().any(|s| s.pivot_index == 5
            && s.classification == "higher_high"
            && s.compared_to_pivot_index == Some(3)));
        assert!(labels
            .iter()
            .any(|s| s.pivot_index == 3 && s.classification == "initial_low"));
        assert!(labels.iter().any(|s| s.pivot_index == 7
            && s.classification == "lower_low"
            && s.compared_to_pivot_index == Some(5)));
        assert!(labels.iter().any(|s| s.pivot_index == 9
            && s.classification == "higher_low"
            && s.compared_to_pivot_index == Some(7)));
        assert!(labels.iter().all(|s| s.confirmed_at_index > s.pivot_index));
    }

    #[test]
    fn equal_price_swings_are_not_mislabeled_as_higher_or_lower() {
        let candles = vec![
            c(1, 8.0, 10.0, 5.0, 8.0),
            c(2, 10.0, 12.0, 7.0, 11.0),
            c(3, 9.0, 10.0, 6.0, 8.0),
            c(4, 8.0, 12.0, 4.0, 9.0),
            c(5, 9.0, 10.0, 6.0, 8.0),
            c(6, 10.0, 12.0, 4.0, 11.0),
            c(7, 8.0, 9.0, 6.0, 8.0),
        ];
        let labels = classify_confirmed_swings(&candles, 1, 1).unwrap();
        assert!(labels
            .iter()
            .any(|s| s.pivot_index == 5 && s.classification == "equal_high"));
        assert!(labels
            .iter()
            .any(|s| s.pivot_index == 5 && s.classification == "equal_low"));
    }

    #[test]
    fn bullish_and_bearish_fvg_geometry_is_deterministic_and_confirmed_on_third_bar() {
        let bullish = vec![
            c(1, 10.0, 11.0, 9.0, 10.5),
            c(2, 10.5, 13.0, 10.4, 12.8),
            c(3, 12.9, 14.0, 12.0, 13.5),
        ];
        let gaps = detect_fair_value_gaps(&bullish).unwrap();
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].direction, "bullish");
        assert_eq!(gaps[0].formed_at_index, 2);
        assert_eq!(
            (gaps[0].lower, gaps[0].upper, gaps[0].midpoint),
            (11.0, 12.0, 11.5)
        );

        let bearish = vec![
            c(1, 10.0, 12.0, 9.0, 10.0),
            c(2, 9.5, 10.0, 7.0, 7.5),
            c(3, 7.8, 8.5, 6.0, 6.5),
        ];
        let gaps = detect_fair_value_gaps(&bearish).unwrap();
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].direction, "bearish");
        assert_eq!((gaps[0].lower, gaps[0].upper), (8.5, 9.0));
    }

    #[test]
    fn fvg_detector_does_not_infer_future_fill_state() {
        let candles = vec![
            c(1, 10.0, 11.0, 9.0, 10.0),
            c(2, 11.0, 13.0, 10.5, 12.5),
            c(3, 12.5, 14.0, 12.0, 13.0),
            c(4, 12.0, 12.5, 10.0, 11.0),
        ];
        let gaps = detect_fair_value_gaps(&candles).unwrap();
        assert_eq!(gaps[0].formed_at_index, 2);
        assert_eq!(gaps[0].formed_at_ms, candles[2].time);
        assert!(!format!("{:?}", gaps[0]).contains("fill_state"));
    }

    #[test]
    fn fvg_detector_rejects_finite_prices_whose_derived_width_overflows() {
        let candles = vec![
            c(-3, -1.0e308, -1.0e308, -1.7e308, -1.2e308),
            c(-2, -1.2e308, -1.1e308, -1.6e308, -1.3e308),
            c(-1, 1.2e308, 1.7e308, 1.0e308, 1.3e308),
        ];
        assert!(candles.iter().all(|candle| candle.open.is_finite()
            && candle.high.is_finite()
            && candle.low.is_finite()
            && candle.close.is_finite()));
        assert!(detect_fair_value_gaps(&candles).is_err());
    }

    #[test]
    fn malformed_or_non_monotonic_candles_fail_closed() {
        let malformed = vec![
            c(1, 10.0, 9.0, 8.0, 9.0),
            c(2, 9.0, 10.0, 8.0, 9.0),
            c(3, 9.0, 10.0, 8.0, 9.0),
        ];
        assert!(detect_fair_value_gaps(&malformed).is_err());
        let non_monotonic = vec![
            c(2, 10.0, 11.0, 9.0, 10.0),
            c(1, 10.0, 11.0, 9.0, 10.0),
            c(3, 10.0, 11.0, 9.0, 10.0),
        ];
        assert!(detect_fair_value_gaps(&non_monotonic).is_err());
        assert!(detect_confirmed_liquidity_sweeps(&non_monotonic, 2, 2).is_err());
    }

    #[test]
    fn liquidity_sweep_requires_confirmed_pivot_and_close_back_inside() {
        let candles = vec![
            c(1, 9.0, 10.0, 8.0, 9.0),
            c(2, 9.0, 12.0, 8.5, 11.0),
            c(3, 11.0, 11.5, 9.0, 10.0),
            c(4, 10.0, 10.2, 9.2, 9.8),
            c(5, 10.0, 12.5, 9.4, 11.5),
            c(6, 11.5, 11.8, 9.0, 10.0),
            c(7, 11.5, 13.0, 11.0, 11.5),
        ];
        let sweeps = detect_confirmed_liquidity_sweeps(&candles, 1, 1).unwrap();
        assert!(sweeps.iter().any(|sweep| sweep.side == "buy_side"
            && sweep.pivot_index == 1
            && sweep.pivot_confirmed_at_index == 2
            && sweep.swept_at_index == 4));
        assert!(
            sweeps.iter().any(|sweep| sweep.side == "buy_side"
                && sweep.pivot_index == 1
                && sweep.swept_at_index == 6),
            "price-index optimization must preserve repeated valid sweep observations"
        );
        assert!(sweeps
            .iter()
            .all(|sweep| sweep.swept_at_index > sweep.pivot_confirmed_at_index));
    }

    #[test]
    fn fvg_lifecycle_emits_partial_then_full_fill_once_and_never_on_formation_bar() {
        let candles = vec![
            c(1, 10.0, 11.0, 9.0, 10.5),
            c(2, 10.5, 13.0, 10.4, 12.8),
            c(3, 12.9, 14.0, 12.0, 13.5),
            c(4, 12.5, 12.8, 11.5, 12.0),
            c(5, 11.8, 12.0, 10.8, 11.0),
            c(6, 10.9, 11.2, 10.0, 10.5),
        ];
        let gaps = detect_fair_value_gaps(&candles).unwrap();
        let events = assess_fvg_lifecycle(&candles, &gaps).unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == "partial_fill")
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == "full_fill")
                .count(),
            1
        );
        assert!(events
            .iter()
            .all(|e| e.observed_at_index > e.formed_at_index));
        assert_eq!(
            events
                .iter()
                .find(|e| e.event_type == "partial_fill")
                .unwrap()
                .observed_at_index,
            3
        );
        assert_eq!(
            events
                .iter()
                .find(|e| e.event_type == "full_fill")
                .unwrap()
                .observed_at_index,
            4
        );
    }

    #[test]
    fn fvg_close_beyond_far_boundary_is_only_an_inversion_candidate() {
        let candles = vec![
            c(1, 10.0, 11.0, 9.0, 10.5),
            c(2, 10.5, 13.0, 10.4, 12.8),
            c(3, 12.9, 14.0, 12.0, 13.5),
            c(4, 11.0, 11.4, 10.2, 10.5),
        ];
        let gaps = detect_fair_value_gaps(&candles).unwrap();
        let events = assess_fvg_lifecycle(&candles, &gaps).unwrap();
        assert!(events.iter().any(|e| e.event_type == "full_fill"));
        assert!(events.iter().any(|e| e.event_type == "inversion_candidate"));
        assert!(!events.iter().any(|e| e.event_type == "IFVG"));
    }

    #[test]
    fn fvg_lifecycle_rejects_forged_or_misaligned_gap_references() {
        let candles = vec![
            c(1, 10.0, 11.0, 9.0, 10.5),
            c(2, 10.5, 13.0, 10.4, 12.8),
            c(3, 12.9, 14.0, 12.0, 13.5),
        ];
        let invalid = FairValueGap {
            direction: "bullish",
            formed_at_index: 2,
            formed_at_ms: 999,
            lower: 11.0,
            upper: 12.0,
            midpoint: 11.5,
            width: 1.0,
        };
        assert!(assess_fvg_lifecycle(&candles, &[invalid]).is_err());
    }

    #[test]
    fn fvg_lifecycle_rejects_fabricated_zone_geometry_even_with_valid_timestamp() {
        let candles = vec![
            c(1, 10.0, 11.0, 9.0, 10.5),
            c(2, 10.5, 13.0, 10.4, 12.8),
            c(3, 12.9, 14.0, 12.0, 13.5),
        ];
        let forged = FairValueGap {
            direction: "bullish",
            formed_at_index: 2,
            formed_at_ms: 3,
            lower: 11.1,
            upper: 12.0,
            midpoint: 11.55,
            width: 0.9,
        };
        assert!(assess_fvg_lifecycle(&candles, &[forged]).is_err());
    }

    #[test]
    fn invalid_swing_confirmation_parameters_are_rejected() {
        let candles = vec![
            c(1, 10.0, 11.0, 9.0, 10.0),
            c(2, 10.0, 11.0, 9.0, 10.0),
            c(3, 10.0, 11.0, 9.0, 10.0),
        ];
        assert!(detect_confirmed_liquidity_sweeps(&candles, 0, 2).is_err());
        assert!(detect_confirmed_liquidity_sweeps(&candles, 2, 0).is_err());
    }

    #[test]
    fn indexed_sweep_detector_preserves_confirmation_and_deterministic_event_order() {
        let candles = vec![
            c(1, 9.0, 10.0, 8.0, 9.0),
            c(2, 10.0, 12.0, 9.0, 11.0),
            c(3, 11.0, 11.0, 8.5, 9.0), // confirms high 12 and low 8.5
            c(4, 9.0, 12.5, 8.8, 11.5), // high sweep
            c(5, 11.0, 11.5, 8.0, 9.0), // low sweep
            c(6, 9.0, 12.5, 8.0, 9.0), // potential repeat; valid only if wick/close conditions match
        ];
        let actual = detect_confirmed_liquidity_sweeps(&candles, 1, 1).unwrap();
        assert!(actual.iter().any(|event| event.side == "buy_side"
            && event.level == 12.0
            && event.swept_at_index == 3
            && event.pivot_confirmed_at_index == 2));
        assert!(actual.windows(2).all(|pair| {
            (pair[0].swept_at_index, pair[0].pivot_index, pair[0].side)
                <= (pair[1].swept_at_index, pair[1].pivot_index, pair[1].side)
        }));
    }

    #[test]
    fn structure_break_requires_close_cross_after_level_confirmation() {
        let candles = vec![
            c(1, 9.5, 10.0, 9.0, 9.5),
            c(2, 9.5, 12.0, 9.2, 11.0),
            c(3, 11.0, 11.2, 10.0, 10.5), // confirms pivot at index 1
            c(4, 10.5, 12.8, 10.4, 12.5), // close breaks confirmed level
        ];
        let breaks = detect_confirmed_structure_breaks(&candles, 1, 1).unwrap();
        assert!(breaks.iter().any(|event| event.direction == "bullish"
            && event.level == 12.0
            && event.level_confirmed_at_index == 2
            && event.broken_at_index == 3
            && event.classification == "initial_break"));
    }

    #[test]
    fn wick_only_breach_is_not_a_structure_break() {
        let candles = vec![
            c(1, 9.5, 10.0, 9.0, 9.5),
            c(2, 9.5, 12.0, 9.2, 11.0),
            c(3, 11.0, 11.2, 10.0, 10.5),
            c(4, 10.5, 12.5, 10.2, 11.5),
        ];
        let breaks = detect_confirmed_structure_breaks(&candles, 1, 1).unwrap();
        assert!(!breaks.iter().any(|event| event.level == 12.0));
    }

    #[test]
    fn multi_level_breaks_on_one_candle_use_one_pre_candle_bias_snapshot() {
        let candles = vec![
            c(1, 8.0, 9.0, 7.0, 8.0),
            c(2, 9.0, 12.0, 8.0, 11.0), // swing high 12
            c(3, 10.0, 10.0, 8.0, 9.0), // confirms high 12
            c(4, 9.0, 11.0, 8.5, 10.5), // lower swing high 11
            c(5, 10.0, 10.0, 8.0, 9.5), // confirms high 11
            c(6, 9.5, 13.0, 9.0, 12.5), // breaks both levels on one close
        ];
        let breaks = detect_confirmed_structure_breaks(&candles, 1, 1).unwrap();
        let same_candle: Vec<_> = breaks
            .iter()
            .filter(|event| event.broken_at_index == 5)
            .collect();
        assert_eq!(
            same_candle.len(),
            2,
            "both distinct crossed levels must be retained"
        );
        assert!(same_candle.iter().all(|event| event.direction == "bullish"));
        assert!(same_candle
            .iter()
            .all(|event| event.prior_bias == "unknown"));
        assert!(
            same_candle
                .iter()
                .all(|event| event.classification == "initial_break"),
            "one candle's event order must not turn later levels into continuation breaks"
        );
    }

    #[test]
    fn outside_bar_swing_high_and_low_have_independent_break_lifecycles() {
        let candles = vec![
            c(1, 10.0, 11.0, 9.0, 10.0),
            c(2, 10.0, 15.0, 5.0, 10.0), // outside-bar pivot: both high and low
            c(3, 10.0, 12.0, 8.0, 10.0), // confirms both pivot kinds
            c(4, 10.0, 16.0, 7.0, 15.5), // breaks high only
            c(5, 15.0, 15.2, 4.5, 4.8),  // later breaks low independently
        ];
        let breaks = detect_confirmed_structure_breaks(&candles, 1, 1).unwrap();
        assert!(breaks.iter().any(|event| event.pivot_index == 1
            && event.level_kind == "swing_high"
            && event.broken_at_index == 3));
        assert!(
            breaks.iter().any(|event| event.pivot_index == 1
                && event.level_kind == "swing_low"
                && event.broken_at_index == 4),
            "breaking one side of an outside-bar pivot must not consume the other side"
        );
    }

    #[test]
    fn structure_break_rejects_invalid_confirmation_windows() {
        let candles = vec![
            c(1, 10.0, 11.0, 9.0, 10.0),
            c(2, 10.0, 11.0, 9.0, 10.0),
            c(3, 10.0, 11.0, 9.0, 10.0),
        ];
        assert!(detect_confirmed_structure_breaks(&candles, 0, 1).is_err());
    }

    #[test]
    fn structure_timeline_is_point_in_time_and_has_one_snapshot_per_candle() {
        let candles = vec![
            c(1, 9.0, 10.0, 8.0, 9.0),
            c(2, 10.0, 12.0, 9.0, 11.0),
            c(3, 11.0, 11.0, 8.5, 9.0),
            c(4, 9.0, 10.0, 7.0, 8.0),
            c(5, 8.0, 9.0, 7.5, 8.5),
            c(6, 8.5, 12.5, 8.0, 12.2),
        ];
        let timeline = build_structure_state_timeline(&candles, 1, 1).unwrap();
        assert_eq!(timeline.len(), candles.len());
        assert_eq!(timeline[0].at_ms, candles[0].time);
        assert_eq!(
            timeline[1].active_swing_high, None,
            "unconfirmed pivot must not leak into earlier snapshots"
        );
        assert_eq!(
            timeline[2].active_swing_high,
            Some(12.0),
            "pivot appears at its confirmation index"
        );
        assert_eq!(
            timeline.iter().map(|s| s.at_index).collect::<Vec<_>>(),
            (0..candles.len()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn structure_timeline_rejects_invalid_swing_parameters() {
        let candles = vec![
            c(1, 9.0, 10.0, 8.0, 9.0),
            c(2, 9.0, 11.0, 8.0, 10.0),
            c(3, 10.0, 10.5, 7.5, 8.0),
        ];
        assert!(build_structure_state_timeline(&candles, 0, 1).is_err());
    }
}
