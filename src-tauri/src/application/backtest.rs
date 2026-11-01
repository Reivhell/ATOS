use crate::domain::models::{
    BacktestCostStressScenario, BacktestOutOfSampleFold, BacktestPoint, BacktestRequest,
    BacktestResult, Candle,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Fingerprint an OHLCV sequence using an explicit byte-level format so that it is
/// stable across OS/CPU endianness and independent of JSON float formatting.
/// Version 1: domain separator, u64 candle count, then i64 timestamp and five f64
/// IEEE-754 bit patterns, all encoded little-endian.
pub(crate) fn dataset_sha256(candles: &[Candle]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agentic-trading-os.ohlcv.canonical.v1\0");
    hasher.update((candles.len() as u64).to_le_bytes());
    const RECORD_BYTES: usize = 6 * 8;
    const CHUNK_BYTES: usize = 64 * 1024;
    let mut chunk = Vec::with_capacity(CHUNK_BYTES);
    for candle in candles {
        let mut record = [0_u8; RECORD_BYTES];
        record[..8].copy_from_slice(&candle.time.to_le_bytes());
        for (index, value) in [
            candle.open,
            candle.high,
            candle.low,
            candle.close,
            candle.volume,
        ]
        .into_iter()
        .enumerate()
        {
            let start = 8 + index * 8;
            record[start..start + 8].copy_from_slice(&value.to_bits().to_le_bytes());
        }
        if chunk.len() + record.len() > chunk.capacity() {
            hasher.update(&chunk);
            chunk.clear();
        }
        chunk.extend_from_slice(&record);
    }
    if !chunk.is_empty() {
        hasher.update(&chunk);
    }
    let digest = hasher.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[(byte >> 4) as usize]));
        encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    encoded
}

pub fn run(
    symbol: &str,
    candles: &[Candle],
    request: BacktestRequest,
    source: &str,
) -> Result<BacktestResult, String> {
    let mut baseline = run_once(symbol, candles, request.clone(), source, 0)?;

    // Cost stress probes answer a practical production question: does this result survive
    // plausible friction shocks? Floors make the scenarios meaningful even if the base case
    // was configured with zero costs. Caps are explicit because the core rejects unsafe costs.
    let stress_specs = [
        ("Moderate cost shock", 1.5_f64, 2.0_f64, 1.5_f64, 5.0_f64),
        ("Severe cost shock", 2.0_f64, 5.0_f64, 2.0_f64, 10.0_f64),
    ];
    for (name, fee_multiplier, fee_floor_delta, slippage_multiplier, slippage_floor_delta) in
        stress_specs
    {
        let raw_fee = (request.fee_bps * fee_multiplier).max(request.fee_bps + fee_floor_delta);
        let raw_slippage = (request.slippage_bps * slippage_multiplier)
            .max(request.slippage_bps + slippage_floor_delta);
        let fee_bps = raw_fee.min(500.0);
        let slippage_bps = raw_slippage.min(2_000.0);
        let mut stressed_request = request.clone();
        stressed_request.fee_bps = fee_bps;
        stressed_request.slippage_bps = slippage_bps;
        let stressed = run_once(symbol, candles, stressed_request, source, 0)?;
        baseline
            .cost_stress_scenarios
            .push(BacktestCostStressScenario {
                name: name.into(),
                fee_bps,
                slippage_bps,
                fee_cap_reached: raw_fee > 500.0,
                slippage_cap_reached: raw_slippage > 2_000.0,
                final_equity: stressed.final_equity,
                total_return_pct: stressed.total_return_pct,
                return_delta_percentage_points: stressed.total_return_pct
                    - baseline.total_return_pct,
                max_drawdown_pct: stressed.max_drawdown_pct,
                sharpe_ratio: stressed.sharpe_ratio,
                trade_count: stressed.trade_count,
            });
    }

    baseline.warnings.push("Cost stress scenarios are deterministic fee/slippage shocks, not exchange-specific liquidity, spread, latency, market impact, or fill-quality forecasts. Cost ceilings may limit the applied shock.".into());
    baseline.out_of_sample_folds = build_out_of_sample_folds(symbol, candles, &request, source)?;
    if baseline.out_of_sample_folds.is_empty() {
        baseline.warnings.push(format!(
            "Out-of-sample stability validation was not produced: at least two complete test windows are required, each at least max(2 × slow SMA period, 30) bars, plus {} bars of indicator warm-up. The single-window backtest is not evidence of out-of-sample robustness.",
            request.slow_period + 2
        ));
    } else {
        baseline.warnings.push(format!(
            "Automatic out-of-sample stability check evaluated {} chronological windows using fixed {}-bar tests and {} warm-up bars per window. Windows reset to initial capital and do not carry positions between folds; this measures window-to-window stability, not parameter optimization or proof of future performance.",
            baseline.out_of_sample_folds.len(),
            baseline.out_of_sample_folds.first().map(|fold| fold.candles_evaluated).unwrap_or(0),
            request.slow_period + 2
        ));
    }
    Ok(baseline)
}

/// Training-only diagnostics for a parameter candidate. These values are never
/// interpreted as out-of-sample evidence or execution authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentCandidateScore {
    pub fast_period: usize,
    pub slow_period: usize,
    pub training_sharpe_ratio: f64,
    pub training_return_pct: f64,
    pub training_max_drawdown_pct: f64,
    pub training_trade_count: usize,
    pub qualifies_for_selection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WalkForwardTournamentFold {
    pub fold_index: usize,
    pub training_start_ms: i64,
    pub training_end_ms: i64,
    pub testing_start_ms: i64,
    pub testing_end_ms: i64,
    pub selected_fast_period: Option<usize>,
    pub selected_slow_period: Option<usize>,
    pub selected_training_sharpe_ratio: Option<f64>,
    pub out_of_sample_return_pct: Option<f64>,
    pub out_of_sample_max_drawdown_pct: Option<f64>,
    pub out_of_sample_sharpe_ratio: Option<f64>,
    pub out_of_sample_trade_count: Option<usize>,
    pub candidate_scores: Vec<TournamentCandidateScore>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyParameterTournament {
    pub algorithm_version: String,
    pub selection_metric: String,
    pub candidate_count: usize,
    pub candidate_evaluations: usize,
    pub requested_folds: usize,
    pub completed_folds: usize,
    /// Positive folds divided by requested folds; missing candidates count as failures.
    pub positive_oos_fold_ratio: Option<f64>,
    pub median_oos_return_pct: Option<f64>,
    pub worst_oos_max_drawdown_pct: Option<f64>,
    /// Share of completed folds selecting the modal parameter pair.
    pub parameter_stability_pct: Option<f64>,
    pub dataset_sha256: String,
    pub dataset_start_ms: i64,
    pub dataset_end_ms: i64,
    pub folds: Vec<WalkForwardTournamentFold>,
    pub blockers: Vec<String>,
    pub caveats: Vec<String>,
}

const TOURNAMENT_MAX_CANDIDATES: usize = 25;
const TOURNAMENT_MIN_TRAINING_TRADES: usize = 3;
const TOURNAMENT_MAX_FOLDS: usize = 5;
const TOURNAMENT_WORK_BUDGET: u64 = 50_000_000;

/// Ordering used by both the selector and the persisted-evidence verifier.
/// `Less` means the left candidate is preferred. All score comparisons are exact
/// total-order float comparisons; no tolerance-based tie can disagree with verification.
pub(crate) fn compare_tournament_candidate_priority(
    left: (f64, f64, usize, usize),
    right: (f64, f64, usize, usize),
) -> std::cmp::Ordering {
    let (left_sharpe, left_drawdown, left_fast, left_slow) = left;
    let (right_sharpe, right_drawdown, right_fast, right_slow) = right;
    right_sharpe
        .total_cmp(&left_sharpe)
        .then_with(|| left_drawdown.total_cmp(&right_drawdown))
        .then_with(|| (left_fast, left_slow).cmp(&(right_fast, right_slow)))
}

/// Runs an expanding-window, nested walk-forward tournament over a bounded SMA
/// neighbourhood. Each outer fold selects parameters from its training prefix
/// only, then evaluates the selected pair on the immediately following unseen
/// block. The test block is never used to select its own parameters.
pub fn run_walk_forward_parameter_tournament(
    symbol: &str,
    candles: &[Candle],
    base_request: &BacktestRequest,
    source: &str,
    requested_folds: usize,
) -> Result<StrategyParameterTournament, String> {
    if !(2..=TOURNAMENT_MAX_FOLDS).contains(&requested_folds) {
        return Err(format!(
            "Walk-forward tournament requires 2 to {TOURNAMENT_MAX_FOLDS} folds."
        ));
    }
    if base_request.fast_period < 2
        || base_request.slow_period <= base_request.fast_period
        || base_request.slow_period > 300
    {
        return Err("Tournament anchor periods must satisfy 2 <= fast < slow <= 300.".into());
    }
    if candles.len() > 2_000_000 {
        return Err("Tournament dataset exceeds the 2,000,000 candle safety limit.".into());
    }
    if candles.len() < 150 {
        return Err(
            "At least 150 validated candles are required for a nested walk-forward tournament."
                .into(),
        );
    }

    let fast_values = neighborhood(base_request.fast_period, &[4, 2, 0, -2, -4], 2, 100);
    let slow_values = neighborhood(base_request.slow_period, &[8, 4, 0, -4, -8], 3, 300);
    let mut candidates = Vec::new();
    for fast in fast_values {
        for slow in &slow_values {
            if fast < *slow {
                candidates.push((fast, *slow));
            }
        }
    }
    candidates.sort_unstable();
    candidates.dedup();
    if !candidates.contains(&(base_request.fast_period, base_request.slow_period)) {
        candidates.push((base_request.fast_period, base_request.slow_period));
        candidates.sort_unstable();
    }
    if candidates.is_empty() || candidates.len() > TOURNAMENT_MAX_CANDIDATES {
        return Err("Tournament parameter grid violates the candidate-count safety bound.".into());
    }

    let slowest = candidates
        .iter()
        .map(|(_, slow)| *slow)
        .max()
        .unwrap_or(base_request.slow_period);
    let test_bars = candles.len() / (requested_folds + 2);
    if test_bars < 30 {
        return Err("Each walk-forward testing block must contain at least 30 candles.".into());
    }
    let initial_training_bars = candles
        .len()
        .saturating_sub(requested_folds.saturating_mul(test_bars));
    let minimum_training_bars = slowest.saturating_mul(2).saturating_add(30).max(120);
    if initial_training_bars < minimum_training_bars {
        return Err(format!(
            "The tournament needs at least {minimum_training_bars} initial training candles for the current parameter grid; found {initial_training_bars}."
        ));
    }

    // run_once validates every prefix it consumes. Bound the sum of training-prefix
    // replays plus the selected OOS prefixes before allocating results or doing work.
    let mut estimated_work = 0_u64;
    for fold_index in 0..requested_folds {
        let train_end = initial_training_bars + fold_index * test_bars;
        let test_end = train_end + test_bars;
        let training_cost = (candidates.len() as u64)
            .checked_mul(train_end as u64)
            .ok_or_else(|| "Tournament training work estimate overflowed.".to_string())?;
        estimated_work = estimated_work
            .checked_add(training_cost)
            .and_then(|value| value.checked_add(test_end as u64))
            .ok_or_else(|| "Tournament work estimate overflowed.".to_string())?;
    }
    if estimated_work > TOURNAMENT_WORK_BUDGET {
        return Err(format!(
            "Tournament estimated work of {estimated_work} candle-evaluations exceeds the {TOURNAMENT_WORK_BUDGET} safety budget. Reduce data length or parameter breadth."
        ));
    }

    let mut folds = Vec::with_capacity(requested_folds);
    let mut selected_pairs = Vec::with_capacity(requested_folds);
    let mut positive_folds = 0_usize;
    let mut oos_returns = Vec::with_capacity(requested_folds);
    let mut oos_drawdowns = Vec::with_capacity(requested_folds);
    let mut candidate_evaluations = 0_usize;

    for fold_offset in 0..requested_folds {
        let train_end = initial_training_bars + fold_offset * test_bars;
        let test_end = train_end + test_bars;
        let training = &candles[..train_end];
        let mut candidate_scores = Vec::with_capacity(candidates.len());
        let mut selected: Option<(usize, usize, f64, f64)> = None;

        for (fast, slow) in &candidates {
            let mut candidate_request = base_request.clone();
            candidate_request.fast_period = *fast;
            candidate_request.slow_period = *slow;
            let training_result = run_once(
                symbol,
                training,
                candidate_request,
                source,
                slow.saturating_add(2),
            )?;
            candidate_evaluations = candidate_evaluations
                .checked_add(1)
                .ok_or_else(|| "Tournament candidate evaluation count overflowed.".to_string())?;
            let qualifies = training_result.trade_count >= TOURNAMENT_MIN_TRAINING_TRADES;
            candidate_scores.push(TournamentCandidateScore {
                fast_period: *fast,
                slow_period: *slow,
                training_sharpe_ratio: training_result.sharpe_ratio,
                training_return_pct: training_result.total_return_pct,
                training_max_drawdown_pct: training_result.max_drawdown_pct.abs(),
                training_trade_count: training_result.trade_count,
                qualifies_for_selection: qualifies,
            });
            if !qualifies {
                continue;
            }
            let replace =
                selected.is_none_or(|(best_fast, best_slow, best_sharpe, best_drawdown)| {
                    compare_tournament_candidate_priority(
                        (
                            training_result.sharpe_ratio,
                            training_result.max_drawdown_pct.abs(),
                            *fast,
                            *slow,
                        ),
                        (best_sharpe, best_drawdown, best_fast, best_slow),
                    )
                    .is_lt()
                });
            if replace {
                selected = Some((
                    *fast,
                    *slow,
                    training_result.sharpe_ratio,
                    training_result.max_drawdown_pct.abs(),
                ));
            }
        }

        let training_start_ms = candles
            .first()
            .map(|candle| candle.time)
            .unwrap_or_default();
        let training_end_ms = candles[train_end - 1].time;
        let testing_start_ms = candles[train_end].time;
        let testing_end_ms = candles[test_end - 1].time;
        if let Some((selected_fast, selected_slow, training_sharpe, _)) = selected {
            let mut selected_request = base_request.clone();
            selected_request.fast_period = selected_fast;
            selected_request.slow_period = selected_slow;
            let out_of_sample = run_once(
                symbol,
                &candles[..test_end],
                selected_request,
                source,
                train_end,
            )?;
            candidate_evaluations = candidate_evaluations
                .checked_add(1)
                .ok_or_else(|| "Tournament evaluation count overflowed.".to_string())?;
            if !out_of_sample.total_return_pct.is_finite()
                || !out_of_sample.max_drawdown_pct.is_finite()
                || !out_of_sample.sharpe_ratio.is_finite()
                || out_of_sample.final_equity <= 0.0
            {
                return Err(format!(
                    "Tournament fold {} produced invalid out-of-sample metrics.",
                    fold_offset + 1
                ));
            }
            if out_of_sample.total_return_pct > 0.0 {
                positive_folds += 1;
            }
            oos_returns.push(out_of_sample.total_return_pct);
            oos_drawdowns.push(out_of_sample.max_drawdown_pct.abs());
            selected_pairs.push((selected_fast, selected_slow));
            folds.push(WalkForwardTournamentFold {
                fold_index: fold_offset + 1,
                training_start_ms,
                training_end_ms,
                testing_start_ms,
                testing_end_ms,
                selected_fast_period: Some(selected_fast),
                selected_slow_period: Some(selected_slow),
                selected_training_sharpe_ratio: Some(training_sharpe),
                out_of_sample_return_pct: Some(out_of_sample.total_return_pct),
                out_of_sample_max_drawdown_pct: Some(out_of_sample.max_drawdown_pct.abs()),
                out_of_sample_sharpe_ratio: Some(out_of_sample.sharpe_ratio),
                out_of_sample_trade_count: Some(out_of_sample.trade_count),
                candidate_scores,
                status: "COMPLETED".into(),
            });
        } else {
            folds.push(WalkForwardTournamentFold {
                fold_index: fold_offset + 1,
                training_start_ms,
                training_end_ms,
                testing_start_ms,
                testing_end_ms,
                selected_fast_period: None,
                selected_slow_period: None,
                selected_training_sharpe_ratio: None,
                out_of_sample_return_pct: None,
                out_of_sample_max_drawdown_pct: None,
                out_of_sample_sharpe_ratio: None,
                out_of_sample_trade_count: None,
                candidate_scores,
                status: "NO_QUALIFIED_CANDIDATE".into(),
            });
        }
    }

    oos_returns.sort_by(f64::total_cmp);
    oos_drawdowns.sort_by(f64::total_cmp);
    let completed_folds = selected_pairs.len();
    let positive_oos_fold_ratio =
        (completed_folds > 0).then_some(positive_folds as f64 / requested_folds as f64);
    let median_oos_return_pct = if oos_returns.is_empty() {
        None
    } else {
        Some(median_sorted(&oos_returns))
    };
    let worst_oos_max_drawdown_pct = oos_drawdowns.last().copied();
    let parameter_stability_pct = if selected_pairs.is_empty() {
        None
    } else {
        let mut counts = std::collections::BTreeMap::new();
        for pair in &selected_pairs {
            *counts.entry(*pair).or_insert(0_usize) += 1;
        }
        counts
            .values()
            .copied()
            .max()
            .map(|count| count as f64 / completed_folds as f64 * 100.0)
    };
    let mut blockers = Vec::new();
    if completed_folds < requested_folds {
        blockers.push(format!("Only {completed_folds} of {requested_folds} outer folds selected a candidate with at least {TOURNAMENT_MIN_TRAINING_TRADES} training trades."));
    }
    if completed_folds == 0 {
        blockers
            .push("No candidate met the minimum training-trade requirement in any fold.".into());
    }
    Ok(StrategyParameterTournament {
        algorithm_version: "atos-nested-walk-forward-sma-v1".into(),
        selection_metric: "Highest training Sharpe; ties resolve by lower training drawdown, then lexicographic periods".into(),
        candidate_count: candidates.len(),
        candidate_evaluations,
        requested_folds,
        completed_folds,
        positive_oos_fold_ratio,
        median_oos_return_pct,
        worst_oos_max_drawdown_pct,
        parameter_stability_pct,
        dataset_sha256: dataset_sha256(candles),
        dataset_start_ms: candles.first().map(|candle| candle.time).unwrap_or_default(),
        dataset_end_ms: candles.last().map(|candle| candle.time).unwrap_or_default(),
        folds,
        blockers,
        caveats: vec![
            "Each outer fold selects SMA periods using only its expanding training prefix and tests the chosen pair on the next unseen time block.".into(),
            "The parameter grid is a bounded local neighbourhood around the requested anchor; this is not a global optimizer or an alpha library.".into(),
            "Selection uses training Sharpe after a minimum-trade filter. Repeated parameter selection across folds is reported as stability, not as a formal probability-of-backtest-overfitting estimate.".into(),
            "Each outer test fold starts flat with independent initial capital; positions are not carried across fold boundaries.".into(),
        ],
    })
}

fn neighborhood(anchor: usize, offsets: &[i32], minimum: usize, maximum: usize) -> Vec<usize> {
    let mut values = offsets
        .iter()
        .filter_map(|offset| {
            let candidate = if *offset < 0 {
                anchor.checked_sub(offset.unsigned_abs() as usize)
            } else {
                anchor.checked_add(*offset as usize)
            }?;
            (candidate >= minimum && candidate <= maximum).then_some(candidate)
        })
        .collect::<Vec<_>>();
    values.push(anchor.clamp(minimum, maximum));
    values.sort_unstable();
    values.dedup();
    values
}

fn median_sorted(sorted: &[f64]) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        sorted[middle - 1] * 0.5 + sorted[middle] * 0.5
    } else {
        sorted[middle]
    }
}

fn build_out_of_sample_folds(
    symbol: &str,
    candles: &[Candle],
    request: &BacktestRequest,
    source: &str,
) -> Result<Vec<BacktestOutOfSampleFold>, String> {
    let warmup_bars = request
        .slow_period
        .checked_add(2)
        .ok_or_else(|| "Out-of-sample warm-up size overflowed.".to_string())?;
    let minimum_test_bars = request.slow_period.saturating_mul(2).max(30);
    if candles.len() <= warmup_bars {
        return Ok(Vec::new());
    }
    let available_oos_bars = candles.len() - warmup_bars;
    let complete_window_count = available_oos_bars / minimum_test_bars;
    if complete_window_count < 2 {
        return Ok(Vec::new());
    }

    // Keep the most recent 12 complete, fixed-length windows. The window size is independent
    // of the overall sample length, which avoids squeezing/reshaping every fold to fit a report.
    let fold_count = complete_window_count.min(12);
    let first_window_number = complete_window_count - fold_count;
    let mut folds = Vec::with_capacity(fold_count);
    for fold_offset in 0..fold_count {
        let window_number = first_window_number + fold_offset;
        let start = warmup_bars + window_number * minimum_test_bars;
        let end = start + minimum_test_bars;
        let context_start = start - warmup_bars;
        let window = &candles[context_start..end];
        let fold_result = run_once(symbol, window, request.clone(), source, warmup_bars)?;
        folds.push(BacktestOutOfSampleFold {
            fold_index: fold_offset + 1,
            start_ms: candles[start].time,
            end_ms: candles[end - 1].time,
            candles_evaluated: end - start,
            final_equity: fold_result.final_equity,
            total_return_pct: fold_result.total_return_pct,
            max_drawdown_pct: fold_result.max_drawdown_pct,
            sharpe_ratio: fold_result.sharpe_ratio,
            trade_count: fold_result.trade_count,
        });
    }
    Ok(folds)
}

fn run_once(
    symbol: &str,
    candles: &[Candle],
    request: BacktestRequest,
    source: &str,
    evaluation_start_index: usize,
) -> Result<BacktestResult, String> {
    if request.fast_period < 2 || request.slow_period <= request.fast_period {
        return Err(
            "Use a fast period >= 2 and a slow period greater than the fast period.".into(),
        );
    }
    if request.fast_period > 10_000 || request.slow_period > 100_000 {
        return Err("Moving-average periods exceed the supported limit.".into());
    }
    if candles.len() > 2_000_000 {
        return Err("Dataset exceeds the 2,000,000 candle safety limit.".into());
    }
    if candles.len() < request.slow_period + 3 {
        return Err(format!(
            "Not enough candles. Need at least {} and found {}.",
            request.slow_period + 3,
            candles.len()
        ));
    }
    if evaluation_start_index >= candles.len() {
        return Err("Evaluation start index must be inside the candle series.".into());
    }
    if !request.initial_capital.is_finite() || request.initial_capital <= 0.0 {
        return Err("Initial capital must be a finite positive number.".into());
    }
    if !request.fee_bps.is_finite() || !(0.0..=500.0).contains(&request.fee_bps) {
        return Err("Fees must be finite and between 0 and 500 basis points.".into());
    }
    if !request.slippage_bps.is_finite() || !(0.0..=2_000.0).contains(&request.slippage_bps) {
        return Err("Slippage must be finite and between 0 and 2,000 basis points.".into());
    }
    if request.initial_capital > 1_000_000_000_000.0 {
        return Err("Initial capital exceeds the supported $1 trillion safety limit.".into());
    }
    let mut previous_time: Option<i64> = None;
    let mut expected_interval_ms: Option<i64> = None;
    let mut close_prefix = Vec::with_capacity(candles.len() + 1);
    close_prefix.push(0.0_f64);
    let mut prefix_total = 0.0_f64;
    for (index, candle) in candles.iter().enumerate() {
        if let Some(prev) = previous_time {
            if candle.time <= prev {
                return Err(format!(
                    "Candle timestamps must be strictly increasing (invalid row {index})."
                ));
            }
        }
        if !candle.open.is_finite()
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
            return Err(format!("Candle OHLCV values are invalid at row {index}."));
        }
        if let Some(prev) = previous_time {
            let observed_interval_ms = candle.time.checked_sub(prev).ok_or_else(|| {
                format!("Candle timestamp delta overflows the supported range at row {index}.")
            })?;
            match expected_interval_ms {
                None => expected_interval_ms = Some(observed_interval_ms),
                Some(expected) if observed_interval_ms != expected => {
                    let missing_intervals = if observed_interval_ms % expected == 0 {
                        format!(
                            "; approximately {} interval(s) missing",
                            observed_interval_ms / expected - 1
                        )
                    } else {
                        String::new()
                    };
                    return Err(format!(
                        "Backtest requires a continuous, uniform candle series: row {index} has a {observed_interval_ms} ms delta, expected {expected} ms{missing_intervals}. Fill gaps from a trusted source or select a session-aware dataset before backtesting."
                    ));
                }
                Some(_) => {}
            }
        }
        previous_time = Some(candle.time);
        prefix_total += candle.close;
        if !prefix_total.is_finite() {
            return Err(format!(
                "Cumulative close-price sum exceeds the supported numeric range at row {index}."
            ));
        }
        close_prefix.push(prefix_total);
    }

    // Cadence is accepted only when every adjacent bar agrees. A median hides internal
    // missing bars and can distort both annualized risk metrics and the apparent strategy path.
    // This engine currently targets continuous markets; session-aware instruments need a
    // venue calendar and must not be silently treated as continuous data.
    let median_interval_ms = expected_interval_ms.unwrap_or(300_000).max(1);
    let periods_per_year = ((365.25 * 24.0 * 60.0 * 60.0 * 1_000.0) / median_interval_ms as f64)
        .clamp(1.0, 365.25 * 24.0 * 60.0 * 60.0);

    let initial = request.initial_capital;
    let fee_rate = request.fee_bps / 10_000.0;
    let slip_rate = request.slippage_bps / 10_000.0;
    let mut cash = initial;
    let mut quantity = 0.0;
    let mut in_position = false;
    let mut entry_cash_spent = 0.0;
    let mut peak = initial;
    let mut max_drawdown = 0.0_f64;
    let mut previous_equity = initial;
    let mut bar_returns: Vec<f64> = Vec::new();
    let mut trade_pnls: Vec<f64> = Vec::new();
    let mut points: Vec<BacktestPoint> = Vec::with_capacity(candles.len() - evaluation_start_index);
    let first_price = candles[evaluation_start_index].close;
    let benchmark_entry_price = first_price * (1.0 + slip_rate) * (1.0 + fee_rate);
    if !benchmark_entry_price.is_finite() || benchmark_entry_price <= 0.0 {
        return Err("Benchmark entry price is outside the supported numeric range.".into());
    }
    let benchmark_quantity = initial / benchmark_entry_price;
    if !benchmark_quantity.is_finite() || benchmark_quantity <= 0.0 {
        return Err("Benchmark quantity is outside the supported numeric range.".into());
    }

    for (i, candle) in candles.iter().enumerate().skip(evaluation_start_index) {
        let price = candle.close;
        if i > request.slow_period {
            // Detect a true crossover using two completed candles, then execute at this bar's
            // open. The execution bar's close is never used to decide the order.
            let signal_index = i - 1;
            let current_end = signal_index + 1;
            let previous_end = signal_index;
            // Prefix sums make every moving average O(1) per bar.
            let fast = (close_prefix[current_end]
                - close_prefix[current_end - request.fast_period])
                / request.fast_period as f64;
            let slow = (close_prefix[current_end]
                - close_prefix[current_end - request.slow_period])
                / request.slow_period as f64;
            let previous_fast = (close_prefix[previous_end]
                - close_prefix[previous_end - request.fast_period])
                / request.fast_period as f64;
            let previous_slow = (close_prefix[previous_end]
                - close_prefix[previous_end - request.slow_period])
                / request.slow_period as f64;
            let crossed_up = fast > slow && previous_fast <= previous_slow;
            let crossed_down = fast <= slow && previous_fast > previous_slow;
            if crossed_up && !in_position {
                let fill = candle.open * (1.0 + slip_rate);
                let entry_denominator = fill * (1.0 + fee_rate);
                if !fill.is_finite()
                    || fill <= 0.0
                    || !entry_denominator.is_finite()
                    || entry_denominator <= 0.0
                {
                    return Err(format!(
                        "Buy fill is outside the supported numeric range at row {i}."
                    ));
                }
                let affordable = cash / entry_denominator;
                if !affordable.is_finite() || affordable <= 0.0 {
                    return Err(format!(
                        "Buy quantity is outside the supported numeric range at row {i}."
                    ));
                }
                let cost = affordable * fill;
                let fee = cost * fee_rate;
                let total_cost = cost + fee;
                let remaining_cash = cash - total_cost;
                if !cost.is_finite()
                    || !fee.is_finite()
                    || !total_cost.is_finite()
                    || !remaining_cash.is_finite()
                    || remaining_cash < -initial * 1e-12
                {
                    return Err(format!(
                        "Buy accounting exceeded the supported numeric range at row {i}."
                    ));
                }
                quantity = affordable;
                cash = remaining_cash.max(0.0);
                entry_cash_spent = total_cost;
                in_position = true;
            } else if crossed_down && in_position {
                let fill = candle.open * (1.0 - slip_rate);
                let proceeds = quantity * fill;
                let fee = proceeds * fee_rate;
                let net_proceeds = proceeds - fee;
                let resulting_cash = cash + net_proceeds;
                let trade_pnl = net_proceeds - entry_cash_spent;
                if !fill.is_finite()
                    || fill <= 0.0
                    || !proceeds.is_finite()
                    || !fee.is_finite()
                    || !net_proceeds.is_finite()
                    || !resulting_cash.is_finite()
                    || !trade_pnl.is_finite()
                {
                    return Err(format!(
                        "Sell accounting exceeded the supported numeric range at row {i}."
                    ));
                }
                cash = resulting_cash;
                trade_pnls.push(trade_pnl);
                quantity = 0.0;
                        in_position = false;
            }
        }
        let equity = cash + quantity * price;
        let benchmark_equity = benchmark_quantity * price;
        if !equity.is_finite() || !benchmark_equity.is_finite() {
            return Err(format!(
                "Equity calculation exceeded the supported numeric range at row {i}."
            ));
        }
        peak = peak.max(equity);
        if peak > 0.0 {
            max_drawdown = max_drawdown.max((peak - equity) / peak * 100.0);
        }
        if previous_equity > 0.0 {
            bar_returns.push(equity / previous_equity - 1.0);
        }
        previous_equity = equity;
        points.push(BacktestPoint {
            time: candle.time,
            equity,
            benchmark: benchmark_equity,
        });
    }
    if in_position {
        let last = candles.last().expect("length validated").close;
        let fill = last * (1.0 - slip_rate);
        let proceeds = quantity * fill;
        let fee = proceeds * fee_rate;
        let net = cash + proceeds - fee;
        let trade_pnl = proceeds - fee - entry_cash_spent;
        if !fill.is_finite()
            || fill <= 0.0
            || !proceeds.is_finite()
            || !fee.is_finite()
            || !net.is_finite()
            || !trade_pnl.is_finite()
        {
            return Err("Final liquidation exceeded the supported numeric range.".into());
        }
        trade_pnls.push(trade_pnl);
        cash = net;
        quantity = 0.0;
        entry_cash_spent = 0.0;
        if let Some(last_point) = points.last_mut() {
            last_point.equity = net;
        }
        if points.len() > 1 {
            let previous = points[points.len() - 2].equity;
            if previous > 0.0 {
                if let Some(last_return) = bar_returns.last_mut() {
                    *last_return = net / previous - 1.0;
                }
            }
        }
        peak = peak.max(net);
        if peak > 0.0 {
            max_drawdown = max_drawdown.max((peak - net) / peak * 100.0);
        }
    }
    // The buy-and-hold benchmark is liquidated at the final close regardless of whether
    // the strategy itself still has an open position. Otherwise benchmark costs depend on
    // an unrelated strategy state and can bias the reported relative performance.
    if let Some(last_point) = points.last_mut() {
        last_point.benchmark *= 1.0 - slip_rate;
        last_point.benchmark *= 1.0 - fee_rate;
        if !last_point.benchmark.is_finite() || last_point.benchmark < 0.0 {
            return Err("Benchmark final liquidation exceeded the supported numeric range.".into());
        }
    }
    let final_equity = cash + quantity * candles.last().expect("length validated").close;
    let wins = trade_pnls.iter().filter(|p| **p > 0.0).count();
    let win_rate = if trade_pnls.is_empty() {
        None
    } else {
        Some(wins as f64 / trade_pnls.len() as f64 * 100.0)
    };
    let gains = trade_pnls.iter().filter(|p| **p > 0.0).sum::<f64>();
    let losses = trade_pnls
        .iter()
        .filter(|p| **p < 0.0)
        .map(|p| p.abs())
        .sum::<f64>();
    // A null profit factor needs the gross profit/loss values to distinguish a true infinity
    // (profitable trades and no losses) from an undefined statistic (no gains and no losses).
    let profit_factor = if losses > 0.0 {
        Some(gains / losses)
    } else {
        None
    };
    let total_return_pct = (final_equity / initial - 1.0) * 100.0;
    if !final_equity.is_finite()
        || !total_return_pct.is_finite()
        || win_rate.is_some_and(|value| !value.is_finite())
        || !max_drawdown.is_finite()
        || !gains.is_finite()
        || !losses.is_finite()
        || profit_factor.is_some_and(|value| !value.is_finite())
    {
        return Err("Final performance metrics exceeded the supported numeric range.".into());
    }
    let mean = if bar_returns.is_empty() {
        0.0
    } else {
        bar_returns.iter().sum::<f64>() / bar_returns.len() as f64
    };
    let variance = if bar_returns.len() > 1 {
        bar_returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (bar_returns.len() - 1) as f64
    } else {
        0.0
    };
    let stddev = variance.sqrt();
    // Cadence-aware annualization avoids reporting a five-minute Sharpe for hourly or daily data.
    // The estimate is still fragile for short, irregular, or synthetic samples.
    let sharpe = if stddev > 0.0 {
        mean / stddev * periods_per_year.sqrt()
    } else {
        0.0
    };
    if !mean.is_finite() || !variance.is_finite() || !stddev.is_finite() || !sharpe.is_finite() {
        return Err("Return statistics exceeded the supported numeric range.".into());
    }
    let mut warnings = vec!["Backtest is a simulation, not a prediction. Fill quality and liquidity can differ materially in live markets.".to_string()];
    if source.contains("fixture") || source.contains("Fixture") || source.contains("deterministic")
    {
        warnings.push("Input candles are bundled synthetic fixture data. Do not use these metrics to evaluate a real strategy.".into());
    }
    if profit_factor.is_none() && gains > 0.0 && losses == 0.0 {
        warnings.push("Profit factor is mathematically unbounded because profitable trades exist and no losing trades were recorded; gross profit and gross loss are included so consumers can distinguish this from missing data.".into());
    } else if trade_pnls.is_empty() {
        warnings.push("Win rate and profit factor are unavailable because no closed strategy trades exist; do not interpret unavailable metrics as 0% win rate or infinite profit factor.".into());
    } else if gains == 0.0 && losses == 0.0 {
        warnings.push("Profit factor is unavailable because closed trades contain no non-zero PnL; win rate still counts break-even trades as non-winners.".into());
    }
    warnings.push("Signals use the prior completed candle and execute at the next bar open; the final open position is liquidated at the last close. Fees and fixed slippage are applied, but order-book depth, latency and market impact are not modeled.".into());
    warnings.push(format!("Sharpe annualization uses a median bar interval of {median_interval_ms} ms (approximately {periods_per_year:.0} periods/year); treat it as an estimate, especially on short or irregular datasets."));
    let first_candle = candles
        .first()
        .expect("minimum candle count checked before backtest");
    let last_candle = candles
        .last()
        .expect("minimum candle count checked before backtest");
    Ok(BacktestResult {
        id: Uuid::new_v4().to_string(),
        symbol: symbol.into(),
        strategy_name: "SMA Crossover (deterministic)".into(),
        fast_period: request.fast_period,
        slow_period: request.slow_period,
        initial_capital: initial,
        final_equity,
        total_return_pct,
        max_drawdown_pct: -max_drawdown,
        win_rate_pct: win_rate,
        profit_factor,
        gross_profit: gains,
        gross_loss: losses,
        sharpe_ratio: sharpe,
        trade_count: trade_pnls.len(),
        fee_bps: request.fee_bps,
        slippage_bps: request.slippage_bps,
        candles_used: candles.len(),
        data_source: source.into(),
        dataset_sha256: dataset_sha256(candles),
        dataset_encoding_version: 1,
        dataset_start_ms: first_candle.time,
        dataset_end_ms: last_candle.time,
        median_candle_interval_ms: median_interval_ms,
        equity_curve: points,
        cost_stress_scenarios: Vec::new(),
        out_of_sample_folds: Vec::new(),
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backtest_rejects_bad_parameters() {
        let candles = (0..100)
            .map(|i| Candle {
                time: i as i64,
                open: 10.0,
                high: 11.0,
                low: 9.0,
                close: 10.0 + i as f64 * 0.01,
                volume: 1.0,
            })
            .collect::<Vec<_>>();
        let req = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 20,
            slow_period: 10,
            initial_capital: 1000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        assert!(run("TEST", &candles, req, "fixture").is_err());
    }

    #[test]
    fn backtest_rejects_missing_or_irregular_candle_intervals() {
        let mut candles = (0..100)
            .map(|i| {
                let close = 100.0 + i as f64 * 0.1;
                Candle {
                    time: 1_700_000_000_000 + i as i64 * 300_000,
                    open: close,
                    high: close + 1.0,
                    low: close - 1.0,
                    close,
                    volume: 1.0,
                }
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        for candle in candles.iter_mut().skip(50) {
            candle.time += 300_000;
        }
        let error = run("TEST", &candles, request.clone(), "fixture").unwrap_err();
        assert!(error.contains("continuous, uniform candle series"));
        assert!(error.contains("approximately 1 interval(s) missing"));

        candles[50].time -= 150_000;
        let error = run("TEST", &candles, request, "fixture").unwrap_err();
        assert!(error.contains("continuous, uniform candle series"));
    }

    #[test]
    fn backtest_rejects_timestamp_delta_overflow() {
        let mut candles = (0..100)
            .map(|i| {
                let close = 100.0 + i as f64 * 0.1;
                Candle {
                    time: i as i64 - 1,
                    open: close,
                    high: close + 1.0,
                    low: close - 1.0,
                    close,
                    volume: 1.0,
                }
            })
            .collect::<Vec<_>>();
        candles[0].time = i64::MIN;
        candles[1].time = 0;
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let error = run("TEST", &candles, request, "fixture").unwrap_err();
        assert!(error.contains("timestamp delta overflows"));
    }

    #[test]
    fn backtest_rejects_non_finite_cost_assumptions() {
        let candles = (0..100)
            .map(|i| Candle {
                time: i as i64 + 1,
                open: 10.0,
                high: 11.0,
                low: 9.0,
                close: 10.0 + i as f64 * 0.01,
                volume: 1.0,
            })
            .collect::<Vec<_>>();
        let req = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1000.0,
            fee_bps: f64::NAN,
            slippage_bps: 5.0,
        };
        assert!(run("TEST", &candles, req, "fixture").is_err());
    }

    #[test]
    fn backtest_rejects_bad_ohlcv() {
        let mut candles = (0..100)
            .map(|i| Candle {
                time: i as i64 + 1,
                open: 10.0,
                high: 11.0,
                low: 9.0,
                close: 10.0 + i as f64 * 0.01,
                volume: 1.0,
            })
            .collect::<Vec<_>>();
        candles[25].high = 1.0;
        let req = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        assert!(run("TEST", &candles, req, "fixture").is_err());
    }

    #[test]
    fn execution_uses_next_bar_open_not_the_same_bar_close() {
        let closes = (0..180)
            .map(|i| {
                100.0
                    + (i as f64 * 0.19).sin() * 7.0
                    + (i as f64 * 0.045).cos() * 3.0
                    + i as f64 * 0.015
            })
            .collect::<Vec<_>>();
        let make_candles = |open_multiplier: f64| {
            closes
                .iter()
                .enumerate()
                .map(|(i, close)| {
                    let open = close * open_multiplier;
                    Candle {
                        time: 1_700_000_000_000 + i as i64 * 300_000,
                        open,
                        high: open.max(*close) + 2.0,
                        low: (open.min(*close) - 2.0).max(0.01),
                        close: *close,
                        volume: 100.0,
                    }
                })
                .collect::<Vec<_>>()
        };
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 18,
            initial_capital: 1_000.0,
            fee_bps: 0.0,
            slippage_bps: 0.0,
        };
        let baseline = run("TEST", &make_candles(1.0), request.clone(), "test data").unwrap();
        let gap_up = run("TEST", &make_candles(1.02), request, "test data").unwrap();
        assert!(
            baseline.trade_count > 0,
            "test series must trigger at least one trade"
        );
        assert!(
            (baseline.final_equity - gap_up.final_equity).abs() > 0.01,
            "changing opens while keeping all closes identical must affect fills"
        );
        assert!(baseline
            .warnings
            .iter()
            .any(|w| w.contains("next bar open")));
    }

    #[test]
    fn sharpe_annualization_tracks_observed_candle_interval() {
        let candles = (0..120)
            .map(|i| {
                let close = 100.0 + (i as f64 * 0.17).sin() * 5.0 + i as f64 * 0.08;
                Candle {
                    time: 1_700_000_000_000 + i as i64 * 3_600_000,
                    open: close - 0.1,
                    high: close + 1.0,
                    low: (close - 1.0).max(0.01),
                    close,
                    volume: 100.0,
                }
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let result = run("TEST", &candles, request, "test data").unwrap();
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("3600000 ms") && w.contains("periods/year")));
    }

    #[test]
    fn backtest_is_deterministic_for_same_inputs() {
        let candles = (0..150)
            .map(|i| {
                let close = 100.0 + (i as f64 * 0.17).sin() * 5.0 + i as f64 * 0.08;
                Candle {
                    time: i as i64 + 1,
                    open: close - 0.1,
                    high: close + 1.0,
                    low: close - 1.0,
                    close,
                    volume: 100.0,
                }
            })
            .collect::<Vec<_>>();
        let req = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let first = run("TEST", &candles, req.clone(), "fixture").unwrap();
        let second = run("TEST", &candles, req, "fixture").unwrap();
        assert_eq!(first.final_equity, second.final_equity);
        assert_eq!(first.trade_count, second.trade_count);
        assert_eq!(
            first
                .equity_curve
                .iter()
                .map(|p| p.equity)
                .collect::<Vec<_>>(),
            second
                .equity_curve
                .iter()
                .map(|p| p.equity)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn backtest_rejects_overflowing_prefix_sums() {
        let candles = (0..30)
            .map(|i| Candle {
                time: 1_700_000_000_000 + i as i64 * 60_000,
                open: f64::MAX / 2.0,
                high: f64::MAX,
                low: 1.0,
                close: f64::MAX / 2.0,
                volume: 1.0,
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 2,
            slow_period: 5,
            initial_capital: 1_000.0,
            fee_bps: 0.0,
            slippage_bps: 0.0,
        };
        assert!(run("TEST", &candles, request, "test").is_err());
    }

    #[test]
    fn backtest_rejects_non_finite_benchmark_quantity() {
        let tiny = f64::MIN_POSITIVE;
        let candles = (0..30)
            .map(|i| Candle {
                time: 1_700_000_000_000 + i as i64 * 60_000,
                open: tiny,
                high: tiny * 2.0,
                low: tiny / 2.0,
                close: tiny,
                volume: 1.0,
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 2,
            slow_period: 5,
            initial_capital: 1_000.0,
            fee_bps: 0.0,
            slippage_bps: 0.0,
        };
        assert!(run("TEST", &candles, request, "test").is_err());
    }

    #[test]
    fn benchmark_liquidation_costs_apply_when_strategy_is_flat() {
        let candles = (0..80)
            .map(|i| {
                let close = 100.0 + i as f64;
                Candle {
                    time: 1_700_000_000_000 + i as i64 * 60_000,
                    open: close,
                    high: close + 1.0,
                    low: close - 1.0,
                    close,
                    volume: 100.0,
                }
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 3,
            slow_period: 10,
            initial_capital: 1_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let result = run("TEST", &candles, request.clone(), "test data").unwrap();
        assert_eq!(
            result.trade_count, 0,
            "the monotonically rising sample has no observed cross-up"
        );
        assert_eq!(
            result.win_rate_pct, None,
            "no closed trades means no estimated win rate"
        );
        assert_eq!(
            result.profit_factor, None,
            "no gains/losses means profit factor is unavailable, not infinite"
        );
        assert_eq!(result.gross_profit, 0.0);
        assert_eq!(result.gross_loss, 0.0);
        let fee = request.fee_bps / 10_000.0;
        let slippage = request.slippage_bps / 10_000.0;
        let initial_buy_price = candles[0].close * (1.0 + slippage) * (1.0 + fee);
        let expected_liquidated_benchmark = request.initial_capital / initial_buy_price
            * candles.last().unwrap().close
            * (1.0 - slippage)
            * (1.0 - fee);
        let reported = result.equity_curve.last().unwrap().benchmark;
        assert!((reported - expected_liquidated_benchmark).abs() < 1e-9,
            "benchmark must include exit costs even when the strategy ends flat: {reported} vs {expected_liquidated_benchmark}");
    }

    #[test]
    fn crossover_strategy_requires_an_observed_cross_up() {
        let candles = (0..80)
            .map(|i| {
                let close = 100.0 + i as f64;
                Candle {
                    time: 1_700_000_000_000 + i as i64 * 60_000,
                    open: close,
                    high: close + 1.0,
                    low: close - 1.0,
                    close,
                    volume: 100.0,
                }
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 3,
            slow_period: 10,
            initial_capital: 1_000.0,
            fee_bps: 0.0,
            slippage_bps: 0.0,
        };
        let result = run("TEST", &candles, request, "test data").unwrap();
        assert_eq!(
            result.trade_count, 0,
            "the sample contains no observable fast-below-to-above crossover"
        );
        assert_eq!(result.final_equity, 1_000.0);
    }

    #[test]
    fn backtest_reports_cost_stress_scenarios_with_applied_costs_and_deltas() {
        let candles = (0..320)
            .map(|i| {
                let close = 100.0 + (i as f64 * 0.17).sin() * 5.0 + i as f64 * 0.08;
                Candle {
                    time: 1_700_000_000_000 + i as i64 * 60_000,
                    open: close - 0.1,
                    high: close + 1.0,
                    low: (close - 1.0).max(0.01),
                    close,
                    volume: 100.0,
                }
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let result = run("TEST", &candles, request, "test data").unwrap();
        assert_eq!(result.cost_stress_scenarios.len(), 2);
        assert_eq!(result.cost_stress_scenarios[0].name, "Moderate cost shock");
        assert_eq!(result.cost_stress_scenarios[1].name, "Severe cost shock");
        assert!(result.cost_stress_scenarios[0].fee_bps > result.fee_bps);
        assert!(result.cost_stress_scenarios[0].slippage_bps > result.slippage_bps);
        assert!(result.cost_stress_scenarios[1].fee_bps >= result.cost_stress_scenarios[0].fee_bps);
        assert!(
            result.cost_stress_scenarios[1].slippage_bps
                >= result.cost_stress_scenarios[0].slippage_bps
        );
        for scenario in &result.cost_stress_scenarios {
            assert!(scenario.final_equity.is_finite());
            assert!(scenario.total_return_pct.is_finite());
            assert!(scenario.return_delta_percentage_points.is_finite());
            assert!(scenario.max_drawdown_pct.is_finite());
            assert!(scenario.sharpe_ratio.is_finite());
        }
    }

    #[test]
    fn backtest_builds_fixed_chronological_out_of_sample_windows() {
        let candles = (0..320)
            .map(|i| {
                let close = 100.0 + (i as f64 * 0.17).sin() * 5.0 + i as f64 * 0.08;
                Candle {
                    time: 1_700_000_000_000 + i as i64 * 60_000,
                    open: close - 0.1,
                    high: close + 1.0,
                    low: (close - 1.0).max(0.01),
                    close,
                    volume: 100.0,
                }
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let result = run("TEST", &candles, request, "test data").unwrap();
        assert!(result.out_of_sample_folds.len() >= 2);
        assert_eq!(result.out_of_sample_folds[0].start_ms, candles[22].time);
        for fold in &result.out_of_sample_folds {
            assert_eq!(fold.candles_evaluated, 40);
            assert!(fold.start_ms <= fold.end_ms);
            assert!(fold.final_equity.is_finite());
            assert!(fold.total_return_pct.is_finite());
            assert!(fold.max_drawdown_pct.is_finite());
            assert!(fold.sharpe_ratio.is_finite());
        }
        assert!(result
            .out_of_sample_folds
            .windows(2)
            .all(|pair| pair[0].end_ms < pair[1].start_ms));
    }

    #[test]
    fn earlier_out_of_sample_fold_is_invariant_to_future_candle_values() {
        let make_candles = || {
            (0..320)
                .map(|i| {
                    let close = 100.0 + (i as f64 * 0.17).sin() * 5.0 + i as f64 * 0.08;
                    Candle {
                        time: 1_700_000_000_000 + i as i64 * 60_000,
                        open: close - 0.1,
                        high: close + 1.0,
                        low: (close - 1.0).max(0.01),
                        close,
                        volume: 100.0,
                    }
                })
                .collect::<Vec<_>>()
        };
        let mut original = make_candles();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let first = run("TEST", &original, request.clone(), "test data").unwrap();
        let first_fold = first.out_of_sample_folds.first().unwrap();
        let first_fold_end_index = original
            .iter()
            .position(|candle| candle.time == first_fold.end_ms)
            .unwrap();
        for (index, candle) in original
            .iter_mut()
            .enumerate()
            .skip(first_fold_end_index + 1)
        {
            let close = 250.0 + (index as f64 * 0.11).sin() * 4.0;
            candle.open = close - 0.1;
            candle.close = close;
            candle.high = close + 1.0;
            candle.low = close - 1.0;
        }
        let changed = run("TEST", &original, request, "test data").unwrap();
        assert_eq!(first_fold.start_ms, changed.out_of_sample_folds[0].start_ms);
        assert_eq!(first_fold.end_ms, changed.out_of_sample_folds[0].end_ms);
        assert_eq!(
            first_fold.final_equity,
            changed.out_of_sample_folds[0].final_equity
        );
        assert_eq!(
            first_fold.total_return_pct,
            changed.out_of_sample_folds[0].total_return_pct
        );
        assert_eq!(
            first_fold.trade_count,
            changed.out_of_sample_folds[0].trade_count
        );
    }

    #[test]
    fn out_of_sample_validation_fails_closed_when_history_is_too_short() {
        let candles = (0..100)
            .map(|i| {
                let close = 100.0 + (i as f64 * 0.1).sin();
                Candle {
                    time: 1_700_000_000_000 + i as i64 * 60_000,
                    open: close,
                    high: close + 0.5,
                    low: close - 0.5,
                    close,
                    volume: 10.0,
                }
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let result = run("TEST", &candles, request, "test data").unwrap();
        assert!(result.out_of_sample_folds.is_empty());
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("not evidence of out-of-sample robustness")));
    }

    #[test]
    fn canonical_dataset_sha256_matches_known_vector() {
        let candles = vec![Candle {
            time: 1,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 3.0,
        }];
        assert_eq!(
            dataset_sha256(&candles),
            "f2372778376160ebe9f4759a6a81dc8e0b76020e21dcc8dafe48d1f76134dec6"
        );
    }

    #[test]
    fn dataset_sha256_is_deterministic_and_changes_for_any_candle_field() {
        let candle = Candle {
            time: 10,
            open: 10.0,
            high: 12.0,
            low: 9.0,
            close: 11.0,
            volume: 42.0,
        };
        let baseline = dataset_sha256(std::slice::from_ref(&candle));
        assert_eq!(baseline, dataset_sha256(std::slice::from_ref(&candle)));
        let changed_time = Candle {
            time: 11,
            ..candle.clone()
        };
        let changed_open = Candle {
            open: 10.5,
            ..candle.clone()
        };
        let changed_high = Candle {
            high: 12.5,
            ..candle.clone()
        };
        let changed_low = Candle {
            low: 8.5,
            ..candle.clone()
        };
        let changed_close = Candle {
            close: 11.5,
            ..candle.clone()
        };
        let changed_volume = Candle {
            volume: 43.0,
            ..candle.clone()
        };
        for changed in [
            changed_time,
            changed_open,
            changed_high,
            changed_low,
            changed_close,
            changed_volume,
        ] {
            assert_ne!(baseline, dataset_sha256(&[changed]));
        }
    }

    #[test]
    fn backtest_result_exposes_input_range_and_fingerprint_metadata() {
        let candles = (0..100)
            .map(|i| {
                let close = 100.0 + i as f64 * 0.1;
                Candle {
                    time: 1_700_000_000_000 + i as i64 * 300_000,
                    open: close,
                    high: close + 1.0,
                    low: close - 1.0,
                    close,
                    volume: 5.0,
                }
            })
            .collect::<Vec<_>>();
        let request = BacktestRequest {
            symbol: "TEST".into(),
            fast_period: 5,
            slow_period: 20,
            initial_capital: 1_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let result = run("TEST", &candles, request, "fixture").unwrap();
        assert_eq!(result.dataset_encoding_version, 1);
        assert_eq!(result.dataset_sha256, dataset_sha256(&candles));
        assert_eq!(result.dataset_start_ms, candles.first().unwrap().time);
        assert_eq!(result.dataset_end_ms, candles.last().unwrap().time);
        assert_eq!(result.median_candle_interval_ms, 300_000);
    }

    fn tournament_candles(length: usize) -> Vec<Candle> {
        (0..length)
            .map(|index| {
                let close = 100.0 + index as f64 * 0.008 + (index as f64 * 0.31).sin() * 4.0;
                Candle {
                    time: 1_700_000_000_000 + index as i64 * 300_000,
                    open: close,
                    high: close + 0.5,
                    low: close - 0.5,
                    close,
                    volume: 100.0 + (index % 17) as f64,
                }
            })
            .collect()
    }

    #[test]
    fn nested_walk_forward_tournament_is_deterministic_and_bounded() {
        let candles = tournament_candles(600);
        let request = BacktestRequest {
            symbol: "BTCUSDT".into(),
            fast_period: 5,
            slow_period: 12,
            initial_capital: 10_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let first = run_walk_forward_parameter_tournament(
            "BTCUSDT",
            &candles,
            &request,
            "unit-market-source",
            3,
        )
        .unwrap();
        let second = run_walk_forward_parameter_tournament(
            "BTCUSDT",
            &candles,
            &request,
            "unit-market-source",
            3,
        )
        .unwrap();
        assert_eq!(first.candidate_count, second.candidate_count);
        assert!(first.candidate_count <= TOURNAMENT_MAX_CANDIDATES);
        assert_eq!(first.completed_folds, 3);
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert!(first
            .folds
            .iter()
            .all(|fold| fold.candidate_scores.len() == first.candidate_count));
    }

    #[test]
    fn nested_walk_forward_selection_does_not_peek_into_its_first_test_block() {
        let candles = tournament_candles(600);
        let mut future_changed = candles.clone();
        for (index, candle) in future_changed.iter_mut().enumerate().skip(240) {
            let adjustment = if index % 40 < 20 { 1.8 } else { -1.8 };
            candle.open += adjustment;
            candle.high += adjustment;
            candle.low += adjustment;
            candle.close += adjustment;
        }
        let request = BacktestRequest {
            symbol: "BTCUSDT".into(),
            fast_period: 5,
            slow_period: 12,
            initial_capital: 10_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        let original = run_walk_forward_parameter_tournament(
            "BTCUSDT",
            &candles,
            &request,
            "unit-market-source",
            3,
        )
        .unwrap();
        let altered = run_walk_forward_parameter_tournament(
            "BTCUSDT",
            &future_changed,
            &request,
            "unit-market-source",
            3,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&original.folds[0].candidate_scores).unwrap(),
            serde_json::to_value(&altered.folds[0].candidate_scores).unwrap(),
        );
        assert_eq!(
            original.folds[0].selected_fast_period,
            altered.folds[0].selected_fast_period
        );
        assert_eq!(
            original.folds[0].selected_slow_period,
            altered.folds[0].selected_slow_period
        );
    }

    #[test]
    fn tournament_priority_uses_one_exact_order_for_selection_and_verification() {
        let slightly_better_sharpe =
            compare_tournament_candidate_priority((1.0 + 1e-13, 9.0, 10, 30), (1.0, 1.0, 8, 24));
        assert!(
            slightly_better_sharpe.is_lt(),
            "a real Sharpe difference must not be swallowed by a tolerance"
        );
        let lower_drawdown_tie =
            compare_tournament_candidate_priority((1.0, 4.0, 10, 30), (1.0, 5.0, 8, 24));
        assert!(lower_drawdown_tie.is_lt());
        let lexicographic_tie =
            compare_tournament_candidate_priority((1.0, 4.0, 8, 24), (1.0, 4.0, 10, 30));
        assert!(lexicographic_tie.is_lt());
    }

    #[test]
    fn nested_walk_forward_rejects_invalid_fold_policy() {
        let candles = tournament_candles(600);
        let request = BacktestRequest {
            symbol: "BTCUSDT".into(),
            fast_period: 5,
            slow_period: 12,
            initial_capital: 10_000.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        assert!(run_walk_forward_parameter_tournament(
            "BTCUSDT",
            &candles,
            &request,
            "unit-market-source",
            1
        )
        .is_err());
        assert!(run_walk_forward_parameter_tournament(
            "BTCUSDT",
            &candles,
            &request,
            "unit-market-source",
            6
        )
        .is_err());
    }
}
