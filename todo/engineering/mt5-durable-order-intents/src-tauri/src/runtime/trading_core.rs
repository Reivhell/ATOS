use crate::domain::models::{
    latest_closed_bar_at_ms, maximum_live_candle_age_ms, validate_candle_freshness,
    validate_candle_series, *,
};
use crate::domain::state_decision::evaluate_state_commit;
use chrono::Utc;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use uuid::Uuid;

const ZERO_COMMIT_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

fn state_payload_hash(payload: &str) -> String {
    sha256_hex(payload.as_bytes())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn state_commit_record_hash(
    sequence: i64,
    committed_at_ms: i64,
    transition: &str,
    state_sha256: &str,
    previous_hash: &str,
) -> String {
    // Newline-delimited, fixed-order fields avoid ambiguous concatenations.
    sha256_hex(
        format!("{sequence}\n{committed_at_ms}\n{transition}\n{state_sha256}\n{previous_hash}")
            .as_bytes(),
    )
}

/// Validate every durable commit receipt and bind the chain head to the exact current payload.
/// An empty ledger is treated as a legacy database and is anchored explicitly by the next commit.
fn read_state_commit_chain_head(
    connection: &Connection,
    current_payload: Option<&str>,
) -> Result<Option<(i64, String)>, String> {
    let mut statement = connection
        .prepare(
            "SELECT sequence, committed_at_ms, transition, state_sha256, previous_hash, record_hash
         FROM state_commit_ledger ORDER BY sequence ASC",
        )
        .map_err(|error| format!("Cannot inspect durable state-commit ledger: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| format!("Cannot read durable state-commit ledger: {error}"))?;

    let mut expected_sequence = 1_i64;
    let mut expected_previous = ZERO_COMMIT_HASH.to_string();
    let mut head: Option<(i64, String, String)> = None;
    for row in rows {
        let (sequence, committed_at_ms, transition, state_sha256, previous_hash, record_hash) =
            row.map_err(|error| format!("Cannot decode durable state-commit ledger row: {error}"))?;
        if sequence != expected_sequence
            || committed_at_ms <= 0
            || transition.trim().is_empty()
            || transition.len() > 128
            || transition.chars().any(char::is_control)
            || !is_sha256_hex(&state_sha256)
            || !is_sha256_hex(&previous_hash)
            || previous_hash != expected_previous
            || !is_sha256_hex(&record_hash)
            || record_hash
                != state_commit_record_hash(
                    sequence,
                    committed_at_ms,
                    &transition,
                    &state_sha256,
                    &previous_hash,
                )
        {
            return Err(format!("Durable state-commit ledger integrity failure at sequence {sequence}; refusing state writes."));
        }
        expected_previous = record_hash.clone();
        head = Some((sequence, record_hash, state_sha256));
        expected_sequence = expected_sequence
            .checked_add(1)
            .ok_or_else(|| "Durable state-commit ledger sequence overflowed.".to_string())?;
    }
    drop(statement);

    match (&head, current_payload) {
        (Some((sequence, _, recorded_state_hash)), Some(payload)) => {
            if recorded_state_hash != &state_payload_hash(payload) {
                return Err(format!("Durable state-commit ledger does not match the current state payload at sequence {sequence}; refusing state writes."));
            }
        }
        (Some((sequence, _, _)), None) => {
            return Err(format!("Durable state-commit ledger has sequence {sequence} but the account state payload is missing; refusing recovery by overwrite."));
        }
        (None, _) => {}
    }
    Ok(head.map(|(sequence, record_hash, _)| (sequence, record_hash)))
}

fn append_state_commit_record(
    transaction: &Transaction<'_>,
    sequence: i64,
    committed_at_ms: i64,
    transition: &str,
    payload: &str,
    previous_hash: &str,
) -> Result<String, String> {
    let state_sha256 = state_payload_hash(payload);
    let record_hash = state_commit_record_hash(
        sequence,
        committed_at_ms,
        transition,
        &state_sha256,
        previous_hash,
    );
    transaction.execute(
        "INSERT INTO state_commit_ledger(sequence, committed_at_ms, transition, state_sha256, previous_hash, record_hash)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![sequence, committed_at_ms, transition, state_sha256, previous_hash, record_hash],
    ).map_err(|error| format!("Cannot append durable state-commit receipt: {error}"))?;
    Ok(record_hash)
}

pub struct TradingCore {
    pub(crate) state: Mutex<EngineState>,
    persistence_path: PathBuf,
    // Holds an exclusive SQLite transaction on a separate guard database for the full
    // engine lifetime. A second process cannot open the same account and overwrite a stale
    // in-memory snapshot. The connection is intentionally held but otherwise unused.
    _instance_lock: Mutex<Connection>,
}

#[derive(Debug)]
enum StateLoadError {
    UnsupportedSchema(i64),
    Unreadable(String),
}

impl TradingCore {
    /// Convenience constructor for tests and embedded use. Production startup should use
    /// `open` so an unavailable durable store is surfaced instead of silently running.
    pub fn new(persistence_path: PathBuf) -> Self {
        Self::open(persistence_path)
            .unwrap_or_else(|error| panic!("cannot initialize trading core: {error}"))
    }

    /// Open the durable store fail-closed. A database created by a newer version is never
    /// renamed or overwritten by an older binary; corrupt state is backed up before recovery.
    pub fn open(persistence_path: PathBuf) -> Result<Self, String> {
        let instance_lock = Self::acquire_instance_lock(&persistence_path)?;
        // A previously chained account must not be silently reset by the generic corrupt-state
        // recovery path when its payload or commit receipts have been modified.
        Self::verify_existing_state_commit_chain(&persistence_path)?;
        let mut recovery_reason: Option<String> = None;
        let mut state = match Self::load_state(&persistence_path) {
            Ok(Some(state)) if Self::state_is_valid(&state) => state,
            Ok(Some(_)) => {
                recovery_reason = Some("Persisted engine state failed invariant validation".into());
                Self::preserve_corrupt_state(&persistence_path)?;
                Self::fresh_state()
            }
            Ok(None) if Self::has_sqlite_sidecars(&persistence_path) => {
                recovery_reason =
                    Some("Orphaned SQLite WAL/SHM sidecar exists without its main database".into());
                Self::preserve_corrupt_state(&persistence_path)?;
                Self::fresh_state()
            }
            Ok(None) => Self::fresh_state(),
            Err(StateLoadError::UnsupportedSchema(version)) => {
                return Err(format!(
                    "Local database schema version {version} is newer than this application supports. The database was left untouched; use a compatible application version."
                ));
            }
            Err(StateLoadError::Unreadable(_reason))
                if !persistence_path.exists() && !Self::has_sqlite_sidecars(&persistence_path) =>
            {
                Self::fresh_state()
            }
            Err(StateLoadError::Unreadable(reason)) => {
                recovery_reason = Some(reason);
                Self::preserve_corrupt_state(&persistence_path)?;
                Self::fresh_state()
            }
        };
        // Strip synthetic observations left by older builds before any snapshot can expose them.
        // This is data hygiene, not a UI-only label: legacy rows are removed from persisted state.
        let legacy_data_removed = Self::purge_legacy_synthetic_state(&mut state);
        if legacy_data_removed {
            Self::push_event(
                &mut state,
                "WARN",
                "migration",
                "Legacy synthetic observations removed",
                "Synthetic market, screener, or placeholder-agent records from an older build were removed from active state. The kill switch was engaged; refresh trusted provider data and explicitly review risk controls before continuing.".into(),
            );
        }
        // A persisted CONNECTED/isLive flag describes the previous process, not the provider's
        // current reachability. Invalidate it before any snapshot, analysis, or order can observe
        // restored state. Cached values remain available as historical context, never as live data.
        if Self::invalidate_restored_provider_sessions(&mut state) {
            Self::push_event(
                &mut state,
                "WARN",
                "market-data",
                "Persisted provider session invalidated",
                "Live status from the previous application process was cleared. Cached observations remain historical only; refresh providers and validate new observations before market-dependent decisions.".into(),
            );
        }
        // Process restart is a risk boundary. Even when a prior session left the kill switch
        // released, re-arm it so a successful refresh alone cannot resume order entry without
        // explicit operator review in the new process.
        if !state.kill_switch_engaged {
            state.kill_switch_engaged = true;
            Self::push_event(
                &mut state,
                "WARN",
                "risk",
                "Kill switch re-engaged on startup",
                "Order entry was re-armed after process restart. Refresh trusted market data, review current risk limits and positions, then explicitly release the kill switch if paper trading should resume.".into(),
            );
        }
        // Establish today's risk baseline before the first snapshot or order can be evaluated.
        Self::mark_positions(&mut state);
        Self::roll_daily_baseline(&mut state);
        if !Self::state_is_valid(&state) {
            recovery_reason = Some(
                "Persisted state became invalid after marking positions against saved quotes"
                    .into(),
            );
            Self::preserve_corrupt_state(&persistence_path)?;
            state = Self::fresh_state();
            Self::mark_positions(&mut state);
            Self::roll_daily_baseline(&mut state);
            if !Self::state_is_valid(&state) {
                return Err("The default paper account failed startup invariant validation; refusing to start.".into());
            }
        }
        let core = Self {
            state: Mutex::new(state),
            persistence_path,
            _instance_lock: Mutex::new(instance_lock),
        };
        if let Some(reason) = recovery_reason {
            core.record_event(
                "ERROR", "persistence", "State recovery created a new paper account",
                format!("Recovery reason: {reason}. The previous database and/or available SQLite sidecars were moved to a timestamped backup. Review the backup before continuing. No live execution is configured."),
            )?;
        }
        {
            let state = core
                .state
                .lock()
                .map_err(|_| "Trading core lock poisoned during startup".to_string())?;
            core.persist(&state)?;
        }
        Ok(core)
    }

    /// Dedicated durable store for strict-analysis work-plan issuance and one-time consumption.
    /// It is separate from the mutable paper-account snapshot so work-plan replay records can
    /// be committed atomically without coupling their lifetime to the in-memory account state.
    pub fn strict_analysis_store_path(&self) -> PathBuf {
        let mut path = self.persistence_path.as_os_str().to_os_string();
        path.push(".strict-analysis.sqlite3");
        PathBuf::from(path)
    }

    /// Acquire a process-lifetime exclusive lock for this local account. SQLite releases
    /// the file lock automatically if the process exits unexpectedly, unlike stale lockfiles.
    /// The guard database is separate from the state database so holding the lock cannot
    /// block normal state/audit transactions or interfere with WAL recovery.
    fn acquire_instance_lock(path: &Path) -> Result<Connection, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("Cannot create local data directory for instance lock: {error}")
            })?;
        }
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".instance-lock.sqlite3");
        let connection = Connection::open(PathBuf::from(lock_path))
            .map_err(|error| format!("Cannot open local instance-lock database: {error}"))?;
        connection
            .busy_timeout(Duration::from_millis(200))
            .map_err(|error| format!("Cannot configure local instance lock timeout: {error}"))?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS instance_guard (
                singleton INTEGER PRIMARY KEY CHECK(singleton = 1)
             );
             BEGIN EXCLUSIVE;
             INSERT OR IGNORE INTO instance_guard(singleton) VALUES(1);"
        ).map_err(|error| format!(
            "Another application instance may already be using this paper account; refusing to start without the exclusive instance lock: {error}"
        ))?;
        Ok(connection)
    }

    /// Verify a previously established commit chain before startup recovery can move a database
    /// aside. Otherwise a valid-JSON payload tamper could be mistaken for ordinary corruption.
    /// Databases from older releases without this table remain eligible for explicit legacy anchoring.
    fn verify_existing_state_commit_chain(path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Ok(());
        }
        let connection = match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
            Ok(connection) => connection,
            Err(_) => return Ok(()), // Existing load/recovery path handles unreadable SQLite files.
        };
        let database_version: i64 =
            match connection.pragma_query_value(None, "user_version", |row| row.get(0)) {
                Ok(value) => value,
                Err(_) => return Ok(()), // Not a valid SQLite database; recovery path handles corrupt state.
            };
        if database_version > 2 {
            return Err(format!("Local database schema version {database_version} is newer than this application supports."));
        }
        let table_exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'state_commit_ledger')",
            [], |row| row.get(0),
        ).map_err(|error| format!("Cannot inspect state-commit ledger before startup: {error}"))?;
        if !table_exists {
            if database_version >= 2 {
                return Err(
                    "State-commit ledger is missing from schema-v2 account; refusing startup."
                        .into(),
                );
            }
            return Ok(());
        }
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload FROM app_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| {
                format!("Cannot inspect account payload before commit-chain verification: {error}")
            })?;
        let head = read_state_commit_chain_head(&connection, payload.as_deref())?;
        if database_version >= 2 && head.is_none() {
            return Err("Schema-v2 account has no state-commit receipts; refusing startup rather than re-anchoring a missing chain.".into());
        }
        Ok(())
    }

    fn load_state(path: &Path) -> Result<Option<EngineState>, StateLoadError> {
        if !path.exists() {
            return Ok(None);
        }
        let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| {
                StateLoadError::Unreadable(format!("Cannot open local database: {error}"))
            })?;
        let database_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| {
                StateLoadError::Unreadable(format!(
                    "Cannot read local database schema version: {error}"
                ))
            })?;
        if database_version > 2 {
            return Err(StateLoadError::UnsupportedSchema(database_version));
        }
        if database_version != 1 && database_version != 2 {
            return Err(StateLoadError::Unreadable(format!(
                "Unexpected local database schema version {database_version}"
            )));
        }
        let payload: Option<String> = connection
            .query_row(
                "SELECT payload FROM app_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| {
                StateLoadError::Unreadable(format!("Cannot read local state payload: {error}"))
            })?;
        let Some(payload) = payload else {
            return Err(StateLoadError::Unreadable(
                "Local state payload is missing".into(),
            ));
        };
        let payload_value: serde_json::Value = serde_json::from_str(&payload).map_err(|error| {
            StateLoadError::Unreadable(format!("Local state payload is invalid: {error}"))
        })?;
        // Inspect the embedded version before deserializing the whole structure. A future
        // payload may add required fields, but an older binary still must not archive it as corrupt.
        if let Some(version) = payload_value
            .get("schemaVersion")
            .and_then(serde_json::Value::as_u64)
        {
            if version > 1 {
                return Err(StateLoadError::UnsupportedSchema(version as i64));
            }
        }
        let state: EngineState = serde_json::from_value(payload_value).map_err(|error| {
            StateLoadError::Unreadable(format!("Local state payload is incompatible: {error}"))
        })?;
        if state.schema_version > 1 {
            return Err(StateLoadError::UnsupportedSchema(
                state.schema_version as i64,
            ));
        }
        Ok(Some(state))
    }

    fn initialize_database(connection: &Connection) -> Result<(), String> {
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| format!("Cannot inspect local database schema: {e}"))?;
        if version > 2 {
            return Err(format!(
                "Local database schema version {version} is newer than this application supports."
            ));
        }
        if version != 0 && version != 1 && version != 2 {
            return Err(format!(
                "Unsupported local database schema version {version}."
            ));
        }
        if version == 0 {
            connection.execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE IF NOT EXISTS app_state (
                    singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                    schema_version INTEGER NOT NULL,
                    payload TEXT NOT NULL,
                    updated_at_ms INTEGER NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS audit_events (
                    event_id TEXT PRIMARY KEY,
                    timestamp_ms INTEGER NOT NULL,
                    level TEXT NOT NULL,
                    category TEXT NOT NULL,
                    title TEXT NOT NULL,
                    detail TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp ON audit_events(timestamp_ms DESC);
                 CREATE TABLE IF NOT EXISTS state_commit_ledger (
                    sequence INTEGER PRIMARY KEY CHECK(sequence > 0),
                    committed_at_ms INTEGER NOT NULL CHECK(committed_at_ms > 0),
                    transition TEXT NOT NULL,
                    state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64),
                    previous_hash TEXT NOT NULL CHECK(length(previous_hash) = 64),
                    record_hash TEXT NOT NULL UNIQUE CHECK(length(record_hash) = 64)
                 );
                 PRAGMA user_version = 1;
                 COMMIT;"
            ).map_err(|e| format!("Cannot migrate local database: {e}"))?;
        }
        // Add the commit-receipt ledger to existing schema-v1 databases and advance the SQLite
        // schema marker only after the table and append-only triggers are installed. The first
        // post-upgrade commit anchors the exact legacy payload explicitly.
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS state_commit_ledger (
                sequence INTEGER PRIMARY KEY CHECK(sequence > 0),
                committed_at_ms INTEGER NOT NULL CHECK(committed_at_ms > 0),
                transition TEXT NOT NULL,
                state_sha256 TEXT NOT NULL CHECK(length(state_sha256) = 64),
                previous_hash TEXT NOT NULL CHECK(length(previous_hash) = 64),
                record_hash TEXT NOT NULL UNIQUE CHECK(length(record_hash) = 64)
             );
             CREATE TRIGGER IF NOT EXISTS state_commit_ledger_no_update
                BEFORE UPDATE ON state_commit_ledger BEGIN
                    SELECT RAISE(ABORT, 'state commit receipts are immutable');
             END;
             CREATE TRIGGER IF NOT EXISTS state_commit_ledger_no_delete
                BEFORE DELETE ON state_commit_ledger BEGIN
                    SELECT RAISE(ABORT, 'state commit receipts are append-only');
             END;
             CREATE TRIGGER IF NOT EXISTS audit_events_no_update
                BEFORE UPDATE ON audit_events BEGIN
                    SELECT RAISE(ABORT, 'audit events are immutable');
             END;
             CREATE TRIGGER IF NOT EXISTS audit_events_no_delete
                BEFORE DELETE ON audit_events BEGIN
                    SELECT RAISE(ABORT, 'audit events are append-only');
             END;",
            )
            .map_err(|e| format!("Cannot enforce append-only local audit history: {e}"))?;
        if version < 2 {
            // Schema v2 means the receipt chain is mandatory. This marker prevents a missing or
            // emptied receipt table from being misclassified as a pre-chain legacy database.
            connection
                .execute_batch("PRAGMA user_version = 2;")
                .map_err(|e| format!("Cannot mark state-commit-chain schema migration: {e}"))?;
        }
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL; PRAGMA synchronous = FULL; PRAGMA foreign_keys = ON;",
            )
            .map_err(|e| format!("Cannot configure local database durability: {e}"))?;
        Ok(())
    }

    fn persist(&self, state: &EngineState) -> Result<(), String> {
        if !Self::state_is_valid(state) {
            return Err(
                "State commit rejected: the candidate failed native engine invariants.".into(),
            );
        }
        if let Some(parent) = self.persistence_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create local data directory: {e}"))?;
        }
        let mut connection = Connection::open(&self.persistence_path)
            .map_err(|e| format!("Cannot open local SQLite database: {e}"))?;
        let schema_version_before_migration: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| format!("Cannot inspect local schema before state persistence: {e}"))?;
        Self::initialize_database(&connection)?;
        let candidate_value = serde_json::to_value(state)
            .map_err(|e| format!("Cannot encode candidate local state: {e}"))?;
        let payload = serde_json::to_string(state)
            .map_err(|e| format!("Cannot serialize local state: {e}"))?;
        let transaction = connection
            .transaction()
            .map_err(|e| format!("Cannot begin local state transaction: {e}"))?;
        let previous_payload: Option<String> = transaction
            .query_row(
                "SELECT payload FROM app_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Cannot inspect previous local state: {e}"))?;
        let previous_value = previous_payload.as_deref().map(|encoded| {
            serde_json::from_str::<serde_json::Value>(encoded)
                .map_err(|error| format!("State commit rejected: previously persisted state cannot be decoded: {error}"))
        }).transpose()?;
        let decision = evaluate_state_commit(
            previous_value.as_ref(),
            &candidate_value,
            Self::state_is_valid(state),
            Utc::now().timestamp_millis(),
        );
        if !decision.allowed {
            return Err(format!(
                "State commit rejected by strict native decision layer [{}]: {}",
                decision.transition,
                decision.reasons.join(", "),
            ));
        }
        // Verify the existing chain against the payload read in this same transaction. Legacy
        // stores without receipts are anchored once, then all future transitions are chained.
        let mut chain_head =
            read_state_commit_chain_head(&transaction, previous_payload.as_deref())?;
        let has_existing_state = previous_payload.is_some();
        if schema_version_before_migration >= 2 && chain_head.is_none() {
            return Err("Schema-v2 account has no state-commit receipts; refusing to re-anchor a missing chain.".into());
        }
        if chain_head.is_none() {
            if let Some(legacy_payload) = previous_payload.as_deref() {
                let now = Utc::now().timestamp_millis().max(1);
                let legacy_hash = append_state_commit_record(
                    &transaction,
                    1,
                    now,
                    "LEGACY_BASELINE_UNVERIFIED",
                    legacy_payload,
                    ZERO_COMMIT_HASH,
                )?;
                chain_head = Some((1, legacy_hash));
            }
        }
        let committed_at_ms = Utc::now().timestamp_millis().max(1);
        transaction.execute(
            "INSERT INTO app_state(singleton, schema_version, payload, updated_at_ms)
             VALUES(1, ?1, ?2, ?3)
             ON CONFLICT(singleton) DO UPDATE SET schema_version=excluded.schema_version, payload=excluded.payload, updated_at_ms=excluded.updated_at_ms",
            params![state.schema_version, payload, committed_at_ms],
        ).map_err(|e| format!("Cannot persist local state: {e}"))?;
        let (next_sequence, previous_hash) = match chain_head {
            Some((sequence, hash)) => (
                sequence.checked_add(1).ok_or_else(|| {
                    "Durable state-commit ledger sequence overflowed.".to_string()
                })?,
                hash,
            ),
            None => (1, ZERO_COMMIT_HASH.to_string()),
        };
        append_state_commit_record(
            &transaction,
            next_sequence,
            committed_at_ms,
            decision.transition,
            &payload,
            &previous_hash,
        )?;
        // State, state-commit receipt, and audit rows commit together. Audit IDs are idempotent.
        let events_to_write = if has_existing_state {
            state.events.iter().take(10).collect::<Vec<_>>()
        } else {
            state.events.iter().collect::<Vec<_>>()
        };
        for event in events_to_write {
            transaction.execute(
                "INSERT OR IGNORE INTO audit_events(event_id, timestamp_ms, level, category, title, detail) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![event.id, event.timestamp_ms, event.level, event.category, event.title, event.detail],
            ).map_err(|e| format!("Cannot persist audit event: {e}"))?;
        }
        transaction
            .commit()
            .map_err(|e| format!("Cannot commit local state transaction: {e}"))?;
        Ok(())
    }

    /// Persists intentionally manipulated unit-test fixtures, including corrupt states that
    /// production commits must reject. This function is compiled only in the test harness.
    #[cfg(test)]
    fn persist_test_fixture(&self, state: &EngineState) -> Result<(), String> {
        if let Some(parent) = self.persistence_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Cannot create fixture data directory: {e}"))?;
        }
        let mut connection = Connection::open(&self.persistence_path)
            .map_err(|e| format!("Cannot open fixture SQLite database: {e}"))?;
        Self::initialize_database(&connection)?;
        let payload = serde_json::to_string(state)
            .map_err(|e| format!("Cannot serialize fixture state: {e}"))?;
        let transaction = connection
            .transaction()
            .map_err(|e| format!("Cannot begin fixture transaction: {e}"))?;
        let previous_payload: Option<String> = transaction
            .query_row(
                "SELECT payload FROM app_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Cannot inspect state before fixture write: {e}"))?;
        let mut chain_head =
            read_state_commit_chain_head(&transaction, previous_payload.as_deref())?;
        if chain_head.is_none() {
            if let Some(legacy_payload) = previous_payload.as_deref() {
                let legacy_hash = append_state_commit_record(
                    &transaction,
                    1,
                    Utc::now().timestamp_millis().max(1),
                    "LEGACY_BASELINE_UNVERIFIED",
                    legacy_payload,
                    ZERO_COMMIT_HASH,
                )?;
                chain_head = Some((1, legacy_hash));
            }
        }
        let committed_at_ms = Utc::now().timestamp_millis().max(1);
        transaction.execute(
            "INSERT INTO app_state(singleton, schema_version, payload, updated_at_ms)
             VALUES(1, ?1, ?2, ?3)
             ON CONFLICT(singleton) DO UPDATE SET schema_version=excluded.schema_version, payload=excluded.payload, updated_at_ms=excluded.updated_at_ms",
            params![state.schema_version, payload, committed_at_ms],
        ).map_err(|e| format!("Cannot persist test fixture: {e}"))?;
        let (next_sequence, previous_hash) = match chain_head {
            Some((sequence, hash)) => (
                sequence
                    .checked_add(1)
                    .ok_or_else(|| "Fixture state-commit sequence overflowed.".to_string())?,
                hash,
            ),
            None => (1, ZERO_COMMIT_HASH.to_string()),
        };
        append_state_commit_record(
            &transaction,
            next_sequence,
            committed_at_ms,
            "TEST_FIXTURE_BYPASS",
            &payload,
            &previous_hash,
        )?;
        for event in &state.events {
            transaction.execute(
                "INSERT OR IGNORE INTO audit_events(event_id, timestamp_ms, level, category, title, detail) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![event.id, event.timestamp_ms, event.level, event.category, event.title, event.detail],
            ).map_err(|e| format!("Cannot persist test fixture audit event: {e}"))?;
        }
        transaction
            .commit()
            .map_err(|e| format!("Cannot commit test fixture: {e}"))?;
        Ok(())
    }

    fn has_sqlite_sidecars(path: &Path) -> bool {
        let base = path.to_string_lossy();
        PathBuf::from(format!("{base}-wal")).exists()
            || PathBuf::from(format!("{base}-shm")).exists()
    }

    fn preserve_corrupt_state(path: &Path) -> Result<(), String> {
        let has_main = path.exists();
        let has_sidecars = Self::has_sqlite_sidecars(path);
        if !has_main && !has_sidecars {
            return Ok(());
        }
        let stamp = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("state");
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("sqlite3");
        let backup = path.with_file_name(format!("{stem}.corrupt-{stamp}.{extension}"));
        let backup_base = backup.to_string_lossy().into_owned();
        let source_base = path.to_string_lossy().into_owned();
        // Preserve SQLite WAL/SHM sidecars as well as the main database. Renaming only the
        // main file can leave a stale sidecar beside the replacement database after recovery.
        let candidates = [
            (PathBuf::from(&source_base), PathBuf::from(&backup_base)),
            (
                PathBuf::from(format!("{source_base}-wal")),
                PathBuf::from(format!("{backup_base}-wal")),
            ),
            (
                PathBuf::from(format!("{source_base}-shm")),
                PathBuf::from(format!("{backup_base}-shm")),
            ),
        ];
        let mut completed: Vec<(PathBuf, PathBuf)> = Vec::new();
        for (source, destination) in candidates {
            if !source.exists() {
                continue;
            }
            if let Err(error) = fs::rename(&source, &destination) {
                for (original, moved) in completed.iter().rev() {
                    let _ = fs::rename(moved, original);
                }
                return Err(format!(
                    "Cannot preserve corrupt local database file {}: {error}",
                    source.display()
                ));
            }
            completed.push((source, destination));
        }
        Ok(())
    }

    fn timestamp_is_fresh(now_ms: i64, observed_at_ms: i64, maximum_age_ms: i64) -> bool {
        observed_at_ms > 0
            && maximum_age_ms >= 0
            && now_ms >= observed_at_ms
            && now_ms.saturating_sub(observed_at_ms) <= maximum_age_ms
    }

    fn provider_observation_is_fresh(state: &EngineState, now_ms: i64) -> bool {
        state.provider_status.starts_with("CONNECTED")
            && Self::timestamp_is_fresh(
                now_ms,
                state.provider_updated_at_ms,
                MARKET_DATA_OBSERVATION_MAX_AGE_MS,
            )
    }

    fn quote_observation_is_fresh(state: &EngineState, quote: &Quote, now_ms: i64) -> bool {
        quote.is_live
            && Self::provider_observation_is_fresh(state, now_ms)
            && Self::timestamp_is_fresh(
                now_ms,
                quote.updated_at_ms,
                MARKET_DATA_OBSERVATION_MAX_AGE_MS,
            )
            && Self::timestamp_is_fresh(
                now_ms,
                quote.retrieved_at_ms,
                MARKET_DATA_OBSERVATION_MAX_AGE_MS,
            )
    }

    fn state_is_valid(state: &EngineState) -> bool {
        let aggregate_equity =
            state.cash_balance + state.positions.values().map(|p| p.notional).sum::<f64>();
        let aggregate_realized_pnl = state
            .orders
            .iter()
            .filter(|order| order.status.starts_with("FILLED") && order.side == "SELL")
            .map(|order| order.realized_pnl)
            .sum::<f64>();
        let valid_candle = |c: &Candle| {
            c.time > 0
                && c.open.is_finite()
                && c.high.is_finite()
                && c.low.is_finite()
                && c.close.is_finite()
                && c.volume.is_finite()
                && c.open > 0.0
                && c.high > 0.0
                && c.low > 0.0
                && c.close > 0.0
                && c.volume >= 0.0
                && c.high >= c.open.max(c.close).max(c.low)
                && c.low <= c.open.min(c.close).min(c.high)
        };
        state.schema_version == 1
            && state.cash_balance.is_finite()
            && state.cash_balance >= -1e-8
            && state.initial_balance.is_finite()
            && state.initial_balance > 0.0
            && state.candle_retrieved_at_ms >= 0
            && (!state.candles.is_empty() || state.candle_retrieved_at_ms == 0)
            && state.equity_peak.is_finite()
            && state.equity_peak > 0.0
            && state.daily_start_equity.is_finite()
            && state.daily_start_equity > 0.0
            && !state.daily_start_date.is_empty()
            && state.max_drawdown_pct.is_finite()
            && state.max_drawdown_pct <= 0.0
            && state.risk_limits.max_order_notional.is_finite()
            && state.risk_limits.max_order_notional > 0.0
            && state.risk_limits.max_position_notional.is_finite()
            && state.risk_limits.max_position_notional >= state.risk_limits.max_order_notional
            && state.risk_limits.max_daily_loss.is_finite()
            && state.risk_limits.max_daily_loss > 0.0
            && state.risk_limits.fee_bps.is_finite()
            && (0.0..=500.0).contains(&state.risk_limits.fee_bps)
            && state.risk_limits.slippage_bps.is_finite()
            && (0.0..=500.0).contains(&state.risk_limits.slippage_bps)
            && aggregate_equity.is_finite()
            && aggregate_realized_pnl.is_finite()
            && state.positions.iter().all(|(symbol, p)| {
                symbol == &p.symbol
                    && p.quantity.is_finite()
                    && p.quantity > 0.0
                    && p.average_entry.is_finite()
                    && p.average_entry > 0.0
                    && p.mark_price.is_finite()
                    && p.mark_price > 0.0
                    && (p.quantity * p.average_entry).is_finite()
                    && (p.quantity * p.mark_price).is_finite()
                    && p.notional.is_finite()
                    && p.notional >= 0.0
                    && p.unrealized_pnl.is_finite()
                    && state
                        .quotes
                        .get(symbol)
                        .is_none_or(|quote| (p.quantity * quote.price).is_finite())
            })
            && state.quotes.iter().all(|(symbol, q)| {
                symbol == &q.symbol
                    && q.price.is_finite()
                    && q.price > 0.0
                    && q.change_24h_pct.is_finite()
                    && q.volume_24h.is_finite()
                    && q.volume_24h >= 0.0
                    && q.updated_at_ms >= 0
                    && q.retrieved_at_ms >= 0
            })
            && state.orders.iter().all(|o| {
                o.quantity.is_finite()
                    && o.quantity > 0.0
                    && o.requested_price.is_finite()
                    && o.requested_price > 0.0
                    && o.notional.is_finite()
                    && o.notional >= 0.0
                    && o.fee.is_finite()
                    && o.fee >= 0.0
                    && o.realized_pnl.is_finite()
                    && o.fill_price.is_none_or(|p| p.is_finite() && p > 0.0)
            })
            && state.candles.values().all(|rows| {
                rows.iter().all(valid_candle)
                    && rows.windows(2).all(|pair| pair[0].time < pair[1].time)
            })
            && state.screener.len() <= 500
            && state.screener.iter().all(|token| {
                token.price.is_finite()
                    && token.price > 0.0
                    && token.change_1h_pct.is_finite()
                    && token.change_24h_pct.is_finite()
                    && token.volume_24h.is_finite()
                    && token.volume_24h >= 0.0
                    && token.liquidity.is_finite()
                    && token.liquidity >= 0.0
                    && token.momentum_score <= 100
                    && token.risk_score <= 100
                    && token.price_change_5m_pct.is_none_or(f64::is_finite)
                    && token
                        .market_cap_usd
                        .is_none_or(|v| v.is_finite() && v >= 0.0)
                    && token.fdv_usd.is_none_or(|v| v.is_finite() && v >= 0.0)
            })
    }

    fn transact<T>(
        &self,
        operation: impl FnOnce(&mut EngineState) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "Trading core lock poisoned".to_string())?;
        let original = guard.clone();
        Self::mark_positions(&mut guard);
        Self::roll_daily_baseline(&mut guard);
        if !Self::state_is_valid(&guard) {
            *guard = original;
            return Err("Current quote marking would produce an invalid account state; operation was blocked.".into());
        }
        let before = guard.clone();
        let result = match operation(&mut guard) {
            Ok(value) => value,
            Err(error) => {
                *guard = before;
                return Err(error);
            }
        };
        if !Self::state_is_valid(&guard) {
            *guard = before;
            return Err(
                "Operation would produce an invalid engine state; change was rolled back.".into(),
            );
        }
        if let Err(error) = self.persist(&guard) {
            *guard = before;
            return Err(error);
        }
        Ok(result)
    }

    pub fn snapshot(&self) -> Result<DashboardSnapshot, String> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| "Trading core lock poisoned".to_string())?;
        let before = guard.clone();
        Self::mark_positions(&mut guard);
        Self::roll_daily_baseline(&mut guard);
        if !Self::state_is_valid(&guard) {
            *guard = before;
            return Err(
                "Cannot produce dashboard snapshot because state validation failed.".into(),
            );
        }
        if let Err(error) = self.persist(&guard) {
            *guard = before;
            return Err(error);
        }
        Ok(Self::make_snapshot(&guard))
    }

    /// Return a candle series only for an explicitly supported symbol/interval pair.
    /// There is deliberately no fallback to BTCUSDT 5m for unsupported requests.
    pub fn get_candle_series(
        &self,
        request: CandleSeriesRequest,
    ) -> Result<CandleSeriesResponse, String> {
        let symbol = request.symbol.trim().to_ascii_uppercase();
        if symbol.is_empty()
            || symbol.len() > 30
            || !symbol
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err("Candle-series symbol is malformed.".into());
        }
        if symbol != "BTCUSDT" || request.interval != CandleInterval::FiveMinutes {
            return Err(format!(
                "Unsupported candle series {symbol} {}. This adapter currently supports BTCUSDT 5m only; no substitute series was returned.",
                request.interval.as_str()
            ));
        }

        let guard = self
            .state
            .lock()
            .map_err(|_| "Trading core lock poisoned".to_string())?;
        let candles = guard.candles.get(&symbol).cloned().ok_or_else(|| {
            format!("No validated candle series is currently loaded for {symbol} 5m.")
        })?;
        validate_candle_series(&candles, request.interval, 30).map_err(|reason| {
            format!("Cached {symbol} candle series failed integrity validation: {reason}")
        })?;
        let now = Utc::now().timestamp_millis();
        let retrieved_at_ms = guard.candle_retrieved_at_ms;
        let latest_closed_bar_at_ms = latest_closed_bar_at_ms(&candles, request.interval)?;
        let candle_fresh = validate_candle_freshness(
            &candles,
            request.interval,
            now,
            maximum_live_candle_age_ms(request.interval),
        )
        .is_ok();
        let retrieval_fresh =
            Self::timestamp_is_fresh(now, retrieved_at_ms, CANDLE_PROVIDER_RETRIEVAL_MAX_AGE_MS);
        let is_live = candle_fresh
            && retrieval_fresh
            && Self::provider_observation_is_fresh(&guard, now)
            && guard
                .quotes
                .get(&symbol)
                .is_some_and(|quote| Self::quote_observation_is_fresh(&guard, quote, now));
        // Bind every downstream analysis to the exact OHLCV vector returned by this response.
        let dataset_sha256 = crate::application::backtest::dataset_sha256(&candles);
        Ok(CandleSeriesResponse {
            request: CandleSeriesRequest {
                symbol,
                interval: request.interval,
            },
            source: guard.provider_source.clone(),
            provider_status: guard.provider_status.clone(),
            updated_at_ms: retrieved_at_ms,
            retrieved_at_ms,
            latest_closed_bar_at_ms,
            dataset_sha256,
            is_live,
            candles,
        })
    }

    /// Native preflight for research/paper analysis. UI-supplied booleans are never accepted as
    /// risk authority; this check reads and validates the durable Rust-owned account state.
    pub fn strict_analysis_risk_preflight(&self) -> Result<String, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Trading core lock poisoned".to_string())?;
        let before = state.clone();
        Self::mark_positions(&mut state);
        Self::roll_daily_baseline(&mut state);
        if !Self::state_is_valid(&state) {
            *state = before;
            return Err("Rust risk preflight rejected invalid account state.".into());
        }
        if let Err(error) = self.persist(&state) {
            *state = before;
            return Err(error);
        }
        if state.kill_switch_engaged {
            return Err("Rust risk preflight blocked because the kill switch is engaged.".into());
        }
        let equity = state.cash_balance
            + state
                .positions
                .values()
                .map(|position| position.notional)
                .sum::<f64>();
        let daily_pnl = equity - state.daily_start_equity;
        if !equity.is_finite() || equity <= 0.0 || !daily_pnl.is_finite() {
            return Err(
                "Rust risk preflight rejected non-finite or non-positive account equity.".into(),
            );
        }
        if !state.risk_limits.max_daily_loss.is_finite() || state.risk_limits.max_daily_loss <= 0.0
        {
            return Err("Rust risk preflight rejected an invalid maximum-daily-loss limit.".into());
        }
        if daily_pnl <= -state.risk_limits.max_daily_loss {
            return Err(format!(
                "Rust risk preflight blocked: daily PnL ${daily_pnl:.2} reached the maximum-loss threshold -${:.2}.",
                state.risk_limits.max_daily_loss
            ));
        }
        Ok(format!(
            "Rust account preflight passed: kill switch disengaged; equity ${equity:.2}; daily PnL ${daily_pnl:.2}; loss limit -${:.2}. Order-specific limits will be checked again by the paper-order gateway.",
            state.risk_limits.max_daily_loss
        ))
    }

    pub fn place_order(&self, request: PlaceOrderRequest) -> Result<PaperOrder, String> {
        self.transact(|state| {
            let now = Utc::now().timestamp_millis();
            let symbol = request.symbol.trim().to_ascii_uppercase();
            let side = request.side.trim().to_ascii_uppercase();
            if symbol.is_empty() || symbol.len() > 30 || !symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                return Err("symbol must be a valid canonical instrument identifier".into());
            }
            if !["BUY", "SELL"].contains(&side.as_str()) {
                return Err("side must be BUY or SELL".to_string());
            }
            if !request.quantity.is_finite() || request.quantity <= 0.0 {
                return Err("quantity must be a finite number greater than zero".to_string());
            }
            if request.limit_price.is_some_and(|p| !p.is_finite() || p <= 0.0) {
                return Err("limit_price, when supplied, must be a finite positive number".into());
            }
            let client_order_id = request.client_order_id.as_ref().map(|id| id.trim().to_string()).filter(|id| !id.is_empty());
            if client_order_id.as_ref().is_some_and(|id| id.len() > 128 || id.chars().any(char::is_control)) {
                return Err("client_order_id must be at most 128 printable characters".into());
            }
            if let Some(key) = client_order_id.as_ref() {
                if let Some(previous) = state.orders.iter().find(|o| o.client_order_id.as_deref() == Some(key.as_str())) {
                    let same_request = previous.symbol == symbol
                        && previous.side == side
                        && previous.requested_limit_price == request.limit_price
                        && (previous.quantity - request.quantity).abs() <= f64::EPSILON * request.quantity.abs().max(1.0);
                    if !same_request {
                        return Err("client_order_id has already been used for a different order".into());
                    }
                    return Ok(previous.clone());
                }
            }
            let Some(quote) = state.quotes.get(&symbol).cloned() else {
                return Err(format!("No quote is available for {symbol}"));
            };
            if !quote.price.is_finite() || quote.price <= 0.0 {
                return Err(format!("The quote for {symbol} is invalid; order entry is blocked."));
            }
            let quote_age_ms = now.saturating_sub(quote.updated_at_ms);
            let quote_fresh = Self::quote_observation_is_fresh(state, &quote, now);
            let provider_observation_fresh = Self::provider_observation_is_fresh(state, now);
            let current_position = state.positions.get(&symbol).cloned();
            let fee_rate = state.risk_limits.fee_bps / 10_000.0;
            let slip_rate = state.risk_limits.slippage_bps / 10_000.0;
            let market_price = quote.price;
            let estimated_fill = if side == "BUY" { market_price * (1.0 + slip_rate) } else { market_price * (1.0 - slip_rate) };
            let estimated_notional = request.quantity * estimated_fill;
            let estimated_fee = estimated_notional * fee_rate;
            if !estimated_fill.is_finite() || estimated_fill <= 0.0 || !estimated_notional.is_finite() || !estimated_fee.is_finite() {
                return Err("Order estimates overflowed or produced invalid values; order entry is blocked.".into());
            }
            let current_position_value = current_position.as_ref().map(|p| p.quantity * market_price).unwrap_or(0.0);
            let after_position_value = if side == "BUY" { current_position_value + estimated_notional } else { (current_position_value - estimated_notional).max(0.0) };
            // Risk-increasing buys require a fresh, positive, provider-backed mark for every
            // open position. Falling back to a saved notional can understate current portfolio
            // exposure and incorrectly pass daily-loss / position-risk gates.
            let portfolio_marks_fresh = state.positions.iter().all(|(position_symbol, position)| {
                if !position.quantity.is_finite() || position.quantity <= 0.0 {
                    return false;
                }
                state.quotes.get(position_symbol).is_some_and(|position_quote| {
                    position_quote.price.is_finite() && position_quote.price > 0.0
                        && Self::quote_observation_is_fresh(state, position_quote, now)
                })
            });
            let estimated_equity = state.cash_balance + state.positions.iter().map(|(sym, p)| {
                state.quotes.get(sym).map(|q| p.quantity * q.price).unwrap_or(p.notional)
            }).sum::<f64>();
            let daily_pnl = estimated_equity - state.daily_start_equity;
            // Count the immediately realized execution friction for a BUY so an order cannot slip
            // through a nearly breached loss guard merely because the pre-order equity was above it.
            let projected_buy_cost = if side == "BUY" {
                ((estimated_fill - market_price) * request.quantity).max(0.0) + estimated_fee
            } else { 0.0 };
            let projected_daily_pnl = daily_pnl - projected_buy_cost;
            let order_type_supported = request.limit_price.is_none();

            let risk_checks = vec![
                RiskCheck { rule: "Kill switch".into(), passed: !state.kill_switch_engaged, detail: if state.kill_switch_engaged { "Kill switch is engaged; all new paper order intents are blocked. Emergency liquidation is not implemented.".into() } else { "Trading controls are armed.".into() } },
                RiskCheck { rule: "Symbol allowlist".into(), passed: state.quotes.contains_key(&symbol), detail: format!("{symbol} must be on the configured instrument watchlist.") },
                RiskCheck { rule: "Market-data provenance".into(), passed: quote.is_live && provider_observation_fresh, detail: if quote.is_live && provider_observation_fresh { format!("{} supplied this quote through a currently connected and recently observed public provider snapshot.", quote.source) } else { "No trusted quote from a recently confirmed connected provider snapshot is available; the last-known cache cannot authorize new paper-order intents.".into() } },
                RiskCheck { rule: "Market-data freshness".into(), passed: quote_fresh, detail: if quote_fresh { format!("Quote age: {} ms; maximum permitted age is {} ms.", quote_age_ms, MARKET_DATA_OBSERVATION_MAX_AGE_MS) } else { format!("Quote age: {} ms; refresh market data before placing an order.", quote_age_ms) } },
                RiskCheck { rule: "Quantity validity".into(), passed: request.quantity.is_finite() && request.quantity > 0.0, detail: "Quantity must be positive and finite.".into() },
                RiskCheck { rule: "Supported order type".into(), passed: order_type_supported, detail: if order_type_supported { "Market orders only in this paper broker.".into() } else { "Limit/stop orders are not implemented; no resting order will be created.".into() } },
                RiskCheck { rule: "Order notional".into(), passed: estimated_notional <= state.risk_limits.max_order_notional, detail: format!("Estimated ${estimated_notional:.2}; limit ${:.2}.", state.risk_limits.max_order_notional) },
                RiskCheck { rule: "Cash / inventory".into(), passed: if side == "BUY" { state.cash_balance + 1e-9 >= estimated_notional + estimated_fee } else { current_position.as_ref().map(|p| p.quantity + 1e-12 >= request.quantity).unwrap_or(false) }, detail: if side == "BUY" { format!("Available paper cash ${:.2}; estimated debit ${:.2} including fees.", state.cash_balance, estimated_notional + estimated_fee) } else { format!("Requested quantity {:.8}; available {:.8}.", request.quantity, current_position.as_ref().map(|p| p.quantity).unwrap_or(0.0)) } },
                RiskCheck { rule: "Position cap".into(), passed: side == "SELL" || after_position_value <= state.risk_limits.max_position_notional, detail: format!("Estimated position ${after_position_value:.2}; limit ${:.2}.", state.risk_limits.max_position_notional) },
                RiskCheck { rule: "Portfolio mark freshness".into(), passed: side == "SELL" || portfolio_marks_fresh, detail: if side == "SELL" { "Risk-reducing sells do not require fresh marks for unrelated positions.".into() } else if portfolio_marks_fresh { format!("Every open position has a positive, live quote no older than {} ms.", MARKET_DATA_OBSERVATION_MAX_AGE_MS) } else { "At least one open position has a missing, stale, non-live, or invalid quote; risk-increasing buys are blocked until portfolio marks refresh.".into() } },
                RiskCheck { rule: "Daily loss guard".into(), passed: side == "SELL" || projected_daily_pnl > -state.risk_limits.max_daily_loss, detail: if side == "SELL" { format!("Session PnL ${daily_pnl:.2}; risk-reducing sells remain allowed after the loss threshold.") } else { format!("Session PnL before order ${daily_pnl:.2}; estimated after immediate fee/slippage ${projected_daily_pnl:.2}; limit -${:.2} for the UTC day.", state.risk_limits.max_daily_loss) } },
                RiskCheck { rule: "Execution mode".into(), passed: true, detail: "Paper broker only. No live exchange endpoint is configured.".into() },
            ];
            let rejection = risk_checks.iter().find(|c| !c.passed).map(|c| format!("{}: {}", c.rule, c.detail));
            let id = Uuid::new_v4().to_string();
            if let Some(reason) = rejection {
                let order = PaperOrder { id: id.clone(), client_order_id: client_order_id.clone(), symbol: symbol.clone(), side: side.clone(), quantity: request.quantity, requested_price: market_price, requested_limit_price: request.limit_price, fill_price: None, notional: estimated_notional, fee: 0.0, realized_pnl: 0.0, status: "REJECTED".into(), created_at_ms: now, risk_checks: risk_checks.clone() };
                state.orders.insert(0, order.clone());
                Self::push_event(state, "WARN", "risk", "Order rejected by risk gateway", format!("{symbol} {side} rejected: {reason}"));
                return Ok(order);
            }

            let fill_price = estimated_fill;
            let notional = request.quantity * fill_price;
            let fee = notional * fee_rate;
            let mut realized_pnl = 0.0;
            if side == "BUY" {
                state.cash_balance -= notional + fee;
                let position = state.positions.entry(symbol.clone()).or_insert(Position { symbol: symbol.clone(), quantity: 0.0, average_entry: fill_price, mark_price: market_price, notional: 0.0, unrealized_pnl: 0.0 });
                let old_cost_basis = position.quantity * position.average_entry;
                position.quantity += request.quantity;
                // Incorporate entry fees into cost basis so realized PnL on later sells is net of entry and exit fees.
                position.average_entry = (old_cost_basis + notional + fee) / position.quantity;
                position.mark_price = market_price;
                position.notional = position.quantity * market_price;
                position.unrealized_pnl = position.quantity * (market_price - position.average_entry);
            } else {
                let mut remove_position = false;
                if let Some(position) = state.positions.get_mut(&symbol) {
                    realized_pnl = request.quantity * (fill_price - position.average_entry) - fee;
                    position.quantity -= request.quantity;
                    position.mark_price = market_price;
                    position.notional = position.quantity.max(0.0) * market_price;
                    position.unrealized_pnl = position.quantity.max(0.0) * (market_price - position.average_entry);
                    remove_position = position.quantity <= 1e-10;
                }
                if remove_position { state.positions.remove(&symbol); }
                state.cash_balance += notional - fee;
            }
            let order = PaperOrder { id: id.clone(), client_order_id, symbol: symbol.clone(), side: side.clone(), quantity: request.quantity, requested_price: market_price, requested_limit_price: request.limit_price, fill_price: Some(fill_price), notional, fee, realized_pnl, status: "FILLED (PAPER)".into(), created_at_ms: now, risk_checks };
            state.orders.insert(0, order.clone());
            Self::mark_positions(state);
            Self::push_event(state, "INFO", "execution", "Paper order filled", format!("{} {:.8} {} @ {:.6}; fee ${:.4}. No live order sent.", side, request.quantity, symbol, fill_price, fee));
            Ok(order)
        })
    }

    pub fn toggle_agent(&self, agent_id: String, running: bool) -> Result<Agent, String> {
        self.transact(|state| {
            let Some(agent) = state.agents.iter_mut().find(|a| a.id == agent_id) else {
                return Err(format!("Agent '{agent_id}' was not found"));
            };
            agent.status = if running {
                "RUNNING · OBSERVE ONLY".into()
            } else {
                "STOPPED".into()
            };
            agent.last_decision = if running {
                "Observe-only session enabled; awaiting an approved input.".into()
            } else {
                "Stopped by operator. No further observations will be scheduled.".into()
            };
            agent.updated_at_ms = Utc::now().timestamp_millis();
            let cloned = agent.clone();
            Self::push_event(
                state,
                "INFO",
                "agent",
                if running {
                    "Agent started in observe-only mode"
                } else {
                    "Agent stopped"
                },
                format!(
                    "{} — no LLM provider is configured in this build.",
                    cloned.name
                ),
            );
            Ok(cloned)
        })
    }

    pub fn toggle_kill_switch(&self, engaged: bool) -> Result<bool, String> {
        self.transact(|state| {
            state.kill_switch_engaged = engaged;
            Self::push_event(
                state,
                if engaged { "WARN" } else { "INFO" },
                "risk",
                if engaged {
                    "Kill switch engaged"
                } else {
                    "Kill switch released"
                },
                if engaged {
                    "All new paper order intents will be rejected until released."
                } else {
                    "Paper order entry restored, subject to risk checks."
                }
                .into(),
            );
            Ok(state.kill_switch_engaged)
        })
    }

    pub fn reset_paper_account(&self) -> Result<DashboardSnapshot, String> {
        self.transact(|state| {
            state.cash_balance = state.initial_balance;
            state.orders.clear();
            state.positions.clear();
            // Resetting account data must never release an emergency trading lock.
            // Releasing the kill switch is a separate, explicit operator action.
            state.equity_peak = state.initial_balance;
            state.max_drawdown_pct = 0.0;
            state.daily_start_equity = state.initial_balance;
            state.daily_start_date = Utc::now().date_naive().to_string();
            // Reset is a security boundary: a cleared account is never left armed merely
            // because the operator had previously released the kill switch.
            state.kill_switch_engaged = true;
            Self::push_event(
                state, "WARN", "account", "Paper account reset",
                "Paper positions and order history were cleared. No real funds were touched. The kill switch is engaged and must be released separately after fresh market data and risk review.".into(),
            );
            Ok(Self::make_snapshot(state))
        })
    }

    pub fn refresh_market_data(&self) -> Result<DashboardSnapshot, String> {
        Err("refresh_market_data should be routed through the async provider command".into())
    }

    pub fn update_market_data(
        &self,
        incoming_quotes: Vec<Quote>,
        btc_candles: Option<Vec<Candle>>,
        status: String,
        source: String,
    ) -> Result<DashboardSnapshot, String> {
        self.transact(|state| {
            let now = Utc::now().timestamp_millis();
            if incoming_quotes.is_empty() {
                return Err("Market-data update contained no valid quotes; previous state was retained.".into());
            }
            let incoming_count = incoming_quotes.len();
            for mut quote in incoming_quotes {
                quote.symbol = quote.symbol.trim().to_ascii_uppercase();
                if quote.symbol.is_empty() || quote.symbol.len() > 30
                    || !quote.symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                    || !quote.price.is_finite() || quote.price <= 0.0
                    || !quote.change_24h_pct.is_finite()
                    || !quote.volume_24h.is_finite() || quote.volume_24h < 0.0
                    || !Self::timestamp_is_fresh(now, quote.updated_at_ms, MARKET_DATA_OBSERVATION_MAX_AGE_MS) {
                    return Err("Market-data update contained an invalid, stale, or future-dated quote observation; previous state was retained.".into());
                }
                if state.quotes.get(&quote.symbol).is_some_and(|previous| {
                    previous.retrieved_at_ms > 0 && previous.updated_at_ms > quote.updated_at_ms
                }) {
                    return Err(format!("Market-data update for {} moved backwards in exchange time; previous state was retained.", quote.symbol));
                }
                quote.retrieved_at_ms = now;
                quote.source = source.clone();
                quote.is_live = true;
                state.quotes.insert(quote.symbol.clone(), quote);
            }
            let mut candle_integrity_warning: Option<String> = None;
            if let Some(candles) = btc_candles {
                match validate_candle_series(&candles, CandleInterval::FiveMinutes, 30) {
                    Err(reason) => {
                        // Malformed provider batches invalidate the cached series instead of being repaired.
                        state.candles.remove("BTCUSDT");
                        state.candle_retrieved_at_ms = 0;
                        candle_integrity_warning = Some(reason);
                    }
                    Ok(()) => match validate_candle_freshness(
                        &candles, CandleInterval::FiveMinutes, now,
                        maximum_live_candle_age_ms(CandleInterval::FiveMinutes),
                    ) {
                        Ok(_) => {
                            state.candles.insert("BTCUSDT".into(), candles);
                            state.candle_retrieved_at_ms = now;
                        }
                        Err(reason) => {
                            // Replayed or forming bars cannot replace the cache or renew its receipt timestamp.
                            candle_integrity_warning = Some(reason);
                        }
                    },
                }
            }
            state.provider_status = status.clone();
            state.provider_source = source.clone();
            state.provider_updated_at_ms = now;
            Self::mark_positions(state);
            Self::push_event(state, "INFO", "market-data", "Market data refreshed", format!("{source}: {incoming_count} instrument quotes updated; {status}."));
            if let Some(reason) = candle_integrity_warning {
                Self::push_event(state, "WARN", "market-data", "BTCUSDT candle series rejected", format!("Quotes were retained, but the candle series was withheld from charts, indicators, strict analysis, and backtests: {reason}"));
            }
            Ok(Self::make_snapshot(state))
        })
    }

    pub fn record_market_data_failure(&self, reason: String) -> Result<DashboardSnapshot, String> {
        self.transact(|state| {
            state.provider_status = "STALE · LAST KNOWN DATA RETAINED".into();
            // Keep the original source and timestamp. Never relabel old/live quotes as fresh demo data.
            Self::push_event(
                state,
                "WARN",
                "market-data",
                "Market-data refresh failed; retained last-known values",
                reason,
            );
            Ok(Self::make_snapshot(state))
        })
    }

    pub fn update_screener_data(
        &self,
        incoming_tokens: Vec<ScreenerToken>,
    ) -> Result<DashboardSnapshot, String> {
        self.transact(|state| {
            if incoming_tokens.is_empty() {
                return Err("Screener refresh contained no valid token observations; previous records were retained.".into());
            }
            if incoming_tokens.len() > 500 {
                return Err("Screener update exceeded the 500-record safety limit.".into());
            }
            let now = Utc::now().timestamp_millis();
            let mut seen = std::collections::BTreeSet::new();
            for token in &incoming_tokens {
                let key = format!("{}:{}", token.chain.to_ascii_lowercase(), token.token_address.as_deref().unwrap_or(""));
                let address_valid = token.token_address.as_deref().is_some_and(|address| {
                    (20..=64).contains(&address.len()) && address.chars().all(|c| c.is_ascii_alphanumeric())
                });
                if token.symbol.trim().is_empty() || token.symbol.len() > 32
                    || token.name.trim().is_empty() || token.name.len() > 100
                    || token.chain.trim().is_empty() || token.chain.len() > 24
                    || !token.price.is_finite() || token.price <= 0.0
                    || !token.change_1h_pct.is_finite() || !token.change_24h_pct.is_finite()
                    || !token.volume_24h.is_finite() || token.volume_24h < 0.0
                    || !token.liquidity.is_finite() || token.liquidity < 0.0
                    || token.momentum_score > 100 || token.risk_score > 100
                    || token.price_change_5m_pct.is_some_and(|v| !v.is_finite())
                    || token.market_cap_usd.is_some_and(|v| !v.is_finite() || v < 0.0)
                    || token.fdv_usd.is_some_and(|v| !v.is_finite() || v < 0.0)
                    || !address_valid || token.ruleset_version != "momentum-risk-v1"
                    || !seen.insert(key) {
                    return Err("Screener update contained duplicate or invalid token data; previous records were retained atomically.".into());
                }
                for address in [token.pair_address.as_deref(), token.token_address.as_deref()].into_iter().flatten() {
                    if address.len() > 80 || !address.chars().all(|c| c.is_ascii_alphanumeric()) {
                        return Err("Screener update contained an invalid address field; previous records were retained.".into());
                    }
                }
                if let Some(url) = token.pair_url.as_deref() {
                    if !(url.starts_with("https://dexscreener.com/") && url.len() <= 500) {
                        return Err("Screener update contained an unexpected pair URL; previous records were retained.".into());
                    }
                }
                if token.updated_at_ms <= 0 || token.updated_at_ms > now + 60_000 {
                    return Err("Screener update contained an invalid observation timestamp.".into());
                }
            }
            let count = incoming_tokens.len();
            state.screener = incoming_tokens;
            state.screener_provider_status = "CONNECTED · DEX SCREENER BOOST FEED".into();
            state.screener_provider_source = "DEX Screener public API · latest boost feed (promotion-biased)".into();
            state.screener_provider_updated_at_ms = now;
            Self::push_event(state, "INFO", "screener", "Screener data refreshed", format!("Received {count} validated token records from the latest boosted-token discovery feed. Contract safety, holder distribution, and liquidity locks remain unverified."));
            Ok(Self::make_snapshot(state))
        })
    }

    pub fn record_screener_failure(&self, reason: String) -> Result<DashboardSnapshot, String> {
        self.transact(|state| {
            state.screener_provider_status = if state.screener.is_empty() {
                "NOT CONNECTED · NO OBSERVATIONS".into()
            } else {
                "STALE · LAST KNOWN DATA RETAINED".into()
            };
            // The previous records, source and observation timestamp remain untouched.
            Self::push_event(
                state,
                "WARN",
                "screener",
                "Screener refresh failed; retained last-known records",
                reason,
            );
            Ok(Self::make_snapshot(state))
        })
    }

    pub fn record_screener_partial(
        &self,
        warning_count: usize,
        details: String,
    ) -> Result<(), String> {
        self.transact(|state| {
            state.screener_provider_status =
                format!("PARTIAL · DEX SCREENER FEED ({warning_count} chain warning(s))");
            Self::push_event(
                state,
                "WARN",
                "screener",
                "Partial DEX Screener result",
                details,
            );
            Ok(())
        })
    }

    pub fn update_risk_limits(&self, risk_limits: RiskLimits) -> Result<DashboardSnapshot, String> {
        self.transact(|state| {
            let finite_positive = |value: f64| value.is_finite() && value > 0.0;
            if !finite_positive(risk_limits.max_order_notional)
                || !finite_positive(risk_limits.max_position_notional)
                || !finite_positive(risk_limits.max_daily_loss) {
                return Err("Order, position and daily-loss limits must be finite positive numbers.".into());
            }
            if risk_limits.max_order_notional > risk_limits.max_position_notional {
                return Err("Max order notional cannot exceed max position notional.".into());
            }
            if !risk_limits.fee_bps.is_finite() || !(0.0..=500.0).contains(&risk_limits.fee_bps) {
                return Err("Fee must be between 0 and 500 basis points.".into());
            }
            if !risk_limits.slippage_bps.is_finite() || !(0.0..=500.0).contains(&risk_limits.slippage_bps) {
                return Err("Slippage must be between 0 and 500 basis points.".into());
            }
            state.risk_limits = risk_limits.clone();
            Self::push_event(state, "INFO", "risk", "Risk policy updated", format!("Max order ${:.2}; max position ${:.2}; daily loss ${:.2}; fees {:.1} bps; slippage {:.1} bps.", risk_limits.max_order_notional, risk_limits.max_position_notional, risk_limits.max_daily_loss, risk_limits.fee_bps, risk_limits.slippage_bps));
            Ok(Self::make_snapshot(state))
        })
    }

    /// Read the current native risk assumptions without invoking `snapshot()`, which also
    /// marks positions and can persist ordinary valuation changes. A research run must not
    /// cause account writes merely to capture fee/slippage assumptions.
    pub fn research_risk_limits(&self) -> Result<RiskLimits, String> {
        let guard = self
            .state
            .lock()
            .map_err(|_| "Trading core lock poisoned".to_string())?;
        if !Self::state_is_valid(&guard) {
            return Err(
                "Research cannot capture risk assumptions from an invalid account state.".into(),
            );
        }
        Ok(guard.risk_limits.clone())
    }

    pub fn run_backtest(&self, request: BacktestRequest) -> Result<BacktestResult, String> {
        let guard = self
            .state
            .lock()
            .map_err(|_| "Trading core lock poisoned".to_string())?;
        let symbol = request.symbol.trim().to_uppercase();
        let candles = guard
            .candles
            .get(&symbol)
            .ok_or_else(|| format!("No candle data is available for {symbol}"))?
            .clone();
        crate::application::backtest::run(&symbol, &candles, request, &guard.provider_source)
    }

    /// Evaluate the base strategy and optional nested parameter tournament against one
    /// immutable candle/provider snapshot. The engine mutex is held only while cloning
    /// the inputs; no account state is mutated and expensive research work runs unlocked.
    pub fn run_research_backtest_bundle(
        &self,
        request: BacktestRequest,
        include_tournament: bool,
        requested_folds: usize,
    ) -> Result<
        (
            BacktestResult,
            Option<crate::application::backtest::StrategyParameterTournament>,
        ),
        String,
    > {
        let symbol = request.symbol.trim().to_ascii_uppercase();
        let (candles, source) = {
            let guard = self
                .state
                .lock()
                .map_err(|_| "Trading core lock poisoned".to_string())?;
            let candles = guard
                .candles
                .get(&symbol)
                .ok_or_else(|| format!("No candle data is available for {symbol}"))?
                .clone();
            (candles, guard.provider_source.clone())
        };

        if include_tournament {
            if symbol != "BTCUSDT" {
                return Err("Native strategy tournaments currently require BTCUSDT; no substitute market data is permitted.".into());
            }
            validate_candle_series(&candles, CandleInterval::FiveMinutes, 150)
                .map_err(|reason| format!("Strategy tournament requires at least 150 validated BTCUSDT 5m candles: {reason}"))?;
        }

        let base_result =
            crate::application::backtest::run(&symbol, &candles, request.clone(), &source)?;
        let tournament = if include_tournament {
            Some(
                crate::application::backtest::run_walk_forward_parameter_tournament(
                    &symbol,
                    &candles,
                    &request,
                    &source,
                    requested_folds,
                )?,
            )
        } else {
            None
        };
        Ok((base_result, tournament))
    }

    pub fn record_event(
        &self,
        level: &str,
        category: &str,
        title: &str,
        detail: String,
    ) -> Result<(), String> {
        self.transact(|state| {
            Self::push_event(state, level, category, title, detail);
            Ok(())
        })
    }

    pub fn audit_history(&self, requested_limit: usize) -> Result<Vec<AuditEvent>, String> {
        let limit = requested_limit.clamp(1, 5_000) as i64;
        let connection = Connection::open(&self.persistence_path)
            .map_err(|e| format!("Cannot open audit database: {e}"))?;
        Self::initialize_database(&connection)?;
        let mut statement = connection.prepare(
            "SELECT event_id, timestamp_ms, level, category, title, detail FROM audit_events ORDER BY timestamp_ms DESC, event_id DESC LIMIT ?1"
        ).map_err(|e| format!("Cannot query audit history: {e}"))?;
        let rows = statement
            .query_map([limit], |row| {
                Ok(AuditEvent {
                    id: row.get(0)?,
                    timestamp_ms: row.get(1)?,
                    level: row.get(2)?,
                    category: row.get(3)?,
                    title: row.get(4)?,
                    detail: row.get(5)?,
                })
            })
            .map_err(|e| format!("Cannot read audit history: {e}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Cannot decode audit history: {e}"))
    }

    fn roll_daily_baseline(state: &mut EngineState) {
        let today = Utc::now().date_naive().to_string();
        if state.daily_start_date != today {
            state.daily_start_equity =
                state.cash_balance + state.positions.values().map(|p| p.notional).sum::<f64>();
            state.daily_start_date = today;
        }
    }

    fn push_event(
        state: &mut EngineState,
        level: &str,
        category: &str,
        title: &str,
        detail: String,
    ) {
        state.events.insert(
            0,
            AuditEvent {
                id: Uuid::new_v4().to_string(),
                timestamp_ms: Utc::now().timestamp_millis(),
                level: level.into(),
                category: category.into(),
                title: title.into(),
                detail,
            },
        );
        // SQLite is the durable audit ledger. Keep the embedded snapshot bounded to avoid unbounded JSON growth.
        state.events.truncate(1_500);
    }

    fn mark_positions(state: &mut EngineState) {
        for (symbol, position) in state.positions.iter_mut() {
            if let Some(quote) = state.quotes.get(symbol) {
                position.mark_price = quote.price;
                position.notional = position.quantity * quote.price;
                position.unrealized_pnl =
                    position.quantity * (quote.price - position.average_entry);
            }
        }
        let equity = state.cash_balance + state.positions.values().map(|p| p.notional).sum::<f64>();
        state.equity_peak = state.equity_peak.max(equity);
        let drawdown = if state.equity_peak > 0.0 {
            ((equity / state.equity_peak) - 1.0).min(0.0) * 100.0
        } else {
            0.0
        };
        state.max_drawdown_pct = state.max_drawdown_pct.min(drawdown);
    }

    fn make_snapshot(state: &EngineState) -> DashboardSnapshot {
        let positions: Vec<Position> = state.positions.values().cloned().collect();
        let unrealized_pnl = positions.iter().map(|p| p.unrealized_pnl).sum::<f64>();
        let realized_pnl = state
            .orders
            .iter()
            .filter(|o| o.status.starts_with("FILLED") && o.side == "SELL")
            .map(|o| o.realized_pnl)
            .sum::<f64>();
        let equity = state.cash_balance + positions.iter().map(|p| p.notional).sum::<f64>();
        let closed: Vec<&PaperOrder> = state
            .orders
            .iter()
            .filter(|o| o.status.starts_with("FILLED") && o.side == "SELL")
            .collect();
        let wins = closed.iter().filter(|o| o.realized_pnl > 0.0).count();
        let closed_trade_count = closed.len();
        let win_rate_pct = if closed_trade_count == 0 {
            None
        } else {
            Some((wins as f64 / closed_trade_count as f64) * 100.0)
        };
        let candles = state.candles.get("BTCUSDT").cloned().unwrap_or_default();
        let candle_series_available =
            validate_candle_series(&candles, CandleInterval::FiveMinutes, 30).is_ok();
        let snapshot_now_ms = Utc::now().timestamp_millis();
        let candle_series_live = candle_series_available
            && validate_candle_freshness(
                &candles,
                CandleInterval::FiveMinutes,
                snapshot_now_ms,
                maximum_live_candle_age_ms(CandleInterval::FiveMinutes),
            )
            .is_ok()
            && Self::timestamp_is_fresh(
                snapshot_now_ms,
                state.candle_retrieved_at_ms,
                CANDLE_PROVIDER_RETRIEVAL_MAX_AGE_MS,
            )
            && Self::provider_observation_is_fresh(state, snapshot_now_ms)
            && state.quotes.get("BTCUSDT").is_some_and(|quote| {
                Self::quote_observation_is_fresh(state, quote, snapshot_now_ms)
            });
        let market_data_capabilities = MarketDataCapabilities {
            quote_symbols: state.quotes.keys().cloned().collect(),
            // This is the complete set of series implemented by the current public adapter, not
            // a promise that every exchange-listed interval or quote symbol has candle history.
            candle_series: vec![CandleSeriesCapability {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes,
                supported: true,
                available: candle_series_available,
                is_live: candle_series_live,
                candle_count: candles.len(),
                latest_candle_time_ms: candles.last().map(|candle| candle.time),
                latest_closed_bar_at_ms: latest_closed_bar_at_ms(
                    &candles,
                    CandleInterval::FiveMinutes,
                )
                .ok(),
                retrieved_at_ms: state.candle_retrieved_at_ms,
                source: state.provider_source.clone(),
                note: if !candle_series_available {
                    "No validated 30+ candle BTCUSDT 5m series is currently cached.".into()
                } else if candle_series_live {
                    "Validated closed candles from the currently fresh public provider snapshot."
                        .into()
                } else {
                    "A candle series is cached but not verified fresh against a current provider snapshot; market research is blocked until refresh.".into()
                },
            }],
        };
        DashboardSnapshot {
            mode: "PAPER · SIMULATED".into(),
            initial_balance: state.initial_balance,
            cash_balance: state.cash_balance,
            equity,
            realized_pnl,
            unrealized_pnl,
            day_change_pct: if state.daily_start_equity > 0.0 {
                (equity / state.daily_start_equity - 1.0) * 100.0
            } else {
                0.0
            },
            max_drawdown_pct: state.max_drawdown_pct,
            win_rate_pct,
            closed_trade_count,
            positions,
            orders: state.orders.iter().take(20).cloned().collect(),
            agents: state.agents.clone(),
            events: state.events.iter().take(30).cloned().collect(),
            quotes: state.quotes.values().cloned().collect(),
            candles,
            market_data_capabilities,
            screener: state.screener.clone(),
            screener_provider_status: state.screener_provider_status.clone(),
            screener_provider_source: state.screener_provider_source.clone(),
            screener_provider_updated_at_ms: state.screener_provider_updated_at_ms,
            kill_switch_engaged: state.kill_switch_engaged,
            provider_status: state.provider_status.clone(),
            provider_source: state.provider_source.clone(),
            provider_updated_at_ms: state.provider_updated_at_ms,
            risk_limits: state.risk_limits.clone(),
        }
    }

    fn is_legacy_synthetic_source(value: &str) -> bool {
        let value = value.to_ascii_lowercase();
        value.contains("fixture")
            || value.contains("synthetic")
            || value.contains("browser preview")
            || value.contains("bundled deterministic")
            || value.contains("demo fixture")
    }

    /// Remove the exact synthetic fallback data shipped by earlier application versions.
    /// Persisted state is untrusted input; labels in the UI are not sufficient to make it safe.
    fn purge_legacy_synthetic_state(state: &mut EngineState) -> bool {
        let mut changed = false;
        let market_source_is_synthetic = Self::is_legacy_synthetic_source(&state.provider_status)
            || Self::is_legacy_synthetic_source(&state.provider_source);
        if market_source_is_synthetic {
            changed |= !state.quotes.is_empty() || !state.candles.is_empty();
            state.quotes.clear();
            state.candles.clear();
            state.candle_retrieved_at_ms = 0;
            state.provider_status = "NOT CONNECTED · NO MARKET DATA".into();
            state.provider_source = "No market-data provider observation loaded".into();
            state.provider_updated_at_ms = 0;
        } else {
            let old_quote_count = state.quotes.len();
            let btc_quote_was_synthetic = state
                .quotes
                .get("BTCUSDT")
                .is_some_and(|quote| Self::is_legacy_synthetic_source(&quote.source));
            state
                .quotes
                .retain(|_, quote| !Self::is_legacy_synthetic_source(&quote.source));
            changed |= state.quotes.len() != old_quote_count;
            if btc_quote_was_synthetic {
                changed |= state.candles.remove("BTCUSDT").is_some();
                state.candle_retrieved_at_ms = 0;
            }
            if state.quotes.is_empty() {
                state.provider_status = "NOT CONNECTED · NO MARKET DATA".into();
                state.provider_source = "No market-data provider observation loaded".into();
                state.provider_updated_at_ms = 0;
            }
        }

        let screener_source_is_synthetic =
            Self::is_legacy_synthetic_source(&state.screener_provider_status)
                || Self::is_legacy_synthetic_source(&state.screener_provider_source);
        let old_screener_count = state.screener.len();
        if screener_source_is_synthetic {
            state.screener.clear();
        } else {
            state.screener.retain(|token| {
                token.ruleset_version != "fixture-v0"
                    && !Self::is_legacy_synthetic_source(&token.source)
            });
        }
        changed |= state.screener.len() != old_screener_count;
        if state.screener.is_empty() {
            if screener_source_is_synthetic || state.screener_provider_status.contains("DEMO") {
                changed = true;
            }
            state.screener_provider_status = "NOT CONNECTED · NO OBSERVATIONS".into();
            state.screener_provider_source = "No screener provider observation loaded".into();
            state.screener_provider_updated_at_ms = 0;
        }

        let old_agent_count = state.agents.len();
        // These identifiers were shipped as inert, bundled supervisors; they never represented
        // configured runtime agents. Remove them rather than migrate false operating status.
        state.agents.retain(|agent| {
            !matches!(
                agent.id.as_str(),
                "xau-scalper" | "news-watcher" | "meme-hunter"
            )
        });
        changed |= state.agents.len() != old_agent_count;
        if changed {
            state.kill_switch_engaged = true;
        }
        changed
    }

    /// Invalidate data-plane connection claims restored from a prior process. Last-known values
    /// are retained for historical inspection, but no cached quote or screener row remains live.
    fn invalidate_restored_provider_sessions(state: &mut EngineState) -> bool {
        let mut changed = false;

        for quote in state.quotes.values_mut() {
            if quote.is_live {
                quote.is_live = false;
                changed = true;
            }
        }
        if state.quotes.is_empty() {
            if state.provider_status != "NOT CONNECTED · NO MARKET DATA"
                || state.provider_source != "No market-data provider observation loaded"
                || state.provider_updated_at_ms != 0
            {
                changed = true;
            }
            state.provider_status = "NOT CONNECTED · NO MARKET DATA".into();
            state.provider_source = "No market-data provider observation loaded".into();
            state.provider_updated_at_ms = 0;
        } else {
            let stale_status = "STALE · RESTART REVALIDATION REQUIRED";
            if state.provider_status != stale_status {
                state.provider_status = stale_status.into();
                changed = true;
            }
        }

        if state.screener.is_empty() {
            if state.screener_provider_status != "NOT CONNECTED · NO OBSERVATIONS"
                || state.screener_provider_source != "No screener provider observation loaded"
                || state.screener_provider_updated_at_ms != 0
            {
                changed = true;
            }
            state.screener_provider_status = "NOT CONNECTED · NO OBSERVATIONS".into();
            state.screener_provider_source = "No screener provider observation loaded".into();
            state.screener_provider_updated_at_ms = 0;
        } else {
            let stale_status = "STALE · RESTART REVALIDATION REQUIRED";
            if state.screener_provider_status != stale_status {
                state.screener_provider_status = stale_status.into();
                changed = true;
            }
        }
        changed
    }

    fn fresh_state() -> EngineState {
        let mut state = EngineState {
            schema_version: 1,
            cash_balance: 10_000.0,
            initial_balance: 10_000.0,
            daily_start_equity: 10_000.0,
            daily_start_date: Utc::now().date_naive().to_string(),
            max_drawdown_pct: 0.0,
            orders: Vec::new(),
            positions: BTreeMap::new(),
            // Runtime agents are registered only through a real configured runtime path.
            agents: Vec::new(),
            events: Vec::new(),
            // No market or screener observations are manufactured at startup/recovery.
            quotes: BTreeMap::new(),
            candles: BTreeMap::new(),
            candle_retrieved_at_ms: 0,
            screener: Vec::new(),
            screener_provider_status: "NOT CONNECTED · NO OBSERVATIONS".into(),
            screener_provider_source: "No screener provider observation loaded".into(),
            screener_provider_updated_at_ms: 0,
            // New and recovered accounts require an explicit operator release before paper orders.
            kill_switch_engaged: true,
            provider_status: "NOT CONNECTED · NO MARKET DATA".into(),
            provider_source: "No market-data provider observation loaded".into(),
            provider_updated_at_ms: 0,
            risk_limits: RiskLimits {
                max_order_notional: 500.0,
                max_position_notional: 2_000.0,
                max_daily_loss: 250.0,
                fee_bps: 10.0,
                slippage_bps: 5.0,
            },
            equity_peak: 10_000.0,
        };
        Self::push_event(
            &mut state,
            "INFO",
            "system",
            "Local paper account initialized",
            "A new paper account is ready. No market observations, screener records, or agent sessions are preloaded; connect a provider before research or paper order entry.".into(),
        );
        Self::push_event(
            &mut state,
            "WARN",
            "market-data",
            "Market data unavailable",
            "No market-data provider snapshot has been loaded. Market-dependent analysis and paper order entry must remain unavailable until a real provider observation passes validation.".into(),
        );
        Self::push_event(
            &mut state,
            "INFO",
            "risk",
            "Risk gateway initialized in fail-closed state",
            "The kill switch starts engaged and requires an explicit operator release. Paper order intents remain subject to deterministic Rust risk checks; no live venue execution is configured.".into(),
        );
        // A fresh paper account starts with a balance, not with fabricated market evidence.
        debug_assert!(
            state.quotes.is_empty() && state.candles.is_empty() && state.screener.is_empty()
        );
        debug_assert!(state.agents.is_empty());
        debug_assert!(state.kill_switch_engaged);
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("atos-{label}-{}.json", Uuid::new_v4()))
    }

    fn enable_test_market_data(core: &TradingCore) {
        let now = Utc::now().timestamp_millis();
        let interval_ms = CandleInterval::FiveMinutes.duration_ms();
        let aligned_now = now.div_euclid(interval_ms) * interval_ms;
        let mut previous = 67_000.0;
        let candles = (0..120)
            .map(|index| {
                let close = 67_000.0 + index as f64 * 0.75;
                let candle = Candle {
                    time: aligned_now - ((120 - index) as i64 * interval_ms),
                    open: previous,
                    high: previous.max(close) + 2.0,
                    low: previous.min(close) - 2.0,
                    close,
                    volume: 10.0 + index as f64,
                };
                previous = close;
                candle
            })
            .collect::<Vec<_>>();
        let mut state = core.state.lock().unwrap();
        state.quotes.insert(
            "BTCUSDT".into(),
            Quote {
                symbol: "BTCUSDT".into(),
                price: previous,
                change_24h_pct: 0.0,
                volume_24h: 1_000_000.0,
                source: "isolated Rust unit-test input".into(),
                updated_at_ms: now,
                retrieved_at_ms: now,
                is_live: true,
            },
        );
        state.candles.insert("BTCUSDT".into(), candles);
        state.candle_retrieved_at_ms = now;
        state.provider_status = "CONNECTED · UNIT TEST SOURCE".into();
        state.provider_source = "isolated Rust unit-test input".into();
        state.provider_updated_at_ms = now;
        state.kill_switch_engaged = false;
        core.persist_test_fixture(&state)
            .expect("persist unit-test market fixture");
    }

    fn buy_request(client_order_id: &str) -> PlaceOrderRequest {
        PlaceOrderRequest {
            symbol: "BTCUSDT".into(),
            side: "BUY".into(),
            quantity: 0.001,
            limit_price: None,
            client_order_id: Some(client_order_id.into()),
        }
    }

    #[test]
    fn durable_audit_rows_reject_update_and_delete() {
        let path = test_path("append-only-audit");
        let core = TradingCore::new(path.clone());
        let connection = rusqlite::Connection::open(&path).unwrap();
        let update = connection.execute("UPDATE audit_events SET title = 'tampered'", []);
        let delete = connection.execute("DELETE FROM audit_events", []);
        assert!(update.is_err(), "durable audit entries must reject updates");
        assert!(
            delete.is_err(),
            "durable audit entries must reject deletion"
        );
        drop(connection);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn durable_state_commit_chain_detects_payload_tampering_and_rejects_receipt_mutation() {
        let path = test_path("state-commit-chain");
        let core =
            TradingCore::open(path.clone()).expect("new account should create a genesis receipt");
        let connection = rusqlite::Connection::open(&path).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM state_commit_ledger", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert!(
            count >= 1,
            "every persisted state must have a commit receipt"
        );
        assert!(connection
            .execute("UPDATE state_commit_ledger SET transition = 'forged'", [])
            .is_err());
        assert!(connection
            .execute("DELETE FROM state_commit_ledger", [])
            .is_err());
        drop(connection);
        drop(core);

        let tamper = rusqlite::Connection::open(&path).unwrap();
        tamper.execute_batch("UPDATE app_state SET payload = replace(payload, 'No market-data provider observation loaded', 'Tampered market-data provider observation') WHERE singleton = 1;").unwrap();
        drop(tamper);
        let reopened = TradingCore::open(path.clone());
        assert!(
            reopened.is_err(),
            "payload tampering must fail closed on next startup"
        );
        let _ = fs::remove_file(&path);
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".instance-lock.sqlite3");
        let _ = fs::remove_file(PathBuf::from(lock_path));
    }

    #[test]
    fn schema_v2_refuses_startup_when_the_receipt_chain_is_removed() {
        let path = test_path("state-commit-chain-removed");
        let core = TradingCore::open(path.clone()).expect("new account should establish schema v2");
        drop(core);
        let tamper = rusqlite::Connection::open(&path).unwrap();
        tamper
            .execute_batch(
                "DROP TRIGGER state_commit_ledger_no_delete; DELETE FROM state_commit_ledger;",
            )
            .unwrap();
        drop(tamper);
        let reopened = TradingCore::open(path.clone());
        assert!(
            reopened.is_err(),
            "schema-v2 database must not re-anchor an emptied receipt chain"
        );
        let _ = fs::remove_file(&path);
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".instance-lock.sqlite3");
        let _ = fs::remove_file(PathBuf::from(lock_path));
    }

    #[test]
    fn second_engine_instance_is_rejected_and_lock_is_released_on_close() {
        let path = test_path("single-instance-lock");
        let first = TradingCore::open(path.clone()).unwrap();
        let second = TradingCore::open(path.clone());
        let error = second
            .err()
            .expect("second process must not open the same account");
        assert!(
            error.contains("exclusive instance lock"),
            "unexpected lock error: {error}"
        );
        assert!(
            first.snapshot().is_ok(),
            "first instance should remain usable after the second is rejected"
        );
        drop(first);

        let reopened = TradingCore::open(path.clone())
            .expect("lock must be released when the owning instance closes");
        assert_eq!(reopened.snapshot().unwrap().equity, 10_000.0);
        drop(reopened);
        let _ = fs::remove_file(&path);
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".instance-lock.sqlite3");
        let _ = fs::remove_file(PathBuf::from(lock_path));
    }

    #[test]
    fn restored_provider_session_is_invalidated_until_a_new_refresh_succeeds() {
        let path = test_path("restart-provider-session-invalidated");
        {
            let core = TradingCore::open(path.clone()).unwrap();
            enable_test_market_data(&core);
            let state = core.state.lock().unwrap();
            core.persist_test_fixture(&state).unwrap();
        }

        let restored = TradingCore::open(path.clone()).unwrap();
        let snapshot = restored.snapshot().unwrap();
        assert!(!snapshot.provider_status.starts_with("CONNECTED"));
        assert_eq!(
            snapshot.provider_status,
            "STALE · RESTART REVALIDATION REQUIRED"
        );
        assert!(
            snapshot.kill_switch_engaged,
            "a restarted process must require explicit operator re-arm"
        );
        assert!(snapshot
            .events
            .iter()
            .any(|event| event.title == "Kill switch re-engaged on startup"));
        assert!(
            snapshot.quotes.iter().all(|quote| !quote.is_live),
            "restored quote live claims must be invalidated"
        );
        assert!(!snapshot
            .market_data_capabilities
            .candle_series
            .iter()
            .any(|series| series.is_live));
        let candle_series = restored
            .get_candle_series(CandleSeriesRequest {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes,
            })
            .unwrap();
        assert!(
            !candle_series.is_live,
            "persisted session metadata must not certify the restored candle series"
        );

        let blocked = restored
            .place_order(buy_request("restart-revalidation-required"))
            .unwrap();
        assert_eq!(blocked.status, "REJECTED");
        assert!(blocked
            .risk_checks
            .iter()
            .any(|check| check.rule == "Market-data provenance" && !check.passed));

        // Only a successful observation in this process can restore connected/live status.
        let now = Utc::now().timestamp_millis();
        let quotes = vec![Quote {
            symbol: "BTCUSDT".into(),
            price: 67_089.25,
            change_24h_pct: 0.0,
            volume_24h: 1_000_000.0,
            source: "isolated Rust unit-test input".into(),
            updated_at_ms: now,
            retrieved_at_ms: now,
            is_live: true,
        }];
        restored
            .update_market_data(
                quotes,
                Some(snapshot.candles),
                "CONNECTED · UNIT TEST SOURCE".into(),
                "isolated Rust unit-test input".into(),
            )
            .unwrap();
        let refreshed = restored.snapshot().unwrap();
        assert!(refreshed.provider_status.starts_with("CONNECTED"));
        assert!(refreshed
            .quotes
            .iter()
            .find(|quote| quote.symbol == "BTCUSDT")
            .is_some_and(|quote| quote.is_live));
        assert!(
            refreshed.kill_switch_engaged,
            "fresh market data after restart must not release the kill switch automatically"
        );
        drop(restored);
        let _ = fs::remove_file(&path);
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".instance-lock.sqlite3");
        let _ = fs::remove_file(PathBuf::from(lock_path));
    }

    #[test]
    fn stale_replayed_candles_do_not_refresh_the_cached_market_observation() {
        let path = test_path("stale-candle-replay");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let before = core
            .get_candle_series(CandleSeriesRequest {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes,
            })
            .unwrap();
        let stale = before
            .candles
            .iter()
            .map(|candle| Candle {
                time: candle.time - 100 * 300_000,
                ..candle.clone()
            })
            .collect();
        core.update_market_data(
            vec![Quote {
                symbol: "BTCUSDT".into(),
                price: 67_100.0,
                change_24h_pct: 0.0,
                volume_24h: 1_000_000.0,
                source: "fresh quote response".into(),
                updated_at_ms: Utc::now().timestamp_millis(),
                retrieved_at_ms: 0,
                is_live: true,
            }],
            Some(stale),
            "CONNECTED · UNIT TEST SOURCE".into(),
            "isolated Rust unit-test input".into(),
        )
        .unwrap();
        let after = core
            .get_candle_series(CandleSeriesRequest {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes,
            })
            .unwrap();
        assert_eq!(after.retrieved_at_ms, before.retrieved_at_ms);
        assert_eq!(
            after.latest_closed_bar_at_ms,
            before.latest_closed_bar_at_ms
        );
        assert!(after.is_live);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn stale_first_candle_batch_is_not_cached_as_live() {
        let path = test_path("stale-first-candle-batch");
        let core = TradingCore::new(path.clone());
        let now = Utc::now().timestamp_millis();
        let interval_ms = CandleInterval::FiveMinutes.duration_ms();
        let aligned_now = now.div_euclid(interval_ms) * interval_ms;
        let candles = (0..30)
            .map(|index| Candle {
                time: aligned_now - (200 + 30 - index) * interval_ms,
                open: 100.0,
                high: 102.0,
                low: 99.0,
                close: 101.0,
                volume: 10.0,
            })
            .collect();
        core.update_market_data(
            vec![Quote {
                symbol: "BTCUSDT".into(),
                price: 101.0,
                change_24h_pct: 0.0,
                volume_24h: 1_000.0,
                source: "fresh quote response".into(),
                updated_at_ms: now,
                retrieved_at_ms: now,
                is_live: true,
            }],
            Some(candles),
            "CONNECTED · UNIT TEST SOURCE".into(),
            "isolated Rust unit-test input".into(),
        )
        .unwrap();
        let snapshot = core.snapshot().unwrap();
        assert!(!snapshot.market_data_capabilities.candle_series[0].available);
        assert_eq!(
            snapshot.market_data_capabilities.candle_series[0].retrieved_at_ms,
            0
        );
        assert!(core
            .get_candle_series(CandleSeriesRequest {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes
            })
            .is_err());
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_candle_refresh_preserves_cache_and_original_receipt_time() {
        let path = test_path("missing-candle-refresh");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let before = core
            .get_candle_series(CandleSeriesRequest {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes,
            })
            .unwrap();
        core.update_market_data(
            vec![Quote {
                symbol: "BTCUSDT".into(),
                price: 67_100.0,
                change_24h_pct: 0.0,
                volume_24h: 1_000_000.0,
                source: "quote-only refresh".into(),
                updated_at_ms: Utc::now().timestamp_millis(),
                retrieved_at_ms: 0,
                is_live: true,
            }],
            None,
            "PARTIAL · UNIT TEST SOURCE".into(),
            "quote-only test provider".into(),
        )
        .unwrap();
        let after = core
            .get_candle_series(CandleSeriesRequest {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes,
            })
            .unwrap();
        assert_eq!(after.retrieved_at_ms, before.retrieved_at_ms);
        assert!(!after.is_live);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn stale_quote_replay_cannot_refresh_the_cached_observation() {
        let path = test_path("stale-quote-replay");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let before = core
            .snapshot()
            .unwrap()
            .quotes
            .into_iter()
            .find(|quote| quote.symbol == "BTCUSDT")
            .unwrap();
        let stale_observation = before.updated_at_ms - 180_000;
        let result = core.update_market_data(
            vec![Quote {
                symbol: "BTCUSDT".into(),
                price: before.price + 5.0,
                change_24h_pct: 0.0,
                volume_24h: before.volume_24h,
                source: "stale replay fixture".into(),
                updated_at_ms: stale_observation,
                retrieved_at_ms: 0,
                is_live: true,
            }],
            None,
            "CONNECTED · REPLAY FIXTURE".into(),
            "replay test provider".into(),
        );
        assert!(result.unwrap_err().contains("stale, or future-dated"));
        let after = core
            .snapshot()
            .unwrap()
            .quotes
            .into_iter()
            .find(|quote| quote.symbol == "BTCUSDT")
            .unwrap();
        assert_eq!(after.price, before.price);
        assert_eq!(after.updated_at_ms, before.updated_at_ms);
        assert_eq!(after.retrieved_at_ms, before.retrieved_at_ms);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn quote_observation_time_cannot_move_backwards_within_a_live_session() {
        let path = test_path("quote-time-regression");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let before = core
            .snapshot()
            .unwrap()
            .quotes
            .into_iter()
            .find(|quote| quote.symbol == "BTCUSDT")
            .unwrap();
        let result = core.update_market_data(
            vec![Quote {
                symbol: "BTCUSDT".into(),
                price: before.price - 1.0,
                change_24h_pct: 0.0,
                volume_24h: before.volume_24h,
                source: "out-of-order fixture".into(),
                updated_at_ms: before.updated_at_ms - 1,
                retrieved_at_ms: 0,
                is_live: true,
            }],
            None,
            "CONNECTED · OUT-OF-ORDER FIXTURE".into(),
            "replay test provider".into(),
        );
        assert!(result
            .unwrap_err()
            .contains("moved backwards in exchange time"));
        let after = core
            .snapshot()
            .unwrap()
            .quotes
            .into_iter()
            .find(|quote| quote.symbol == "BTCUSDT")
            .unwrap();
        assert_eq!(after.price, before.price);
        assert_eq!(after.updated_at_ms, before.updated_at_ms);
        assert_eq!(after.retrieved_at_ms, before.retrieved_at_ms);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn future_provider_timestamps_never_satisfy_freshness_checks() {
        let now = Utc::now().timestamp_millis();
        assert!(TradingCore::timestamp_is_fresh(
            now,
            now,
            MARKET_DATA_OBSERVATION_MAX_AGE_MS
        ));
        assert!(TradingCore::timestamp_is_fresh(
            now,
            now - MARKET_DATA_OBSERVATION_MAX_AGE_MS,
            MARKET_DATA_OBSERVATION_MAX_AGE_MS
        ));
        assert!(!TradingCore::timestamp_is_fresh(
            now,
            now + 1,
            MARKET_DATA_OBSERVATION_MAX_AGE_MS
        ));
        assert!(!TradingCore::timestamp_is_fresh(
            now,
            0,
            MARKET_DATA_OBSERVATION_MAX_AGE_MS
        ));
        assert!(!TradingCore::timestamp_is_fresh(
            now,
            now - MARKET_DATA_OBSERVATION_MAX_AGE_MS - 1,
            MARKET_DATA_OBSERVATION_MAX_AGE_MS
        ));
    }

    #[test]
    fn typed_candle_series_returns_only_the_exact_supported_symbol_and_interval() {
        let path = test_path("typed-candle-series");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let response = core
            .get_candle_series(CandleSeriesRequest {
                symbol: " btcusdt ".into(),
                interval: CandleInterval::FiveMinutes,
            })
            .unwrap();
        assert_eq!(response.request.symbol, "BTCUSDT");
        assert_eq!(response.request.interval, CandleInterval::FiveMinutes);
        assert!(response.candles.len() >= 30);
        assert!(response
            .candles
            .windows(2)
            .all(|pair| pair[1].time - pair[0].time == 300_000));
        assert_eq!(
            response.dataset_sha256,
            crate::application::backtest::dataset_sha256(&response.candles)
        );
        assert!(is_sha256_hex(&response.dataset_sha256));
        assert!(core
            .get_candle_series(CandleSeriesRequest {
                symbol: "ETHUSDT".into(),
                interval: CandleInterval::FiveMinutes
            })
            .unwrap_err()
            .contains("no substitute series was returned"));
        assert!(core
            .get_candle_series(CandleSeriesRequest {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::OneHour
            })
            .unwrap_err()
            .contains("no substitute series was returned"));
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn market_data_capabilities_separate_quote_symbols_from_candle_series() {
        let path = test_path("market-data-capabilities");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let snapshot = core.snapshot().unwrap();
        assert_eq!(
            snapshot.market_data_capabilities.quote_symbols,
            vec!["BTCUSDT".to_string()]
        );
        assert!(snapshot
            .market_data_capabilities
            .candle_series
            .iter()
            .any(|series| series.symbol == "BTCUSDT"
                && series.interval == CandleInterval::FiveMinutes
                && series.supported
                && series.available));
        assert!(!snapshot
            .market_data_capabilities
            .candle_series
            .iter()
            .any(|series| series.symbol == "ETHUSDT"));
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn candle_interval_serde_uses_exchange_style_literals() {
        assert_eq!(
            serde_json::to_string(&CandleInterval::FiveMinutes).unwrap(),
            "\"5m\""
        );
        assert_eq!(
            serde_json::from_str::<CandleInterval>("\"1h\"").unwrap(),
            CandleInterval::OneHour
        );
    }

    #[test]
    fn paper_buy_and_sell_account_for_fees() {
        let path = test_path("buy-sell");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let original_cash = core.snapshot().unwrap().cash_balance;
        let buy = core.place_order(buy_request("buy-1")).unwrap();
        assert_eq!(buy.status, "FILLED (PAPER)");
        assert!(core.snapshot().unwrap().cash_balance < original_cash);

        let sell = core
            .place_order(PlaceOrderRequest {
                symbol: "BTCUSDT".into(),
                side: "SELL".into(),
                quantity: 0.001,
                limit_price: None,
                client_order_id: Some("sell-1".into()),
            })
            .unwrap();
        assert_eq!(sell.status, "FILLED (PAPER)");
        assert!(
            sell.realized_pnl < 0.0,
            "entry and exit fees/slippage should be reflected in PnL"
        );
        assert!(core.snapshot().unwrap().positions.is_empty());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn idempotency_key_returns_the_original_order() {
        let path = test_path("idempotency");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let first = core.place_order(buy_request("intent-unique")).unwrap();
        let duplicate = core.place_order(buy_request("intent-unique")).unwrap();
        assert_eq!(first.id, duplicate.id);
        assert_eq!(core.snapshot().unwrap().orders.len(), 1);
        let mut conflict = buy_request("intent-unique");
        conflict.quantity = 0.002;
        assert!(core.place_order(conflict).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn kill_switch_rejects_new_order_intents() {
        let path = test_path("kill-switch");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        core.toggle_kill_switch(true).unwrap();
        let result = core.place_order(buy_request("blocked-intent")).unwrap();
        assert_eq!(result.status, "REJECTED");
        assert!(result
            .risk_checks
            .iter()
            .any(|check| check.rule == "Kill switch" && !check.passed));
        assert_eq!(core.snapshot().unwrap().cash_balance, 10_000.0);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn resetting_paper_account_does_not_release_kill_switch() {
        let path = test_path("reset-preserves-kill-switch");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        core.toggle_kill_switch(true).unwrap();
        let reset = core.reset_paper_account().unwrap();
        assert!(
            reset.kill_switch_engaged,
            "account reset must not re-arm order entry"
        );
        let order = core.place_order(buy_request("reset-while-locked")).unwrap();
        assert_eq!(order.status, "REJECTED");
        assert!(order
            .risk_checks
            .iter()
            .any(|check| check.rule == "Kill switch" && !check.passed));
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn resetting_after_kill_switch_release_reengages_it_and_requires_explicit_rearm() {
        let path = test_path("reset-reengages-kill-switch");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        core.toggle_kill_switch(false).unwrap();
        assert!(!core.snapshot().unwrap().kill_switch_engaged);
        let reset = core.reset_paper_account().unwrap();
        assert!(
            reset.kill_switch_engaged,
            "reset must not leave the account armed"
        );
        let blocked = core
            .place_order(buy_request("reset-rearm-required"))
            .unwrap();
        assert_eq!(blocked.status, "REJECTED");
        assert!(blocked
            .risk_checks
            .iter()
            .any(|check| check.rule == "Kill switch" && !check.passed));
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn failed_provider_refresh_blocks_orders_even_when_cached_quote_is_young() {
        let path = test_path("provider-failure-provenance");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        core.toggle_kill_switch(false).unwrap();
        core.record_market_data_failure("provider timeout".into())
            .unwrap();

        let order = core
            .place_order(buy_request("provider-failure-blocked-buy"))
            .unwrap();
        assert_eq!(order.status, "REJECTED");
        assert!(order
            .risk_checks
            .iter()
            .any(|check| check.rule == "Market-data provenance" && !check.passed));
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn stale_quote_fails_closed() {
        let path = test_path("stale-quote");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        {
            let mut state = core.state.lock().unwrap();
            state.quotes.get_mut("BTCUSDT").unwrap().updated_at_ms =
                Utc::now().timestamp_millis() - 180_000;
            core.persist_test_fixture(&state).unwrap();
        }
        let result = core.place_order(buy_request("stale-intent")).unwrap();
        assert_eq!(result.status, "REJECTED");
        assert!(result
            .risk_checks
            .iter()
            .any(|check| check.rule == "Market-data freshness" && !check.passed));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_mark_to_market_update_is_rolled_back() {
        let path = test_path("invalid-mark-rollback");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        {
            let mut state = core.state.lock().unwrap();
            let quote = state.quotes.get("BTCUSDT").unwrap().clone();
            state.positions.insert(
                "BTCUSDT".into(),
                Position {
                    symbol: "BTCUSDT".into(),
                    quantity: 10.0,
                    average_entry: quote.price,
                    mark_price: quote.price,
                    notional: 10.0 * quote.price,
                    unrealized_pnl: 0.0,
                },
            );
            core.persist_test_fixture(&state).unwrap();
        }
        let before = core.snapshot().unwrap();
        let result = core.update_market_data(
            vec![Quote {
                symbol: "BTCUSDT".into(),
                price: f64::MAX,
                change_24h_pct: 0.0,
                volume_24h: 1.0,
                source: "malformed extreme provider value".into(),
                updated_at_ms: Utc::now().timestamp_millis(),
                retrieved_at_ms: 0,
                is_live: true,
            }],
            None,
            "CONNECTED".into(),
            "test provider".into(),
        );
        assert!(
            result.is_err(),
            "overflowed mark-to-market state must not commit"
        );
        let after = core.snapshot().unwrap();
        assert_eq!(
            after
                .quotes
                .iter()
                .find(|quote| quote.symbol == "BTCUSDT")
                .unwrap()
                .price,
            before
                .quotes
                .iter()
                .find(|quote| quote.symbol == "BTCUSDT")
                .unwrap()
                .price
        );
        assert!(after.equity.is_finite());
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn strict_analysis_risk_preflight_uses_rust_account_state_and_blocks_kill_switch() {
        let path = test_path("strict-analysis-risk-preflight");
        let core = TradingCore::new(path.clone());
        let error = core.strict_analysis_risk_preflight().unwrap_err();
        assert!(error.contains("kill switch is engaged"));
        drop(core);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("sqlite3-wal"));
        let _ = fs::remove_file(path.with_extension("sqlite3-shm"));
        let _ = fs::remove_file(path.with_extension("sqlite3.lock"));
    }

    #[test]
    fn invalid_risk_limits_are_rejected_without_mutation() {
        let path = test_path("risk-limits");
        let core = TradingCore::new(path.clone());
        let before = core.snapshot().unwrap().risk_limits;
        let invalid = RiskLimits {
            max_order_notional: 3_000.0,
            max_position_notional: 2_000.0,
            max_daily_loss: 100.0,
            fee_bps: 10.0,
            slippage_bps: 5.0,
        };
        assert!(core.update_risk_limits(invalid).is_err());
        let after = core.snapshot().unwrap().risk_limits;
        assert_eq!(before.max_order_notional, after.max_order_notional);
        assert_eq!(before.max_position_notional, after.max_position_notional);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_state_is_preserved_before_recovery() {
        let path = test_path("corrupt");
        fs::write(&path, b"{not-json").unwrap();
        let core = TradingCore::new(path.clone());
        assert_eq!(core.snapshot().unwrap().equity, 10_000.0);
        assert!(fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(path.file_stem().unwrap().to_string_lossy().as_ref())
                    && entry.file_name().to_string_lossy().contains(".corrupt-")
            }));
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn future_schema_database_is_rejected_without_mutation_or_backup() {
        let path = test_path("future-schema");
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection.execute_batch(
                "CREATE TABLE protected_data(value TEXT NOT NULL); INSERT INTO protected_data VALUES('keep-me'); PRAGMA user_version = 99;"
            ).unwrap();
        }
        let error = match TradingCore::open(path.clone()) {
            Err(error) => error,
            Ok(_) => panic!("future schema unexpectedly opened"),
        };
        assert!(error.contains("newer than this application supports"));
        assert!(path.exists(), "newer-version database must stay in place");
        let connection = rusqlite::Connection::open(&path).unwrap();
        let value: String = connection
            .query_row("SELECT value FROM protected_data", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "keep-me");
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            99
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn future_embedded_state_schema_is_rejected_without_mutation() {
        let path = test_path("future-payload-schema");
        {
            let core = TradingCore::new(path.clone());
            let mut state = core.state.lock().unwrap();
            state.schema_version = 99;
            core.persist_test_fixture(&state).unwrap();
        }
        let error = match TradingCore::open(path.clone()) {
            Err(error) => error,
            Ok(_) => panic!("future embedded state schema unexpectedly opened"),
        };
        assert!(error.contains("schema version 99 is newer than this application supports"));
        assert!(
            path.exists(),
            "future payload schema must not be renamed as corrupt"
        );
        let connection = rusqlite::Connection::open(&path).unwrap();
        let payload: String = connection
            .query_row(
                "SELECT payload FROM app_state WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let restored: EngineState = serde_json::from_str(&payload).unwrap();
        assert_eq!(restored.schema_version, 99);
        drop(connection);
        for entry in fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
        {
            if entry
                .file_name()
                .to_string_lossy()
                .contains("future-payload-schema")
                && entry.file_name().to_string_lossy().contains(".corrupt-")
            {
                let _ = fs::remove_file(entry.path());
            }
        }
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_database_and_sqlite_sidecars_are_preserved_together() {
        let path = test_path("corrupt-sidecars");
        fs::write(&path, b"invalid-db").unwrap();
        let wal = PathBuf::from(format!("{}-wal", path.to_string_lossy()));
        let shm = PathBuf::from(format!("{}-shm", path.to_string_lossy()));
        fs::write(&wal, b"wal-evidence").unwrap();
        fs::write(&shm, b"shm-evidence").unwrap();
        TradingCore::preserve_corrupt_state(&path).unwrap();
        let backups = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|candidate| {
                candidate.to_string_lossy().contains("corrupt-sidecars")
                    && candidate.to_string_lossy().contains(".corrupt-")
                    && !candidate.to_string_lossy().ends_with("-wal")
                    && !candidate.to_string_lossy().ends_with("-shm")
            })
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        let backup = backups.first().unwrap();
        assert!(backup.exists());
        assert!(PathBuf::from(format!("{}-wal", backup.to_string_lossy())).exists());
        assert!(PathBuf::from(format!("{}-shm", backup.to_string_lossy())).exists());
        assert!(!path.exists() && !wal.exists() && !shm.exists());
        for entry in fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
        {
            if entry
                .file_name()
                .to_string_lossy()
                .contains("corrupt-sidecars")
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    #[test]
    fn startup_fails_when_durable_storage_cannot_be_created() {
        let blocker = test_path("storage-parent-blocker");
        fs::write(&blocker, b"not-a-directory").unwrap();
        let database_path = blocker.join("agentic-trading-os.sqlite3");
        let result = TradingCore::open(database_path);
        assert!(
            result.is_err(),
            "the application must not start with silently unavailable persistence"
        );
        let _ = fs::remove_file(blocker);
    }

    #[test]
    fn startup_preserves_state_if_saved_quotes_make_marked_equity_overflow() {
        let path = test_path("startup-mark-overflow");
        {
            let core = TradingCore::new(path.clone());
            let mut state = core.state.lock().unwrap();
            for symbol in ["BTCUSDT", "ETHUSDT"] {
                state.quotes.insert(
                    symbol.into(),
                    Quote {
                        symbol: symbol.into(),
                        price: f64::MAX * 0.6,
                        change_24h_pct: 0.0,
                        volume_24h: 1.0,
                        source: "trusted-provider".into(),
                        updated_at_ms: Utc::now().timestamp_millis(),
                        retrieved_at_ms: 0,
                        is_live: false,
                    },
                );
                state.positions.insert(
                    symbol.into(),
                    Position {
                        symbol: symbol.into(),
                        quantity: 1.0,
                        average_entry: 1.0,
                        mark_price: 1.0,
                        notional: 1.0,
                        unrealized_pnl: 0.0,
                    },
                );
            }
            core.persist_test_fixture(&state).unwrap();
        }
        let restored = TradingCore::open(path.clone()).unwrap();
        assert_eq!(restored.snapshot().unwrap().equity, 10_000.0);
        assert!(restored
            .snapshot()
            .unwrap()
            .events
            .iter()
            .any(|event| event.title == "State recovery created a new paper account"));
        drop(restored);
        for entry in fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
        {
            if entry
                .file_name()
                .to_string_lossy()
                .contains("startup-mark-overflow")
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    #[test]
    fn orphaned_sqlite_sidecars_are_preserved_before_new_database_creation() {
        let path = test_path("orphaned-sidecars");
        let wal = PathBuf::from(format!("{}-wal", path.to_string_lossy()));
        let shm = PathBuf::from(format!("{}-shm", path.to_string_lossy()));
        fs::write(&wal, b"orphan-wal").unwrap();
        fs::write(&shm, b"orphan-shm").unwrap();
        let core = TradingCore::open(path.clone()).unwrap();
        assert_eq!(core.snapshot().unwrap().equity, 10_000.0);
        assert!(
            path.exists(),
            "a clean database should be created after orphan sidecars are moved"
        );
        assert!(
            !wal.exists() && !shm.exists(),
            "orphaned sidecars must not remain beside the new database"
        );
        assert!(fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .contains("orphaned-sidecars")
                    && entry.file_name().to_string_lossy().contains(".corrupt-")
            }));
        drop(core);
        for entry in fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
        {
            if entry
                .file_name()
                .to_string_lossy()
                .contains("orphaned-sidecars")
            {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    #[test]
    fn client_order_id_remains_idempotent_after_restart() {
        let path = test_path("idempotency-restart");
        let first_id = {
            let core = TradingCore::new(path.clone());
            enable_test_market_data(&core);
            core.place_order(buy_request("durable-intent")).unwrap().id
        };
        let restored = TradingCore::new(path.clone());
        let duplicate = restored.place_order(buy_request("durable-intent")).unwrap();
        assert_eq!(duplicate.id, first_id);
        assert_eq!(restored.snapshot().unwrap().orders.len(), 1);
        drop(restored);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn risk_increasing_buy_requires_fresh_marks_for_every_open_position_but_sell_remains_available()
    {
        let path = test_path("portfolio-mark-freshness");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        core.toggle_kill_switch(false).unwrap();
        {
            let mut state = core.state.lock().unwrap();
            let quote = state.quotes.get("BTCUSDT").unwrap().clone();
            state.positions.insert(
                "BTCUSDT".into(),
                Position {
                    symbol: "BTCUSDT".into(),
                    quantity: 0.005,
                    average_entry: quote.price,
                    mark_price: quote.price,
                    notional: 0.005 * quote.price,
                    unrealized_pnl: 0.0,
                },
            );
            state.positions.insert(
                "ETHUSDT".into(),
                Position {
                    symbol: "ETHUSDT".into(),
                    quantity: 0.02,
                    average_entry: 2_500.0,
                    mark_price: 2_500.0,
                    notional: 50.0,
                    unrealized_pnl: 0.0,
                },
            );
            core.persist_test_fixture(&state).unwrap();
        }

        let buy = core.place_order(buy_request("portfolio-mark-buy")).unwrap();
        assert_eq!(buy.status, "REJECTED");
        assert!(buy
            .risk_checks
            .iter()
            .any(|check| check.rule == "Portfolio mark freshness" && !check.passed));

        let sell = core
            .place_order(PlaceOrderRequest {
                symbol: "BTCUSDT".into(),
                side: "SELL".into(),
                quantity: 0.001,
                limit_price: None,
                client_order_id: Some("portfolio-mark-sell".into()),
            })
            .unwrap();
        assert_eq!(sell.status, "FILLED (PAPER)");
        assert!(sell
            .risk_checks
            .iter()
            .any(|check| check.rule == "Portfolio mark freshness" && check.passed));
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn daily_loss_guard_blocks_new_buys_but_allows_risk_reducing_sells() {
        let path = test_path("daily-loss-guard");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        {
            let mut state = core.state.lock().unwrap();
            let quote = state.quotes.get("BTCUSDT").unwrap().clone();
            state.cash_balance = 8_000.0;
            state.daily_start_equity = 10_000.0;
            state.positions.insert(
                "BTCUSDT".into(),
                Position {
                    symbol: "BTCUSDT".into(),
                    quantity: 0.02,
                    average_entry: quote.price,
                    mark_price: quote.price,
                    notional: 0.02 * quote.price,
                    unrealized_pnl: 0.0,
                },
            );
            core.persist_test_fixture(&state).unwrap();
        }
        let buy = core.place_order(buy_request("daily-loss-buy")).unwrap();
        assert_eq!(buy.status, "REJECTED");
        assert!(buy
            .risk_checks
            .iter()
            .any(|check| check.rule == "Daily loss guard" && !check.passed));
        let sell = core
            .place_order(PlaceOrderRequest {
                symbol: "BTCUSDT".into(),
                side: "SELL".into(),
                quantity: 0.001,
                limit_price: None,
                client_order_id: Some("daily-loss-sell".into()),
            })
            .unwrap();
        assert_eq!(sell.status, "FILLED (PAPER)");
        assert!(sell
            .risk_checks
            .iter()
            .any(|check| check.rule == "Daily loss guard" && check.passed));
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn daily_baseline_rolls_over_utc_date() {
        let path = test_path("daily-rollover");
        let core = TradingCore::new(path.clone());
        {
            let mut state = core.state.lock().unwrap();
            state.daily_start_date = "2000-01-01".into();
        }
        let snapshot = core.snapshot().unwrap();
        let state = core.state.lock().unwrap();
        assert_eq!(state.daily_start_date, Utc::now().date_naive().to_string());
        assert!((state.daily_start_equity - snapshot.equity).abs() < 1e-8);
        drop(state);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn idempotency_conflicts_when_limit_price_changes() {
        let path = test_path("idempotency-limit");
        let core = TradingCore::new(path.clone());
        enable_test_market_data(&core);
        let mut limit = buy_request("same-intent");
        limit.limit_price = Some(60_000.0);
        let rejected = core.place_order(limit).unwrap();
        assert_eq!(rejected.status, "REJECTED");
        let market_with_same_key = buy_request("same-intent");
        assert!(core.place_order(market_with_same_key).is_err());
        drop(core);
        let _ = fs::remove_file(path);
    }

    fn valid_test_screener_token(_core: &TradingCore) -> ScreenerToken {
        ScreenerToken {
            symbol: "TESTTOKEN".into(),
            name: "Isolated Unit Test Token".into(),
            chain: "SOL".into(),
            price: 1.25,
            change_1h_pct: 1.0,
            change_24h_pct: 2.0,
            volume_24h: 2_000_000.0,
            liquidity: 500_000.0,
            momentum_score: 55,
            risk_score: 40,
            status: "Test input".into(),
            source: "isolated Rust unit-test input".into(),
            updated_at_ms: Utc::now().timestamp_millis(),
            risk_flags: vec!["test-only observation".into()],
            token_address: Some("So11111111111111111111111111111111111111112".into()),
            pair_address: Some("11111111111111111111111111111111".into()),
            pair_url: Some(
                "https://dexscreener.com/solana/11111111111111111111111111111111".into(),
            ),
            price_change_5m_pct: Some(1.5),
            market_cap_usd: Some(5_000_000.0),
            fdv_usd: Some(6_000_000.0),
            age_minutes: Some(240),
            buys_24h: Some(120),
            sells_24h: Some(80),
            boost_active: Some(3),
            ruleset_version: "momentum-risk-v1".into(),
        }
    }

    #[test]
    fn legacy_fixture_state_is_removed_and_forces_fail_closed_controls() {
        let mut state = TradingCore::fresh_state();
        let now = Utc::now().timestamp_millis();
        state.provider_status = "DEMO FIXTURE · NOT LIVE".into();
        state.provider_source = "Bundled deterministic fixture".into();
        state.quotes.insert(
            "BTCUSDT".into(),
            Quote {
                symbol: "BTCUSDT".into(),
                price: 67_421.12,
                change_24h_pct: 1.27,
                volume_24h: 28_400_000_000.0,
                source: "Bundled deterministic fixture".into(),
                updated_at_ms: now,
                retrieved_at_ms: 0,
                is_live: false,
            },
        );
        state.candles.insert(
            "BTCUSDT".into(),
            vec![Candle {
                time: now - CandleInterval::FiveMinutes.duration_ms(),
                open: 67_000.0,
                high: 67_100.0,
                low: 66_900.0,
                close: 67_050.0,
                volume: 10.0,
            }],
        );
        state.screener_provider_status = "DEMO FIXTURE · NOT LIVE".into();
        state.screener_provider_source = "Bundled screener fixtures".into();
        state.screener.push(ScreenerToken {
            symbol: "PEPE".into(),
            name: "Bundled fixture token".into(),
            chain: "ETH".into(),
            price: 0.00001,
            change_1h_pct: 1.0,
            change_24h_pct: 2.0,
            volume_24h: 1_000.0,
            liquidity: 100.0,
            momentum_score: 50,
            risk_score: 50,
            status: "Test only".into(),
            source: "Bundled example fixture — NOT LIVE".into(),
            updated_at_ms: now,
            risk_flags: vec!["test fixture".into()],
            token_address: None,
            pair_address: None,
            pair_url: None,
            price_change_5m_pct: None,
            market_cap_usd: None,
            fdv_usd: None,
            age_minutes: None,
            buys_24h: None,
            sells_24h: None,
            boost_active: None,
            ruleset_version: "fixture-v0".into(),
        });
        state.agents.push(Agent {
            id: "meme-hunter".into(),
            name: "MemeHunter placeholder".into(),
            role: "fixture".into(),
            status: "STOPPED".into(),
            mode: "OBSERVE ONLY".into(),
            objective: "Bundled placeholder".into(),
            last_decision: "fixture only".into(),
            pnl: 0.0,
            tools: Vec::new(),
            updated_at_ms: now,
        });
        state.kill_switch_engaged = false;

        assert!(TradingCore::purge_legacy_synthetic_state(&mut state));
        assert!(state.quotes.is_empty() && state.candles.is_empty() && state.screener.is_empty());
        assert!(state.agents.is_empty());
        assert!(state.kill_switch_engaged);
        assert_eq!(state.provider_updated_at_ms, 0);
        assert_eq!(state.screener_provider_updated_at_ms, 0);
        assert!(state.provider_status.contains("NO MARKET DATA"));
    }

    #[test]
    fn fresh_account_starts_without_market_observations_or_agent_placeholders() {
        let path = test_path("empty-native-startup");
        let core = TradingCore::new(path.clone());
        let snapshot = core.snapshot().unwrap();
        assert!(snapshot.quotes.is_empty());
        assert!(snapshot.candles.is_empty());
        assert_eq!(snapshot.closed_trade_count, 0);
        assert_eq!(
            snapshot.win_rate_pct, None,
            "empty statistics must remain semantically unavailable at the native contract boundary"
        );
        assert!(snapshot.screener.is_empty());
        assert!(snapshot.agents.is_empty());
        assert_eq!(snapshot.provider_updated_at_ms, 0);
        assert!(snapshot.provider_status.contains("NO MARKET DATA"));
        assert_eq!(snapshot.screener_provider_updated_at_ms, 0);
        assert!(snapshot
            .events
            .iter()
            .any(|event| event.title == "Market data unavailable"));
        assert!(
            snapshot.kill_switch_engaged,
            "a new account must require an explicit operator release before paper order entry"
        );
        assert!(core
            .get_candle_series(CandleSeriesRequest {
                symbol: "BTCUSDT".into(),
                interval: CandleInterval::FiveMinutes
            })
            .is_err());
        assert!(core.place_order(buy_request("no-quote-order")).is_err());
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn screener_failure_retains_last_good_rows_and_observation_timestamp() {
        let path = test_path("screener-stale");
        let core = TradingCore::new(path.clone());
        let token = valid_test_screener_token(&core);
        let connected = core.update_screener_data(vec![token]).unwrap();
        let old_rows = connected.screener.clone();
        let old_source = connected.screener_provider_source.clone();
        let old_timestamp = connected.screener_provider_updated_at_ms;
        let stale = core
            .record_screener_failure("test network failure".into())
            .unwrap();
        assert_eq!(
            stale.screener_provider_status,
            "STALE · LAST KNOWN DATA RETAINED"
        );
        assert_eq!(stale.screener, old_rows);
        assert_eq!(stale.screener_provider_source, old_source);
        assert_eq!(stale.screener_provider_updated_at_ms, old_timestamp);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn invalid_screener_batch_is_rejected_atomically() {
        let path = test_path("screener-atomic");
        let core = TradingCore::new(path.clone());
        let token = valid_test_screener_token(&core);
        let before = core.snapshot().unwrap().screener;
        let mut invalid = token;
        invalid.price = f64::NAN;
        assert!(core.update_screener_data(vec![invalid]).is_err());
        assert_eq!(core.snapshot().unwrap().screener, before);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn fresh_account_kill_switch_blocks_orders_even_after_a_quote_arrives() {
        let path = test_path("startup-kill-switch");
        let core = TradingCore::new(path.clone());
        let now = Utc::now().timestamp_millis();
        {
            let mut state = core.state.lock().unwrap();
            state.quotes.insert(
                "BTCUSDT".into(),
                Quote {
                    symbol: "BTCUSDT".into(),
                    price: 67_000.0,
                    change_24h_pct: 0.0,
                    volume_24h: 1_000_000.0,
                    source: "isolated Rust unit-test input".into(),
                    updated_at_ms: now,
                    retrieved_at_ms: now,
                    is_live: true,
                },
            );
            state.provider_status = "CONNECTED · UNIT TEST SOURCE".into();
            state.provider_source = "isolated Rust unit-test input".into();
            state.provider_updated_at_ms = now;
            // Do not release the default kill switch in this test.
            assert!(state.kill_switch_engaged);
            core.persist_test_fixture(&state).unwrap();
        }
        let order = core
            .place_order(buy_request("startup-lock-intent"))
            .unwrap();
        assert_eq!(order.status, "REJECTED");
        assert!(order
            .risk_checks
            .iter()
            .any(|check| check.rule == "Kill switch" && !check.passed));
        assert!(core.snapshot().unwrap().kill_switch_engaged);
        drop(core);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_market_observation_cannot_create_a_paper_fill_or_order_record() {
        let path = test_path("missing-market-observation");
        let core = TradingCore::new(path.clone());
        let error = core
            .place_order(buy_request("missing-observation-intent"))
            .unwrap_err();
        assert!(error.contains("No quote is available for BTCUSDT"));
        assert!(core.snapshot().unwrap().orders.is_empty());
        drop(core);
        let _ = fs::remove_file(path);
    }
}
