//! Durable intent ledger for a future MT5 execution controller.
//!
//! This module persists intent before any broker dispatch. It deliberately does not
//! submit orders; callers must reconcile ambiguous dispatches before considering retries.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

const SCHEMA_VERSION: i64 = 1;
const MAX_PAYLOAD_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentState {
    Prepared,
    Dispatching,
    Accepted,
    Rejected,
    Unknown,
    Reconciled,
}

impl IntentState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Dispatching => "dispatching",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Unknown => "unknown",
            Self::Reconciled => "reconciled",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "dispatching" => Ok(Self::Dispatching),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "unknown" => Ok(Self::Unknown),
            "reconciled" => Ok(Self::Reconciled),
            _ => Err("Order-intent ledger contains an unknown state.".into()),
        }
    }

    fn allows(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Prepared, Self::Dispatching)
                | (Self::Dispatching, Self::Accepted)
                | (Self::Dispatching, Self::Rejected)
                | (Self::Dispatching, Self::Unknown)
                | (Self::Unknown, Self::Accepted)
                | (Self::Unknown, Self::Rejected)
                | (Self::Unknown, Self::Reconciled)
                | (Self::Accepted, Self::Reconciled)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewOrderIntent {
    pub broker_server: String,
    pub account_login: String,
    pub idempotency_key: String,
    pub symbol: String,
    pub side: String,
    pub volume: f64,
    pub stop_loss: f64,
    pub take_profit: Option<f64>,
    pub payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OrderIntentRecord {
    pub intent_id: String,
    pub broker_server: String,
    pub account_login: String,
    pub idempotency_key: String,
    pub payload_sha256: String,
    pub symbol: String,
    pub side: String,
    pub volume: f64,
    pub stop_loss: f64,
    pub take_profit: Option<f64>,
    pub state: IntentState,
    pub broker_order_ticket: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentEvent {
    pub event_id: i64,
    pub intent_id: String,
    pub from_state: Option<IntentState>,
    pub to_state: IntentState,
    pub detail: String,
    pub occurred_at: String,
}

pub struct Mt5OrderIntentLedger {
    connection: Connection,
}

impl Mt5OrderIntentLedger {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let connection = Connection::open(path)
            .map_err(|_| "MT5 order-intent database could not be opened.".to_string())?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|_| {
                "MT5 order-intent database timeout could not be configured.".to_string()
            })?;
        connection
            .execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA synchronous=FULL;
                 PRAGMA foreign_keys=ON;",
            )
            .map_err(|_| {
                "MT5 order-intent database safety settings could not be applied.".to_string()
            })?;
        let mut ledger = Self { connection };
        ledger.initialize()?;
        Ok(ledger)
    }

    fn initialize(&mut self) -> Result<(), String> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "MT5 order-intent schema transaction could not start.".to_string())?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS mt5_order_intent_schema (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                version INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS mt5_order_intents (
                intent_id TEXT PRIMARY KEY NOT NULL,
                broker_server TEXT NOT NULL,
                account_login TEXT NOT NULL,
                idempotency_key TEXT NOT NULL,
                payload_sha256 TEXT NOT NULL,
                symbol TEXT NOT NULL,
                side TEXT NOT NULL,
                volume REAL NOT NULL CHECK (volume > 0),
                stop_loss REAL NOT NULL CHECK (stop_loss > 0),
                take_profit REAL,
                state TEXT NOT NULL CHECK (state IN
                    ('prepared','dispatching','accepted','rejected','unknown','reconciled')),
                broker_order_ticket TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE (broker_server, account_login, idempotency_key)
             );
             CREATE TABLE IF NOT EXISTS mt5_order_intent_events (
                event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                intent_id TEXT NOT NULL REFERENCES mt5_order_intents(intent_id),
                from_state TEXT,
                to_state TEXT NOT NULL,
                detail TEXT NOT NULL,
                occurred_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS mt5_order_intents_unresolved
                ON mt5_order_intents (broker_server, account_login, state);",
        )
        .map_err(|_| "MT5 order-intent schema could not be created.".to_string())?;
        let version: Option<i64> = tx
            .query_row(
                "SELECT version FROM mt5_order_intent_schema WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| "MT5 order-intent schema version could not be read.".to_string())?;
        match version {
            Some(v) if v != SCHEMA_VERSION => {
                return Err("MT5 order-intent schema version is unsupported; refusing to mutate the ledger.".into());
            }
            Some(_) => {}
            None => {
                tx.execute(
                    "INSERT INTO mt5_order_intent_schema (singleton, version) VALUES (1, ?1)",
                    [SCHEMA_VERSION],
                )
                .map_err(|_| {
                    "MT5 order-intent schema version could not be recorded.".to_string()
                })?;
            }
        }
        tx.commit()
            .map_err(|_| "MT5 order-intent schema transaction could not commit.".to_string())
    }

    pub fn prepare(&mut self, intent: &NewOrderIntent) -> Result<OrderIntentRecord, String> {
        validate_new_intent(intent)?;
        let canonical_intent = serde_json::to_vec(intent)
            .map_err(|_| "MT5 order intent could not be fingerprinted.".to_string())?;
        let payload_sha256 = hex_sha256(&canonical_intent);
        let now = Utc::now().to_rfc3339();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "MT5 order-intent transaction could not start.".to_string())?;
        let existing: Option<OrderIntentRecord> = tx
            .query_row(
                "SELECT intent_id, broker_server, account_login, idempotency_key, payload_sha256,
                        symbol, side, volume, stop_loss, take_profit, state, broker_order_ticket,
                        created_at, updated_at
                 FROM mt5_order_intents
                 WHERE broker_server=?1 AND account_login=?2 AND idempotency_key=?3",
                params![
                    intent.broker_server,
                    intent.account_login,
                    intent.idempotency_key
                ],
                row_to_record,
            )
            .optional()
            .map_err(|_| "Existing MT5 order intent could not be read.".to_string())?;
        if let Some(record) = existing {
            if record.payload_sha256 != payload_sha256 {
                return Err(
                    "Idempotency key was already used for a different order payload.".into(),
                );
            }
            tx.commit()
                .map_err(|_| "MT5 order-intent transaction could not commit.".to_string())?;
            return Ok(record);
        }

        let intent_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO mt5_order_intents
             (intent_id, broker_server, account_login, idempotency_key, payload_sha256,
              symbol, side, volume, stop_loss, take_profit, state, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'prepared',?11,?11)",
            params![
                intent_id, intent.broker_server, intent.account_login,
                intent.idempotency_key, payload_sha256, intent.symbol, intent.side,
                intent.volume, intent.stop_loss, intent.take_profit, now
            ],
        ).map_err(|_| "MT5 order intent could not be persisted.".to_string())?;
        tx.execute(
            "INSERT INTO mt5_order_intent_events
             (intent_id, from_state, to_state, detail, occurred_at)
             VALUES (?1,NULL,'prepared','intent_persisted_before_dispatch',?2)",
            params![intent_id, now],
        )
        .map_err(|_| "MT5 order-intent audit event could not be persisted.".to_string())?;
        let record = tx.query_row(
            "SELECT intent_id, broker_server, account_login, idempotency_key, payload_sha256,
                    symbol, side, volume, stop_loss, take_profit, state, broker_order_ticket,
                    created_at, updated_at
             FROM mt5_order_intents WHERE intent_id=?1",
            [&intent_id],
            row_to_record,
        ).map_err(|_| "Persisted MT5 order intent could not be reloaded.".to_string())?;
        tx.commit()
                .map_err(|_| "MT5 order-intent transaction could not commit.".to_string())?;
        Ok(record)
    }

    pub fn transition(
        &mut self,
        intent_id: &str,
        expected: IntentState,
        next: IntentState,
        detail: &str,
        broker_order_ticket: Option<&str>,
    ) -> Result<OrderIntentRecord, String> {
        if detail.trim().is_empty() || detail.len() > 2048 {
            return Err("Order-intent transition detail is empty or exceeds the limit.".into());
        }
        if !expected.allows(next) {
            return Err(format!(
                "Illegal MT5 order-intent transition: {} -> {}.",
                expected.as_str(),
                next.as_str()
            ));
        }
        if broker_order_ticket.is_some_and(|ticket| ticket.trim().is_empty() || ticket.len() > 128)
        {
            return Err("Broker order ticket is invalid.".into());
        }
        let now = Utc::now().to_rfc3339();
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| "MT5 order-intent transaction could not start.".to_string())?;
        let current: String = tx.query_row(
            "SELECT state FROM mt5_order_intents WHERE intent_id=?1",
            [intent_id],
            |row| row.get(0),
        ).map_err(|_| "MT5 order intent does not exist.".to_string())?;
        let current_state = IntentState::parse(&current)?;
        if current_state != expected {
            return Err(format!("MT5 order intent is {}, not {}; transition rejected.", current, expected.as_str()));
        }
        tx.execute(
            "UPDATE mt5_order_intents
             SET state=?2, broker_order_ticket=COALESCE(?3, broker_order_ticket), updated_at=?4
             WHERE intent_id=?1 AND state=?5",
            params![intent_id, next.as_str(), broker_order_ticket, now, expected.as_str()],
        ).map_err(|_| "MT5 order intent state could not be updated.".to_string())?;
        tx.execute(
            "INSERT INTO mt5_order_intent_events
             (intent_id, from_state, to_state, detail, occurred_at)
             VALUES (?1,?2,?3,?4,?5)",
            params![intent_id, expected.as_str(), next.as_str(), detail, now],
        )
        .map_err(|_| "MT5 order-intent audit event could not be persisted.".to_string())?;
        let record = tx.query_row(
            "SELECT intent_id, broker_server, account_login, idempotency_key, payload_sha256,
                    symbol, side, volume, stop_loss, take_profit, state, broker_order_ticket,
                    created_at, updated_at
             FROM mt5_order_intents WHERE intent_id=?1",
            [intent_id],
            row_to_record,
        ).map_err(|_| "Updated MT5 order intent could not be reloaded.".to_string())?;
        tx.commit()
                .map_err(|_| "MT5 order-intent transaction could not commit.".to_string())?;
        Ok(record)
    }

    pub fn unresolved(
        &self,
        broker_server: &str,
        account_login: &str,
    ) -> Result<Vec<OrderIntentRecord>, String> {
        let mut statement = self.connection.prepare(
            "SELECT intent_id, broker_server, account_login, idempotency_key, payload_sha256,
                    symbol, side, volume, stop_loss, take_profit, state, broker_order_ticket,
                    created_at, updated_at
             FROM mt5_order_intents
             WHERE broker_server=?1 AND account_login=?2 AND state IN ('prepared','dispatching','unknown')
             ORDER BY created_at ASC",
        ).map_err(|_| "Unresolved MT5 order intents could not be queried.".to_string())?;
        let rows = statement
            .query_map(params![broker_server, account_login], row_to_record)
            .map_err(|_| "Unresolved MT5 order intents could not be read.".to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Unresolved MT5 order intent record is malformed.".to_string())
    }

    pub fn events(&self, intent_id: &str) -> Result<Vec<IntentEvent>, String> {
        let mut statement = self.connection.prepare(
            "SELECT event_id, intent_id, from_state, to_state, detail, occurred_at
             FROM mt5_order_intent_events WHERE intent_id=?1 ORDER BY event_id ASC",
        ).map_err(|_| "MT5 order-intent events could not be queried.".to_string())?;
        let rows = statement.query_map([intent_id], |row| {
            let from: Option<String> = row.get(2)?;
            let to: String = row.get(3)?;
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, from, to,
                row.get::<_, String>(4)?, row.get::<_, String>(5)?))
        }).map_err(|_| "MT5 order-intent events could not be read.".to_string())?;
        let mut events = Vec::new();
        for row in rows {
            let (event_id, intent_id, from, to, detail, occurred_at) =
                row.map_err(|_| "MT5 order-intent event is malformed.".to_string())?;
            events.push(IntentEvent {
                event_id,
                intent_id,
                from_state: from.map(|s| IntentState::parse(&s)).transpose()?,
                to_state: IntentState::parse(&to)?,
                detail,
                occurred_at,
            });
        }
        Ok(events)
    }
}

fn validate_new_intent(intent: &NewOrderIntent) -> Result<(), String> {
    for (label, value, max) in [
        ("broker server", intent.broker_server.as_str(), 256usize),
        ("account login", intent.account_login.as_str(), 128),
        ("idempotency key", intent.idempotency_key.as_str(), 256),
        ("symbol", intent.symbol.as_str(), 64),
        ("side", intent.side.as_str(), 8),
    ] {
        if value.trim().is_empty() || value.len() > max {
            return Err(format!(
                "MT5 order-intent {label} is empty or exceeds the limit."
            ));
        }
    }
    if !matches!(intent.side.as_str(), "buy" | "sell") {
        return Err("MT5 order-intent side must be buy or sell.".into());
    }
    if !intent.volume.is_finite()
        || intent.volume <= 0.0
        || !intent.stop_loss.is_finite()
        || intent.stop_loss <= 0.0
        || intent
            .take_profit
            .is_some_and(|v| !v.is_finite() || v <= 0.0)
    {
        return Err("MT5 order-intent price or volume is invalid.".into());
    }
    if intent.payload_json.len() > MAX_PAYLOAD_BYTES {
        return Err("MT5 order-intent payload exceeds the size limit.".into());
    }
    let payload: serde_json::Value = serde_json::from_str(&intent.payload_json)
        .map_err(|_| "MT5 order-intent payload is not valid JSON.".to_string())?;
    if !payload.is_object() {
        return Err("MT5 order-intent payload must be a JSON object.".into());
    }
    Ok(())
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<OrderIntentRecord> {
    let state: String = row.get(10)?;
    Ok(OrderIntentRecord {
        intent_id: row.get(0)?,
        broker_server: row.get(1)?,
        account_login: row.get(2)?,
        idempotency_key: row.get(3)?,
        payload_sha256: row.get(4)?,
        symbol: row.get(5)?,
        side: row.get(6)?,
        volume: row.get(7)?,
        stop_loss: row.get(8)?,
        take_profit: row.get(9)?,
        state: IntentState::parse(&state).map_err(|_| rusqlite::Error::InvalidQuery)?,
        broker_order_ticket: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(key: &str) -> NewOrderIntent {
        NewOrderIntent {
            broker_server: "demo-server".into(),
            account_login: "123456".into(),
            idempotency_key: key.into(),
            symbol: "EURUSD".into(),
            side: "buy".into(),
            volume: 0.1,
            stop_loss: 1.05,
            take_profit: Some(1.08),
            payload_json: r#"{"symbol":"EURUSD","side":"buy","volume":0.1,"stopLoss":1.05}"#.into(),
        }
    }

    #[test]
    fn prepare_is_idempotent_for_the_same_payload() {
        let mut ledger = Mt5OrderIntentLedger::open(":memory:").unwrap();
        let first = ledger.prepare(&intent("key-1")).unwrap();
        let second = ledger.prepare(&intent("key-1")).unwrap();
        assert_eq!(first.intent_id, second.intent_id);
        assert_eq!(first.state, IntentState::Prepared);
        assert_eq!(ledger.events(&first.intent_id).unwrap().len(), 1);
    }

    #[test]
    fn idempotency_key_cannot_be_reused_for_a_different_payload() {
        let mut ledger = Mt5OrderIntentLedger::open(":memory:").unwrap();
        ledger.prepare(&intent("key-1")).unwrap();
        let mut changed = intent("key-1");
        changed.volume = 0.2;
        assert!(ledger.prepare(&changed).is_err());
    }

    #[test]
    fn unknown_dispatch_must_be_reconciled_before_any_retry() {
        let mut ledger = Mt5OrderIntentLedger::open(":memory:").unwrap();
        let prepared = ledger.prepare(&intent("key-1")).unwrap();
        ledger.transition(&prepared.intent_id, IntentState::Prepared, IntentState::Dispatching, "dispatch_started", None).unwrap();
        ledger.transition(&prepared.intent_id, IntentState::Dispatching, IntentState::Unknown, "timeout_after_send", None).unwrap();
        assert_eq!(ledger.unresolved("demo-server", "123456").unwrap().len(), 1);
        assert!(ledger.transition(&prepared.intent_id, IntentState::Unknown, IntentState::Dispatching, "retry", None).is_err());
        ledger.transition(&prepared.intent_id, IntentState::Unknown, IntentState::Accepted, "broker_ticket_reconciled", Some("9001")).unwrap();
        assert_eq!(ledger.unresolved("demo-server", "123456").unwrap().len(), 0);
    }

    #[test]
    fn concurrent_state_change_is_rejected_by_expected_state() {
        let mut ledger = Mt5OrderIntentLedger::open(":memory:").unwrap();
        let prepared = ledger.prepare(&intent("key-1")).unwrap();
        ledger.transition(&prepared.intent_id, IntentState::Prepared, IntentState::Dispatching, "dispatch_started", None).unwrap();
        assert!(ledger.transition(&prepared.intent_id, IntentState::Prepared, IntentState::Dispatching, "duplicate_dispatch", None).is_err());
    }

    #[test]
    fn invalid_order_payload_is_rejected_before_persistence() {
        let mut ledger = Mt5OrderIntentLedger::open(":memory:").unwrap();
        let mut invalid = intent("key-1");
        invalid.volume = f64::NAN;
        assert!(ledger.prepare(&invalid).is_err());
        assert!(ledger
            .unresolved("demo-server", "123456")
            .unwrap()
            .is_empty());
    }
}
