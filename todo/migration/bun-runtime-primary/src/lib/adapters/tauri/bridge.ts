import { invoke } from '@tauri-apps/api/core';
import { assertNativeRuntime } from './native-runtime-guard.js';
import type { AuditEvent, BacktestRequest, BacktestResult, CandleSeriesRequest, CandleSeriesResponse, CreateStrictWorkPlanRequest, CreateResearchHypothesisRequest, DashboardSnapshot, MarketDataProviderCatalogEntry, UnifiedMarketDataQuery, UnifiedMarketDataResult, Mt5BridgeStatus, Mt5BridgeSetup, Mt5InstrumentSpecificationRecord, Mt5SessionCalendarRecord, Mt5RiskSizingRequest, Mt5RiskSizingResult, Mt5SeriesQuery, Mt5SeriesQueryResponse, PlaceOrderRequest, ResearchHypothesis, ResearchIntegrityStatus, ResearchRunCardRecord, ResearchWorkflowRequest, RiskLimits, StrictAnalysisWorkPlan, MemeCoinHunterReport, MemeCoinHunterSecurityStatus, StrictAuditChainStatus, StrictCompletionRequest, StrictCompletionResult } from '../../types';

export const isDesktopRuntime = (): boolean =>
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function requireDesktopRuntime(capability: Parameters<typeof assertNativeRuntime>[1]): void {
  assertNativeRuntime(isDesktopRuntime(), capability);
}

export async function getDashboardState(): Promise<DashboardSnapshot> {
  requireDesktopRuntime('dashboard');
  return invoke<DashboardSnapshot>('get_dashboard_state');
}

export async function getMt5BridgeStatus(): Promise<Mt5BridgeStatus> {
  requireDesktopRuntime('MT5 bridge');
  return invoke<Mt5BridgeStatus>('get_mt5_bridge_status');
}

/** Reveal the pairing token only on explicit operator request; it is stored in the OS credential vault. */
export async function getMt5BridgeSetup(): Promise<Mt5BridgeSetup> {
  requireDesktopRuntime('MT5 bridge setup');
  return invoke<Mt5BridgeSetup>('get_mt5_bridge_setup');
}

export async function getMt5InstrumentSpecification(
  brokerServer: string,
  accountLogin: string,
  symbol: string
): Promise<Mt5InstrumentSpecificationRecord> {
  requireDesktopRuntime('MT5 instrument specification');
  if (typeof brokerServer !== 'string' || brokerServer.trim().length === 0 || brokerServer.length > 128
    || typeof accountLogin !== 'string' || accountLogin.trim().length === 0 || accountLogin.length > 32
    || typeof symbol !== 'string' || symbol.trim().length === 0 || symbol.length > 64) {
    throw new TypeError('A valid authenticated broker, account, and symbol identity is required.');
  }
  return invoke<Mt5InstrumentSpecificationRecord>('get_mt5_instrument_specification', {
    brokerServer, accountLogin, symbol
  });
}

export async function getMt5SessionCalendar(
  brokerServer: string,
  accountLogin: string,
  symbol: string
): Promise<Mt5SessionCalendarRecord> {
  requireDesktopRuntime('MT5 session calendar');
  if (typeof brokerServer !== 'string' || brokerServer.trim().length === 0 || brokerServer.length > 128
    || typeof accountLogin !== 'string' || accountLogin.trim().length === 0 || accountLogin.length > 32
    || typeof symbol !== 'string' || symbol.trim().length === 0 || symbol.length > 64) {
    throw new TypeError('A valid authenticated broker, account, and symbol identity is required.');
  }
  return invoke<Mt5SessionCalendarRecord>('get_mt5_session_calendar', { brokerServer, accountLogin, symbol });
}

export async function calculateMt5RiskSizing(request: Mt5RiskSizingRequest): Promise<Mt5RiskSizingResult> {
  requireDesktopRuntime('MT5 risk sizing');
  if (!request || typeof request !== 'object') throw new TypeError('A valid MT5 risk-sizing request is required.');
  if (typeof request.brokerServer !== 'string' || !request.brokerServer.trim() || request.brokerServer.length > 128
    || typeof request.accountLogin !== 'string' || !request.accountLogin.trim() || request.accountLogin.length > 32
    || typeof request.symbol !== 'string' || !request.symbol.trim() || request.symbol.length > 64) {
    throw new TypeError('A valid broker, account, and symbol identity is required.');
  }
  if (!Number.isFinite(request.stopLossPrice) || request.stopLossPrice <= 0) throw new RangeError('Stop-loss price must be finite and positive.');
  if (!Number.isFinite(request.riskPercent) || request.riskPercent < 0.01 || request.riskPercent > 5) throw new RangeError('Risk percentage must be between 0.01 and 5.');
  if (!['BUY', 'SELL'].includes(request.side)) throw new TypeError('Side must be BUY or SELL.');
  return invoke<Mt5RiskSizingResult>('calculate_mt5_risk_sizing', { request });
}

export async function queryMt5MarketDataSeries(request: Mt5SeriesQuery): Promise<Mt5SeriesQueryResponse> {
  requireDesktopRuntime('MT5 market-data series');
  if (!request || typeof request !== 'object') throw new TypeError('A valid MT5 series query is required.');
  if (!Number.isSafeInteger(request.fromMs) || !Number.isSafeInteger(request.toMs) || request.fromMs <= 0 || request.toMs < request.fromMs) {
    throw new RangeError('MT5 series query timestamps are invalid.');
  }
  if (!Number.isSafeInteger(request.limit) || request.limit < 1 || request.limit > 5000) {
    throw new RangeError('MT5 series query limit must be between 1 and 5,000.');
  }
  return invoke<Mt5SeriesQueryResponse>('query_mt5_market_data_series', { request });
}

export async function getMarketDataProviderCatalog(): Promise<MarketDataProviderCatalogEntry[]> {
  requireDesktopRuntime('market-data provider registry');
  return invoke<MarketDataProviderCatalogEntry[]>('get_market_data_provider_catalog');
}

export async function queryUnifiedMarketData(request: UnifiedMarketDataQuery): Promise<UnifiedMarketDataResult> {
  requireDesktopRuntime('unified market-data query');
  if (!request || typeof request !== 'object') throw new TypeError('A valid unified market-data query is required.');
  if (typeof request.symbol !== 'string' || !request.symbol.trim() || request.symbol.length > 64) {
    throw new TypeError('Market-data symbol must contain between 1 and 64 characters.');
  }
  if (typeof request.timeframe !== 'string' || !request.timeframe.trim() || request.timeframe.length > 8) {
    throw new TypeError('Market-data timeframe is invalid.');
  }
  if (!Number.isSafeInteger(request.fromMs) || !Number.isSafeInteger(request.toMs)
    || request.fromMs <= 0 || request.toMs < request.fromMs
    || request.toMs - request.fromMs > 366 * 24 * 60 * 60 * 1000) {
    throw new RangeError('Market-data query range is invalid or exceeds 366 days.');
  }
  if (request.afterMs !== null && (!Number.isSafeInteger(request.afterMs)
    || request.afterMs < request.fromMs - 1 || request.afterMs > request.toMs)) {
    throw new RangeError('Market-data pagination cursor is outside the requested range.');
  }
  if (request.providerId != null && (typeof request.providerId !== 'string' || !/^[a-z0-9_-]{1,48}$/i.test(request.providerId))) {
    throw new TypeError('Market-data provider identifier is invalid.');
  }
  if (!Number.isSafeInteger(request.limit) || request.limit < 1 || request.limit > 5000) {
    throw new RangeError('Market-data page size must be between 1 and 5,000.');
  }
  if (!Number.isSafeInteger(request.maxDataAgeMs) || request.maxDataAgeMs < 1000 || request.maxDataAgeMs > 7 * 24 * 60 * 60 * 1000) {
    throw new RangeError('Market-data freshness limit must be between one second and seven days.');
  }
  if (!['research', 'backtest', 'brokerContext'].includes(request.purpose)) {
    throw new TypeError('Market-data purpose is unsupported.');
  }
  if (typeof request.allowStale !== 'boolean') throw new TypeError('allowStale must be boolean.');
  return invoke<UnifiedMarketDataResult>('query_unified_market_data', { request });
}

export async function getCandleSeries(request: CandleSeriesRequest): Promise<CandleSeriesResponse> {
  requireDesktopRuntime('market data');
  return invoke<CandleSeriesResponse>('get_candle_series', { request });
}

export async function refreshMarketData(): Promise<DashboardSnapshot> {
  requireDesktopRuntime('market data');
  return invoke<DashboardSnapshot>('refresh_public_market_data');
}

export async function getMemeCoinHunterSecurityStatus(): Promise<MemeCoinHunterSecurityStatus> {
  requireDesktopRuntime('meme coin hunter');
  return invoke<MemeCoinHunterSecurityStatus>('get_meme_coin_hunter_security_status');
}

export async function saveMemeCoinHunterSecurityToken(token: string): Promise<MemeCoinHunterSecurityStatus> {
  requireDesktopRuntime('meme coin hunter');
  if (typeof token !== 'string' || token.length < 32 || token.length > 4096
    || token.trim() !== token || /[\s\u0000-\u001f]/.test(token)
    || !/^[\x21-\x7E]+$/.test(token)) {
    throw new TypeError('A valid GoPlus access token is required.');
  }
  return invoke<MemeCoinHunterSecurityStatus>('save_meme_coin_hunter_security_token', { token });
}

export async function removeMemeCoinHunterSecurityToken(): Promise<MemeCoinHunterSecurityStatus> {
  requireDesktopRuntime('meme coin hunter');
  return invoke<MemeCoinHunterSecurityStatus>('remove_meme_coin_hunter_security_token');
}

export async function runMemeCoinHunter(): Promise<MemeCoinHunterReport> {
  requireDesktopRuntime('meme coin hunter');
  return invoke<MemeCoinHunterReport>('run_meme_coin_hunter');
}

export async function getLatestMemeCoinHunterReport(): Promise<MemeCoinHunterReport | null> {
  requireDesktopRuntime('meme coin hunter');
  return invoke<MemeCoinHunterReport | null>('get_latest_meme_coin_hunter_report');
}

export async function refreshMemeScreener(): Promise<DashboardSnapshot> {
  requireDesktopRuntime('screener');
  return invoke<DashboardSnapshot>('refresh_public_screener');
}

/** Create a native Rust-owned work plan for all 364 master-list concepts. This does not dispatch agents. */
export async function createStrictAnalysisWorkPlan(request: CreateStrictWorkPlanRequest): Promise<StrictAnalysisWorkPlan> {
  requireDesktopRuntime('strict analysis');
  return invoke<StrictAnalysisWorkPlan>('create_strict_analysis_work_plan', { request });
}

/** Ask the native Rust completion barrier to validate all 364 per-concept assessments before handoff. */
export async function evaluateStrictAnalysisCompletion(request: StrictCompletionRequest): Promise<StrictCompletionResult> {
  requireDesktopRuntime('strict analysis');
  return invoke<StrictCompletionResult>('evaluate_strict_analysis_completion', { request });
}

/** Verify the durable Rust audit chain before relying on historical strict-analysis decisions. */
export async function verifyStrictAnalysisAuditChain(): Promise<StrictAuditChainStatus> {
  requireDesktopRuntime('audit history');
  return invoke<StrictAuditChainStatus>('verify_strict_analysis_audit_chain');
}

export async function placePaperOrder(request: PlaceOrderRequest) {
  requireDesktopRuntime('paper order');
  return invoke('place_paper_order', { request });
}

export async function runBacktest(request: BacktestRequest): Promise<BacktestResult> {
  requireDesktopRuntime('backtest');
  return invoke<BacktestResult>('run_backtest', { request });
}

export async function setAgentRunning(agentId: string, running: boolean) {
  requireDesktopRuntime('agent lifecycle');
  if (typeof agentId !== 'string' || agentId.trim().length === 0 || agentId.length > 128) {
    throw new TypeError('Agent identifier must be a non-empty string of at most 128 characters.');
  }
  if (typeof running !== 'boolean') throw new TypeError('Agent running state must be boolean.');
  return invoke('set_agent_running', { agentId, running });
}

export async function setKillSwitch(engaged: boolean) {
  requireDesktopRuntime('risk controls');
  if (typeof engaged !== 'boolean') throw new TypeError('Kill-switch state must be boolean.');
  return invoke<boolean>('set_kill_switch', { engaged });
}

export async function resetPaperAccount() {
  requireDesktopRuntime('account maintenance');
  return invoke<DashboardSnapshot>('reset_paper_account');
}

export async function updateRiskLimits(riskLimits: RiskLimits) {
  requireDesktopRuntime('risk settings');
  return invoke<DashboardSnapshot>('update_risk_limits', { riskLimits });
}

export async function getAuditHistory(limit = 250): Promise<AuditEvent[]> {
  requireDesktopRuntime('audit history');
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > 1000) {
    throw new RangeError('Audit history limit must be an integer between 1 and 1000.');
  }
  return invoke<AuditEvent[]>('get_audit_history', { limit });
}

export async function createResearchHypothesis(request: CreateResearchHypothesisRequest): Promise<ResearchHypothesis> {
  requireDesktopRuntime('research workspace');
  return invoke<ResearchHypothesis>('create_research_hypothesis', { request });
}

export async function listResearchHypotheses(limit = 50): Promise<ResearchHypothesis[]> {
  requireDesktopRuntime('research workspace');
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > 200) {
    throw new RangeError('Research hypothesis limit must be an integer between 1 and 200.');
  }
  return invoke<ResearchHypothesis[]>('list_research_hypotheses', { limit });
}

export async function invalidateResearchHypothesis(hypothesisId: string, reason: string, expectedRevision: number): Promise<ResearchHypothesis> {
  requireDesktopRuntime('research workspace');
  if (typeof hypothesisId !== 'string' || hypothesisId.trim().length === 0 || hypothesisId.length > 64) {
    throw new TypeError('Hypothesis identifier must be a non-empty string of at most 64 characters.');
  }
  if (typeof reason !== 'string' || reason.trim().length === 0 || reason.length > 2_000) {
    throw new TypeError('Invalidation reason must contain between 1 and 2,000 characters.');
  }
  if (!Number.isSafeInteger(expectedRevision) || expectedRevision < 1) {
    throw new RangeError('Expected hypothesis revision must be a positive safe integer.');
  }
  return invoke<ResearchHypothesis>('invalidate_research_hypothesis', { hypothesisId, reason, expectedRevision });
}

export async function executeResearchWorkflow(request: ResearchWorkflowRequest): Promise<ResearchRunCardRecord> {
  requireDesktopRuntime('research workspace');
  return invoke<ResearchRunCardRecord>('execute_research_workflow', { request });
}

export async function listResearchRunCards(limit = 40): Promise<ResearchRunCardRecord[]> {
  requireDesktopRuntime('research workspace');
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > 100) {
    throw new RangeError('Research run-card limit must be an integer between 1 and 100.');
  }
  return invoke<ResearchRunCardRecord[]>('list_research_run_cards', { limit });
}

export async function getResearchRunCard(runId: string): Promise<ResearchRunCardRecord> {
  requireDesktopRuntime('research workspace');
  if (typeof runId !== 'string' || runId.trim().length === 0 || runId.length > 64) {
    throw new TypeError('Run-card identifier must be a non-empty string of at most 64 characters.');
  }
  return invoke<ResearchRunCardRecord>('get_research_run_card', { runId });
}

export async function verifyResearchWorkspace(): Promise<ResearchIntegrityStatus> {
  requireDesktopRuntime('research workspace');
  return invoke<ResearchIntegrityStatus>('verify_research_workspace');
}
