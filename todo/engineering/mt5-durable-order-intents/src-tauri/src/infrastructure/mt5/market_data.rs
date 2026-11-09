//! Durable MT5 tick and closed-bar ingestion.
//!
//! The store is deliberately separate from the paper-account ledger and research workspace.
//! A frame receipt, tick inserts, and bar revisions commit in one SQLite transaction so an
//! ACK never advertises data that was not durably written.

use super::risk_decision_ledger;
pub use super::risk_decision_ledger::{
    Mt5RiskDecisionLedgerStatus, Mt5RiskDecisionReceipt, Mt5RiskDecisionRecord,
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

pub const MAX_TICKS_PER_FRAME: usize = 8_192;
pub const MAX_BARS_PER_FRAME: usize = 8_192;
pub const MAX_SYMBOLS_PER_FRAME: usize = 32;
const MAX_SAFE_JSON_INTEGER: i64 = 9_007_199_254_740_991;
const MAX_SESSION_CALENDAR_AGE_MS: i64 = 3_660_000;
const MAX_CLOCK_SKEW_MS: i64 = 60_000;
const MARKET_DATA_SCHEMA_VERSION: i64 = 3;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5MarketDataBatch {
    #[serde(default)]
    pub ticks: Vec<Mt5Tick>,
    #[serde(default)]
    pub bars: Vec<Mt5Bar>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5Tick {
    pub symbol: String,
    pub time_ms: i64,
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub volume: u64,
    pub volume_real: f64,
    pub flags: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5Bar {
    pub symbol: String,
    pub timeframe: String,
    pub time_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub tick_volume: u64,
    pub spread: i64,
    pub real_volume: u64,
    pub closed: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5SeriesQuery {
    pub broker_server: String,
    pub account_login: String,
    pub symbol: String,
    pub timeframe: String,
    pub from_ms: i64,
    pub to_ms: i64,
    #[serde(default)]
    pub after_ms: Option<i64>,
    pub limit: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5InstrumentSpecification {
    pub symbol: String,
    pub digits: i64,
    pub point: f64,
    pub contract_size: f64,
    pub calculation_mode: i64,
    pub trade_mode: i64,
    pub execution_mode: i64,
    pub filling_mode: i64,
    pub order_mode: i64,
    pub tick_size: f64,
    pub tick_value: f64,
    pub tick_value_profit: f64,
    pub tick_value_loss: f64,
    pub volume_min: f64,
    pub volume_max: f64,
    pub volume_step: f64,
    pub volume_limit: f64,
    pub stops_level_points: i64,
    pub freeze_level_points: i64,
    pub currency_base: String,
    pub currency_profit: String,
    pub currency_margin: String,
    pub swap_mode: i64,
    pub swap_long: f64,
    pub swap_short: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5InstrumentSpecificationRecord {
    pub broker_server: String,
    pub account_login: String,
    pub specification: Mt5InstrumentSpecification,
    pub fingerprint_sha256: String,
    pub revision: u64,
    pub first_observed_at_ms: i64,
    pub last_observed_at_ms: i64,
    pub changed_at_ms: Option<i64>,
    pub completeness: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5SessionWindow {
    pub index: u16,
    pub from_seconds: u32,
    pub to_seconds: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5SessionDay {
    /// MQL5 ENUM_DAY_OF_WEEK: Sunday=0 through Saturday=6.
    pub day_of_week: u8,
    pub sessions: Vec<Mt5SessionWindow>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5SessionSchedule {
    pub symbol: String,
    /// UTC capture instant, distinct from the broker-server wall-clock session boundaries.
    pub captured_at_ms: i64,
    /// Exactly seven broker-server wall-clock weekdays, including empty days.
    pub days: Vec<Mt5SessionDay>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5SessionCalendarRecord {
    pub broker_server: String,
    pub account_login: String,
    pub schedule: Mt5SessionSchedule,
    pub fingerprint_sha256: String,
    pub revision: u64,
    pub first_observed_at_ms: i64,
    pub last_observed_at_ms: i64,
    pub changed_at_ms: Option<i64>,
    pub time_basis: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5StoredBar {
    pub symbol: String,
    pub timeframe: String,
    pub time_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub tick_volume: u64,
    pub spread: i64,
    pub real_volume: u64,
    pub payload_sha256: String,
    pub revision: u64,
    pub first_received_at_ms: i64,
    pub last_received_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5SeriesGapCandidate {
    pub previous_bar_time_ms: i64,
    pub next_bar_time_ms: i64,
    pub elapsed_ms: i64,
    pub nominal_intervals_elapsed: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5SeriesQuality {
    pub bars_total: u64,
    pub first_bar_time_ms: Option<i64>,
    pub last_bar_time_ms: Option<i64>,
    pub revisions_total: u64,
    pub nominal_interval_ms: i64,
    pub interval_anomaly_count: u64,
    pub largest_interval_ms: Option<i64>,
    pub gap_candidates: Vec<Mt5SeriesGapCandidate>,
    pub continuity_interpretation: String,
    pub session_calendar_revision: Option<u64>,
    pub session_calendar_fingerprint_sha256: Option<String>,
    pub session_calendar_time_basis: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5SeriesQueryResponse {
    pub broker_server: String,
    pub account_login: String,
    pub symbol: String,
    pub timeframe: String,
    pub from_ms: i64,
    pub to_ms: i64,
    pub dataset_fingerprint_sha256: String,
    pub bars: Vec<Mt5StoredBar>,
    pub quality: Mt5SeriesQuality,
    pub has_more: bool,
    pub next_after_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5MarketDataStatus {
    pub database_path: String,
    pub ticks_total: u64,
    pub bars_total: u64,
    pub symbols_total: u64,
    pub last_ingest_at_ms: Option<i64>,
    pub storage_available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5SynchronizationCursors {
    pub tick_cursors: HashMap<String, i64>,
    pub bar_cursors: HashMap<String, i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestReceipt {
    pub inserted_ticks: usize,
    pub inserted_bars: usize,
    pub revised_bars: usize,
    pub duplicate_frame: bool,
}

#[derive(Debug, Clone)]
pub struct Mt5MarketDataStore {
    path: PathBuf,
}

impl Mt5MarketDataStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, String> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("MT5 market-data directory could not be created: {error}")
            })?;
        }
        let store = Self { path };
        let connection = store.open_connection()?;
        let schema_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| {
                format!("MT5 market-data schema version could not be read: {error}")
            })?;
        if schema_version > MARKET_DATA_SCHEMA_VERSION {
            return Err(format!("MT5 market-data database schema version {schema_version} is newer than this application supports ({MARKET_DATA_SCHEMA_VERSION}); refusing to open it."));
        }
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS mt5_market_data_stats (
                 singleton_id INTEGER PRIMARY KEY CHECK(singleton_id = 1),
                 ticks_total INTEGER NOT NULL DEFAULT 0 CHECK(ticks_total >= 0),
                 bars_total INTEGER NOT NULL DEFAULT 0 CHECK(bars_total >= 0),
                 last_ingest_at_ms INTEGER
             );
             INSERT OR IGNORE INTO mt5_market_data_stats(singleton_id) VALUES (1);
             CREATE TABLE IF NOT EXISTS mt5_symbols (
                 broker_server TEXT NOT NULL,
                 account_login TEXT NOT NULL,
                 symbol TEXT NOT NULL,
                 first_seen_at_ms INTEGER NOT NULL,
                 last_seen_at_ms INTEGER NOT NULL,
                 PRIMARY KEY(broker_server, account_login, symbol)
             ) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS mt5_frame_receipts (
                 client_id TEXT NOT NULL,
                 session_id TEXT NOT NULL,
                 sequence INTEGER NOT NULL CHECK(sequence > 0),
                 frame_sha256 TEXT NOT NULL CHECK(length(frame_sha256) = 64),
                 committed_at_ms INTEGER NOT NULL,
                 PRIMARY KEY(client_id, session_id, sequence)
             ) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS mt5_ticks (
                 fingerprint TEXT PRIMARY KEY CHECK(length(fingerprint) = 64),
                 account_login TEXT NOT NULL,
                 broker_server TEXT NOT NULL,
                 symbol TEXT NOT NULL,
                 time_ms INTEGER NOT NULL,
                 bid REAL NOT NULL,
                 ask REAL NOT NULL,
                 last REAL NOT NULL,
                 volume INTEGER NOT NULL,
                 volume_real REAL NOT NULL,
                 flags INTEGER NOT NULL,
                 received_at_ms INTEGER NOT NULL
             ) WITHOUT ROWID;
             CREATE INDEX IF NOT EXISTS idx_mt5_ticks_series
                 ON mt5_ticks(broker_server, account_login, symbol, time_ms);
             CREATE TABLE IF NOT EXISTS mt5_bars (
                 broker_server TEXT NOT NULL,
                 account_login TEXT NOT NULL,
                 symbol TEXT NOT NULL,
                 timeframe TEXT NOT NULL,
                 time_ms INTEGER NOT NULL,
                 open REAL NOT NULL,
                 high REAL NOT NULL,
                 low REAL NOT NULL,
                 close REAL NOT NULL,
                 tick_volume INTEGER NOT NULL,
                 spread INTEGER NOT NULL,
                 real_volume INTEGER NOT NULL,
                 payload_sha256 TEXT NOT NULL CHECK(length(payload_sha256) = 64),
                 revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0),
                 first_received_at_ms INTEGER NOT NULL,
                 last_received_at_ms INTEGER NOT NULL,
                 PRIMARY KEY(broker_server, account_login, symbol, timeframe, time_ms)
             ) WITHOUT ROWID;
             CREATE INDEX IF NOT EXISTS idx_mt5_bars_series
                 ON mt5_bars(broker_server, account_login, symbol, timeframe, time_ms);
             CREATE TABLE IF NOT EXISTS mt5_bar_revisions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 broker_server TEXT NOT NULL,
                 account_login TEXT NOT NULL,
                 symbol TEXT NOT NULL,
                 timeframe TEXT NOT NULL,
                 time_ms INTEGER NOT NULL,
                 previous_sha256 TEXT NOT NULL,
                 replacement_sha256 TEXT NOT NULL,
                 observed_at_ms INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_mt5_bar_revisions_series
                 ON mt5_bar_revisions(broker_server, account_login, symbol, timeframe, time_ms);
             CREATE TABLE IF NOT EXISTS mt5_instrument_specs (
                 broker_server TEXT NOT NULL,
                 account_login TEXT NOT NULL,
                 symbol TEXT NOT NULL,
                 specification_json TEXT NOT NULL,
                 fingerprint_sha256 TEXT NOT NULL CHECK(length(fingerprint_sha256) = 64),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 first_observed_at_ms INTEGER NOT NULL,
                 last_observed_at_ms INTEGER NOT NULL,
                 changed_at_ms INTEGER,
                 PRIMARY KEY(broker_server, account_login, symbol)
             ) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS mt5_instrument_spec_revisions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 broker_server TEXT NOT NULL,
                 account_login TEXT NOT NULL,
                 symbol TEXT NOT NULL,
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 previous_sha256 TEXT,
                 replacement_sha256 TEXT NOT NULL CHECK(length(replacement_sha256) = 64),
                 specification_json TEXT NOT NULL,
                 observed_at_ms INTEGER NOT NULL,
                 UNIQUE(broker_server, account_login, symbol, revision)
             );
             CREATE INDEX IF NOT EXISTS idx_mt5_instrument_spec_history
                 ON mt5_instrument_spec_revisions(broker_server, account_login, symbol, revision DESC);
             CREATE TRIGGER IF NOT EXISTS trg_mt5_instrument_spec_revisions_no_update
             BEFORE UPDATE ON mt5_instrument_spec_revisions
             BEGIN SELECT RAISE(ABORT, 'MT5 instrument specification history is append-only'); END;
             CREATE TRIGGER IF NOT EXISTS trg_mt5_instrument_spec_revisions_no_delete
             BEFORE DELETE ON mt5_instrument_spec_revisions
             BEGIN SELECT RAISE(ABORT, 'MT5 instrument specification history is append-only'); END;
             CREATE TABLE IF NOT EXISTS mt5_session_calendars (
                 broker_server TEXT NOT NULL,
                 account_login TEXT NOT NULL,
                 symbol TEXT NOT NULL,
                 schedule_json TEXT NOT NULL,
                 fingerprint_sha256 TEXT NOT NULL CHECK(length(fingerprint_sha256) = 64),
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 first_observed_at_ms INTEGER NOT NULL,
                 last_observed_at_ms INTEGER NOT NULL,
                 changed_at_ms INTEGER,
                 PRIMARY KEY(broker_server, account_login, symbol)
             ) WITHOUT ROWID;
             CREATE TABLE IF NOT EXISTS mt5_session_calendar_revisions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 broker_server TEXT NOT NULL,
                 account_login TEXT NOT NULL,
                 symbol TEXT NOT NULL,
                 revision INTEGER NOT NULL CHECK(revision >= 1),
                 previous_sha256 TEXT,
                 replacement_sha256 TEXT NOT NULL CHECK(length(replacement_sha256) = 64),
                 schedule_json TEXT NOT NULL,
                 observed_at_ms INTEGER NOT NULL,
                 UNIQUE(broker_server, account_login, symbol, revision)
             );
             CREATE INDEX IF NOT EXISTS idx_mt5_session_calendar_history
                 ON mt5_session_calendar_revisions(broker_server, account_login, symbol, revision DESC);
             CREATE TRIGGER IF NOT EXISTS trg_mt5_session_calendar_revisions_no_update
             BEFORE UPDATE ON mt5_session_calendar_revisions
             BEGIN SELECT RAISE(ABORT, 'MT5 session calendar history is append-only'); END;
             CREATE TRIGGER IF NOT EXISTS trg_mt5_session_calendar_revisions_no_delete
             BEFORE DELETE ON mt5_session_calendar_revisions
             BEGIN SELECT RAISE(ABORT, 'MT5 session calendar history is append-only'); END;"
        ).map_err(|error| format!("MT5 market-data schema initialization failed: {error}"))?;
        risk_decision_ledger::initialize(&connection)?;
        connection
            .pragma_update(None, "user_version", MARKET_DATA_SCHEMA_VERSION)
            .map_err(|error| {
                format!("MT5 market-data schema version could not be committed: {error}")
            })?;
        Ok(store)
    }

    fn open_connection(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path)
            .map_err(|error| format!("MT5 market-data database could not be opened: {error}"))?;
        connection.busy_timeout(BUSY_TIMEOUT).map_err(|error| {
            format!("MT5 market-data database busy timeout could not be configured: {error}")
        })?;
        connection
            .execute_batch("PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")
            .map_err(|error| {
                format!("MT5 market-data database durability settings failed: {error}")
            })?;
        Ok(connection)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append_risk_decision(
        &self,
        record: &Mt5RiskDecisionRecord,
    ) -> Result<Mt5RiskDecisionReceipt, String> {
        risk_decision_ledger::append(&self.path, record)
    }

    pub fn risk_decision_ledger_status(
        &self,
        broker_server: &str,
        account_login: &str,
    ) -> Result<Mt5RiskDecisionLedgerStatus, String> {
        risk_decision_ledger::status(&self.path, broker_server, account_login)
    }

    pub fn synchronization_cursors(
        &self,
        broker_server: &str,
        account_login: &str,
        symbols: &[String],
        timeframes: &[String],
    ) -> Result<Mt5SynchronizationCursors, String> {
        if !valid_identity(broker_server, 128)
            || !valid_identity(account_login, 32)
            || symbols.is_empty()
            || symbols.len() > 8
            || timeframes.is_empty()
            || timeframes.len() > 6
            || symbols.iter().any(|symbol| !valid_symbol(symbol))
            || timeframes
                .iter()
                .any(|timeframe| !valid_timeframe(timeframe))
            || symbols.iter().collect::<BTreeSet<_>>().len() != symbols.len()
            || timeframes.iter().collect::<BTreeSet<_>>().len() != timeframes.len()
        {
            return Err(
                "MT5 synchronization cursor request contains invalid or duplicate series.".into(),
            );
        }
        let connection = self.open_connection()?;
        let mut tick_cursors = HashMap::with_capacity(symbols.len());
        for symbol in symbols {
            let timestamp: i64 = connection.query_row(
                "SELECT COALESCE(MAX(time_ms), 0) FROM mt5_ticks WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
                params![broker_server, account_login, symbol], |row| row.get(0)
            ).map_err(|error| format!("MT5 tick synchronization cursor lookup failed: {error}"))?;
            tick_cursors.insert(symbol.clone(), timestamp);
        }
        let mut bar_cursors = HashMap::with_capacity(symbols.len() * timeframes.len());
        for symbol in symbols {
            for timeframe in timeframes {
                let timestamp: i64 = connection.query_row(
                    "SELECT COALESCE(MAX(time_ms), 0) FROM mt5_bars WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4",
                    params![broker_server, account_login, symbol, timeframe], |row| row.get(0)
                ).map_err(|error| format!("MT5 bar synchronization cursor lookup failed: {error}"))?;
                bar_cursors.insert(format!("{symbol}|{timeframe}"), timestamp);
            }
        }
        Ok(Mt5SynchronizationCursors {
            tick_cursors,
            bar_cursors,
        })
    }

    pub fn last_committed_sequence(
        &self,
        client_id: &str,
        session_id: &str,
    ) -> Result<u64, String> {
        let connection = self.open_connection()?;
        let sequence: i64 = connection.query_row(
            "SELECT COALESCE(MAX(sequence), 0) FROM mt5_frame_receipts WHERE client_id=?1 AND session_id=?2",
            params![client_id, session_id], |row| row.get(0)
        ).map_err(|error| format!("MT5 durable frame sequence could not be read: {error}"))?;
        u64::try_from(sequence).map_err(|_| "MT5 durable frame sequence is corrupt.".to_string())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn commit_frame(
        &self,
        client_id: &str,
        session_id: &str,
        sequence: u64,
        frame_sha256: &str,
        account_login: &str,
        broker_server: &str,
        batch: &Mt5MarketDataBatch,
        instrument_specification: &Mt5InstrumentSpecification,
        required_symbols: &[String],
        session_schedules: &[Mt5SessionSchedule],
        received_at_ms: i64,
    ) -> Result<IngestReceipt, String> {
        validate_batch(batch)?;
        if sequence == 0
            || sequence > i64::MAX as u64
            || !valid_identity(client_id, 192)
            || !valid_identity(session_id, 128)
            || !valid_identity(account_login, 32)
            || !valid_identity(broker_server, 128)
            || !is_sha256_hex(frame_sha256)
            || received_at_ms <= 0
        {
            return Err("MT5 market-data frame identity or commit metadata is invalid.".into());
        }
        validate_instrument_specification(instrument_specification)?;
        validate_required_session_symbols(required_symbols)?;
        validate_session_schedules(session_schedules)?;
        if !session_schedules.is_empty() {
            let required: BTreeSet<&str> = required_symbols.iter().map(String::as_str).collect();
            let supplied: BTreeSet<&str> = session_schedules
                .iter()
                .map(|schedule| schedule.symbol.as_str())
                .collect();
            if required != supplied || required_symbols.len() != session_schedules.len() {
                return Err(
                    "MT5 session calendar update must cover every required symbol exactly once."
                        .into(),
                );
            }
        }

        let mut connection = self.open_connection()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| format!("MT5 market-data transaction could not begin: {error}"))?;
        let existing: Option<String> = tx.query_row(
            "SELECT frame_sha256 FROM mt5_frame_receipts WHERE client_id=?1 AND session_id=?2 AND sequence=?3",
            params![client_id, session_id, sequence as i64], |row| row.get(0)
        ).optional().map_err(|error| format!("MT5 frame receipt lookup failed: {error}"))?;
        if let Some(existing_hash) = existing {
            if existing_hash == frame_sha256 {
                tx.commit().map_err(|error| {
                    format!("MT5 duplicate receipt transaction failed: {error}")
                })?;
                return Ok(IngestReceipt {
                    inserted_ticks: 0,
                    inserted_bars: 0,
                    revised_bars: 0,
                    duplicate_frame: true,
                });
            }
            return Err("MT5 sequence was already committed with a different frame payload; session is rejected.".into());
        }

        let last_sequence: i64 = tx.query_row(
            "SELECT COALESCE(MAX(sequence), 0) FROM mt5_frame_receipts WHERE client_id=?1 AND session_id=?2",
            params![client_id, session_id], |row| row.get(0)
        ).map_err(|error| format!("MT5 frame sequence lookup failed: {error}"))?;
        if sequence as i64 != last_sequence.saturating_add(1) {
            return Err(format!(
                "MT5 durable sequence discontinuity: expected {}, received {}.",
                last_sequence.saturating_add(1),
                sequence
            ));
        }

        if session_schedules.is_empty() {
            verify_existing_session_calendars(
                &tx,
                broker_server,
                account_login,
                required_symbols,
                received_at_ms,
            )?;
        }

        let mut inserted_ticks = 0;
        for tick in &batch.ticks {
            let fingerprint = tick_fingerprint(client_id, broker_server, account_login, tick);
            inserted_ticks += tx.execute(
                "INSERT OR IGNORE INTO mt5_ticks
                 (fingerprint, account_login, broker_server, symbol, time_ms, bid, ask, last, volume, volume_real, flags, received_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![fingerprint, account_login, broker_server, tick.symbol, tick.time_ms,
                    tick.bid, tick.ask, tick.last, tick.volume as i64, tick.volume_real,
                    tick.flags as i64, received_at_ms]
            ).map_err(|error| format!("MT5 tick insert failed: {error}"))?;
        }

        for tick in &batch.ticks {
            tx.execute(
                "INSERT INTO mt5_symbols (broker_server, account_login, symbol, first_seen_at_ms, last_seen_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(broker_server, account_login, symbol) DO UPDATE SET last_seen_at_ms=excluded.last_seen_at_ms",
                params![broker_server, account_login, tick.symbol, received_at_ms]
            ).map_err(|error| format!("MT5 symbol registry update failed: {error}"))?;
        }

        let mut inserted_bars = 0;
        let mut revised_bars = 0;
        for bar in &batch.bars {
            let payload_hash = bar_fingerprint(bar);
            let previous_hash: Option<String> = tx.query_row(
                "SELECT payload_sha256 FROM mt5_bars
                 WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4 AND time_ms=?5",
                params![broker_server, account_login, bar.symbol, bar.timeframe, bar.time_ms], |row| row.get(0)
            ).optional().map_err(|error| format!("MT5 existing-bar lookup failed: {error}"))?;
            match previous_hash {
                None => {
                    tx.execute(
                        "INSERT INTO mt5_bars
                         (broker_server, account_login, symbol, timeframe, time_ms, open, high, low, close,
                          tick_volume, spread, real_volume, payload_sha256, revision, first_received_at_ms, last_received_at_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 0, ?14, ?14)",
                        params![broker_server, account_login, bar.symbol, bar.timeframe, bar.time_ms,
                            bar.open, bar.high, bar.low, bar.close, bar.tick_volume as i64,
                            bar.spread, bar.real_volume as i64, payload_hash, received_at_ms]
                    ).map_err(|error| format!("MT5 closed-bar insert failed: {error}"))?;
                    inserted_bars += 1;
                }
                Some(previous) if previous != payload_hash => {
                    tx.execute(
                        "INSERT INTO mt5_bar_revisions
                         (broker_server, account_login, symbol, timeframe, time_ms, previous_sha256, replacement_sha256, observed_at_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                        params![broker_server, account_login, bar.symbol, bar.timeframe, bar.time_ms, previous, payload_hash, received_at_ms]
                    ).map_err(|error| format!("MT5 bar revision audit insert failed: {error}"))?;
                    tx.execute(
                        "UPDATE mt5_bars SET open=?6, high=?7, low=?8, close=?9, tick_volume=?10,
                         spread=?11, real_volume=?12, payload_sha256=?13, revision=revision+1, last_received_at_ms=?14
                         WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4 AND time_ms=?5",
                        params![broker_server, account_login, bar.symbol, bar.timeframe, bar.time_ms,
                            bar.open, bar.high, bar.low, bar.close, bar.tick_volume as i64,
                            bar.spread, bar.real_volume as i64, payload_hash, received_at_ms]
                    ).map_err(|error| format!("MT5 closed-bar revision update failed: {error}"))?;
                    revised_bars += 1;
                }
                Some(_) => {}
            }
        }

        for bar in &batch.bars {
            tx.execute(
                "INSERT INTO mt5_symbols (broker_server, account_login, symbol, first_seen_at_ms, last_seen_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?4)
                 ON CONFLICT(broker_server, account_login, symbol) DO UPDATE SET last_seen_at_ms=excluded.last_seen_at_ms",
                params![broker_server, account_login, bar.symbol, received_at_ms]
            ).map_err(|error| format!("MT5 symbol registry update failed: {error}"))?;
        }
        // Broker specifications are versioned independently from quotes. A heartbeat
        // updates last_observed_at_ms, while only a material fingerprint change creates
        // an immutable revision record.
        let specification_json =
            serde_json::to_string(instrument_specification).map_err(|error| {
                format!("MT5 instrument specification serialization failed: {error}")
            })?;
        let specification_hash = instrument_specification_fingerprint(instrument_specification)?;
        let previous_specification_row: Option<(String, i64, String, i64, i64)> = tx.query_row(
            "SELECT fingerprint_sha256, revision, specification_json, first_observed_at_ms, last_observed_at_ms
             FROM mt5_instrument_specs
             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
            params![broker_server, account_login, instrument_specification.symbol],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        ).optional().map_err(|error| format!("MT5 instrument specification lookup failed: {error}"))?;
        let previous_specification = if let Some((
            fingerprint,
            revision,
            payload,
            first_seen,
            last_seen,
        )) = previous_specification_row
        {
            let stored: Mt5InstrumentSpecification = serde_json::from_str(&payload)
                .map_err(|_| "Stored MT5 instrument specification projection is corrupt; ingestion is blocked.".to_string())?;
            validate_instrument_specification(&stored)?;
            if revision < 1
                || first_seen <= 0
                || last_seen < first_seen
                || stored.symbol != instrument_specification.symbol
                || instrument_specification_fingerprint(&stored)? != fingerprint
            {
                return Err("Stored MT5 instrument specification projection failed integrity validation; ingestion is blocked.".into());
            }
            verify_instrument_specification_history(
                &tx,
                broker_server,
                account_login,
                &instrument_specification.symbol,
                revision,
                &fingerprint,
            )?;
            Some((fingerprint, revision))
        } else {
            None
        };
        match previous_specification {
            None => {
                tx.execute(
                    "INSERT INTO mt5_instrument_specs
                     (broker_server, account_login, symbol, specification_json, fingerprint_sha256, revision,
                      first_observed_at_ms, last_observed_at_ms, changed_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, ?6)",
                    params![broker_server, account_login, instrument_specification.symbol,
                        specification_json, specification_hash, received_at_ms]
                ).map_err(|error| format!("MT5 instrument specification insert failed: {error}"))?;
                tx.execute(
                    "INSERT INTO mt5_instrument_spec_revisions
                     (broker_server, account_login, symbol, revision, previous_sha256, replacement_sha256, specification_json, observed_at_ms)
                     VALUES (?1, ?2, ?3, 1, NULL, ?4, ?5, ?6)",
                    params![broker_server, account_login, instrument_specification.symbol,
                        specification_hash, specification_json, received_at_ms]
                ).map_err(|error| format!("MT5 initial instrument specification audit failed: {error}"))?;
            }
            Some((previous_hash, _previous_revision)) if previous_hash == specification_hash => {
                // Tick values are broker-calculated in account/deposit currency and can move
                // as conversion rates change. Refresh the current valuation projection without
                // creating a false contract-specification revision.
                tx.execute(
                    "UPDATE mt5_instrument_specs SET specification_json=?4,
                     last_observed_at_ms=MAX(last_observed_at_ms, ?5)
                     WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
                    params![
                        broker_server,
                        account_login,
                        instrument_specification.symbol,
                        specification_json,
                        received_at_ms
                    ],
                )
                .map_err(|error| {
                    format!("MT5 instrument specification observation update failed: {error}")
                })?;
            }
            Some((previous_hash, previous_revision)) => {
                let revision = previous_revision.checked_add(1).ok_or_else(|| {
                    "MT5 instrument specification revision counter is exhausted.".to_string()
                })?;
                tx.execute(
                    "UPDATE mt5_instrument_specs SET specification_json=?4, fingerprint_sha256=?5,
                     revision=?6, last_observed_at_ms=MAX(last_observed_at_ms, ?7),
                     changed_at_ms=MAX(COALESCE(changed_at_ms, first_observed_at_ms), ?7)
                     WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
                    params![
                        broker_server,
                        account_login,
                        instrument_specification.symbol,
                        specification_json,
                        specification_hash,
                        revision,
                        received_at_ms
                    ],
                )
                .map_err(|error| {
                    format!("MT5 instrument specification revision update failed: {error}")
                })?;
                tx.execute(
                    "INSERT INTO mt5_instrument_spec_revisions
                     (broker_server, account_login, symbol, revision, previous_sha256, replacement_sha256, specification_json, observed_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![broker_server, account_login, instrument_specification.symbol,
                        revision, previous_hash, specification_hash, specification_json, received_at_ms]
                ).map_err(|error| format!("MT5 instrument specification revision audit failed: {error}"))?;
            }
        }

        for schedule in session_schedules {
            let schedule_json = serde_json::to_string(schedule)
                .map_err(|error| format!("MT5 session calendar serialization failed: {error}"))?;
            let fingerprint = session_schedule_fingerprint(schedule)?;
            let previous: Option<(String, i64, i64, i64, String)> = tx.query_row(
                "SELECT fingerprint_sha256, revision, first_observed_at_ms, last_observed_at_ms, schedule_json
                 FROM mt5_session_calendars WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
                params![broker_server, account_login, schedule.symbol],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            ).optional().map_err(|error| format!("MT5 session calendar projection lookup failed: {error}"))?;
            match previous {
                None => {
                    tx.execute(
                        "INSERT INTO mt5_session_calendars
                         (broker_server, account_login, symbol, schedule_json, fingerprint_sha256, revision, first_observed_at_ms, last_observed_at_ms, changed_at_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, ?6)",
                        params![broker_server, account_login, schedule.symbol, schedule_json, fingerprint, schedule.captured_at_ms]
                    ).map_err(|error| format!("MT5 session calendar projection insert failed: {error}"))?;
                    tx.execute(
                        "INSERT INTO mt5_session_calendar_revisions
                         (broker_server, account_login, symbol, revision, previous_sha256, replacement_sha256, schedule_json, observed_at_ms)
                         VALUES (?1, ?2, ?3, 1, NULL, ?4, ?5, ?6)",
                        params![broker_server, account_login, schedule.symbol, fingerprint, schedule_json, schedule.captured_at_ms]
                    ).map_err(|error| format!("MT5 session calendar initial revision insert failed: {error}"))?;
                }
                Some((previous_hash, revision, first_observed, last_observed, previous_json)) => {
                    let previous_schedule: Mt5SessionSchedule = serde_json::from_str(
                        &previous_json,
                    )
                    .map_err(|_| {
                        "Existing MT5 session calendar projection is corrupt; refusing ingestion."
                            .to_string()
                    })?;
                    validate_session_schedule(&previous_schedule)?;
                    if previous_schedule.symbol != schedule.symbol
                        || session_schedule_fingerprint(&previous_schedule)? != previous_hash
                        || previous_schedule.captured_at_ms != last_observed
                        || !is_sha256_hex(&previous_hash)
                        || revision < 1
                        || first_observed <= 0
                        || last_observed < first_observed
                    {
                        return Err("Existing MT5 session calendar projection failed its integrity checks; refusing ingestion.".into());
                    }
                    if schedule.captured_at_ms < last_observed {
                        return Err("MT5 session calendar capture time moved backwards; refusing to overwrite newer broker metadata.".into());
                    }
                    if previous_hash == fingerprint {
                        verify_session_calendar_history(
                            &tx,
                            broker_server,
                            account_login,
                            &schedule.symbol,
                            revision,
                            &previous_hash,
                        )?;
                        let latest: Option<(i64, String)> = tx.query_row(
                            "SELECT revision, replacement_sha256 FROM mt5_session_calendar_revisions
                             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 ORDER BY revision DESC LIMIT 1",
                            params![broker_server, account_login, schedule.symbol],
                            |row| Ok((row.get(0)?, row.get(1)?))
                        ).optional().map_err(|error| format!("MT5 session calendar latest revision lookup failed: {error}"))?;
                        if latest != Some((revision, previous_hash.clone())) {
                            return Err("MT5 session calendar current projection disagrees with its latest immutable revision.".into());
                        }
                        tx.execute(
                            "UPDATE mt5_session_calendars SET schedule_json=?4, last_observed_at_ms=?5
                             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
                            params![broker_server, account_login, schedule.symbol, schedule_json, schedule.captured_at_ms]
                        ).map_err(|error| format!("MT5 session calendar observation update failed: {error}"))?;
                    } else {
                        verify_session_calendar_history(
                            &tx,
                            broker_server,
                            account_login,
                            &schedule.symbol,
                            revision,
                            &previous_hash,
                        )?;
                        let next_revision = revision.checked_add(1).ok_or_else(|| {
                            "MT5 session calendar revision counter exhausted.".to_string()
                        })?;
                        tx.execute(
                            "UPDATE mt5_session_calendars SET schedule_json=?4, fingerprint_sha256=?5, revision=?6,
                             first_observed_at_ms=?7, last_observed_at_ms=?8, changed_at_ms=?8
                             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
                            params![broker_server, account_login, schedule.symbol, schedule_json, fingerprint, next_revision, first_observed, schedule.captured_at_ms]
                        ).map_err(|error| format!("MT5 session calendar projection update failed: {error}"))?;
                        tx.execute(
                            "INSERT INTO mt5_session_calendar_revisions
                             (broker_server, account_login, symbol, revision, previous_sha256, replacement_sha256, schedule_json, observed_at_ms)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                            params![broker_server, account_login, schedule.symbol, next_revision, previous_hash, fingerprint, schedule_json, schedule.captured_at_ms]
                        ).map_err(|error| format!("MT5 session calendar revision insert failed: {error}"))?;
                    }
                }
            }
        }

        tx.execute(
            "UPDATE mt5_market_data_stats
             SET ticks_total=ticks_total+?1, bars_total=bars_total+?2, last_ingest_at_ms=?3
             WHERE singleton_id=1",
            params![inserted_ticks as i64, inserted_bars as i64, received_at_ms],
        )
        .map_err(|error| format!("MT5 ingestion counters update failed: {error}"))?;

        tx.execute(
            "INSERT INTO mt5_frame_receipts (client_id, session_id, sequence, frame_sha256, committed_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![client_id, session_id, sequence as i64, frame_sha256, received_at_ms]
        ).map_err(|error| format!("MT5 durable frame receipt insert failed: {error}"))?;
        tx.commit()
            .map_err(|error| format!("MT5 market-data durability commit failed: {error}"))?;

        Ok(IngestReceipt {
            inserted_ticks,
            inserted_bars,
            revised_bars,
            duplicate_frame: false,
        })
    }

    pub fn instrument_specification(
        &self,
        broker_server: &str,
        account_login: &str,
        symbol: &str,
    ) -> Result<Mt5InstrumentSpecificationRecord, String> {
        if !valid_identity(broker_server, 128)
            || !valid_identity(account_login, 32)
            || !valid_symbol(symbol)
        {
            return Err("MT5 instrument specification query identity is invalid.".into());
        }
        let connection = self.open_connection()?;
        let row: Option<(String, String, i64, i64, i64, Option<i64>)> = connection
            .query_row(
                "SELECT specification_json, fingerprint_sha256, revision, first_observed_at_ms,
             last_observed_at_ms, changed_at_ms FROM mt5_instrument_specs
             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
                params![broker_server, account_login, symbol],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| format!("MT5 instrument specification query failed: {error}"))?;
        let (json, fingerprint, revision, first_observed, last_observed, changed_at) = row
            .ok_or_else(|| "No broker-reported MT5 instrument specification is stored for this account and symbol.".to_string())?;
        let specification: Mt5InstrumentSpecification =
            serde_json::from_str(&json).map_err(|_| {
                "Stored MT5 instrument specification is corrupt or incompatible.".to_string()
            })?;
        validate_instrument_specification(&specification)?;
        if specification.symbol != symbol
            || !is_sha256_hex(&fingerprint)
            || instrument_specification_fingerprint(&specification)? != fingerprint
            || revision < 1
            || first_observed <= 0
            || last_observed < first_observed
        {
            return Err("Stored MT5 instrument specification metadata is corrupt.".into());
        }
        verify_instrument_specification_history(
            &connection,
            broker_server,
            account_login,
            symbol,
            revision,
            &fingerprint,
        )?;
        Ok(Mt5InstrumentSpecificationRecord {
            broker_server: broker_server.to_string(),
            account_login: account_login.to_string(),
            specification,
            fingerprint_sha256: fingerprint,
            revision: u64::try_from(revision)
                .map_err(|_| "Stored MT5 specification revision is corrupt.".to_string())?,
            first_observed_at_ms: first_observed,
            last_observed_at_ms: last_observed,
            changed_at_ms: changed_at,
            completeness: "broker_snapshot_v1_partial".into(),
        })
    }

    pub fn session_calendar(
        &self,
        broker_server: &str,
        account_login: &str,
        symbol: &str,
    ) -> Result<Mt5SessionCalendarRecord, String> {
        if !valid_identity(broker_server, 128)
            || !valid_identity(account_login, 32)
            || !valid_symbol(symbol)
        {
            return Err("MT5 session calendar query identity is invalid.".into());
        }
        let connection = self.open_connection()?;
        let row: Option<(String, String, i64, i64, i64, Option<i64>)> = connection.query_row(
            "SELECT schedule_json, fingerprint_sha256, revision, first_observed_at_ms, last_observed_at_ms, changed_at_ms
             FROM mt5_session_calendars WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
            params![broker_server, account_login, symbol],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
        ).optional().map_err(|error| format!("MT5 session calendar query failed: {error}"))?;
        let (json, fingerprint, revision, first_observed, last_observed, changed_at) = row
            .ok_or_else(|| {
                "No broker-reported MT5 session calendar is stored for this account and symbol."
                    .to_string()
            })?;
        let schedule: Mt5SessionSchedule = serde_json::from_str(&json)
            .map_err(|_| "Stored MT5 session calendar is corrupt or incompatible.".to_string())?;
        validate_session_schedule(&schedule)?;
        if schedule.symbol != symbol
            || schedule.captured_at_ms != last_observed
            || !is_sha256_hex(&fingerprint)
            || session_schedule_fingerprint(&schedule)? != fingerprint
            || revision < 1
            || first_observed <= 0
            || last_observed < first_observed
        {
            return Err("Stored MT5 session calendar metadata is corrupt.".into());
        }
        verify_session_calendar_history(
            &connection,
            broker_server,
            account_login,
            symbol,
            revision,
            &fingerprint,
        )?;
        Ok(Mt5SessionCalendarRecord {
            broker_server: broker_server.into(),
            account_login: account_login.into(),
            schedule,
            fingerprint_sha256: fingerprint,
            revision: u64::try_from(revision)
                .map_err(|_| "Stored MT5 session calendar revision is corrupt.".to_string())?,
            first_observed_at_ms: first_observed,
            last_observed_at_ms: last_observed,
            changed_at_ms: changed_at,
            time_basis: "broker_server_wall_clock; no UTC offset or DST inference".into(),
        })
    }

    pub fn query_series(&self, request: &Mt5SeriesQuery) -> Result<Mt5SeriesQueryResponse, String> {
        validate_series_query(request)?;
        let nominal_interval_ms = timeframe_interval_ms(&request.timeframe)
            .ok_or_else(|| "MT5 timeframe has no supported nominal interval.".to_string())?;
        let mut connection = self.open_connection()?;
        // Coverage, revision and page reads must observe the same SQLite snapshot even if
        // the ingestion worker commits a new frame while this query is running.
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|error| format!("MT5 series read snapshot could not begin: {error}"))?;

        let calendar_row: Option<(String, String, i64, i64)> = tx.query_row(
            "SELECT schedule_json, fingerprint_sha256, revision, last_observed_at_ms FROM mt5_session_calendars
             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
            params![request.broker_server, request.account_login, request.symbol],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        ).optional().map_err(|error| format!("MT5 session calendar lookup failed: {error}"))?;
        let (session_calendar_revision, session_calendar_fingerprint_sha256) = match calendar_row {
            Some((schedule_json, fingerprint, revision, last_observed_at_ms)) => {
                let schedule: Mt5SessionSchedule =
                    serde_json::from_str(&schedule_json).map_err(|_| {
                        "Stored MT5 session calendar is corrupt or incompatible.".to_string()
                    })?;
                validate_session_schedule(&schedule)?;
                if schedule.captured_at_ms != last_observed_at_ms
                    || session_schedule_fingerprint(&schedule)? != fingerprint
                    || !is_sha256_hex(&fingerprint)
                    || revision < 1
                {
                    return Err(
                        "Stored MT5 session calendar fingerprint or revision is corrupt.".into(),
                    );
                }
                verify_session_calendar_history(
                    &tx,
                    &request.broker_server,
                    &request.account_login,
                    &request.symbol,
                    revision,
                    &fingerprint,
                )?;
                (
                    Some(u64::try_from(revision).map_err(|_| {
                        "Stored MT5 session calendar revision is corrupt.".to_string()
                    })?),
                    Some(fingerprint),
                )
            }
            None => (None, None),
        };

        let (bars_total, first_bar_time_ms, last_bar_time_ms): (i64, Option<i64>, Option<i64>) = tx
            .query_row(
                "SELECT COUNT(*), MIN(time_ms), MAX(time_ms) FROM mt5_bars
             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4
               AND time_ms>=?5 AND time_ms<=?6",
                params![
                    request.broker_server,
                    request.account_login,
                    request.symbol,
                    request.timeframe,
                    request.from_ms,
                    request.to_ms
                ],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| format!("MT5 series coverage query failed: {error}"))?;
        // Stable commitment over the selected series identity, range and ordered stored
        // bar fingerprints. This is a reproducibility identifier, not a signature or HMAC.
        let mut dataset_hasher = Sha256::new();
        dataset_hasher.update(b"ATOS-MT5-SERIES-MANIFEST-V1");
        for part in [
            request.broker_server.as_bytes(),
            request.account_login.as_bytes(),
            request.symbol.as_bytes(),
            request.timeframe.as_bytes(),
        ] {
            dataset_hasher.update((part.len() as u64).to_be_bytes());
            dataset_hasher.update(part);
        }
        dataset_hasher.update(request.from_ms.to_be_bytes());
        dataset_hasher.update(request.to_ms.to_be_bytes());
        dataset_hasher.update(b"ATOS-MT5-SESSION-CALENDAR-CONTEXT-V1");
        if let Some(calendar_fingerprint) = &session_calendar_fingerprint_sha256 {
            dataset_hasher.update(calendar_fingerprint.as_bytes());
        } else {
            dataset_hasher.update(b"SESSION-CALENDAR-UNKNOWN");
        }
        let mut manifest_statement = tx
            .prepare(
                "SELECT time_ms, payload_sha256 FROM mt5_bars
             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4
               AND time_ms>=?5 AND time_ms<=?6 ORDER BY time_ms ASC",
            )
            .map_err(|error| {
                format!("MT5 dataset manifest query could not be prepared: {error}")
            })?;
        let manifest_rows = manifest_statement
            .query_map(
                params![
                    request.broker_server,
                    request.account_login,
                    request.symbol,
                    request.timeframe,
                    request.from_ms,
                    request.to_ms
                ],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .map_err(|error| format!("MT5 dataset manifest query failed: {error}"))?;
        for row in manifest_rows {
            let (time_ms, payload_hash) =
                row.map_err(|error| format!("MT5 dataset manifest row is unreadable: {error}"))?;
            if !is_sha256_hex(&payload_hash) {
                return Err(
                    "MT5 dataset manifest contains a malformed payload fingerprint.".into(),
                );
            }
            dataset_hasher.update(time_ms.to_be_bytes());
            dataset_hasher.update(payload_hash.as_bytes());
        }
        drop(manifest_statement);
        let dataset_fingerprint_sha256 = dataset_hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();

        let revisions_total: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM mt5_bar_revisions
             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4
               AND time_ms>=?5 AND time_ms<=?6",
                params![
                    request.broker_server,
                    request.account_login,
                    request.symbol,
                    request.timeframe,
                    request.from_ms,
                    request.to_ms
                ],
                |row| row.get(0),
            )
            .map_err(|error| format!("MT5 series revision query failed: {error}"))?;

        // A discontinuity is deliberately reported as a candidate, not a confirmed missing
        // candle: this store has no broker trading-session calendar or holiday schedule.
        let gap_threshold = nominal_interval_ms.saturating_mul(3) / 2;
        let (interval_anomaly_count, largest_interval_ms): (i64, Option<i64>) = tx.query_row(
            "WITH ordered AS (
                SELECT time_ms, LAG(time_ms) OVER (ORDER BY time_ms) AS previous_time_ms
                FROM mt5_bars
                WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4
                  AND time_ms>=?5 AND time_ms<=?6
             ), deltas AS (
                SELECT time_ms-previous_time_ms AS elapsed_ms FROM ordered WHERE previous_time_ms IS NOT NULL
             )
             SELECT COALESCE(SUM(CASE WHEN elapsed_ms>?7 THEN 1 ELSE 0 END), 0), MAX(elapsed_ms)
             FROM deltas",
            params![request.broker_server, request.account_login, request.symbol, request.timeframe, request.from_ms, request.to_ms, gap_threshold],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).map_err(|error| format!("MT5 series interval analysis failed: {error}"))?;

        let mut gap_statement = tx
            .prepare(
                "WITH ordered AS (
                SELECT time_ms, LAG(time_ms) OVER (ORDER BY time_ms) AS previous_time_ms
                FROM mt5_bars
                WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4
                  AND time_ms>=?5 AND time_ms<=?6
             )
             SELECT previous_time_ms, time_ms FROM ordered
             WHERE previous_time_ms IS NOT NULL AND time_ms-previous_time_ms>?7
             ORDER BY previous_time_ms ASC LIMIT 64",
            )
            .map_err(|error| format!("MT5 series gap query could not be prepared: {error}"))?;
        let gap_rows = gap_statement
            .query_map(
                params![
                    request.broker_server,
                    request.account_login,
                    request.symbol,
                    request.timeframe,
                    request.from_ms,
                    request.to_ms,
                    gap_threshold
                ],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .map_err(|error| format!("MT5 series gap query failed: {error}"))?;
        let mut gap_candidates = Vec::new();
        for row in gap_rows {
            let (previous_bar_time_ms, next_bar_time_ms) =
                row.map_err(|error| format!("MT5 series gap row could not be read: {error}"))?;
            let elapsed_ms = next_bar_time_ms.saturating_sub(previous_bar_time_ms);
            gap_candidates.push(Mt5SeriesGapCandidate {
                previous_bar_time_ms,
                next_bar_time_ms,
                elapsed_ms,
                nominal_intervals_elapsed: elapsed_ms / nominal_interval_ms,
            });
        }

        let cursor_ms = request
            .after_ms
            .unwrap_or(request.from_ms.saturating_sub(1));
        let mut statement = tx.prepare(
            "SELECT symbol, timeframe, time_ms, open, high, low, close, tick_volume, spread, real_volume,
                    payload_sha256, revision, first_received_at_ms, last_received_at_ms
             FROM mt5_bars
             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 AND timeframe=?4
               AND time_ms>=?5 AND time_ms<=?6 AND time_ms>?7
             ORDER BY time_ms ASC LIMIT ?8"
        ).map_err(|error| format!("MT5 series page query could not be prepared: {error}"))?;
        let mut rows = statement
            .query(params![
                request.broker_server,
                request.account_login,
                request.symbol,
                request.timeframe,
                request.from_ms,
                request.to_ms,
                cursor_ms,
                (request.limit + 1) as i64
            ])
            .map_err(|error| format!("MT5 series page query failed: {error}"))?;
        let mut bars = Vec::with_capacity(request.limit + 1);
        while let Some(row) = rows
            .next()
            .map_err(|error| format!("MT5 series page cursor failed: {error}"))?
        {
            let tick_volume: i64 = row
                .get(7)
                .map_err(|error| format!("MT5 stored tick volume is unreadable: {error}"))?;
            let real_volume: i64 = row
                .get(9)
                .map_err(|error| format!("MT5 stored real volume is unreadable: {error}"))?;
            let revision: i64 = row
                .get(11)
                .map_err(|error| format!("MT5 stored revision counter is unreadable: {error}"))?;
            let stored_bar = Mt5StoredBar {
                symbol: row
                    .get(0)
                    .map_err(|error| format!("MT5 stored symbol is unreadable: {error}"))?,
                timeframe: row
                    .get(1)
                    .map_err(|error| format!("MT5 stored timeframe is unreadable: {error}"))?,
                time_ms: row
                    .get(2)
                    .map_err(|error| format!("MT5 stored timestamp is unreadable: {error}"))?,
                open: row
                    .get(3)
                    .map_err(|error| format!("MT5 stored open is unreadable: {error}"))?,
                high: row
                    .get(4)
                    .map_err(|error| format!("MT5 stored high is unreadable: {error}"))?,
                low: row
                    .get(5)
                    .map_err(|error| format!("MT5 stored low is unreadable: {error}"))?,
                close: row
                    .get(6)
                    .map_err(|error| format!("MT5 stored close is unreadable: {error}"))?,
                tick_volume: u64::try_from(tick_volume)
                    .map_err(|_| "MT5 stored tick volume is corrupt.".to_string())?,
                spread: row
                    .get(8)
                    .map_err(|error| format!("MT5 stored spread is unreadable: {error}"))?,
                real_volume: u64::try_from(real_volume)
                    .map_err(|_| "MT5 stored real volume is corrupt.".to_string())?,
                payload_sha256: row
                    .get(10)
                    .map_err(|error| format!("MT5 stored payload hash is unreadable: {error}"))?,
                revision: u64::try_from(revision)
                    .map_err(|_| "MT5 stored revision counter is corrupt.".to_string())?,
                first_received_at_ms: row.get(12).map_err(|error| {
                    format!("MT5 first receipt timestamp is unreadable: {error}")
                })?,
                last_received_at_ms: row.get(13).map_err(|error| {
                    format!("MT5 last receipt timestamp is unreadable: {error}")
                })?,
            };
            if stored_bar.tick_volume > MAX_SAFE_JSON_INTEGER as u64
                || stored_bar.real_volume > MAX_SAFE_JSON_INTEGER as u64
                || stored_bar.revision > MAX_SAFE_JSON_INTEGER as u64
                || stored_bar.spread > MAX_SAFE_JSON_INTEGER
                || stored_bar.first_received_at_ms > MAX_SAFE_JSON_INTEGER
                || stored_bar.last_received_at_ms > MAX_SAFE_JSON_INTEGER
            {
                return Err("MT5 stored integer value exceeds the exact JSON number range; refusing lossy export.".into());
            }
            let integrity_bar = Mt5Bar {
                symbol: stored_bar.symbol.clone(),
                timeframe: stored_bar.timeframe.clone(),
                time_ms: stored_bar.time_ms,
                open: stored_bar.open,
                high: stored_bar.high,
                low: stored_bar.low,
                close: stored_bar.close,
                tick_volume: stored_bar.tick_volume,
                spread: stored_bar.spread,
                real_volume: stored_bar.real_volume,
                closed: true,
            };
            validate_batch(&Mt5MarketDataBatch {
                ticks: vec![],
                bars: vec![integrity_bar.clone()],
            })
            .map_err(|_| {
                "MT5 stored bar failed its integrity invariants; refusing to export corrupted data."
                    .to_string()
            })?;
            if !is_sha256_hex(&stored_bar.payload_sha256)
                || bar_fingerprint(&integrity_bar) != stored_bar.payload_sha256
                || stored_bar.first_received_at_ms <= 0
                || stored_bar.last_received_at_ms < stored_bar.first_received_at_ms
            {
                return Err("MT5 stored bar provenance or payload fingerprint is corrupt; refusing to export it.".into());
            }
            bars.push(stored_bar);
        }
        let has_more = bars.len() > request.limit;
        if has_more {
            bars.pop();
        }
        let next_after_ms = if has_more {
            bars.last().map(|bar| bar.time_ms)
        } else {
            None
        };
        drop(rows);
        drop(statement);
        drop(gap_statement);
        tx.commit().map_err(|error| {
            format!("MT5 series read snapshot could not close cleanly: {error}")
        })?;

        let bars_total = u64::try_from(bars_total)
            .map_err(|_| "MT5 series bar count is corrupt.".to_string())?;
        let revisions_total = u64::try_from(revisions_total)
            .map_err(|_| "MT5 series revision count is corrupt.".to_string())?;
        let interval_anomaly_count = u64::try_from(interval_anomaly_count)
            .map_err(|_| "MT5 series interval-anomaly count is corrupt.".to_string())?;
        if [bars_total, revisions_total, interval_anomaly_count]
            .into_iter()
            .any(|value| value > MAX_SAFE_JSON_INTEGER as u64)
        {
            return Err(
                "MT5 series count exceeds the exact JSON number range; refusing lossy export."
                    .into(),
            );
        }

        Ok(Mt5SeriesQueryResponse {
            broker_server: request.broker_server.clone(),
            account_login: request.account_login.clone(),
            symbol: request.symbol.clone(),
            timeframe: request.timeframe.clone(),
            from_ms: request.from_ms,
            to_ms: request.to_ms,
            dataset_fingerprint_sha256,
            bars,
            quality: Mt5SeriesQuality {
                bars_total,
                first_bar_time_ms,
                last_bar_time_ms,
                revisions_total,
                nominal_interval_ms,
                interval_anomaly_count,
                largest_interval_ms,
                gap_candidates,
                continuity_interpretation: if session_calendar_revision.is_some() {
                    "Broker-reported weekly trade sessions are attached to dataset provenance, but gap candidates remain unclassified until broker timezone/DST transitions and dated holiday/exception calendars are independently resolved. Do not treat them as confirmed missing tradable candles.".into()
                } else {
                    "Broker session calendar is unknown. Interval anomalies are unclassified candidates; do not infer market closures or confirmed missing tradable candles.".into()
                },
                session_calendar_revision,
                session_calendar_fingerprint_sha256: session_calendar_fingerprint_sha256.clone(),
                session_calendar_time_basis: if session_calendar_revision.is_some() {
                    "broker_server_wall_clock; no UTC offset or DST inference".into()
                } else {
                    "unknown; no broker session calendar is stored".into()
                },
            },
            has_more,
            next_after_ms,
        })
    }

    pub fn status(&self) -> Result<Mt5MarketDataStatus, String> {
        let connection = self.open_connection()?;
        let (ticks_total, bars_total, last_ingest_at_ms): (i64, i64, Option<i64>) = connection.query_row(
            "SELECT ticks_total, bars_total, last_ingest_at_ms FROM mt5_market_data_stats WHERE singleton_id=1",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        ).map_err(|error| format!("MT5 market-data aggregate status query failed: {error}"))?;
        let symbols_total: i64 = connection
            .query_row("SELECT COUNT(*) FROM mt5_symbols", [], |row| row.get(0))
            .map_err(|error| format!("MT5 symbol coverage query failed: {error}"))?;
        let ticks_total = u64::try_from(ticks_total)
            .map_err(|_| "MT5 durable tick counter is negative or corrupt.".to_string())?;
        let bars_total = u64::try_from(bars_total)
            .map_err(|_| "MT5 durable bar counter is negative or corrupt.".to_string())?;
        let symbols_total = u64::try_from(symbols_total)
            .map_err(|_| "MT5 durable symbol count is negative or corrupt.".to_string())?;
        Ok(Mt5MarketDataStatus {
            database_path: self.path.display().to_string(),
            ticks_total,
            bars_total,
            symbols_total,
            last_ingest_at_ms,
            storage_available: true,
        })
    }
}

pub fn validate_series_query(request: &Mt5SeriesQuery) -> Result<(), String> {
    if !valid_identity(&request.broker_server, 128)
        || !valid_identity(&request.account_login, 32)
        || !valid_symbol(&request.symbol)
        || !valid_timeframe(&request.timeframe)
        || timeframe_interval_ms(&request.timeframe).is_none()
        || request.from_ms <= 0
        || request.from_ms > MAX_SAFE_JSON_INTEGER
        || request.to_ms < request.from_ms
        || request.to_ms > MAX_SAFE_JSON_INTEGER
        || request.to_ms.saturating_sub(request.from_ms) > 366_i64 * 24 * 60 * 60 * 1000
        || request.after_ms.is_some_and(|cursor| {
            cursor < request.from_ms.saturating_sub(1)
                || cursor > request.to_ms
                || cursor > MAX_SAFE_JSON_INTEGER
        })
        || !(1..=5_000).contains(&request.limit)
    {
        return Err("MT5 series query is invalid; verify identity, symbol, timeframe, range, cursor, and page limit.".into());
    }
    Ok(())
}

fn verify_instrument_specification_history(
    connection: &Connection,
    broker_server: &str,
    account_login: &str,
    symbol: &str,
    current_revision: i64,
    current_fingerprint: &str,
) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            "SELECT revision, previous_sha256, replacement_sha256, specification_json
         FROM mt5_instrument_spec_revisions
         WHERE broker_server=?1 AND account_login=?2 AND symbol=?3
         ORDER BY revision ASC",
        )
        .map_err(|error| {
            format!("MT5 instrument specification history could not be prepared: {error}")
        })?;
    let mut rows = statement
        .query(params![broker_server, account_login, symbol])
        .map_err(|error| {
            format!("MT5 instrument specification history could not be read: {error}")
        })?;
    let mut expected_revision = 1_i64;
    let mut previous_fingerprint: Option<String> = None;
    while let Some(row) = rows.next().map_err(|error| {
        format!("MT5 instrument specification history iteration failed: {error}")
    })? {
        let revision: i64 = row
            .get(0)
            .map_err(|error| format!("MT5 specification history revision is corrupt: {error}"))?;
        let previous: Option<String> = row.get(1).map_err(|error| {
            format!("MT5 specification history predecessor is corrupt: {error}")
        })?;
        let replacement: String = row.get(2).map_err(|error| {
            format!("MT5 specification history fingerprint is corrupt: {error}")
        })?;
        let payload: String = row
            .get(3)
            .map_err(|error| format!("MT5 specification history payload is corrupt: {error}"))?;
        if revision != expected_revision
            || previous != previous_fingerprint
            || !is_sha256_hex(&replacement)
        {
            return Err(
                "MT5 instrument specification revision chain is discontinuous or corrupt.".into(),
            );
        }
        let historical_specification: Mt5InstrumentSpecification = serde_json::from_str(&payload)
            .map_err(|_| {
            "MT5 instrument specification revision payload is invalid JSON.".to_string()
        })?;
        validate_instrument_specification(&historical_specification)?;
        if historical_specification.symbol != symbol
            || instrument_specification_fingerprint(&historical_specification)? != replacement
        {
            return Err(
                "MT5 instrument specification revision payload does not match its fingerprint."
                    .into(),
            );
        }
        previous_fingerprint = Some(replacement);
        expected_revision = expected_revision.checked_add(1).ok_or_else(|| {
            "MT5 instrument specification history revision counter is exhausted.".to_string()
        })?;
    }
    if expected_revision != current_revision.saturating_add(1)
        || previous_fingerprint.as_deref() != Some(current_fingerprint)
    {
        return Err("MT5 instrument specification current projection does not match its append-only revision history.".into());
    }
    Ok(())
}

fn verify_session_calendar_history(
    connection: &Connection,
    broker_server: &str,
    account_login: &str,
    symbol: &str,
    current_revision: i64,
    current_fingerprint: &str,
) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            "SELECT revision, previous_sha256, replacement_sha256, schedule_json
         FROM mt5_session_calendar_revisions
         WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 ORDER BY revision ASC",
        )
        .map_err(|error| format!("MT5 session calendar history could not be prepared: {error}"))?;
    let mut rows = statement
        .query(params![broker_server, account_login, symbol])
        .map_err(|error| format!("MT5 session calendar history could not be read: {error}"))?;
    let mut expected_revision = 1_i64;
    let mut previous_fingerprint: Option<String> = None;
    while let Some(row) = rows
        .next()
        .map_err(|error| format!("MT5 session calendar history iteration failed: {error}"))?
    {
        let revision: i64 = row
            .get(0)
            .map_err(|error| format!("MT5 session calendar revision is corrupt: {error}"))?;
        let previous: Option<String> = row
            .get(1)
            .map_err(|error| format!("MT5 session calendar predecessor is corrupt: {error}"))?;
        let replacement: String = row
            .get(2)
            .map_err(|error| format!("MT5 session calendar fingerprint is corrupt: {error}"))?;
        let payload: String = row
            .get(3)
            .map_err(|error| format!("MT5 session calendar payload is corrupt: {error}"))?;
        if revision != expected_revision
            || previous != previous_fingerprint
            || !is_sha256_hex(&replacement)
        {
            return Err("MT5 session calendar revision chain is discontinuous or corrupt.".into());
        }
        let historical_schedule: Mt5SessionSchedule = serde_json::from_str(&payload)
            .map_err(|_| "MT5 session calendar revision payload is invalid JSON.".to_string())?;
        validate_session_schedule(&historical_schedule)?;
        if historical_schedule.symbol != symbol
            || session_schedule_fingerprint(&historical_schedule)? != replacement
        {
            return Err(
                "MT5 session calendar revision payload does not match its fingerprint.".into(),
            );
        }
        previous_fingerprint = Some(replacement);
        expected_revision = expected_revision
            .checked_add(1)
            .ok_or_else(|| "MT5 session calendar revision counter is exhausted.".to_string())?;
    }
    if expected_revision != current_revision.saturating_add(1)
        || previous_fingerprint.as_deref() != Some(current_fingerprint)
    {
        return Err(
            "MT5 session calendar projection does not match its append-only revision history."
                .into(),
        );
    }
    Ok(())
}

fn validate_required_session_symbols(symbols: &[String]) -> Result<(), String> {
    if symbols.is_empty() || symbols.len() > MAX_SYMBOLS_PER_FRAME {
        return Err("MT5 authenticated session-symbol scope is empty or exceeds its bound.".into());
    }
    let mut unique = BTreeSet::new();
    if symbols
        .iter()
        .any(|symbol| !valid_symbol(symbol) || !unique.insert(symbol.as_str()))
    {
        return Err(
            "MT5 authenticated session-symbol scope contains an invalid or duplicate symbol."
                .into(),
        );
    }
    Ok(())
}

fn verify_existing_session_calendars(
    connection: &Connection,
    broker_server: &str,
    account_login: &str,
    required_symbols: &[String],
    received_at_ms: i64,
) -> Result<(), String> {
    for symbol in required_symbols {
        let row: Option<(String, String, i64, i64)> = connection
            .query_row(
                "SELECT schedule_json, fingerprint_sha256, revision, last_observed_at_ms
             FROM mt5_session_calendars WHERE broker_server=?1 AND account_login=?2 AND symbol=?3",
                params![broker_server, account_login, symbol],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(|error| format!("MT5 retained session calendar lookup failed: {error}"))?;
        let (payload, fingerprint, revision, last_observed) = row
            .ok_or_else(|| format!("No retained broker session calendar exists for {symbol}; the next frame must include a complete schedule."))?;
        let schedule: Mt5SessionSchedule = serde_json::from_str(&payload)
            .map_err(|_| format!("Retained MT5 session calendar for {symbol} is corrupt."))?;
        validate_session_schedule(&schedule)?;
        if schedule.symbol.as_str() != symbol.as_str()
            || schedule.captured_at_ms != last_observed
            || revision < 1
            || !is_sha256_hex(&fingerprint)
            || session_schedule_fingerprint(&schedule)? != fingerprint
        {
            return Err(format!(
                "Retained MT5 session calendar for {symbol} failed integrity validation."
            ));
        }
        let age_ms = received_at_ms.saturating_sub(last_observed);
        if !(-MAX_CLOCK_SKEW_MS..=MAX_SESSION_CALENDAR_AGE_MS).contains(&age_ms) {
            return Err(format!("Retained MT5 session calendar for {symbol} is stale or future-dated; a fresh complete schedule is required."));
        }
        let latest: Option<(i64, String)> = connection.query_row(
            "SELECT revision, replacement_sha256 FROM mt5_session_calendar_revisions
             WHERE broker_server=?1 AND account_login=?2 AND symbol=?3 ORDER BY revision DESC LIMIT 1",
            params![broker_server, account_login, symbol],
            |row| Ok((row.get(0)?, row.get(1)?))
        ).optional().map_err(|error| format!("MT5 retained session calendar revision lookup failed: {error}"))?;
        if latest != Some((revision, fingerprint)) {
            return Err(format!("Retained MT5 session calendar history for {symbol} does not match its current projection."));
        }
    }
    Ok(())
}

fn validate_session_schedules(schedules: &[Mt5SessionSchedule]) -> Result<(), String> {
    if schedules.len() > MAX_SYMBOLS_PER_FRAME {
        return Err("MT5 frame exceeds the maximum session calendar count.".into());
    }
    let mut symbols = BTreeSet::new();
    for schedule in schedules {
        validate_session_schedule(schedule)?;
        if !symbols.insert(schedule.symbol.as_str()) {
            return Err("MT5 frame contains duplicate session calendars for a symbol.".into());
        }
    }
    Ok(())
}

fn session_schedule_fingerprint(schedule: &Mt5SessionSchedule) -> Result<String, String> {
    let mut stable_schedule = schedule.clone();
    stable_schedule.captured_at_ms = 0;
    let canonical = serde_json::to_vec(&stable_schedule).map_err(|error| {
        format!("MT5 session calendar fingerprint serialization failed: {error}")
    })?;
    let mut hasher = Sha256::new();
    hasher.update(b"ATOS-MT5-SESSION-SCHEDULE-V1");
    hasher.update(canonical);
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>())
}

pub fn validate_session_schedule(schedule: &Mt5SessionSchedule) -> Result<(), String> {
    if !valid_symbol(&schedule.symbol)
        || schedule.days.len() != 7
        || schedule.captured_at_ms <= 0
        || schedule.captured_at_ms > MAX_SAFE_JSON_INTEGER
    {
        return Err("MT5 session calendar must identify a valid symbol, capture instant, and all seven weekdays.".into());
    }
    for (expected_day, day) in schedule.days.iter().enumerate() {
        if day.day_of_week as usize != expected_day || day.sessions.len() > 16 {
            return Err(
                "MT5 session calendar weekdays or per-day session count are invalid.".into(),
            );
        }
        for (expected_index, session) in day.sessions.iter().enumerate() {
            if session.index as usize != expected_index
                || session.from_seconds > 86_400
                || session.to_seconds > 86_400
            {
                return Err(
                    "MT5 broker session has invalid ordering or seconds-from-midnight boundaries."
                        .into(),
                );
            }
        }
    }
    Ok(())
}

pub fn validate_instrument_specification(spec: &Mt5InstrumentSpecification) -> Result<(), String> {
    if !valid_symbol(&spec.symbol)
        || !(0..=12).contains(&spec.digits)
        || !(0..=255).contains(&spec.calculation_mode)
        || !(0..=4).contains(&spec.trade_mode)
        || !(0..=3).contains(&spec.execution_mode)
        || spec.filling_mode < 0
        || spec.filling_mode > 7
        || spec.order_mode < 0
        || spec.order_mode > 127
        || spec.stops_level_points < 0
        || spec.freeze_level_points < 0
        || !(0..=9).contains(&spec.swap_mode)
    {
        return Err(
            "MT5 broker instrument specification contains an invalid enum, flag, or integer range."
                .into(),
        );
    }
    for (label, value) in [
        ("point", spec.point),
        ("contract size", spec.contract_size),
        ("tick size", spec.tick_size),
        ("tick value", spec.tick_value),
        ("profit tick value", spec.tick_value_profit),
        ("loss tick value", spec.tick_value_loss),
        ("minimum volume", spec.volume_min),
        ("maximum volume", spec.volume_max),
        ("volume step", spec.volume_step),
        ("volume limit", spec.volume_limit),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!(
                "MT5 broker instrument {label} must be finite and non-negative."
            ));
        }
    }
    for (label, value) in [
        ("long swap", spec.swap_long),
        ("short swap", spec.swap_short),
    ] {
        if !value.is_finite() {
            return Err(format!("MT5 broker instrument {label} must be finite."));
        }
    }
    if spec.point <= 0.0
        || spec.contract_size <= 0.0
        || spec.tick_size <= 0.0
        || spec.volume_min <= 0.0
        || spec.volume_max < spec.volume_min
        || spec.volume_step <= 0.0
        || (spec.volume_limit > 0.0 && spec.volume_limit < spec.volume_min)
    {
        return Err(
            "MT5 broker instrument point, contract, tick, or volume constraints are inconsistent."
                .into(),
        );
    }
    for (label, currency) in [
        ("base", &spec.currency_base),
        ("profit", &spec.currency_profit),
        ("margin", &spec.currency_margin),
    ] {
        if currency.len() > 16
            || (!currency.is_empty() && !currency.bytes().all(|byte| byte.is_ascii_alphanumeric()))
        {
            return Err(format!(
                "MT5 broker instrument {label} currency identifier is malformed."
            ));
        }
    }
    Ok(())
}

fn instrument_specification_fingerprint(
    spec: &Mt5InstrumentSpecification,
) -> Result<String, String> {
    // These three values are live account-currency valuations, not stable contract terms.
    // They are stored in the current projection but excluded from contract revision identity.
    let mut contract = spec.clone();
    contract.tick_value = 0.0;
    contract.tick_value_profit = 0.0;
    contract.tick_value_loss = 0.0;
    let canonical = serde_json::to_vec(&contract).map_err(|error| {
        format!("MT5 instrument specification fingerprint serialization failed: {error}")
    })?;
    Ok(Sha256::digest(canonical)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>())
}

fn timeframe_interval_ms(timeframe: &str) -> Option<i64> {
    let minute = 60_000_i64;
    match timeframe {
        "M1" => Some(minute),
        "M2" => Some(2 * minute),
        "M3" => Some(3 * minute),
        "M4" => Some(4 * minute),
        "M5" => Some(5 * minute),
        "M6" => Some(6 * minute),
        "M10" => Some(10 * minute),
        "M12" => Some(12 * minute),
        "M15" => Some(15 * minute),
        "M20" => Some(20 * minute),
        "M30" => Some(30 * minute),
        "H1" => Some(60 * minute),
        "H2" => Some(120 * minute),
        "H3" => Some(180 * minute),
        "H4" => Some(240 * minute),
        "H6" => Some(360 * minute),
        "H8" => Some(480 * minute),
        "H12" => Some(720 * minute),
        "D1" => Some(1_440 * minute),
        "W1" => Some(10_080 * minute),
        // MT5 MN1 is calendar-month based; there is no fixed duration suitable for gap arithmetic.
        "MN1" => None,
        _ => None,
    }
}

pub fn validate_batch(batch: &Mt5MarketDataBatch) -> Result<(), String> {
    if batch.ticks.len() > MAX_TICKS_PER_FRAME || batch.bars.len() > MAX_BARS_PER_FRAME {
        return Err(
            "MT5 market-data batch exceeds the configured tick or bar cardinality limit.".into(),
        );
    }
    let mut symbols = BTreeSet::new();
    let mut last_time_by_symbol: HashMap<&str, i64> = HashMap::new();
    for tick in &batch.ticks {
        if !valid_symbol(&tick.symbol)
            || tick.time_ms <= 0
            || ![tick.bid, tick.ask, tick.last, tick.volume_real]
                .into_iter()
                .all(f64::is_finite)
            || [tick.bid, tick.ask, tick.last, tick.volume_real]
                .into_iter()
                .any(|value| value < 0.0)
            || tick.volume > i64::MAX as u64
            || tick.flags > i64::MAX as u64
            || (tick.bid > 0.0 && tick.ask > 0.0 && tick.ask < tick.bid)
        {
            return Err(format!(
                "MT5 tick for {} violates symbol, timestamp, quote, volume, or numeric invariants.",
                tick.symbol
            ));
        }
        if let Some(previous) = last_time_by_symbol.insert(tick.symbol.as_str(), tick.time_ms) {
            if tick.time_ms < previous {
                return Err(format!(
                    "MT5 ticks for {} are not ordered by non-decreasing broker timestamp.",
                    tick.symbol
                ));
            }
        }
        symbols.insert(tick.symbol.as_str());
    }

    let mut unique_bars = BTreeSet::new();
    for bar in &batch.bars {
        if !valid_symbol(&bar.symbol)
            || !valid_timeframe(&bar.timeframe)
            || bar.time_ms <= 0
            || !bar.closed
            || ![bar.open, bar.high, bar.low, bar.close]
                .into_iter()
                .all(f64::is_finite)
            || [bar.open, bar.high, bar.low, bar.close]
                .into_iter()
                .any(|value| value <= 0.0)
            || bar.high < bar.open.max(bar.close).max(bar.low)
            || bar.low > bar.open.min(bar.close).min(bar.high)
            || bar.tick_volume > i64::MAX as u64
            || bar.real_volume > i64::MAX as u64
            || bar.spread < 0
        {
            return Err(format!("MT5 bar for {} {} violates closed-bar, OHLC, timestamp, volume, or spread invariants.", bar.symbol, bar.timeframe));
        }
        let key = (bar.symbol.as_str(), bar.timeframe.as_str(), bar.time_ms);
        if !unique_bars.insert(key) {
            return Err(format!(
                "MT5 market-data batch contains a duplicate bar key for {} {} at {}.",
                bar.symbol, bar.timeframe, bar.time_ms
            ));
        }
        symbols.insert(bar.symbol.as_str());
    }
    if symbols.len() > MAX_SYMBOLS_PER_FRAME {
        return Err(format!(
            "MT5 market-data batch exceeds the {MAX_SYMBOLS_PER_FRAME}-symbol safety limit."
        ));
    }
    Ok(())
}

fn valid_symbol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.trim() == value
        && value.bytes().enumerate().all(|(index, byte)| {
            (byte == b' ' && index > 0 && index + 1 < value.len())
                || ((0x21..=0x7e).contains(&byte) && !matches!(byte, b'"' | b'\\' | b'|' | b','))
        })
}

fn valid_timeframe(value: &str) -> bool {
    matches!(
        value,
        "M1" | "M2"
            | "M3"
            | "M4"
            | "M5"
            | "M6"
            | "M10"
            | "M12"
            | "M15"
            | "M20"
            | "M30"
            | "H1"
            | "H2"
            | "H3"
            | "H4"
            | "H6"
            | "H8"
            | "H12"
            | "D1"
            | "W1"
            | "MN1"
    )
}

fn valid_identity(value: &str, max_len: usize) -> bool {
    !value.is_empty() && value.len() <= max_len && !value.chars().any(char::is_control)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn tick_fingerprint(
    client_id: &str,
    broker_server: &str,
    account_login: &str,
    tick: &Mt5Tick,
) -> String {
    let mut hasher = Sha256::new();
    for part in [
        client_id.as_bytes(),
        broker_server.as_bytes(),
        account_login.as_bytes(),
        tick.symbol.as_bytes(),
    ] {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part);
    }
    hasher.update(tick.time_ms.to_be_bytes());
    hasher.update(tick.bid.to_bits().to_be_bytes());
    hasher.update(tick.ask.to_bits().to_be_bytes());
    hasher.update(tick.last.to_bits().to_be_bytes());
    hasher.update(tick.volume.to_be_bytes());
    hasher.update(tick.volume_real.to_bits().to_be_bytes());
    hasher.update(tick.flags.to_be_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

fn bar_fingerprint(bar: &Mt5Bar) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bar.symbol.as_bytes());
    hasher.update(bar.timeframe.as_bytes());
    hasher.update(bar.time_ms.to_be_bytes());
    for value in [bar.open, bar.high, bar.low, bar.close] {
        hasher.update(value.to_bits().to_be_bytes());
    }
    hasher.update(bar.tick_volume.to_be_bytes());
    hasher.update(bar.spread.to_be_bytes());
    hasher.update(bar.real_volume.to_be_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn store() -> Mt5MarketDataStore {
        let path =
            std::env::temp_dir().join(format!("atos-mt5-market-data-{}.sqlite3", Uuid::new_v4()));
        Mt5MarketDataStore::open(path).unwrap()
    }
    fn tick(time_ms: i64) -> Mt5Tick {
        Mt5Tick {
            symbol: "XAUUSD.a".into(),
            time_ms,
            bid: 2300.0,
            ask: 2300.2,
            last: 0.0,
            volume: 0,
            volume_real: 0.0,
            flags: 6,
        }
    }
    fn bar(time_ms: i64) -> Mt5Bar {
        Mt5Bar {
            symbol: "XAUUSD.a".into(),
            timeframe: "M1".into(),
            time_ms,
            open: 2300.0,
            high: 2301.0,
            low: 2299.0,
            close: 2300.5,
            tick_volume: 120,
            spread: 20,
            real_volume: 0,
            closed: true,
        }
    }
    fn specification() -> Mt5InstrumentSpecification {
        Mt5InstrumentSpecification {
            symbol: "XAUUSD.a".into(),
            digits: 2,
            point: 0.01,
            contract_size: 100.0,
            calculation_mode: 0,
            trade_mode: 4,
            execution_mode: 2,
            filling_mode: 3,
            order_mode: 127,
            tick_size: 0.01,
            tick_value: 1.0,
            tick_value_profit: 1.0,
            tick_value_loss: 1.0,
            volume_min: 0.01,
            volume_max: 100.0,
            volume_step: 0.01,
            volume_limit: 0.0,
            stops_level_points: 0,
            freeze_level_points: 0,
            currency_base: "XAU".into(),
            currency_profit: "USD".into(),
            currency_margin: "USD".into(),
            swap_mode: 0,
            swap_long: -1.0,
            swap_short: 0.2,
        }
    }
    fn schedule() -> Mt5SessionSchedule {
        Mt5SessionSchedule {
            symbol: "XAUUSD.a".into(),
            captured_at_ms: 1_800_000_000_000,
            days: (0..7)
                .map(|day| Mt5SessionDay {
                    day_of_week: day,
                    sessions: if (1..=5).contains(&day) {
                        vec![Mt5SessionWindow {
                            index: 0,
                            from_seconds: 60,
                            to_seconds: 86_340,
                        }]
                    } else {
                        Vec::new()
                    },
                })
                .collect(),
        }
    }
    fn commit(
        store: &Mt5MarketDataStore,
        sequence: u64,
        frame_hash: &str,
        batch: &Mt5MarketDataBatch,
    ) -> Result<IngestReceipt, String> {
        store.commit_frame(
            "12345@Broker",
            "session-1234567890",
            sequence,
            frame_hash,
            "12345",
            "Broker",
            batch,
            &specification(),
            &["XAUUSD.a".into()],
            &[schedule()],
            1_800_000_000_000,
        )
    }

    #[test]
    fn validates_broker_native_symbol_names_without_json_or_cursor_delimiters() {
        assert!(valid_symbol("XAUUSD.a"));
        assert!(valid_symbol("US 500+"));
        assert!(valid_symbol("GOLD#"));
        assert!(!valid_symbol(" XAUUSD"));
        assert!(!valid_symbol("XAU|USD"));
        assert!(!valid_symbol("XAU\"USD"));
        assert!(!valid_symbol("XAU,USD"));
    }

    #[test]
    fn rejects_crossed_quotes_and_non_finite_values() {
        let mut batch = Mt5MarketDataBatch::default();
        let mut invalid = tick(1_800_000_000_000);
        invalid.ask = invalid.bid - 1.0;
        batch.ticks.push(invalid);
        assert!(validate_batch(&batch).is_err());
        let mut invalid = tick(1_800_000_000_001);
        invalid.bid = f64::NAN;
        assert!(validate_batch(&Mt5MarketDataBatch {
            ticks: vec![invalid],
            bars: vec![]
        })
        .is_err());
    }

    #[test]
    fn rejects_out_of_order_ticks_and_unclosed_or_invalid_bars() {
        assert!(validate_batch(&Mt5MarketDataBatch {
            ticks: vec![tick(20), tick(19)],
            bars: vec![]
        })
        .is_err());
        let mut unclosed = bar(60_000);
        unclosed.closed = false;
        assert!(validate_batch(&Mt5MarketDataBatch {
            ticks: vec![],
            bars: vec![unclosed]
        })
        .is_err());
        let mut invalid = bar(60_000);
        invalid.low = invalid.high + 1.0;
        assert!(validate_batch(&Mt5MarketDataBatch {
            ticks: vec![],
            bars: vec![invalid]
        })
        .is_err());
    }

    #[test]
    fn frame_commit_is_atomic_idempotent_and_sequence_strict() {
        let store = store();
        let batch = Mt5MarketDataBatch {
            ticks: vec![tick(1_800_000_000_000)],
            bars: vec![bar(1_800_000_000_000)],
        };
        let receipt = commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        assert_eq!(receipt.inserted_ticks, 1);
        assert_eq!(receipt.inserted_bars, 1);
        assert!(!receipt.duplicate_frame);
        let duplicate = commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        assert!(duplicate.duplicate_frame);
        assert_eq!(duplicate.inserted_ticks, 0);
        assert!(commit(&store, 1, &"b".repeat(64), &batch).is_err());
        assert!(commit(&store, 3, &"c".repeat(64), &batch).is_err());
        assert_eq!(
            store
                .last_committed_sequence("12345@Broker", "session-1234567890")
                .unwrap(),
            1
        );
    }

    #[test]
    fn deduplicates_tick_replay_and_audits_bar_corrections() {
        let store = store();
        let mut batch = Mt5MarketDataBatch {
            ticks: vec![tick(1_800_000_000_000)],
            bars: vec![bar(1_800_000_000_000)],
        };
        commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        let mut revised = bar(1_800_000_000_000);
        revised.close = 2300.75;
        batch.bars[0] = revised;
        let receipt = commit(&store, 2, &"b".repeat(64), &batch).unwrap();
        assert_eq!(receipt.inserted_ticks, 0);
        assert_eq!(receipt.revised_bars, 1);
        let connection = store.open_connection().unwrap();
        let revisions: i64 = connection
            .query_row("SELECT COUNT(*) FROM mt5_bar_revisions", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(revisions, 1);
    }

    #[test]
    fn schema_v1_database_is_upgraded_additively_to_session_calendar_v3() {
        let path = std::env::temp_dir().join(format!(
            "atos-mt5-market-data-v1-{}.sqlite3",
            Uuid::new_v4()
        ));
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "user_version", 1_i64)
            .unwrap();
        drop(connection);
        let store = Mt5MarketDataStore::open(&path).unwrap();
        let connection = store.open_connection().unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);
        let table_count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('mt5_instrument_specs','mt5_instrument_spec_revisions','mt5_session_calendars','mt5_session_calendar_revisions')",
            [], |row| row.get(0)
        ).unwrap();
        assert_eq!(table_count, 4);
    }

    #[test]
    fn schema_v2_upgrade_preserves_instrument_projection_and_adds_session_history() {
        let path = std::env::temp_dir().join(format!(
            "atos-mt5-market-data-v2-{}.sqlite3",
            Uuid::new_v4()
        ));
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(
            "CREATE TABLE mt5_instrument_specs (
                broker_server TEXT NOT NULL, account_login TEXT NOT NULL, symbol TEXT NOT NULL,
                specification_json TEXT NOT NULL, fingerprint_sha256 TEXT NOT NULL,
                revision INTEGER NOT NULL, first_observed_at_ms INTEGER NOT NULL,
                last_observed_at_ms INTEGER NOT NULL, changed_at_ms INTEGER,
                PRIMARY KEY(broker_server, account_login, symbol)
             ) WITHOUT ROWID;
             INSERT INTO mt5_instrument_specs VALUES ('Broker','12345','XAUUSD.a','{}', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 1, 100, 100, 100);"
        ).unwrap();
        connection
            .pragma_update(None, "user_version", 2_i64)
            .unwrap();
        drop(connection);
        let store = Mt5MarketDataStore::open(&path).unwrap();
        let connection = store.open_connection().unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 3);
        let preserved: i64 = connection.query_row(
            "SELECT COUNT(*) FROM mt5_instrument_specs WHERE broker_server='Broker' AND account_login='12345' AND symbol='XAUUSD.a' AND revision=1",
            [], |row| row.get(0)
        ).unwrap();
        assert_eq!(
            preserved, 1,
            "schema v2 instrument metadata must survive the additive v3 upgrade"
        );
        let calendar_tables: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('mt5_session_calendars','mt5_session_calendar_revisions')",
            [], |row| row.get(0)
        ).unwrap();
        assert_eq!(calendar_tables, 2);
    }

    #[test]
    fn newer_database_schema_is_rejected_without_downgrade() {
        let path = std::env::temp_dir().join(format!(
            "atos-mt5-market-data-future-{}.sqlite3",
            Uuid::new_v4()
        ));
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "user_version", MARKET_DATA_SCHEMA_VERSION + 1)
            .unwrap();
        drop(connection);
        assert!(Mt5MarketDataStore::open(&path).is_err());
        let connection = Connection::open(&path).unwrap();
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, MARKET_DATA_SCHEMA_VERSION + 1);
    }

    #[test]
    fn synchronization_cursors_resume_each_symbol_and_timeframe_from_durable_storage() {
        let store = store();
        let batch = Mt5MarketDataBatch {
            ticks: vec![tick(1_800_000_000_000)],
            bars: vec![bar(1_799_999_940_000)],
        };
        commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        let symbols = vec!["XAUUSD.a".to_string(), "EURUSD".to_string()];
        let timeframes = vec!["M1".to_string(), "M5".to_string()];
        let cursors = store
            .synchronization_cursors("Broker", "12345", &symbols, &timeframes)
            .unwrap();
        assert_eq!(
            cursors.tick_cursors.get("XAUUSD.a"),
            Some(&1_800_000_000_000)
        );
        assert_eq!(cursors.tick_cursors.get("EURUSD"), Some(&0));
        assert_eq!(
            cursors.bar_cursors.get("XAUUSD.a|M1"),
            Some(&1_799_999_940_000)
        );
        assert_eq!(cursors.bar_cursors.get("XAUUSD.a|M5"), Some(&0));
        assert_eq!(cursors.bar_cursors.get("EURUSD|M1"), Some(&0));
    }

    #[test]
    fn synchronization_cursor_request_rejects_invalid_or_duplicate_series() {
        let store = store();
        let duplicate_symbols = vec!["XAUUSD".to_string(), "XAUUSD".to_string()];
        let timeframes = vec!["M1".to_string()];
        assert!(store
            .synchronization_cursors("Broker", "12345", &duplicate_symbols, &timeframes)
            .is_err());
        let symbols = vec!["XAUUSD".to_string()];
        let invalid_timeframes = vec!["PERIOD_M1".to_string()];
        assert!(store
            .synchronization_cursors("Broker", "12345", &symbols, &invalid_timeframes)
            .is_err());
    }

    #[test]
    fn market_data_store_reports_durable_coverage() {
        let store = store();
        let batch = Mt5MarketDataBatch {
            ticks: vec![tick(1_800_000_000_000)],
            bars: vec![bar(1_800_000_000_000)],
        };
        commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        let status = store.status().unwrap();
        assert_eq!(status.ticks_total, 1);
        assert_eq!(status.bars_total, 1);
        assert_eq!(status.symbols_total, 1);
        assert!(status.storage_available);
    }

    #[test]
    fn series_query_pages_oldest_first_and_returns_full_window_quality() {
        let store = store();
        let base = 1_800_000_000_000_i64;
        for (sequence, timestamp) in [base, base + 60_000, base + 240_000]
            .into_iter()
            .enumerate()
        {
            let batch = Mt5MarketDataBatch {
                ticks: vec![],
                bars: vec![bar(timestamp)],
            };
            commit(
                &store,
                sequence as u64 + 1,
                &format!("{}", char::from(b'a' + sequence as u8)).repeat(64),
                &batch,
            )
            .unwrap();
        }
        let request = Mt5SeriesQuery {
            broker_server: "Broker".into(),
            account_login: "12345".into(),
            symbol: "XAUUSD.a".into(),
            timeframe: "M1".into(),
            from_ms: base,
            to_ms: base + 300_000,
            after_ms: None,
            limit: 2,
        };
        let first_page = store.query_series(&request).unwrap();
        assert_eq!(first_page.bars.len(), 2);
        assert!(first_page.has_more);
        assert_eq!(first_page.next_after_ms, Some(base + 60_000));
        assert_eq!(first_page.bars[0].time_ms, base);
        assert_eq!(first_page.quality.bars_total, 3);
        assert_eq!(first_page.quality.interval_anomaly_count, 1);
        assert_eq!(first_page.dataset_fingerprint_sha256.len(), 64);
        let second_page = store
            .query_series(&Mt5SeriesQuery {
                after_ms: first_page.next_after_ms,
                ..request
            })
            .unwrap();
        assert_eq!(second_page.bars.len(), 1);
        assert!(!second_page.has_more);
        assert_eq!(second_page.bars[0].time_ms, base + 240_000);
        assert_eq!(
            first_page.dataset_fingerprint_sha256,
            second_page.dataset_fingerprint_sha256
        );
    }

    #[test]
    fn series_query_rejects_unbounded_ranges_invalid_pages_and_calendar_month_gap_math() {
        let store = store();
        let base = 1_800_000_000_000_i64;
        let invalid = Mt5SeriesQuery {
            broker_server: "Broker".into(),
            account_login: "12345".into(),
            symbol: "XAUUSD.a".into(),
            timeframe: "M1".into(),
            from_ms: base,
            to_ms: base + 367_i64 * 24 * 60 * 60 * 1000,
            after_ms: None,
            limit: 100,
        };
        assert!(store.query_series(&invalid).is_err());
        let unsafe_epoch = Mt5SeriesQuery {
            to_ms: MAX_SAFE_JSON_INTEGER + 1,
            ..Mt5SeriesQuery {
                broker_server: "Broker".into(),
                account_login: "12345".into(),
                symbol: "XAUUSD.a".into(),
                timeframe: "M1".into(),
                from_ms: base,
                to_ms: base + 60_000,
                after_ms: None,
                limit: 10,
            }
        };
        assert!(store.query_series(&unsafe_epoch).is_err());
        let calendar_month = Mt5SeriesQuery {
            timeframe: "MN1".into(),
            to_ms: base,
            ..invalid.clone()
        };
        assert!(store.query_series(&calendar_month).is_err());
        let invalid_limit = Mt5SeriesQuery {
            limit: 5_001,
            to_ms: base + 60_000,
            ..invalid
        };
        assert!(store.query_series(&invalid_limit).is_err());
    }

    #[test]
    fn series_query_revalidates_payload_fingerprint_before_export() {
        let store = store();
        let base = 1_800_000_000_000_i64;
        let batch = Mt5MarketDataBatch {
            ticks: vec![],
            bars: vec![bar(base)],
        };
        commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        let connection = store.open_connection().unwrap();
        connection.execute(
            "UPDATE mt5_bars SET close=close+1 WHERE broker_server='Broker' AND account_login='12345' AND symbol='XAUUSD.a'",
            [],
        ).unwrap();
        let request = Mt5SeriesQuery {
            broker_server: "Broker".into(),
            account_login: "12345".into(),
            symbol: "XAUUSD.a".into(),
            timeframe: "M1".into(),
            from_ms: base,
            to_ms: base + 60_000,
            after_ms: None,
            limit: 10,
        };
        let error = store.query_series(&request).unwrap_err();
        assert!(
            error.contains("failed its integrity invariants")
                || error.contains("fingerprint is corrupt")
        );
    }

    #[test]
    fn session_calendar_is_versioned_and_bound_into_dataset_provenance() {
        let store = store();
        let batch = Mt5MarketDataBatch {
            ticks: vec![],
            bars: vec![bar(1_800_000_000_000)],
        };
        commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        let request = Mt5SeriesQuery {
            broker_server: "Broker".into(),
            account_login: "12345".into(),
            symbol: "XAUUSD.a".into(),
            timeframe: "M1".into(),
            from_ms: 1_800_000_000_000,
            to_ms: 1_800_000_060_000,
            after_ms: None,
            limit: 10,
        };
        let first = store.query_series(&request).unwrap();
        assert_eq!(first.quality.session_calendar_revision, Some(1));
        assert!(first
            .quality
            .session_calendar_fingerprint_sha256
            .as_ref()
            .is_some_and(|hash| hash.len() == 64));
        assert!(first
            .quality
            .session_calendar_time_basis
            .contains("broker_server_wall_clock"));
        let mut heartbeat = schedule();
        heartbeat.captured_at_ms += 25;
        store
            .commit_frame(
                "12345@Broker",
                "session-1234567890",
                2,
                &"b".repeat(64),
                "12345",
                "Broker",
                &batch,
                &specification(),
                &["XAUUSD.a".into()],
                &[heartbeat],
                1_800_000_000_025,
            )
            .unwrap();
        let unchanged = store.query_series(&request).unwrap();
        assert_eq!(
            unchanged.quality.session_calendar_revision,
            Some(1),
            "capture timestamps must not create false calendar revisions"
        );
        assert_eq!(
            first.dataset_fingerprint_sha256,
            unchanged.dataset_fingerprint_sha256
        );
        let mut changed = schedule();
        changed.captured_at_ms += 50;
        changed.days[1].sessions[0].to_seconds = 86_000;
        store
            .commit_frame(
                "12345@Broker",
                "session-1234567890",
                3,
                &"c".repeat(64),
                "12345",
                "Broker",
                &batch,
                &specification(),
                &["XAUUSD.a".into()],
                &[changed],
                1_800_000_000_050,
            )
            .unwrap();
        let second = store.query_series(&request).unwrap();
        assert_eq!(second.quality.session_calendar_revision, Some(2));
        assert_ne!(
            first.dataset_fingerprint_sha256,
            second.dataset_fingerprint_sha256
        );
        let connection = store.open_connection().unwrap();
        let revisions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM mt5_session_calendar_revisions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revisions, 2);
        assert!(connection
            .execute("DELETE FROM mt5_session_calendar_revisions", [])
            .is_err());
    }

    #[test]
    fn unchanged_session_calendar_is_not_retransmitted_but_retained_state_must_be_fresh() {
        let store = store();
        let batch = Mt5MarketDataBatch::default();
        let symbols = vec!["XAUUSD.a".to_string()];
        assert!(
            store
                .commit_frame(
                    "12345@Broker",
                    "session-1234567890",
                    1,
                    &"a".repeat(64),
                    "12345",
                    "Broker",
                    &batch,
                    &specification(),
                    &symbols,
                    &[],
                    1_800_000_000_000
                )
                .is_err(),
            "a first frame cannot omit the required broker calendar"
        );
        store
            .commit_frame(
                "12345@Broker",
                "session-1234567890",
                1,
                &"b".repeat(64),
                "12345",
                "Broker",
                &batch,
                &specification(),
                &symbols,
                &[schedule()],
                1_800_000_000_000,
            )
            .unwrap();
        let retained = store
            .commit_frame(
                "12345@Broker",
                "session-1234567890",
                2,
                &"c".repeat(64),
                "12345",
                "Broker",
                &batch,
                &specification(),
                &symbols,
                &[],
                1_800_000_001_000,
            )
            .unwrap();
        assert!(!retained.duplicate_frame);
        let calendar = store
            .session_calendar("Broker", "12345", "XAUUSD.a")
            .unwrap();
        assert_eq!(calendar.revision, 1);
        assert_eq!(
            calendar.last_observed_at_ms, 1_800_000_000_000,
            "omitted cached metadata must not pretend to be freshly observed"
        );
        assert!(
            store
                .commit_frame(
                    "12345@Broker",
                    "session-1234567890",
                    3,
                    &"d".repeat(64),
                    "12345",
                    "Broker",
                    &batch,
                    &specification(),
                    &symbols,
                    &[],
                    1_800_003_700_001
                )
                .is_err(),
            "retained calendars older than the one-hour ceiling must fail closed"
        );
    }

    #[test]
    fn session_calendar_rejects_incomplete_week_and_invalid_boundaries() {
        let mut invalid = schedule();
        invalid.days.pop();
        assert!(validate_session_schedule(&invalid).is_err());
        let mut invalid = schedule();
        invalid.days[1].sessions[0].to_seconds = 86_401;
        assert!(validate_session_schedule(&invalid).is_err());
        let mut overnight = schedule();
        overnight.days[1].sessions[0].from_seconds = 80_000;
        overnight.days[1].sessions[0].to_seconds = 2_000;
        assert!(
            validate_session_schedule(&overnight).is_ok(),
            "overnight sessions must remain representable"
        );
    }

    #[test]
    fn instrument_specification_is_versioned_only_on_material_broker_change() {
        let store = store();
        let batch = Mt5MarketDataBatch::default();
        commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        commit(&store, 2, &"b".repeat(64), &batch).unwrap();
        let first = store
            .instrument_specification("Broker", "12345", "XAUUSD.a")
            .unwrap();
        assert_eq!(first.revision, 1);
        assert_eq!(first.completeness, "broker_snapshot_v1_partial");
        assert_eq!(first.specification.contract_size, 100.0);
        let connection = store.open_connection().unwrap();
        let unchanged_revisions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM mt5_instrument_spec_revisions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(unchanged_revisions, 1);
        let mut valuation_changed = specification();
        valuation_changed.tick_value_profit = 1.25;
        valuation_changed.tick_value_loss = 1.10;
        store
            .commit_frame(
                "12345@Broker",
                "session-1234567890",
                3,
                &"c".repeat(64),
                "12345",
                "Broker",
                &batch,
                &valuation_changed,
                &["XAUUSD.a".into()],
                &[schedule()],
                1_800_000_000_050,
            )
            .unwrap();
        let valuation = store
            .instrument_specification("Broker", "12345", "XAUUSD.a")
            .unwrap();
        assert_eq!(
            valuation.revision, 1,
            "dynamic account-currency tick valuations must not create contract revisions"
        );
        assert_eq!(valuation.specification.tick_value_profit, 1.25);
        let mut changed = specification();
        changed.contract_size = 10.0;
        store
            .commit_frame(
                "12345@Broker",
                "session-1234567890",
                4,
                &"d".repeat(64),
                "12345",
                "Broker",
                &batch,
                &changed,
                &["XAUUSD.a".into()],
                &[schedule()],
                1_800_000_000_100,
            )
            .unwrap();
        let updated = store
            .instrument_specification("Broker", "12345", "XAUUSD.a")
            .unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.specification.contract_size, 10.0);
        let revisions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM mt5_instrument_spec_revisions",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revisions, 2);
    }

    #[test]
    fn corrupted_current_instrument_projection_blocks_new_frame_commit() {
        let store = store();
        let batch = Mt5MarketDataBatch::default();
        commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        let connection = store.open_connection().unwrap();
        connection.execute(
            "UPDATE mt5_instrument_specs SET specification_json='{}' WHERE broker_server='Broker' AND account_login='12345' AND symbol='XAUUSD.a'",
            []
        ).unwrap();
        assert!(commit(&store, 2, &"b".repeat(64), &batch).is_err());
        assert_eq!(
            store
                .last_committed_sequence("12345@Broker", "session-1234567890")
                .unwrap(),
            1
        );
    }

    #[test]
    fn instrument_specification_history_is_append_only_and_verified_on_read() {
        let store = store();
        let batch = Mt5MarketDataBatch::default();
        commit(&store, 1, &"a".repeat(64), &batch).unwrap();
        let connection = store.open_connection().unwrap();
        assert!(connection
            .execute("DELETE FROM mt5_instrument_spec_revisions", [])
            .is_err());
        connection
            .execute_batch("DROP TRIGGER trg_mt5_instrument_spec_revisions_no_delete;")
            .unwrap();
        connection
            .execute(
                "DELETE FROM mt5_instrument_spec_revisions WHERE revision=1",
                [],
            )
            .unwrap();
        assert!(store
            .instrument_specification("Broker", "12345", "XAUUSD.a")
            .is_err());
    }

    #[test]
    fn instrument_specification_rejects_invalid_contract_and_currency_metadata() {
        let mut invalid = specification();
        invalid.contract_size = 0.0;
        assert!(validate_instrument_specification(&invalid).is_err());
        let mut invalid = specification();
        invalid.volume_step = 0.0;
        assert!(validate_instrument_specification(&invalid).is_err());
        let mut invalid = specification();
        invalid.currency_profit = "USD|INJECT".into();
        assert!(validate_instrument_specification(&invalid).is_err());
        let mut invalid = specification();
        invalid.swap_long = f64::NAN;
        assert!(validate_instrument_specification(&invalid).is_err());
    }
}
