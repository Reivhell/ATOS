//! Durable, reproducible research workflows.
//!
//! This workspace is deliberately isolated from the account-state database: research
//! artifacts must not be able to mutate balances, positions, order history, or the risk
//! gateway. Every completed run is immutable, content-hashed, and linked to an append-only
//! hash-chained event ledger. Research decisions are never execution permissions.

use crate::{
    application::backtest::{compare_tournament_candidate_priority, StrategyParameterTournament},
    domain::models::{
        BacktestCostStressScenario, BacktestOutOfSampleFold, BacktestRequest, BacktestResult,
        CandleInterval, RiskLimits,
    },
    runtime::trading_core::TradingCore,
};
use chrono::Utc;
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    sync::Mutex,
    time::Duration,
};
use uuid::Uuid;

const WORKSPACE_SCHEMA_VERSION: i64 = 1;
const ZERO_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_HYPOTHESIS_TITLE_CHARS: usize = 160;
const MAX_HYPOTHESIS_THESIS_CHARS: usize = 8_000;
const MAX_RESEARCH_GOAL_CHARS: usize = 2_000;
const MAX_RUN_CARD_BYTES: usize = 1_000_000;
const MAX_HYPOTHESES: usize = 200;
const MAX_RUN_CARDS: usize = 100;
const MAX_RESEARCH_EVENTS: i64 = 1_024;
const MAX_BOOTSTRAP_OBSERVATIONS: usize = 500_000;
const BOOTSTRAP_WORK_BUDGET: u64 = 25_000_000;
const RESEARCH_GATE_VERSION: &str = "atos-research-gate-v1";

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_text(
    value: &str,
    field: &str,
    max_chars: usize,
    allow_newlines: bool,
) -> Result<(), String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} cannot be empty."));
    }
    if trimmed.chars().count() > max_chars {
        return Err(format!("{field} exceeds the {max_chars}-character limit."));
    }
    if trimmed.chars().any(|character| {
        character.is_control()
            && !(allow_newlines && (character == '\n' || character == '\r' || character == '\t'))
    }) {
        return Err(format!("{field} contains unsupported control characters."));
    }
    Ok(())
}

fn validate_symbol(symbol: &str) -> Result<String, String> {
    let canonical = symbol.trim().to_ascii_uppercase();
    if canonical.is_empty()
        || canonical.len() > 30
        || !canonical.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
    {
        return Err(
            "Symbol must be a canonical instrument identifier of at most 30 ASCII characters."
                .into(),
        );
    }
    Ok(canonical)
}

fn validate_interval(value: &str) -> Result<CandleInterval, String> {
    match value {
        "1m" => Ok(CandleInterval::OneMinute),
        "5m" => Ok(CandleInterval::FiveMinutes),
        "15m" => Ok(CandleInterval::FifteenMinutes),
        "1h" => Ok(CandleInterval::OneHour),
        "4h" => Ok(CandleInterval::FourHours),
        "1d" => Ok(CandleInterval::OneDay),
        _ => Err(format!("Unsupported candle interval '{value}'.")),
    }
}

fn interval_from_storage(value: String) -> Result<CandleInterval, String> {
    validate_interval(&value)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateResearchHypothesisRequest {
    pub title: String,
    pub thesis: String,
    pub symbol: String,
    pub interval: CandleInterval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchHypothesis {
    pub id: String,
    pub title: String,
    pub thesis: String,
    pub symbol: String,
    pub interval: CandleInterval,
    pub status: String,
    pub revision: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchWorkflowRequest {
    pub goal: String,
    #[serde(default)]
    pub hypothesis_id: Option<String>,
    pub symbol: String,
    pub interval: CandleInterval,
    pub fast_period: usize,
    pub slow_period: usize,
    pub initial_capital: f64,
    pub max_drawdown_pct: f64,
    pub minimum_positive_oos_ratio: f64,
    pub minimum_sharpe_ratio: f64,
    #[serde(default)]
    pub strategy_tournament: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNode {
    pub node_id: String,
    pub depends_on: Vec<String>,
    pub status: String,
    pub detail: String,
    pub completed_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutOfSampleReview {
    pub fold_count: usize,
    pub positive_fold_count: usize,
    pub positive_fold_ratio: Option<f64>,
    pub median_return_pct: Option<f64>,
    pub worst_drawdown_pct: Option<f64>,
    pub passed: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockBootstrapSummary {
    pub method: String,
    pub available: bool,
    pub simulations: u16,
    pub observations: usize,
    pub block_length: usize,
    pub seed_sha256: String,
    pub return_p05_pct: Option<f64>,
    pub return_median_pct: Option<f64>,
    pub return_p95_pct: Option<f64>,
    pub max_drawdown_p95_pct: Option<f64>,
    pub unavailable_reason: Option<String>,
    pub caveat: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecialistReview {
    pub role: String,
    pub status: String,
    pub summary: String,
    pub metrics: Value,
    pub blockers: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchBacktestEvidence {
    pub result_id: String,
    pub strategy_name: String,
    pub fast_period: usize,
    pub slow_period: usize,
    pub initial_capital: f64,
    pub final_equity: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub win_rate_pct: Option<f64>,
    pub profit_factor: Option<f64>,
    pub gross_profit: f64,
    pub gross_loss: f64,
    pub sharpe_ratio: f64,
    pub trade_count: usize,
    pub fee_bps: f64,
    pub slippage_bps: f64,
    pub candles_used: usize,
    pub data_source: String,
    pub dataset_sha256: String,
    pub dataset_encoding_version: u32,
    pub dataset_start_ms: i64,
    pub dataset_end_ms: i64,
    pub median_candle_interval_ms: i64,
    pub cost_stress_scenarios: Vec<BacktestCostStressScenario>,
    pub out_of_sample_folds: Vec<BacktestOutOfSampleFold>,
    pub warnings: Vec<String>,
}

impl From<&BacktestResult> for ResearchBacktestEvidence {
    fn from(result: &BacktestResult) -> Self {
        Self {
            result_id: result.id.clone(),
            strategy_name: result.strategy_name.clone(),
            fast_period: result.fast_period,
            slow_period: result.slow_period,
            initial_capital: result.initial_capital,
            final_equity: result.final_equity,
            total_return_pct: result.total_return_pct,
            max_drawdown_pct: result.max_drawdown_pct,
            win_rate_pct: result.win_rate_pct,
            profit_factor: result.profit_factor,
            gross_profit: result.gross_profit,
            gross_loss: result.gross_loss,
            sharpe_ratio: result.sharpe_ratio,
            trade_count: result.trade_count,
            fee_bps: result.fee_bps,
            slippage_bps: result.slippage_bps,
            candles_used: result.candles_used,
            data_source: result.data_source.clone(),
            dataset_sha256: result.dataset_sha256.clone(),
            dataset_encoding_version: result.dataset_encoding_version,
            dataset_start_ms: result.dataset_start_ms,
            dataset_end_ms: result.dataset_end_ms,
            median_candle_interval_ms: result.median_candle_interval_ms,
            cost_stress_scenarios: result.cost_stress_scenarios.clone(),
            out_of_sample_folds: result.out_of_sample_folds.clone(),
            warnings: result.warnings.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchGateParameters {
    pub maximum_drawdown_pct: f64,
    pub minimum_positive_oos_ratio: f64,
    pub minimum_sharpe_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchGateDecision {
    pub policy_version: String,
    pub status: String,
    pub eligible_for_paper_validation: bool,
    pub blockers: Vec<String>,
    pub caveats: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchRunCard {
    pub schema_version: u32,
    pub id: String,
    pub hypothesis_id: Option<String>,
    pub hypothesis_revision: Option<i64>,
    pub goal: String,
    pub symbol: String,
    pub interval: CandleInterval,
    pub created_at_ms: i64,
    pub risk_limits_context: RiskLimits,
    pub gate_parameters: ResearchGateParameters,
    pub workflow_graph: Vec<WorkflowNode>,
    pub backtest: ResearchBacktestEvidence,
    pub out_of_sample_review: OutOfSampleReview,
    pub block_bootstrap: BlockBootstrapSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_tournament: Option<StrategyParameterTournament>,
    pub specialist_reviews: Vec<SpecialistReview>,
    pub decision: ResearchGateDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchRunCardRecord {
    pub card: ResearchRunCard,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchIntegrityStatus {
    pub valid: bool,
    pub event_count: u64,
    pub run_card_count: u64,
    pub chain_head_sha256: String,
    pub verified_at_ms: i64,
}

pub struct ResearchWorkspace {
    connection: Mutex<Connection>,
}

impl ResearchWorkspace {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Cannot create research workspace directory: {error}"))?;
        }
        Self::verify_existing_database(path)?;
        let connection = Connection::open(path)
            .map_err(|error| format!("Cannot open research workspace database: {error}"))?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| {
                format!("Cannot configure research workspace lock timeout: {error}")
            })?;
        Self::initialize_database(&connection)?;
        Self::verify_connection(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn verify_existing_database(path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| {
                format!("Cannot inspect existing research workspace before migration: {error}")
            })?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| format!("Cannot read research workspace schema version: {error}"))?;
        if version > WORKSPACE_SCHEMA_VERSION {
            return Err(format!(
                "Research workspace schema {version} is newer than this application supports."
            ));
        }
        if version == WORKSPACE_SCHEMA_VERSION {
            Self::verify_connection(&connection)?;
        } else if version == 0 {
            let object_count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table','view','trigger') AND name NOT LIKE 'sqlite_%'",
                [], |row| row.get(0),
            ).map_err(|error| format!("Cannot inspect unversioned research workspace: {error}"))?;
            if object_count != 0 {
                return Err("Unversioned research workspace contains existing objects; refusing destructive initialization.".into());
            }
        } else {
            return Err(format!(
                "Unsupported research workspace schema version {version}."
            ));
        }
        Ok(())
    }

    fn initialize_database(connection: &Connection) -> Result<(), String> {
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| format!("Cannot inspect research workspace schema: {error}"))?;
        if version > WORKSPACE_SCHEMA_VERSION {
            return Err(format!(
                "Research workspace schema {version} is newer than this application supports."
            ));
        }
        if version == 0 {
            connection.execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE research_hypotheses (
                    id TEXT PRIMARY KEY,
                    title TEXT NOT NULL CHECK(length(title) BETWEEN 1 AND 160),
                    thesis TEXT NOT NULL CHECK(length(thesis) BETWEEN 1 AND 8000),
                    symbol TEXT NOT NULL CHECK(length(symbol) BETWEEN 1 AND 30),
                    interval TEXT NOT NULL CHECK(interval IN ('1m','5m','15m','1h','4h','1d')),
                    status TEXT NOT NULL CHECK(status IN ('DRAFT','UNDER_REVIEW','PAPER_VALIDATION_ELIGIBLE','RESEARCH_FAILED','INCONCLUSIVE','INVALIDATED')),
                    revision INTEGER NOT NULL CHECK(revision > 0),
                    created_at_ms INTEGER NOT NULL CHECK(created_at_ms > 0),
                    updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms >= created_at_ms)
                 );
                 CREATE INDEX idx_research_hypotheses_updated ON research_hypotheses(updated_at_ms DESC);
                 CREATE TRIGGER research_hypotheses_identity_no_update BEFORE UPDATE OF title, thesis, symbol, interval, created_at_ms ON research_hypotheses BEGIN SELECT RAISE(ABORT, 'research hypothesis identity is immutable'); END;
                 CREATE TRIGGER research_hypotheses_no_delete BEFORE DELETE ON research_hypotheses BEGIN SELECT RAISE(ABORT, 'research hypotheses are append-only'); END;
                 CREATE TABLE research_run_cards (
                    id TEXT PRIMARY KEY,
                    hypothesis_id TEXT REFERENCES research_hypotheses(id),
                    created_at_ms INTEGER NOT NULL CHECK(created_at_ms > 0),
                    card_json TEXT NOT NULL CHECK(length(card_json) BETWEEN 2 AND 1000000),
                    card_sha256 TEXT NOT NULL CHECK(length(card_sha256) = 64)
                 );
                 CREATE INDEX idx_research_run_cards_created ON research_run_cards(created_at_ms DESC);
                 CREATE TABLE research_event_chain (
                    sequence INTEGER PRIMARY KEY CHECK(sequence > 0),
                    timestamp_ms INTEGER NOT NULL CHECK(timestamp_ms > 0),
                    aggregate_id TEXT NOT NULL CHECK(length(aggregate_id) BETWEEN 1 AND 64),
                    event_type TEXT NOT NULL CHECK(length(event_type) BETWEEN 1 AND 96),
                    payload_json TEXT NOT NULL CHECK(length(payload_json) BETWEEN 2 AND 20000),
                    previous_hash TEXT NOT NULL CHECK(length(previous_hash) = 64),
                    event_hash TEXT NOT NULL UNIQUE CHECK(length(event_hash) = 64)
                 );
                 CREATE TRIGGER research_event_chain_no_update BEFORE UPDATE ON research_event_chain BEGIN SELECT RAISE(ABORT, 'research event chain is append-only'); END;
                 CREATE TRIGGER research_event_chain_no_delete BEFORE DELETE ON research_event_chain BEGIN SELECT RAISE(ABORT, 'research event chain is append-only'); END;
                 CREATE TRIGGER research_run_cards_no_update BEFORE UPDATE ON research_run_cards BEGIN SELECT RAISE(ABORT, 'research run cards are immutable'); END;
                 CREATE TRIGGER research_run_cards_no_delete BEFORE DELETE ON research_run_cards BEGIN SELECT RAISE(ABORT, 'research run cards are immutable'); END;
                 PRAGMA user_version = 1;
                 COMMIT;"
            ).map_err(|error| format!("Cannot initialize durable research workspace schema: {error}"))?;
        }
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;",
            )
            .map_err(|error| format!("Cannot configure durable research workspace: {error}"))?;
        Ok(())
    }

    fn event_hash(
        sequence: i64,
        timestamp_ms: i64,
        aggregate_id: &str,
        event_type: &str,
        payload_json: &str,
        previous_hash: &str,
    ) -> String {
        sha256_hex(format!(
            "atos-research-event-v1\n{sequence}\n{timestamp_ms}\n{aggregate_id}\n{event_type}\n{payload_json}\n{previous_hash}"
        ).as_bytes())
    }

    fn verify_event_chain(connection: &Connection) -> Result<(u64, String), String> {
        let mut statement = connection.prepare(
            "SELECT sequence, timestamp_ms, aggregate_id, event_type, payload_json, previous_hash, event_hash
             FROM research_event_chain ORDER BY sequence ASC",
        ).map_err(|error| format!("Cannot prepare research event-chain verification: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(|error| format!("Cannot read research event-chain records: {error}"))?;
        let mut expected_sequence = 1_i64;
        let mut previous_hash = ZERO_HASH.to_string();
        let mut count = 0_u64;
        for row in rows {
            if count >= MAX_RESEARCH_EVENTS as u64 {
                return Err(format!(
                    "Research event ledger exceeds the {MAX_RESEARCH_EVENTS}-record safety bound."
                ));
            }
            let (
                sequence,
                timestamp_ms,
                aggregate_id,
                event_type,
                payload_json,
                recorded_previous,
                recorded_hash,
            ) =
                row.map_err(|error| format!("Cannot decode research event-chain record: {error}"))?;
            if sequence != expected_sequence
                || timestamp_ms <= 0
                || aggregate_id.trim().is_empty()
                || aggregate_id.len() > 64
                || aggregate_id.chars().any(char::is_control)
                || event_type.trim().is_empty()
                || event_type.len() > 96
                || event_type.chars().any(char::is_control)
                || payload_json.len() > 20_000
                || serde_json::from_str::<Value>(&payload_json).is_err()
                || !valid_sha256(&recorded_previous)
                || recorded_previous != previous_hash
                || !valid_sha256(&recorded_hash)
                || recorded_hash
                    != Self::event_hash(
                        sequence,
                        timestamp_ms,
                        &aggregate_id,
                        &event_type,
                        &payload_json,
                        &recorded_previous,
                    )
            {
                return Err(format!("Research event-chain integrity failure at sequence {sequence}; research writes are disabled."));
            }
            previous_hash = recorded_hash;
            expected_sequence = expected_sequence
                .checked_add(1)
                .ok_or_else(|| "Research event-chain sequence overflowed.".to_string())?;
            count = count
                .checked_add(1)
                .ok_or_else(|| "Research event count overflowed.".to_string())?;
        }
        Ok((count, previous_hash))
    }

    fn validate_strategy_tournament(
        tournament: &StrategyParameterTournament,
        expected_dataset_sha256: &str,
        expected_dataset_start_ms: i64,
        expected_dataset_end_ms: i64,
    ) -> Result<(), String> {
        const MIN_TRAINING_TRADES: usize = 3;
        const CANDLE_INTERVAL_MS: i64 = 300_000;
        if tournament.algorithm_version != "atos-nested-walk-forward-sma-v1"
            || tournament.selection_metric != "Highest training Sharpe; ties resolve by lower training drawdown, then lexicographic periods"
            || tournament.candidate_count == 0 || tournament.candidate_count > 25
            || tournament.requested_folds < 2 || tournament.requested_folds > 5
            || tournament.folds.len() != tournament.requested_folds
            || tournament.completed_folds > tournament.requested_folds
            || tournament.candidate_evaluations < tournament.candidate_count
            || tournament.dataset_sha256 != expected_dataset_sha256
            || !valid_sha256(&tournament.dataset_sha256)
            || tournament.dataset_start_ms != expected_dataset_start_ms
            || tournament.dataset_end_ms != expected_dataset_end_ms
            || tournament.dataset_start_ms <= 0
            || tournament.dataset_end_ms <= tournament.dataset_start_ms
            || tournament.positive_oos_fold_ratio.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
            || tournament.median_oos_return_pct.is_some_and(|value| !value.is_finite())
            || tournament.worst_oos_max_drawdown_pct.is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || tournament.parameter_stability_pct.is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || tournament.blockers.len() > 32
            || tournament.caveats.is_empty() || tournament.caveats.len() > 16
            || tournament.blockers.iter().any(|value| value.trim().is_empty() || value.len() > 2_000)
            || tournament.caveats.iter().any(|value| value.trim().is_empty() || value.len() > 2_000) {
            return Err("Research run-card nested walk-forward tournament metadata is malformed.".into());
        }

        let completed = tournament
            .folds
            .iter()
            .filter(|fold| fold.status == "COMPLETED")
            .count();
        if completed != tournament.completed_folds {
            return Err(
                "Research run-card tournament completed-fold count is inconsistent.".into(),
            );
        }
        let expected_evaluations = tournament
            .candidate_count
            .checked_mul(tournament.requested_folds)
            .and_then(|value| value.checked_add(tournament.completed_folds))
            .ok_or_else(|| {
                "Research run-card tournament evaluation count overflowed.".to_string()
            })?;
        if tournament.candidate_evaluations != expected_evaluations {
            return Err(
                "Research run-card tournament candidate evaluation count is inconsistent.".into(),
            );
        }

        let mut candidate_catalog: Option<Vec<(usize, usize)>> = None;
        let mut selected_pairs = Vec::with_capacity(completed);
        let mut positive_oos_folds = 0_usize;
        let mut oos_returns = Vec::with_capacity(completed);
        let mut oos_drawdowns = Vec::with_capacity(completed);
        let mut previous_test_end: Option<i64> = None;
        let mut test_window_duration: Option<i64> = None;

        for (index, fold) in tournament.folds.iter().enumerate() {
            if fold.fold_index != index + 1
                || fold.training_start_ms != tournament.dataset_start_ms
                || fold.training_end_ms <= fold.training_start_ms
                || fold.testing_start_ms <= fold.training_end_ms
                || fold.testing_start_ms.checked_sub(fold.training_end_ms)
                    != Some(CANDLE_INTERVAL_MS)
                || fold.testing_end_ms < fold.testing_start_ms
                || fold.testing_end_ms > tournament.dataset_end_ms
                || fold
                    .testing_end_ms
                    .checked_sub(fold.testing_start_ms)
                    .is_none_or(|duration| duration % CANDLE_INTERVAL_MS != 0)
                || fold.candidate_scores.len() != tournament.candidate_count
                || fold.candidate_scores.iter().any(|score| {
                    score.fast_period < 2
                        || score.slow_period <= score.fast_period
                        || score.slow_period > 300
                        || !score.training_sharpe_ratio.is_finite()
                        || !score.training_return_pct.is_finite()
                        || !score.training_max_drawdown_pct.is_finite()
                        || !(0.0..=100.0).contains(&score.training_max_drawdown_pct)
                        || score.training_trade_count > 2_000_000
                        || score.qualifies_for_selection
                            != (score.training_trade_count >= MIN_TRAINING_TRADES)
                })
            {
                return Err(format!(
                    "Research run-card tournament fold {} is malformed.",
                    fold.fold_index
                ));
            }

            if let Some(previous_end) = previous_test_end {
                if fold.training_end_ms != previous_end || fold.testing_start_ms <= previous_end {
                    return Err(
                        "Nested walk-forward folds are not contiguous expanding windows.".into(),
                    );
                }
            }
            let current_duration = fold.testing_end_ms - fold.testing_start_ms;
            if test_window_duration.is_some_and(|duration| duration != current_duration) {
                return Err("Nested walk-forward test windows have inconsistent durations.".into());
            }
            test_window_duration = Some(current_duration);
            previous_test_end = Some(fold.testing_end_ms);

            let catalog = fold
                .candidate_scores
                .iter()
                .map(|score| (score.fast_period, score.slow_period))
                .collect::<Vec<_>>();
            if catalog.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(
                    "Tournament candidate grid must be unique and canonically ordered.".into(),
                );
            }
            if candidate_catalog
                .as_ref()
                .is_some_and(|expected| expected != &catalog)
            {
                return Err(
                    "Nested walk-forward folds used inconsistent parameter candidate grids.".into(),
                );
            }
            candidate_catalog.get_or_insert(catalog);

            let best_candidate = fold
                .candidate_scores
                .iter()
                .filter(|score| score.qualifies_for_selection)
                .min_by(|left, right| {
                    compare_tournament_candidate_priority(
                        left.training_sharpe_ratio,
                        left.training_max_drawdown_pct,
                        left.fast_period,
                        left.slow_period,
                        right.training_sharpe_ratio,
                        right.training_max_drawdown_pct,
                        right.fast_period,
                        right.slow_period,
                    )
                });

            match fold.status.as_str() {
                "COMPLETED" => {
                    let (
                        Some(fast),
                        Some(slow),
                        Some(training_sharpe),
                        Some(test_return),
                        Some(test_drawdown),
                        Some(test_sharpe),
                        Some(test_trades),
                    ) = (
                        fold.selected_fast_period,
                        fold.selected_slow_period,
                        fold.selected_training_sharpe_ratio,
                        fold.out_of_sample_return_pct,
                        fold.out_of_sample_max_drawdown_pct,
                        fold.out_of_sample_sharpe_ratio,
                        fold.out_of_sample_trade_count,
                    )
                    else {
                        return Err("Completed tournament fold is missing selected parameters or OOS metrics.".into());
                    };
                    if fast < 2
                        || slow <= fast
                        || slow > 300
                        || !training_sharpe.is_finite()
                        || !test_return.is_finite()
                        || !test_drawdown.is_finite()
                        || !(0.0..=100.0).contains(&test_drawdown)
                        || !test_sharpe.is_finite()
                        || test_trades > 2_000_000
                    {
                        return Err(
                            "Completed tournament fold contains invalid parameters or OOS metrics."
                                .into(),
                        );
                    }
                    let winner = best_candidate.ok_or_else(|| {
                        "Completed tournament fold has no qualifying training candidate."
                            .to_string()
                    })?;
                    if winner.fast_period != fast
                        || winner.slow_period != slow
                        || (winner.training_sharpe_ratio - training_sharpe).abs() > 1e-9
                    {
                        return Err("Tournament selected parameters are not the deterministic training-only winner.".into());
                    }
                    selected_pairs.push((fast, slow));
                    if test_return > 0.0 {
                        positive_oos_folds += 1;
                    }
                    oos_returns.push(test_return);
                    oos_drawdowns.push(test_drawdown);
                }
                "NO_QUALIFIED_CANDIDATE" => {
                    if best_candidate.is_some()
                        || fold.selected_fast_period.is_some()
                        || fold.selected_slow_period.is_some()
                        || fold.selected_training_sharpe_ratio.is_some()
                        || fold.out_of_sample_return_pct.is_some()
                        || fold.out_of_sample_max_drawdown_pct.is_some()
                        || fold.out_of_sample_sharpe_ratio.is_some()
                        || fold.out_of_sample_trade_count.is_some()
                    {
                        return Err("Unqualified tournament fold must not contain a selected pair or OOS values.".into());
                    }
                }
                _ => {
                    return Err(
                        "Research run-card tournament contains an unsupported fold status.".into(),
                    )
                }
            }
        }

        if tournament.completed_folds == 0 {
            if tournament.positive_oos_fold_ratio.is_some()
                || tournament.median_oos_return_pct.is_some()
                || tournament.worst_oos_max_drawdown_pct.is_some()
                || tournament.parameter_stability_pct.is_some()
                || tournament.blockers.is_empty()
            {
                return Err("Empty tournament must carry blockers and must not report aggregate OOS metrics.".into());
            }
        } else {
            if tournament.positive_oos_fold_ratio.is_none()
                || tournament.median_oos_return_pct.is_none()
                || tournament.worst_oos_max_drawdown_pct.is_none()
                || tournament.parameter_stability_pct.is_none()
            {
                return Err("Completed tournament is missing aggregate OOS metrics.".into());
            }
            let mut counts = BTreeMap::new();
            for pair in selected_pairs {
                *counts.entry(pair).or_insert(0_usize) += 1;
            }
            let expected_stability =
                counts.values().copied().max().unwrap_or(0) as f64 / completed as f64 * 100.0;
            oos_returns.sort_by(f64::total_cmp);
            let middle = oos_returns.len() / 2;
            let expected_median = if oos_returns.len() % 2 == 0 {
                oos_returns[middle - 1] * 0.5 + oos_returns[middle] * 0.5
            } else {
                oos_returns[middle]
            };
            let expected_ratio = positive_oos_folds as f64 / tournament.requested_folds as f64;
            let expected_worst_drawdown = oos_drawdowns.into_iter().max_by(f64::total_cmp);
            let consistent = (tournament.positive_oos_fold_ratio.unwrap() - expected_ratio).abs()
                <= 1e-12
                && (tournament.median_oos_return_pct.unwrap() - expected_median).abs() <= 1e-9
                && (tournament.parameter_stability_pct.unwrap() - expected_stability).abs() <= 1e-9
                && expected_worst_drawdown.is_some_and(|expected| {
                    (tournament.worst_oos_max_drawdown_pct.unwrap() - expected).abs() <= 1e-9
                });
            if !consistent {
                return Err("Research run-card tournament aggregate statistics do not match its stored fold evidence.".into());
            }
            if tournament.completed_folds == tournament.requested_folds
                && !tournament.blockers.is_empty()
            {
                return Err(
                    "A fully completed tournament cannot retain incomplete-fold blockers.".into(),
                );
            }
            if tournament.completed_folds < tournament.requested_folds
                && tournament.blockers.is_empty()
            {
                return Err("An incomplete tournament must retain explicit blockers.".into());
            }
        }
        Ok(())
    }

    fn validate_run_card_semantics(
        card: &ResearchRunCard,
        expected_id: &str,
    ) -> Result<(), String> {
        let backtest = &card.backtest;
        let gate = &card.gate_parameters;
        let risk = &card.risk_limits_context;
        if card.schema_version != 1
            || card.id != expected_id
            || card.created_at_ms <= 0
            || card.symbol != "BTCUSDT"
            || card.interval != CandleInterval::FiveMinutes
            || card.hypothesis_id.is_some() != card.hypothesis_revision.is_some()
            || card
                .hypothesis_revision
                .is_some_and(|revision| revision <= 0)
            || card.hypothesis_id.as_deref().is_some_and(|id| {
                id.is_empty() || id.len() > 64 || id.chars().any(char::is_control)
            })
        {
            return Err(format!(
                "Research run-card identity or native market contract failed for {expected_id}."
            ));
        }
        if backtest.fast_period < 2
            || backtest.slow_period <= backtest.fast_period
            || backtest.candles_used < backtest.slow_period.saturating_add(3)
            || backtest.dataset_encoding_version != 1
            || !valid_sha256(&backtest.dataset_sha256)
            || backtest.dataset_start_ms <= 0
            || backtest.dataset_end_ms <= backtest.dataset_start_ms
            || backtest.median_candle_interval_ms != CandleInterval::FiveMinutes.duration_ms()
            || backtest.data_source.trim().is_empty()
            || backtest
                .data_source
                .to_ascii_lowercase()
                .contains("fixture")
            || backtest
                .data_source
                .to_ascii_lowercase()
                .contains("synthetic")
        {
            return Err(format!("Research run-card data provenance or strategy input contract failed for {expected_id}."));
        }
        let backtest_metrics = [
            backtest.initial_capital,
            backtest.final_equity,
            backtest.total_return_pct,
            backtest.max_drawdown_pct,
            backtest.gross_profit,
            backtest.gross_loss,
            backtest.sharpe_ratio,
            backtest.fee_bps,
            backtest.slippage_bps,
        ];
        let risk_metrics = [
            risk.max_order_notional,
            risk.max_position_notional,
            risk.max_daily_loss,
            risk.fee_bps,
            risk.slippage_bps,
        ];
        if backtest_metrics.iter().any(|value| !value.is_finite())
            || risk_metrics.iter().any(|value| !value.is_finite())
            || backtest.initial_capital <= 0.0
            || backtest.final_equity <= 0.0
            || risk.max_order_notional <= 0.0
            || risk.max_position_notional < risk.max_order_notional
            || risk.max_daily_loss <= 0.0
            || !(0.0..=500.0).contains(&risk.fee_bps)
            || !(0.0..=500.0).contains(&risk.slippage_bps)
            || !gate.maximum_drawdown_pct.is_finite()
            || !(0.1..=100.0).contains(&gate.maximum_drawdown_pct)
            || !gate.minimum_positive_oos_ratio.is_finite()
            || !(0.0..=1.0).contains(&gate.minimum_positive_oos_ratio)
            || !gate.minimum_sharpe_ratio.is_finite()
            || !(-10.0..=20.0).contains(&gate.minimum_sharpe_ratio)
        {
            return Err(format!(
                "Research run-card contains non-finite or out-of-policy metrics for {expected_id}."
            ));
        }
        if validate_workflow_graph(&card.workflow_graph).is_err()
            || card.specialist_reviews.len() != 5
        {
            return Err(format!(
                "Research run-card workflow/reviewer graph is malformed for {expected_id}."
            ));
        }
        if let Some(tournament) = card.strategy_tournament.as_ref() {
            Self::validate_strategy_tournament(
                tournament,
                &backtest.dataset_sha256,
                backtest.dataset_start_ms,
                backtest.dataset_end_ms,
            )?;
        }
        validate_run_card_graph_contract(
            &card.workflow_graph,
            card.created_at_ms,
            card.strategy_tournament.is_some(),
            card.strategy_tournament
                .as_ref()
                .is_some_and(|tournament| tournament.completed_folds == tournament.requested_folds),
        )?;
        let roles: BTreeSet<&str> = card
            .specialist_reviews
            .iter()
            .map(|review| review.role.as_str())
            .collect();
        if roles.len() != 5
            || ![
                "Data Steward",
                "Quant Researcher",
                "Independent Robustness Reviewer",
                "Risk Officer",
                "Investment Committee",
            ]
            .iter()
            .all(|role| roles.contains(role))
        {
            return Err(format!(
                "Research run-card specialist review roster is incomplete for {expected_id}."
            ));
        }
        let graph_status = |node_id: &str| {
            card.workflow_graph
                .iter()
                .find(|node| node.node_id == node_id)
                .map(|node| node.status.as_str())
        };
        let risk_status = card
            .specialist_reviews
            .iter()
            .find(|review| review.role == "Risk Officer")
            .map(|review| review.status.as_str());
        let committee_status = card
            .specialist_reviews
            .iter()
            .find(|review| review.role == "Investment Committee")
            .map(|review| review.status.as_str());
        let out_of_sample_passed = card.specialist_reviews.iter()
            .find(|review| review.role == "Independent Robustness Reviewer")
            .and_then(|review| review.metrics.get("outOfSample"))
            .and_then(|review| review.get("passed"))
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("Research run-card omitted the out-of-sample decision evidence for {expected_id}."))?;
        if graph_status("risk_review") != risk_status
            || graph_status("investment_committee") != committee_status
            || graph_status("walk_forward_review")
                != Some(if out_of_sample_passed {
                    "PASSED"
                } else {
                    "REVIEW_REQUIRED"
                })
            || graph_status("block_bootstrap")
                != Some(if card.block_bootstrap.available {
                    "COMPLETED"
                } else {
                    "UNAVAILABLE"
                })
        {
            return Err(format!("Research run-card DAG status does not match its authoritative review evidence for {expected_id}."));
        }
        if ![
            RESEARCH_GATE_VERSION,
            "atos-research-gate-v2-nested-walk-forward",
        ]
        .contains(&card.decision.policy_version.as_str())
            || (card.strategy_tournament.is_some()
                != (card.decision.policy_version == "atos-research-gate-v2-nested-walk-forward"))
            || (card.decision.eligible_for_paper_validation
                != (card.decision.status == "ELIGIBLE_FOR_PAPER_VALIDATION"
                    && card.decision.blockers.is_empty()))
        {
            return Err(format!(
                "Research run-card decision consistency check failed for {expected_id}."
            ));
        }
        if card.block_bootstrap.method
            != "moving-block bootstrap of strategy equity returns; PCG32 v1"
        {
            return Err(format!(
                "Research run-card bootstrap algorithm contract failed for {expected_id}."
            ));
        }
        if card.block_bootstrap.available {
            if card.block_bootstrap.simulations < 32
                || card.block_bootstrap.simulations > 500
                || card.block_bootstrap.observations < 30
                || !(2..=64).contains(&card.block_bootstrap.block_length)
                || !valid_sha256(&card.block_bootstrap.seed_sha256)
                || card
                    .block_bootstrap
                    .return_p05_pct
                    .is_none_or(|value| !value.is_finite())
                || card
                    .block_bootstrap
                    .return_median_pct
                    .is_none_or(|value| !value.is_finite())
                || card
                    .block_bootstrap
                    .return_p95_pct
                    .is_none_or(|value| !value.is_finite())
                || card
                    .block_bootstrap
                    .max_drawdown_p95_pct
                    .is_none_or(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            {
                return Err(format!(
                    "Research run-card bootstrap evidence is malformed for {expected_id}."
                ));
            }
        } else if card.block_bootstrap.simulations != 0
            || card.block_bootstrap.unavailable_reason.is_none()
        {
            return Err(format!(
                "Research run-card unavailable bootstrap record is inconsistent for {expected_id}."
            ));
        }
        Ok(())
    }

    fn verify_run_cards(connection: &Connection) -> Result<u64, String> {
        let mut statement = connection.prepare(
            "SELECT id, card_json, card_sha256 FROM research_run_cards ORDER BY created_at_ms ASC",
        ).map_err(|error| format!("Cannot prepare research run-card verification: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| format!("Cannot read persisted research run cards: {error}"))?;
        let mut count = 0_u64;
        for row in rows {
            let (id, card_json, expected_hash) =
                row.map_err(|error| format!("Cannot decode research run-card row: {error}"))?;
            if card_json.len() > MAX_RUN_CARD_BYTES
                || !valid_sha256(&expected_hash)
                || sha256_hex(card_json.as_bytes()) != expected_hash
            {
                return Err(format!("Research run-card integrity failure for {id}; research workspace is read-only until investigated."));
            }
            let card: ResearchRunCard = serde_json::from_str(&card_json).map_err(|_| {
                format!("Research run-card payload {id} is not a valid supported record.")
            })?;
            Self::validate_run_card_semantics(&card, &id)?;
            count = count
                .checked_add(1)
                .ok_or_else(|| "Research run-card count overflowed.".to_string())?;
            if count > MAX_RUN_CARDS as u64 {
                return Err(format!(
                    "Research run-card ledger exceeds the {MAX_RUN_CARDS}-record safety bound."
                ));
            }
        }
        Ok(count)
    }

    fn verify_schema_contract(connection: &Connection) -> Result<(), String> {
        const REQUIRED_TRIGGERS: [(&str, &str); 6] = [
            ("research_hypotheses_identity_no_update", "CREATE TRIGGER research_hypotheses_identity_no_update BEFORE UPDATE OF title, thesis, symbol, interval, created_at_ms ON research_hypotheses BEGIN SELECT RAISE(ABORT, 'research hypothesis identity is immutable'); END"),
            ("research_hypotheses_no_delete", "CREATE TRIGGER research_hypotheses_no_delete BEFORE DELETE ON research_hypotheses BEGIN SELECT RAISE(ABORT, 'research hypotheses are append-only'); END"),
            ("research_event_chain_no_update", "CREATE TRIGGER research_event_chain_no_update BEFORE UPDATE ON research_event_chain BEGIN SELECT RAISE(ABORT, 'research event chain is append-only'); END"),
            ("research_event_chain_no_delete", "CREATE TRIGGER research_event_chain_no_delete BEFORE DELETE ON research_event_chain BEGIN SELECT RAISE(ABORT, 'research event chain is append-only'); END"),
            ("research_run_cards_no_update", "CREATE TRIGGER research_run_cards_no_update BEFORE UPDATE ON research_run_cards BEGIN SELECT RAISE(ABORT, 'research run cards are immutable'); END"),
            ("research_run_cards_no_delete", "CREATE TRIGGER research_run_cards_no_delete BEFORE DELETE ON research_run_cards BEGIN SELECT RAISE(ABORT, 'research run cards are immutable'); END"),
        ];
        for (name, required_sql) in REQUIRED_TRIGGERS {
            let sql: Option<String> = connection
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type='trigger' AND name=?1",
                    [name],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| {
                    format!("Cannot inspect research database guard {name}: {error}")
                })?;
            let Some(sql) = sql else {
                return Err(format!(
                    "Research database guard {name} is missing; writes are disabled."
                ));
            };
            let normalize_sql = |value: &str| {
                value
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_ascii_uppercase()
            };
            if normalize_sql(&sql) != normalize_sql(required_sql) {
                return Err(format!("Research database guard {name} does not exactly match the required immutable-table policy."));
            }
        }
        Ok(())
    }

    fn verify_hypotheses(connection: &Connection) -> Result<u64, String> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            i64,
            i64,
            i64,
        )> = {
            let mut statement = connection.prepare(
                "SELECT id,title,thesis,symbol,interval,status,revision,created_at_ms,updated_at_ms
                 FROM research_hypotheses ORDER BY id ASC",
            ).map_err(|error| format!("Cannot prepare research hypothesis verification: {error}"))?;
            let mapped = statement
                .query_map([], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                })
                .map_err(|error| {
                    format!("Cannot read research hypotheses for verification: {error}")
                })?;
            mapped
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| {
                    format!("Cannot decode research hypothesis during verification: {error}")
                })?
        };
        if rows.len() > MAX_HYPOTHESES {
            return Err(format!(
                "Research hypothesis registry exceeds the {MAX_HYPOTHESES}-record safety bound."
            ));
        }

        for (
            id,
            title,
            thesis,
            symbol,
            interval_text,
            status,
            revision,
            created_at_ms,
            updated_at_ms,
        ) in &rows
        {
            Self::validate_identifier(id, "Persisted hypothesis ID")?;
            validate_text(
                title,
                "Persisted hypothesis title",
                MAX_HYPOTHESIS_TITLE_CHARS,
                false,
            )?;
            validate_text(
                thesis,
                "Persisted hypothesis thesis",
                MAX_HYPOTHESIS_THESIS_CHARS,
                true,
            )?;
            if validate_symbol(symbol)? != symbol.as_str()
                || *symbol != "BTCUSDT"
                || interval_from_storage(interval_text.clone())? != CandleInterval::FiveMinutes
                || *revision <= 0
                || *created_at_ms <= 0
                || *updated_at_ms < *created_at_ms
            {
                return Err(format!("Persisted hypothesis identity or metadata is outside the supported contract: {id}."));
            }

            let events: Vec<(i64, String, String)> = {
                let mut statement = connection
                    .prepare(
                        "SELECT timestamp_ms,event_type,payload_json FROM research_event_chain
                     WHERE aggregate_id=?1 ORDER BY sequence ASC",
                    )
                    .map_err(|error| {
                        format!("Cannot prepare event replay for hypothesis {id}: {error}")
                    })?;
                let mapped = statement
                    .query_map([id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                    .map_err(|error| {
                        format!("Cannot read event replay for hypothesis {id}: {error}")
                    })?;
                mapped
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|error| {
                        format!("Cannot decode event replay for hypothesis {id}: {error}")
                    })?
            };
            if events.is_empty() {
                return Err(format!("Persisted hypothesis {id} has no creation event; integrity verification failed."));
            }

            let mut expected_revision = 0_i64;
            let mut expected_status = String::new();
            let mut previous_timestamp = 0_i64;
            for (index, (timestamp_ms, event_type, payload_json)) in events.iter().enumerate() {
                if *timestamp_ms <= 0 || (index > 0 && *timestamp_ms < previous_timestamp) {
                    return Err(format!(
                        "Hypothesis {id} has non-monotonic lifecycle timestamps."
                    ));
                }
                let payload: Value = serde_json::from_str(payload_json).map_err(|_| {
                    format!("Hypothesis {id} has a malformed lifecycle event payload.")
                })?;
                if index == 0 {
                    if event_type != "HYPOTHESIS_CREATED"
                        || payload.get("title").and_then(Value::as_str) != Some(title.as_str())
                        || payload.get("thesis").and_then(Value::as_str) != Some(thesis.as_str())
                        || payload.get("symbol").and_then(Value::as_str) != Some(symbol.as_str())
                        || payload.get("interval").and_then(Value::as_str)
                            != Some(interval_text.as_str())
                        || payload.get("revision").and_then(Value::as_i64) != Some(1)
                    {
                        return Err(format!(
                            "Hypothesis {id} creation event does not match its persisted identity."
                        ));
                    }
                    expected_revision = 1;
                    expected_status = "DRAFT".into();
                    if *timestamp_ms != *created_at_ms {
                        return Err(format!("Hypothesis {id} creation timestamp does not match its first audit event."));
                    }
                } else {
                    if expected_status == "INVALIDATED" {
                        return Err(format!(
                            "Hypothesis {id} has lifecycle events after invalidation."
                        ));
                    }
                    match event_type.as_str() {
                        "HYPOTHESIS_RESEARCH_REVIEWED" => {
                            let next_status = payload
                                .get("status")
                                .and_then(Value::as_str)
                                .ok_or_else(|| {
                                    format!("Hypothesis {id} review event has no status.")
                                })?;
                            if ![
                                "PAPER_VALIDATION_ELIGIBLE",
                                "INCONCLUSIVE",
                                "RESEARCH_FAILED",
                            ]
                            .contains(&next_status)
                                || payload.get("fromRevision").and_then(Value::as_i64)
                                    != Some(expected_revision)
                            {
                                return Err(format!("Hypothesis {id} review event has an invalid status or stale revision."));
                            }
                            expected_revision =
                                expected_revision.checked_add(1).ok_or_else(|| {
                                    format!(
                                        "Hypothesis {id} revision overflowed during event replay."
                                    )
                                })?;
                            expected_status = next_status.into();
                        }
                        "HYPOTHESIS_INVALIDATED" => {
                            if payload.get("expectedRevision").and_then(Value::as_i64)
                                != Some(expected_revision)
                                || payload
                                    .get("reason")
                                    .and_then(Value::as_str)
                                    .is_none_or(str::is_empty)
                            {
                                return Err(format!("Hypothesis {id} invalidation event has an invalid reason or stale revision."));
                            }
                            expected_revision = expected_revision.checked_add(1)
                                .ok_or_else(|| format!("Hypothesis {id} revision overflowed during invalidation replay."))?;
                            expected_status = "INVALIDATED".into();
                        }
                        _ => {
                            return Err(format!(
                            "Hypothesis {id} contains an unsupported lifecycle event {event_type}."
                        ))
                        }
                    }
                }
                previous_timestamp = *timestamp_ms;
            }
            if expected_revision != *revision
                || expected_status != *status
                || previous_timestamp != *updated_at_ms
            {
                return Err(format!(
                    "Persisted hypothesis {id} does not match its audited lifecycle state."
                ));
            }
        }

        let orphaned_hypothesis_events: i64 = connection
            .query_row(
                "SELECT COUNT(DISTINCT e.aggregate_id) FROM research_event_chain e
             LEFT JOIN research_hypotheses h ON h.id=e.aggregate_id
             WHERE e.event_type LIKE 'HYPOTHESIS_%' AND h.id IS NULL",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("Cannot verify orphaned hypothesis audit events: {error}"))?;
        if orphaned_hypothesis_events != 0 {
            return Err("Research audit history contains hypothesis events without persisted hypothesis records.".into());
        }
        Ok(rows.len() as u64)
    }

    fn verify_connection(connection: &Connection) -> Result<(u64, u64, String), String> {
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| {
                format!("Cannot read research workspace version during verification: {error}")
            })?;
        if version != WORKSPACE_SCHEMA_VERSION {
            return Err(format!(
                "Research workspace schema version {version} is not supported."
            ));
        }
        Self::verify_schema_contract(connection)?;
        let (event_count, head) = Self::verify_event_chain(connection)?;
        let _hypothesis_count = Self::verify_hypotheses(connection)?;
        let card_count = Self::verify_run_cards(connection)?;
        Ok((event_count, card_count, head))
    }

    fn append_event(
        transaction: &Transaction<'_>,
        timestamp_ms: i64,
        aggregate_id: &str,
        event_type: &str,
        payload: &Value,
    ) -> Result<String, String> {
        if timestamp_ms <= 0
            || aggregate_id.is_empty()
            || aggregate_id.len() > 64
            || aggregate_id.chars().any(char::is_control)
            || event_type.is_empty()
            || event_type.len() > 96
            || event_type.chars().any(char::is_control)
        {
            return Err("Research audit event metadata violates its bounded schema.".into());
        }
        let payload_json = serde_json::to_string(payload)
            .map_err(|error| format!("Cannot encode research audit payload: {error}"))?;
        if payload_json.len() > 20_000 {
            return Err("Research audit payload exceeds the 20 KB safety bound.".into());
        }
        let head: Option<(i64, String)> = transaction.query_row(
            "SELECT sequence, event_hash FROM research_event_chain ORDER BY sequence DESC LIMIT 1",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional().map_err(|error| format!("Cannot read research audit head: {error}"))?;
        let (sequence, previous_hash) = match head {
            Some((last_sequence, hash)) => (
                last_sequence
                    .checked_add(1)
                    .ok_or_else(|| "Research audit sequence overflowed.".to_string())?,
                hash,
            ),
            None => (1, ZERO_HASH.to_string()),
        };
        if sequence > MAX_RESEARCH_EVENTS {
            return Err(format!("Research event ledger is limited to {MAX_RESEARCH_EVENTS} records; export and archive this workspace before further writes."));
        }
        let event_hash = Self::event_hash(
            sequence,
            timestamp_ms,
            aggregate_id,
            event_type,
            &payload_json,
            &previous_hash,
        );
        transaction.execute(
            "INSERT INTO research_event_chain(sequence,timestamp_ms,aggregate_id,event_type,payload_json,previous_hash,event_hash)
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![sequence, timestamp_ms, aggregate_id, event_type, payload_json, previous_hash, event_hash],
        ).map_err(|error| format!("Cannot append immutable research audit event: {error}"))?;
        Ok(event_hash)
    }

    pub fn create_hypothesis(
        &self,
        request: CreateResearchHypothesisRequest,
    ) -> Result<ResearchHypothesis, String> {
        validate_text(
            &request.title,
            "Hypothesis title",
            MAX_HYPOTHESIS_TITLE_CHARS,
            false,
        )?;
        validate_text(
            &request.thesis,
            "Hypothesis thesis",
            MAX_HYPOTHESIS_THESIS_CHARS,
            true,
        )?;
        let symbol = validate_symbol(&request.symbol)?;
        if symbol != "BTCUSDT" || request.interval != CandleInterval::FiveMinutes {
            return Err("Research hypotheses currently accept only the validated BTCUSDT 5m native candle series.".into());
        }
        let now = Utc::now().timestamp_millis();
        let hypothesis = ResearchHypothesis {
            id: Uuid::new_v4().to_string(),
            title: request.title.trim().to_string(),
            thesis: request.thesis.trim().to_string(),
            symbol,
            interval: request.interval,
            status: "DRAFT".into(),
            revision: 1,
            created_at_ms: now,
            updated_at_ms: now,
        };
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Research workspace lock is poisoned.".to_string())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("Cannot begin hypothesis transaction: {error}"))?;
        Self::verify_connection(&transaction)?;
        let hypothesis_count: i64 = transaction
            .query_row("SELECT COUNT(*) FROM research_hypotheses", [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("Cannot enforce research hypothesis capacity: {error}"))?;
        if hypothesis_count >= MAX_HYPOTHESES as i64 {
            return Err(format!("Research workspace is limited to {MAX_HYPOTHESES} hypotheses; invalidate or archive an existing research line before creating another."));
        }
        transaction.execute(
            "INSERT INTO research_hypotheses(id,title,thesis,symbol,interval,status,revision,created_at_ms,updated_at_ms)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![hypothesis.id, hypothesis.title, hypothesis.thesis, hypothesis.symbol,
                hypothesis.interval.as_str(), hypothesis.status, hypothesis.revision,
                hypothesis.created_at_ms, hypothesis.updated_at_ms],
        ).map_err(|error| format!("Cannot persist research hypothesis: {error}"))?;
        Self::append_event(
            &transaction,
            now,
            &hypothesis.id,
            "HYPOTHESIS_CREATED",
            &json!({
                "title": hypothesis.title, "thesis": hypothesis.thesis, "symbol": hypothesis.symbol,
                "interval": hypothesis.interval.as_str(), "revision": hypothesis.revision,
            }),
        )?;
        transaction
            .commit()
            .map_err(|error| format!("Cannot commit research hypothesis: {error}"))?;
        Ok(hypothesis)
    }

    pub fn list_hypotheses(
        &self,
        requested_limit: usize,
    ) -> Result<Vec<ResearchHypothesis>, String> {
        let limit = requested_limit.clamp(1, MAX_HYPOTHESES) as i64;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Research workspace lock is poisoned.".to_string())?;
        let mut statement = connection
            .prepare(
                "SELECT id,title,thesis,symbol,interval,status,revision,created_at_ms,updated_at_ms
             FROM research_hypotheses ORDER BY updated_at_ms DESC,id ASC LIMIT ?1",
            )
            .map_err(|error| format!("Cannot prepare hypothesis listing: {error}"))?;
        let rows = statement
            .query_map([limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            })
            .map_err(|error| format!("Cannot query research hypotheses: {error}"))?;
        let mut hypotheses = Vec::new();
        for row in rows {
            let (
                id,
                title,
                thesis,
                symbol,
                interval,
                status,
                revision,
                created_at_ms,
                updated_at_ms,
            ) = row.map_err(|error| format!("Cannot decode research hypothesis: {error}"))?;
            hypotheses.push(ResearchHypothesis {
                id,
                title,
                thesis,
                symbol,
                interval: interval_from_storage(interval)?,
                status,
                revision,
                created_at_ms,
                updated_at_ms,
            });
        }
        Ok(hypotheses)
    }

    pub fn invalidate_hypothesis(
        &self,
        hypothesis_id: String,
        reason: String,
        expected_revision: i64,
    ) -> Result<ResearchHypothesis, String> {
        validate_text(&reason, "Invalidation reason", 2_000, true)?;
        Self::validate_identifier(&hypothesis_id, "Hypothesis ID")?;
        if expected_revision <= 0 {
            return Err("Expected hypothesis revision must be a positive integer.".into());
        }
        let now = Utc::now().timestamp_millis();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Research workspace lock is poisoned.".to_string())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                format!("Cannot begin hypothesis invalidation transaction: {error}")
            })?;
        Self::verify_connection(&transaction)?;
        let changed = transaction.execute(
            "UPDATE research_hypotheses SET status='INVALIDATED',revision=revision+1,updated_at_ms=?1
             WHERE id=?2 AND revision=?3 AND status NOT IN ('INVALIDATED')",
            params![now, hypothesis_id, expected_revision],
        ).map_err(|error| format!("Cannot invalidate research hypothesis: {error}"))?;
        if changed != 1 {
            return Err("Hypothesis was not changed: it may not exist, may already be invalidated, or its revision is stale.".into());
        }
        Self::append_event(
            &transaction,
            now,
            &hypothesis_id,
            "HYPOTHESIS_INVALIDATED",
            &json!({
                "reason": reason.trim(), "expectedRevision": expected_revision,
            }),
        )?;
        let hypothesis = Self::load_hypothesis(&transaction, &hypothesis_id)?.ok_or_else(|| {
            "Invalidated hypothesis disappeared inside its transaction.".to_string()
        })?;
        transaction
            .commit()
            .map_err(|error| format!("Cannot commit hypothesis invalidation: {error}"))?;
        Ok(hypothesis)
    }

    fn validate_identifier(value: &str, field: &str) -> Result<(), String> {
        if value.is_empty() || value.len() > 64 || value.chars().any(char::is_control) {
            return Err(format!("{field} is invalid."));
        }
        Ok(())
    }

    fn load_hypothesis(
        connection: &Connection,
        hypothesis_id: &str,
    ) -> Result<Option<ResearchHypothesis>, String> {
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            i64,
            i64,
            i64,
        )> = connection
            .query_row(
                "SELECT id,title,thesis,symbol,interval,status,revision,created_at_ms,updated_at_ms
             FROM research_hypotheses WHERE id=?1",
                [hypothesis_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                        row.get(8)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| format!("Cannot load research hypothesis: {error}"))?;
        row.map(
            |(
                id,
                title,
                thesis,
                symbol,
                interval,
                status,
                revision,
                created_at_ms,
                updated_at_ms,
            )| {
                Ok(ResearchHypothesis {
                    id,
                    title,
                    thesis,
                    symbol,
                    interval: interval_from_storage(interval)?,
                    status,
                    revision,
                    created_at_ms,
                    updated_at_ms,
                })
            },
        )
        .transpose()
    }

    pub fn run_workflow(
        &self,
        core: &TradingCore,
        request: ResearchWorkflowRequest,
    ) -> Result<ResearchRunCardRecord, String> {
        validate_text(
            &request.goal,
            "Research goal",
            MAX_RESEARCH_GOAL_CHARS,
            true,
        )?;
        let symbol = validate_symbol(&request.symbol)?;
        if request.fast_period < 2
            || request.slow_period <= request.fast_period
            || request.slow_period > 300
        {
            return Err("Research strategy periods must satisfy 2 <= fast < slow <= 300.".into());
        }
        if !request.initial_capital.is_finite()
            || request.initial_capital <= 0.0
            || request.initial_capital > 1_000_000_000_000.0
        {
            return Err("Initial research capital must be finite, positive, and no greater than USD 1 trillion.".into());
        }
        if !request.max_drawdown_pct.is_finite()
            || !(0.1..=100.0).contains(&request.max_drawdown_pct)
        {
            return Err("Maximum tolerated drawdown must be between 0.1% and 100%.".into());
        }
        if !request.minimum_positive_oos_ratio.is_finite()
            || !(0.0..=1.0).contains(&request.minimum_positive_oos_ratio)
        {
            return Err("Minimum positive out-of-sample ratio must be between 0 and 1.".into());
        }
        if !request.minimum_sharpe_ratio.is_finite()
            || !(-10.0..=20.0).contains(&request.minimum_sharpe_ratio)
        {
            return Err("Minimum Sharpe threshold must be finite and between -10 and 20.".into());
        }
        // The present native market-data adapter guarantees only this exact series.
        // Do not let a caller label the BTCUSDT 5m cache as a different instrument/timeframe.
        if symbol != "BTCUSDT" || request.interval != CandleInterval::FiveMinutes {
            return Err("Research currently accepts only the validated BTCUSDT 5m native candle series. No substitute symbol or timeframe will be used.".into());
        }
        let hypothesis_id = request.hypothesis_id.as_deref();
        let hypothesis_revision = if let Some(id) = hypothesis_id {
            Self::validate_identifier(id, "Hypothesis ID")?;
            let connection = self
                .connection
                .lock()
                .map_err(|_| "Research workspace lock is poisoned.".to_string())?;
            let hypothesis = Self::load_hypothesis(&connection, id)?
                .ok_or_else(|| "The selected research hypothesis does not exist.".to_string())?;
            if hypothesis.symbol != symbol || hypothesis.interval != request.interval {
                return Err(
                    "Research run symbol and interval must exactly match the linked hypothesis."
                        .into(),
                );
            }
            if hypothesis.status == "INVALIDATED" {
                return Err(
                    "An invalidated hypothesis cannot be used for a new research run.".into(),
                );
            }
            Some(hypothesis.revision)
        } else {
            None
        };

        let risk_limits_context = core.research_risk_limits()?;
        let backtest_request = BacktestRequest {
            symbol: symbol.clone(),
            fast_period: request.fast_period,
            slow_period: request.slow_period,
            initial_capital: request.initial_capital,
            fee_bps: risk_limits_context.fee_bps,
            slippage_bps: risk_limits_context.slippage_bps,
        };
        let (result, strategy_tournament) =
            core.run_research_backtest_bundle(backtest_request, request.strategy_tournament, 3)?;
        Self::validate_backtest_evidence(&result, request.interval, request.slow_period)?;
        if let Some(tournament) = strategy_tournament.as_ref() {
            Self::validate_strategy_tournament(
                tournament,
                &result.dataset_sha256,
                result.dataset_start_ms,
                result.dataset_end_ms,
            )?;
        }

        // Independent validation branches operate on the same immutable BacktestResult.
        // Panics are converted into hard errors; a half-built run card is never persisted.
        let (oos_review, bootstrap) = std::thread::scope(|scope| {
            let oos_job =
                scope.spawn(|| analyze_out_of_sample(&result, request.minimum_positive_oos_ratio));
            let bootstrap_job = scope.spawn(|| block_bootstrap(&result));
            let oos = oos_job
                .join()
                .map_err(|_| "Out-of-sample reviewer panicked; run aborted.".to_string())??;
            let monte_carlo = bootstrap_job
                .join()
                .map_err(|_| "Block-bootstrap reviewer panicked; run aborted.".to_string())??;
            Ok::<_, String>((oos, monte_carlo))
        })?;

        let risk_review = review_drawdown(&result, &bootstrap, request.max_drawdown_pct);
        let committee = evaluate_committee(
            &result,
            &oos_review,
            &bootstrap,
            &risk_review,
            strategy_tournament.as_ref(),
            request.max_drawdown_pct,
            request.minimum_positive_oos_ratio,
            request.minimum_sharpe_ratio,
        );
        let created_at_ms = Utc::now().timestamp_millis();
        let id = Uuid::new_v4().to_string();
        let graph = completed_workflow_graph(
            created_at_ms,
            &result,
            &oos_review,
            &bootstrap,
            &risk_review,
            strategy_tournament.as_ref(),
            &committee,
        );
        validate_workflow_graph(&graph)?;

        let data_review = SpecialistReview {
            role: "Data Steward".into(),
            status: "COMPLETED".into(),
            summary: format!("Validated {} candles from {} for {} {}.", result.candles_used, result.data_source, symbol, request.interval.as_str()),
            metrics: json!({
                "datasetSha256": result.dataset_sha256,
                "datasetEncodingVersion": result.dataset_encoding_version,
                "datasetStartMs": result.dataset_start_ms,
                "datasetEndMs": result.dataset_end_ms,
                "medianCandleIntervalMs": result.median_candle_interval_ms,
                "observations": result.candles_used,
            }),
            blockers: Vec::new(),
            caveats: vec!["A valid historical dataset does not establish that future market observations will be complete or executable.".into()],
        };
        let quant_review = SpecialistReview {
            role: "Quant Researcher".into(),
            status: "COMPLETED".into(),
            summary: format!("{} produced {:.2}% total return with {:.2}% maximum drawdown and {:.3} Sharpe over {} completed trades.",
                result.strategy_name, result.total_return_pct, result.max_drawdown_pct.abs(), result.sharpe_ratio, result.trade_count),
            metrics: json!({
                "totalReturnPct": result.total_return_pct,
                "maxDrawdownPct": result.max_drawdown_pct,
                "sharpeRatio": result.sharpe_ratio,
                "tradeCount": result.trade_count,
                "winRatePct": result.win_rate_pct,
                "profitFactor": result.profit_factor,
            }),
            blockers: Vec::new(),
            caveats: vec!["The deterministic engine currently tests the SMA crossover strategy; it does not infer a profitable strategy from the natural-language goal.".into()],
        };
        let tournament_robustness_passed = strategy_tournament.as_ref().is_none_or(|tournament| {
            tournament.completed_folds == tournament.requested_folds
                && tournament
                    .positive_oos_fold_ratio
                    .is_some_and(|ratio| ratio >= request.minimum_positive_oos_ratio)
                && tournament
                    .worst_oos_max_drawdown_pct
                    .is_some_and(|drawdown| drawdown <= request.max_drawdown_pct)
        });
        let robustness_review = SpecialistReview {
            role: "Independent Robustness Reviewer".into(),
            status: if oos_review.passed && bootstrap.available && tournament_robustness_passed { "PASSED".into() } else { "REVIEW_REQUIRED".into() },
            summary: format!("{} out-of-sample folds; {:.1}% were positive. Block-bootstrap {}.",
                oos_review.fold_count, oos_review.positive_fold_ratio.unwrap_or(0.0) * 100.0,
                if bootstrap.available { "completed" } else { "unavailable" }),
            metrics: json!({ "outOfSample": oos_review, "blockBootstrap": bootstrap, "strategyTournament": strategy_tournament }),
            blockers: {
                let mut blockers = oos_review.blockers.clone();
                if !bootstrap.available { blockers.push(bootstrap.unavailable_reason.clone().unwrap_or_else(|| "Block-bootstrap evidence is unavailable.".into())); }
                if let Some(tournament) = strategy_tournament.as_ref() { blockers.extend(tournament.blockers.clone()); }
                blockers
            },
            caveats: vec!["Walk-forward folds reset capital independently and do not carry positions between folds.".into(),
                "Block-bootstrap resamples historical strategy returns; it is a sensitivity distribution, not a forecast or a guarantee.".into(),
                "Nested parameter selection is restricted to each fold's training prefix; selected parameters are evaluated on the following unseen block.".into()],
        };
        let risk_specialist = SpecialistReview {
            role: "Risk Officer".into(),
            status: if risk_review.passed { "PASSED".into() } else { "BLOCKED".into() },
            summary: format!("Observed drawdown {:.2}% against a {:.2}% research ceiling; block-bootstrap 95th percentile drawdown {}.",
                result.max_drawdown_pct.abs(), request.max_drawdown_pct,
                bootstrap.max_drawdown_p95_pct.map(|value| format!("{value:.2}%")).unwrap_or_else(|| "unavailable".into())),
            metrics: json!({
                "maxAllowedDrawdownPct": request.max_drawdown_pct,
                "observedMaxDrawdownPct": result.max_drawdown_pct.abs(),
                "blockBootstrapMaxDrawdownP95Pct": bootstrap.max_drawdown_p95_pct,
                "passed": risk_review.passed,
            }),
            blockers: risk_review.blockers.clone(),
            caveats: vec!["This is a strategy research threshold, not a live account loss limit or a broker-side protection.".into()],
        };
        let committee_review = SpecialistReview {
            role: "Investment Committee".into(),
            status: if committee.eligible_for_paper_validation {
                "ELIGIBLE_FOR_PAPER_VALIDATION".into()
            } else {
                "NO_GO".into()
            },
            summary: if committee.eligible_for_paper_validation {
                "Research gates passed. The strategy may proceed to a separately supervised paper-validation phase; no execution authority is granted.".into()
            } else {
                "One or more research quality gates failed. Do not promote this result; inspect blockers and revise the hypothesis or strategy.".into()
            },
            metrics: json!({ "policyVersion": committee.policy_version, "blockerCount": committee.blockers.len() }),
            blockers: committee.blockers.clone(),
            caveats: committee.caveats.clone(),
        };

        let card = ResearchRunCard {
            schema_version: 1,
            id: id.clone(),
            hypothesis_id: request.hypothesis_id.clone(),
            hypothesis_revision,
            goal: request.goal.trim().to_string(),
            symbol,
            interval: request.interval,
            created_at_ms,
            risk_limits_context,
            gate_parameters: ResearchGateParameters {
                maximum_drawdown_pct: request.max_drawdown_pct,
                minimum_positive_oos_ratio: request.minimum_positive_oos_ratio,
                minimum_sharpe_ratio: request.minimum_sharpe_ratio,
            },
            workflow_graph: graph,
            backtest: ResearchBacktestEvidence::from(&result),
            out_of_sample_review: oos_review,
            block_bootstrap: bootstrap,
            strategy_tournament,
            specialist_reviews: vec![
                data_review,
                quant_review,
                robustness_review,
                risk_specialist,
                committee_review,
            ],
            decision: committee,
        };
        self.persist_run_card(&card, hypothesis_revision)
    }

    fn validate_backtest_evidence(
        result: &BacktestResult,
        interval: CandleInterval,
        slow_period: usize,
    ) -> Result<(), String> {
        if result.candles_used < slow_period.saturating_add(3)
            || result.dataset_encoding_version != 1
            || !valid_sha256(&result.dataset_sha256)
            || result.dataset_start_ms <= 0
            || result.dataset_end_ms <= result.dataset_start_ms
            || result.median_candle_interval_ms != interval.duration_ms()
            || result.data_source.trim().is_empty()
            || result.data_source.to_ascii_lowercase().contains("fixture")
            || result
                .data_source
                .to_ascii_lowercase()
                .contains("synthetic")
        {
            return Err(
                "Backtest evidence failed the research data-provenance or timeframe contract."
                    .into(),
            );
        }
        let metrics = [
            result.initial_capital,
            result.final_equity,
            result.total_return_pct,
            result.max_drawdown_pct,
            result.gross_profit,
            result.gross_loss,
            result.sharpe_ratio,
            result.fee_bps,
            result.slippage_bps,
        ];
        if metrics.iter().any(|value| !value.is_finite())
            || result
                .win_rate_pct
                .is_some_and(|value| !value.is_finite() || !(0.0..=100.0).contains(&value))
            || result
                .profit_factor
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || result.cost_stress_scenarios.iter().any(|scenario| {
                !scenario.fee_bps.is_finite()
                    || !scenario.slippage_bps.is_finite()
                    || !scenario.final_equity.is_finite()
                    || !scenario.total_return_pct.is_finite()
                    || !scenario.return_delta_percentage_points.is_finite()
                    || !scenario.max_drawdown_pct.is_finite()
                    || !scenario.sharpe_ratio.is_finite()
            })
            || result.initial_capital <= 0.0
            || result.final_equity <= 0.0
        {
            return Err("Backtest result contains non-finite or non-positive account metrics; run card was not created.".into());
        }
        Ok(())
    }

    fn persist_run_card(
        &self,
        card: &ResearchRunCard,
        hypothesis_revision: Option<i64>,
    ) -> Result<ResearchRunCardRecord, String> {
        let card_json = serde_json::to_string(card)
            .map_err(|error| format!("Cannot serialize immutable research run card: {error}"))?;
        if card_json.len() > MAX_RUN_CARD_BYTES {
            return Err("Research run card exceeds the 1 MB persistence bound.".into());
        }
        let content_sha256 = sha256_hex(card_json.as_bytes());
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| "Research workspace lock is poisoned.".to_string())?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("Cannot begin research run-card transaction: {error}"))?;
        Self::verify_connection(&transaction)?;
        let persisted_card_count: i64 = transaction
            .query_row("SELECT COUNT(*) FROM research_run_cards", [], |row| {
                row.get(0)
            })
            .map_err(|error| format!("Cannot enforce immutable run-card capacity: {error}"))?;
        if persisted_card_count >= MAX_RUN_CARDS as i64 {
            return Err(format!("Research workspace is limited to {MAX_RUN_CARDS} immutable run cards. Export and archive the workspace before creating a new workspace for further research."));
        }
        if let Some(hypothesis_id) = card.hypothesis_id.as_deref() {
            let revision = hypothesis_revision
                .ok_or_else(|| "Linked hypothesis revision is missing.".to_string())?;
            let next_status = if card.decision.eligible_for_paper_validation {
                "PAPER_VALIDATION_ELIGIBLE"
            } else if card.decision.status == "INCONCLUSIVE" {
                "INCONCLUSIVE"
            } else {
                "RESEARCH_FAILED"
            };
            let updated = transaction
                .execute(
                    "UPDATE research_hypotheses SET status=?1,revision=revision+1,updated_at_ms=?2
                 WHERE id=?3 AND revision=?4 AND status NOT IN ('INVALIDATED')",
                    params![next_status, card.created_at_ms, hypothesis_id, revision],
                )
                .map_err(|error| format!("Cannot advance linked hypothesis lifecycle: {error}"))?;
            if updated != 1 {
                return Err("Hypothesis changed while this run was executing; no run card was committed. Reload the hypothesis and rerun.".into());
            }
            Self::append_event(
                &transaction,
                card.created_at_ms,
                hypothesis_id,
                "HYPOTHESIS_RESEARCH_REVIEWED",
                &json!({
                    "runCardId": card.id, "status": next_status, "fromRevision": revision,
                }),
            )?;
        }
        transaction.execute(
            "INSERT INTO research_run_cards(id,hypothesis_id,created_at_ms,card_json,card_sha256)
             VALUES(?1,?2,?3,?4,?5)",
            params![card.id, card.hypothesis_id, card.created_at_ms, card_json, content_sha256],
        ).map_err(|error| format!("Cannot persist immutable research run card: {error}"))?;
        Self::append_event(
            &transaction,
            card.created_at_ms,
            &card.id,
            "RESEARCH_RUN_COMPLETED",
            &json!({
                "hypothesisId": card.hypothesis_id, "symbol": card.symbol,
                "datasetSha256": card.backtest.dataset_sha256,
                "decision": card.decision.status,
                "contentSha256": content_sha256,
            }),
        )?;
        transaction
            .commit()
            .map_err(|error| format!("Cannot commit research run-card transaction: {error}"))?;
        Ok(ResearchRunCardRecord {
            card: card.clone(),
            content_sha256,
        })
    }

    pub fn list_run_cards(
        &self,
        requested_limit: usize,
    ) -> Result<Vec<ResearchRunCardRecord>, String> {
        let limit = requested_limit.clamp(1, MAX_RUN_CARDS) as i64;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Research workspace lock is poisoned.".to_string())?;
        let mut statement = connection.prepare(
            "SELECT id,card_json,card_sha256 FROM research_run_cards ORDER BY created_at_ms DESC,id DESC LIMIT ?1",
        ).map_err(|error| format!("Cannot prepare research run-card listing: {error}"))?;
        let rows = statement
            .query_map([limit], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| format!("Cannot query research run cards: {error}"))?;
        let mut cards = Vec::new();
        for row in rows {
            let (id, card_json, content_sha256) =
                row.map_err(|error| format!("Cannot decode research run card: {error}"))?;
            if card_json.len() > MAX_RUN_CARD_BYTES
                || sha256_hex(card_json.as_bytes()) != content_sha256
            {
                return Err(format!(
                    "Research run card {id} failed its persisted content-hash check."
                ));
            }
            let card: ResearchRunCard = serde_json::from_str(&card_json)
                .map_err(|error| format!("Research run card {id} could not be decoded: {error}"))?;
            Self::validate_run_card_semantics(&card, &id)?;
            cards.push(ResearchRunCardRecord {
                card,
                content_sha256,
            });
        }
        Ok(cards)
    }

    pub fn get_run_card(&self, run_id: String) -> Result<ResearchRunCardRecord, String> {
        Self::validate_identifier(&run_id, "Run-card ID")?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Research workspace lock is poisoned.".to_string())?;
        let (card_json, content_sha256): (String, String) = connection
            .query_row(
                "SELECT card_json,card_sha256 FROM research_run_cards WHERE id=?1",
                [run_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|error| format!("Research run card is unavailable: {error}"))?;
        if card_json.len() > MAX_RUN_CARD_BYTES
            || sha256_hex(card_json.as_bytes()) != content_sha256
        {
            return Err("Research run card failed its content-integrity check.".into());
        }
        let card: ResearchRunCard = serde_json::from_str(&card_json)
            .map_err(|error| format!("Research run card is malformed: {error}"))?;
        Self::validate_run_card_semantics(&card, &run_id)?;
        Ok(ResearchRunCardRecord {
            card,
            content_sha256,
        })
    }

    pub fn verify_integrity(&self) -> Result<ResearchIntegrityStatus, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "Research workspace lock is poisoned.".to_string())?;
        let (event_count, run_card_count, chain_head_sha256) =
            Self::verify_connection(&connection)?;
        Ok(ResearchIntegrityStatus {
            valid: true,
            event_count,
            run_card_count,
            chain_head_sha256,
            verified_at_ms: Utc::now().timestamp_millis(),
        })
    }
}

fn validate_run_card_graph_contract(
    nodes: &[WorkflowNode],
    created_at_ms: i64,
    tournament_present: bool,
    tournament_completed: bool,
) -> Result<(), String> {
    let mut expected: Vec<(&str, Vec<&str>)> = vec![
        ("data_integrity", vec![]),
        ("deterministic_backtest", vec!["data_integrity"]),
        ("walk_forward_review", vec!["deterministic_backtest"]),
        ("block_bootstrap", vec!["deterministic_backtest"]),
        (
            "risk_review",
            vec!["walk_forward_review", "block_bootstrap"],
        ),
    ];
    let graph_has_tournament = nodes
        .iter()
        .any(|node| node.node_id == "nested_parameter_tournament");
    if graph_has_tournament != tournament_present {
        return Err(
            "Research run-card DAG tournament node presence does not match its evidence payload."
                .into(),
        );
    }
    if tournament_present {
        expected.push(("nested_parameter_tournament", vec!["data_integrity"]));
    }
    let mut committee_dependencies = vec!["risk_review"];
    if expected
        .iter()
        .any(|(node_id, _)| *node_id == "nested_parameter_tournament")
    {
        committee_dependencies.push("nested_parameter_tournament");
    }
    expected.push(("investment_committee", committee_dependencies));

    if nodes.len() != expected.len() {
        return Err("Research run-card DAG has an unexpected node count.".into());
    }
    for (index, (node, (expected_id, expected_dependencies))) in
        nodes.iter().zip(expected.iter()).enumerate()
    {
        let actual_dependencies = node
            .depends_on
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if node.node_id.as_str() != *expected_id
            || actual_dependencies.as_slice() != expected_dependencies.as_slice()
            || node.completed_at_ms != created_at_ms
            || node.detail.trim().is_empty()
            || node.detail.len() > 4_096
        {
            return Err(format!("Research run-card DAG node {} violates its canonical identity, dependency, timestamp, or detail contract.", index + 1));
        }
        let status_is_valid = match node.node_id.as_str() {
            "data_integrity" | "deterministic_backtest" => node.status == "COMPLETED",
            "walk_forward_review" => ["PASSED", "REVIEW_REQUIRED"].contains(&node.status.as_str()),
            "block_bootstrap" => ["COMPLETED", "UNAVAILABLE"].contains(&node.status.as_str()),
            "risk_review" => ["PASSED", "BLOCKED"].contains(&node.status.as_str()),
            "nested_parameter_tournament" => {
                node.status
                    == if tournament_completed {
                        "COMPLETED"
                    } else {
                        "REVIEW_REQUIRED"
                    }
            }
            "investment_committee" => {
                ["ELIGIBLE_FOR_PAPER_VALIDATION", "NO_GO"].contains(&node.status.as_str())
            }
            _ => false,
        };
        if !status_is_valid {
            return Err(format!(
                "Research run-card DAG node {} has an unsupported status.",
                node.node_id
            ));
        }
    }
    Ok(())
}

fn validate_workflow_graph(nodes: &[WorkflowNode]) -> Result<(), String> {
    if nodes.is_empty() || nodes.len() > 32 {
        return Err("Research workflow graph must contain between 1 and 32 nodes.".into());
    }
    let mut indices = BTreeMap::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.node_id.is_empty()
            || node.node_id.len() > 64
            || indices.insert(node.node_id.as_str(), index).is_some()
        {
            return Err(
                "Research workflow graph contains an invalid or duplicate node identifier.".into(),
            );
        }
        if node.depends_on.len() > 32
            || node
                .depends_on
                .iter()
                .any(|dependency| dependency == &node.node_id)
        {
            return Err(format!(
                "Research workflow node {} has invalid dependencies.",
                node.node_id
            ));
        }
    }
    let mut indegree = vec![0_usize; nodes.len()];
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        let mut unique = BTreeSet::new();
        for dependency in &node.depends_on {
            if !unique.insert(dependency) {
                return Err(format!(
                    "Research workflow node {} repeats a dependency.",
                    node.node_id
                ));
            }
            let Some(&dependency_index) = indices.get(dependency.as_str()) else {
                return Err(format!(
                    "Research workflow node {} refers to missing dependency {}.",
                    node.node_id, dependency
                ));
            };
            indegree[index] += 1;
            outgoing[dependency_index].push(index);
        }
    }
    let mut ready: Vec<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let mut visited = 0_usize;
    while let Some(index) = ready.pop() {
        visited += 1;
        for dependent in &outgoing[index] {
            indegree[*dependent] -= 1;
            if indegree[*dependent] == 0 {
                ready.push(*dependent);
            }
        }
    }
    if visited != nodes.len() {
        return Err("Research workflow graph contains a dependency cycle.".into());
    }
    Ok(())
}

fn analyze_out_of_sample(
    result: &BacktestResult,
    minimum_positive_ratio: f64,
) -> Result<OutOfSampleReview, String> {
    let folds = &result.out_of_sample_folds;
    if folds.iter().any(|fold| {
        !fold.total_return_pct.is_finite()
            || !fold.max_drawdown_pct.is_finite()
            || fold.candles_evaluated == 0
    }) {
        return Err("Out-of-sample review received malformed fold metrics.".into());
    }
    let positive_fold_count = folds
        .iter()
        .filter(|fold| fold.total_return_pct > 0.0)
        .count();
    let mut returns: Vec<f64> = folds.iter().map(|fold| fold.total_return_pct).collect();
    returns.sort_by(f64::total_cmp);
    let median_return_pct = if returns.is_empty() {
        None
    } else {
        Some(quantile(&returns, 0.5))
    };
    let positive_fold_ratio = if folds.is_empty() {
        None
    } else {
        Some(positive_fold_count as f64 / folds.len() as f64)
    };
    let worst_drawdown_pct = folds
        .iter()
        .map(|fold| fold.max_drawdown_pct.abs())
        .max_by(f64::total_cmp);
    let mut blockers = Vec::new();
    if folds.len() < 2 {
        blockers.push("At least two independent out-of-sample folds are required.".into());
    }
    if positive_fold_ratio.is_none_or(|ratio| ratio < minimum_positive_ratio) {
        blockers.push(format!(
            "Positive out-of-sample fold ratio is below the configured {:.1}% threshold.",
            minimum_positive_ratio * 100.0
        ));
    }
    Ok(OutOfSampleReview {
        fold_count: folds.len(),
        positive_fold_count,
        positive_fold_ratio,
        median_return_pct,
        worst_drawdown_pct,
        passed: blockers.is_empty(),
        blockers,
    })
}

fn block_bootstrap(result: &BacktestResult) -> Result<BlockBootstrapSummary, String> {
    let seed_sha256 = result.dataset_sha256.clone();
    let unavailable = |observations: usize, reason: String| BlockBootstrapSummary {
        method: "moving-block bootstrap of strategy equity returns; PCG32 v1".into(),
        available: false,
        simulations: 0,
        observations,
        block_length: 0,
        seed_sha256: seed_sha256.clone(),
        return_p05_pct: None,
        return_median_pct: None,
        return_p95_pct: None,
        max_drawdown_p95_pct: None,
        unavailable_reason: Some(reason),
        caveat: "This resampling diagnostic is not a probabilistic market forecast.".into(),
    };
    if result.equity_curve.len() < 31 {
        return Ok(unavailable(
            result.equity_curve.len(),
            "At least 31 evaluated equity observations are required.".into(),
        ));
    }
    if result.equity_curve.len() > MAX_BOOTSTRAP_OBSERVATIONS {
        return Ok(unavailable(result.equity_curve.len(), format!(
            "The dataset exceeds the bounded bootstrap budget of {MAX_BOOTSTRAP_OBSERVATIONS} equity observations."
        )));
    }
    let mut returns = Vec::with_capacity(result.equity_curve.len().saturating_sub(1));
    for pair in result.equity_curve.windows(2) {
        if !pair[0].equity.is_finite()
            || pair[0].equity <= 0.0
            || !pair[1].equity.is_finite()
            || pair[1].equity <= 0.0
            || pair[1].time <= pair[0].time
        {
            return Err(
                "Block-bootstrap input contains invalid equity or timestamp observations.".into(),
            );
        }
        let observed_return = pair[1].equity / pair[0].equity - 1.0;
        if !observed_return.is_finite() || observed_return <= -1.0 {
            return Err("Block-bootstrap input contains an invalid simple return.".into());
        }
        returns.push(observed_return);
    }
    if returns.len() < 30 {
        return Ok(unavailable(
            returns.len(),
            "At least 30 contiguous strategy returns are required.".into(),
        ));
    }
    let block_length = (returns.len() as f64).sqrt().ceil() as usize;
    let block_length = block_length.clamp(2, 64).min(returns.len());
    let simulations = (BOOTSTRAP_WORK_BUDGET / returns.len() as u64).clamp(32, 500) as u16;
    let seed = u64::from_str_radix(&seed_sha256[..16], 16)
        .unwrap_or(0x9e37_79b9_7f4a_7c15)
        .max(1);
    let mut random = Pcg32::new(seed);
    let mut terminal_returns = Vec::with_capacity(simulations as usize);
    let mut maximum_drawdowns = Vec::with_capacity(simulations as usize);
    for _ in 0..simulations {
        let mut equity = 1.0_f64;
        let mut peak = 1.0_f64;
        let mut maximum_drawdown = 0.0_f64;
        let mut written = 0_usize;
        while written < returns.len() {
            let max_start = returns.len() - block_length;
            let start = random.index(max_start.saturating_add(1));
            let count = block_length.min(returns.len() - written);
            for offset in 0..count {
                equity *= 1.0 + returns[start + offset];
                if !equity.is_finite() || equity <= 0.0 {
                    return Err("Block-bootstrap path exceeded the supported equity range.".into());
                }
                peak = peak.max(equity);
                let drawdown = (peak - equity) / peak * 100.0;
                maximum_drawdown = maximum_drawdown.max(drawdown);
            }
            written += count;
        }
        terminal_returns.push((equity - 1.0) * 100.0);
        maximum_drawdowns.push(maximum_drawdown);
    }
    terminal_returns.sort_by(f64::total_cmp);
    maximum_drawdowns.sort_by(f64::total_cmp);
    Ok(BlockBootstrapSummary {
        method: "moving-block bootstrap of strategy equity returns; PCG32 v1".into(),
        available: true,
        simulations,
        observations: returns.len(),
        block_length,
        seed_sha256,
        return_p05_pct: Some(quantile(&terminal_returns, 0.05)),
        return_median_pct: Some(quantile(&terminal_returns, 0.50)),
        return_p95_pct: Some(quantile(&terminal_returns, 0.95)),
        max_drawdown_p95_pct: Some(quantile(&maximum_drawdowns, 0.95)),
        unavailable_reason: None,
        caveat: "Historical strategy returns are resampled in contiguous blocks. The output is a deterministic sensitivity distribution conditional on this sample, not a forward return forecast, confidence guarantee, or liquidity model.".into(),
    })
}

/// PCG-XSH-RR 64/32 with a fixed stream and rejection-sampled bounded indices.
/// This is deterministic statistical sampling, not a cryptographic random source.
struct Pcg32 {
    state: u64,
    increment: u64,
}

impl Pcg32 {
    fn new(seed: u64) -> Self {
        let mut generator = Self {
            state: 0,
            increment: 0xda3e_39cb_94b9_5bdb | 1,
        };
        let _ = generator.next_u32();
        generator.state = generator.state.wrapping_add(seed);
        let _ = generator.next_u32();
        generator
    }

    fn next_u32(&mut self) -> u32 {
        let previous = self.state;
        self.state = previous
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(self.increment);
        let shifted = (((previous >> 18) ^ previous) >> 27) as u32;
        let rotation = (previous >> 59) as u32;
        shifted.rotate_right(rotation)
    }

    fn next_u64(&mut self) -> u64 {
        (u64::from(self.next_u32()) << 32) | u64::from(self.next_u32())
    }

    fn index(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive <= 1 {
            return 0;
        }
        let bound = upper_exclusive as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let sample = self.next_u64();
            if sample >= threshold {
                return (sample % bound) as usize;
            }
        }
    }
}

fn quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = position - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

#[derive(Debug)]
struct RiskReviewResult {
    passed: bool,
    blockers: Vec<String>,
}

fn review_drawdown(
    result: &BacktestResult,
    bootstrap: &BlockBootstrapSummary,
    maximum_drawdown_pct: f64,
) -> RiskReviewResult {
    let observed_drawdown = result.max_drawdown_pct.abs();
    let mut blockers = Vec::new();
    if observed_drawdown > maximum_drawdown_pct {
        blockers.push(format!(
            "Observed maximum drawdown {:.2}% exceeds the {:.2}% research ceiling.",
            observed_drawdown, maximum_drawdown_pct
        ));
    }
    match bootstrap.max_drawdown_p95_pct {
        Some(value) if value > maximum_drawdown_pct => blockers.push(format!(
            "Block-bootstrap 95th-percentile path drawdown {:.2}% exceeds the {:.2}% research ceiling.", value, maximum_drawdown_pct
        )),
        None => blockers.push("Block-bootstrap drawdown evidence is unavailable; risk review cannot pass.".into()),
        _ => {}
    }
    RiskReviewResult {
        passed: blockers.is_empty(),
        blockers,
    }
}

fn evaluate_committee(
    result: &BacktestResult,
    oos: &OutOfSampleReview,
    bootstrap: &BlockBootstrapSummary,
    risk: &RiskReviewResult,
    tournament: Option<&StrategyParameterTournament>,
    maximum_drawdown_pct: f64,
    minimum_positive_oos_ratio: f64,
    minimum_sharpe_ratio: f64,
) -> ResearchGateDecision {
    let mut blockers = Vec::new();
    if result.trade_count < 3 {
        blockers.push(
            "At least three completed strategy trades are required for this research gate.".into(),
        );
    }
    if !oos.passed {
        blockers.extend(oos.blockers.clone());
    }
    if !bootstrap.available {
        blockers.push(
            bootstrap
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "Block-bootstrap evidence is unavailable.".into()),
        );
    }
    if !risk.passed {
        blockers.extend(risk.blockers.clone());
    }
    if let Some(tournament) = tournament {
        if tournament.completed_folds != tournament.requested_folds {
            blockers.push(format!(
                "Nested walk-forward tournament completed only {}/{} folds.",
                tournament.completed_folds, tournament.requested_folds
            ));
        }
        if tournament
            .positive_oos_fold_ratio
            .is_none_or(|ratio| ratio < minimum_positive_oos_ratio)
        {
            blockers.push(format!(
                "Nested walk-forward positive OOS ratio is below the configured {:.1}% threshold.",
                minimum_positive_oos_ratio * 100.0
            ));
        }
        if tournament
            .worst_oos_max_drawdown_pct
            .is_none_or(|drawdown| drawdown > maximum_drawdown_pct)
        {
            blockers.push(format!("Nested walk-forward worst OOS drawdown exceeds the configured {:.2}% research ceiling or is unavailable.", maximum_drawdown_pct));
        }
    }
    if result.sharpe_ratio < minimum_sharpe_ratio {
        blockers.push(format!(
            "Observed Sharpe {:.3} is below the configured {:.3} minimum.",
            result.sharpe_ratio, minimum_sharpe_ratio
        ));
    }
    if let Some(severe) = result
        .cost_stress_scenarios
        .iter()
        .find(|scenario| scenario.name.to_ascii_lowercase().contains("severe"))
    {
        if severe.total_return_pct < 0.0 {
            blockers.push(format!(
                "Severe cost-stress return is negative ({:.2}%).",
                severe.total_return_pct
            ));
        }
    } else {
        blockers
            .push("The backtest did not produce the required severe cost-stress scenario.".into());
    }
    blockers.sort();
    blockers.dedup();
    let eligible = blockers.is_empty();
    let tournament_failed = tournament.is_some_and(|review| {
        review.completed_folds != review.requested_folds
            || review
                .positive_oos_fold_ratio
                .is_none_or(|ratio| ratio < minimum_positive_oos_ratio)
            || review
                .worst_oos_max_drawdown_pct
                .is_none_or(|drawdown| drawdown > maximum_drawdown_pct)
    });
    let status = if eligible {
        "ELIGIBLE_FOR_PAPER_VALIDATION"
    } else if bootstrap.available && (!oos.passed || tournament_failed) {
        "RESEARCH_FAILED"
    } else {
        "INCONCLUSIVE"
    };
    ResearchGateDecision {
        policy_version: if tournament.is_some() { "atos-research-gate-v2-nested-walk-forward".into() } else { RESEARCH_GATE_VERSION.into() },
        status: status.into(),
        eligible_for_paper_validation: eligible,
        blockers,
        caveats: vec![
            "Passing this research gate is not a trade signal, investment recommendation, or permission to submit orders.".into(),
            "Paper validation is a separate supervised phase with its own account-level Rust risk gateway and kill switch.".into(),
            "Historical backtests, walk-forward folds, and resampled return paths do not guarantee future performance.".into(),
        ],
    }
}

fn completed_workflow_graph(
    completed_at_ms: i64,
    result: &BacktestResult,
    oos: &OutOfSampleReview,
    bootstrap: &BlockBootstrapSummary,
    risk: &RiskReviewResult,
    tournament: Option<&StrategyParameterTournament>,
    committee: &ResearchGateDecision,
) -> Vec<WorkflowNode> {
    let mut nodes = vec![
        WorkflowNode {
            node_id: "data_integrity".into(),
            depends_on: vec![],
            status: "COMPLETED".into(),
            detail: format!(
                "{} candles, SHA-256 {}.",
                result.candles_used, result.dataset_sha256
            ),
            completed_at_ms,
        },
        WorkflowNode {
            node_id: "deterministic_backtest".into(),
            depends_on: vec!["data_integrity".into()],
            status: "COMPLETED".into(),
            detail: format!("{} strategy trades evaluated.", result.trade_count),
            completed_at_ms,
        },
        WorkflowNode {
            node_id: "walk_forward_review".into(),
            depends_on: vec!["deterministic_backtest".into()],
            status: if oos.passed {
                "PASSED".into()
            } else {
                "REVIEW_REQUIRED".into()
            },
            detail: format!(
                "{} independent chronological folds evaluated.",
                oos.fold_count
            ),
            completed_at_ms,
        },
        WorkflowNode {
            node_id: "block_bootstrap".into(),
            depends_on: vec!["deterministic_backtest".into()],
            status: if bootstrap.available {
                "COMPLETED".into()
            } else {
                "UNAVAILABLE".into()
            },
            detail: bootstrap.unavailable_reason.clone().unwrap_or_else(|| {
                format!("{} block-resampled paths evaluated.", bootstrap.simulations)
            }),
            completed_at_ms,
        },
        WorkflowNode {
            node_id: "risk_review".into(),
            depends_on: vec!["walk_forward_review".into(), "block_bootstrap".into()],
            status: if risk.passed {
                "PASSED".into()
            } else {
                "BLOCKED".into()
            },
            detail: if risk.blockers.is_empty() {
                "Observed and resampled drawdown passed the configured research ceiling.".into()
            } else {
                risk.blockers.join(" ")
            },
            completed_at_ms,
        },
    ];
    if let Some(tournament) = tournament {
        nodes.push(WorkflowNode {
            node_id: "nested_parameter_tournament".into(),
            depends_on: vec!["data_integrity".into()],
            status: if tournament.completed_folds == tournament.requested_folds {
                "COMPLETED".into()
            } else {
                "REVIEW_REQUIRED".into()
            },
            detail: format!(
                "{} candidate parameters across {} outer folds; {:.1}% positive OOS folds.",
                tournament.candidate_count,
                tournament.requested_folds,
                tournament.positive_oos_fold_ratio.unwrap_or(0.0) * 100.0
            ),
            completed_at_ms,
        });
    }
    let mut committee_dependencies = vec!["risk_review".into()];
    if tournament.is_some() {
        committee_dependencies.push("nested_parameter_tournament".into());
    }
    nodes.push(WorkflowNode {
        node_id: "investment_committee".into(),
        depends_on: committee_dependencies,
        status: if committee.eligible_for_paper_validation {
            "ELIGIBLE_FOR_PAPER_VALIDATION".into()
        } else {
            "NO_GO".into()
        },
        detail: format!("{} research blocker(s).", committee.blockers.len()),
        completed_at_ms,
    });
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, dependencies: &[&str]) -> WorkflowNode {
        WorkflowNode {
            node_id: id.into(),
            depends_on: dependencies.iter().map(|value| (*value).into()).collect(),
            status: "COMPLETED".into(),
            detail: "test node".into(),
            completed_at_ms: 1,
        }
    }

    #[test]
    fn workflow_graph_accepts_dag_and_rejects_cycles_missing_nodes_and_duplicate_edges() {
        assert!(validate_workflow_graph(&[
            node("a", &[]),
            node("b", &["a"]),
            node("c", &["a"]),
            node("d", &["b", "c"])
        ])
        .is_ok());
        assert!(
            validate_workflow_graph(&[node("a", &["b"]), node("b", &["a"])])
                .unwrap_err()
                .contains("cycle")
        );
        assert!(validate_workflow_graph(&[node("a", &["missing"])])
            .unwrap_err()
            .contains("missing dependency"));
        assert!(
            validate_workflow_graph(&[node("a", &[]), node("b", &["a", "a"])])
                .unwrap_err()
                .contains("repeats")
        );
    }

    #[test]
    fn run_card_graph_contract_rejects_missing_or_rewired_dependencies() {
        let mut nodes = vec![
            node("data_integrity", &[]),
            node("deterministic_backtest", &["data_integrity"]),
            node("walk_forward_review", &["deterministic_backtest"]),
            node("block_bootstrap", &["deterministic_backtest"]),
            node("risk_review", &["walk_forward_review", "block_bootstrap"]),
            node("nested_parameter_tournament", &["data_integrity"]),
            node(
                "investment_committee",
                &["risk_review", "nested_parameter_tournament"],
            ),
        ];
        for item in &mut nodes {
            item.completed_at_ms = 10;
        }
        nodes[2].status = "PASSED".into();
        nodes[3].status = "COMPLETED".into();
        nodes[4].status = "PASSED".into();
        nodes[5].status = "COMPLETED".into();
        nodes[6].status = "NO_GO".into();
        assert!(validate_workflow_graph(&nodes).is_ok());
        assert!(validate_run_card_graph_contract(&nodes, 10, true, true).is_ok());

        let mut missing_edge = nodes.clone();
        missing_edge[6].depends_on = vec!["risk_review".into()];
        assert!(validate_workflow_graph(&missing_edge).is_ok());
        assert!(validate_run_card_graph_contract(&missing_edge, 10, true, true).is_err());

        let mut wrong_timestamp = nodes.clone();
        wrong_timestamp[2].completed_at_ms = 11;
        assert!(validate_run_card_graph_contract(&wrong_timestamp, 10, true, true).is_err());

        let mut incomplete_tournament = nodes.clone();
        incomplete_tournament[5].status = "REVIEW_REQUIRED".into();
        assert!(validate_run_card_graph_contract(&incomplete_tournament, 10, true, false).is_ok());
        let without_tournament = vec![
            node("data_integrity", &[]),
            node("deterministic_backtest", &["data_integrity"]),
            node("walk_forward_review", &["deterministic_backtest"]),
            node("block_bootstrap", &["deterministic_backtest"]),
            node("risk_review", &["walk_forward_review", "block_bootstrap"]),
            node("investment_committee", &["risk_review"]),
        ];
        let mut without_tournament = without_tournament;
        for item in &mut without_tournament {
            item.completed_at_ms = 10;
        }
        without_tournament[2].status = "PASSED".into();
        without_tournament[4].status = "PASSED".into();
        without_tournament[5].status = "NO_GO".into();
        assert!(validate_run_card_graph_contract(&without_tournament, 10, false, false).is_ok());
        assert!(validate_run_card_graph_contract(&nodes, 10, false, true).is_err());
    }

    #[test]
    fn pcg32_is_reproducible_and_bounded_sampling_stays_in_range() {
        let mut first = Pcg32::new(0x1234_5678);
        let mut second = Pcg32::new(0x1234_5678);
        let mut different_seed = Pcg32::new(0x1234_5679);
        let sequence_a: Vec<u32> = (0..100).map(|_| first.next_u32()).collect();
        let sequence_b: Vec<u32> = (0..100).map(|_| second.next_u32()).collect();
        let sequence_c: Vec<u32> = (0..100).map(|_| different_seed.next_u32()).collect();
        assert_eq!(sequence_a, sequence_b);
        assert_ne!(sequence_a, sequence_c);

        let mut bounded = Pcg32::new(0xfeed_beef);
        for _ in 0..10_000 {
            assert!(bounded.index(97) < 97);
            assert!(bounded.index(50_003) < 50_003);
        }
    }

    #[test]
    fn quantile_interpolates_and_handles_single_observation() {
        assert_eq!(quantile(&[10.0], 0.95), 10.0);
        assert_eq!(quantile(&[0.0, 10.0, 20.0], 0.5), 10.0);
        assert_eq!(quantile(&[0.0, 10.0], 0.25), 2.5);
    }

    #[test]
    fn symbol_and_text_contract_rejects_controls_and_ambiguous_symbols() {
        assert_eq!(validate_symbol(" btcusdt ").unwrap(), "BTCUSDT");
        assert!(validate_symbol("BTC/USDT").is_err());
        assert!(validate_text("\u{0007}title", "title", 10, false).is_err());
        assert!(validate_text(" ", "title", 10, false).is_err());
    }

    fn temporary_workspace_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("atos-research-{}.sqlite3", Uuid::new_v4()))
    }

    fn remove_workspace_files(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = std::fs::remove_file(path.with_extension("sqlite3-shm"));
    }

    #[test]
    fn hypothesis_lifecycle_is_durable_revision_bound_and_hash_chained() {
        let path = temporary_workspace_path();
        let workspace = ResearchWorkspace::open(&path).unwrap();
        let created = workspace
            .create_hypothesis(CreateResearchHypothesisRequest {
                title: "Trend persistence".into(),
                thesis: "Price continuation should persist after a volatility contraction.".into(),
                symbol: "btcusdt".into(),
                interval: CandleInterval::FiveMinutes,
            })
            .unwrap();
        assert_eq!(created.status, "DRAFT");
        assert_eq!(created.revision, 1);
        assert_eq!(workspace.verify_integrity().unwrap().event_count, 1);

        let invalidated = workspace
            .invalidate_hypothesis(created.id.clone(), "The premise was falsified.".into(), 1)
            .unwrap();
        assert_eq!(invalidated.status, "INVALIDATED");
        assert_eq!(invalidated.revision, 2);
        assert_eq!(workspace.verify_integrity().unwrap().event_count, 2);
        assert!(workspace
            .invalidate_hypothesis(created.id.clone(), "Stale revision".into(), 1)
            .is_err());
        drop(workspace);

        let reopened = ResearchWorkspace::open(&path).unwrap();
        assert_eq!(
            reopened.list_hypotheses(10).unwrap()[0].status,
            "INVALIDATED"
        );
        assert!(reopened.verify_integrity().unwrap().valid);
        drop(reopened);
        remove_workspace_files(&path);
    }

    #[test]
    fn hypothesis_state_tampering_without_a_matching_event_fails_integrity_check() {
        let path = temporary_workspace_path();
        let workspace = ResearchWorkspace::open(&path).unwrap();
        let created = workspace
            .create_hypothesis(CreateResearchHypothesisRequest {
                title: "Immutable premise".into(),
                thesis: "A falsifiable statement.".into(),
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes,
            })
            .unwrap();
        let connection = Connection::open(&path).unwrap();
        connection.execute(
            "UPDATE research_hypotheses SET status='INVALIDATED',revision=revision+1 WHERE id=?1",
            [created.id.as_str()],
        ).unwrap();
        assert!(workspace.verify_integrity().is_err());
        drop(connection);
        drop(workspace);
        remove_workspace_files(&path);
    }

    #[test]
    fn weakened_immutability_trigger_fails_closed_on_reopen() {
        let path = temporary_workspace_path();
        let workspace = ResearchWorkspace::open(&path).unwrap();
        drop(workspace);
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(
            "DROP TRIGGER research_event_chain_no_update;
             CREATE TRIGGER research_event_chain_no_update BEFORE UPDATE ON research_event_chain BEGIN SELECT RAISE(IGNORE); END;"
        ).unwrap();
        drop(connection);
        let reopened = ResearchWorkspace::open(&path);
        assert!(
            matches!(reopened, Err(ref error) if error.contains("does not exactly match the required immutable-table policy"))
        );
        remove_workspace_files(&path);
    }

    #[test]
    fn missing_immutability_trigger_fails_closed_on_reopen() {
        let path = temporary_workspace_path();
        let workspace = ResearchWorkspace::open(&path).unwrap();
        drop(workspace);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute("DROP TRIGGER research_event_chain_no_update", [])
            .unwrap();
        drop(connection);
        let reopened = ResearchWorkspace::open(&path);
        assert!(
            matches!(reopened, Err(ref error) if error.contains("guard research_event_chain_no_update is missing"))
        );
        remove_workspace_files(&path);
    }
}
