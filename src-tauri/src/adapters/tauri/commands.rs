//! Tauri IPC adapter. All commands delegate to application/runtime services.
use crate::{
    application::{
        backtest, market_data_platform, market_data_query, mt5_risk_sizing, research_workspace,
        strict_analysis, strict_analysis_service,
    },
    domain::{
        technical_analysis::{self, IndicatorReading},
        market_data_platform::{
            MarketDataProviderCatalogEntry, MarketDataQuery, UnifiedMarketDataResult,
        },
        models::{
            Agent, AuditEvent, BacktestRequest, BacktestResult, Candle, CandleSeriesRequest,
            CandleSeriesResponse, DashboardSnapshot, PaperOrder, PlaceOrderRequest, RiskLimits,
        },
    },
    infrastructure::{
        ai::agent_runtime as ai_agent_runtime,
        mt5::{bridge as mt5_bridge, market_data as mt5_market_data},
        providers::{meme_coin_hunter, public_market},
    },
    runtime::trading_core::TradingCore,
};
use chrono::{SecondsFormat, Utc};
use tauri::{Manager, State};

#[tauri::command]
pub fn calculate_technical_indicator_readings(
    candles: Vec<Candle>,
) -> Result<Vec<IndicatorReading>, String> {
    technical_analysis::calculate_readings(&candles)
}

#[tauri::command]
pub fn get_dashboard_state(core: State<'_, TradingCore>) -> Result<DashboardSnapshot, String> {
    core.snapshot()
}

#[tauri::command]
pub fn get_mt5_bridge_status(
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
) -> mt5_bridge::Mt5BridgeStatus {
    bridge.status()
}

#[tauri::command]
pub fn get_mt5_bridge_setup(
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
) -> Result<mt5_bridge::Mt5BridgeSetup, String> {
    bridge.setup()
}

#[tauri::command]
pub fn get_mt5_market_data_status(
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
) -> Result<mt5_market_data::Mt5MarketDataStatus, String> {
    market_data_query::status(bridge.inner())
}

#[tauri::command]
pub fn query_mt5_market_data_series(
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
    request: mt5_market_data::Mt5SeriesQuery,
) -> Result<mt5_market_data::Mt5SeriesQueryResponse, String> {
    market_data_query::query(bridge.inner(), request)
}

#[tauri::command]
pub fn get_mt5_instrument_specification(
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
    broker_server: String,
    account_login: String,
    symbol: String,
) -> Result<mt5_market_data::Mt5InstrumentSpecificationRecord, String> {
    market_data_query::instrument_specification(
        bridge.inner(),
        &broker_server,
        &account_login,
        &symbol,
    )
}

#[tauri::command]
pub fn calculate_mt5_risk_sizing(
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
    request: mt5_risk_sizing::Mt5RiskSizingRequest,
) -> Result<mt5_risk_sizing::Mt5RiskSizingResult, String> {
    mt5_risk_sizing::calculate(bridge.inner(), request)
}

#[tauri::command]
pub fn get_mt5_risk_decision_ledger_status(
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
    broker_server: String,
    account_login: String,
) -> Result<mt5_market_data::Mt5RiskDecisionLedgerStatus, String> {
    bridge.risk_decision_ledger_status(&broker_server, &account_login)
}

#[tauri::command]
pub fn get_mt5_session_calendar(
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
    broker_server: String,
    account_login: String,
    symbol: String,
) -> Result<mt5_market_data::Mt5SessionCalendarRecord, String> {
    market_data_query::session_calendar(bridge.inner(), &broker_server, &account_login, &symbol)
}

#[tauri::command]
pub fn get_market_data_provider_catalog(
    core: State<'_, TradingCore>,
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
) -> Result<Vec<MarketDataProviderCatalogEntry>, String> {
    market_data_platform::provider_catalog(core.inner(), bridge.inner())
}

#[tauri::command]
pub fn query_unified_market_data(
    core: State<'_, TradingCore>,
    bridge: State<'_, mt5_bridge::Mt5Bridge>,
    request: MarketDataQuery,
) -> Result<UnifiedMarketDataResult, String> {
    market_data_platform::query(core.inner(), bridge.inner(), request)
}

#[tauri::command]
pub fn get_candle_series(
    core: State<'_, TradingCore>,
    request: CandleSeriesRequest,
) -> Result<CandleSeriesResponse, String> {
    core.get_candle_series(request)
}

#[tauri::command]
pub fn place_paper_order(
    core: State<'_, TradingCore>,
    request: PlaceOrderRequest,
) -> Result<PaperOrder, String> {
    core.place_order(request)
}

#[tauri::command]
pub fn run_backtest(
    core: State<'_, TradingCore>,
    request: BacktestRequest,
) -> Result<BacktestResult, String> {
    core.run_backtest(request)
}

#[tauri::command]
pub fn set_agent_running(
    core: State<'_, TradingCore>,
    agent_id: String,
    running: bool,
) -> Result<Agent, String> {
    core.toggle_agent(agent_id, running)
}

#[tauri::command]
pub fn get_ai_agent_runtime_status(
    app: tauri::AppHandle,
) -> Result<ai_agent_runtime::AgentRuntimeStatus, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::runtime_status(data_dir)
}

#[tauri::command]
pub fn save_ai_provider_key(
    app: tauri::AppHandle,
    provider_id: String,
    api_key: String,
) -> Result<ai_agent_runtime::ProviderStatus, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::save_provider_key(provider_id, api_key, data_dir)
}

#[tauri::command]
pub fn remove_ai_provider_key(
    app: tauri::AppHandle,
    provider_id: String,
) -> Result<ai_agent_runtime::ProviderStatus, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::remove_provider_key(provider_id, data_dir)
}

#[tauri::command]
pub async fn test_ai_provider_connection(
    app: tauri::AppHandle,
    provider_id: String,
) -> Result<ai_agent_runtime::ProviderConnectionResult, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::test_provider_connection(provider_id, data_dir).await
}

#[tauri::command]
pub fn list_imported_ai_models(
    app: tauri::AppHandle,
) -> Result<Vec<ai_agent_runtime::ImportedModelArtifact>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::list_imported_models(data_dir)
}

#[tauri::command]
pub fn import_local_ai_model(
    app: tauri::AppHandle,
    source_path: String,
    expected_sha256: Option<String>,
) -> Result<ai_agent_runtime::ImportedModelArtifact, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::import_local_model(data_dir, source_path, expected_sha256)
}

#[tauri::command]
pub fn remove_imported_ai_model(
    app: tauri::AppHandle,
    model_id: String,
) -> Result<ai_agent_runtime::ModelRemovalResult, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::remove_imported_model(data_dir, model_id)
}

#[tauri::command]
pub fn verify_imported_ai_model(
    app: tauri::AppHandle,
    model_id: String,
) -> Result<ai_agent_runtime::ImportedModelArtifact, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::verify_imported_model(data_dir, model_id)
}

#[tauri::command]
pub fn verify_ai_agent_audit_chain(
    app: tauri::AppHandle,
) -> Result<ai_agent_runtime::AgentAuditStatus, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    ai_agent_runtime::verify_audit_chain(data_dir)
}

#[tauri::command]
pub async fn run_ai_research_agent(
    app: tauri::AppHandle,
    core: State<'_, TradingCore>,
    request: ai_agent_runtime::AgentResearchRequest,
) -> Result<ai_agent_runtime::AgentResearchReport, String> {
    ai_agent_runtime::run_research_agent(app, core.inner(), request).await
}

#[tauri::command]
pub fn set_kill_switch(core: State<'_, TradingCore>, engaged: bool) -> Result<bool, String> {
    core.toggle_kill_switch(engaged)
}

#[tauri::command]
pub fn reset_paper_account(core: State<'_, TradingCore>) -> Result<DashboardSnapshot, String> {
    core.reset_paper_account()
}

#[tauri::command]
pub fn update_risk_limits(
    core: State<'_, TradingCore>,
    risk_limits: RiskLimits,
) -> Result<DashboardSnapshot, String> {
    core.update_risk_limits(risk_limits)
}

#[tauri::command]
pub fn get_audit_history(
    core: State<'_, TradingCore>,
    limit: Option<usize>,
) -> Result<Vec<AuditEvent>, String> {
    core.audit_history(limit.unwrap_or(250))
}

#[tauri::command]
pub async fn refresh_public_screener(
    core: State<'_, TradingCore>,
) -> Result<DashboardSnapshot, String> {
    public_market::refresh_public_screener(core.inner()).await
}

#[tauri::command]
pub fn get_meme_coin_hunter_security_status() -> meme_coin_hunter::MemeCoinHunterSecurityStatus {
    meme_coin_hunter::security_status()
}

#[tauri::command]
pub fn save_meme_coin_hunter_security_token(
    token: String,
) -> Result<meme_coin_hunter::MemeCoinHunterSecurityStatus, String> {
    meme_coin_hunter::save_security_token(token)
}

#[tauri::command]
pub fn remove_meme_coin_hunter_security_token(
) -> Result<meme_coin_hunter::MemeCoinHunterSecurityStatus, String> {
    meme_coin_hunter::remove_security_token()
}

#[tauri::command]
pub async fn run_meme_coin_hunter(
    app: tauri::AppHandle,
) -> Result<meme_coin_hunter::MemeCoinHunterReport, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    meme_coin_hunter::run_scan(data_dir).await
}

#[tauri::command]
pub fn get_latest_meme_coin_hunter_report(
    app: tauri::AppHandle,
) -> Result<Option<meme_coin_hunter::MemeCoinHunterReport>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application data directory is unavailable.".to_string())?;
    meme_coin_hunter::latest_report(data_dir)
}

#[tauri::command]
pub async fn refresh_public_market_data(
    core: State<'_, TradingCore>,
) -> Result<DashboardSnapshot, String> {
    public_market::refresh_public_market_data(core.inner()).await
}

#[tauri::command]
pub fn create_strict_analysis_work_plan(
    core: State<'_, TradingCore>,
    mut request: strict_analysis::CreateWorkPlanRequest,
) -> Result<strict_analysis::StrictAnalysisWorkPlan, String> {
    let series = strict_analysis_service::authoritative_strict_series(
        core.inner(),
        &request.symbol,
        &request.timeframe,
    )?;
    let as_of =
        chrono::DateTime::<Utc>::from_timestamp_millis(series.updated_at_ms).ok_or_else(|| {
            "Provider timestamp cannot be represented as an ISO-8601 snapshot cutoff.".to_string()
        })?;
    request.symbol = series.request.symbol.clone();
    request.timeframe = series.request.interval.as_str().to_string();
    request.as_of = as_of.to_rfc3339_opts(SecondsFormat::Millis, true);
    request.dataset_sha256 = backtest::dataset_sha256(&series.candles);
    strict_analysis::create_work_plan_persisted(request, &core.strict_analysis_store_path())
}

#[tauri::command]
pub fn evaluate_strict_analysis_completion(
    core: State<'_, TradingCore>,
    mut request: strict_analysis::StrictCompletionRequest,
) -> Result<strict_analysis::StrictCompletionResult, String> {
    let symbol = request.work_plan.symbol.clone();
    let timeframe = request.work_plan.timeframe.clone();
    let series =
        strict_analysis_service::authoritative_strict_series(core.inner(), &symbol, &timeframe)?;
    let fingerprint = backtest::dataset_sha256(&series.candles);
    // Rebuild the market context from the native provider response. The UI payload is not trusted.
    request.market_data = strict_analysis::StrictMarketDataContext {
        validated: true,
        source: series.source.clone(),
        as_of: request.work_plan.as_of.clone(),
        dataset_sha256: fingerprint,
        symbol: series.request.symbol.clone(),
        timeframe: series.request.interval.as_str().to_string(),
        candles: series.candles,
    };
    // Ignore caller-supplied risk.passed; derive a new approval from Rust-owned account state.
    request.risk = match core.strict_analysis_risk_preflight() {
        Ok(reason) => strict_analysis::StrictRiskApproval {
            passed: true,
            reason,
        },
        Err(reason) => strict_analysis::StrictRiskApproval {
            passed: false,
            reason,
        },
    };
    strict_analysis::evaluate_completion_persisted(request, &core.strict_analysis_store_path())
}

#[tauri::command]
pub fn verify_strict_analysis_audit_chain(
    core: State<'_, TradingCore>,
) -> Result<strict_analysis::StrictAuditChainStatus, String> {
    strict_analysis::verify_persisted_audit_chain(&core.strict_analysis_store_path())
}

#[tauri::command]
pub fn create_research_hypothesis(
    workspace: State<'_, research_workspace::ResearchWorkspace>,
    request: research_workspace::CreateResearchHypothesisRequest,
) -> Result<research_workspace::ResearchHypothesis, String> {
    workspace.create_hypothesis(request)
}

#[tauri::command]
pub fn list_research_hypotheses(
    workspace: State<'_, research_workspace::ResearchWorkspace>,
    limit: Option<usize>,
) -> Result<Vec<research_workspace::ResearchHypothesis>, String> {
    workspace.list_hypotheses(limit.unwrap_or(50))
}

#[tauri::command]
pub fn invalidate_research_hypothesis(
    workspace: State<'_, research_workspace::ResearchWorkspace>,
    hypothesis_id: String,
    reason: String,
    expected_revision: i64,
) -> Result<research_workspace::ResearchHypothesis, String> {
    workspace.invalidate_hypothesis(hypothesis_id, reason, expected_revision)
}

#[tauri::command]
pub fn execute_research_workflow(
    workspace: State<'_, research_workspace::ResearchWorkspace>,
    core: State<'_, TradingCore>,
    request: research_workspace::ResearchWorkflowRequest,
) -> Result<research_workspace::ResearchRunCardRecord, String> {
    workspace.run_workflow(core.inner(), request)
}

#[tauri::command]
pub fn list_research_run_cards(
    workspace: State<'_, research_workspace::ResearchWorkspace>,
    limit: Option<usize>,
) -> Result<Vec<research_workspace::ResearchRunCardRecord>, String> {
    workspace.list_run_cards(limit.unwrap_or(40))
}

#[tauri::command]
pub fn get_research_run_card(
    workspace: State<'_, research_workspace::ResearchWorkspace>,
    run_id: String,
) -> Result<research_workspace::ResearchRunCardRecord, String> {
    workspace.get_run_card(run_id)
}

#[tauri::command]
pub fn verify_research_workspace(
    workspace: State<'_, research_workspace::ResearchWorkspace>,
) -> Result<research_workspace::ResearchIntegrityStatus, String> {
    workspace.verify_integrity()
}
