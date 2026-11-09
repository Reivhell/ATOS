//! Authenticated, localhost-only MetaTrader 5 telemetry bridge.
//!
//! This module intentionally has no trade-submission capability. It imports broker state
//! from an MQL5 Expert Advisor so later execution-control work can be based on verified
//! account, position, pending-order, quote, and symbol metadata rather than synthetic or stale data.

use crate::infrastructure::mt5::market_data::{
    validate_batch, IngestReceipt, Mt5InstrumentSpecification, Mt5InstrumentSpecificationRecord,
    Mt5MarketDataBatch, Mt5MarketDataStatus, Mt5MarketDataStore, Mt5SeriesQuery,
    Mt5SeriesQueryResponse, Mt5SessionCalendarRecord, Mt5SessionSchedule,
};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, VecDeque},
    io::{BufRead, BufReader, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicI64, Ordering},
        Arc, Condvar, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const SERVICE_NAME: &str = "dev.agentictrading.os.mt5-bridge";
const CREDENTIAL_NAME: &str = "pairing-token-v1";
pub const BRIDGE_HOST: &str = "127.0.0.1";
pub const BRIDGE_PORT: u16 = 47650;
pub const PROTOCOL_VERSION: u16 = 6;
const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;
const MAX_POSITIONS: usize = 200;
const MAX_PENDING_ORDERS: usize = 200;
const ORDER_STATE_STARTED: i64 = 0;
const ORDER_STATE_PLACED: i64 = 1;
const ORDER_STATE_PARTIAL: i64 = 3;
const ORDER_STATE_CANCELED: i64 = 2;
const ORDER_STATE_FILLED: i64 = 4;
const ORDER_STATE_REJECTED: i64 = 5;
const ORDER_STATE_EXPIRED: i64 = 6;
const ORDER_STATE_REQUEST_ADD: i64 = 7;
const ORDER_STATE_REQUEST_MODIFY: i64 = 8;
const ORDER_STATE_REQUEST_CANCEL: i64 = 9;
const HEARTBEAT_STALE_AFTER_MS: i64 = 5_000;
const MAX_CLOCK_SKEW_MS: i64 = 60_000;
const MAX_SESSION_CALENDAR_AGE_MS: i64 = 3_660_000;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
const FRAME_TIMEOUT: Duration = Duration::from_secs(4);
const MARGIN_PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(8);
const MARGIN_PREFLIGHT_MAX_AGE_MS: i64 = 5_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5BridgeStatus {
    pub protocol_version: u16,
    pub listening: bool,
    pub address: String,
    pub connected: bool,
    pub stale: bool,
    pub quote_stale: bool,
    pub client_id: Option<String>,
    pub account_login: Option<String>,
    pub account_server: Option<String>,
    pub symbol: Option<String>,
    pub last_heartbeat_at_ms: Option<i64>,
    pub heartbeat_age_ms: Option<i64>,
    pub last_sequence: u64,
    pub last_error: Option<String>,
    pub snapshot: Option<Mt5Snapshot>,
    pub market_data_storage: Option<Mt5MarketDataStatus>,
    pub account_risk: Option<Mt5AccountRiskInventory>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5BridgeSetup {
    pub host: String,
    pub port: u16,
    pub protocol_version: u16,
    pub pairing_token: String,
    pub expert_advisor_path: String,
    pub transport_security: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5Snapshot {
    pub account: Mt5AccountSnapshot,
    pub terminal: Mt5TerminalSnapshot,
    pub symbol: Mt5SymbolSnapshot,
    pub positions: Vec<Mt5PositionSnapshot>,
    pub pending_orders: Vec<Mt5PendingOrderSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5AccountSnapshot {
    pub login: String,
    pub server: String,
    pub currency: String,
    pub balance: f64,
    pub equity: f64,
    pub credit: f64,
    pub profit: f64,
    pub margin: f64,
    pub free_margin: f64,
    pub margin_level_pct: f64,
    pub margin_initial: f64,
    pub margin_maintenance: f64,
    pub margin_call_level: f64,
    pub stop_out_level: f64,
    pub margin_stopout_mode: i64,
    pub margin_mode: i64,
    pub account_hedge_allowed: bool,
    pub account_limit_orders: i64,
    pub leverage: i64,
    pub trade_allowed: bool,
    pub account_trade_allowed: bool,
    pub terminal_trade_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5TerminalSnapshot {
    pub name: String,
    pub company: String,
    pub build: i64,
    pub connected: bool,
    pub algo_trading_allowed: bool,
    pub server_time_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5SymbolSnapshot {
    pub name: String,
    pub bid: f64,
    pub ask: f64,
    pub point: f64,
    pub tick_size: f64,
    pub tick_value: f64,
    pub volume_min: f64,
    pub volume_max: f64,
    pub volume_step: f64,
    pub digits: i64,
    pub trade_mode: i64,
    pub quote_time_ms: i64,
    pub contract_size: f64,
    pub calculation_mode: i64,
    pub execution_mode: i64,
    pub filling_mode: i64,
    pub order_mode: i64,
    pub tick_value_profit: f64,
    pub tick_value_loss: f64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5PositionSnapshot {
    pub ticket: String,
    pub symbol: String,
    pub position_type: i64,
    pub volume: f64,
    pub price_open: f64,
    pub price_current: f64,
    pub profit: f64,
    pub swap: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub opened_at_ms: i64,
    pub magic: String,
    pub risk_status: String,
    pub risk_at_stop_loss: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5PendingOrderSnapshot {
    pub ticket: String,
    pub symbol: String,
    pub order_type: i64,
    pub state: i64,
    pub volume_initial: f64,
    pub volume_current: f64,
    pub price_open: f64,
    pub price_current: f64,
    pub price_stop_limit: f64,
    pub stop_loss: f64,
    pub take_profit: f64,
    pub setup_at_ms: i64,
    pub expiration_at_ms: i64,
    pub magic: String,
    pub risk_status: String,
    pub risk_at_stop_loss: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5AccountRiskInventory {
    pub account_currency: String,
    pub equity: f64,
    pub current_stop_risk: f64,
    pub current_stop_risk_percent: f64,
    pub protected_position_count: usize,
    pub protected_pending_order_count: usize,
    pub unprotected_position_count: usize,
    pub unprotected_pending_order_count: usize,
    pub unresolved_position_risk_count: usize,
    pub unresolved_pending_order_risk_count: usize,
    pub margin_used: f64,
    pub initial_margin: f64,
    pub maintenance_margin: f64,
    pub free_margin: f64,
    pub margin_level_pct: f64,
    pub margin_call_level: f64,
    pub stop_out_level: f64,
    pub margin_stopout_mode: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mt5MarginPreflightCommand {
    pub request_id: String,
    pub symbol: String,
    pub side: String,
    pub volume: f64,
    pub stop_loss_price: f64,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Mt5MarginPreflightResult {
    pub request_id: String,
    pub symbol: String,
    pub side: String,
    pub volume: f64,
    pub success: bool,
    pub calc_margin: Option<f64>,
    pub loss_at_stop_loss: Option<f64>,
    pub price: f64,
    pub bid: f64,
    pub ask: f64,
    pub quote_time_ms: i64,
    pub account_margin: f64,
    pub account_free_margin: f64,
    pub account_equity: f64,
    pub margin_call_level: f64,
    pub stop_out_level: f64,
    pub margin_stopout_mode: i64,
    pub calculated_at_ms: i64,
    pub error_code: i64,
    /// Rust-owned sequence of the authenticated telemetry frame that carried this result.
    /// This value is never supplied by the EA; it binds the native calculation to the exact
    /// account/position/order snapshot received in the same frame.
    #[serde(skip)]
    pub snapshot_sequence: u64,
}

#[derive(Default)]
struct RuntimeState {
    listening: bool,
    connected: bool,
    client_id: Option<String>,
    snapshot: Option<Mt5Snapshot>,
    last_heartbeat_at_ms: Option<i64>,
    last_sequence: u64,
    last_error: Option<String>,
    pending_margin_preflight: Option<Mt5MarginPreflightCommand>,
    margin_preflight_dispatched: bool,
    completed_margin_preflight: Option<Mt5MarginPreflightResult>,
}

pub struct Mt5Bridge {
    state: Arc<Mutex<RuntimeState>>,
    margin_preflight_changed: Arc<Condvar>,
    shutdown: Arc<AtomicBool>,
    address: String,
    startup_error: Option<String>,
    market_data_store: Option<Mt5MarketDataStore>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuthenticationFrame {
    protocol_version: u16,
    kind: String,
    token: String,
    client_id: String,
    sent_at_ms: i64,
    nonce: String,
    session_id: String,
    account_login: String,
    broker_server: String,
    symbols: Vec<String>,
    timeframes: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TelemetryFrame {
    protocol_version: u16,
    kind: String,
    sequence: u64,
    sent_at_ms: i64,
    snapshot: Mt5Snapshot,
    #[serde(default)]
    market_data: Mt5MarketDataBatch,
    session_schedules: Vec<Mt5SessionSchedule>,
    #[serde(default)]
    margin_preflight_results: Vec<Mt5MarginPreflightResult>,
}

impl Mt5Bridge {
    pub fn start(app_data_dir: PathBuf) -> Self {
        let state = Arc::new(Mutex::new(RuntimeState::default()));
        let margin_preflight_changed = Arc::new(Condvar::new());
        let shutdown = Arc::new(AtomicBool::new(false));
        let market_data_path = app_data_dir.join("mt5-market-data.sqlite3");
        let (market_data_store, market_data_error) =
            match Mt5MarketDataStore::open(market_data_path) {
                Ok(store) => (Some(store), None),
                Err(error) => (None, Some(error)),
            };
        let (token_digest_value, credential_error) = match load_or_create_pairing_token() {
            Ok(token_digest) => (token_digest, None),
            Err(error) => ([0; 32], Some(error)),
        };
        let token_digest = Arc::new(Mutex::new(token_digest_value));
        let address = format!("{BRIDGE_HOST}:{BRIDGE_PORT}");

        let startup_error = if let Some(error) = market_data_error.or(credential_error) {
            Some(error)
        } else {
            match TcpListener::bind(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::LOCALHOST),
                BRIDGE_PORT,
            )) {
                Ok(listener) => {
                    if let Err(error) = listener.set_nonblocking(true) {
                        Some(format!(
                            "MT5 bridge listener could not enable non-blocking accept: {error}"
                        ))
                    } else {
                        if let Ok(mut current) = state.lock() {
                            current.listening = true;
                        }
                        let worker_state = Arc::clone(&state);
                        let worker_margin_preflight_changed = Arc::clone(&margin_preflight_changed);
                        let worker_digest = Arc::clone(&token_digest);
                        let worker_shutdown = Arc::clone(&shutdown);
                        match market_data_store.clone() {
                            Some(worker_store) => thread::Builder::new()
                                .name("atos-mt5-bridge-listener".into())
                                .spawn(move || listener_loop(listener, worker_state, worker_margin_preflight_changed, worker_digest, worker_store, worker_shutdown))
                                .map(|_| None)
                                .unwrap_or_else(|error| Some(format!("MT5 bridge listener thread could not start: {error}"))),
                            None => Some("MT5 market-data storage disappeared during bridge startup; listener remains disabled.".into()),
                        }
                    }
                }
                Err(error) => Some(format!("MT5 bridge could not bind to {address}: {error}")),
            }
        };

        if let Some(error) = startup_error.as_ref() {
            if let Ok(mut current) = state.lock() {
                current.listening = false;
                current.last_error = Some(error.clone());
            }
        }

        Self {
            state,
            margin_preflight_changed,
            shutdown,
            address,
            startup_error,
            market_data_store,
        }
    }

    pub fn status(&self) -> Mt5BridgeStatus {
        let now = now_ms();
        let (
            listening,
            connected_state,
            client_id,
            snapshot,
            last_heartbeat_at_ms,
            last_sequence,
            last_error,
        ) = {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            (
                state.listening,
                state.connected,
                state.client_id.clone(),
                state.snapshot.clone(),
                state.last_heartbeat_at_ms,
                state.last_sequence,
                state.last_error.clone(),
            )
        };
        let age = last_heartbeat_at_ms.map(|timestamp| now.saturating_sub(timestamp));
        let fresh = age.is_some_and(|value| (0..=HEARTBEAT_STALE_AFTER_MS).contains(&value));
        let quote_stale = snapshot.as_ref().is_none_or(|current| {
            let quote_age = now.saturating_sub(current.symbol.quote_time_ms);
            !(0..=60_000).contains(&quote_age)
        });
        let connected = connected_state && fresh;
        Mt5BridgeStatus {
            protocol_version: PROTOCOL_VERSION,
            listening,
            address: self.address.clone(),
            connected,
            stale: last_heartbeat_at_ms.is_some() && !fresh,
            quote_stale,
            client_id,
            account_login: snapshot.as_ref().map(|value| value.account.login.clone()),
            account_server: snapshot.as_ref().map(|value| value.account.server.clone()),
            symbol: snapshot.as_ref().map(|value| value.symbol.name.clone()),
            last_heartbeat_at_ms,
            heartbeat_age_ms: age,
            last_sequence,
            last_error: last_error.or_else(|| self.startup_error.clone()),
            snapshot: snapshot.clone(),
            market_data_storage: self
                .market_data_store
                .as_ref()
                .and_then(|store| store.status().ok()),
            account_risk: snapshot
                .as_ref()
                .and_then(|value| account_risk_inventory(value).ok()),
        }
    }

    pub fn authenticated_account_identity(&self) -> Result<(String, String), String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "MT5 bridge runtime identity is unavailable.".to_string())?;
        let snapshot = state.snapshot.as_ref().ok_or_else(|| {
            "An authenticated MT5 account identity is required before querying broker data."
                .to_string()
        })?;
        Ok((
            snapshot.account.server.clone(),
            snapshot.account.login.clone(),
        ))
    }

    pub fn market_data_status(&self) -> Result<Mt5MarketDataStatus, String> {
        self.market_data_store
            .as_ref()
            .ok_or_else(|| {
                self.startup_error
                    .clone()
                    .unwrap_or_else(|| "MT5 market-data store is unavailable.".into())
            })?
            .status()
    }

    pub fn append_risk_decision(&self, record: &Mt5RiskDecisionRecord) -> Result<Mt5RiskDecisionReceipt, String> {
        let (broker_server, account_login) = self.authenticated_account_identity()?;
        if record.broker_server != broker_server || record.account_login != account_login {
            return Err("Risk-decision audit identity does not match the currently authenticated MT5 account.".into());
        }
        self.market_data_store.as_ref().ok_or_else(|| {
            self.startup_error.clone().unwrap_or_else(|| "MT5 risk-decision ledger is unavailable.".into())
        })?.append_risk_decision(record)
    }

    pub fn risk_decision_ledger_status(&self, broker_server: &str, account_login: &str) -> Result<Mt5RiskDecisionLedgerStatus, String> {
        let (authenticated_server, authenticated_login) = self.authenticated_account_identity()?;
        if broker_server != authenticated_server || account_login != authenticated_login {
            return Err("Risk-decision ledger identity does not match the currently authenticated MT5 account.".into());
        }
        self.market_data_store.as_ref().ok_or_else(|| {
            self.startup_error.clone().unwrap_or_else(|| "MT5 risk-decision ledger is unavailable.".into())
        })?.risk_decision_ledger_status(broker_server, account_login)
    }

    pub fn market_data_series_query(
        &self,
        request: Mt5SeriesQuery,
    ) -> Result<Mt5SeriesQueryResponse, String> {
        let authenticated_identity = {
            let state = self
                .state
                .lock()
                .map_err(|_| "MT5 bridge runtime identity is unavailable.".to_string())?;
            let snapshot = state.snapshot.as_ref()
                .ok_or_else(|| "An authenticated MT5 account identity is required before querying stored broker data.".to_string())?;
            (
                snapshot.account.server.clone(),
                snapshot.account.login.clone(),
            )
        };
        if request.broker_server != authenticated_identity.0
            || request.account_login != authenticated_identity.1
        {
            return Err("MT5 market-data query identity does not match the last authenticated terminal account.".into());
        }
        self.market_data_store
            .as_ref()
            .ok_or_else(|| {
                self.startup_error
                    .clone()
                    .unwrap_or_else(|| "MT5 market-data store is unavailable.".into())
            })?
            .query_series(&request)
    }

    pub fn instrument_specification(
        &self,
        broker_server: &str,
        account_login: &str,
        symbol: &str,
    ) -> Result<Mt5InstrumentSpecificationRecord, String> {
        let authenticated_identity = {
            let state = self
                .state
                .lock()
                .map_err(|_| "MT5 bridge runtime identity is unavailable.".to_string())?;
            let snapshot = state.snapshot.as_ref()
                .ok_or_else(|| "An authenticated MT5 account identity is required before querying broker specifications.".to_string())?;
            (
                snapshot.account.server.clone(),
                snapshot.account.login.clone(),
            )
        };
        if broker_server != authenticated_identity.0 || account_login != authenticated_identity.1 {
            return Err("MT5 instrument specification identity does not match the authenticated terminal account.".into());
        }
        self.market_data_store
            .as_ref()
            .ok_or_else(|| {
                self.startup_error
                    .clone()
                    .unwrap_or_else(|| "MT5 market-data store is unavailable.".into())
            })?
            .instrument_specification(broker_server, account_login, symbol)
    }

    pub fn session_calendar(
        &self,
        broker_server: &str,
        account_login: &str,
        symbol: &str,
    ) -> Result<Mt5SessionCalendarRecord, String> {
        let authenticated_identity = {
            let state = self
                .state
                .lock()
                .map_err(|_| "MT5 bridge runtime identity is unavailable.".to_string())?;
            let snapshot = state.snapshot.as_ref()
                .ok_or_else(|| "An authenticated MT5 account identity is required before querying broker sessions.".to_string())?;
            (
                snapshot.account.server.clone(),
                snapshot.account.login.clone(),
            )
        };
        if broker_server != authenticated_identity.0 || account_login != authenticated_identity.1 {
            return Err(
                "MT5 session calendar identity does not match the authenticated terminal account."
                    .into(),
            );
        }
        self.market_data_store
            .as_ref()
            .ok_or_else(|| {
                self.startup_error
                    .clone()
                    .unwrap_or_else(|| "MT5 market-data store is unavailable.".into())
            })?
            .session_calendar(broker_server, account_login, symbol)
    }

    pub fn margin_preflight(
        &self,
        broker_server: &str,
        account_login: &str,
        symbol: &str,
        side: &str,
        volume: f64,
        stop_loss_price: f64,
    ) -> Result<Mt5MarginPreflightResult, String> {
        if !volume.is_finite()
            || volume <= 0.0
            || !stop_loss_price.is_finite()
            || stop_loss_price <= 0.0
            || !matches!(side, "BUY" | "SELL")
        {
            return Err(
                "Margin preflight request contains an invalid side, volume, or stop-loss price."
                    .into(),
            );
        }
        let request = Mt5MarginPreflightCommand {
            request_id: Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            side: side.to_string(),
            volume,
            stop_loss_price,
            expires_at_ms: now_ms().saturating_add(MARGIN_PREFLIGHT_MAX_AGE_MS),
        };
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "MT5 bridge runtime state is unavailable.".to_string())?;
            let snapshot = state.snapshot.as_ref().ok_or_else(|| "No authenticated MT5 account snapshot is available for native margin preflight.".to_string())?;
            if !state.connected
                || snapshot.account.server != broker_server
                || snapshot.account.login != account_login
                || snapshot.symbol.name != symbol
                || !snapshot.terminal.connected
            {
                return Err("Native margin preflight identity or terminal state does not match the authenticated MT5 session.".into());
            }
            if state.pending_margin_preflight.is_some() {
                return Err("Another native MT5 margin preflight is already in progress; concurrent account checks are serialized.".into());
            }
            state.completed_margin_preflight = None;
            state.pending_margin_preflight = Some(request.clone());
            state.margin_preflight_dispatched = false;
        }

        let deadline = Instant::now() + MARGIN_PREFLIGHT_TIMEOUT;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "MT5 bridge runtime state is unavailable.".to_string())?;
        loop {
            if let Some(result) = state.completed_margin_preflight.take() {
                if result.request_id != request.request_id {
                    return Err(
                        "Native MT5 margin preflight returned a mismatched correlation identifier."
                            .into(),
                    );
                }
                validate_margin_preflight_result(&request, &result)?;
                return Ok(result);
            }
            if !state.connected {
                if state
                    .pending_margin_preflight
                    .as_ref()
                    .is_some_and(|pending| pending.request_id == request.request_id)
                {
                    state.pending_margin_preflight = None;
                    state.margin_preflight_dispatched = false;
                }
                return Err(
                    "MT5 terminal disconnected before native margin preflight completed.".into(),
                );
            }
            let now = Instant::now();
            if now >= deadline {
                if state
                    .pending_margin_preflight
                    .as_ref()
                    .is_some_and(|pending| pending.request_id == request.request_id)
                {
                    state.pending_margin_preflight = None;
                    state.margin_preflight_dispatched = false;
                }
                return Err(
                    "Native MT5 margin preflight timed out; no approval was produced.".into(),
                );
            }
            let wait = deadline.saturating_duration_since(now);
            let (next_state, timeout) = self
                .margin_preflight_changed
                .wait_timeout(state, wait)
                .map_err(|_| "MT5 margin-preflight synchronization was poisoned.".to_string())?;
            state = next_state;
            if timeout.timed_out() && state.completed_margin_preflight.is_none() {
                if state
                    .pending_margin_preflight
                    .as_ref()
                    .is_some_and(|pending| pending.request_id == request.request_id)
                {
                    state.pending_margin_preflight = None;
                    state.margin_preflight_dispatched = false;
                }
                return Err(
                    "Native MT5 margin preflight timed out; no approval was produced.".into(),
                );
            }
        }
    }

    pub fn setup(&self) -> Result<Mt5BridgeSetup, String> {
        if let Some(error) = self.startup_error.as_ref() {
            return Err(error.clone());
        }
        let token = read_pairing_token()?;
        Ok(Mt5BridgeSetup {
            host: BRIDGE_HOST.into(),
            port: BRIDGE_PORT,
            protocol_version: PROTOCOL_VERSION,
            pairing_token: token,
            expert_advisor_path: "MQL5/Experts/ATOS_Bridge.mq5".into(),
            transport_security: "Loopback-only TCP with a high-entropy pairing token. The token is not encrypted in transit; do not expose this port beyond 127.0.0.1 or use it across machines.".into(),
        })
    }
}

impl Drop for Mt5Bridge {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        self.margin_preflight_changed.notify_all();
    }
}

fn listener_loop(
    listener: TcpListener,
    state: Arc<Mutex<RuntimeState>>,
    margin_preflight_changed: Arc<Condvar>,
    token_digest: Arc<Mutex<[u8; 32]>>,
    market_data_store: Mt5MarketDataStore,
    shutdown: Arc<AtomicBool>,
) {
    let active_connection = Arc::new(AtomicBool::new(false));
    let last_handshake_ms = Arc::new(AtomicI64::new(0));
    let nonce_guard = Arc::new(Mutex::new(RecentNonceSet::default()));
    while !shutdown.load(Ordering::Acquire) {
        match listener.accept() {
            Ok((stream, peer)) => {
                if let Ok(mut current) = state.lock() {
                    current.listening = true;
                }
                if !peer.ip().is_loopback() {
                    continue;
                }
                if active_connection
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    let mut stream = stream;
                    let _ = write_json_line(
                        &mut stream,
                        &serde_json::json!({"kind":"rejected","reason":"another MT5 terminal session is already attached"}),
                    );
                    continue;
                }
                let worker_state = Arc::clone(&state);
                let worker_margin_preflight_changed = Arc::clone(&margin_preflight_changed);
                let worker_active = Arc::clone(&active_connection);
                let worker_last_handshake = Arc::clone(&last_handshake_ms);
                let worker_nonce_guard = Arc::clone(&nonce_guard);
                let worker_digest = Arc::clone(&token_digest);
                let worker_store = market_data_store.clone();
                let worker = thread::Builder::new()
                    .name("atos-mt5-bridge-session".into())
                    .spawn(move || {
                        let _guard = ActiveConnectionGuard(worker_active);
                        let result = handle_connection(
                            stream,
                            worker_state.clone(),
                            worker_margin_preflight_changed.clone(),
                            worker_digest,
                            worker_last_handshake,
                            worker_nonce_guard,
                            worker_store,
                        );
                        if let Err(error) = result {
                            if let Ok(mut current) = worker_state.lock() {
                                current.connected = false;
                                current.last_error = Some(error);
                            }
                            worker_margin_preflight_changed.notify_all();
                        }
                    });
                if let Err(error) = worker {
                    active_connection.store(false, Ordering::Release);
                    if let Ok(mut current) = state.lock() {
                        current.last_error = Some(format!(
                            "MT5 bridge could not create a session worker: {error}"
                        ));
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50))
            }
            Err(error) => {
                if let Ok(mut current) = state.lock() {
                    current.listening = false;
                    current.connected = false;
                    current.last_error = Some(format!("MT5 bridge listener failed: {error}"));
                }
                thread::sleep(Duration::from_millis(250));
            }
        }
    }
    if let Ok(mut current) = state.lock() {
        current.listening = false;
        current.connected = false;
    }
}

#[derive(Default)]
struct RecentNonceSet {
    seen: BTreeSet<String>,
    order: VecDeque<String>,
}

impl RecentNonceSet {
    fn insert(&mut self, nonce: &str) -> bool {
        if self.seen.contains(nonce) {
            return false;
        }
        let owned = nonce.to_string();
        self.seen.insert(owned.clone());
        self.order.push_back(owned);
        while self.order.len() > 4096 {
            if let Some(expired) = self.order.pop_front() {
                self.seen.remove(&expired);
            }
        }
        true
    }
}

struct ActiveConnectionGuard(Arc<AtomicBool>);
impl Drop for ActiveConnectionGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

fn handle_connection(
    stream: TcpStream,
    state: Arc<Mutex<RuntimeState>>,
    margin_preflight_changed: Arc<Condvar>,
    token_digest: Arc<Mutex<[u8; 32]>>,
    last_handshake_ms: Arc<AtomicI64>,
    nonce_guard: Arc<Mutex<RecentNonceSet>>,
    market_data_store: Mt5MarketDataStore,
) -> Result<(), String> {
    stream
        .set_nodelay(true)
        .map_err(|error| format!("Unable to configure MT5 bridge socket: {error}"))?;
    stream
        .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
        .map_err(|error| format!("Unable to configure MT5 bridge handshake timeout: {error}"))?;
    stream
        .set_write_timeout(Some(HANDSHAKE_TIMEOUT))
        .map_err(|error| format!("Unable to configure MT5 bridge response timeout: {error}"))?;
    let writer = stream
        .try_clone()
        .map_err(|error| format!("Unable to clone MT5 bridge socket: {error}"))?;
    let mut reader = BufReader::with_capacity(4096, stream);
    let auth_bytes = Zeroizing::new(read_bounded_line(&mut reader, 4096)?);
    let mut auth: AuthenticationFrame =
        serde_json::from_slice(auth_bytes.as_slice()).map_err(|_| {
            "MT5 bridge authentication frame is invalid or contains unsupported fields.".to_string()
        })?;
    let supplied_digest = digest(auth.token.as_bytes());
    auth.token.zeroize();
    let now = now_ms();
    let previous = last_handshake_ms.load(Ordering::Acquire);
    if now.saturating_sub(previous) < 1_000
        || last_handshake_ms
            .compare_exchange(previous, now, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
    {
        return Err("MT5 bridge authentication is rate-limited; wait before reconnecting.".into());
    }
    let expected_digest = *token_digest
        .lock()
        .map_err(|_| "MT5 pairing state is unavailable.".to_string())?;
    if auth.protocol_version != PROTOCOL_VERSION || auth.kind != "authenticate" {
        return Err(
            "MT5 bridge protocol version or authentication message type is unsupported.".into(),
        );
    }
    if !constant_time_eq(&supplied_digest, &expected_digest) {
        return Err("MT5 pairing token was rejected.".into());
    }
    if !valid_identifier(&auth.client_id, 192)
        || !valid_identifier(&auth.nonce, 128)
        || auth.nonce.len() < 16
        || !valid_identifier(&auth.session_id, 128)
        || auth.session_id.len() < 16
        || !valid_identifier(&auth.account_login, 32)
        || !valid_identifier(&auth.broker_server, 128)
        || auth.client_id != format!("{}@{}", auth.account_login, auth.broker_server)
        || auth.symbols.is_empty()
        || auth.symbols.len() > 8
        || auth.timeframes.is_empty()
        || auth.timeframes.len() > 6
    {
        return Err("MT5 bridge client identity, synchronization series, or authentication nonce is malformed.".into());
    }
    validate_timestamp(auth.sent_at_ms)?;
    if !nonce_guard
        .lock()
        .map_err(|_| "MT5 nonce-replay guard is unavailable.".to_string())?
        .insert(&auth.nonce)
    {
        return Err(
            "MT5 authentication nonce was already used; reconnect with a fresh nonce.".into(),
        );
    }

    let mut writer = writer;
    let last_committed_sequence =
        market_data_store.last_committed_sequence(&auth.client_id, &auth.session_id)?;
    let cursors = market_data_store.synchronization_cursors(
        &auth.broker_server,
        &auth.account_login,
        &auth.symbols,
        &auth.timeframes,
    )?;
    write_json_line(
        &mut writer,
        &serde_json::json!({
            "kind": "authenticated",
            "protocolVersion": PROTOCOL_VERSION,
            "serverTimeMs": now,
            "maxFrameBytes": MAX_FRAME_BYTES,
            "lastCommittedSequence": last_committed_sequence,
            "tickCursors": cursors.tick_cursors,
            "barCursors": cursors.bar_cursors
        }),
    )?;
    reader
        .get_ref()
        .set_read_timeout(Some(FRAME_TIMEOUT))
        .map_err(|error| format!("Unable to configure MT5 telemetry timeout: {error}"))?;
    writer
        .set_write_timeout(Some(FRAME_TIMEOUT))
        .map_err(|error| format!("Unable to configure MT5 telemetry response timeout: {error}"))?;

    let mut expected_sequence = last_committed_sequence.saturating_add(1);
    {
        let mut current = state
            .lock()
            .map_err(|_| "MT5 bridge runtime state is unavailable.".to_string())?;
        current.connected = true;
        current.client_id = Some(auth.client_id.clone());
        current.last_error = None;
        current.last_sequence = last_committed_sequence;
    }

    loop {
        let bytes = read_bounded_line(&mut reader, MAX_FRAME_BYTES)?;
        let frame: TelemetryFrame = serde_json::from_slice(&bytes).map_err(|_| {
            "MT5 telemetry frame is invalid or contains unsupported fields.".to_string()
        })?;
        if frame.protocol_version != PROTOCOL_VERSION || frame.kind != "snapshot" {
            return Err("MT5 telemetry protocol version or message type is unsupported.".into());
        }
        if frame.sequence != expected_sequence {
            return Err(format!(
                "MT5 telemetry sequence discontinuity: expected {expected_sequence}, received {}.",
                frame.sequence
            ));
        }
        validate_timestamp(frame.sent_at_ms)?;
        if frame.margin_preflight_results.len() > 1 {
            return Err("MT5 telemetry frame exceeded the single outstanding margin-preflight result limit.".into());
        }
        validate_snapshot(&frame.snapshot)?;
        validate_market_data_scope(
            (&auth.account_login, &auth.broker_server),
            &frame.snapshot,
            &frame.market_data,
            &frame.session_schedules,
            frame.sent_at_ms,
            &auth.symbols,
            &auth.timeframes,
        )?;
        validate_batch(&frame.market_data)?;
        let received_at_ms = now_ms();
        let frame_hash = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let receipt: IngestReceipt = market_data_store.commit_frame(
            &auth.client_id,
            &auth.session_id,
            frame.sequence,
            &frame_hash,
            &frame.snapshot.account.login,
            &frame.snapshot.account.server,
            &frame.market_data,
            &instrument_specification_from_snapshot(&frame.snapshot.symbol),
            &auth.symbols,
            &frame.session_schedules,
            received_at_ms,
        )?;
        let margin_preflight_request = {
            let mut current = state
                .lock()
                .map_err(|_| "MT5 bridge runtime state is unavailable.".to_string())?;
            current.snapshot = Some(frame.snapshot);
            current.last_heartbeat_at_ms = Some(received_at_ms);
            current.last_sequence = frame.sequence;
            current.last_error = None;
            current.connected = true;

            if let Some(mut result) = frame.margin_preflight_results.into_iter().next() {
                if current
                    .pending_margin_preflight
                    .as_ref()
                    .is_some_and(|pending| pending.request_id == result.request_id)
                {
                    result.snapshot_sequence = frame.sequence;
                    current.completed_margin_preflight = Some(result);
                    current.pending_margin_preflight = None;
                    current.margin_preflight_dispatched = false;
                    margin_preflight_changed.notify_all();
                }
            }
            if current.pending_margin_preflight.is_some() && !current.margin_preflight_dispatched {
                current.margin_preflight_dispatched = true;
                current.pending_margin_preflight.clone()
            } else {
                None
            }
        };
        write_json_line(
            &mut writer,
            &serde_json::json!({
                "kind": "ack",
                "sequence": expected_sequence,
                "accepted": true,
                "duplicateFrame": receipt.duplicate_frame,
                "insertedTicks": receipt.inserted_ticks,
                "insertedBars": receipt.inserted_bars,
                "revisedBars": receipt.revised_bars,
                "serverTimeMs": received_at_ms,
                "marginPreflightRequest": margin_preflight_request
            }),
        )?;
        expected_sequence = expected_sequence.checked_add(1).ok_or_else(|| {
            "MT5 telemetry sequence has exhausted the protocol range; reconnect the terminal."
                .to_string()
        })?;
    }
}

fn validate_margin_preflight_result(
    request: &Mt5MarginPreflightCommand,
    result: &Mt5MarginPreflightResult,
) -> Result<(), String> {
    let now = now_ms();
    if result.request_id != request.request_id
        || result.symbol != request.symbol
        || result.side != request.side
        || !result.volume.is_finite()
        || (result.volume - request.volume).abs() > request.volume.abs().max(1.0) * 1e-9
    {
        return Err("Native MT5 margin preflight response does not match the requested symbol, side, or volume.".into());
    }
    validate_recent_calculation_timestamp(
        result.calculated_at_ms,
        now_ms(),
        MARGIN_PREFLIGHT_MAX_AGE_MS,
    )?;
    for (label, value) in [
        ("price", result.price),
        ("bid", result.bid),
        ("ask", result.ask),
        ("account margin", result.account_margin),
        ("account equity", result.account_equity),
        ("margin-call level", result.margin_call_level),
        ("stop-out level", result.stop_out_level),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!(
                "Native MT5 margin preflight returned an invalid {label}."
            ));
        }
    }
    if !result.account_free_margin.is_finite() {
        return Err("Native MT5 margin preflight returned a non-finite free-margin value.".into());
    }
    if result.account_equity <= 0.0
        || result.account_free_margin > result.account_equity * 10.0
        || result.account_margin > result.account_equity * 100.0
        || !(0..=1).contains(&result.margin_stopout_mode)
    {
        return Err("Native MT5 margin preflight returned inconsistent account state.".into());
    }
    if !result.success {
        return Err(format!(
            "MT5 OrderCalcMargin or OrderCalcProfit failed (terminal error {}).",
            result.error_code
        ));
    }
    let expected_price = if result.side == "BUY" {
        result.ask
    } else {
        result.bid
    };
    let price_tolerance = expected_price.abs().max(1.0) * 1e-10;
    if result.price <= 0.0
        || result.bid <= 0.0
        || result.ask < result.bid
        || (result.price - expected_price).abs() > price_tolerance
        || result.quote_time_ms <= 0
        || now.saturating_sub(result.quote_time_ms) < 0
        || now.saturating_sub(result.quote_time_ms) > MARGIN_PREFLIGHT_MAX_AGE_MS
    {
        return Err("Successful MT5 native calculation used an invalid, inconsistent, expired, or future-dated bid/ask quote.".into());
    }
    if result
        .calc_margin
        .is_none_or(|value| !value.is_finite() || value < 0.0)
        || result
            .loss_at_stop_loss
            .is_none_or(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(
            "MT5 native margin/loss calculation did not return valid account-currency amounts."
                .into(),
        );
    }
    if now > request.expires_at_ms {
        return Err("Native MT5 margin preflight command expired before the terminal response was returned.".into());
    }
    Ok(())
}

fn validate_recent_calculation_timestamp(
    timestamp_ms: i64,
    now_ms: i64,
    max_age_ms: i64,
) -> Result<(), String> {
    let age_ms = now_ms.saturating_sub(timestamp_ms);
    if timestamp_ms <= 0 || max_age_ms < 0 || age_ms < 0 || age_ms > max_age_ms {
        return Err(
            "Native MT5 margin preflight calculation timestamp is expired or future-dated.".into(),
        );
    }
    Ok(())
}

fn read_bounded_line(reader: &mut BufReader<TcpStream>, maximum: usize) -> Result<Vec<u8>, String> {
    let mut frame = Vec::with_capacity(1024);
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| format!("MT5 bridge socket read failed: {error}"))?;
        if available.is_empty() {
            return Err("MT5 terminal closed the bridge connection.".into());
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        if frame.len().saturating_add(consumed) > maximum.saturating_add(1) {
            return Err(format!(
                "MT5 bridge frame exceeded the {maximum}-byte protocol limit."
            ));
        }
        frame.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if newline.is_some() {
            if frame.last() == Some(&b'\n') {
                frame.pop();
            }
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            if frame.is_empty() {
                return Err("MT5 bridge does not accept empty frames.".into());
            }
            return Ok(frame);
        }
    }
}

fn write_json_line(stream: &mut TcpStream, value: &serde_json::Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec(value)
        .map_err(|_| "MT5 bridge response serialization failed.".to_string())?;
    if bytes.len() > MAX_FRAME_BYTES {
        return Err("MT5 bridge response exceeded its configured frame limit.".into());
    }
    bytes.push(b'\n');
    stream
        .write_all(&bytes)
        .map_err(|error| format!("MT5 bridge response write failed: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("MT5 bridge response flush failed: {error}"))
}

fn validate_market_data_scope(
    authenticated_account: (&str, &str),
    snapshot: &Mt5Snapshot,
    batch: &Mt5MarketDataBatch,
    session_schedules: &[Mt5SessionSchedule],
    frame_sent_at_ms: i64,
    subscribed_symbols: &[String],
    subscribed_timeframes: &[String],
) -> Result<(), String> {
    if snapshot.account.login != authenticated_account.0
        || snapshot.account.server != authenticated_account.1
    {
        return Err("MT5 telemetry account identity changed after authentication; rejecting the session frame.".into());
    }
    let symbol_scope: BTreeSet<&str> = subscribed_symbols.iter().map(String::as_str).collect();
    let timeframe_scope: BTreeSet<&str> =
        subscribed_timeframes.iter().map(String::as_str).collect();
    if !symbol_scope.contains(snapshot.symbol.name.as_str()) {
        return Err(
            "MT5 primary quote symbol is not included in the authenticated subscription.".into(),
        );
    }
    if batch
        .ticks
        .iter()
        .any(|tick| !symbol_scope.contains(tick.symbol.as_str()))
    {
        return Err(
            "MT5 tick frame contains a symbol outside the authenticated subscription.".into(),
        );
    }
    if batch.bars.iter().any(|bar| {
        !symbol_scope.contains(bar.symbol.as_str())
            || !timeframe_scope.contains(bar.timeframe.as_str())
    }) {
        return Err(
            "MT5 bar frame contains a symbol or timeframe outside the authenticated subscription."
                .into(),
        );
    }
    if !session_schedules.is_empty() {
        let schedule_scope: BTreeSet<&str> = session_schedules
            .iter()
            .map(|schedule| schedule.symbol.as_str())
            .collect();
        if schedule_scope != symbol_scope || session_schedules.len() != subscribed_symbols.len() {
            return Err("MT5 frame session calendar update must cover every authenticated symbol exactly once.".into());
        }
    }
    for schedule in session_schedules {
        crate::infrastructure::mt5::market_data::validate_session_schedule(schedule)?;
        let age_ms = frame_sent_at_ms.saturating_sub(schedule.captured_at_ms);
        if !(0..=MAX_SESSION_CALENDAR_AGE_MS).contains(&age_ms) {
            return Err("MT5 broker session calendar is future-dated or older than the one-hour freshness ceiling.".into());
        }
    }
    Ok(())
}

fn instrument_specification_from_snapshot(
    symbol: &Mt5SymbolSnapshot,
) -> Mt5InstrumentSpecification {
    Mt5InstrumentSpecification {
        symbol: symbol.name.clone(),
        digits: symbol.digits,
        point: symbol.point,
        contract_size: symbol.contract_size,
        calculation_mode: symbol.calculation_mode,
        trade_mode: symbol.trade_mode,
        execution_mode: symbol.execution_mode,
        filling_mode: symbol.filling_mode,
        order_mode: symbol.order_mode,
        tick_size: symbol.tick_size,
        tick_value: symbol.tick_value,
        tick_value_profit: symbol.tick_value_profit,
        tick_value_loss: symbol.tick_value_loss,
        volume_min: symbol.volume_min,
        volume_max: symbol.volume_max,
        volume_step: symbol.volume_step,
        volume_limit: symbol.volume_limit,
        stops_level_points: symbol.stops_level_points,
        freeze_level_points: symbol.freeze_level_points,
        currency_base: symbol.currency_base.clone(),
        currency_profit: symbol.currency_profit.clone(),
        currency_margin: symbol.currency_margin.clone(),
        swap_mode: symbol.swap_mode,
        swap_long: symbol.swap_long,
        swap_short: symbol.swap_short,
    }
}

fn validate_snapshot(snapshot: &Mt5Snapshot) -> Result<(), String> {
    let account = &snapshot.account;
    for (label, value) in [
        ("balance", account.balance),
        ("equity", account.equity),
        ("credit", account.credit),
        ("profit", account.profit),
        ("margin", account.margin),
        ("free margin", account.free_margin),
        ("margin level", account.margin_level_pct),
        ("initial margin", account.margin_initial),
        ("maintenance margin", account.margin_maintenance),
        ("margin call level", account.margin_call_level),
        ("stop-out level", account.stop_out_level),
    ] {
        if !value.is_finite() {
            return Err(format!("MT5 account {label} is not finite."));
        }
    }
    if account.login.is_empty()
        || !valid_identifier(&account.login, 32)
        || !valid_identifier(&account.server, 128)
        || !valid_identifier(&account.currency, 16)
    {
        return Err("MT5 account identity fields are malformed.".into());
    }
    if !valid_identifier(&snapshot.terminal.name, 128)
        || !valid_identifier(&snapshot.terminal.company, 128)
        || snapshot.terminal.build < 0
        || snapshot.terminal.server_time_ms <= 0
        || !snapshot.terminal.connected
    {
        return Err(
            "MT5 terminal metadata is malformed or the broker terminal is disconnected.".into(),
        );
    }
    if account.leverage <= 0
        || !(0..=1).contains(&account.margin_stopout_mode)
        || !(0..=2).contains(&account.margin_mode)
        || account.account_limit_orders < 0
        || account.margin < 0.0
        || account.margin_initial < 0.0
        || account.margin_maintenance < 0.0
        || account.margin_call_level < 0.0
        || account.stop_out_level < 0.0
    {
        return Err("MT5 account leverage, margin mode, stop-out policy, or margin balances are outside valid broker ranges.".into());
    }
    if account.stop_out_level > 0.0
        && account.margin_call_level > 0.0
        && account.stop_out_level > account.margin_call_level
    {
        return Err("MT5 broker stop-out threshold exceeds its margin-call threshold; rejecting inconsistent account policy.".into());
    }
    let effective_trade_permission = account.account_trade_allowed
        && account.terminal_trade_allowed
        && snapshot.terminal.algo_trading_allowed;
    if account.trade_allowed != effective_trade_permission {
        return Err("MT5 aggregate trade permission disagrees with its account, terminal, or EA permission flags.".into());
    }
    let symbol = &snapshot.symbol;
    if !valid_identifier(&symbol.name, 64)
        || symbol.digits < 0
        || symbol.digits > 12
        || !(0..=4).contains(&symbol.trade_mode)
        || symbol.quote_time_ms <= 0
    {
        return Err("MT5 symbol metadata is malformed.".into());
    }
    for (label, value) in [
        ("bid", symbol.bid),
        ("ask", symbol.ask),
        ("point", symbol.point),
        ("tick size", symbol.tick_size),
        ("tick value", symbol.tick_value),
        ("minimum volume", symbol.volume_min),
        ("maximum volume", symbol.volume_max),
        ("volume step", symbol.volume_step),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!("MT5 symbol {label} is invalid."));
        }
    }
    if symbol.bid <= 0.0 || symbol.ask <= 0.0 {
        return Err(
            "MT5 symbol bid and ask must both be positive for a valid quote snapshot.".into(),
        );
    }
    if symbol.ask < symbol.bid {
        return Err("MT5 symbol ask is below bid; rejecting inconsistent quote data.".into());
    }
    if symbol.point <= 0.0 || symbol.tick_size <= 0.0 {
        return Err(
            "MT5 symbol point and tick size must be positive broker-reported values.".into(),
        );
    }
    if symbol.volume_min <= 0.0
        || symbol.volume_max < symbol.volume_min
        || symbol.volume_step <= 0.0
    {
        return Err("MT5 symbol volume minimum, maximum, and step must form a positive, consistent contract.".into());
    }
    crate::infrastructure::mt5::market_data::validate_instrument_specification(
        &instrument_specification_from_snapshot(symbol),
    )?;
    if snapshot.positions.len() > MAX_POSITIONS {
        return Err(format!(
            "MT5 position snapshot exceeds the {MAX_POSITIONS}-position safety limit."
        ));
    }
    let mut tickets = BTreeSet::new();
    for position in &snapshot.positions {
        if !valid_identifier(&position.ticket, 32)
            || !tickets.insert(position.ticket.as_str())
            || !valid_identifier(&position.symbol, 64)
            || !valid_identifier(&position.magic, 32)
            || position.position_type < 0
            || position.position_type > 1
            || position.opened_at_ms <= 0
        {
            return Err("MT5 position identity or type is malformed.".into());
        }
        for (label, value) in [
            ("volume", position.volume),
            ("open price", position.price_open),
            ("current price", position.price_current),
            ("stop loss", position.stop_loss),
            ("take profit", position.take_profit),
            ("profit", position.profit),
            ("swap", position.swap),
        ] {
            if !value.is_finite() || ((label != "profit" && label != "swap") && value < 0.0) {
                return Err(format!("MT5 position {label} is invalid."));
            }
        }
        if position.volume <= 0.0 || position.price_open <= 0.0 || position.price_current <= 0.0 {
            return Err("MT5 open position contains a non-positive volume or price.".into());
        }
        validate_stop_risk(
            &position.risk_status,
            position.stop_loss,
            position.risk_at_stop_loss.as_ref(),
            "position",
        )?;
        if position.risk_status == "CALCULATED"
            && !protective_stop_is_loss_side(
                position.position_type == 0,
                position.price_current,
                position.stop_loss,
            )
        {
            return Err("MT5 position is marked as calculated although its stop-loss is not on the loss side of the current mark.".into());
        }
    }
    if snapshot.pending_orders.len() > MAX_PENDING_ORDERS {
        return Err(format!(
            "MT5 pending-order snapshot exceeds the {MAX_PENDING_ORDERS}-order safety limit."
        ));
    }
    let mut order_tickets = BTreeSet::new();
    for order in &snapshot.pending_orders {
        if !valid_identifier(&order.ticket, 32)
            || !order_tickets.insert(order.ticket.as_str())
            || !valid_identifier(&order.symbol, 64)
            || !valid_identifier(&order.magic, 32)
            || !(2..=7).contains(&order.order_type)
            || !is_active_pending_order_state(order.state)
            || order.setup_at_ms <= 0
            || order.expiration_at_ms < 0
        {
            return Err(
                "MT5 pending-order identity, type, state, or timestamps are malformed.".into(),
            );
        }
        for (label, value) in [
            ("initial volume", order.volume_initial),
            ("remaining volume", order.volume_current),
            ("open price", order.price_open),
            ("current price", order.price_current),
            ("stop-limit price", order.price_stop_limit),
            ("stop loss", order.stop_loss),
            ("take profit", order.take_profit),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("MT5 pending-order {label} is invalid."));
            }
        }
        if order.volume_initial <= 0.0
            || order.volume_current <= 0.0
            || order.volume_current > order.volume_initial
            || order.price_open <= 0.0
        {
            return Err("MT5 pending order contains an invalid volume or open price.".into());
        }
        if matches!(order.order_type, 6 | 7) && order.price_stop_limit <= 0.0 {
            return Err(
                "MT5 stop-limit order is missing its broker-reported stop-limit activation price."
                    .into(),
            );
        }
        validate_stop_risk(
            &order.risk_status,
            order.stop_loss,
            order.risk_at_stop_loss.as_ref(),
            "pending order",
        )?;
        let pending_entry = if matches!(order.order_type, 6 | 7) {
            order.price_stop_limit
        } else {
            order.price_open
        };
        if order.risk_status == "CALCULATED"
            && !protective_stop_is_loss_side(
                matches!(order.order_type, 2 | 4 | 6),
                pending_entry,
                order.stop_loss,
            )
        {
            return Err("MT5 pending order is marked as calculated although its stop-loss is not on the loss side of its activation price.".into());
        }
    }
    Ok(())
}

fn is_active_pending_order_state(state: i64) -> bool {
    if [
        ORDER_STATE_CANCELED,
        ORDER_STATE_FILLED,
        ORDER_STATE_REJECTED,
        ORDER_STATE_EXPIRED,
    ]
    .contains(&state)
    {
        return false;
    }
    matches!(
        state,
        ORDER_STATE_STARTED
            | ORDER_STATE_PLACED
            | ORDER_STATE_PARTIAL
            | ORDER_STATE_REQUEST_ADD
            | ORDER_STATE_REQUEST_MODIFY
            | ORDER_STATE_REQUEST_CANCEL
    )
}

fn validate_stop_risk(
    status: &str,
    stop_loss: f64,
    risk: Option<&f64>,
    subject: &str,
) -> Result<(), String> {
    match (status, stop_loss > 0.0, risk) {
        ("UNPROTECTED", false, None) => Ok(()),
        // A zero loss is not proof of a protective stop. It can indicate an SL on the
        // profitable side, a crossed stop, an equal-price stop, or a broken conversion.
        // The EA must classify those as CALCULATION_FAILED so the account-risk gate blocks.
        ("CALCULATED", true, Some(value)) if value.is_finite() && *value > 0.0 => Ok(()),
        ("CALCULATION_FAILED", true, None) => Ok(()),
        _ => Err(format!(
            "MT5 {subject} stop-loss risk status and payload are inconsistent."
        )),
    }
}

fn protective_stop_is_loss_side(is_buy: bool, reference_price: f64, stop_loss: f64) -> bool {
    reference_price.is_finite()
        && stop_loss.is_finite()
        && reference_price > 0.0
        && stop_loss > 0.0
        && if is_buy {
            stop_loss < reference_price
        } else {
            stop_loss > reference_price
        }
}

pub fn account_risk_inventory(snapshot: &Mt5Snapshot) -> Result<Mt5AccountRiskInventory, String> {
    validate_snapshot(snapshot)?;

    let mut current_stop_risk = 0.0_f64;
    let mut protected_position_count = 0_usize;
    let mut protected_pending_order_count = 0_usize;
    let mut unprotected_position_count = 0_usize;
    let mut unprotected_pending_order_count = 0_usize;
    let mut unresolved_position_risk_count = 0_usize;
    let mut unresolved_pending_order_risk_count = 0_usize;

    for position in &snapshot.positions {
        match position.risk_status.as_str() {
            "CALCULATED" => {
                let risk = position.risk_at_stop_loss.ok_or_else(|| {
                    "A calculated position risk is missing its amount.".to_string()
                })?;
                add_account_stop_risk(&mut current_stop_risk, risk, "position")?;
                protected_position_count += 1;
            }
            "UNPROTECTED" => unprotected_position_count += 1,
            "CALCULATION_FAILED" => unresolved_position_risk_count += 1,
            _ => return Err("An unsupported position risk state was encountered.".into()),
        }
    }
    for order in &snapshot.pending_orders {
        match order.risk_status.as_str() {
            "CALCULATED" => {
                let risk = order.risk_at_stop_loss.ok_or_else(|| {
                    "A calculated pending-order risk is missing its amount.".to_string()
                })?;
                add_account_stop_risk(&mut current_stop_risk, risk, "pending order")?;
                protected_pending_order_count += 1;
            }
            "UNPROTECTED" => unprotected_pending_order_count += 1,
            "CALCULATION_FAILED" => unresolved_pending_order_risk_count += 1,
            _ => return Err("An unsupported pending-order risk state was encountered.".into()),
        }
    }
    if !current_stop_risk.is_finite() || snapshot.account.equity <= 0.0 {
        return Err(
            "Account-wide stop-loss risk aggregation overflowed or equity is invalid.".into(),
        );
    }
    let current_stop_risk_percent = current_stop_risk / snapshot.account.equity * 100.0;
    if !current_stop_risk_percent.is_finite() {
        return Err("Account-wide stop-loss risk percentage overflowed; risk inventory is not safe to serialize.".into());
    }
    Ok(Mt5AccountRiskInventory {
        account_currency: snapshot.account.currency.clone(),
        equity: snapshot.account.equity,
        current_stop_risk,
        current_stop_risk_percent,
        protected_position_count,
        protected_pending_order_count,
        unprotected_position_count,
        unprotected_pending_order_count,
        unresolved_position_risk_count,
        unresolved_pending_order_risk_count,
        margin_used: snapshot.account.margin,
        initial_margin: snapshot.account.margin_initial,
        maintenance_margin: snapshot.account.margin_maintenance,
        free_margin: snapshot.account.free_margin,
        margin_level_pct: snapshot.account.margin_level_pct,
        margin_call_level: snapshot.account.margin_call_level,
        stop_out_level: snapshot.account.stop_out_level,
        margin_stopout_mode: snapshot.account.margin_stopout_mode,
    })
}

fn add_account_stop_risk(total: &mut f64, risk: f64, subject: &str) -> Result<(), String> {
    if !risk.is_finite() || risk <= 0.0 {
        return Err(format!(
            "MT5 {subject} stop-loss risk must be finite and strictly positive."
        ));
    }
    let aggregate = *total + risk;
    if !aggregate.is_finite() || aggregate < *total {
        return Err("Account-wide stop-loss risk aggregation overflowed; risk inventory is not safe to serialize.".into());
    }
    *total = aggregate;
    Ok(())
}

fn valid_identifier(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

fn validate_timestamp(timestamp_ms: i64) -> Result<(), String> {
    let delta = now_ms().abs_diff(timestamp_ms);
    if timestamp_ms <= 0 || delta > MAX_CLOCK_SKEW_MS as u64 {
        return Err("MT5 bridge timestamp is outside the permitted clock-skew window.".into());
    }
    Ok(())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right.iter())
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

fn load_or_create_pairing_token() -> Result<[u8; 32], String> {
    let entry = Entry::new(SERVICE_NAME, CREDENTIAL_NAME)
        .map_err(|_| "Operating-system credential store could not be initialized; MT5 bridge remains disabled. No plaintext token fallback was used.".to_string())?;
    match entry.get_password() {
        Ok(token) => {
            let secret = Zeroizing::new(token);
            if !is_valid_token(secret.as_str()) {
                return Err("Stored MT5 pairing token is malformed. Remove the credential and restart the application.".into());
            }
            Ok(digest(secret.as_bytes()))
        }
        Err(keyring::Error::NoEntry) => {
            let token = Zeroizing::new(format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()));
            entry.set_password(token.as_str())
                .map_err(|_| "MT5 pairing token could not be saved to the operating-system credential store. No plaintext token fallback was used.".to_string())?;
            Ok(digest(token.as_bytes()))
        }
        Err(_) => Err("Operating-system credential store could not read the MT5 pairing token; bridge remains disabled.".into()),
    }
}

fn read_pairing_token() -> Result<String, String> {
    let entry = Entry::new(SERVICE_NAME, CREDENTIAL_NAME)
        .map_err(|_| "Operating-system credential store could not be initialized.".to_string())?;
    let token = Zeroizing::new(entry.get_password().map_err(|_| {
        "MT5 pairing token is unavailable in the operating-system credential store.".to_string()
    })?);
    if !is_valid_token(token.as_str()) {
        return Err("Stored MT5 pairing token is malformed.".into());
    }
    Ok(token.as_str().to_string())
}

fn is_valid_token(token: &str) -> bool {
    token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_snapshot() -> Mt5Snapshot {
        Mt5Snapshot {
            account: Mt5AccountSnapshot {
                login: "12345678".into(),
                server: "Broker-Demo".into(),
                currency: "USD".into(),
                balance: 1000.0,
                equity: 1000.0,
                credit: 0.0,
                profit: 0.0,
                margin: 0.0,
                free_margin: 1000.0,
                margin_level_pct: 0.0,
                margin_initial: 0.0,
                margin_maintenance: 0.0,
                margin_call_level: 50.0,
                stop_out_level: 30.0,
                margin_stopout_mode: 0,
                margin_mode: 2,
                account_hedge_allowed: true,
                account_limit_orders: 200,
                leverage: 100,
                trade_allowed: true,
                account_trade_allowed: true,
                terminal_trade_allowed: true,
            },
            terminal: Mt5TerminalSnapshot {
                name: "MetaTrader 5".into(),
                company: "Broker".into(),
                build: 5000,
                connected: true,
                algo_trading_allowed: true,
                server_time_ms: now_ms(),
            },
            symbol: Mt5SymbolSnapshot {
                name: "XAUUSD".into(),
                bid: 2300.0,
                ask: 2300.1,
                point: 0.01,
                tick_size: 0.01,
                tick_value: 1.0,
                volume_min: 0.01,
                volume_max: 100.0,
                volume_step: 0.01,
                digits: 2,
                trade_mode: 4,
                quote_time_ms: now_ms(),
                contract_size: 100.0,
                calculation_mode: 0,
                execution_mode: 2,
                filling_mode: 3,
                order_mode: 127,
                tick_value_profit: 1.0,
                tick_value_loss: 1.0,
                volume_limit: 0.0,
                stops_level_points: 0,
                freeze_level_points: 0,
                currency_base: "XAU".into(),
                currency_profit: "USD".into(),
                currency_margin: "USD".into(),
                swap_mode: 0,
                swap_long: -1.0,
                swap_short: 0.2,
            },
            positions: Vec::new(),
            pending_orders: Vec::new(),
        }
    }

    #[test]
    fn native_margin_calculation_timestamp_rejects_future_and_expired_values() {
        assert!(validate_recent_calculation_timestamp(10_000, 10_000, 5_000).is_ok());
        assert!(validate_recent_calculation_timestamp(5_000, 10_000, 5_000).is_ok());
        assert!(validate_recent_calculation_timestamp(10_001, 10_000, 5_000).is_err());
        assert!(validate_recent_calculation_timestamp(4_999, 10_000, 5_000).is_err());
        assert!(validate_recent_calculation_timestamp(0, 10_000, 5_000).is_err());
    }

    #[test]
    fn rejects_zero_loss_and_non_protective_stops_as_verified_account_protection() {
        let position =
            |position_type: i64, current: f64, stop: f64, risk: f64| Mt5PositionSnapshot {
                ticket: "9019".into(),
                symbol: "XAUUSD".into(),
                position_type,
                volume: 0.01,
                price_open: 2300.0,
                price_current: current,
                profit: 0.0,
                swap: 0.0,
                stop_loss: stop,
                take_profit: 0.0,
                opened_at_ms: now_ms(),
                magic: "0".into(),
                risk_status: "CALCULATED".into(),
                risk_at_stop_loss: Some(risk),
            };
        let mut zero_loss = valid_snapshot();
        zero_loss.positions.push(position(0, 2300.0, 2299.0, 0.0));
        assert!(
            validate_snapshot(&zero_loss).is_err(),
            "zero loss cannot establish protective coverage"
        );

        let mut crossed_buy = valid_snapshot();
        crossed_buy.positions.push(position(0, 2300.0, 2300.1, 1.0));
        assert!(
            validate_snapshot(&crossed_buy).is_err(),
            "a buy stop at/above the live mark is not loss-side protection"
        );

        let mut valid_buy = valid_snapshot();
        valid_buy.positions.push(position(0, 2300.0, 2299.0, 1.0));
        assert!(validate_snapshot(&valid_buy).is_ok());

        let mut valid_sell = valid_snapshot();
        valid_sell.positions.push(position(1, 2300.0, 2301.0, 1.0));
        assert!(validate_snapshot(&valid_sell).is_ok());

        let mut crossed_sell = valid_snapshot();
        crossed_sell
            .positions
            .push(position(1, 2300.0, 2299.9, 1.0));
        assert!(
            validate_snapshot(&crossed_sell).is_err(),
            "a sell stop at/below the live mark is not loss-side protection"
        );
    }

    #[test]
    fn stop_limit_inventory_requires_a_broker_activation_price() {
        let mut snapshot = valid_snapshot();
        snapshot.pending_orders.push(Mt5PendingOrderSnapshot {
            ticket: "7029".into(),
            symbol: "XAUUSD".into(),
            order_type: 6,
            state: 1,
            volume_initial: 0.10,
            volume_current: 0.10,
            price_open: 2300.0,
            price_current: 2300.0,
            price_stop_limit: 0.0,
            stop_loss: 0.0,
            take_profit: 0.0,
            setup_at_ms: now_ms(),
            expiration_at_ms: 0,
            magic: "0".into(),
            risk_status: "UNPROTECTED".into(),
            risk_at_stop_loss: None,
        });
        assert!(validate_snapshot(&snapshot).is_err());
    }

    #[test]
    fn account_risk_inventory_aggregates_positions_and_pending_orders_in_account_currency() {
        let mut snapshot = valid_snapshot();
        snapshot.positions.push(Mt5PositionSnapshot {
            ticket: "9010".into(),
            symbol: "EURUSD".into(),
            position_type: 0,
            volume: 0.10,
            price_open: 1.10,
            price_current: 1.09,
            profit: -100.0,
            swap: 0.0,
            stop_loss: 1.08,
            take_profit: 0.0,
            opened_at_ms: now_ms(),
            magic: "1".into(),
            risk_status: "CALCULATED".into(),
            risk_at_stop_loss: Some(25.0),
        });
        snapshot.positions.push(Mt5PositionSnapshot {
            ticket: "9011".into(),
            symbol: "XAUUSD".into(),
            position_type: 1,
            volume: 0.01,
            price_open: 2300.0,
            price_current: 2301.0,
            profit: -1.0,
            swap: 0.0,
            stop_loss: 0.0,
            take_profit: 0.0,
            opened_at_ms: now_ms(),
            magic: "2".into(),
            risk_status: "UNPROTECTED".into(),
            risk_at_stop_loss: None,
        });
        snapshot.pending_orders.push(Mt5PendingOrderSnapshot {
            ticket: "7010".into(),
            symbol: "GBPUSD".into(),
            order_type: 5,
            state: 1,
            volume_initial: 0.20,
            volume_current: 0.10,
            price_open: 1.25,
            price_current: 1.24,
            price_stop_limit: 0.0,
            stop_loss: 1.26,
            take_profit: 0.0,
            setup_at_ms: now_ms(),
            expiration_at_ms: 0,
            magic: "3".into(),
            risk_status: "CALCULATED".into(),
            risk_at_stop_loss: Some(40.0),
        });
        snapshot.pending_orders.push(Mt5PendingOrderSnapshot {
            ticket: "7011".into(),
            symbol: "USDJPY".into(),
            order_type: 3,
            state: 1,
            volume_initial: 0.10,
            volume_current: 0.10,
            price_open: 150.0,
            price_current: 149.9,
            price_stop_limit: 0.0,
            stop_loss: 150.1,
            take_profit: 0.0,
            setup_at_ms: now_ms(),
            expiration_at_ms: 0,
            magic: "4".into(),
            risk_status: "CALCULATION_FAILED".into(),
            risk_at_stop_loss: None,
        });
        let inventory = account_risk_inventory(&snapshot).unwrap();
        assert_eq!(inventory.current_stop_risk, 65.0);
        assert_eq!(inventory.current_stop_risk_percent, 6.5);
        assert_eq!(inventory.protected_position_count, 1);
        assert_eq!(inventory.protected_pending_order_count, 1);
        assert_eq!(inventory.unprotected_position_count, 1);
        assert_eq!(inventory.unresolved_pending_order_risk_count, 1);
    }

    #[test]
    fn account_risk_inventory_validates_snapshots_at_its_own_boundary() {
        let mut invalid_account = valid_snapshot();
        invalid_account.account.equity = f64::NAN;
        assert!(
            account_risk_inventory(&invalid_account).is_err(),
            "direct callers must not bypass account validation"
        );

        let mut invalid_contract = valid_snapshot();
        invalid_contract.symbol.tick_size = 0.0;
        assert!(
            account_risk_inventory(&invalid_contract).is_err(),
            "direct callers must not bypass broker-contract validation"
        );

        let mut invalid_inventory = valid_snapshot();
        invalid_inventory.positions.push(Mt5PositionSnapshot {
            ticket: "9030".into(),
            symbol: "EURUSD".into(),
            position_type: 0,
            volume: 0.10,
            price_open: 1.10,
            price_current: 1.09,
            profit: -1.0,
            swap: 0.0,
            stop_loss: 1.08,
            take_profit: 0.0,
            opened_at_ms: now_ms(),
            magic: "1".into(),
            risk_status: "CALCULATED".into(),
            risk_at_stop_loss: Some(-1.0),
        });
        assert!(
            account_risk_inventory(&invalid_inventory).is_err(),
            "negative stop-risk cannot enter the aggregate"
        );

        let mut invalid_state = valid_snapshot();
        invalid_state.positions.push(Mt5PositionSnapshot {
            ticket: "9031".into(),
            symbol: "EURUSD".into(),
            position_type: 0,
            volume: 0.10,
            price_open: 1.10,
            price_current: 1.09,
            profit: -1.0,
            swap: 0.0,
            stop_loss: 1.08,
            take_profit: 0.0,
            opened_at_ms: now_ms(),
            magic: "1".into(),
            risk_status: "UNKNOWN".into(),
            risk_at_stop_loss: None,
        });
        assert!(
            account_risk_inventory(&invalid_state).is_err(),
            "unknown risk states must fail at the aggregate boundary"
        );

        let mut invalid_equity = valid_snapshot();
        invalid_equity.account.equity = 0.0;
        assert!(
            account_risk_inventory(&invalid_equity).is_err(),
            "zero equity cannot produce a risk percentage"
        );
    }

    #[test]
    fn account_risk_inventory_rejects_overflowed_aggregation() {
        let mut snapshot = valid_snapshot();
        snapshot.positions.push(Mt5PositionSnapshot {
            ticket: "9020".into(),
            symbol: "EURUSD".into(),
            position_type: 0,
            volume: 0.10,
            price_open: 1.10,
            price_current: 1.09,
            profit: -1.0,
            swap: 0.0,
            stop_loss: 1.08,
            take_profit: 0.0,
            opened_at_ms: now_ms(),
            magic: "1".into(),
            risk_status: "CALCULATED".into(),
            risk_at_stop_loss: Some(f64::MAX),
        });
        snapshot.pending_orders.push(Mt5PendingOrderSnapshot {
            ticket: "7020".into(),
            symbol: "GBPUSD".into(),
            order_type: 5,
            state: 1,
            volume_initial: 0.10,
            volume_current: 0.10,
            price_open: 1.25,
            price_current: 1.24,
            price_stop_limit: 0.0,
            stop_loss: 1.26,
            take_profit: 0.0,
            setup_at_ms: now_ms(),
            expiration_at_ms: 0,
            magic: "3".into(),
            risk_status: "CALCULATED".into(),
            risk_at_stop_loss: Some(f64::MAX),
        });
        assert!(account_risk_inventory(&snapshot).is_err());
    }

    #[test]
    fn accepts_consistent_mt5_account_symbol_and_position_snapshot() {
        assert!(validate_snapshot(&valid_snapshot()).is_ok());
        let mut snapshot = valid_snapshot();
        snapshot.positions.push(Mt5PositionSnapshot {
            ticket: "9002".into(),
            symbol: "XAUUSD".into(),
            position_type: 1,
            volume: 0.01,
            price_open: 2300.0,
            price_current: 2299.9,
            profit: -1.0,
            swap: -0.2,
            stop_loss: 0.0,
            take_profit: 0.0,
            opened_at_ms: now_ms(),
            magic: "0".into(),
            risk_status: "UNPROTECTED".into(),
            risk_at_stop_loss: None,
        });
        assert!(
            validate_snapshot(&snapshot).is_ok(),
            "negative floating P/L and swap are valid broker states"
        );
    }

    #[test]
    fn negative_free_margin_is_preserved_as_a_distressed_broker_state() {
        let mut snapshot = valid_snapshot();
        snapshot.account.margin = 1_200.0;
        snapshot.account.free_margin = -200.0;
        snapshot.account.margin_level_pct = 83.333333;
        assert!(
            validate_snapshot(&snapshot).is_ok(),
            "negative free margin is a valid distressed account state that must remain observable"
        );
    }

    #[test]
    fn validates_complete_pending_order_inventory_and_rejects_duplicates_or_bad_volumes() {
        let pending = Mt5PendingOrderSnapshot {
            ticket: "7001".into(),
            symbol: "XAUUSD".into(),
            order_type: 2,
            state: 3,
            volume_initial: 0.10,
            volume_current: 0.04,
            price_open: 2300.0,
            price_current: 2301.0,
            price_stop_limit: 0.0,
            stop_loss: 0.0,
            take_profit: 0.0,
            setup_at_ms: now_ms(),
            expiration_at_ms: 0,
            magic: "0".into(),
            risk_status: "UNPROTECTED".into(),
            risk_at_stop_loss: None,
        };
        for active_state in [
            ORDER_STATE_STARTED,
            ORDER_STATE_PLACED,
            ORDER_STATE_PARTIAL,
            ORDER_STATE_REQUEST_ADD,
            ORDER_STATE_REQUEST_MODIFY,
            ORDER_STATE_REQUEST_CANCEL,
        ] {
            let mut state_snapshot = valid_snapshot();
            let mut state_order = pending.clone();
            state_order.state = active_state;
            state_snapshot.pending_orders.push(state_order);
            assert!(
                validate_snapshot(&state_snapshot).is_ok(),
                "active MQL5 order state {active_state} must be accepted"
            );
        }
        for terminal_state in [2_i64, 4, 5, 6] {
            let mut state_snapshot = valid_snapshot();
            let mut state_order = pending.clone();
            state_order.state = terminal_state;
            state_snapshot.pending_orders.push(state_order);
            assert!(validate_snapshot(&state_snapshot).is_err(), "terminal MQL5 order state {terminal_state} must not appear in active pending-order inventory");
        }

        let mut snapshot = valid_snapshot();
        snapshot.pending_orders.push(pending.clone());
        assert!(validate_snapshot(&snapshot).is_ok(), "ORDER_STATE_PARTIAL remains active and its remaining volume must stay in the account inventory");
        let inventory = account_risk_inventory(&snapshot).unwrap();
        assert_eq!(
            inventory.unprotected_pending_order_count, 1,
            "remaining partial-order exposure must block new risk until protected"
        );

        snapshot.pending_orders.push(pending.clone());
        assert!(
            validate_snapshot(&snapshot).is_err(),
            "duplicate active order tickets must be rejected"
        );

        let mut snapshot = valid_snapshot();
        let mut malformed = pending;
        malformed.volume_current = f64::NAN;
        snapshot.pending_orders.push(malformed);
        assert!(
            validate_snapshot(&snapshot).is_err(),
            "non-finite order volume must be rejected"
        );
    }

    #[test]
    fn rejects_pending_order_inventory_above_protocol_cap() {
        let mut snapshot = valid_snapshot();
        snapshot.pending_orders = (0..=MAX_PENDING_ORDERS)
            .map(|index| Mt5PendingOrderSnapshot {
                ticket: format!("{index}"),
                symbol: "XAUUSD".into(),
                order_type: 2,
                state: 1,
                volume_initial: 0.01,
                volume_current: 0.01,
                price_open: 2300.0,
                price_current: 2300.0,
                price_stop_limit: 0.0,
                stop_loss: 0.0,
                take_profit: 0.0,
                setup_at_ms: now_ms(),
                expiration_at_ms: 0,
                magic: "0".into(),
                risk_status: "UNPROTECTED".into(),
                risk_at_stop_loss: None,
            })
            .collect();
        assert!(validate_snapshot(&snapshot).is_err());
    }

    #[test]
    fn rejects_non_finite_account_and_symbol_values() {
        let mut snapshot = valid_snapshot();
        snapshot.account.equity = f64::NAN;
        assert!(validate_snapshot(&snapshot).is_err());
        let mut snapshot = valid_snapshot();
        snapshot.symbol.ask = f64::INFINITY;
        assert!(validate_snapshot(&snapshot).is_err());
    }

    #[test]
    fn rejects_missing_or_crossed_quotes_and_inconsistent_volume_rules() {
        let mut snapshot = valid_snapshot();
        snapshot.symbol.bid = 0.0;
        assert!(validate_snapshot(&snapshot).is_err());
        let mut snapshot = valid_snapshot();
        snapshot.symbol.ask = snapshot.symbol.bid - 0.1;
        assert!(validate_snapshot(&snapshot).is_err());
        let mut snapshot = valid_snapshot();
        snapshot.symbol.volume_step = 0.0;
        assert!(validate_snapshot(&snapshot).is_err());
    }

    #[test]
    fn rejects_duplicate_or_invalid_positions() {
        let mut snapshot = valid_snapshot();
        let position = Mt5PositionSnapshot {
            ticket: "9001".into(),
            symbol: "XAUUSD".into(),
            position_type: 0,
            volume: 0.01,
            price_open: 2300.0,
            price_current: 2300.1,
            profit: 1.0,
            swap: 0.0,
            stop_loss: 0.0,
            take_profit: 0.0,
            opened_at_ms: now_ms(),
            magic: "0".into(),
            risk_status: "UNPROTECTED".into(),
            risk_at_stop_loss: None,
        };
        snapshot.positions = vec![position.clone(), position];
        assert!(validate_snapshot(&snapshot).is_err());
        let mut snapshot = valid_snapshot();
        let mut position = Mt5PositionSnapshot {
            ticket: "9001".into(),
            symbol: "XAUUSD".into(),
            position_type: 0,
            volume: 0.0,
            price_open: 2300.0,
            price_current: 2300.1,
            profit: 1.0,
            swap: 0.0,
            stop_loss: 0.0,
            take_profit: 0.0,
            opened_at_ms: now_ms(),
            magic: "0".into(),
            risk_status: "UNPROTECTED".into(),
            risk_at_stop_loss: None,
        };
        position.volume = 0.0;
        snapshot.positions.push(position);
        assert!(validate_snapshot(&snapshot).is_err());
    }

    #[test]
    fn rejects_disconnected_terminal_and_inconsistent_trade_permissions() {
        let mut snapshot = valid_snapshot();
        snapshot.terminal.connected = false;
        assert!(validate_snapshot(&snapshot).is_err());

        let mut snapshot = valid_snapshot();
        snapshot.account.trade_allowed = true;
        snapshot.account.account_trade_allowed = false;
        assert!(validate_snapshot(&snapshot).is_err());

        let mut snapshot = valid_snapshot();
        snapshot.terminal.algo_trading_allowed = false;
        assert!(
            validate_snapshot(&snapshot).is_err(),
            "the aggregate permission flag must reflect disabled Algo Trading"
        );

        let mut snapshot = valid_snapshot();
        snapshot.terminal.algo_trading_allowed = false;
        snapshot.account.trade_allowed = false;
        assert!(validate_snapshot(&snapshot).is_ok(), "a valid read-only terminal state remains observable but must not pass the actionable sizing gate");

        let mut snapshot = valid_snapshot();
        snapshot.account.terminal_trade_allowed = false;
        snapshot.account.trade_allowed = false;
        assert!(validate_snapshot(&snapshot).is_ok(), "terminal-level trading disablement must be represented by the effective permission flag");
    }

    #[test]
    fn rejects_invalid_broker_contract_metadata_and_leverage() {
        let mut snapshot = valid_snapshot();
        snapshot.symbol.point = 0.0;
        assert!(validate_snapshot(&snapshot).is_err());

        let mut snapshot = valid_snapshot();
        snapshot.symbol.tick_size = 0.0;
        assert!(validate_snapshot(&snapshot).is_err());

        let mut snapshot = valid_snapshot();
        snapshot.symbol.volume_max = snapshot.symbol.volume_min / 2.0;
        assert!(validate_snapshot(&snapshot).is_err());

        let mut snapshot = valid_snapshot();
        snapshot.account.leverage = 0;
        assert!(validate_snapshot(&snapshot).is_err());
    }

    #[test]
    fn pairing_token_comparison_is_fixed_width_and_token_format_is_strict() {
        let valid = [7_u8; 32];
        assert!(constant_time_eq(&valid, &valid));
        assert!(!constant_time_eq(&valid, &[8_u8; 32]));
        assert!(is_valid_token(&"a".repeat(64)));
        assert!(!is_valid_token(&"a".repeat(63)));
        assert!(!is_valid_token(&"g".repeat(64)));
    }

    #[test]
    fn authentication_nonce_replay_is_rejected_and_cache_is_bounded() {
        let mut guard = RecentNonceSet::default();
        assert!(guard.insert("session-nonce-0001"));
        assert!(!guard.insert("session-nonce-0001"));
        for index in 0..4097 {
            assert!(guard.insert(&format!("nonce-{index:04}")));
        }
        assert!(guard.order.len() <= 4096);
        assert!(!guard.seen.contains("session-nonce-0001"));
    }

    #[test]
    fn timestamp_guard_rejects_future_and_expired_frames() {
        assert!(validate_timestamp(now_ms()).is_ok());
        assert!(validate_timestamp(now_ms() + MAX_CLOCK_SKEW_MS * 2).is_err());
        assert!(validate_timestamp(now_ms() - MAX_CLOCK_SKEW_MS * 2).is_err());
    }

    use crate::infrastructure::mt5::market_data::{
        Mt5Bar, Mt5SessionDay, Mt5SessionSchedule, Mt5Tick,
    };

    fn snapshot() -> Mt5Snapshot {
        Mt5Snapshot {
            account: Mt5AccountSnapshot {
                login: "12345".into(),
                server: "Broker-Live".into(),
                currency: "USD".into(),
                balance: 1_000.0,
                equity: 1_000.0,
                credit: 0.0,
                profit: 0.0,
                margin: 0.0,
                free_margin: 1_000.0,
                margin_level_pct: 0.0,
                margin_initial: 0.0,
                margin_maintenance: 0.0,
                margin_call_level: 50.0,
                stop_out_level: 30.0,
                margin_stopout_mode: 0,
                margin_mode: 2,
                account_hedge_allowed: true,
                account_limit_orders: 200,
                leverage: 100,
                trade_allowed: false,
                account_trade_allowed: false,
                terminal_trade_allowed: false,
            },
            terminal: Mt5TerminalSnapshot {
                name: "MetaTrader 5".into(),
                company: "Broker".into(),
                build: 0,
                connected: true,
                algo_trading_allowed: false,
                server_time_ms: 1,
            },
            symbol: Mt5SymbolSnapshot {
                name: "XAUUSD.a".into(),
                bid: 2300.0,
                ask: 2300.2,
                point: 0.01,
                tick_size: 0.01,
                tick_value: 1.0,
                volume_min: 0.01,
                volume_max: 100.0,
                volume_step: 0.01,
                digits: 2,
                trade_mode: 0,
                quote_time_ms: 1,
                contract_size: 100.0,
                calculation_mode: 0,
                execution_mode: 2,
                filling_mode: 3,
                order_mode: 127,
                tick_value_profit: 1.0,
                tick_value_loss: 1.0,
                volume_limit: 0.0,
                stops_level_points: 0,
                freeze_level_points: 0,
                currency_base: "XAU".into(),
                currency_profit: "USD".into(),
                currency_margin: "USD".into(),
                swap_mode: 0,
                swap_long: -1.0,
                swap_short: 0.2,
            },
            positions: Vec::new(),
            pending_orders: Vec::new(),
        }
    }

    #[test]
    fn frame_scope_binds_account_and_market_data_to_authenticated_subscriptions() {
        let symbols = vec!["XAUUSD.a".to_string(), "EURUSD".to_string()];
        let timeframes = vec!["M1".to_string(), "M5".to_string()];
        let schedules: Vec<Mt5SessionSchedule> = symbols
            .iter()
            .map(|symbol| Mt5SessionSchedule {
                symbol: symbol.clone(),
                captured_at_ms: 1,
                days: (0..7)
                    .map(|day| Mt5SessionDay {
                        day_of_week: day,
                        sessions: Vec::new(),
                    })
                    .collect(),
            })
            .collect();
        let valid = snapshot();
        assert!(validate_market_data_scope(
            ("12345", "Broker-Live"),
            &valid,
            &Mt5MarketDataBatch::default(),
            &schedules,
            1,
            &symbols,
            &timeframes
        )
        .is_ok());
        assert!(
            validate_market_data_scope(
                ("12345", "Broker-Live"),
                &valid,
                &Mt5MarketDataBatch::default(),
                &[],
                1,
                &symbols,
                &timeframes
            )
            .is_ok(),
            "transport may omit an unchanged calendar; the durable store verifies retained state"
        );

        assert!(validate_market_data_scope(
            ("99999", "Broker-Live"),
            &valid,
            &Mt5MarketDataBatch::default(),
            &schedules,
            1,
            &symbols,
            &timeframes
        )
        .is_err());
        assert!(validate_market_data_scope(
            ("12345", "Other-Broker"),
            &valid,
            &Mt5MarketDataBatch::default(),
            &schedules,
            1,
            &symbols,
            &timeframes
        )
        .is_err());

        let out_of_scope_tick = Mt5MarketDataBatch {
            ticks: vec![Mt5Tick {
                symbol: "GBPUSD".into(),
                time_ms: 1,
                bid: 1.0,
                ask: 1.1,
                last: 0.0,
                volume: 0,
                volume_real: 0.0,
                flags: 0,
            }],
            bars: Vec::new(),
        };
        assert!(validate_market_data_scope(
            ("12345", "Broker-Live"),
            &valid,
            &out_of_scope_tick,
            &schedules,
            1,
            &symbols,
            &timeframes
        )
        .is_err());

        let out_of_scope_bar = Mt5MarketDataBatch {
            ticks: Vec::new(),
            bars: vec![Mt5Bar {
                symbol: "XAUUSD.a".into(),
                timeframe: "H1".into(),
                time_ms: 1,
                open: 1.0,
                high: 1.0,
                low: 1.0,
                close: 1.0,
                tick_volume: 0,
                spread: 0,
                real_volume: 0,
                closed: true,
            }],
        };
        assert!(validate_market_data_scope(
            ("12345", "Broker-Live"),
            &valid,
            &out_of_scope_bar,
            &schedules,
            1,
            &symbols,
            &timeframes
        )
        .is_err());
        assert!(validate_market_data_scope(
            ("12345", "Broker-Live"),
            &valid,
            &Mt5MarketDataBatch::default(),
            &schedules[..1],
            1,
            &symbols,
            &timeframes
        )
        .is_err());
        assert!(
            validate_market_data_scope(
                ("12345", "Broker-Live"),
                &valid,
                &Mt5MarketDataBatch::default(),
                &schedules,
                MAX_SESSION_CALENDAR_AGE_MS + 2,
                &symbols,
                &timeframes
            )
            .is_err(),
            "cached broker calendars older than the one-hour ceiling must fail closed"
        );
    }
}
