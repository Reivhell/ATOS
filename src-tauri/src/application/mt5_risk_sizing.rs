//! Broker-native, stop-loss-based MT5 position sizing.
//!
//! This is a calculation and pre-trade control boundary, not an execution API. It uses the
//! terminal's current account equity, quote, symbol contract and loss tick value. Unknown
//! margin requirements and commissions are deliberately not guessed. Pending-order volume
//! is reconciled into broker directional volume-limit checks from the authenticated snapshot.
use crate::infrastructure::mt5::{
    bridge::{
        account_risk_inventory, Mt5AccountRiskInventory, Mt5Bridge, Mt5BridgeStatus,
        Mt5MarginPreflightResult, Mt5PendingOrderSnapshot, Mt5PositionSnapshot, Mt5Snapshot,
    },
    market_data::Mt5InstrumentSpecificationRecord,
};
use serde::{Deserialize, Serialize};

const MAX_QUOTE_AGE_MS: i64 = 60_000;
const MAX_HEARTBEAT_AGE_MS: i64 = 5_000;
const MAX_SPECIFICATION_AGE_MS: i64 = 60_000;
const MAX_RISK_PERCENT: f64 = 5.0;
const MAX_ACCOUNT_RISK_PERCENT: f64 = 5.0;
const MIN_MARGIN_LEVEL_PCT: f64 = 300.0;
const MIN_POST_TRADE_FREE_MARGIN_PERCENT_EQUITY: f64 = 20.0;
const MAX_ACCOUNT_STATE_DRIFT_PERCENT: f64 = 0.5;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5RiskSizingRequest {
    pub broker_server: String,
    pub account_login: String,
    pub symbol: String,
    pub side: String,
    pub stop_loss_price: f64,
    pub risk_percent: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5RiskSizingResult {
    pub decision: String,
    pub reasons: Vec<String>,
    pub limitations: Vec<String>,
    pub broker_server: String,
    pub account_login: String,
    pub symbol: String,
    pub side: String,
    pub account_currency: String,
    pub equity: f64,
    pub risk_percent: f64,
    pub risk_budget: f64,
    pub effective_risk_budget: f64,
    pub account_available_risk_budget: f64,
    pub entry_price: f64,
    pub stop_loss_price: f64,
    pub stop_distance: f64,
    pub tick_size: f64,
    pub tick_value_loss_per_lot: f64,
    pub estimated_loss_per_lot: f64,
    pub raw_volume: f64,
    pub proposed_volume: f64,
    pub normalized_volume: f64,
    pub estimated_loss: f64,
    pub estimated_risk_percent: f64,
    pub volume_min: f64,
    pub volume_max: f64,
    pub volume_step: f64,
    pub existing_symbol_volume: f64,
    pub pending_symbol_volume: f64,
    pub total_directional_symbol_volume: f64,
    pub volume_limit: f64,
    pub account_stop_risk: f64,
    pub account_stop_risk_percent: f64,
    pub unprotected_position_count: usize,
    pub unprotected_pending_order_count: usize,
    pub unresolved_position_risk_count: usize,
    pub unresolved_pending_order_risk_count: usize,
    pub max_account_risk_percent: f64,
    pub max_account_risk_budget: f64,
    pub projected_account_stop_risk: f64,
    pub candidate_margin: Option<f64>,
    pub current_margin: Option<f64>,
    pub projected_margin: Option<f64>,
    pub projected_free_margin: Option<f64>,
    pub projected_margin_level_pct: Option<f64>,
    pub broker_margin_call_level: Option<f64>,
    pub broker_stop_out_level: Option<f64>,
    pub broker_stopout_mode: Option<i64>,
    pub margin_preflight_at_ms: Option<i64>,
    pub margin_estimation: String,
    pub commission_estimation: String,
    pub quote_age_ms: i64,
    pub specification_revision: u64,
    pub specification_fingerprint_sha256: String,
    pub contract_completeness: String,
    pub calculated_at_ms: i64,
    pub snapshot_sequence: u64,
}

struct CorrelatedRiskInventory {
    snapshot: Mt5Snapshot,
    account_risk: Mt5AccountRiskInventory,
}

pub fn calculate(
    bridge: &Mt5Bridge,
    request: Mt5RiskSizingRequest,
) -> Result<Mt5RiskSizingResult, String> {
    let now = chrono::Utc::now().timestamp_millis();
    let broker_server = request.broker_server.trim();
    let account_login = request.account_login.trim();
    let symbol = request.symbol.trim();
    let side = request.side.trim().to_ascii_uppercase();
    if broker_server.is_empty()
        || broker_server.len() > 128
        || account_login.is_empty()
        || account_login.len() > 32
    {
        return Err("A valid broker server and account identity are required.".into());
    }
    if symbol.is_empty()
        || symbol.len() > 64
        || !symbol
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
    {
        return Err("A valid broker-native symbol is required.".into());
    }
    if !matches!(side.as_str(), "BUY" | "SELL") {
        return Err("Side must be BUY or SELL.".into());
    }
    if !request.risk_percent.is_finite()
        || !(0.01..=MAX_RISK_PERCENT).contains(&request.risk_percent)
    {
        return Err(format!(
            "Risk percentage must be between 0.01 and {MAX_RISK_PERCENT:.2}% of current equity."
        ));
    }
    if !request.stop_loss_price.is_finite() || request.stop_loss_price <= 0.0 {
        return Err("Stop-loss price must be finite and positive.".into());
    }

    let status: Mt5BridgeStatus = bridge.status();
    let mut snapshot = status
        .snapshot
        .ok_or_else(|| "No authenticated MT5 terminal snapshot is available.".to_string())?;
    if snapshot.account.server != broker_server || snapshot.account.login != account_login {
        return Err(
            "Risk sizing identity does not match the currently authenticated MT5 account.".into(),
        );
    }
    if !status.connected || status.stale || status.quote_stale || !snapshot.terminal.connected {
        return Err(
            "MT5 bridge, terminal, or quote is stale/disconnected; risk sizing is blocked.".into(),
        );
    }
    let heartbeat_age = status.heartbeat_age_ms.unwrap_or(i64::MAX);
    if !(0..=MAX_HEARTBEAT_AGE_MS).contains(&heartbeat_age) {
        return Err("MT5 heartbeat is outside the risk-sizing freshness window.".into());
    }
    quote_age_within_window(now, snapshot.symbol.quote_time_ms).ok_or_else(|| {
            "MT5 quote timestamp is future-dated or older than 60 seconds.".to_string()
        })?;
    if snapshot.symbol.name != symbol {
        return Err("Requested symbol does not match the live terminal quote symbol. Select that symbol in MT5 and wait for a fresh bridge frame.".into());
    }
    if !snapshot.account.equity.is_finite() || snapshot.account.equity <= 0.0 {
        return Err("Current MT5 account equity is invalid; sizing is blocked.".into());
    }
    if !snapshot.symbol.bid.is_finite()
        || !snapshot.symbol.ask.is_finite()
        || snapshot.symbol.bid <= 0.0
        || snapshot.symbol.ask < snapshot.symbol.bid
    {
        return Err("Current bid/ask quote is invalid; sizing is blocked.".into());
    }

    let specification: Mt5InstrumentSpecificationRecord =
        bridge.instrument_specification(broker_server, account_login, symbol)?;
    let specification_age = now.saturating_sub(specification.last_observed_at_ms);
    if !(0..=MAX_SPECIFICATION_AGE_MS).contains(&specification_age) {
        return Err(
            "Broker instrument specification is future-dated or older than 60 seconds.".into(),
        );
    }
    let contract = specification.specification;
    if contract.symbol != symbol {
        return Err("Stored broker contract symbol does not match the live quote.".into());
    }
    if !nearly_equal(snapshot.symbol.point, contract.point)
        || !nearly_equal(snapshot.symbol.tick_size, contract.tick_size)
        || !nearly_equal(snapshot.symbol.tick_value_loss, contract.tick_value_loss)
        || !nearly_equal(snapshot.symbol.volume_min, contract.volume_min)
        || !nearly_equal(snapshot.symbol.volume_max, contract.volume_max)
        || !nearly_equal(snapshot.symbol.volume_step, contract.volume_step)
        || snapshot.symbol.stops_level_points != contract.stops_level_points
    {
        return Err("Live quote snapshot and durable broker contract projection disagree; refresh the MT5 frame before sizing.".into());
    }
    let tick_size = contract.tick_size;
    let tick_value_loss = contract.tick_value_loss;
    let volume_min = contract.volume_min;
    let volume_max = contract.volume_max;
    let volume_step = contract.volume_step;
    if !tick_size.is_finite()
        || tick_size <= 0.0
        || !tick_value_loss.is_finite()
        || tick_value_loss <= 0.0
        || !volume_min.is_finite()
        || volume_min <= 0.0
        || !volume_max.is_finite()
        || volume_max < volume_min
        || !volume_step.is_finite()
        || volume_step <= 0.0
    {
        return Err("Broker contract lacks valid tick-loss or volume metadata. No fallback contract assumptions are permitted.".into());
    }
    if !contract.point.is_finite() || contract.point <= 0.0 || contract.stops_level_points < 0 {
        return Err("Broker stop-distance metadata is invalid.".into());
    }
    let mut entry = if side == "BUY" {
        snapshot.symbol.ask
    } else {
        snapshot.symbol.bid
    };
    let mut stop_distance = if side == "BUY" {
        entry - request.stop_loss_price
    } else {
        request.stop_loss_price - entry
    };
    if !stop_distance.is_finite() || stop_distance <= 0.0 {
        return Err("Stop loss is on the wrong side of the executable bid/ask entry price.".into());
    }
    let broker_stop_distance = if side == "BUY" {
        snapshot.symbol.bid - request.stop_loss_price
    } else {
        request.stop_loss_price - snapshot.symbol.ask
    };
    let minimum_stop_distance = contract.stops_level_points as f64 * contract.point;
    if !broker_stop_distance.is_finite()
        || broker_stop_distance <= 0.0
        || broker_stop_distance < minimum_stop_distance
    {
        return Err(format!("Stop-loss placement is on the wrong side of the broker trigger quote or closer than the minimum stop distance {minimum_stop_distance:.10}."));
    }
    let grid_ratio = request.stop_loss_price / tick_size;
    if !grid_ratio.is_finite() || (grid_ratio - grid_ratio.round()).abs() > 1e-7 {
        return Err("Stop-loss price is not aligned to the broker tick-size grid.".into());
    }

    let mut account_risk: Mt5AccountRiskInventory = account_risk_inventory(&snapshot)?;
    let mut max_account_risk_budget = snapshot.account.equity * MAX_ACCOUNT_RISK_PERCENT / 100.0;
    let mut risk_budget = snapshot.account.equity * request.risk_percent / 100.0;
    let mut effective_risk_budget = cap_trade_risk_budget(
        risk_budget,
        max_account_risk_budget,
        account_risk.current_stop_risk,
    )?;
    let mut account_available_risk_budget =
        (max_account_risk_budget - account_risk.current_stop_risk).max(0.0);
    let loss_per_lot = estimate_loss_per_lot(stop_distance, tick_size, tick_value_loss);
    if !risk_budget.is_finite()
        || risk_budget <= 0.0
        || !account_available_risk_budget.is_finite()
        || !effective_risk_budget.is_finite()
        || !loss_per_lot.is_finite()
        || loss_per_lot <= 0.0
    {
        return Err("Trade risk budget, account-wide risk capacity, or broker-native loss estimate is invalid.".into());
    }
    let raw_volume = if effective_risk_budget > 0.0 {
        effective_risk_budget / loss_per_lot
    } else {
        0.0
    };
    if !raw_volume.is_finite() {
        return Err("Raw position volume overflowed; sizing is blocked.".into());
    }
    let mut normalized_volume = normalize_volume_down(raw_volume, volume_step, volume_max);
    let mut estimated_loss = normalized_volume * loss_per_lot;
    // MQL5 SYMBOL_VOLUME_LIMIT is directional: BUY positions consume the BUY limit and
    // SELL positions consume the SELL limit. MQL5 position type 0=BUY, 1=SELL.
    let expected_position_type = if side == "BUY" { 0 } else { 1 };
    let mut existing_symbol_volume =
        same_direction_volume(&snapshot.positions, symbol, expected_position_type)?;
    let mut pending_symbol_volume =
        same_direction_pending_volume(&snapshot.pending_orders, symbol, side == "BUY")?;
    let mut total_directional_symbol_volume = checked_volume_total(
        [existing_symbol_volume, pending_symbol_volume],
        "combined directional inventory",
    )?;
    let volume_limit = contract.volume_limit;
    if !volume_limit.is_finite() || volume_limit < 0.0 {
        return Err("Broker volume-limit metadata is invalid.".into());
    }
    let mut reasons = Vec::new();
    if let Some(reason) = market_trade_permission_block_reason(
        snapshot.account.account_trade_allowed,
        snapshot.symbol.trade_mode,
        snapshot.symbol.order_mode,
        &side,
    ) {
        reasons.push(reason.into());
    }
    if account_risk.unprotected_position_count > 0
        || account_risk.unprotected_pending_order_count > 0
    {
        reasons.push(format!("Account-wide exposure contains {} unprotected positions and {} unprotected pending orders without a broker-calculated stop-loss loss; new exposure is blocked.", account_risk.unprotected_position_count, account_risk.unprotected_pending_order_count));
    }
    if account_risk.unresolved_position_risk_count > 0
        || account_risk.unresolved_pending_order_risk_count > 0
    {
        reasons.push(format!("Account-wide stop-loss risk is incomplete for {} positions and {} pending orders; new exposure is blocked.", account_risk.unresolved_position_risk_count, account_risk.unresolved_pending_order_risk_count));
    }
    if account_risk.current_stop_risk > max_account_risk_budget {
        reasons.push(
            "Existing portfolio stop-loss risk already exceeds the account-wide risk budget."
                .into(),
        );
    }
    if account_available_risk_budget <= 0.0 {
        reasons.push(
            "No account-wide stop-loss risk capacity remains under the ATOS exposure ceiling."
                .into(),
        );
    }
    if normalized_volume < volume_min {
        reasons.push(format!("Available risk budget supports {normalized_volume:.8} lots, below broker minimum {volume_min:.8}; minimum lot would exceed the available risk capacity."));
    }
    if volume_limit > 0.0
        && (!volume_limit.is_finite()
            || total_directional_symbol_volume + normalized_volume > volume_limit)
    {
        reasons.push("Proposed volume plus existing same-direction position exceeds the broker volume limit.".into());
    }
    if !normalized_volume.is_finite()
        || normalized_volume < 0.0
        || !estimated_loss.is_finite()
        || estimated_loss > effective_risk_budget * (1.0 + 1e-9)
    {
        reasons.push("Normalized volume does not satisfy the remaining per-trade and account-wide risk budgets.".into());
    }
    if !snapshot.account.currency.trim().is_empty() && contract.currency_profit.trim().is_empty() {
        reasons.push("Broker profit-currency metadata is missing; account-currency conversion cannot be independently verified.".into());
    }

    let mut native_margin: Option<Mt5MarginPreflightResult> = None;
    let mut calculation_snapshot_sequence = status.last_sequence;
    if reasons.is_empty() && normalized_volume >= volume_min {
        match bridge.margin_preflight(
            broker_server,
            account_login,
            symbol,
            &side,
            normalized_volume,
            request.stop_loss_price,
        ) {
            Ok(result) => {
                match correlated_preflight_inventory(&bridge, result, broker_server, account_login, symbol) {
                    Err(error) => reasons.push(format!("Native calculation could not be bound to a complete account inventory: {error}")),
                    Ok(correlated) => {
                        snapshot = correlated.snapshot;
                        account_risk = correlated.account_risk;
                        risk_budget = snapshot.account.equity * request.risk_percent / 100.0;
                        max_account_risk_budget = snapshot.account.equity * MAX_ACCOUNT_RISK_PERCENT / 100.0;
                        effective_risk_budget = cap_trade_risk_budget(risk_budget, max_account_risk_budget, account_risk.current_stop_risk)?;
                        account_available_risk_budget = (max_account_risk_budget - account_risk.current_stop_risk).max(0.0);
                        calculation_snapshot_sequence = result.snapshot_sequence;
                        let preflight_checked_at_ms = chrono::Utc::now().timestamp_millis();
                        quote_age = preflight_checked_at_ms.saturating_sub(snapshot.symbol.quote_time_ms);
                        entry = if side == "BUY" { snapshot.symbol.ask } else { snapshot.symbol.bid };
                        stop_distance = if side == "BUY" { entry - request.stop_loss_price } else { request.stop_loss_price - entry };
                        if !(0..=MAX_QUOTE_AGE_MS).contains(&quote_age) || !stop_distance.is_finite() || stop_distance <= 0.0 {
                            reasons.push("The correlated account frame no longer contains a fresh quote with the requested stop on the loss side; refresh and retry.".into());
                        }
                        let native_loss = result.loss_at_stop_loss.unwrap_or(f64::NAN);
                        if !native_loss.is_finite() || native_loss <= 0.0 {
                            reasons.push("MT5 native OrderCalcProfit did not produce a valid candidate loss in account currency.".into());
                        } else if !snapshot_contract_matches(&snapshot, &contract) {
                            reasons.push("Broker instrument specification changed between sizing and native preflight; refresh the contract and retry.".into());
                        } else if !risk_budget.is_finite() || !effective_risk_budget.is_finite() || !account_available_risk_budget.is_finite() {
                            reasons.push("The correlated account risk budget is non-finite; approval is blocked.".into());
                        } else if native_loss > effective_risk_budget * (1.0 + 1e-9) {
                            // Reconcile terminal-native loss against risk headroom calculated
                            // from the exact inventory frame that carried this native result.
                            let corrected = normalize_volume_down(normalized_volume * effective_risk_budget / native_loss, volume_step, volume_max);
                            if corrected < volume_min {
                                reasons.push("Broker-native OrderCalcProfit shows that even the minimum lot exceeds the latest account-wide/per-trade risk capacity.".into());
                            } else {
                                normalized_volume = corrected;
                                match bridge.margin_preflight(broker_server, account_login, symbol, &side, normalized_volume, request.stop_loss_price) {
                                    Ok(rechecked) => {
                                        match correlated_preflight_inventory(&bridge, rechecked, broker_server, account_login, symbol) {
                                            Err(error) => reasons.push(format!("Corrected-volume calculation could not be bound to a complete account inventory: {error}")),
                                            Ok(correlated_again) => {
                                                snapshot = correlated_again.snapshot;
                                                account_risk = correlated_again.account_risk;
                                                risk_budget = snapshot.account.equity * request.risk_percent / 100.0;
                                                max_account_risk_budget = snapshot.account.equity * MAX_ACCOUNT_RISK_PERCENT / 100.0;
                                                effective_risk_budget = cap_trade_risk_budget(risk_budget, max_account_risk_budget, account_risk.current_stop_risk)?;
                                                account_available_risk_budget = (max_account_risk_budget - account_risk.current_stop_risk).max(0.0);
                                                calculation_snapshot_sequence = rechecked.snapshot_sequence;
                                                let corrected_checked_at_ms = chrono::Utc::now().timestamp_millis();
                                                quote_age = corrected_checked_at_ms.saturating_sub(snapshot.symbol.quote_time_ms);
                                                entry = if side == "BUY" { snapshot.symbol.ask } else { snapshot.symbol.bid };
                                                stop_distance = if side == "BUY" { entry - request.stop_loss_price } else { request.stop_loss_price - entry };
                                                if !(0..=MAX_QUOTE_AGE_MS).contains(&quote_age) || !stop_distance.is_finite() || stop_distance <= 0.0 {
                                                    reasons.push("The corrected-volume snapshot no longer contains a fresh quote with the requested stop on the loss side; approval is blocked.".into());
                                                }
                                                let corrected_loss = rechecked.loss_at_stop_loss.unwrap_or(f64::NAN);
                                                if !snapshot_contract_matches(&snapshot, &contract) {
                                                    reasons.push("Broker instrument specification changed during corrected-volume preflight; refresh and retry.".into());
                                                }
                                                if !corrected_loss.is_finite() || corrected_loss <= 0.0 || corrected_loss > effective_risk_budget * (1.0 + 1e-9) {
                                                    reasons.push("The corrected volume still fails the broker-native stop-loss budget against the latest complete account inventory.".into());
                                                }
                                                estimated_loss = corrected_loss;
                                                native_margin = Some(rechecked);
                                            }
                                        }
                                    }
                                    Err(error) => reasons.push(format!("Corrected-volume native margin preflight failed closed: {error}")),
                                }
                            }
                        } else {
                            estimated_loss = native_loss;
                            native_margin = Some(result);
                        }
                    }
                }
            }
            Err(error) => reasons.push(format!(
                "Native MT5 margin preflight failed closed: {error}"
            )),
        }
    }

    // The terminal may change symbol permissions or directional exposure while the
    // native calculation is in flight. Re-evaluate those limits from the correlated frame.
    existing_symbol_volume =
        same_direction_volume(&snapshot.positions, symbol, expected_position_type)?;
    pending_symbol_volume =
        same_direction_pending_volume(&snapshot.pending_orders, symbol, side == "BUY")?;
    total_directional_symbol_volume = checked_volume_total(
        [existing_symbol_volume, pending_symbol_volume],
        "latest combined directional inventory",
    )?;
    if let Some(reason) = market_trade_permission_block_reason(
        snapshot.account.account_trade_allowed,
        snapshot.symbol.trade_mode,
        snapshot.symbol.order_mode,
        &side,
    ) {
        if !reasons.iter().any(|existing| existing == reason) {
            reasons.push(reason.into());
        }
    }
    if volume_limit > 0.0 && total_directional_symbol_volume + normalized_volume > volume_limit {
        reasons.push("Latest account inventory plus proposed volume exceeds the broker directional volume limit.".into());
    }

    let proposed_volume = normalized_volume;
    let mut candidate_margin = None;
    let mut current_margin = None;
    let mut projected_margin = None;
    let mut projected_free_margin = None;
    let mut projected_margin_level_pct = None;
    let mut broker_margin_call_level = None;
    let mut broker_stop_out_level = None;
    let mut broker_stopout_mode = None;
    let mut margin_preflight_at_ms = None;
    if let Some(preflight) = native_margin.as_ref() {
        let candidate = preflight.calc_margin.unwrap_or(f64::NAN);
        let account_denominator = snapshot.account.equity.max(1.0);
        let equity_drift = (preflight.account_equity - snapshot.account.equity).abs()
            / account_denominator
            * 100.0;
        let margin_drift = (preflight.account_margin - snapshot.account.margin).abs()
            / account_denominator
            * 100.0;
        let free_margin_drift = (preflight.account_free_margin - snapshot.account.free_margin)
            .abs()
            / account_denominator
            * 100.0;
        let account_state_drift = equity_drift.max(margin_drift).max(free_margin_drift);
        let native_stop_distance = if side == "BUY" {
            preflight.bid - request.stop_loss_price
        } else {
            request.stop_loss_price - preflight.ask
        };
        let minimum_native_stop_distance = contract.stops_level_points as f64 * contract.point;
        if !candidate.is_finite()
            || candidate < 0.0
            || account_state_drift > MAX_ACCOUNT_STATE_DRIFT_PERCENT
            || !native_stop_distance.is_finite()
            || native_stop_distance < minimum_native_stop_distance
            || native_stop_distance <= 0.0
            || preflight.margin_stopout_mode != snapshot.account.margin_stopout_mode
            || (preflight.margin_call_level - snapshot.account.margin_call_level).abs() > 1e-8
            || (preflight.stop_out_level - snapshot.account.stop_out_level).abs() > 1e-8
        {
            reasons.push("Account equity, used/free margin, stop distance, or broker stop-out policy changed between the telemetry snapshot and native terminal calculation; refresh and retry.".into());
        } else {
            let margin_after = preflight.account_margin + candidate;
            let free_after = preflight.account_free_margin - candidate;
            let current_and_candidate_stop_risk = account_risk.current_stop_risk + estimated_loss;
            let stressed_equity = preflight.account_equity - current_and_candidate_stop_risk;
            if !margin_after.is_finite()
                || !free_after.is_finite()
                || !stressed_equity.is_finite()
                || !current_and_candidate_stop_risk.is_finite()
            {
                reasons.push("Projected margin or stressed-equity arithmetic overflowed; the native preflight result cannot be represented safely.".into());
            } else {
                candidate_margin = Some(candidate);
                current_margin = Some(preflight.account_margin);
                projected_margin = Some(margin_after);
                projected_free_margin = Some(free_after);
                broker_margin_call_level = Some(preflight.margin_call_level);
                broker_stop_out_level = Some(preflight.stop_out_level);
                broker_stopout_mode = Some(preflight.margin_stopout_mode);
                margin_preflight_at_ms = Some(preflight.calculated_at_ms);
                if candidate > preflight.account_free_margin {
                    reasons.push("Native MT5 candidate margin exceeds current free margin.".into());
                }
                let min_free_margin =
                    preflight.account_equity * MIN_POST_TRADE_FREE_MARGIN_PERCENT_EQUITY / 100.0;
                if free_after < min_free_margin {
                    reasons.push(format!("Projected free margin {free_after:.2} {0} is below the ATOS reserve floor of {min_free_margin:.2} ({1:.0}% of equity).", snapshot.account.currency, MIN_POST_TRADE_FREE_MARGIN_PERCENT_EQUITY));
                }
                if preflight.margin_stopout_mode == 0 {
                    let required_margin_level = MIN_MARGIN_LEVEL_PCT
                        .max(preflight.margin_call_level * 2.0)
                        .max(preflight.stop_out_level * 3.0);
                    if margin_after > 0.0 {
                        let margin_level_after = stressed_equity / margin_after * 100.0;
                        if !margin_level_after.is_finite() {
                            reasons.push("Stressed projected margin-level arithmetic overflowed; risk approval is blocked.".into());
                        } else {
                            projected_margin_level_pct = Some(margin_level_after);
                            if margin_level_after < required_margin_level {
                                reasons.push(format!("Stressed projected margin level {margin_level_after:.2}% is below the ATOS safety floor of {required_margin_level:.2}% (broker call/stop-out thresholds: {:.2}% / {:.2}%).", preflight.margin_call_level, preflight.stop_out_level));
                            }
                        }
                    } else if stressed_equity <= 0.0 {
                        reasons.push("Stressed account equity is non-positive even though the projected trade requires no additional margin.".into());
                    }
                } else if preflight.margin_stopout_mode == 1 {
                    let money_floor = (preflight.margin_call_level * 2.0)
                        .max(preflight.stop_out_level * 3.0)
                        .max(preflight.account_equity * 0.10);
                    if stressed_equity <= money_floor {
                        reasons.push(format!("Stressed account equity {stressed_equity:.2} {0} does not clear the money-denominated broker stop-out reserve of {1:.2}.", snapshot.account.currency, money_floor));
                    }
                } else {
                    reasons.push(
                        "Broker stop-out mode is unknown; margin policy cannot be evaluated."
                            .into(),
                    );
                }
            }
        }
    }

    let projected_account_stop_risk = account_risk.current_stop_risk + estimated_loss;
    if !projected_account_stop_risk.is_finite() {
        return Err("Projected account-wide stop-loss risk overflowed; no numeric risk decision can be serialized safely.".into());
    }
    if projected_account_stop_risk > max_account_risk_budget * (1.0 + 1e-9) {
        reasons.push(format!("Projected account-wide stop-loss risk {projected_account_stop_risk:.2} exceeds the ATOS hard budget of {max_account_risk_budget:.2} ({MAX_ACCOUNT_RISK_PERCENT:.2}% of equity)."));
    }

    // A correlated result can become stale after the native response is received but
    // before this object is serialized. Bind the returned decision to the exact latest
    // frame and evaluate freshness at completion, not at function entry.
    let completed_at_ms = chrono::Utc::now().timestamp_millis();
    let quote_age = completed_at_ms.saturating_sub(snapshot.symbol.quote_time_ms);
    if !(0..=MAX_QUOTE_AGE_MS).contains(&quote_age) {
        reasons.push("The MT5 quote became future-dated or stale while risk sizing was being finalized; refresh and retry.".into());
    }
    let final_specification_age = completed_at_ms.saturating_sub(specification.last_observed_at_ms);
    if !(0..=MAX_SPECIFICATION_AGE_MS).contains(&final_specification_age) {
        reasons.push("The broker instrument specification expired while risk sizing was being finalized; refresh and retry.".into());
    }
    let final_status = bridge.status();
    let final_heartbeat_age = final_status.heartbeat_age_ms.unwrap_or(i64::MAX);
    let final_snapshot_matches = final_status.snapshot.as_ref().is_some_and(|current| {
        current.account.server == broker_server
            && current.account.login == account_login
            && current.symbol.name == symbol
            && current.symbol.quote_time_ms == snapshot.symbol.quote_time_ms
            && current.terminal.connected
    });
    if !final_status.connected
        || final_status.stale
        || final_status.quote_stale
        || !(0..=MAX_HEARTBEAT_AGE_MS).contains(&final_heartbeat_age)
        || final_status.last_sequence != calculation_snapshot_sequence
        || !final_snapshot_matches
    {
        reasons.push("The authenticated MT5 account snapshot advanced, disconnected, or became stale while sizing was being finalized; no actionable result is available. Refresh and retry.".into());
    }

    let limitations = vec![
        "Account-wide stop-loss exposure aggregates MT5 OrderCalcProfit results for all open positions and active pending orders; any unprotected or unresolved item blocks new risk. New risk is capped by both the requested trade budget and remaining account-wide capacity.".to_string(),
        "Candidate margin and stop-loss are calculated by MT5 OrderCalcMargin and OrderCalcProfit on the live terminal. Candidate margin is conservatively added to current used margin; broker netting/hedged offsets, aggregate volume tiers and server-side trade checks may differ.".to_string(),
        "ATOS applies a 5% account-wide stop-loss risk ceiling and a margin reserve floor; this is a risk policy, not a guarantee against gaps, slippage, broker liquidation, negative balance or execution rejection.".to_string(),
        "Commission, swap, spread widening, financing, stop-gap execution loss and future margin-policy changes are excluded.".to_string(),
        "The preflight is a point-in-time calculation, not an order reservation. Account state can change before any later execution.".to_string(),
    ];
    let approved_volume = if reasons.is_empty() {
        normalized_volume
    } else {
        0.0
    };
    let approved_loss = if approved_volume > 0.0 {
        estimated_loss
    } else {
        0.0
    };
    let approved_risk_percent = approved_loss / snapshot.account.equity * 100.0;
    let decision = if reasons.is_empty() {
        "SIZED_WITH_LIMITATIONS"
    } else {
        "BLOCKED"
    };
    Ok(Mt5RiskSizingResult {
        decision: decision.into(), reasons, limitations, broker_server: broker_server.into(), account_login: account_login.into(), symbol: symbol.into(), side,
        account_currency: snapshot.account.currency, equity: snapshot.account.equity, risk_percent: request.risk_percent,
        risk_budget, effective_risk_budget, account_available_risk_budget, entry_price: entry, stop_loss_price: request.stop_loss_price, stop_distance, tick_size,
        tick_value_loss_per_lot: tick_value_loss, estimated_loss_per_lot: if normalized_volume > 0.0 { estimated_loss / normalized_volume } else { loss_per_lot }, raw_volume,
        proposed_volume,
        normalized_volume: approved_volume,
        estimated_loss: approved_loss,
        estimated_risk_percent: approved_risk_percent,
        volume_min, volume_max, volume_step, existing_symbol_volume, pending_symbol_volume, total_directional_symbol_volume, volume_limit,
        account_stop_risk: account_risk.current_stop_risk,
        account_stop_risk_percent: account_risk.current_stop_risk_percent,
        unprotected_position_count: account_risk.unprotected_position_count,
        unprotected_pending_order_count: account_risk.unprotected_pending_order_count,
        unresolved_position_risk_count: account_risk.unresolved_position_risk_count,
        unresolved_pending_order_risk_count: account_risk.unresolved_pending_order_risk_count,
        max_account_risk_percent: MAX_ACCOUNT_RISK_PERCENT,
        max_account_risk_budget,
        projected_account_stop_risk,
        candidate_margin, current_margin, projected_margin, projected_free_margin, projected_margin_level_pct,
        broker_margin_call_level, broker_stop_out_level, broker_stopout_mode, margin_preflight_at_ms,
        margin_estimation: "MT5 OrderCalcMargin on the requested side/volume at the terminal quote, added to current account margin without assuming hedged/netting offsets".into(),
        commission_estimation: "NOT_COMPUTED: commission schedule is broker-specific; estimated stop-loss risk excludes commission, swap, slippage and gap risk".into(),
        quote_age_ms: quote_age, specification_revision: specification.revision,
        specification_fingerprint_sha256: specification.fingerprint_sha256, contract_completeness: specification.completeness,
        calculated_at_ms: completed_at_ms, snapshot_sequence: calculation_snapshot_sequence,
    })
}

fn correlated_preflight_inventory(
    bridge: &Mt5Bridge,
    preflight: &Mt5MarginPreflightResult,
    broker_server: &str,
    account_login: &str,
    symbol: &str,
) -> Result<CorrelatedRiskInventory, String> {
    let status: Mt5BridgeStatus = bridge.status();
    if !status.connected
        || status.stale
        || status.quote_stale
        || status.last_sequence != preflight.snapshot_sequence
    {
        return Err("terminal telemetry advanced, became stale, or disconnected before the native result was reconciled".into());
    }
    let heartbeat_age = status.heartbeat_age_ms.unwrap_or(i64::MAX);
    if !(0..=MAX_HEARTBEAT_AGE_MS).contains(&heartbeat_age) {
        return Err(
            "the authenticated telemetry heartbeat is outside the five-second correlation window"
                .into(),
        );
    }
    let snapshot = status
        .snapshot
        .ok_or_else(|| "the correlated telemetry frame has no account snapshot".to_string())?;
    if snapshot.account.server != broker_server
        || snapshot.account.login != account_login
        || snapshot.symbol.name != symbol
        || !snapshot.terminal.connected
    {
        return Err("the correlated telemetry frame belongs to a different account, symbol, or disconnected terminal".into());
    }
    let now = chrono::Utc::now().timestamp_millis();
    quote_age_within_window(now, snapshot.symbol.quote_time_ms)
        .ok_or_else(|| "the quote in the correlated telemetry frame is future-dated or older than sixty seconds".to_string())?;
    if preflight.symbol != symbol || !preflight.success {
        return Err("the native terminal result did not succeed for the requested symbol".into());
    }
    let quote_matches = preflight.quote_time_ms == snapshot.symbol.quote_time_ms
        && nearly_equal(preflight.bid, snapshot.symbol.bid)
        && nearly_equal(preflight.ask, snapshot.symbol.ask);
    if !quote_matches {
        return Err("native margin/profit calculations used a different quote than the correlated account snapshot".into());
    }
    let denominator = snapshot.account.equity.max(1.0);
    let equity_drift =
        (preflight.account_equity - snapshot.account.equity).abs() / denominator * 100.0;
    let margin_drift =
        (preflight.account_margin - snapshot.account.margin).abs() / denominator * 100.0;
    let free_margin_drift =
        (preflight.account_free_margin - snapshot.account.free_margin).abs() / denominator * 100.0;
    if !equity_drift.is_finite()
        || !margin_drift.is_finite()
        || !free_margin_drift.is_finite()
        || equity_drift.max(margin_drift).max(free_margin_drift) > MAX_ACCOUNT_STATE_DRIFT_PERCENT
    {
        return Err(
            "equity, used margin, or free margin changed beyond the permitted account-state drift"
                .into(),
        );
    }
    if preflight.margin_stopout_mode != snapshot.account.margin_stopout_mode
        || (preflight.margin_call_level - snapshot.account.margin_call_level).abs() > 1e-8
        || (preflight.stop_out_level - snapshot.account.stop_out_level).abs() > 1e-8
    {
        return Err(
            "broker stop-out policy changed between the native calculation and correlated frame"
                .into(),
        );
    }
    let account_risk = account_risk_inventory(&snapshot)?;
    if account_risk.unprotected_position_count > 0
        || account_risk.unprotected_pending_order_count > 0
        || account_risk.unresolved_position_risk_count > 0
        || account_risk.unresolved_pending_order_risk_count > 0
    {
        return Err("the correlated account inventory contains unprotected or unresolved position/order risk".into());
    }
    Ok(CorrelatedRiskInventory {
        snapshot,
        account_risk,
    })
}

fn snapshot_contract_matches(
    snapshot: &Mt5Snapshot,
    contract: &crate::infrastructure::mt5::market_data::Mt5InstrumentSpecification,
) -> bool {
    let live = &snapshot.symbol;
    live.name == contract.symbol
        && nearly_equal(live.point, contract.point)
        && nearly_equal(live.tick_size, contract.tick_size)
        && nearly_equal(live.tick_value_profit, contract.tick_value_profit)
        && nearly_equal(live.tick_value_loss, contract.tick_value_loss)
        && nearly_equal(live.contract_size, contract.contract_size)
        && nearly_equal(live.volume_min, contract.volume_min)
        && nearly_equal(live.volume_max, contract.volume_max)
        && nearly_equal(live.volume_step, contract.volume_step)
        && nearly_equal(live.volume_limit, contract.volume_limit)
        && live.digits == contract.digits
        && live.calculation_mode == contract.calculation_mode
        && live.trade_mode == contract.trade_mode
        && live.execution_mode == contract.execution_mode
        && live.filling_mode == contract.filling_mode
        && live.order_mode == contract.order_mode
        && live.stops_level_points == contract.stops_level_points
        && live.freeze_level_points == contract.freeze_level_points
        && live.currency_base == contract.currency_base
        && live.currency_profit == contract.currency_profit
        && live.currency_margin == contract.currency_margin
        && live.swap_mode == contract.swap_mode
        && nearly_equal(live.swap_long, contract.swap_long)
        && nearly_equal(live.swap_short, contract.swap_short)
}

fn market_trade_permission_block_reason(
    broker_account_trade_allowed: bool,
    symbol_trade_mode: i64,
    symbol_order_mode: i64,
    side: &str,
) -> Option<&'static str> {
    if !broker_account_trade_allowed {
        return Some("Broker account-level trading permission is disabled; actionable risk preflight is blocked.");
    }
    match symbol_trade_mode {
        0 => return Some("Broker has disabled trading for this symbol."),
        1 if side == "BUY" => {}
        1 => return Some("Broker permits long positions only for this symbol."),
        2 if side == "SELL" => {}
        2 => return Some("Broker permits short positions only for this symbol."),
        3 => return Some("Broker marks this symbol close-only; new exposure is blocked."),
        4 => {}
        _ => return Some("Broker symbol trading mode is unknown; risk preflight is blocked."),
    }
    const SYMBOL_ORDER_MARKET: i64 = 1;
    const SYMBOL_ORDER_SL: i64 = 16;
    if symbol_order_mode & SYMBOL_ORDER_MARKET == 0 {
        return Some("Broker symbol contract does not permit market orders.");
    }
    if symbol_order_mode & SYMBOL_ORDER_SL == 0 {
        return Some(
            "Broker symbol contract does not permit stop-loss orders; stop-risk sizing is blocked.",
        );
    }
    None
}

fn same_direction_volume(
    positions: &[Mt5PositionSnapshot],
    symbol: &str,
    position_type: i64,
) -> Result<f64, String> {
    checked_volume_total(
        positions
            .iter()
            .filter(|position| position.symbol == symbol && position.position_type == position_type)
            .map(|position| position.volume),
        "same-direction open-position inventory",
    )
}

fn same_direction_pending_volume(
    orders: &[Mt5PendingOrderSnapshot],
    symbol: &str,
    buy: bool,
) -> Result<f64, String> {
    checked_volume_total(
        orders
            .iter()
            .filter(|order| order.symbol == symbol && pending_order_is_buy(order.order_type) == buy)
            .map(|order| order.volume_current),
        "same-direction pending-order inventory",
    )
}

fn checked_volume_total(
    values: impl IntoIterator<Item = f64>,
    subject: &str,
) -> Result<f64, String> {
    let mut total = 0.0_f64;
    for volume in values {
        if !volume.is_finite() || volume < 0.0 {
            return Err(format!(
                "MT5 {subject} contains a non-finite or negative volume."
            ));
        }
        let next = total + volume;
        if !next.is_finite() || next < total {
            return Err(format!(
                "MT5 {subject} overflowed; directional exposure cannot be verified safely."
            ));
        }
        total = next;
    }
    Ok(total)
}

fn pending_order_is_buy(order_type: i64) -> bool {
    // MQL5: BUY_LIMIT=2, BUY_STOP=4, BUY_STOP_LIMIT=6; SELL variants are 3, 5, 7.
    matches!(order_type, 2 | 4 | 6)
}

fn quote_age_within_window(now_ms: i64, observed_at_ms: i64) -> Option<i64> {
    let age = now_ms.saturating_sub(observed_at_ms);
    (0..=MAX_QUOTE_AGE_MS).contains(&age).then_some(age)
}

fn cap_trade_risk_budget(
    requested_trade_budget: f64,
    account_budget: f64,
    current_account_risk: f64,
) -> Result<f64, String> {
    if !requested_trade_budget.is_finite()
        || requested_trade_budget < 0.0
        || !account_budget.is_finite()
        || account_budget < 0.0
        || !current_account_risk.is_finite()
        || current_account_risk < 0.0
    {
        return Err("Risk budget inputs must be finite and non-negative.".into());
    }
    Ok(requested_trade_budget.min((account_budget - current_account_risk).max(0.0)))
}

fn nearly_equal(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= left.abs().max(right.abs()).max(1.0) * 1e-9
}

fn estimate_loss_per_lot(stop_distance: f64, tick_size: f64, tick_value_loss: f64) -> f64 {
    if !stop_distance.is_finite()
        || stop_distance <= 0.0
        || !tick_size.is_finite()
        || tick_size <= 0.0
        || !tick_value_loss.is_finite()
        || tick_value_loss <= 0.0
    {
        return f64::NAN;
    }
    (stop_distance / tick_size) * tick_value_loss
}

fn normalize_volume_down(raw_volume: f64, volume_step: f64, volume_max: f64) -> f64 {
    if !raw_volume.is_finite()
        || raw_volume <= 0.0
        || !volume_step.is_finite()
        || volume_step <= 0.0
        || !volume_max.is_finite()
        || volume_max <= 0.0
    {
        return 0.0;
    }
    let requested_steps = (raw_volume / volume_step).floor();
    let maximum_steps = (volume_max / volume_step).floor();
    if !requested_steps.is_finite()
        || !maximum_steps.is_finite()
        || requested_steps < 1.0
        || maximum_steps < 1.0
    {
        return 0.0;
    }
    let steps = requested_steps.min(maximum_steps);
    let volume = steps * volume_step;
    if volume.is_finite() && volume > 0.0 && volume <= raw_volume && volume <= volume_max {
        volume
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cap_trade_risk_budget, checked_volume_total, estimate_loss_per_lot,
        market_trade_permission_block_reason, nearly_equal, normalize_volume_down,
        quote_age_within_window, same_direction_pending_volume, same_direction_volume,
    };
    use crate::infrastructure::mt5::bridge::{Mt5PendingOrderSnapshot, Mt5PositionSnapshot};

    #[test]
    fn quote_age_is_evaluated_against_the_current_completion_time() {
        assert_eq!(quote_age_within_window(10_000, 10_000), Some(0));
        assert_eq!(quote_age_within_window(70_000, 10_000), Some(60_000));
        assert_eq!(quote_age_within_window(70_001, 10_000), None);
        assert_eq!(quote_age_within_window(9_999, 10_000), None);
    }

    #[test]
    fn actionable_preflight_obeys_broker_account_symbol_and_order_permissions() {
        assert!(market_trade_permission_block_reason(true, 4, 17, "BUY").is_none());
        assert!(market_trade_permission_block_reason(true, 1, 17, "BUY").is_none());
        assert!(market_trade_permission_block_reason(true, 2, 17, "SELL").is_none());
        assert!(
            market_trade_permission_block_reason(false, 4, 17, "BUY").is_some(),
            "broker account-level permission is required for actionable preflight"
        );
        assert!(market_trade_permission_block_reason(true, 0, 17, "BUY").is_some());
        assert!(market_trade_permission_block_reason(true, 1, 17, "SELL").is_some());
        assert!(market_trade_permission_block_reason(true, 2, 17, "BUY").is_some());
        assert!(market_trade_permission_block_reason(true, 3, 17, "BUY").is_some());
        assert!(market_trade_permission_block_reason(true, 4, 16, "BUY").is_some());
        assert!(market_trade_permission_block_reason(true, 4, 1, "BUY").is_some());
        assert!(market_trade_permission_block_reason(true, 9, 17, "BUY").is_some());
    }

    #[test]
    fn trade_budget_is_capped_by_remaining_account_wide_risk_capacity() {
        assert!((cap_trade_risk_budget(200.0, 500.0, 400.0).unwrap() - 100.0).abs() < 1e-12);
        assert!((cap_trade_risk_budget(50.0, 500.0, 100.0).unwrap() - 50.0).abs() < 1e-12);
        assert_eq!(cap_trade_risk_budget(50.0, 500.0, 500.0).unwrap(), 0.0);
        assert!(cap_trade_risk_budget(f64::NAN, 500.0, 0.0).is_err());
        assert!(cap_trade_risk_budget(50.0, 500.0, f64::INFINITY).is_err());
    }

    #[test]
    fn volume_limit_aggregation_is_symbol_and_direction_specific() {
        let position =
            |ticket: &str, symbol: &str, position_type: i64, volume: f64| Mt5PositionSnapshot {
                ticket: ticket.into(),
                symbol: symbol.into(),
                position_type,
                volume,
                price_open: 1.0,
                price_current: 1.0,
                profit: 0.0,
                swap: 0.0,
                stop_loss: 0.0,
                take_profit: 0.0,
                opened_at_ms: 1,
                magic: String::new(),
                risk_status: "UNPROTECTED".into(),
                risk_at_stop_loss: None,
            };
        let positions = vec![
            position("1", "XAUUSD", 0, 0.20),
            position("2", "XAUUSD", 1, 0.70),
            position("3", "EURUSD", 0, 1.00),
        ];
        assert!((same_direction_volume(&positions, "XAUUSD", 0).unwrap() - 0.20).abs() < 1e-12);
        assert!((same_direction_volume(&positions, "XAUUSD", 1).unwrap() - 0.70).abs() < 1e-12);
    }

    #[test]
    fn directional_volume_aggregation_rejects_non_finite_totals() {
        let position = |ticket: &str, volume: f64| Mt5PositionSnapshot {
            ticket: ticket.into(),
            symbol: "XAUUSD".into(),
            position_type: 0,
            volume,
            price_open: 1.0,
            price_current: 1.0,
            profit: 0.0,
            swap: 0.0,
            stop_loss: 0.0,
            take_profit: 0.0,
            opened_at_ms: 1,
            magic: String::new(),
            risk_status: "UNPROTECTED".into(),
            risk_at_stop_loss: None,
        };
        let positions = vec![position("1", f64::MAX), position("2", f64::MAX)];
        assert!(same_direction_volume(&positions, "XAUUSD", 0).is_err());
        assert!(checked_volume_total([f64::MAX, f64::MAX], "test inventory").is_err());
        assert!(checked_volume_total([f64::NAN], "test inventory").is_err());
    }

    #[test]
    fn pending_order_volume_is_aggregated_by_symbol_and_direction() {
        let order =
            |ticket: &str, symbol: &str, order_type: i64, volume: f64| Mt5PendingOrderSnapshot {
                ticket: ticket.into(),
                symbol: symbol.into(),
                order_type,
                state: 1,
                volume_initial: volume,
                volume_current: volume,
                price_open: 1.0,
                price_current: 1.0,
                price_stop_limit: 0.0,
                stop_loss: 0.0,
                take_profit: 0.0,
                setup_at_ms: 1,
                expiration_at_ms: 0,
                magic: "0".into(),
                risk_status: "UNPROTECTED".into(),
                risk_at_stop_loss: None,
            };
        let orders = vec![
            order("1", "XAUUSD", 2, 0.10),
            order("2", "XAUUSD", 5, 0.30),
            order("3", "EURUSD", 4, 1.0),
        ];
        assert!(
            (same_direction_pending_volume(&orders, "XAUUSD", true).unwrap() - 0.10).abs() < 1e-12
        );
        assert!(
            (same_direction_pending_volume(&orders, "XAUUSD", false).unwrap() - 0.30).abs() < 1e-12
        );
    }

    #[test]
    fn live_snapshot_and_durable_contract_comparison_is_tolerant_but_strict() {
        assert!(nearly_equal(0.00001, 0.000010000000001));
        assert!(!nearly_equal(0.00001, 0.00002));
        assert!(!nearly_equal(f64::NAN, 0.00001));
    }

    #[test]
    fn loss_per_lot_uses_broker_tick_loss_value() {
        assert!((estimate_loss_per_lot(0.50, 0.01, 1.25) - 62.5).abs() < 1e-10);
        assert!(estimate_loss_per_lot(0.50, 0.0, 1.25).is_nan());
    }

    #[test]
    fn volume_is_rounded_down_not_nearest() {
        assert!((normalize_volume_down(0.0199, 0.01, 100.0) - 0.01).abs() < 1e-12);
        assert!((normalize_volume_down(0.0200, 0.01, 100.0) - 0.02).abs() < 1e-12);
    }

    #[test]
    fn volume_is_capped_at_broker_maximum() {
        assert!((normalize_volume_down(8.0, 0.1, 2.5) - 2.5).abs() < 1e-12);
    }

    #[test]
    fn volume_cap_is_rounded_down_to_a_broker_valid_step() {
        assert!((normalize_volume_down(8.0, 0.1, 2.55) - 2.5).abs() < 1e-12);
        assert!((normalize_volume_down(2.55, 0.1, 2.55) - 2.5).abs() < 1e-12);
        assert_eq!(normalize_volume_down(8.0, 0.1, 0.05), 0.0);
        assert_eq!(
            normalize_volume_down(f64::MAX, f64::MIN_POSITIVE, 2.55),
            0.0
        );
    }

    #[test]
    fn invalid_inputs_never_produce_volume() {
        assert_eq!(normalize_volume_down(f64::NAN, 0.01, 10.0), 0.0);
        assert_eq!(normalize_volume_down(1.0, 0.0, 10.0), 0.0);
        assert_eq!(normalize_volume_down(1.0, 0.01, f64::INFINITY), 0.0);
        assert_eq!(normalize_volume_down(-1.0, 0.01, 10.0), 0.0);
    }
}
