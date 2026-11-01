//! Fail-closed decision policy for every durable EngineState commit.
//!
//! This layer intentionally operates on the serialized state that is already in SQLite and
//! the candidate payload in the same write transaction. New EngineState fields therefore need
//! an explicit policy classification before changed values can be committed.
use crate::domain::models::MARKET_DATA_OBSERVATION_MAX_AGE_MS;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_AUDIT_EVENTS: usize = 1_500;
const STATE_SCHEMA_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateCommitDecision {
    pub allowed: bool,
    pub transition: &'static str,
    pub reasons: Vec<&'static str>,
}

impl StateCommitDecision {
    fn allow(transition: &'static str) -> Self {
        Self {
            allowed: true,
            transition,
            reasons: Vec::new(),
        }
    }

    fn block(transition: &'static str, reason: &'static str) -> Self {
        Self {
            allowed: false,
            transition,
            reasons: vec![reason],
        }
    }
}

fn field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value.as_object()?.get(name)
}

fn changed(previous: &Value, next: &Value, name: &str) -> bool {
    field(previous, name) != field(next, name)
}

fn as_str<'a>(value: Option<&'a Value>) -> Option<&'a str> {
    value.and_then(Value::as_str)
}

fn as_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64)
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

fn quote_observation_is_fresh(quote: &Value, now_ms: i64) -> bool {
    ["updatedAtMs", "retrievedAtMs"].iter().all(|field_name| {
        as_i64(field(quote, field_name)).is_some_and(|timestamp| {
            timestamp > 0
                && now_ms >= timestamp
                && now_ms.saturating_sub(timestamp) <= MARKET_DATA_OBSERVATION_MAX_AGE_MS
        })
    })
}

fn is_empty_array(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_array)
        .is_some_and(|items| items.is_empty())
}

fn is_empty_object(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_object)
        .is_some_and(|object| object.is_empty())
}

fn event_id(value: &Value) -> Option<&str> {
    as_str(field(value, "id"))
}

fn event_history_is_append_only(previous: &Value, next: &Value) -> bool {
    let (Some(old_events), Some(new_events)) = (
        field(previous, "events").and_then(Value::as_array),
        field(next, "events").and_then(Value::as_array),
    ) else {
        return false;
    };
    if new_events.len() > MAX_AUDIT_EVENTS || old_events.len() > MAX_AUDIT_EVENTS {
        return false;
    }

    let old_ids: Vec<&str> = old_events.iter().filter_map(event_id).collect();
    let new_ids: Vec<&str> = new_events.iter().filter_map(event_id).collect();
    if old_ids.len() != old_events.len() || new_ids.len() != new_events.len() {
        return false;
    }
    let old_set: BTreeSet<&str> = old_ids.iter().copied().collect();
    let new_set: BTreeSet<&str> = new_ids.iter().copied().collect();
    if old_set.len() != old_ids.len() || new_set.len() != new_ids.len() {
        return false;
    }

    let new_count = new_ids
        .iter()
        .take_while(|id| old_set.iter().all(|old| *old != **id))
        .count();
    if new_ids
        .iter()
        .skip(new_count)
        .any(|id| old_set.iter().all(|old| *old != *id))
    {
        return false;
    }
    let retained_count = old_events
        .len()
        .min(MAX_AUDIT_EVENTS.saturating_sub(new_count));
    if new_events.len() != new_count + retained_count
        || new_ids[new_count..] != old_ids[..retained_count]
    {
        return false;
    }

    // Existing audit rows are immutable. Bounded retention may remove only the oldest tail.
    let old_by_id: BTreeMap<&str, &Value> = old_events
        .iter()
        .map(|event| (event_id(event).unwrap_or_default(), event))
        .collect();
    new_events
        .iter()
        .skip(new_count)
        .all(|event| event_id(event).and_then(|id| old_by_id.get(id).copied()) == Some(event))
}

fn has_new_event(previous: &Value, next: &Value, category: &str, title: &str) -> bool {
    let old_ids: BTreeSet<&str> = field(previous, "events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(event_id)
        .collect();
    field(next, "events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|event| {
            event_id(event).is_some_and(|id| old_ids.iter().all(|old_id| *old_id != id))
                && as_str(field(event, "category")) == Some(category)
                && as_str(field(event, "title")) == Some(title)
        })
}

fn has_new_migration_event(previous: &Value, next: &Value) -> bool {
    has_new_event(
        previous,
        next,
        "migration",
        "Legacy synthetic observations removed",
    )
}

fn position_exposure_projection(value: &Value) -> Option<Value> {
    let positions = field(value, "positions")?.as_object()?;
    let mut projected = serde_json::Map::new();
    for (symbol, position) in positions {
        let mut item = position.as_object()?.clone();
        // Mark-to-market is derived state; exposure identity/size/cost basis is not.
        item.remove("markPrice");
        item.remove("notional");
        item.remove("unrealizedPnl");
        projected.insert(symbol.clone(), Value::Object(item));
    }
    Some(Value::Object(projected))
}

fn previous_order_ids(value: &Value) -> Option<Vec<&str>> {
    value
        .get("orders")?
        .as_array()?
        .iter()
        .map(|order| as_str(field(order, "id")))
        .collect()
}

const REQUIRED_ORDER_RISK_RULES: &[&str] = &[
    "Kill switch",
    "Symbol allowlist",
    "Market-data provenance",
    "Market-data freshness",
    "Quantity validity",
    "Supported order type",
    "Order notional",
    "Cash / inventory",
    "Position cap",
    "Portfolio mark freshness",
    "Daily loss guard",
    "Execution mode",
];

fn order_append_is_valid<'a>(
    previous: &Value,
    next: &'a Value,
    now_ms: i64,
) -> Option<(bool, &'a Value)> {
    let old_orders = field(previous, "orders")?.as_array()?;
    let new_orders = field(next, "orders")?.as_array()?;
    if new_orders.len() != old_orders.len().saturating_add(1) || new_orders.is_empty() {
        return None;
    }
    // An append may not rewrite a previously accepted/rejected ledger record.
    if new_orders.iter().skip(1).ne(old_orders.iter()) {
        return None;
    }
    let before_ids = previous_order_ids(previous)?;
    if before_ids.iter().any(|id| id.trim().is_empty())
        || before_ids.iter().copied().collect::<BTreeSet<_>>().len() != before_ids.len()
    {
        return None;
    }
    let added = new_orders.first()?;
    let added_id = as_str(field(added, "id"))?;
    let created_at_ms = as_i64(field(added, "createdAtMs"))?;
    if added_id.trim().is_empty()
        || before_ids.contains(&added_id)
        || created_at_ms <= 0
        || created_at_ms > now_ms
    {
        return None;
    }
    let client_order_id = match field(added, "clientOrderId")? {
        Value::Null => None,
        Value::String(id)
            if !id.trim().is_empty() && id.len() <= 128 && !id.chars().any(char::is_control) =>
        {
            Some(id.as_str())
        }
        _ => return None,
    };
    if client_order_id.is_some_and(|id| {
        old_orders
            .iter()
            .any(|old| as_str(field(old, "clientOrderId")) == Some(id))
    }) {
        return None;
    }
    let status = as_str(field(added, "status"))?;
    let checks = field(added, "riskChecks")?.as_array()?;
    let mut rules = BTreeSet::new();
    let mut all_passed = true;
    let mut any_failed = false;
    for check in checks {
        let rule = as_str(field(check, "rule"))?;
        let passed = field(check, "passed")?.as_bool()?;
        if !rules.insert(rule) {
            return None;
        }
        all_passed &= passed;
        any_failed |= !passed;
    }
    let required: BTreeSet<&str> = REQUIRED_ORDER_RISK_RULES.iter().copied().collect();
    if rules != required {
        return None;
    }
    if !order_risk_checks_match_state(previous, next, added, now_ms) {
        return None;
    }
    if status == "FILLED (PAPER)" {
        if !all_passed || !has_new_event(previous, next, "execution", "Paper order filled") {
            return None;
        }
        Some((true, added))
    } else if status == "REJECTED" {
        if !any_failed || !has_new_event(previous, next, "risk", "Order rejected by risk gateway") {
            return None;
        }
        Some((false, added))
    } else {
        None
    }
}

/// Re-evaluate every recorded risk-check result against the pre-commit account snapshot.
/// A risk-check list is evidence only when its rule set and Boolean outcomes match this policy.
fn order_risk_checks_match_state(
    previous: &Value,
    next: &Value,
    order: &Value,
    now_ms: i64,
) -> bool {
    let (
        Some(symbol),
        Some(side),
        Some(quantity),
        Some(requested_price),
        Some(risk_limits),
        Some(quote_map),
        Some(positions),
        Some(checks),
    ) = (
        as_str(field(order, "symbol")),
        as_str(field(order, "side")),
        as_f64(field(order, "quantity")),
        as_f64(field(order, "requestedPrice")),
        field(previous, "riskLimits"),
        field(previous, "quotes").and_then(Value::as_object),
        field(previous, "positions").and_then(Value::as_object),
        field(order, "riskChecks").and_then(Value::as_array),
    )
    else {
        return false;
    };
    if !quantity.is_finite()
        || quantity <= 0.0
        || !requested_price.is_finite()
        || requested_price <= 0.0
    {
        return false;
    }
    if !["BUY", "SELL"].contains(&side) {
        return false;
    }
    let Some(quote) = quote_map.get(symbol) else {
        return false;
    };
    let (
        Some(market_price),
        Some(quote_live),
        Some(provider_status),
        Some(provider_updated),
        Some(fee_bps),
        Some(slippage_bps),
        Some(max_order),
        Some(max_position),
        Some(max_daily_loss),
        Some(cash),
        Some(daily_start_equity),
        Some(kill_switch_engaged),
    ) = (
        as_f64(field(quote, "price")),
        field(quote, "isLive").and_then(Value::as_bool),
        as_str(field(previous, "providerStatus")),
        as_i64(field(previous, "providerUpdatedAtMs")),
        as_f64(field(risk_limits, "feeBps")),
        as_f64(field(risk_limits, "slippageBps")),
        as_f64(field(risk_limits, "maxOrderNotional")),
        as_f64(field(risk_limits, "maxPositionNotional")),
        as_f64(field(risk_limits, "maxDailyLoss")),
        as_f64(field(previous, "cashBalance")),
        as_f64(field(next, "dailyStartEquity")),
        field(previous, "killSwitchEngaged").and_then(Value::as_bool),
    )
    else {
        return false;
    };
    if !market_price.is_finite()
        || market_price <= 0.0
        || !close_enough(requested_price, market_price)
        || !fee_bps.is_finite()
        || !(0.0..=500.0).contains(&fee_bps)
        || !slippage_bps.is_finite()
        || !(0.0..=500.0).contains(&slippage_bps)
    {
        return false;
    }
    let provider_fresh = provider_status.starts_with("CONNECTED")
        && provider_updated > 0
        && now_ms >= provider_updated
        && now_ms.saturating_sub(provider_updated) <= MARKET_DATA_OBSERVATION_MAX_AGE_MS;
    let quote_fresh = quote_live && quote_observation_is_fresh(quote, now_ms) && provider_fresh;
    let estimated_fill = if side == "BUY" {
        market_price * (1.0 + slippage_bps / 10_000.0)
    } else {
        market_price * (1.0 - slippage_bps / 10_000.0)
    };
    let estimated_notional = quantity * estimated_fill;
    let estimated_fee = estimated_notional * fee_bps / 10_000.0;
    if !estimated_fill.is_finite()
        || estimated_fill <= 0.0
        || !estimated_notional.is_finite()
        || estimated_notional < 0.0
        || !estimated_fee.is_finite()
    {
        return false;
    }
    let (
        Some(order_status),
        Some(recorded_notional),
        Some(recorded_fee),
        Some(recorded_realized_pnl),
    ) = (
        as_str(field(order, "status")),
        as_f64(field(order, "notional")),
        as_f64(field(order, "fee")),
        as_f64(field(order, "realizedPnl")),
    )
    else {
        return false;
    };
    if !close_enough(recorded_notional, estimated_notional)
        || !recorded_fee.is_finite()
        || !recorded_realized_pnl.is_finite()
    {
        return false;
    }
    match order_status {
        "REJECTED" => {
            if !field(order, "fillPrice").is_some_and(Value::is_null)
                || !close_enough(recorded_fee, 0.0)
                || !close_enough(recorded_realized_pnl, 0.0)
            {
                return false;
            }
        }
        "FILLED (PAPER)" => {
            if !field(order, "fillPrice")
                .and_then(Value::as_f64)
                .is_some_and(|price| close_enough(price, estimated_fill))
                || !close_enough(recorded_fee, estimated_fee)
            {
                return false;
            }
        }
        _ => return false,
    }
    let current_position = positions.get(symbol);
    let current_quantity = current_position
        .and_then(|p| as_f64(field(p, "quantity")))
        .unwrap_or(0.0);
    let current_position_value = current_position
        .map(|p| as_f64(field(p, "quantity")).unwrap_or(f64::NAN) * market_price)
        .unwrap_or(0.0);
    let after_position_value = if side == "BUY" {
        current_position_value + estimated_notional
    } else {
        (current_position_value - estimated_notional).max(0.0)
    };
    let portfolio_marks_fresh = positions.iter().all(|(position_symbol, position)| {
        let quantity = as_f64(field(position, "quantity")).unwrap_or(f64::NAN);
        if !quantity.is_finite() || quantity <= 0.0 {
            return false;
        }
        quote_map
            .get(position_symbol)
            .is_some_and(|position_quote| {
                let price = as_f64(field(position_quote, "price")).unwrap_or(f64::NAN);
                let live = field(position_quote, "isLive").and_then(Value::as_bool) == Some(true);
                price.is_finite()
                    && price > 0.0
                    && live
                    && provider_fresh
                    && quote_observation_is_fresh(position_quote, now_ms)
            })
    });
    let estimated_equity = cash
        + positions
            .iter()
            .map(|(position_symbol, position)| {
                let quantity = as_f64(field(position, "quantity")).unwrap_or(f64::NAN);
                let notional = as_f64(field(position, "notional")).unwrap_or(f64::NAN);
                quote_map
                    .get(position_symbol)
                    .and_then(|position_quote| as_f64(field(position_quote, "price")))
                    .map(|price| quantity * price)
                    .unwrap_or(notional)
            })
            .sum::<f64>();
    let daily_pnl = estimated_equity - daily_start_equity;
    let projected_buy_cost = if side == "BUY" {
        ((estimated_fill - market_price) * quantity).max(0.0) + estimated_fee
    } else {
        0.0
    };
    let projected_daily_pnl = daily_pnl - projected_buy_cost;
    let requested_limit_price = field(order, "requestedLimitPrice").unwrap_or(&Value::Null);
    if !requested_limit_price.is_null()
        && !requested_limit_price
            .as_f64()
            .is_some_and(|price| price.is_finite() && price > 0.0)
    {
        return false;
    }
    let limit_order_requested = !requested_limit_price.is_null();
    let expected = [
        ("Kill switch", !kill_switch_engaged),
        ("Symbol allowlist", quote_map.contains_key(symbol)),
        ("Market-data provenance", quote_live && provider_fresh),
        // The original gateway separates timestamp freshness from live/provider provenance.
        ("Market-data freshness", quote_fresh),
        ("Quantity validity", quantity.is_finite() && quantity > 0.0),
        ("Supported order type", !limit_order_requested),
        ("Order notional", estimated_notional <= max_order),
        (
            "Cash / inventory",
            if side == "BUY" {
                cash + 1e-9 >= estimated_notional + estimated_fee
            } else {
                current_position.is_some() && current_quantity + 1e-12 >= quantity
            },
        ),
        (
            "Position cap",
            side == "SELL" || after_position_value <= max_position,
        ),
        (
            "Portfolio mark freshness",
            side == "SELL" || portfolio_marks_fresh,
        ),
        (
            "Daily loss guard",
            side == "SELL" || projected_daily_pnl > -max_daily_loss,
        ),
        ("Execution mode", true),
    ];
    if checks.len() != expected.len() {
        return false;
    }
    let mut actual = BTreeMap::new();
    for check in checks {
        let (Some(rule), Some(passed)) = (
            as_str(field(check, "rule")),
            field(check, "passed").and_then(Value::as_bool),
        ) else {
            return false;
        };
        if actual.insert(rule, passed).is_some() {
            return false;
        }
    }
    let mut ok = true;
    for (rule, passed) in &expected {
        if actual.get(rule).copied() != Some(*passed) {
            ok = false;
        }
    }
    ok
}

fn other_position_exposure_is_unchanged(
    previous: &Value,
    next: &Value,
    excluded_symbol: &str,
) -> bool {
    let (Some(Value::Object(mut before)), Some(Value::Object(mut after))) = (
        position_exposure_projection(previous),
        position_exposure_projection(next),
    ) else {
        return false;
    };
    before.remove(excluded_symbol);
    after.remove(excluded_symbol);
    before == after
}

/// Recompute the financial consequences of a committed paper fill independently of the order
/// handler. This prevents a forged ledger record from pairing correct-looking audit text with
/// arbitrary cash, quantity, cost-basis, fee, or realized-PnL values.
fn paper_fill_economics_are_consistent(
    previous: &Value,
    next: &Value,
    order: &Value,
    now_ms: i64,
) -> bool {
    let (
        Some(symbol),
        Some(side),
        Some(quantity),
        Some(requested_price),
        Some(fill_price),
        Some(notional),
        Some(fee),
        Some(realized_pnl),
    ) = (
        as_str(field(order, "symbol")),
        as_str(field(order, "side")),
        as_f64(field(order, "quantity")),
        as_f64(field(order, "requestedPrice")),
        as_f64(field(order, "fillPrice")),
        as_f64(field(order, "notional")),
        as_f64(field(order, "fee")),
        as_f64(field(order, "realizedPnl")),
    )
    else {
        return false;
    };
    if ![
        quantity,
        requested_price,
        fill_price,
        notional,
        fee,
        realized_pnl,
    ]
    .iter()
    .all(|v| v.is_finite())
        || quantity <= 0.0
        || requested_price <= 0.0
        || fill_price <= 0.0
        || notional <= 0.0
        || fee < 0.0
        || !field(order, "requestedLimitPrice").is_some_and(Value::is_null)
        || !other_position_exposure_is_unchanged(previous, next, symbol)
    {
        return false;
    }
    let (Some(previous_quotes), Some(risk_limits)) = (
        field(previous, "quotes").and_then(Value::as_object),
        field(previous, "riskLimits"),
    ) else {
        return false;
    };
    let Some(quote) = previous_quotes.get(symbol) else {
        return false;
    };
    let (
        Some(quote_price),
        Some(quote_is_live),
        Some(provider_status),
        Some(provider_source),
        Some(provider_time),
        Some(fee_bps),
        Some(slippage_bps),
    ) = (
        as_f64(field(quote, "price")),
        field(quote, "isLive").and_then(Value::as_bool),
        as_str(field(previous, "providerStatus")),
        as_str(field(previous, "providerSource")),
        as_i64(field(previous, "providerUpdatedAtMs")),
        as_f64(field(risk_limits, "feeBps")),
        as_f64(field(risk_limits, "slippageBps")),
    )
    else {
        return false;
    };
    let provider_fresh = provider_time > 0
        && now_ms >= provider_time
        && now_ms.saturating_sub(provider_time) <= MARKET_DATA_OBSERVATION_MAX_AGE_MS
        && provider_status.starts_with("CONNECTED");
    let quote_fresh = quote_observation_is_fresh(quote, now_ms) && provider_fresh;
    if !close_enough(requested_price, quote_price)
        || !quote_is_live
        || !quote_fresh
        || !provider_status.starts_with("CONNECTED")
        || provider_source.trim().is_empty()
        || !provider_fresh
        || fee_bps < 0.0
        || fee_bps > 500.0
        || slippage_bps < 0.0
        || slippage_bps > 500.0
    {
        return false;
    }
    let expected_fill = match side {
        "BUY" => requested_price * (1.0 + slippage_bps / 10_000.0),
        "SELL" => requested_price * (1.0 - slippage_bps / 10_000.0),
        _ => return false,
    };
    let expected_notional = quantity * fill_price;
    let expected_fee = expected_notional * fee_bps / 10_000.0;
    if !expected_fill.is_finite()
        || expected_fill <= 0.0
        || !close_enough(fill_price, expected_fill)
        || !close_enough(notional, expected_notional)
        || !close_enough(fee, expected_fee)
    {
        return false;
    }

    let (Some(previous_cash), Some(next_cash), Some(previous_positions), Some(next_positions)) = (
        as_f64(field(previous, "cashBalance")),
        as_f64(field(next, "cashBalance")),
        field(previous, "positions").and_then(Value::as_object),
        field(next, "positions").and_then(Value::as_object),
    ) else {
        return false;
    };
    let previous_position = previous_positions.get(symbol);
    let previous_quantity = previous_position
        .and_then(|position| as_f64(field(position, "quantity")))
        .unwrap_or(0.0);
    let previous_average = previous_position
        .and_then(|position| as_f64(field(position, "averageEntry")))
        .unwrap_or(0.0);
    if !previous_quantity.is_finite()
        || previous_quantity < 0.0
        || !previous_average.is_finite()
        || previous_average < 0.0
    {
        return false;
    }

    match side {
        "BUY" => {
            let Some(new_position) = next_positions.get(symbol) else {
                return false;
            };
            let (Some(new_quantity), Some(new_average)) = (
                as_f64(field(new_position, "quantity")),
                as_f64(field(new_position, "averageEntry")),
            ) else {
                return false;
            };
            let expected_quantity = previous_quantity + quantity;
            let expected_average =
                (previous_quantity * previous_average + notional + fee) / expected_quantity;
            close_enough(next_cash, previous_cash - notional - fee)
                && close_enough(new_quantity, expected_quantity)
                && close_enough(new_average, expected_average)
                && close_enough(realized_pnl, 0.0)
        }
        "SELL" => {
            if previous_quantity + 1e-12 < quantity || previous_position.is_none() {
                return false;
            }
            let expected_quantity = previous_quantity - quantity;
            let expected_realized = quantity * (fill_price - previous_average) - fee;
            let position_transition_valid = if expected_quantity <= 1e-10 {
                !next_positions.contains_key(symbol)
            } else {
                next_positions.get(symbol).is_some_and(|new_position| {
                    as_f64(field(new_position, "quantity"))
                        .is_some_and(|v| close_enough(v, expected_quantity))
                        && as_f64(field(new_position, "averageEntry"))
                            .is_some_and(|v| close_enough(v, previous_average))
                })
            };
            close_enough(next_cash, previous_cash + notional - fee)
                && close_enough(realized_pnl, expected_realized)
                && position_transition_valid
        }
        _ => false,
    }
}

fn reset_event_is_valid(next: &Value, now_ms: i64) -> bool {
    let empty_book =
        is_empty_array(field(next, "orders")) && is_empty_object(field(next, "positions"));
    let cash_matches = match (
        as_f64(field(next, "cashBalance")),
        as_f64(field(next, "initialBalance")),
    ) {
        (Some(cash), Some(initial)) => cash == initial,
        _ => false,
    };
    let _ = now_ms; // The reset policy is deterministic; market freshness applies on re-arm.
    empty_book
        && cash_matches
        && field(next, "killSwitchEngaged").and_then(Value::as_bool) == Some(true)
}

fn provider_snapshot_is_fresh(state: &Value, now_ms: i64) -> bool {
    let status_connected = as_str(field(state, "providerStatus"))
        .is_some_and(|status| status.starts_with("CONNECTED"));
    let source_present =
        as_str(field(state, "providerSource")).is_some_and(|source| !source.trim().is_empty());
    let timestamp = as_i64(field(state, "providerUpdatedAtMs"));
    let provider_fresh = timestamp.is_some_and(|observed| {
        observed > 0
            && now_ms >= observed
            && now_ms.saturating_sub(observed) <= MARKET_DATA_OBSERVATION_MAX_AGE_MS
    });
    let Some(quotes) = field(state, "quotes").and_then(Value::as_object) else {
        return false;
    };
    let quotes_fresh = !quotes.is_empty()
        && quotes.iter().all(|(symbol, quote)| {
            as_str(field(quote, "symbol")) == Some(symbol.as_str())
                && field(quote, "isLive").and_then(Value::as_bool) == Some(true)
                && as_f64(field(quote, "price"))
                    .is_some_and(|price| price.is_finite() && price > 0.0)
                && quote_observation_is_fresh(quote, now_ms)
        });
    let Some(positions) = field(state, "positions").and_then(Value::as_object) else {
        return false;
    };
    let portfolio_fresh = positions.keys().all(|symbol| {
        quotes.get(symbol).is_some_and(|quote| {
            field(quote, "isLive").and_then(Value::as_bool) == Some(true)
                && quote_observation_is_fresh(quote, now_ms)
        })
    });
    status_connected && source_present && provider_fresh && quotes_fresh && portfolio_fresh
}

fn close_enough(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= 1e-8_f64.max(left.abs().max(right.abs()) * 1e-10)
}

fn account_equity(state: &Value) -> Option<f64> {
    let cash = as_f64(field(state, "cashBalance"))?;
    let positions = field(state, "positions")?.as_object()?;
    let mut equity = cash;
    for position in positions.values() {
        equity += as_f64(field(position, "notional"))?;
    }
    equity.is_finite().then_some(equity)
}

fn position_marks_match_quotes(state: &Value) -> bool {
    let (Some(positions), Some(quotes)) = (
        field(state, "positions").and_then(Value::as_object),
        field(state, "quotes").and_then(Value::as_object),
    ) else {
        return false;
    };
    positions.iter().all(|(symbol, position)| {
        let Some(quote) = quotes.get(symbol) else {
            return true;
        };
        let (
            Some(quantity),
            Some(average_entry),
            Some(price),
            Some(mark),
            Some(notional),
            Some(unrealized),
        ) = (
            as_f64(field(position, "quantity")),
            as_f64(field(position, "averageEntry")),
            as_f64(field(quote, "price")),
            as_f64(field(position, "markPrice")),
            as_f64(field(position, "notional")),
            as_f64(field(position, "unrealizedPnl")),
        )
        else {
            return false;
        };
        close_enough(mark, price)
            && close_enough(notional, quantity * price)
            && close_enough(unrealized, quantity * (price - average_entry))
    })
}

fn derived_risk_metrics_are_consistent(
    previous: &Value,
    next: &Value,
    is_reset: bool,
    now_ms: i64,
) -> bool {
    let (Some(previous_peak), Some(next_peak), Some(previous_drawdown), Some(next_drawdown)) = (
        as_f64(field(previous, "equityPeak")),
        as_f64(field(next, "equityPeak")),
        as_f64(field(previous, "maxDrawdownPct")),
        as_f64(field(next, "maxDrawdownPct")),
    ) else {
        return false;
    };
    let (Some(previous_daily_equity), Some(next_daily_equity)) = (
        as_f64(field(previous, "dailyStartEquity")),
        as_f64(field(next, "dailyStartEquity")),
    ) else {
        return false;
    };
    let previous_daily_date = as_str(field(previous, "dailyStartDate"));
    let next_daily_date = as_str(field(next, "dailyStartDate"));
    if is_reset {
        let Some(initial_balance) = as_f64(field(next, "initialBalance")) else {
            return false;
        };
        return close_enough(next_peak, initial_balance)
            && close_enough(next_drawdown, 0.0)
            && close_enough(next_daily_equity, initial_balance)
            && next_daily_date.is_some_and(|date| !date.is_empty());
    }

    let Some(equity) = account_equity(next) else {
        return false;
    };
    let expected_peak = previous_peak.max(equity);
    let expected_drawdown = previous_drawdown.min(((equity / next_peak) - 1.0).min(0.0) * 100.0);
    if !close_enough(next_peak, expected_peak) || !close_enough(next_drawdown, expected_drawdown) {
        return false;
    }

    let date_changed = previous_daily_date != next_daily_date;
    let daily_equity_changed = !close_enough(previous_daily_equity, next_daily_equity);
    // A calendar rollover may legitimately establish the same equity as yesterday. Require
    // the baseline to stay fixed within a day, but do not mistake an unchanged value for a
    // missing rollover when the date itself advances.
    if !date_changed && daily_equity_changed {
        return false;
    }
    if date_changed {
        let expected_date = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(now_ms)
            .map(|time| time.date_naive().to_string());
        let Some(previous_equity) = account_equity(previous) else {
            return false;
        };
        if next_daily_date != expected_date.as_deref()
            || !close_enough(next_daily_equity, previous_equity)
        {
            return false;
        }
    }
    true
}

fn safe_initial_state(state: &Value) -> bool {
    as_i64(field(state, "schemaVersion")) == Some(STATE_SCHEMA_VERSION)
        && field(state, "killSwitchEngaged").and_then(Value::as_bool) == Some(true)
        && is_empty_array(field(state, "orders"))
        && is_empty_object(field(state, "positions"))
        && is_empty_array(field(state, "agents"))
        && is_empty_object(field(state, "quotes"))
        && is_empty_object(field(state, "candles"))
        && is_empty_array(field(state, "screener"))
        && as_str(field(state, "providerStatus")).is_some_and(|s| s.starts_with("NOT CONNECTED"))
        && as_i64(field(state, "providerUpdatedAtMs")) == Some(0)
        && as_str(field(state, "screenerProviderStatus"))
            .is_some_and(|s| s.starts_with("NOT CONNECTED"))
        && as_i64(field(state, "screenerProviderUpdatedAtMs")) == Some(0)
        && match (
            as_f64(field(state, "cashBalance")),
            as_f64(field(state, "initialBalance")),
        ) {
            (Some(cash), Some(initial)) => cash == initial,
            _ => false,
        }
}

/// Evaluate the candidate state against the exact payload currently stored in SQLite.
/// The caller must run this inside the same transaction that writes the candidate payload.
pub(crate) fn evaluate_state_commit(
    previous: Option<&Value>,
    next: &Value,
    next_state_is_valid: bool,
    now_ms: i64,
) -> StateCommitDecision {
    if !next_state_is_valid {
        return StateCommitDecision::block(
            "INVALID_CANDIDATE",
            "candidate_state_invariants_failed",
        );
    }
    if !next.is_object() || as_i64(field(next, "schemaVersion")) != Some(STATE_SCHEMA_VERSION) {
        return StateCommitDecision::block(
            "SCHEMA_REJECTED",
            "unsupported_or_missing_embedded_schema_version",
        );
    }

    let Some(previous) = previous else {
        return if safe_initial_state(next) {
            StateCommitDecision::allow("SAFE_BOOTSTRAP")
        } else {
            StateCommitDecision::block(
                "BOOTSTRAP_REJECTED",
                "first_commit_must_be_an_empty_locked_paper_account",
            )
        };
    };

    if !previous.is_object()
        || as_i64(field(previous, "schemaVersion")) != Some(STATE_SCHEMA_VERSION)
    {
        return StateCommitDecision::block(
            "PRIOR_STATE_REJECTED",
            "previous_state_schema_is_unknown_or_unsupported",
        );
    }
    if !event_history_is_append_only(previous, next) {
        return StateCommitDecision::block(
            "AUDIT_HISTORY_REJECTED",
            "audit_history_must_be_append_only_and_immutable_with_bounded_tail_retention",
        );
    }

    // Explicit field allowlist: when EngineState gains a field, changes to it are blocked until
    // this decision layer classifies the field. Immutable identity fields are checked below.
    const CLASSIFIED_FIELDS: &[&str] = &[
        "schemaVersion",
        "cashBalance",
        "dailyStartEquity",
        "dailyStartDate",
        "maxDrawdownPct",
        "initialBalance",
        "orders",
        "positions",
        "agents",
        "events",
        "quotes",
        "candles",
        "screener",
        "screenerProviderStatus",
        "screenerProviderSource",
        "screenerProviderUpdatedAtMs",
        "killSwitchEngaged",
        "providerStatus",
        "providerSource",
        "providerUpdatedAtMs",
        "riskLimits",
        "equityPeak",
        "candleRetrievedAtMs",
    ];
    let previous_object = previous
        .as_object()
        .expect("previous state object validated above");
    let next_object = next
        .as_object()
        .expect("candidate state object validated above");
    for key in previous_object.keys().chain(next_object.keys()) {
        if !CLASSIFIED_FIELDS.contains(&key.as_str()) && changed(previous, next, key) {
            return StateCommitDecision::block(
                "UNCLASSIFIED_STATE_CHANGE",
                "changed_state_field_has_no_explicit_decision_policy",
            );
        }
    }

    if changed(previous, next, "schemaVersion") || changed(previous, next, "initialBalance") {
        return StateCommitDecision::block(
            "IMMUTABLE_STATE_CHANGED",
            "embedded_schema_and_initial_account_identity_are_immutable",
        );
    }

    let has_migration = has_new_migration_event(previous, next);
    let reset_event = has_new_event(previous, next, "account", "Paper account reset");
    let kill_changed = changed(previous, next, "killSwitchEngaged");
    if kill_changed {
        let previous_engaged = field(previous, "killSwitchEngaged").and_then(Value::as_bool);
        let next_engaged = field(next, "killSwitchEngaged").and_then(Value::as_bool);
        match (previous_engaged, next_engaged) {
            (Some(true), Some(false)) => {
                if !has_new_event(previous, next, "risk", "Kill switch released") {
                    return StateCommitDecision::block(
                        "REARM_EVIDENCE_MISSING",
                        "kill_switch_release_requires_a_new_explicit_risk_audit_event",
                    );
                }
                if !provider_snapshot_is_fresh(next, now_ms) {
                    return StateCommitDecision::block("REARM_DATA_STALE", "kill_switch_release_requires_a_fresh_connected_provider_and_fresh_portfolio_marks");
                }
            }
            (Some(false), Some(true)) => {
                let explicit_engage = has_new_event(previous, next, "risk", "Kill switch engaged")
                    || has_new_event(previous, next, "risk", "Kill switch re-engaged on startup");
                if !explicit_engage && !has_migration && !reset_event {
                    return StateCommitDecision::block(
                        "LOCK_EVIDENCE_MISSING",
                        "engaging_the_kill_switch_requires_a_risk_or_migration_audit_event",
                    );
                }
            }
            _ => {
                return StateCommitDecision::block(
                    "KILL_SWITCH_STATE_INVALID",
                    "kill_switch_transition_has_invalid_boolean_state",
                )
            }
        }
    }

    if changed(previous, next, "riskLimits")
        && !has_new_event(previous, next, "risk", "Risk policy updated")
    {
        return StateCommitDecision::block(
            "RISK_POLICY_EVIDENCE_MISSING",
            "risk_limit_changes_require_a_new_risk_policy_audit_event",
        );
    }

    let orders_changed = changed(previous, next, "orders");
    let cash_changed = changed(previous, next, "cashBalance");
    let exposure_changed =
        position_exposure_projection(previous) != position_exposure_projection(next);
    let mut filled_order = false;
    if orders_changed {
        if reset_event {
            if !reset_event_is_valid(next, now_ms) {
                return StateCommitDecision::block("ACCOUNT_RESET_REJECTED", "account_reset_must_clear_orders_and_positions_restore_initial_cash_and_engage_kill_switch");
            }
        } else {
            let Some((filled, order)) = order_append_is_valid(previous, next, now_ms) else {
                return StateCommitDecision::block("ORDER_TRANSITION_REJECTED", "order_ledger_may_only_append_one_complete_risk_checked_fill_or_rejection_without_rewriting_prior_orders");
            };
            filled_order = filled;
            if filled && !paper_fill_economics_are_consistent(previous, next, order, now_ms) {
                return StateCommitDecision::block("FILL_ECONOMICS_REJECTED", "paper_fill_cash_quantity_cost_basis_fee_and_realized_pnl_must_match_the_order_and_prior_state");
            }
        }
    }
    if reset_event && !reset_event_is_valid(next, now_ms) {
        return StateCommitDecision::block("ACCOUNT_RESET_REJECTED", "account_reset_must_clear_orders_and_positions_restore_initial_cash_and_engage_kill_switch");
    }
    if cash_changed && !filled_order && !reset_event {
        return StateCommitDecision::block(
            "CASH_TRANSITION_REJECTED",
            "cash_balance_changes_require_a_verified_paper_fill_or_account_reset",
        );
    }
    if exposure_changed && !filled_order && !reset_event {
        return StateCommitDecision::block("EXPOSURE_TRANSITION_REJECTED", "position_identity_quantity_and_cost_basis_changes_require_a_verified_paper_fill_or_account_reset");
    }
    if filled_order && (!cash_changed || !exposure_changed) {
        // A fill must mutate cash and position exposure together; partial account ledger writes
        // are never accepted, even when the order object itself looks valid.
        return StateCommitDecision::block(
            "FILL_ATOMICITY_REJECTED",
            "paper_fill_must_commit_order_cash_and_position_exposure_atomically",
        );
    }

    let market_changed = [
        "quotes",
        "candles",
        "providerStatus",
        "providerSource",
        "providerUpdatedAtMs",
    ]
    .iter()
    .any(|key| changed(previous, next, key));
    if market_changed {
        let valid_market_event =
            has_new_event(previous, next, "market-data", "Market data refreshed")
                || has_new_event(
                    previous,
                    next,
                    "market-data",
                    "Market-data refresh failed; retained last-known values",
                )
                || has_new_event(
                    previous,
                    next,
                    "market-data",
                    "Persisted provider session invalidated",
                )
                || has_migration;
        if !valid_market_event {
            return StateCommitDecision::block(
                "MARKET_STATE_EVIDENCE_MISSING",
                "market_observation_changes_require_a_provider_or_startup_invalidation_audit_event",
            );
        }
    }

    let screener_changed = [
        "screener",
        "screenerProviderStatus",
        "screenerProviderSource",
        "screenerProviderUpdatedAtMs",
    ]
    .iter()
    .any(|key| changed(previous, next, key));
    let unrelated_transition = market_changed
        || screener_changed
        || changed(previous, next, "agents")
        || changed(previous, next, "riskLimits");
    if orders_changed && !reset_event && (unrelated_transition || kill_changed) {
        return StateCommitDecision::block("MULTI_TRANSITION_REJECTED", "order_ledger_transition_cannot_be_combined_with_market_screener_agent_risk_policy_or_kill_switch_changes");
    }
    if reset_event && unrelated_transition {
        return StateCommitDecision::block(
            "MULTI_TRANSITION_REJECTED",
            "account_reset_cannot_be_combined_with_market_screener_agent_or_risk_policy_changes",
        );
    }
    if screener_changed {
        let valid_screener_event = [
            "Screener data refreshed",
            "Screener refresh failed; retained last-known records",
            "Partial DEX Screener result",
        ]
        .iter()
        .any(|title| has_new_event(previous, next, "screener", title))
            || has_new_event(
                previous,
                next,
                "market-data",
                "Persisted provider session invalidated",
            )
            || has_migration;
        if !valid_screener_event {
            return StateCommitDecision::block(
                "SCREENER_STATE_EVIDENCE_MISSING",
                "screener_changes_require_a_provider_or_startup_cleanup_audit_event",
            );
        }
    }

    if changed(previous, next, "agents")
        && !has_new_event(
            previous,
            next,
            "agent",
            "Agent started in observe-only mode",
        )
        && !has_new_event(previous, next, "agent", "Agent stopped")
        && !has_migration
    {
        return StateCommitDecision::block(
            "AGENT_STATE_EVIDENCE_MISSING",
            "agent_state_changes_require_an_operator_control_or_migration_audit_event",
        );
    }

    if !position_marks_match_quotes(next) {
        return StateCommitDecision::block(
            "POSITION_MARKS_REJECTED",
            "position_marks_must_match_the_candidate_quote_snapshot",
        );
    }
    if !derived_risk_metrics_are_consistent(previous, next, reset_event, now_ms) {
        return StateCommitDecision::block("DERIVED_STATE_INVALID", "equity_peak_drawdown_and_daily_baseline_must_follow_their_deterministic_state_equations");
    }

    StateCommitDecision::allow(if reset_event {
        "ACCOUNT_RESET"
    } else if filled_order {
        "PAPER_FILL"
    } else if orders_changed {
        "ORDER_REJECTION"
    } else if kill_changed {
        "KILL_SWITCH_TRANSITION"
    } else if changed(previous, next, "riskLimits") {
        "RISK_POLICY_UPDATE"
    } else if market_changed {
        "MARKET_OBSERVATION"
    } else if screener_changed {
        "SCREENER_OBSERVATION"
    } else if changed(previous, next, "agents") {
        "AGENT_CONTROL"
    } else if changed(previous, next, "events") {
        "AUDIT_APPEND"
    } else {
        "DERIVED_OR_NOOP"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_state() -> Value {
        json!({
            "schemaVersion": 1, "cashBalance": 10000.0, "dailyStartEquity": 10000.0,
            "dailyStartDate": "2026-10-29", "maxDrawdownPct": 0.0, "initialBalance": 10000.0,
            "orders": [], "positions": {}, "agents": [], "events": [], "quotes": {}, "candles": {},
            "screener": [], "screenerProviderStatus": "NOT CONNECTED · NO OBSERVATIONS",
            "screenerProviderSource": "No screener provider observation loaded", "screenerProviderUpdatedAtMs": 0,
            "killSwitchEngaged": true, "providerStatus": "NOT CONNECTED · NO MARKET DATA",
            "providerSource": "No market-data provider observation loaded", "providerUpdatedAtMs": 0,
            "riskLimits": {"maxOrderNotional": 500.0, "maxPositionNotional": 2000.0, "maxDailyLoss": 250.0, "feeBps": 10.0, "slippageBps": 5.0},
            "equityPeak": 10000.0
        })
    }

    fn push_event(state: &mut Value, id: &str, category: &str, title: &str) {
        let events = state.get_mut("events").unwrap().as_array_mut().unwrap();
        events.insert(0, json!({"id": id, "timestampMs": 1_798_000_000_000_i64, "level": "INFO", "category": category, "title": title, "detail": "test evidence"}));
    }

    fn connected_live_state(now_ms: i64) -> Value {
        let mut state = base_state();
        state["killSwitchEngaged"] = json!(false);
        state["providerStatus"] = json!("CONNECTED · TEST SOURCE");
        state["providerSource"] = json!("test provider");
        state["providerUpdatedAtMs"] = json!(now_ms);
        state["quotes"]["BTCUSDT"] = json!({
            "symbol": "BTCUSDT", "price": 67000.0, "change24hPct": 0.0,
            "volume24h": 1.0, "source": "test provider", "updatedAtMs": now_ms, "retrievedAtMs": now_ms, "isLive": true
        });
        push_event(&mut state, "evt-release", "risk", "Kill switch released");
        push_event(
            &mut state,
            "evt-market",
            "market-data",
            "Market data refreshed",
        );
        state
    }

    fn complete_passing_risk_checks() -> Vec<Value> {
        REQUIRED_ORDER_RISK_RULES
            .iter()
            .map(|rule| json!({"rule": rule, "passed": true, "detail": "test evidence"}))
            .collect()
    }

    fn valid_buy_fill(now_ms: i64) -> (Value, Value) {
        let previous = connected_live_state(now_ms);
        let quantity = 0.001_f64;
        let requested_price = 67_000.0_f64;
        let fill_price = requested_price * (1.0 + 5.0 / 10_000.0);
        let notional = quantity * fill_price;
        let fee = notional * 10.0 / 10_000.0;
        let cash = 10_000.0 - notional - fee;
        let average_entry = (notional + fee) / quantity;
        let equity = cash + quantity * requested_price;
        let mut next = previous.clone();
        next["cashBalance"] = json!(cash);
        next["maxDrawdownPct"] = json!(((equity / 10_000.0) - 1.0).min(0.0) * 100.0);
        next["positions"]["BTCUSDT"] = json!({
            "symbol": "BTCUSDT", "quantity": quantity, "averageEntry": average_entry,
            "markPrice": requested_price, "notional": quantity * requested_price,
            "unrealizedPnl": quantity * (requested_price - average_entry)
        });
        next["orders"] = json!([{
            "id": "paper-order-new", "clientOrderId": null, "symbol": "BTCUSDT", "side": "BUY",
            "quantity": quantity, "requestedPrice": requested_price, "requestedLimitPrice": null,
            "fillPrice": fill_price, "notional": notional, "fee": fee, "realizedPnl": 0.0,
            "status": "FILLED (PAPER)", "createdAtMs": now_ms, "riskChecks": complete_passing_risk_checks()
        }]);
        push_event(&mut next, "evt-fill", "execution", "Paper order filled");
        (previous, next)
    }

    #[test]
    fn bootstrap_fails_closed_until_account_is_empty_and_locked() {
        let base = base_state();
        assert!(evaluate_state_commit(None, &base, true, 1_798_000_000_000).allowed);
        let mut unsafe_state = base.clone();
        unsafe_state["killSwitchEngaged"] = json!(false);
        assert_eq!(
            evaluate_state_commit(None, &unsafe_state, true, 1_798_000_000_000).transition,
            "BOOTSTRAP_REJECTED"
        );
    }

    #[test]
    fn risk_policy_cannot_change_without_new_audit_evidence() {
        let previous = base_state();
        let mut next = previous.clone();
        next["riskLimits"]["maxOrderNotional"] = json!(400.0);
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).transition,
            "RISK_POLICY_EVIDENCE_MISSING"
        );
        push_event(&mut next, "evt-risk", "risk", "Risk policy updated");
        assert!(evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).allowed);
    }

    #[test]
    fn kill_switch_release_requires_fresh_provider_and_portfolio_quotes() {
        let previous = base_state();
        let mut next = previous.clone();
        next["killSwitchEngaged"] = json!(false);
        push_event(&mut next, "evt-release", "risk", "Kill switch released");
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).transition,
            "REARM_DATA_STALE"
        );

        next["providerStatus"] = json!("CONNECTED · TEST SOURCE");
        next["providerSource"] = json!("test provider");
        next["providerUpdatedAtMs"] = json!(1_798_000_000_000_i64);
        next["quotes"]["BTCUSDT"] = json!({"symbol":"BTCUSDT", "price":67000.0, "updatedAtMs":1_798_000_000_000_i64, "retrievedAtMs":1_798_000_000_000_i64, "isLive":true});
        push_event(
            &mut next,
            "evt-market",
            "market-data",
            "Market data refreshed",
        );
        assert!(evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).allowed);
    }

    #[test]
    fn kill_switch_release_requires_a_fresh_local_quote_receipt_as_well_as_exchange_time() {
        let previous = base_state();
        let mut next = previous.clone();
        next["killSwitchEngaged"] = json!(false);
        next["providerStatus"] = json!("CONNECTED · TEST SOURCE");
        next["providerSource"] = json!("test provider");
        next["providerUpdatedAtMs"] = json!(1_798_000_000_000_i64);
        next["quotes"]["BTCUSDT"] = json!({
            "symbol":"BTCUSDT", "price":67000.0, "updatedAtMs":1_798_000_000_000_i64,
            "retrievedAtMs":0, "isLive":true
        });
        push_event(&mut next, "evt-release", "risk", "Kill switch released");
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).transition,
            "REARM_DATA_STALE"
        );
    }

    #[test]
    fn reset_requires_empty_account_initial_cash_and_engaged_lock() {
        let mut previous = base_state();
        previous["killSwitchEngaged"] = json!(false);
        previous["cashBalance"] = json!(9500.0);
        previous["orders"] = json!([{"id":"old-order"}]);
        previous["positions"] = json!({"BTCUSDT":{"quantity":1.0,"averageEntry":100.0,"markPrice":100.0,"notional":100.0,"unrealizedPnl":0.0}});
        let mut next = base_state();
        push_event(&mut next, "evt-reset", "account", "Paper account reset");
        assert!(evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).allowed);
        next["killSwitchEngaged"] = json!(false);
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).transition,
            "ACCOUNT_RESET_REJECTED"
        );
    }

    #[test]
    fn order_fill_requires_atomic_cash_position_order_and_risk_evidence() {
        let now_ms = 1_798_000_000_000_i64;
        let (previous, next) = valid_buy_fill(now_ms);
        assert!(evaluate_state_commit(Some(&previous), &next, true, now_ms).allowed);

        let mut tampered_cash = next.clone();
        let original_cash = as_f64(field(&tampered_cash, "cashBalance")).unwrap();
        tampered_cash["cashBalance"] = json!(original_cash + 1.0);
        assert_eq!(
            evaluate_state_commit(Some(&previous), &tampered_cash, true, now_ms).transition,
            "FILL_ECONOMICS_REJECTED"
        );

        let mut forged_risk = next.clone();
        forged_risk["orders"][0]["riskChecks"] = json!(complete_passing_risk_checks());
        forged_risk["orders"][0]["riskChecks"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|check| check["rule"] == "Daily loss guard")
            .unwrap()["passed"] = json!(false);
        assert_eq!(
            evaluate_state_commit(Some(&previous), &forged_risk, true, now_ms).transition,
            "ORDER_TRANSITION_REJECTED"
        );
    }

    #[test]
    fn order_append_cannot_rewrite_a_previous_ledger_record() {
        let now_ms = 1_798_000_000_000_i64;
        let (mut previous, mut next) = valid_buy_fill(now_ms);
        previous["orders"] = json!([{
            "id": "paper-order-old", "symbol": "BTCUSDT", "side": "BUY", "status": "REJECTED"
        }]);
        next["orders"].as_array_mut().unwrap().push(json!({
            "id": "paper-order-old", "symbol": "ETHUSDT", "side": "SELL", "status": "FILLED (PAPER)"
        }));
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, now_ms).transition,
            "ORDER_TRANSITION_REJECTED"
        );
    }

    #[test]
    fn market_observations_require_new_provider_audit_evidence() {
        let previous = base_state();
        let mut next = previous.clone();
        next["providerStatus"] = json!("CONNECTED · UNKNOWN SOURCE");
        next["providerSource"] = json!("unknown source");
        next["providerUpdatedAtMs"] = json!(1_798_000_000_000_i64);
        next["quotes"]["BTCUSDT"] = json!({"symbol":"BTCUSDT", "price":67000.0, "updatedAtMs":1_798_000_000_000_i64, "retrievedAtMs":1_798_000_000_000_i64, "isLive":true});
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).transition,
            "MARKET_STATE_EVIDENCE_MISSING"
        );
        push_event(
            &mut next,
            "evt-market",
            "market-data",
            "Market data refreshed",
        );
        assert!(evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).allowed);
    }

    #[test]
    fn unknown_state_fields_are_blocked_until_explicitly_classified() {
        let previous = base_state();
        let mut next = previous.clone();
        next["unreviewedStateField"] = json!("unclassified");
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).transition,
            "UNCLASSIFIED_STATE_CHANGE"
        );
    }

    #[test]
    fn daily_baseline_rollover_is_valid_when_equity_is_unchanged() {
        let previous = base_state();
        let mut next = previous.clone();
        next["dailyStartDate"] = json!("2026-10-30");
        next["dailyStartEquity"] = json!(10_000.0);
        let now_ms = chrono::DateTime::parse_from_rfc3339("2026-10-30T12:00:00Z")
            .unwrap()
            .timestamp_millis();
        assert!(evaluate_state_commit(Some(&previous), &next, true, now_ms).allowed);
    }

    #[test]
    fn order_risk_check_uses_the_candidate_daily_baseline_after_utc_rollover() {
        let now_ms = chrono::DateTime::parse_from_rfc3339("2026-10-30T12:00:00Z")
            .unwrap()
            .timestamp_millis();
        let (mut previous, mut next) = valid_buy_fill(now_ms);
        previous["cashBalance"] = json!(9_000.0);
        previous["dailyStartDate"] = json!("2026-10-29");
        previous["dailyStartEquity"] = json!(10_000.0);
        previous["maxDrawdownPct"] = json!(-10.0);
        let notional = as_f64(field(&next["orders"][0], "notional")).unwrap();
        let fee = as_f64(field(&next["orders"][0], "fee")).unwrap();
        let position_notional = as_f64(field(&next["positions"]["BTCUSDT"], "notional")).unwrap();
        let cash = 9_000.0 - notional - fee;
        let equity = cash + position_notional;
        next["cashBalance"] = json!(cash);
        next["dailyStartDate"] = json!("2026-10-30");
        next["dailyStartEquity"] = json!(9_000.0);
        next["maxDrawdownPct"] = json!((-10.0_f64).min(((equity / 10_000.0) - 1.0) * 100.0));
        assert!(evaluate_state_commit(Some(&previous), &next, true, now_ms).allowed);
    }

    #[test]
    fn removed_unclassified_state_fields_are_rejected() {
        let mut previous = base_state();
        previous["unreviewedStateField"] = json!("persisted");
        let next = base_state();
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).transition,
            "UNCLASSIFIED_STATE_CHANGE"
        );
    }

    #[test]
    fn audit_history_cannot_be_rewritten_or_cleared() {
        let mut previous = base_state();
        push_event(&mut previous, "evt-old", "risk", "Risk policy updated");
        let mut next = previous.clone();
        next["events"] = json!([]);
        assert_eq!(
            evaluate_state_commit(Some(&previous), &next, true, 1_798_000_000_000).transition,
            "AUDIT_HISTORY_REJECTED"
        );
    }
}
