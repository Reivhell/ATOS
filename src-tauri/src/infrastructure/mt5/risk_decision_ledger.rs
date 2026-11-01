//! Append-only, account-scoped audit receipts for MT5 risk-sizing decisions.
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::Path, time::Duration};
use uuid::Uuid;

const LEDGER_SCHEMA_VERSION: i64 = 1;
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const GENESIS_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_LEDGER_EVENTS_PER_ACCOUNT: u64 = 1_000_000;
const MAX_AUDIT_PAYLOAD_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5RiskDecisionRecord {
    pub broker_server: String,
    pub account_login: String,
    pub symbol: String,
    pub decision: String,
    pub decided_at_ms: i64,
    pub request: Value,
    pub result: Value,
    pub snapshot_sequence: u64,
    pub specification_revision: u64,
    pub specification_fingerprint_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5RiskDecisionReceipt {
    pub decision_id: String,
    pub broker_server: String,
    pub account_login: String,
    pub symbol: String,
    pub decision: String,
    pub sequence: u64,
    pub decided_at_ms: i64,
    pub request_sha256: String,
    pub result_sha256: String,
    pub snapshot_sequence: u64,
    pub specification_revision: u64,
    pub specification_fingerprint_sha256: String,
    pub previous_sha256: String,
    pub event_sha256: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5RiskDecisionLedgerStatus {
    pub valid: bool,
    pub broker_server: String,
    pub account_login: String,
    pub event_count: u64,
    pub head_sha256: String,
    pub first_invalid_sequence: Option<u64>,
    pub integrity_error: Option<String>,
}

fn open_connection(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path)
        .map_err(|error| format!("MT5 risk-decision database could not be opened: {error}"))?;
    connection
        .busy_timeout(BUSY_TIMEOUT)
        .map_err(|error| format!("MT5 risk-decision database busy timeout failed: {error}"))?;
    connection
        .execute_batch("PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;")
        .map_err(|error| format!("MT5 risk-decision durability settings failed: {error}"))?;
    Ok(connection)
}

pub(crate) fn initialize(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS mt5_risk_decision_ledger_meta (
                singleton_id INTEGER PRIMARY KEY CHECK(singleton_id = 1),
                schema_version INTEGER NOT NULL CHECK(schema_version >= 1)
            );
            INSERT OR IGNORE INTO mt5_risk_decision_ledger_meta(singleton_id, schema_version)
                VALUES (1, 1);
            CREATE TABLE IF NOT EXISTS mt5_risk_decision_events (
                broker_server TEXT NOT NULL,
                account_login TEXT NOT NULL,
                sequence INTEGER NOT NULL CHECK(sequence > 0),
                decision_id TEXT NOT NULL UNIQUE,
                symbol TEXT NOT NULL,
                decision TEXT NOT NULL CHECK(decision IN ('SIZED_WITH_LIMITATIONS', 'BLOCKED')),
                decided_at_ms INTEGER NOT NULL CHECK(decided_at_ms > 0),
                request_json TEXT NOT NULL,
                request_sha256 TEXT NOT NULL CHECK(length(request_sha256) = 64),
                result_json TEXT NOT NULL,
                result_sha256 TEXT NOT NULL CHECK(length(result_sha256) = 64),
                snapshot_sequence INTEGER NOT NULL CHECK(snapshot_sequence > 0),
                specification_revision INTEGER NOT NULL CHECK(specification_revision > 0),
                specification_fingerprint_sha256 TEXT NOT NULL CHECK(length(specification_fingerprint_sha256) = 64),
                previous_sha256 TEXT NOT NULL CHECK(length(previous_sha256) = 64),
                event_sha256 TEXT NOT NULL CHECK(length(event_sha256) = 64),
                PRIMARY KEY(broker_server, account_login, sequence)
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS idx_mt5_risk_decisions_by_symbol
                ON mt5_risk_decision_events(broker_server, account_login, symbol, sequence DESC);
            CREATE TRIGGER IF NOT EXISTS trg_mt5_risk_decision_events_no_update
            BEFORE UPDATE ON mt5_risk_decision_events
            BEGIN SELECT RAISE(ABORT, 'MT5 risk-decision ledger is append-only'); END;
            CREATE TRIGGER IF NOT EXISTS trg_mt5_risk_decision_events_no_delete
            BEFORE DELETE ON mt5_risk_decision_events
            BEGIN SELECT RAISE(ABORT, 'MT5 risk-decision ledger is append-only'); END;",
        )
        .map_err(|error| format!("MT5 risk-decision ledger schema initialization failed: {error}"))?;

    let version: i64 = connection
        .query_row(
            "SELECT schema_version FROM mt5_risk_decision_ledger_meta WHERE singleton_id=1",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            format!("MT5 risk-decision ledger schema version could not be read: {error}")
        })?;
    if version > LEDGER_SCHEMA_VERSION {
        return Err(format!(
            "MT5 risk-decision ledger schema version {version} is newer than this application supports ({LEDGER_SCHEMA_VERSION}); refusing writes."
        ));
    }
    if version != LEDGER_SCHEMA_VERSION {
        return Err(format!(
            "MT5 risk-decision ledger schema version {version} requires an explicit migration."
        ));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_identity(value: &str, max_len: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= max_len
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn canonical_json(value: &Value) -> Result<String, String> {
    if !value.is_object() {
        return Err("Risk-decision audit payload must be a JSON object.".into());
    }
    let encoded = serde_json::to_string(value)
        .map_err(|error| format!("Risk-decision audit payload could not be serialized: {error}"))?;
    if encoded.len() > MAX_AUDIT_PAYLOAD_BYTES {
        return Err("Risk-decision audit payload exceeds the 256 KiB storage limit.".into());
    }
    Ok(encoded)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn event_sha256(
    broker_server: &str,
    account_login: &str,
    sequence: u64,
    decision_id: &str,
    symbol: &str,
    decision: &str,
    decided_at_ms: i64,
    request_sha256: &str,
    result_sha256: &str,
    snapshot_sequence: u64,
    specification_revision: u64,
    specification_fingerprint_sha256: &str,
    previous_sha256: &str,
) -> String {
    let material = format!(
        "{LEDGER_SCHEMA_VERSION}\n{broker_server}\n{account_login}\n{sequence}\n{decision_id}\n{symbol}\n{decision}\n{decided_at_ms}\n{request_sha256}\n{result_sha256}\n{snapshot_sequence}\n{specification_revision}\n{specification_fingerprint_sha256}\n{previous_sha256}"
    );
    sha256_hex(material.as_bytes())
}

fn invalid_status(
    broker_server: &str,
    account_login: &str,
    event_count: u64,
    head_sha256: String,
    sequence: u64,
    reason: impl Into<String>,
) -> Mt5RiskDecisionLedgerStatus {
    Mt5RiskDecisionLedgerStatus {
        valid: false,
        broker_server: broker_server.into(),
        account_login: account_login.into(),
        event_count,
        head_sha256,
        first_invalid_sequence: Some(sequence),
        integrity_error: Some(reason.into()),
    }
}

fn verify_chain(
    connection: &Connection,
    broker_server: &str,
    account_login: &str,
) -> Result<Mt5RiskDecisionLedgerStatus, String> {
    let mut statement = connection
        .prepare(
            "SELECT sequence, decision_id, symbol, decision, decided_at_ms,
                    request_json, request_sha256, result_json, result_sha256,
                    snapshot_sequence, specification_revision, specification_fingerprint_sha256,
                    previous_sha256, event_sha256
             FROM mt5_risk_decision_events
             WHERE broker_server=?1 AND account_login=?2
             ORDER BY sequence ASC",
        )
        .map_err(|error| format!("MT5 risk-decision chain query could not be prepared: {error}"))?;
    let mut rows = statement
        .query(params![broker_server, account_login])
        .map_err(|error| format!("MT5 risk-decision chain query failed: {error}"))?;
    let mut expected_sequence = 1_u64;
    let mut previous = GENESIS_SHA256.to_string();
    let mut event_count = 0_u64;

    while let Some(row) = rows
        .next()
        .map_err(|error| format!("MT5 risk-decision chain row could not be read: {error}"))?
    {
        let sequence: i64 = row
            .get(0)
            .map_err(|error| format!("Risk-decision sequence is unreadable: {error}"))?;
        let decision_id: String = row
            .get(1)
            .map_err(|error| format!("Risk-decision ID is unreadable: {error}"))?;
        let symbol: String = row
            .get(2)
            .map_err(|error| format!("Risk-decision symbol is unreadable: {error}"))?;
        let decision: String = row
            .get(3)
            .map_err(|error| format!("Risk-decision state is unreadable: {error}"))?;
        let decided_at_ms: i64 = row
            .get(4)
            .map_err(|error| format!("Risk-decision timestamp is unreadable: {error}"))?;
        let request_json: String = row
            .get(5)
            .map_err(|error| format!("Risk-decision request is unreadable: {error}"))?;
        let request_hash: String = row
            .get(6)
            .map_err(|error| format!("Risk-decision request hash is unreadable: {error}"))?;
        let result_json: String = row
            .get(7)
            .map_err(|error| format!("Risk-decision result is unreadable: {error}"))?;
        let result_hash: String = row
            .get(8)
            .map_err(|error| format!("Risk-decision result hash is unreadable: {error}"))?;
        let snapshot_sequence: i64 = row
            .get(9)
            .map_err(|error| format!("Risk-decision snapshot sequence is unreadable: {error}"))?;
        let specification_revision: i64 = row.get(10).map_err(|error| {
            format!("Risk-decision specification revision is unreadable: {error}")
        })?;
        let specification_hash: String = row
            .get(11)
            .map_err(|error| format!("Risk-decision specification hash is unreadable: {error}"))?;
        let previous_hash: String = row
            .get(12)
            .map_err(|error| format!("Risk-decision previous hash is unreadable: {error}"))?;
        let stored_event_hash: String = row
            .get(13)
            .map_err(|error| format!("Risk-decision event hash is unreadable: {error}"))?;
        let sequence_u64 = u64::try_from(sequence).unwrap_or(0);

        let fail = |reason: &str| {
            invalid_status(
                broker_server,
                account_login,
                event_count,
                previous.clone(),
                sequence_u64,
                reason,
            )
        };
        if sequence_u64 != expected_sequence {
            return Ok(fail("Risk-decision sequence is discontinuous."));
        }
        if !matches!(decision.as_str(), "SIZED_WITH_LIMITATIONS" | "BLOCKED")
            || !valid_identity(&decision_id, 64)
            || !valid_identity(&symbol, 64)
            || decided_at_ms <= 0
            || snapshot_sequence <= 0
            || specification_revision <= 0
            || !is_sha256(&request_hash)
            || !is_sha256(&result_hash)
            || !is_sha256(&specification_hash)
            || !is_sha256(&previous_hash)
            || !is_sha256(&stored_event_hash)
        {
            return Ok(fail("Risk-decision row metadata is malformed."));
        }
        if previous_hash != previous {
            return Ok(fail(
                "Risk-decision previous hash does not match the verified chain head.",
            ));
        }
        if sha256_hex(request_json.as_bytes()) != request_hash
            || sha256_hex(result_json.as_bytes()) != result_hash
        {
            return Ok(fail(
                "Risk-decision request or result payload hash does not match its stored bytes.",
            ));
        }
        let request: Value = match serde_json::from_str(&request_json) {
            Ok(value) => value,
            Err(_) => return Ok(fail("Risk-decision request payload is not valid JSON.")),
        };
        let result: Value = match serde_json::from_str(&result_json) {
            Ok(value) => value,
            Err(_) => return Ok(fail("Risk-decision result payload is not valid JSON.")),
        };
        if canonical_json(&request).ok().as_deref() != Some(request_json.as_str())
            || canonical_json(&result).ok().as_deref() != Some(result_json.as_str())
        {
            return Ok(fail(
                "Risk-decision payload is not in canonical serialized form.",
            ));
        }
        let snapshot_sequence = u64::try_from(snapshot_sequence)
            .map_err(|_| "Risk-decision snapshot sequence is negative.".to_string())?;
        let specification_revision = u64::try_from(specification_revision)
            .map_err(|_| "Risk-decision specification revision is negative.".to_string())?;
        if request.get("brokerServer").and_then(Value::as_str) != Some(broker_server)
            || request.get("accountLogin").and_then(Value::as_str) != Some(account_login)
            || request.get("symbol").and_then(Value::as_str) != Some(symbol.as_str())
            || result.get("brokerServer").and_then(Value::as_str) != Some(broker_server)
            || result.get("accountLogin").and_then(Value::as_str) != Some(account_login)
            || result.get("symbol").and_then(Value::as_str) != Some(symbol.as_str())
            || result.get("decision").and_then(Value::as_str) != Some(decision.as_str())
            || result.get("snapshotSequence").and_then(Value::as_u64) != Some(snapshot_sequence)
            || result.get("specificationRevision").and_then(Value::as_u64)
                != Some(specification_revision)
            || result
                .get("specificationFingerprintSha256")
                .and_then(Value::as_str)
                != Some(specification_hash.as_str())
            || result.get("calculatedAtMs").and_then(Value::as_i64) != Some(decided_at_ms)
        {
            return Ok(fail(
                "Risk-decision payload identity does not match its immutable ledger columns.",
            ));
        }
        let calculated = event_sha256(
            broker_server,
            account_login,
            sequence_u64,
            &decision_id,
            &symbol,
            &decision,
            decided_at_ms,
            &request_hash,
            &result_hash,
            snapshot_sequence,
            specification_revision,
            &specification_hash,
            &previous_hash,
        );
        if calculated != stored_event_hash {
            return Ok(fail(
                "Risk-decision event hash does not match its canonical event material.",
            ));
        }
        previous = stored_event_hash;
        event_count = event_count
            .checked_add(1)
            .ok_or_else(|| "Risk-decision event count overflowed.".to_string())?;
        expected_sequence = expected_sequence
            .checked_add(1)
            .ok_or_else(|| "Risk-decision sequence overflowed.".to_string())?;
    }
    Ok(Mt5RiskDecisionLedgerStatus {
        valid: true,
        broker_server: broker_server.into(),
        account_login: account_login.into(),
        event_count,
        head_sha256: previous,
        first_invalid_sequence: None,
        integrity_error: None,
    })
}

fn validate_record(record: &Mt5RiskDecisionRecord) -> Result<(String, String), String> {
    if !valid_identity(&record.broker_server, 128)
        || !valid_identity(&record.account_login, 32)
        || !valid_identity(&record.symbol, 64)
        || !matches!(
            record.decision.as_str(),
            "SIZED_WITH_LIMITATIONS" | "BLOCKED"
        )
        || record.decided_at_ms <= 0
        || record.snapshot_sequence == 0
        || record.specification_revision == 0
        || !is_sha256(&record.specification_fingerprint_sha256)
    {
        return Err("Risk-decision ledger record has invalid account, instrument, decision, time, or provenance metadata.".into());
    }
    let request_json = canonical_json(&record.request)?;
    let result_json = canonical_json(&record.result)?;
    if record.request.get("brokerServer").and_then(Value::as_str)
        != Some(record.broker_server.as_str())
        || record.request.get("accountLogin").and_then(Value::as_str)
            != Some(record.account_login.as_str())
        || record.request.get("symbol").and_then(Value::as_str) != Some(record.symbol.as_str())
        || record.result.get("brokerServer").and_then(Value::as_str)
            != Some(record.broker_server.as_str())
        || record.result.get("accountLogin").and_then(Value::as_str)
            != Some(record.account_login.as_str())
        || record.result.get("symbol").and_then(Value::as_str) != Some(record.symbol.as_str())
        || record.result.get("decision").and_then(Value::as_str) != Some(record.decision.as_str())
        || record
            .result
            .get("snapshotSequence")
            .and_then(Value::as_u64)
            != Some(record.snapshot_sequence)
        || record.result.get("specificationRevision").and_then(Value::as_u64)
            != Some(record.specification_revision)
        || record
            .result
            .get("specificationFingerprintSha256")
            .and_then(Value::as_str)
            != Some(record.specification_fingerprint_sha256.as_str())
        || record.result.get("calculatedAtMs").and_then(Value::as_i64) != Some(record.decided_at_ms)
    {
        return Err("Risk-decision request/result identity is not bound to the authenticated account snapshot.".into());
    }
    Ok((request_json, result_json))
}

pub(crate) fn append(
    path: &Path,
    record: &Mt5RiskDecisionRecord,
) -> Result<Mt5RiskDecisionReceipt, String> {
    let (request_json, result_json) = validate_record(record)?;
    let mut connection = open_connection(path)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| format!("Risk-decision append transaction could not begin: {error}"))?;
    let verified = verify_chain(&transaction, &record.broker_server, &record.account_login)?;
    if !verified.valid {
        return Err(format!(
            "Risk-decision ledger is corrupt at sequence {:?}; refusing to append: {}",
            verified.first_invalid_sequence,
            verified
                .integrity_error
                .unwrap_or_else(|| "integrity verification failed".into())
        ));
    }
    if verified.event_count >= MAX_LEDGER_EVENTS_PER_ACCOUNT {
        return Err("Risk-decision account ledger reached its configured event limit.".into());
    }
    let sequence = verified
        .event_count
        .checked_add(1)
        .ok_or_else(|| "Risk-decision sequence overflowed.".to_string())?;
    let previous_sha256 = verified.head_sha256;
    let decision_id = Uuid::new_v4().to_string();
    let request_sha256 = sha256_hex(request_json.as_bytes());
    let result_sha256 = sha256_hex(result_json.as_bytes());
    let event_hash = event_sha256(
        &record.broker_server,
        &record.account_login,
        sequence,
        &decision_id,
        &record.symbol,
        &record.decision,
        record.decided_at_ms,
        &request_sha256,
        &result_sha256,
        record.snapshot_sequence,
        record.specification_revision,
        &record.specification_fingerprint_sha256,
        &previous_sha256,
    );
    transaction
        .execute(
            "INSERT INTO mt5_risk_decision_events
             (broker_server, account_login, sequence, decision_id, symbol, decision, decided_at_ms,
              request_json, request_sha256, result_json, result_sha256, snapshot_sequence,
              specification_revision, specification_fingerprint_sha256, previous_sha256, event_sha256)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                record.broker_server,
                record.account_login,
                sequence as i64,
                decision_id,
                record.symbol,
                record.decision,
                record.decided_at_ms,
                request_json,
                request_sha256,
                result_json,
                result_sha256,
                record.snapshot_sequence as i64,
                record.specification_revision as i64,
                record.specification_fingerprint_sha256,
                previous_sha256,
                event_hash
            ],
        )
        .map_err(|error| format!("Risk-decision event could not be appended: {error}"))?;
    transaction
        .commit()
        .map_err(|error| format!("Risk-decision event commit failed: {error}"))?;
    let committed_chain = verify_chain(&connection, &record.broker_server, &record.account_login)?;
    if !committed_chain.valid
        || committed_chain.event_count != sequence
        || committed_chain.head_sha256 != event_hash
    {
        return Err(
            "Committed risk-decision receipt did not verify against the durable account chain."
                .into(),
        );
    }

    Ok(Mt5RiskDecisionReceipt {
        decision_id,
        broker_server: record.broker_server.clone(),
        account_login: record.account_login.clone(),
        symbol: record.symbol.clone(),
        decision: record.decision.clone(),
        sequence,
        decided_at_ms: record.decided_at_ms,
        request_sha256,
        result_sha256,
        snapshot_sequence: record.snapshot_sequence,
        specification_revision: record.specification_revision,
        specification_fingerprint_sha256: record.specification_fingerprint_sha256.clone(),
        previous_sha256,
        event_sha256: event_hash,
    })
}

pub(crate) fn status(
    path: &Path,
    broker_server: &str,
    account_login: &str,
) -> Result<Mt5RiskDecisionLedgerStatus, String> {
    if !valid_identity(broker_server, 128) || !valid_identity(account_login, 32) {
        return Err(
            "Risk-decision ledger status requires a valid broker server and account identity."
                .into(),
        );
    }
    let connection = open_connection(path)?;
    verify_chain(&connection, broker_server, account_login)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::{sync::Arc, thread};

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("atos-mt5-risk-ledger-{}.sqlite3", Uuid::new_v4()))
    }

    fn initialized_path() -> std::path::PathBuf {
        let path = temp_path();
        let connection = open_connection(&path).unwrap();
        initialize(&connection).unwrap();
        drop(connection);
        path
    }

    fn sample_record(account: &str, decision: &str, sequence: u64) -> Mt5RiskDecisionRecord {
        let request = serde_json::json!({
            "brokerServer": "Broker-Live",
            "accountLogin": account,
            "symbol": "XAUUSD.a",
            "side": "BUY",
            "stopLossPrice": 2300.0,
            "riskPercent": 1.0
        });
        let result = serde_json::json!({
            "brokerServer": "Broker-Live",
            "accountLogin": account,
            "symbol": "XAUUSD.a",
            "decision": decision,
            "calculatedAtMs": 1_800_000_000_000_i64 + sequence as i64,
            "snapshotSequence": sequence,
            "specificationRevision": 3,
            "specificationFingerprintSha256": "a".repeat(64),
            "normalizedVolume": 0.01
        });
        Mt5RiskDecisionRecord {
            broker_server: "Broker-Live".into(),
            account_login: account.into(),
            symbol: "XAUUSD.a".into(),
            decision: decision.into(),
            decided_at_ms: 1_800_000_000_000_i64 + sequence as i64,
            request,
            result,
            snapshot_sequence: sequence,
            specification_revision: 3,
            specification_fingerprint_sha256: "a".repeat(64),
        }
    }

    #[test]
    fn append_records_both_approved_and_blocked_decisions_in_one_account_chain() {
        let path = initialized_path();
        let approved = append(&path, &sample_record("12345", "SIZED_WITH_LIMITATIONS", 7)).unwrap();
        let blocked = append(&path, &sample_record("12345", "BLOCKED", 8)).unwrap();
        assert_eq!(approved.sequence, 1);
        assert_eq!(approved.previous_sha256, GENESIS_SHA256);
        assert_eq!(blocked.sequence, 2);
        assert_eq!(blocked.previous_sha256, approved.event_sha256);
        let status = status(&path, "Broker-Live", "12345").unwrap();
        assert!(status.valid);
        assert_eq!(status.event_count, 2);
        assert_eq!(status.head_sha256, blocked.event_sha256);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn decision_chains_are_isolated_by_authenticated_account_identity() {
        let path = initialized_path();
        append(&path, &sample_record("12345", "BLOCKED", 1)).unwrap();
        append(&path, &sample_record("67890", "BLOCKED", 1)).unwrap();
        let first = status(&path, "Broker-Live", "12345").unwrap();
        let second = status(&path, "Broker-Live", "67890").unwrap();
        assert_eq!(first.event_count, 1);
        assert_eq!(second.event_count, 1);
        assert_ne!(first.head_sha256, second.head_sha256);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn identity_mismatch_is_rejected_before_the_ledger_changes() {
        let path = initialized_path();
        let mut record = sample_record("12345", "BLOCKED", 1);
        record.request["accountLogin"] = Value::String("99999".into());
        assert!(append(&path, &record).is_err());
        let status = status(&path, "Broker-Live", "12345").unwrap();
        assert_eq!(status.event_count, 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn update_and_delete_are_rejected_by_append_only_triggers() {
        let path = initialized_path();
        append(&path, &sample_record("12345", "BLOCKED", 1)).unwrap();
        let connection = Connection::open(&path).unwrap();
        assert!(
            connection
                .execute(
                    "UPDATE mt5_risk_decision_events SET decision='SIZED_WITH_LIMITATIONS'",
                    []
                )
                .is_err()
        );
        assert!(connection
            .execute("DELETE FROM mt5_risk_decision_events", [])
            .is_err());
        assert!(status(&path, "Broker-Live", "12345").unwrap().valid);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn chain_verification_detects_payload_tampering_even_after_trigger_bypass() {
        let path = initialized_path();
        append(&path, &sample_record("12345", "BLOCKED", 1)).unwrap();
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "DROP TRIGGER trg_mt5_risk_decision_events_no_update;
             UPDATE mt5_risk_decision_events SET result_json='{}';",
            )
            .unwrap();
        let status = status(&path, "Broker-Live", "12345").unwrap();
        assert!(!status.valid);
        assert_eq!(status.first_invalid_sequence, Some(1));
        assert!(status.integrity_error.unwrap().contains("payload hash"));
        assert!(append(&path, &sample_record("12345", "BLOCKED", 2)).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn concurrent_account_appends_receive_unique_contiguous_sequences() {
        let path = Arc::new(initialized_path());
        let mut workers = Vec::new();
        for index in 0..8_u64 {
            let worker_path = Arc::clone(&path);
            workers.push(thread::spawn(move || {
                append(
                    worker_path.as_path(),
                    &sample_record("12345", "BLOCKED", index + 1),
                )
                .unwrap()
                .sequence
            }));
        }
        let mut sequences: Vec<u64> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        sequences.sort_unstable();
        assert_eq!(sequences, (1..=8).collect::<Vec<_>>());
        let status = status(path.as_path(), "Broker-Live", "12345").unwrap();
        assert!(status.valid);
        assert_eq!(status.event_count, 8);
        let _ = std::fs::remove_file(path.as_path());
    }
}
