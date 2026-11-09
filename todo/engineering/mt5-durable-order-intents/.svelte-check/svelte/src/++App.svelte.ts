///<reference types="svelte" />
;
import { onMount } from 'svelte';
import {
    Activity, ArrowDownRight, ArrowUpRight, Bell, Bot, CandlestickChart, CheckCircle2, Copy as CopyIcon,
    ChevronDown, Clock3, Command, Cpu, Database, ExternalLink, Filter, FlaskConical,
    Download, Gauge, LayoutDashboard, LockKeyhole, Menu, MoreHorizontal, Network, Pause, Play,
    Plus, RefreshCw, Search, Settings, ShieldAlert, ShieldCheck, SlidersHorizontal,
    Sparkles, Square, Terminal, TrendingDown, TrendingUp, Wallet, X, Zap, BookOpen, Layers3,
  } from 'lucide-svelte';
import {
    getAuditHistory, getCandleSeries, getDashboardState, isDesktopRuntime, placePaperOrder, refreshMarketData, refreshMemeScreener, resetPaperAccount,
    runBacktest, setAgentRunning, setKillSwitch, updateRiskLimits,
    createResearchHypothesis, invalidateResearchHypothesis, executeResearchWorkflow, getResearchRunCard, listResearchHypotheses,
    listResearchRunCards, verifyResearchWorkspace,
  } from './lib/adapters/tauri/bridge';
import type { Agent, AuditEvent, BacktestRequest, BacktestResult, Candle, CandleSeriesRequest, ValidatedCandleSeriesSnapshot, DashboardSnapshot, PaperOrder, Quote, ScreenerToken, ResearchHypothesis, ResearchIntegrityStatus, ResearchRunCardRecord } from './lib/types';
import { auditEventsToCsv, backtestReportToJson } from './lib/infrastructure/serialization/exports.js';
import { TECHNICAL_ANALYSIS_CATALOG, TECHNICAL_ANALYSIS_CATEGORIES, TECHNICAL_ANALYSIS_MASTER_LIST_VERSION } from './lib/domain/technical-analysis/catalog.js';
import { createTechnicalAnalysisReport } from './lib/application/technical-analysis-service.js';
import AgentRuntimePanel from './lib/components/AgentRuntimePanel.svelte';
import Mt5BridgePanel from './lib/components/Mt5BridgePanel.svelte';
import MarketDataPlatformPanel from './lib/components/MarketDataPlatformPanel.svelte';
import MemeCoinHunterPanel from './lib/components/MemeCoinHunterPanel.svelte';
import { getMaximumVolume, normalizedVolumeHeight, summarizeLastCandle } from './lib/presentation/dashboard/dashboard-model.js';
import { buildCandleSeriesRequest, getAvailableCandleSeries, isOrderChartAligned, validateCandleSeriesResponse } from './lib/domain/market-data/market-data-model.js';
import { isFreshMarketTimestamp } from './lib/domain/market-data/market-data-policy.js';
function $$render() {

  
  
  
  
  
  
  
  
  
  
  
  
  
  

  type NavItem = { label: string; icon: any; group?: string; locked?: boolean };
  const navItems: NavItem[] = [
    { label: 'Home', icon: LayoutDashboard },
    { label: 'Trading', icon: CandlestickChart },
    { label: 'Strategy Builder', icon: SlidersHorizontal },
    { label: 'AI Agents', icon: Bot },
    { label: 'Backtesting', icon: FlaskConical },
    { label: 'Research Lab', icon: BookOpen },
    { label: 'Paper Trading', icon: ShieldCheck },
    { label: 'Live Trading', icon: LockKeyhole, locked: true },
    { label: 'MT5 Bridge', icon: Network },
    { label: 'Market Data', icon: Database },
    { label: 'Technical Analysis', icon: Layers3 },
    { label: 'Meme Coin Hunter', icon: Sparkles },
    { label: 'Portfolio', icon: Wallet },
    { label: 'Audit Log', icon: Clock3 },
    { label: 'Tools & MCP', icon: Network },
    { label: 'Settings', icon: Settings },
  ];

  let snapshot: DashboardSnapshot | null = null;
  let activeNav = 'Home';
  let selectedSymbol = '';
  let activeCandleRequest: CandleSeriesRequest = { symbol: 'BTCUSDT', interval: '5m' };
  let chartSeriesResponse: ValidatedCandleSeriesSnapshot | null = null;
  let chartSeriesBusy = false;
  let chartSeriesError = '';
  let chartRequestEpoch = 0;
  let orderSide: 'BUY' | 'SELL' = 'BUY';
  let quantityText: string | number = '';
  let clockNow = Date.now();
  let orderType = 'Market';
  let fastPeriod = 9;
  let slowPeriod = 21;
  let initialCapital: string | number = '10000';
  let backtestResult: BacktestResult | null = null;
  let orderResult: PaperOrder | null = null;
  let auditEvents: AuditEvent[] = [];
  let auditBusy = false;
  let auditQuery = '';
  let riskForm = { maxOrderNotional: 500, maxPositionNotional: 2000, maxDailyLoss: 250, feeBps: 10, slippageBps: 5 };
  let searchQuery = '';
  let screenerQuery = '';
  let selectedScreenerToken: ScreenerToken | null = null;
  let screenerBusy = false;
  let refreshBusy = false;
  let orderBusy = false;
  let backtestBusy = false;
  let riskBusy = false;
  let loading = true;
  let errorMessage = '';
  let noticeMessage = '';
  let showRiskChecks = true;
  let showMobileNav = false;
  let screenerSort: 'momentum' | 'risk' | 'change' = 'momentum';
  let analysisQuery = '';
  let analysisCategory = 'All categories';
  let analysisGroup = 'All groups';
  let analysisCoverage = 'All statuses';
  let analysisLimit = 60;
  let researchHypothesisTitle = '';
  let researchHypothesisThesis = '';
  let researchInvalidationReason = '';
  let researchInvalidationBusy = false;
  let researchHypothesisCreateBusy = false;
  let researchHypotheses: ResearchHypothesis[] = [];
  let researchRunCards: ResearchRunCardRecord[] = [];
  let selectedResearchHypothesisId = '';
  let selectedResearchRun: ResearchRunCardRecord | null = null;
  let researchGoal = 'Evaluate whether a 9/21 SMA crossover has repeatable edge after costs and out-of-sample review.';
  let researchFastPeriod = 9;
  let researchSlowPeriod = 21;
  let researchInitialCapital: string | number = '10000';
  let researchMaxDrawdownPct = 15;
  let researchMinimumPositiveOosRatio = 0.5;
  let researchMinimumSharpeRatio = 0;
  let researchStrategyTournament = true;
  let researchIntegrity: ResearchIntegrityStatus | null = null;
  let researchWorkspaceBusy = false;
  let researchWorkflowBusy = false;
  let researchErrorMessage = '';
  let researchNoticeMessage = '';

  const desktop = isDesktopRuntime();

  onMount(() => {
    if (!desktop) return;
    void loadSnapshot();
    const clock = window.setInterval(() => { clockNow = Date.now(); }, 1000);
    return () => window.clearInterval(clock);
  });

  async function navigate(label: string) {
    activeNav = label;
    showMobileNav = false;
    if (label === 'Audit Log') await loadAuditHistory();
    if (label === 'Research Lab') await loadResearchWorkspace();
  }

  async function loadAuditHistory() {
    auditBusy = true;
    errorMessage = '';
    try { auditEvents = await getAuditHistory(1000); }
    catch (error) { errorMessage = toMessage(error); }
    finally { auditBusy = false; }
  }

  async function loadResearchWorkspace() {
    researchWorkspaceBusy = true;
    researchErrorMessage = '';
    try {
      const [hypotheses, runs, integrity] = await Promise.all([
        listResearchHypotheses(200),
        listResearchRunCards(40),
        verifyResearchWorkspace(),
      ]);
      researchHypotheses = hypotheses;
      researchRunCards = runs;
      researchIntegrity = integrity;
      if (!hypotheses.some((hypothesis) => hypothesis.id === selectedResearchHypothesisId && hypothesis.status !== 'INVALIDATED')) {
        selectedResearchHypothesisId = hypotheses.find((hypothesis) => hypothesis.status !== 'INVALIDATED')?.id ?? '';
      }
      if (!selectedResearchRun || !runs.some((record) => record.card.id === selectedResearchRun?.card.id)) {
        selectedResearchRun = runs[0] ?? null;
      }
      if (!integrity.valid) throw new Error('Research workspace integrity verification did not pass.');
    } catch (error) {
      researchErrorMessage = toMessage(error);
    } finally {
      researchWorkspaceBusy = false;
    }
  }

  async function createResearchHypothesisRecord() {
    if (researchHypothesisCreateBusy) return;
    researchHypothesisCreateBusy = true;
    researchErrorMessage = '';
    researchNoticeMessage = '';
    try {
      const hypothesis = await createResearchHypothesis({
        title: researchHypothesisTitle,
        thesis: researchHypothesisThesis,
        symbol: 'BTCUSDT',
        interval: '5m',
      });
      researchHypothesisTitle = '';
      researchHypothesisThesis = '';
      selectedResearchHypothesisId = hypothesis.id;
      researchNoticeMessage = `Hypothesis ${hypothesis.id.slice(0, 8)} created at revision ${hypothesis.revision}.`;
      await loadResearchWorkspace();
    } catch (error) {
      researchErrorMessage = toMessage(error);
    } finally {
      researchHypothesisCreateBusy = false;
    }
  }

  async function invalidateSelectedResearchHypothesis() {
    if (researchInvalidationBusy) return;
    const selected = researchHypotheses.find((hypothesis) =>
      hypothesis.id === selectedResearchHypothesisId && hypothesis.status !== 'INVALIDATED'
    );
    const reason = researchInvalidationReason.trim();
    if (!selected || !reason) {
      researchErrorMessage = 'Select an active hypothesis and provide an invalidation reason.';
      return;
    }
    researchInvalidationBusy = true;
    researchErrorMessage = '';
    researchNoticeMessage = '';
    try {
      const invalidated = await invalidateResearchHypothesis(selected.id, reason, selected.revision);
      researchInvalidationReason = '';
      researchNoticeMessage = `Hypothesis ${invalidated.id.slice(0, 8)} invalidated at revision ${invalidated.revision}. Existing run cards remain immutable.`;
      await loadResearchWorkspace();
    } catch (error) {
      researchErrorMessage = toMessage(error);
    } finally {
      researchInvalidationBusy = false;
    }
  }

  async function selectResearchRun(runId: string) {
    researchErrorMessage = '';
    try {
      selectedResearchRun = await getResearchRunCard(runId);
    } catch (error) {
      researchErrorMessage = toMessage(error);
    }
  }

  async function executeResearchRun() {
    if (researchWorkflowBusy) return;
    researchWorkflowBusy = true;
    researchErrorMessage = '';
    researchNoticeMessage = '';
    try {
      const requested = { symbol: 'BTCUSDT' as const, interval: '5m' as const };
      const rawSeries = await getCandleSeries(requested);
      const validatedSeries = validateCandleSeriesResponse(rawSeries, requested);
      if (validatedSeries.candles.length < researchSlowPeriod + 3) {
        throw new Error(`At least ${researchSlowPeriod + 3} validated candles are required for this SMA configuration.`);
      }
      const selectedHypothesis = researchHypotheses.find((hypothesis) => hypothesis.id === selectedResearchHypothesisId && hypothesis.status !== 'INVALIDATED');
      const record = await executeResearchWorkflow({
        goal: researchGoal,
        hypothesisId: selectedHypothesis?.id ?? null,
        symbol: requested.symbol,
        interval: requested.interval,
        fastPeriod: Number(researchFastPeriod),
        slowPeriod: Number(researchSlowPeriod),
        initialCapital: Number(researchInitialCapital),
        maxDrawdownPct: Number(researchMaxDrawdownPct),
        minimumPositiveOosRatio: Number(researchMinimumPositiveOosRatio),
        minimumSharpeRatio: Number(researchMinimumSharpeRatio),
        strategyTournament: researchStrategyTournament,
      });
      selectedResearchRun = record;
      researchNoticeMessage = `Research run ${record.card.id.slice(0, 8)} completed. Gate: ${record.card.decision.status}. This does not grant order authority.`;
      await loadResearchWorkspace();
      selectedResearchRun = record;
    } catch (error) {
      researchErrorMessage = toMessage(error);
    } finally {
      researchWorkflowBusy = false;
    }
  }

  async function loadSnapshot() {
    loading = true;
    try {
      snapshot = await getDashboardState();
      synchronizeSelectedSymbol(snapshot);
      hydrateRiskForm();
      errorMessage = '';
      const availableSeries = getAvailableCandleSeries(snapshot.marketDataCapabilities);
      const preferredSeries = availableSeries.find((series) => series.symbol === activeCandleRequest.symbol && series.interval === activeCandleRequest.interval) ?? availableSeries[0];
      if (preferredSeries) await loadChartSeries({ symbol: preferredSeries.symbol, interval: preferredSeries.interval });
      else {
        chartSeriesResponse = null;
        chartSeriesError = 'No validated candle series is currently available. Refresh market data or inspect provider status.';
      }
    } catch (error) {
      errorMessage = toMessage(error);
      chartSeriesResponse = null;
      chartSeriesError = toMessage(error);
    } finally {
      loading = false;
    }
  }

  async function loadChartSeries(request: CandleSeriesRequest): Promise<ValidatedCandleSeriesSnapshot | null> {
    const requestEpoch = ++chartRequestEpoch;
    chartSeriesBusy = true;
    chartSeriesResponse = null;
    chartSeriesError = '';
    try {
      const typedRequest = buildCandleSeriesRequest(snapshot?.marketDataCapabilities, request.symbol, request.interval);
      const response = await getCandleSeries(typedRequest);
      const validatedResponse = validateCandleSeriesResponse(response, typedRequest);
      if (requestEpoch !== chartRequestEpoch) return null;
      chartSeriesResponse = validatedResponse;
      activeCandleRequest = validatedResponse.request;
      return validatedResponse;
    } catch (error) {
      if (requestEpoch === chartRequestEpoch) chartSeriesError = toMessage(error);
      return null;
    } finally {
      if (requestEpoch === chartRequestEpoch) chartSeriesBusy = false;
    }
  }

  async function openCandleSeries(series: { symbol: string; interval: CandleSeriesRequest['interval'] }) {
    const response = await loadChartSeries({ symbol: series.symbol, interval: series.interval });
    if (response) {
      activeNav = 'Trading';
      noticeMessage = `Loaded explicit candle series ${response.request.symbol} ${response.request.interval}.`;
    }
  }

  function hydrateRiskForm() {
    if (!snapshot) return;
    riskForm = {
      maxOrderNotional: snapshot.riskLimits.maxOrderNotional,
      maxPositionNotional: snapshot.riskLimits.maxPositionNotional,
      maxDailyLoss: snapshot.riskLimits.maxDailyLoss,
      feeBps: snapshot.riskLimits.feeBps,
      slippageBps: snapshot.riskLimits.slippageBps,
    };
  }

  async function refreshSnapshot() {
    refreshBusy = true;
    errorMessage = '';
    try {
      snapshot = await refreshMarketData();
      synchronizeSelectedSymbol(snapshot);
      const availableSeries = getAvailableCandleSeries(snapshot.marketDataCapabilities);
      const preferredSeries = availableSeries.find((series) => series.symbol === activeCandleRequest.symbol && series.interval === activeCandleRequest.interval) ?? availableSeries[0];
      if (preferredSeries) await loadChartSeries({ symbol: preferredSeries.symbol, interval: preferredSeries.interval });
      else {
        chartSeriesResponse = null;
        chartSeriesError = 'No validated candle series is available after refresh; the previous chart was cleared.';
      }
      noticeMessage = 'Market data refresh completed. Review provider status, selected candle series, and timestamp before acting.';
      setTimeout(() => { noticeMessage = ''; }, 5000);
    } catch (error) {
      errorMessage = toMessage(error);
    } finally {
      refreshBusy = false;
    }
  }

  async function refreshScreener() {
    screenerBusy = true;
    errorMessage = '';
    try {
      snapshot = await refreshMemeScreener();
      synchronizeSelectedSymbol(snapshot);
      selectedScreenerToken = snapshot.screener[0] ?? null;
      noticeMessage = `Screener refresh finished: ${snapshot.screenerProviderStatus}. Review source time and risk evidence; this feed is promotion-biased.`;
    } catch (error) { errorMessage = toMessage(error); }
    finally { screenerBusy = false; }
  }

  async function copyTokenAddress() {
    const address = selectedScreenerToken?.tokenAddress;
    if (!address) return;
    try {
      await navigator.clipboard.writeText(address);
      noticeMessage = 'Token address copied.';
    } catch {
      errorMessage = 'Clipboard permission is unavailable. Select and copy the address manually.';
    }
  }

  async function submitOrder() {
    if (!snapshot) return;
    if (!currentQuoteFresh || !Number.isFinite(Number(quantityText)) || Number(quantityText) <= 0) {
      errorMessage = 'A positive quantity and a fresh validated quote for the selected instrument are required.';
      return;
    }
    if (snapshot.killSwitchEngaged) {
      errorMessage = 'The paper-trading kill switch is engaged. Review risk controls before submitting.';
      return;
    }
    orderBusy = true;
    errorMessage = '';
    noticeMessage = '';
    try {
      const quantity = Number(quantityText);
      const result = await placePaperOrder({
        symbol: selectedSymbol,
        side: orderSide,
        quantity,
        clientOrderId: crypto.randomUUID(),
      }) as PaperOrder;
      orderResult = result;
      showRiskChecks = true;
      snapshot = await getDashboardState();
      synchronizeSelectedSymbol(snapshot);
      noticeMessage = result.status.startsWith('FILLED') ? `${orderSide} order simulated and filled in the paper account.` : `Order was not filled: ${result.status}. See the risk checks below.`;
    } catch (error) {
      errorMessage = toMessage(error);
    } finally {
      orderBusy = false;
    }
  }

  async function toggleAgent(agent: Agent) {
    errorMessage = '';
    try {
      const running = !agent.status.startsWith('RUNNING');
      await setAgentRunning(agent.id, running);
      snapshot = await getDashboardState();
      synchronizeSelectedSymbol(snapshot);
      noticeMessage = `${agent.name} ${running ? 'started in observe-only mode' : 'stopped'}.`;
    } catch (error) { errorMessage = toMessage(error); }
  }

  async function toggleKillSwitch() {
    if (!snapshot) return;
    errorMessage = '';
    try {
      await setKillSwitch(!snapshot.killSwitchEngaged);
      snapshot = await getDashboardState();
      synchronizeSelectedSymbol(snapshot);
      noticeMessage = snapshot.killSwitchEngaged ? 'Kill switch engaged. New paper order intents are blocked.' : 'Kill switch released. Orders remain subject to risk checks.';
    } catch (error) { errorMessage = toMessage(error); }
  }

  function downloadTextFile(filename: string, content: string, mimeType: string) {
    const blob = new Blob([content], { type: `${mimeType};charset=utf-8` });
    const objectUrl = URL.createObjectURL(blob);
    const anchor = document.createElement('a');
    anchor.href = objectUrl;
    anchor.download = filename;
    anchor.style.display = 'none';
    document.body.append(anchor);
    anchor.click();
    anchor.remove();
    window.setTimeout(() => URL.revokeObjectURL(objectUrl), 1000);
  }

  function exportAuditCsv() {
    if (filteredAuditEvents.length === 0) return;
    const stamp = new Date().toISOString().replace(/[:.]/g, '-');
    downloadTextFile(`atos-audit-${stamp}.csv`, auditEventsToCsv(filteredAuditEvents), 'text/csv');
    noticeMessage = `Exported ${filteredAuditEvents.length} visible audit event(s). The current search filter was applied.`;
  }

  function exportBacktestJson() {
    if (!backtestResult) return;
    const stamp = new Date().toISOString().replace(/[:.]/g, '-');
    downloadTextFile(`atos-backtest-${backtestResult.id}-${stamp}.json`, backtestReportToJson(backtestResult), 'application/json');
    noticeMessage = 'Backtest report v2 exported with parameters, metrics, caveats, and the canonical input-data fingerprint.';
  }

  async function executeBacktest() {
    backtestBusy = true;
    errorMessage = '';
    try {
      const request: BacktestRequest = {
        symbol: activeCandleRequest.symbol, fastPeriod: Number(fastPeriod), slowPeriod: Number(slowPeriod),
        initialCapital: Number(initialCapital), feeBps: snapshot?.riskLimits.feeBps ?? 10,
        slippageBps: snapshot?.riskLimits.slippageBps ?? 5,
      };
      backtestResult = await runBacktest(request);
      noticeMessage = 'Backtest completed. Read assumptions and warnings before interpreting the metrics.';
    } catch (error) { errorMessage = toMessage(error); }
    finally { backtestBusy = false; }
  }

  async function saveRiskSettings() {
    riskBusy = true;
    errorMessage = '';
    try {
      const riskLimits = {
        maxOrderNotional: Number(riskForm.maxOrderNotional),
        maxPositionNotional: Number(riskForm.maxPositionNotional),
        maxDailyLoss: Number(riskForm.maxDailyLoss),
        feeBps: Number(riskForm.feeBps),
        slippageBps: Number(riskForm.slippageBps),
      };
      snapshot = await updateRiskLimits(riskLimits);
      synchronizeSelectedSymbol(snapshot);
      noticeMessage = 'Risk policy saved to the local Rust engine.';
    } catch (error) { errorMessage = toMessage(error); }
    finally { riskBusy = false; }
  }

  async function resetAccount() {
    if (!confirm('Reset the paper account? Positions and order history will be cleared. No real funds are affected.')) return;
    errorMessage = '';
    try {
      snapshot = await resetPaperAccount();
      synchronizeSelectedSymbol(snapshot);
      orderResult = null;
      backtestResult = null;
      hydrateRiskForm();
      noticeMessage = 'Paper account reset.';
    } catch (error) { errorMessage = toMessage(error); }
  }

  function toMessage(error: unknown): string {
    if (error instanceof Error) return error.message;
    return String(error);
  }

  function money(value: number, compact = false): string {
    if (!Number.isFinite(value)) return '—';
    return new Intl.NumberFormat('en-US', {
      style: 'currency', currency: 'USD', maximumFractionDigits: Math.abs(value) < 1 ? 8 : 2,
      notation: compact && Math.abs(value) >= 100_000 ? 'compact' : 'standard',
    }).format(value);
  }
  function number(value: number, digits = 2): string {
    return Number.isFinite(value) ? new Intl.NumberFormat('en-US', { maximumFractionDigits: digits }).format(value) : '—';
  }
  function signedPct(value: number): string { return `${value > 0 ? '+' : ''}${number(value)}%`; }
  function medianValue(values: number[]): number {
    if (values.length === 0) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    return sorted.length % 2 === 1 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2;
  }
  function compactNumber(value: number): string {
    return Number.isFinite(value) ? new Intl.NumberFormat('en-US', { notation: 'compact', maximumFractionDigits: 1 }).format(value) : '—';
  }
  function timeLabel(value: number): string {
    if (!value) return '—';
    return new Date(value).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
  function isoDateTimeLabel(value: number): string {
    const date = new Date(value);
    return Number.isFinite(value) && Number.isFinite(date.getTime()) ? date.toISOString() : '—';
  }
  function ageLabel(value: number, now = clockNow): string {
    if (!Number.isSafeInteger(value) || value <= 0 || !Number.isSafeInteger(now) || now <= 0) return 'unknown';
    if (value > now) return 'timestamp in future';
    const seconds = Math.floor((now - value) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    return `${Math.floor(seconds / 3600)}h ago`;
  }
  function shortSymbol(symbol: string): string { return symbol.replace('USDT', '/USDT'); }
  // Widened to string so TypeScript does not pin the initial literal during reactive initialization.
  function compareScreenerTokens(a: ScreenerToken, b: ScreenerToken, sort: string): number {
    if (sort === 'risk') return b.riskScore - a.riskScore;
    if (sort === 'change') return b.change24hPct - a.change24hPct;
    return b.momentumScore - a.momentumScore;
  }
  function quoteFor(symbol: string): Quote | undefined { return snapshot?.quotes.find((quote) => quote.symbol === symbol); }
  function quoteIsFresh(quote: Quote | undefined, now = clockNow): boolean {
    const providerObservedAtMs = snapshot?.providerUpdatedAtMs;
    const quoteObservedAtMs = quote?.updatedAtMs;
    const quoteRetrievedAtMs = quote?.retrievedAtMs;
    const providerObservationFresh = isFreshMarketTimestamp(providerObservedAtMs, now);
    const quoteObservationFresh = isFreshMarketTimestamp(quoteObservedAtMs, now);
    const quoteReceiptFresh = isFreshMarketTimestamp(quoteRetrievedAtMs, now);
    return !!quote && !!snapshot && snapshot.providerStatus.startsWith('CONNECTED')
      && providerObservationFresh && quoteObservationFresh && quoteReceiptFresh
      && quote.isLive === true && Number.isFinite(quote.price) && quote.price > 0;
  }
  function positionMarkFresh(symbol: string, now = clockNow): boolean {
    return quoteIsFresh(quoteFor(symbol), now);
  }
  function synchronizeSelectedSymbol(next: DashboardSnapshot): void {
    if (!next.quotes.some((quote) => quote.symbol === selectedSymbol)) selectedSymbol = next.quotes[0]?.symbol ?? '';
  }
  let  activeChartQuote = __sveltets_2_invalidate(() => quoteFor(activeCandleRequest.symbol));
  let  activeChartQuoteFresh = __sveltets_2_invalidate(() => positionMarkFresh(activeCandleRequest.symbol, clockNow));
  function selectSymbol(symbol: string) { selectedSymbol = symbol; activeNav = 'Trading'; if (!isOrderChartAligned(symbol, activeCandleRequest)) noticeMessage = `${symbol} has quote data but no loaded candle series; the chart remains ${activeCandleRequest.symbol} ${activeCandleRequest.interval}. The paper order will use the ${symbol} quote only.`; }

  let  currentQuote = __sveltets_2_invalidate(() => selectedSymbol ? quoteFor(selectedSymbol) : undefined);
  let  currentQuoteFresh = __sveltets_2_invalidate(() => quoteIsFresh(currentQuote, clockNow));
  let  portfolioMarksFresh = __sveltets_2_invalidate(() => Number.isFinite(clockNow) && (snapshot?.positions ?? []).every((position) => positionMarkFresh(position.symbol, clockNow)));
  let  freshQuotes = __sveltets_2_invalidate(() => (snapshot?.quotes ?? []).filter((quote) => quoteIsFresh(quote, clockNow)));
  let  filteredQuotes = __sveltets_2_invalidate(() => (snapshot?.quotes ?? []).filter((quote) => quote.symbol.toLowerCase().includes(searchQuery.toLowerCase())));
  let  filteredAuditEvents = __sveltets_2_invalidate(() => auditEvents.filter((event) => `${event.level} ${event.category} ${event.title} ${event.detail}`.toLowerCase().includes(auditQuery.toLowerCase())));
  // Screener rows are displayed as current evidence only while the provider snapshot is connected,
  // timestamp-valid, and within the same bounded freshness window as market quotes.
  let  screenerSnapshotFresh = __sveltets_2_invalidate(() => !!snapshot
    && snapshot.screenerProviderStatus.startsWith('CONNECTED')
    && isFreshMarketTimestamp(snapshot.screenerProviderUpdatedAtMs, clockNow));
  ;() => {$: if (!screenerSnapshotFresh) selectedScreenerToken = null;}
  let  sortedScreener = __sveltets_2_invalidate(() => [...(screenerSnapshotFresh ? (snapshot?.screener ?? []) : [])]
    .filter((token) => `${token.symbol} ${token.name} ${token.chain}`.toLowerCase().includes(screenerQuery.toLowerCase()))
    .sort((a, b) => compareScreenerTokens(a, b, screenerSort)));
  let  candles = __sveltets_2_invalidate(() => chartSeriesResponse?.candles ?? []);
  let  chartMin = __sveltets_2_invalidate(() => candles.length ? Math.min(...candles.map((c) => c.low)) : 0);
  let  chartMax = __sveltets_2_invalidate(() => candles.length ? Math.max(...candles.map((c) => c.high)) : 1);
  let  chartRange = __sveltets_2_invalidate(() => Math.max(chartMax - chartMin, 0.000001));
  let  orderQuantity = __sveltets_2_invalidate(() => Number(quantityText));
  let  estimatedNotional = __sveltets_2_invalidate(() => currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? currentQuote.price * orderQuantity : 0);
  let  estimatedFee = __sveltets_2_invalidate(() => estimatedNotional * ((snapshot?.riskLimits.feeBps ?? 10) / 10000));
  let  closePath = __sveltets_2_invalidate(() => candles.map((c, i) => `${i === 0 ? 'M' : 'L'} ${chartX(i)} ${priceY(c.close)}`).join(' '));
  let  maFastPath = __sveltets_2_invalidate(() => movingAveragePath(9));
  let  maSlowPath = __sveltets_2_invalidate(() => movingAveragePath(21));
  let  providerIsFresh = __sveltets_2_invalidate(() => !!snapshot && snapshot.providerStatus.startsWith('CONNECTED') && isFreshMarketTimestamp(snapshot.providerUpdatedAtMs, clockNow));
  let  technicalEnvelope = __sveltets_2_invalidate(() => createTechnicalAnalysisReport({
    validatedSeries: chartSeriesResponse,
    requestedSymbol: activeCandleRequest.symbol,
    requestedTimeframe: activeCandleRequest.interval,
  }));
  let  technicalReadings = __sveltets_2_invalidate(() => technicalEnvelope.indicatorReadings);
  let  lastCandleSummary = __sveltets_2_invalidate(() => summarizeLastCandle(candles));
  let  orderChartAligned = __sveltets_2_invalidate(() => isOrderChartAligned(selectedSymbol, activeCandleRequest));
  let  activeSeriesCapability = __sveltets_2_invalidate(() => snapshot?.marketDataCapabilities.candleSeries.find((series) => series.symbol === activeCandleRequest.symbol && series.interval === activeCandleRequest.interval));
  let  maxCandleVolume = __sveltets_2_invalidate(() => getMaximumVolume(candles));
  let  analysisGroups = __sveltets_2_invalidate(() => ['All groups', ...new Set(TECHNICAL_ANALYSIS_CATALOG.filter((item) => analysisCategory === 'All categories' || item.category === analysisCategory).map((item) => item.group))]);
  let  analysisStatusCounts = __sveltets_2_invalidate(() => ({
    calculated: TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Calculated now').length,
    workflow: TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Workflow available').length,
    catalogOnly: TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Catalog only').length,
  }));
  let  filteredAnalysisCatalog = __sveltets_2_invalidate(() => TECHNICAL_ANALYSIS_CATALOG.filter((item) => {
    const search = analysisQuery.trim().toLowerCase();
    const matchesQuery = !search || `${item.name} ${item.group} ${item.category} ${item.kind}`.toLowerCase().includes(search);
    const matchesCategory = analysisCategory === 'All categories' || item.category === analysisCategory;
    const matchesGroup = analysisGroup === 'All groups' || item.group === analysisGroup;
    const matchesCoverage = analysisCoverage === 'All statuses' || item.availability === analysisCoverage;
    return matchesQuery && matchesCategory && matchesGroup && matchesCoverage;
  }));
  let  visibleAnalysisCatalog = __sveltets_2_invalidate(() => filteredAnalysisCatalog.slice(0, analysisLimit));

  function chartX(index: number): number { return 12 + (index / Math.max(candles.length - 1, 1)) * 976; }
  function priceY(price: number): number { return 18 + ((chartMax - price) / chartRange) * 250; }
  function movingAveragePath(period: number): string {
    const parts: string[] = [];
    for (let i = period - 1; i < candles.length; i++) {
      const mean = candles.slice(i - period + 1, i + 1).reduce((total, candle) => total + candle.close, 0) / period;
      parts.push(`${parts.length === 0 ? 'M' : 'L'} ${chartX(i)} ${priceY(mean)}`);
    }
    return parts.join(' ');
  }

  function backtestCurvePath(series: 'equity' | 'benchmark'): string {
    const points = backtestResult?.equityCurve ?? [];
    if (points.length < 2) return '';
    let min = Number.POSITIVE_INFINITY;
    let max = Number.NEGATIVE_INFINITY;
    for (const point of points) {
      min = Math.min(min, point.equity, point.benchmark);
      max = Math.max(max, point.equity, point.benchmark);
    }
    const range = Math.max(max - min, Number.EPSILON);
    const stride = Math.max(1, Math.ceil(points.length / 180));
    const sampled = points.filter((_, index) => index % stride === 0 || index === points.length - 1);
    return sampled.map((point, index) => {
      const x = 8 + (index / Math.max(sampled.length - 1, 1)) * 284;
      const y = 68 - ((point[series] - min) / range) * 54;
      return `${index === 0 ? 'M' : 'L'} ${x} ${y}`;
    }).join(' ');
  }

;
async () => {

 { svelteHTML.createElement("svelte:head", {});
   { svelteHTML.createElement("title", {});    }
   { svelteHTML.createElement("meta", {    "name":`description`,"content":`QuantDinger AI Trading OS — market research, backtesting, and paper trading with a local risk gateway.`,});}
 }

if(!desktop){
   { svelteHTML.createElement("main", {     "class":`runtime-required-screen`,"role":`alert`,"aria-labelledby":`runtime-required-title`,});
     { svelteHTML.createElement("div", { "class":`runtime-required-mark`,}); { const $$_elohyeKkcoL2C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL2C({ target: __sveltets_2_any(), props: {  "size":28,}});} }
     { svelteHTML.createElement("p", { "class":`runtime-required-eyebrow`,});   }
     { svelteHTML.createElement("h1", { "id":`runtime-required-title`,});    }
     { svelteHTML.createElement("p", { "class":`runtime-required-copy`,});                          }
     { svelteHTML.createElement("div", { "class":`runtime-required-details`,});
       { svelteHTML.createElement("div", {}); { const $$_kcehCdleihS3C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS3C({ target: __sveltets_2_any(), props: {  "size":16,}});} { svelteHTML.createElement("span", {});           } }
       { svelteHTML.createElement("div", {}); { const $$_esabataD3C = __sveltets_2_ensureComponent(Database); new $$_esabataD3C({ target: __sveltets_2_any(), props: {  "size":16,}});} { svelteHTML.createElement("span", {});             } }
       { svelteHTML.createElement("div", {}); { const $$_ytivitcA3C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA3C({ target: __sveltets_2_any(), props: {  "size":16,}});} { svelteHTML.createElement("span", {});              } }
     }
     { svelteHTML.createElement("p", { "class":`runtime-required-footnote`,});          }
   }
}else{
 { svelteHTML.createElement("div", { "class":`app-shell`,});
   { svelteHTML.createElement("aside", {   "class":`sidebar`,});showMobileNav;
     { svelteHTML.createElement("div", { "class":`brand`,});
       { svelteHTML.createElement("div", { "class":`brand-mark`,}); { const $$_ytivitcA4C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA4C({ target: __sveltets_2_any(), props: {    "size":22,"strokeWidth":2.4,}});} }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`brand-name`,});  } { svelteHTML.createElement("div", { "class":`brand-sub`,});   } }
       { svelteHTML.createElement("button", {     "class":`icon-button mobile-close`,"aria-label":`Close navigation`,"onclick":() => showMobileNav = false,}); { const $$_X4C = __sveltets_2_ensureComponent(X); new $$_X4C({ target: __sveltets_2_any(), props: {  "size":17,}});} }
     }

     { svelteHTML.createElement("div", { "class":`nav-caption`,});  }
     { svelteHTML.createElement("nav", {   "class":`nav-list`,"aria-label":`Primary navigation`,});
        for(let item of __sveltets_2_ensureArray(navItems)){
         { svelteHTML.createElement("button", {     "class":`nav-item`,"onclick":() => { void navigate(item.label); },});activeNav === item.label;
           { const $$_tnenopmoc_etlevs4C = __sveltets_2_ensureComponent(item.icon); new $$_tnenopmoc_etlevs4C({ target: __sveltets_2_any(), props: {      "size":17,"strokeWidth":1.8,}});}
           { svelteHTML.createElement("span", {});item.label; }
          if(item.locked){ { const $$_elohyeKkcoL4C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL4C({ target: __sveltets_2_any(), props: {    "size":13,"class":`nav-lock`,}});}}
          if(item.label === 'Meme Coin Hunter'){ { svelteHTML.createElement("span", { "class":`new-tag`,});  }}
         }
      }
     }

     { svelteHTML.createElement("div", { "class":`sidebar-bottom`,});
       { svelteHTML.createElement("div", { "class":`quick-panel`,});
         { svelteHTML.createElement("div", { "class":`panel-label`,});  }
         { svelteHTML.createElement("button", { "onclick":() => { void navigate('Strategy Builder'); },}); { const $$_sulP5C = __sveltets_2_ensureComponent(Plus); new $$_sulP5C({ target: __sveltets_2_any(), props: {  "size":14,}});}   }
         { svelteHTML.createElement("button", { "onclick":() => { void navigate('Backtesting'); },}); { const $$_lacinoCksalF5C = __sveltets_2_ensureComponent(FlaskConical); new $$_lacinoCksalF5C({ target: __sveltets_2_any(), props: {  "size":14,}});}   }
         { svelteHTML.createElement("button", { "onclick":() => { void navigate('AI Agents'); },}); { const $$_yalP5C = __sveltets_2_ensureComponent(Play); new $$_yalP5C({ target: __sveltets_2_any(), props: {  "size":14,}});}   }
         { svelteHTML.createElement("button", { "onclick":() => { void navigate('Tools & MCP'); },}); { const $$_lanimreT5C = __sveltets_2_ensureComponent(Terminal); new $$_lanimreT5C({ target: __sveltets_2_any(), props: {  "size":14,}});}   }
       }
       { svelteHTML.createElement("div", { "class":`system-state`,}); { svelteHTML.createElement("span", { "class":`pulse-dot`,}); } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});  } { svelteHTML.createElement("small", {});   } } { svelteHTML.createElement("span", { "class":`system-version`,});  } }
     }
   }

   { svelteHTML.createElement("div", { "class":`app-main`,});
     { svelteHTML.createElement("header", { "class":`topbar`,});
       { svelteHTML.createElement("button", {     "class":`icon-button mobile-menu`,"aria-label":`Open navigation`,"onclick":() => showMobileNav = true,}); { const $$_uneM4C = __sveltets_2_ensureComponent(Menu); new $$_uneM4C({ target: __sveltets_2_any(), props: {  "size":19,}});} }
       { svelteHTML.createElement("div", { "class":`global-search`,}); { const $$_hcraeS4C = __sveltets_2_ensureComponent(Search); new $$_hcraeS4C({ target: __sveltets_2_any(), props: {  "size":16,}});} { svelteHTML.createElement("input", {      "bind:value":searchQuery,"placeholder":`Search symbol, strategy, agent...`,"aria-label":`Search market symbols`,});/*Ωignore_startΩ*/() => searchQuery = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("kbd", {}); { const $$_dnammoC5C = __sveltets_2_ensureComponent(Command); new $$_dnammoC5C({ target: __sveltets_2_any(), props: {  "size":11,}});}  } }
       { svelteHTML.createElement("div", { "class":`topbar-spacer`,}); }
       { svelteHTML.createElement("div", { "class":`runtime-chip`,}); { svelteHTML.createElement("span", { "class":`pulse-dot`,}); }  }
       { svelteHTML.createElement("button", {       "class":`icon-button`,"aria-label":`Latest events`,"title":`Latest events`,"onclick":() => { void navigate('Audit Log'); },}); { const $$_lleB4C = __sveltets_2_ensureComponent(Bell); new $$_lleB4C({ target: __sveltets_2_any(), props: {  "size":17,}});} }
       { svelteHTML.createElement("button", {         "class":`icon-button`,"aria-label":`Refresh market data`,"title":`Refresh market data`,"disabled":refreshBusy,"onclick":refreshSnapshot,}); { const $$_wChserfeR4C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR4C({ target: __sveltets_2_any(), props: {    "size":17,"class":refreshBusy ? 'spin' : '',}});} }
       { svelteHTML.createElement("div", { "class":`top-divider`,}); }
       { svelteHTML.createElement("button", {     "class":`icon-button`,"aria-label":`Settings`,"onclick":() => { void navigate('Settings'); },}); { const $$_sgnitteS4C = __sveltets_2_ensureComponent(Settings); new $$_sgnitteS4C({ target: __sveltets_2_any(), props: {  "size":17,}});} }
       { svelteHTML.createElement("div", { "class":`profile-chip`,}); { svelteHTML.createElement("div", { "class":`avatar`,});  } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});  } { svelteHTML.createElement("small", {});  } } { const $$_nwoDnorvehC4C = __sveltets_2_ensureComponent(ChevronDown); new $$_nwoDnorvehC4C({ target: __sveltets_2_any(), props: {  "size":13,}});} }
     }

    if(errorMessage && snapshot){
       { svelteHTML.createElement("div", {   "class":`flash flash-error`,"role":`alert`,}); { const $$_trelAdleihS3C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS3C({ target: __sveltets_2_any(), props: {  "size":17,}});} { svelteHTML.createElement("span", {});errorMessage; } { svelteHTML.createElement("button", {   "aria-label":`Dismiss error`,"onclick":() => errorMessage = '',}); { const $$_X4C = __sveltets_2_ensureComponent(X); new $$_X4C({ target: __sveltets_2_any(), props: {  "size":15,}});} } }
    }
    if(noticeMessage){
       { svelteHTML.createElement("div", {   "class":`flash flash-info`,"role":`status`,}); { const $$_2elcriCkcehC3C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC3C({ target: __sveltets_2_any(), props: {  "size":17,}});} { svelteHTML.createElement("span", {});noticeMessage; } { svelteHTML.createElement("button", {   "aria-label":`Dismiss notice`,"onclick":() => noticeMessage = '',}); { const $$_X4C = __sveltets_2_ensureComponent(X); new $$_X4C({ target: __sveltets_2_any(), props: {  "size":15,}});} } }
    }

    if(loading){
       { svelteHTML.createElement("div", { "class":`loading-state`,}); { const $$_wChserfeR3C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR3C({ target: __sveltets_2_any(), props: {    "class":`spin`,"size":24,}});} { svelteHTML.createElement("p", {});     } }
    } else if (snapshot){
       { svelteHTML.createElement("div", { "class":`content-grid`,});
         { svelteHTML.createElement("main", { "class":`content-main`,});
          if(activeNav === 'Home'){
             { svelteHTML.createElement("div", { "class":`page-heading`,});
               { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});    } { svelteHTML.createElement("h1", {});   } { svelteHTML.createElement("p", {});         } }
               { svelteHTML.createElement("div", { "class":`heading-actions`,}); { svelteHTML.createElement("span", { "class":`date-label`,}); { const $$_3kcolC7C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC7C({ target: __sveltets_2_any(), props: {  "size":13,}});} new Date().toLocaleDateString([], { weekday: 'short', month: 'short', day: 'numeric' }); } { svelteHTML.createElement("button", {     "class":`button-secondary`,"onclick":refreshSnapshot,"disabled":refreshBusy,}); { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {  "size":14,}});}   } }
             }
             { svelteHTML.createElement("div", { "class":`overview-strip`,});
               { svelteHTML.createElement("span", { "class":`mode-pill`,}); { svelteHTML.createElement("span", { "class":`small-dot`,}); }snapshot.mode; }
               { svelteHTML.createElement("span", {    "class":`provider-pill`,});providerIsFresh;!providerIsFresh; { svelteHTML.createElement("span", { "class":`small-dot`,}); }snapshot.providerStatus; }
               { svelteHTML.createElement("span", { "class":`muted-note`,}); ageLabel(snapshot.providerUpdatedAtMs, clockNow);  snapshot.providerSource; }
               { svelteHTML.createElement("button", {    "class":`kill-button`,"onclick":toggleKillSwitch,});snapshot.killSwitchEngaged; { const $$_trelAdleihS6C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS6C({ target: __sveltets_2_any(), props: {  "size":14,}});} snapshot.killSwitchEngaged ? 'KILL SWITCH ON' : 'Engage kill switch'; }
             }

             { svelteHTML.createElement("section", { "class":`metric-grid`,});
               { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});   } { const $$_tellaW7C = __sveltets_2_ensureComponent(Wallet); new $$_tellaW7C({ target: __sveltets_2_any(), props: {  "size":15,}});} } { svelteHTML.createElement("div", { "class":`metric-value`,});portfolioMarksFresh ? money(snapshot.equity) : 'Unavailable'; } { svelteHTML.createElement("div", { "class":`metric-foot portfolio-foot`,}); { svelteHTML.createElement("span", {  });snapshot.dayChangePct >= 0;snapshot.dayChangePct < 0;portfolioMarksFresh ? `${signedPct(snapshot.dayChangePct)} (UTC day)` : 'Latest portfolio marks are stale'; } { svelteHTML.createElement("span", {}); money(snapshot.realizedPnl);   portfolioMarksFresh ? money(snapshot.unrealizedPnl) : 'Unavailable'; } } }
               { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});   } { const $$_ytivitcA7C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA7C({ target: __sveltets_2_any(), props: {  "size":15,}});} } { svelteHTML.createElement("div", { "class":`metric-value`,});snapshot.closedTradeCount > 0 && snapshot.winRatePct !== null ? number(snapshot.winRatePct) : '—';if(snapshot.closedTradeCount > 0 && snapshot.winRatePct !== null){ { svelteHTML.createElement("small", {});  }} } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", { "class":`neutral-value`,});snapshot.closedTradeCount; } { svelteHTML.createElement("span", {});snapshot.closedTradeCount === 1 ? 'closed paper trade' : 'closed paper trades'; } } }
               { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  } { const $$_nwoDgnidnerT7C = __sveltets_2_ensureComponent(TrendingDown); new $$_nwoDgnidnerT7C({ target: __sveltets_2_any(), props: {  "size":15,}});} } { svelteHTML.createElement("div", {  "class":`metric-value`,});snapshot.maxDrawdownPct < 0;number(snapshot.maxDrawdownPct); { svelteHTML.createElement("small", {});  } } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});  } } }
               { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  } { const $$_toB7C = __sveltets_2_ensureComponent(Bot); new $$_toB7C({ target: __sveltets_2_any(), props: {  "size":15,}});} } { svelteHTML.createElement("div", { "class":`metric-value`,});snapshot.agents.filter((agent) => agent.status.startsWith('RUNNING')).length; { svelteHTML.createElement("small", {});  snapshot.agents.length; } } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", { "class":`status-text`,});snapshot.agents.length > 0 ? 'Agent state reported by runtime' : 'No agent sessions configured'; } } }
             }

             { svelteHTML.createElement("section", { "class":`chart-order-grid`,});
               { svelteHTML.createElement("article", { "class":`panel chart-panel`,});
                 { svelteHTML.createElement("div", { "class":`panel-heading chart-heading`,}); { svelteHTML.createElement("div", { "class":`asset-heading`,}); { svelteHTML.createElement("div", { "class":`asset-icon`,});  } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});activeCandleRequest.symbol.replace('USDT', ' / USDT'); } { svelteHTML.createElement("p", {});   activeCandleRequest.interval;  } } } { svelteHTML.createElement("div", { "class":`chart-tools`,}); { svelteHTML.createElement("button", {     "class":`timeframe unavailable-timeframe`,"title":`Only 5m candles are loaded`,"onclick":() => noticeMessage = '1m candles are not loaded yet; the chart remains on the verified 5m dataset.',});  } { svelteHTML.createElement("span", {   "class":`timeframe active`,"title":`Loaded candle interval`,});  } { svelteHTML.createElement("button", {     "class":`timeframe unavailable-timeframe`,"title":`Only 5m candles are loaded`,"onclick":() => noticeMessage = '15m candles are not loaded yet; the chart remains on the verified 5m dataset.',});  } { svelteHTML.createElement("button", {     "class":`timeframe unavailable-timeframe`,"title":`Only 5m candles are loaded`,"onclick":() => noticeMessage = '1h candles are not loaded yet; the chart remains on the verified 5m dataset.',});  } { svelteHTML.createElement("button", {     "class":`timeframe unavailable-timeframe`,"title":`Only 5m candles are loaded`,"onclick":() => noticeMessage = '4h candles are not loaded yet; the chart remains on the verified 5m dataset.',});  } { svelteHTML.createElement("button", {     "class":`timeframe unavailable-timeframe`,"title":`Only 5m candles are loaded`,"onclick":() => noticeMessage = '1D candles are not loaded yet; the chart remains on the verified 5m dataset.',});  } { svelteHTML.createElement("button", {       "class":`icon-button tiny`,"title":`Technical indicators`,"aria-label":`Technical indicators`,"onclick":() => { void navigate('Technical Analysis'); },}); { const $$_ytivitcA9C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA9C({ target: __sveltets_2_any(), props: {  "size":14,}});} } { svelteHTML.createElement("button", {       "class":`icon-button tiny`,"title":`Market data`,"aria-label":`Market data`,"onclick":() => { void navigate('Market Data'); },}); { const $$_esabataD9C = __sveltets_2_ensureComponent(Database); new $$_esabataD9C({ target: __sveltets_2_any(), props: {  "size":14,}});} } } }
                 { svelteHTML.createElement("div", { "class":`chart-price-row`,}); { svelteHTML.createElement("div", {});if(activeChartQuote && activeChartQuoteFresh){ { svelteHTML.createElement("strong", {});money(activeChartQuote.price); } { svelteHTML.createElement("span", {  });activeChartQuote.change24hPct >= 0;activeChartQuote.change24hPct < 0;signedPct(activeChartQuote.change24hPct); }}else{ { svelteHTML.createElement("strong", {});  } { svelteHTML.createElement("span", { "class":`muted-note`,});activeChartQuote ? 'Stale quote · refresh required' : 'No validated quote'; }} } { svelteHTML.createElement("div", { "class":`chart-source`,}); { svelteHTML.createElement("span", {  "class":`small-dot`,});chartSeriesResponse?.isLive; }chartSeriesResponse?.source || chartSeriesError || 'No candle-series source'; } }
                if(lastCandleSummary){
                   { svelteHTML.createElement("div", {   "class":`chart-ohlc-row`,"aria-label":`Latest validated ${activeCandleRequest.interval} candle`,});
                     { svelteHTML.createElement("span", {});  { svelteHTML.createElement("strong", {});number(lastCandleSummary.open, 2); } } { svelteHTML.createElement("span", {});  { svelteHTML.createElement("strong", {});number(lastCandleSummary.high, 2); } } { svelteHTML.createElement("span", {});  { svelteHTML.createElement("strong", {});number(lastCandleSummary.low, 2); } } { svelteHTML.createElement("span", {});  { svelteHTML.createElement("strong", {  });lastCandleSummary.bullish;!lastCandleSummary.bullish;number(lastCandleSummary.close, 2); } } { svelteHTML.createElement("span", {});  { svelteHTML.createElement("strong", {});number(lastCandleSummary.volume, 2); } } { svelteHTML.createElement("time", {});new Date(lastCandleSummary.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }); }
                   }
                }else{
                   { svelteHTML.createElement("div", { "class":`chart-ohlc-row chart-ohlc-empty`,});    }
                }
                 { svelteHTML.createElement("div", { "class":`chart-canvas`,});
                  if(candles.length > 0){
                   { svelteHTML.createElement("svg", {       "viewBox":`0 0 1000 310`,"preserveAspectRatio":`none`,"role":`img`,"aria-label":`${activeCandleRequest.symbol} candle close chart. Price data source shown above.`,});
                      for(let tick of __sveltets_2_ensureArray(([0, 1, 2, 3, 4]))){
                       { svelteHTML.createElement("line", {          "x1":`0`,"x2":`1000`,"y1":20 + tick * 62,"y2":20 + tick * 62,"class":`chart-gridline`,});}
                    }
                      for(let tick of __sveltets_2_ensureArray(([0, 1, 2, 3, 4, 5, 6]))){
                       { svelteHTML.createElement("line", {          "x1":tick * 166,"x2":tick * 166,"y1":`0`,"y2":`300`,"class":`chart-gridline vertical`,});}
                    }
                     { svelteHTML.createElement("path", {    "d":closePath,"class":`price-area`,});}
                     { svelteHTML.createElement("path", {    "d":maSlowPath,"class":`ma-line ma-slow`,});}
                     { svelteHTML.createElement("path", {    "d":maFastPath,"class":`ma-line ma-fast`,});}
                       for(let candle of __sveltets_2_ensureArray(candles)){let i = 1;
                       { svelteHTML.createElement("line", {            "x1":chartX(i),"x2":chartX(i),"y1":priceY(candle.high),"y2":priceY(candle.low),"class":`candle-wick`,});candle.close >= candle.open;candle.close < candle.open;}
                       { svelteHTML.createElement("rect", {            "x":chartX(i) - Math.max(1.3, 340 / candles.length / 2),"y":Math.min(priceY(candle.open), priceY(candle.close)),"width":Math.max(2.2, 340 / candles.length),"height":Math.max(1.3, Math.abs(priceY(candle.open) - priceY(candle.close))),"class":`candle-body`,});candle.close >= candle.open;candle.close < candle.open;}
                    }
                   }
                   { svelteHTML.createElement("div", { "class":`chart-y-labels`,}); { svelteHTML.createElement("span", {});money(chartMax, true); } { svelteHTML.createElement("span", {});money((chartMax + chartMin) / 2, true); } { svelteHTML.createElement("span", {});money(chartMin, true); } }
                  }else{
                     { svelteHTML.createElement("div", { "class":`empty-chart-state`,});  { const $$_esabataD8C = __sveltets_2_ensureComponent(Database); new $$_esabataD8C({ target: __sveltets_2_any(), props: { "size":21,}});} { svelteHTML.createElement("strong", {});chartSeriesBusy ? 'Loading requested candle series…' : 'Candle series unavailable'; } { svelteHTML.createElement("span", {});chartSeriesError || activeSeriesCapability?.note || 'No candle data is loaded for this exact symbol and interval.'; } }
                  }
                 }
                 { svelteHTML.createElement("div", {   "class":`chart-volume-panel`,"aria-label":`Volume bars from loaded candle data`,});
                   { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("small", {});   activeCandleRequest.interval; } }
                   { svelteHTML.createElement("svg", {       "viewBox":`0 0 1000 40`,"preserveAspectRatio":`none`,"role":`img`,"aria-label":`${activeCandleRequest.interval} candle volume histogram`,});
                       for(let candle of __sveltets_2_ensureArray(candles)){let i = 1;
                       { svelteHTML.createElement("rect", {          "x":chartX(i) - Math.max(1.3, 350 / candles.length / 2),"y":38 - normalizedVolumeHeight(candle.volume, maxCandleVolume, 34, 2),"width":Math.max(2.1, 350 / candles.length),"height":normalizedVolumeHeight(candle.volume, maxCandleVolume, 34, 2),});candle.close >= candle.open;candle.close < candle.open;}
                    }
                   }
                 }
                 { svelteHTML.createElement("div", { "class":`chart-footer`,}); { svelteHTML.createElement("div", { "class":`chart-legend`,}); { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`legend-dot fast`,}); }   } { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`legend-dot slow`,}); }   } { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`legend-dot price`,}); }  } } { svelteHTML.createElement("div", { "class":`chart-times`,});if(candles.length > 0){ { svelteHTML.createElement("span", {});new Date(candles[0].time).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }); } { svelteHTML.createElement("span", {});new Date(candles[Math.floor((candles.length - 1) / 2)].time).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }); } { svelteHTML.createElement("span", {});new Date(candles[candles.length - 1].time).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' }); }}else{ { svelteHTML.createElement("span", {});   }} } }
               }

               { svelteHTML.createElement("article", { "class":`panel order-panel`,});
                 { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});       } } { svelteHTML.createElement("span", { "class":`paper-badge`,});  } }
                if(snapshot.killSwitchEngaged){ { svelteHTML.createElement("div", { "class":`risk-banner`,}); { const $$_trelAdleihS7C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS7C({ target: __sveltets_2_any(), props: {  "size":15,}});} { svelteHTML.createElement("span", {});       } }}
                 { svelteHTML.createElement("div", { "class":`side-switch`,}); { svelteHTML.createElement("button", {   "onclick":() => orderSide = 'BUY',});orderSide === 'BUY';   } { svelteHTML.createElement("button", {   "onclick":() => orderSide = 'SELL',});orderSide === 'SELL';   } }
                 { svelteHTML.createElement("label", {   "class":`field-label`,"for":`order-instrument`,});  }
                 { svelteHTML.createElement("select", {         "id":`order-instrument`,"class":`field-control`,"bind:value":selectedSymbol,"aria-label":`Order instrument`,"disabled":snapshot.quotes.length === 0,});/*Ωignore_startΩ*/() => selectedSymbol = __sveltets_2_any(null);/*Ωignore_endΩ*/
                  if(snapshot.quotes.length === 0){ { svelteHTML.createElement("option", {"value":"",});    }}
                    for(let quote of __sveltets_2_ensureArray(snapshot.quotes)){ { svelteHTML.createElement("option", { "value":quote.symbol,});quote.symbol; }}
                 }
                if(!orderChartAligned){ { svelteHTML.createElement("div", {   "class":`chart-order-mismatch`,"role":`status`,});  { const $$_trelAdleihS7C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS7C({ target: __sveltets_2_any(), props: { "size":13,}});} { svelteHTML.createElement("span", {}); activeCandleRequest.symbol; activeCandleRequest.interval; selectedSymbol;               selectedSymbol;   } }}
                 { svelteHTML.createElement("span", { "class":`field-label`,});  } { svelteHTML.createElement("div", {     "class":`order-type-control`,"role":`group`,"aria-label":`Order type`,}); { svelteHTML.createElement("button", {   "onclick":() => orderType = 'Market',});orderType === 'Market';  } { svelteHTML.createElement("button", {   "class":`disabled-type`,"title":`Limit orders are not supported by this broker`,});  { const $$_elohyeKkcoL8C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL8C({ target: __sveltets_2_any(), props: {  "size":11,}});} } { svelteHTML.createElement("button", {   "class":`disabled-type`,"title":`Stop orders are not supported by this broker`,});  { const $$_elohyeKkcoL8C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL8C({ target: __sveltets_2_any(), props: {  "size":11,}});} } }
                 { svelteHTML.createElement("label", {   "class":`field-label`,"for":`quantity`,});  } { svelteHTML.createElement("div", { "class":`input-with-suffix`,}); { svelteHTML.createElement("input", {          "id":`quantity`,"type":`number`,"min":`0.00000001`,"step":`any`,"bind:value":quantityText,});/*Ωignore_startΩ*/() => quantityText = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("span", {});selectedSymbol.replace('USDT', ''); } }
                 { svelteHTML.createElement("div", { "class":`quantity-presets`,}); { svelteHTML.createElement("button", {   "disabled":!currentQuoteFresh,"onclick":() => { if (currentQuoteFresh && currentQuote) quantityText = String((snapshot?.riskLimits.maxOrderNotional ?? 500) * 0.25 / currentQuote.price); },});  } { svelteHTML.createElement("button", {   "disabled":!currentQuoteFresh,"onclick":() => { if (currentQuoteFresh && currentQuote) quantityText = String((snapshot?.riskLimits.maxOrderNotional ?? 500) * 0.5 / currentQuote.price); },});  } { svelteHTML.createElement("button", {   "disabled":!currentQuoteFresh,"onclick":() => { if (currentQuoteFresh && currentQuote) quantityText = String((snapshot?.riskLimits.maxOrderNotional ?? 500) * 0.75 / currentQuote.price); },});  } { svelteHTML.createElement("button", {   "disabled":!currentQuoteFresh,"onclick":() => { if (currentQuoteFresh && currentQuote) quantityText = String((snapshot?.riskLimits.maxOrderNotional ?? 500) / currentQuote.price); },});  } }
                 { svelteHTML.createElement("div", { "class":`estimate-list`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});currentQuoteFresh && currentQuote ? money(currentQuote.price) : 'Unavailable'; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});currentQuoteFresh && currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? money(estimatedNotional) : 'Unavailable'; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});currentQuoteFresh && currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? money(estimatedFee) : 'Unavailable'; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});money(snapshot.riskLimits.maxOrderNotional); } } }
                 { svelteHTML.createElement("button", {      "class":`submit-order`,"disabled":orderBusy || snapshot.killSwitchEngaged || !desktop || orderType !== 'Market' || !currentQuoteFresh || !Number.isFinite(orderQuantity) || orderQuantity <= 0,"onclick":submitOrder,});orderSide === 'SELL';if(orderBusy){ { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {    "size":15,"class":`spin`,}});} }else{ { const $$_paZ7C = __sveltets_2_ensureComponent(Zap); new $$_paZ7C({ target: __sveltets_2_any(), props: {  "size":15,}});}  orderSide; } }
                 { svelteHTML.createElement("div", { "class":`execution-hint`,}); { const $$_kcehCdleihS7C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS7C({ target: __sveltets_2_any(), props: {  "size":13,}});}         }
                if(orderResult && showRiskChecks){ { svelteHTML.createElement("div", { "class":`risk-check-list`,}); { svelteHTML.createElement("div", { "class":`risk-check-title`,});   orderResult.status; }  for(let check of __sveltets_2_ensureArray(orderResult.riskChecks)){ { svelteHTML.createElement("div", { "class":`risk-check`,}); { svelteHTML.createElement("span", {  });check.passed;!check.passed;if(check.passed){ { const $$_2elcriCkcehC9C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC9C({ target: __sveltets_2_any(), props: {  "size":13,}});}}else{ { const $$_X9C = __sveltets_2_ensureComponent(X); new $$_X9C({ target: __sveltets_2_any(), props: {  "size":13,}});}} } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});check.rule; } { svelteHTML.createElement("small", {});check.detail; } } }} }}
               }
             }

             { svelteHTML.createElement("section", { "class":`panel screener-panel`,});
               { svelteHTML.createElement("div", { "class":`panel-heading screener-heading`,}); { svelteHTML.createElement("div", { "class":`section-title-with-icon`,}); { svelteHTML.createElement("div", { "class":`section-icon purple`,}); { const $$_selkrapS8C = __sveltets_2_ensureComponent(Sparkles); new $$_selkrapS8C({ target: __sveltets_2_any(), props: {  "size":16,}});} } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});snapshot.screenerProviderStatus;  ageLabel(snapshot.screenerProviderUpdatedAtMs, clockNow);     } } } { svelteHTML.createElement("button", {   "class":`button-secondary compact`,"onclick":() => { void navigate('Meme Coin Hunter'); },});     { const $$_kniLlanretxE7C = __sveltets_2_ensureComponent(ExternalLink); new $$_kniLlanretxE7C({ target: __sveltets_2_any(), props: {  "size":13,}});} } }
               { svelteHTML.createElement("div", { "class":`provider-risk-disclosure`,}); { const $$_trelAdleihS6C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS6C({ target: __sveltets_2_any(), props: {  "size":14,}});} { svelteHTML.createElement("span", {});                  } }
               { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table screener-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {}); } } } { svelteHTML.createElement("tbody", {});
                if(sortedScreener.length === 0){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", { "colspan":10,}); { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_esabataD11C = __sveltets_2_ensureComponent(Database); new $$_esabataD11C({ target: __sveltets_2_any(), props: { "size":22,}});} { svelteHTML.createElement("strong", {});screenerSnapshotFresh ? 'No validated screener observations' : 'Screener snapshot stale or disconnected'; } { svelteHTML.createElement("span", {});screenerSnapshotFresh ? 'Refresh the public feed to load provider-backed token observations. No sample rows are substituted.' : 'Stored rows are retained for audit/history only and are not presented as current evidence. Refresh the public feed and verify its timestamp before relying on screener values.'; } } } }}
                   for(let token of __sveltets_2_ensureArray((sortedScreener.slice(0, 5)))){let i = 1; { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", { "class":`rank-cell`,});i + 1; } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("div", { "class":`coin-cell`,}); { svelteHTML.createElement("div", { "class":`coin-logo coin-${token.symbol.toLowerCase()}`,});token.symbol.slice(0, 1); } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});token.symbol; } { svelteHTML.createElement("small", {});token.name; } } } } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", { "class":`chain-tag`,});token.chain; } } { svelteHTML.createElement("td", {});money(token.price); } { svelteHTML.createElement("td", { "class":`positive`,});signedPct(token.change24hPct); } { svelteHTML.createElement("td", {});money(token.volume24h, true); } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("div", { "class":`score-cell`,}); { svelteHTML.createElement("strong", {});token.momentumScore; } { svelteHTML.createElement("div", { "class":`score-track`,}); { svelteHTML.createElement("span", { "style":`width:${token.momentumScore}%`,}); } } } } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", {     "class":`risk-score`,});token.riskScore < 40;token.riskScore >= 40 && token.riskScore < 65;token.riskScore >= 65;token.riskScore; } } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", {  "class":`signal-tag`,});token.riskScore >= 55;token.riskScore >= 55 ? 'High risk' : token.status; } } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("button", {     "class":`table-action`,"title":`Show ${token.symbol} evidence`,"onclick":() => { void navigate('Meme Coin Hunter'); },});  } } }}
               } } }
               { svelteHTML.createElement("div", { "class":`panel-bottom-note`,}); { svelteHTML.createElement("span", {}); { svelteHTML.createElement("span", { "class":`small-dot amber-dot`,}); }         } { svelteHTML.createElement("button", {   "class":`text-action`,"onclick":() => { void navigate('Meme Coin Hunter'); },});    { const $$_thgiRpUworrA7C = __sveltets_2_ensureComponent(ArrowUpRight); new $$_thgiRpUworrA7C({ target: __sveltets_2_any(), props: {  "size":13,}});} } }
             }

             { svelteHTML.createElement("section", { "class":`bottom-grid`,});
               { svelteHTML.createElement("article", { "class":`panel mini-table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});       } } { svelteHTML.createElement("button", {   "class":`icon-button tiny`,"onclick":() => { void navigate('Market Data'); },}); { const $$_latnoziroHeroM8C = __sveltets_2_ensureComponent(MoreHorizontal); new $$_latnoziroHeroM8C({ target: __sveltets_2_any(), props: {  "size":16,}});} } } { svelteHTML.createElement("div", { "class":`mini-rows`,});if(freshQuotes.length === 0){ { svelteHTML.createElement("div", { "class":`empty-small`,});  { const $$_esabataD8C = __sveltets_2_ensureComponent(Database); new $$_esabataD8C({ target: __sveltets_2_any(), props: { "size":18,}});} { svelteHTML.createElement("span", {});     } }}else{  for(let quote of __sveltets_2_ensureArray(([...freshQuotes].sort((a, b) => b.change24hPct - a.change24hPct).slice(0, 5)))){ { svelteHTML.createElement("button", {   "class":`mini-row`,"onclick":() => selectSymbol(quote.symbol),}); { svelteHTML.createElement("span", { "class":`mini-coin`,});quote.symbol.slice(0, 1); } { svelteHTML.createElement("span", { "class":`mini-row-name`,}); { svelteHTML.createElement("strong", {});quote.symbol; } { svelteHTML.createElement("small", {});money(quote.price); } } { svelteHTML.createElement("span", {  });quote.change24hPct >= 0;quote.change24hPct < 0;signedPct(quote.change24hPct); } }}} } }
               { svelteHTML.createElement("article", { "class":`panel mini-table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});     } } { svelteHTML.createElement("button", {   "class":`text-action`,"onclick":() => { void navigate('Paper Trading'); },});  } }if(snapshot.orders.length){ { svelteHTML.createElement("div", { "class":`trade-list`,});  for(let order of __sveltets_2_ensureArray((snapshot.orders.slice(0, 5)))){ { svelteHTML.createElement("div", { "class":`trade-row`,}); { svelteHTML.createElement("span", {    "class":`trade-side`,});order.side === 'BUY';order.side === 'SELL';order.side; } { svelteHTML.createElement("span", {}); { svelteHTML.createElement("strong", {});order.symbol; } { svelteHTML.createElement("small", {});timeLabel(order.createdAtMs);  order.status; } } { svelteHTML.createElement("span", { "class":`trade-value`,});money(order.notional); } }} }}else{ { svelteHTML.createElement("div", { "class":`empty-small`,}); { const $$_ytivitcA7C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA7C({ target: __sveltets_2_any(), props: {  "size":18,}});} { svelteHTML.createElement("span", {});    } }} }
               { svelteHTML.createElement("article", { "class":`panel mini-table-panel performance-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});backtestResult ? `${backtestResult.strategyName} · historical simulation` : 'No backtest curve loaded'; } } { const $$_eguaG7C = __sveltets_2_ensureComponent(Gauge); new $$_eguaG7C({ target: __sveltets_2_any(), props: {    "size":16,"class":`muted-icon`,}});} }
                if(backtestResult && backtestResult.equityCurve.length > 1){
                   { svelteHTML.createElement("div", { "class":`strategy-performance-value`,}); { svelteHTML.createElement("strong", {  });backtestResult.totalReturnPct >= 0;backtestResult.totalReturnPct < 0;signedPct(backtestResult.totalReturnPct); } { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});number(backtestResult.sharpeRatio); } { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", { });backtestResult.maxDrawdownPct < 0;signedPct(backtestResult.maxDrawdownPct); } { svelteHTML.createElement("span", {});  } }
                   { svelteHTML.createElement("div", { "class":`strategy-plot`,}); { svelteHTML.createElement("svg", {       "viewBox":`0 0 300 80`,"preserveAspectRatio":`none`,"role":`img`,"aria-label":`Backtest strategy equity compared with buy and hold benchmark`,});  { svelteHTML.createElement("line", {       "x1":`0`,"y1":`14`,"x2":`300`,"y2":`14`,});}  { svelteHTML.createElement("line", {       "x1":`0`,"y1":`40`,"x2":`300`,"y2":`40`,});}  { svelteHTML.createElement("line", {       "x1":`0`,"y1":`66`,"x2":`300`,"y2":`66`,});}  { svelteHTML.createElement("path", {   "d":backtestCurvePath('benchmark'),"class":`benchmark-curve`,});}  { svelteHTML.createElement("path", {   "d":backtestCurvePath('equity'),"class":`strategy-curve`,});} } { svelteHTML.createElement("div", { "class":`strategy-legend`,}); { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`legend-dot fast`,}); }   } { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`legend-dot slow`,}); }    } } }
                }else{
                   { svelteHTML.createElement("div", { "class":`strategy-empty`,});  { const $$_lacinoCksalF7C = __sveltets_2_ensureComponent(FlaskConical); new $$_lacinoCksalF7C({ target: __sveltets_2_any(), props: { "size":21,}});} { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("span", {});           } { svelteHTML.createElement("button", {   "class":`text-action`,"onclick":() => { void navigate('Backtesting'); },});    { const $$_thgiRpUworrA8C = __sveltets_2_ensureComponent(ArrowUpRight); new $$_thgiRpUworrA8C({ target: __sveltets_2_any(), props: { "size":12,}});} } }
                }
               }
             }

          } else if (activeNav === 'Trading' || activeNav === 'Paper Trading'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});   } { svelteHTML.createElement("h1", {});activeNav === 'Trading' ? 'Trading terminal' : 'Paper trading'; } { svelteHTML.createElement("p", {});               } } { svelteHTML.createElement("span", { "class":`mode-pill`,}); { svelteHTML.createElement("span", { "class":`small-dot`,}); }   } }
             { svelteHTML.createElement("div", { "class":`notice-banner`,}); { const $$_kcehCdleihS5C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS5C({ target: __sveltets_2_any(), props: {  "size":18,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});     } { svelteHTML.createElement("p", {});                  } } }
             { svelteHTML.createElement("section", { "class":`chart-order-grid full-trading-grid`,}); { svelteHTML.createElement("article", { "class":`panel chart-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   activeCandleRequest.symbol;  activeCandleRequest.interval; } { svelteHTML.createElement("p", {});chartSeriesResponse?.source || chartSeriesError || snapshot.providerSource; } } { svelteHTML.createElement("span", { "class":`paper-badge`,});providerIsFresh ? 'FRESH' : 'STALE'; } } { svelteHTML.createElement("div", { "class":`chart-price-row`,});if(activeChartQuote && activeChartQuoteFresh){ { svelteHTML.createElement("strong", {});money(activeChartQuote.price); } { svelteHTML.createElement("span", {  });activeChartQuote.change24hPct >= 0;activeChartQuote.change24hPct < 0;signedPct(activeChartQuote.change24hPct); }}else{ { svelteHTML.createElement("strong", {});  } { svelteHTML.createElement("span", { "class":`muted-note`,});activeChartQuote ? 'Stale quote · refresh required' : 'No validated quote'; }} } { svelteHTML.createElement("div", { "class":`chart-canvas large-chart`,});if(candles.length > 0){ { svelteHTML.createElement("svg", {       "viewBox":`0 0 1000 310`,"preserveAspectRatio":`none`,"role":`img`,"aria-label":`${activeCandleRequest.symbol} ${activeCandleRequest.interval} market close chart`,});  { svelteHTML.createElement("path", {   "d":closePath,"class":`price-area`,});}  { svelteHTML.createElement("path", {   "d":maSlowPath,"class":`ma-line ma-slow`,});}  { svelteHTML.createElement("path", {   "d":maFastPath,"class":`ma-line ma-fast`,});}   for(let candle of __sveltets_2_ensureArray(candles)){let i = 1;  { svelteHTML.createElement("line", {           "x1":chartX(i),"x2":chartX(i),"y1":priceY(candle.high),"y2":priceY(candle.low),"class":`candle-wick`,});candle.close >= candle.open;candle.close < candle.open;}  { svelteHTML.createElement("rect", {           "x":chartX(i) - 1.6,"y":Math.min(priceY(candle.open), priceY(candle.close)),"width":`3.2`,"height":Math.max(1.5, Math.abs(priceY(candle.open) - priceY(candle.close))),"class":`candle-body`,});candle.close >= candle.open;candle.close < candle.open;}} }}else{ { svelteHTML.createElement("div", { "class":`empty-chart-state`,});  { const $$_esabataD8C = __sveltets_2_ensureComponent(Database); new $$_esabataD8C({ target: __sveltets_2_any(), props: { "size":21,}});} { svelteHTML.createElement("strong", {});chartSeriesBusy ? 'Loading requested candle series…' : 'Candle series unavailable'; } { svelteHTML.createElement("span", {});chartSeriesError || activeSeriesCapability?.note || 'No candle data is loaded for this exact symbol and interval.'; } }} } { svelteHTML.createElement("div", { "class":`chart-footer`,}); { svelteHTML.createElement("div", { "class":`chart-legend`,}); { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`legend-dot fast`,}); }   } { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`legend-dot slow`,}); }   } } { svelteHTML.createElement("span", { "class":`muted-note`,}); candles.length;      } } }
               { svelteHTML.createElement("article", { "class":`panel order-panel expanded-order`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});      } } { const $$_kcehCdleihS7C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS7C({ target: __sveltets_2_any(), props: {  "size":17,}});} } { svelteHTML.createElement("div", { "class":`side-switch`,}); { svelteHTML.createElement("button", {   "onclick":() => orderSide = 'BUY',});orderSide === 'BUY';  } { svelteHTML.createElement("button", {   "onclick":() => orderSide = 'SELL',});orderSide === 'SELL';  } } { svelteHTML.createElement("label", {   "class":`field-label`,"for":`order-instrument-2`,});  } { svelteHTML.createElement("select", {       "id":`order-instrument-2`,"class":`field-control`,"bind:value":selectedSymbol,"disabled":snapshot.quotes.length === 0,});/*Ωignore_startΩ*/() => selectedSymbol = __sveltets_2_any(null);/*Ωignore_endΩ*/if(snapshot.quotes.length === 0){ { svelteHTML.createElement("option", {"value":"",});    }}  for(let quote of __sveltets_2_ensureArray(snapshot.quotes)){ { svelteHTML.createElement("option", { "value":quote.symbol,});quote.symbol;  quoteIsFresh(quote, clockNow) ? money(quote.price) : 'quote unavailable'; }} }if(!orderChartAligned){ { svelteHTML.createElement("div", {   "class":`chart-order-mismatch`,"role":`status`,});  { const $$_trelAdleihS7C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS7C({ target: __sveltets_2_any(), props: { "size":13,}});} { svelteHTML.createElement("span", {});  activeCandleRequest.symbol; activeCandleRequest.interval; selectedSymbol;        selectedSymbol;       } }} { svelteHTML.createElement("label", {   "class":`field-label`,"for":`quantity-2`,});  }  { svelteHTML.createElement("input", {           "id":`quantity-2`,"class":`field-control`,"type":`number`,"min":`0.00000001`,"step":`any`,"bind:value":quantityText,});/*Ωignore_startΩ*/() => quantityText = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("div", { "class":`estimate-list`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});currentQuoteFresh && currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? money(estimatedNotional) : 'Unavailable'; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});currentQuoteFresh && currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? money(estimatedFee) : 'Unavailable'; } } } { svelteHTML.createElement("button", {     "class":`submit-order`,"disabled":orderBusy || snapshot.killSwitchEngaged || !desktop || !currentQuoteFresh || !Number.isFinite(orderQuantity) || orderQuantity <= 0,"onclick":submitOrder,});  { const $$_paZ7C = __sveltets_2_ensureComponent(Zap); new $$_paZ7C({ target: __sveltets_2_any(), props: { "size":15,}});}    }if(!currentQuoteFresh){ { svelteHTML.createElement("p", { "class":`last-order-label quote-blocked-note`,});                 }}if(orderResult){ { svelteHTML.createElement("p", { "class":`last-order-label`,});   { svelteHTML.createElement("strong", {});orderResult.status; } }} } }
             { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});                 } } }if(snapshot.positions.length){ { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});  for(let position of __sveltets_2_ensureArray(snapshot.positions)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {}); { svelteHTML.createElement("strong", {});position.symbol; } } { svelteHTML.createElement("td", {});number(position.quantity, 8); } { svelteHTML.createElement("td", {});money(position.averageEntry); } { svelteHTML.createElement("td", {});positionMarkFresh(position.symbol, clockNow) ? money(position.markPrice) : 'Unavailable · stale mark'; } { svelteHTML.createElement("td", {});positionMarkFresh(position.symbol, clockNow) ? money(position.notional) : 'Unavailable'; } { svelteHTML.createElement("td", {  });position.unrealizedPnl >= 0;position.unrealizedPnl < 0;positionMarkFresh(position.symbol, clockNow) ? money(position.unrealizedPnl) : 'Unavailable'; } }} } } }}else{ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_tellaW6C = __sveltets_2_ensureComponent(Wallet); new $$_tellaW6C({ target: __sveltets_2_any(), props: { "size":24,}});} { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});        } }} }
             { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});        } } }if(snapshot.orders.length){ { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});  for(let order of __sveltets_2_ensureArray(snapshot.orders)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {});timeLabel(order.createdAtMs); } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("strong", {});order.symbol; } } { svelteHTML.createElement("td", {  });order.side === 'BUY';order.side === 'SELL';order.side; } { svelteHTML.createElement("td", {});number(order.quantity, 8); } { svelteHTML.createElement("td", {});order.fillPrice ? money(order.fillPrice) : '—'; } { svelteHTML.createElement("td", {});money(order.notional); } { svelteHTML.createElement("td", {});money(order.fee); } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", {    "class":`status-tag`,});order.status.startsWith('FILLED');order.status === 'REJECTED';order.status; } } }} } } }}else{ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_ytivitcA6C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA6C({ target: __sveltets_2_any(), props: { "size":24,}});} { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});         } }} }

          } else if (activeNav === 'Strategy Builder' || activeNav === 'Backtesting'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});   } { svelteHTML.createElement("h1", {});activeNav === 'Backtesting' ? 'Backtesting laboratory' : 'Strategy builder'; } { svelteHTML.createElement("p", {});             } } { svelteHTML.createElement("span", { "class":`mode-pill`,});  { const $$_lacinoCksalF6C = __sveltets_2_ensureComponent(FlaskConical); new $$_lacinoCksalF6C({ target: __sveltets_2_any(), props: { "size":13,}});}   } }
             { svelteHTML.createElement("div", { "class":`notice-banner`,});  { const $$_lacinoCksalF5C = __sveltets_2_ensureComponent(FlaskConical); new $$_lacinoCksalF5C({ target: __sveltets_2_any(), props: { "size":18,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});     } { svelteHTML.createElement("p", {});                                } } }
             { svelteHTML.createElement("section", { "class":`panel config-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});         } } { svelteHTML.createElement("span", { "class":`version-tag`,});   } } { svelteHTML.createElement("div", { "class":`config-grid`,}); { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`bt-symbol`,});  } { svelteHTML.createElement("select", {  "id":`bt-symbol`,"disabled":true,}); { svelteHTML.createElement("option", {});  } } { svelteHTML.createElement("small", {});        } } { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`fast`,});   }  { svelteHTML.createElement("input", {         "id":`fast`,"type":`number`,"min":`2`,"max":`100`,"bind:value":fastPeriod,});/*Ωignore_startΩ*/() => fastPeriod = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});   } } { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`slow`,});   }  { svelteHTML.createElement("input", {         "id":`slow`,"type":`number`,"min":`3`,"max":`300`,"bind:value":slowPeriod,});/*Ωignore_startΩ*/() => slowPeriod = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});    } } { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`capital`,});   }  { svelteHTML.createElement("input", {         "id":`capital`,"type":`number`,"min":`1`,"step":`100`,"bind:value":initialCapital,});/*Ωignore_startΩ*/() => initialCapital = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});    } } } { svelteHTML.createElement("div", { "class":`config-bottom`,}); { svelteHTML.createElement("div", { "class":`assumption-chips`,}); { svelteHTML.createElement("span", {}); number(snapshot.riskLimits.feeBps);  } { svelteHTML.createElement("span", {}); number(snapshot.riskLimits.slippageBps);  } { svelteHTML.createElement("span", {});candles.length;   } } { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":executeBacktest,"disabled":backtestBusy || !desktop,});if(backtestBusy){  { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {   "size":15,"class":`spin`,}});} }else{  { const $$_yalP7C = __sveltets_2_ensureComponent(Play); new $$_yalP7C({ target: __sveltets_2_any(), props: { "size":15,}});}  } } } }
            if(backtestResult){
               { svelteHTML.createElement("section", { "class":`metric-grid backtest-metrics`,}); { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  }  { const $$_pUgnidnerT7C = __sveltets_2_ensureComponent(TrendingUp); new $$_pUgnidnerT7C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", {   "class":`metric-value`,});backtestResult.totalReturnPct >= 0;backtestResult.totalReturnPct < 0;signedPct(backtestResult.totalReturnPct); } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});  money(backtestResult.finalEquity); } } } { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  }  { const $$_nwoDgnidnerT7C = __sveltets_2_ensureComponent(TrendingDown); new $$_nwoDgnidnerT7C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", { "class":`metric-value`,});number(backtestResult.maxDrawdownPct); { svelteHTML.createElement("small", {});  } } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});    } } } { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});   }  { const $$_ytivitcA7C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA7C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", { "class":`metric-value`,});backtestResult.tradeCount > 0 && backtestResult.winRatePct !== null ? number(backtestResult.winRatePct) : '—';if(backtestResult.tradeCount > 0 && backtestResult.winRatePct !== null){ { svelteHTML.createElement("small", {});  }} } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});backtestResult.tradeCount;   } } } { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  }  { const $$_eguaG7C = __sveltets_2_ensureComponent(Gauge); new $$_eguaG7C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", { "class":`metric-value`,});backtestResult.grossProfit > 0 && backtestResult.grossLoss === 0 ? '∞' : backtestResult.profitFactor == null ? '—' : number(backtestResult.profitFactor, 3); } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});     } } } }
               { svelteHTML.createElement("section", { "class":`panel result-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});backtestResult.strategyName;  backtestResult.candlesUsed;   backtestResult.dataSource; } } { svelteHTML.createElement("div", { "class":`heading-actions`,}); { svelteHTML.createElement("span", { "class":`status-good status-tag`,});  } { svelteHTML.createElement("button", {   "class":`button-secondary compact`,"onclick":exportBacktestJson,});  { const $$_daolnwoD8C = __sveltets_2_ensureComponent(Download); new $$_daolnwoD8C({ target: __sveltets_2_any(), props: { "size":13,}});}   } } } { svelteHTML.createElement("div", { "class":`result-strip`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(backtestResult.sharpeRatio, 3); } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(backtestResult.feeBps);  } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(backtestResult.slippageBps);  } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", { "class":`run-id`,});backtestResult.id.slice(0, 12);  } } } { svelteHTML.createElement("details", { "class":`dataset-provenance`,}); { svelteHTML.createElement("summary", {});     } { svelteHTML.createElement("div", { "class":`dataset-provenance-grid`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    backtestResult.datasetEncodingVersion; } { svelteHTML.createElement("code", {});backtestResult.datasetSha256; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {});isoDateTimeLabel(backtestResult.datasetStartMs);  isoDateTimeLabel(backtestResult.datasetEndMs); } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});number(backtestResult.medianCandleIntervalMs);  } } } { svelteHTML.createElement("p", {});                         } } { svelteHTML.createElement("div", { "class":`warnings-list`,}); { svelteHTML.createElement("h3", {});   }  for(let warning of __sveltets_2_ensureArray(backtestResult.warnings)){ { svelteHTML.createElement("div", {});  { const $$_trelAdleihS7C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS7C({ target: __sveltets_2_any(), props: { "size":14,}});} { svelteHTML.createElement("span", {});warning; } }} } }
               { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});                    } } { svelteHTML.createElement("span", { "class":`mode-pill`,});   } }if(backtestResult.costStressScenarios.length){ { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});   } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});  for(let scenario of __sveltets_2_ensureArray(backtestResult.costStressScenarios)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {}); { svelteHTML.createElement("strong", {});scenario.name; }if(scenario.feeCapReached || scenario.slippageCapReached){ { svelteHTML.createElement("small", { "class":`negative`,});     }} } { svelteHTML.createElement("td", {});number(scenario.feeBps, 2);  } { svelteHTML.createElement("td", {});number(scenario.slippageBps, 2);  } { svelteHTML.createElement("td", {  });scenario.totalReturnPct >= 0;scenario.totalReturnPct < 0;signedPct(scenario.totalReturnPct); } { svelteHTML.createElement("td", {  });scenario.returnDeltaPercentagePoints >= 0;scenario.returnDeltaPercentagePoints < 0;scenario.returnDeltaPercentagePoints > 0 ? '+' : '';number(scenario.returnDeltaPercentagePoints, 3);  } { svelteHTML.createElement("td", {});number(scenario.maxDrawdownPct, 3);  } { svelteHTML.createElement("td", {});scenario.tradeCount; } }} } } }}else{ { svelteHTML.createElement("div", { "class":`empty-state`,}); { svelteHTML.createElement("strong", {});     } { svelteHTML.createElement("span", {});        } }} { svelteHTML.createElement("p", { "class":`table-footnote`,});                            } }

               { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});                   } } { svelteHTML.createElement("span", { "class":`mode-pill`,});backtestResult.outOfSampleFolds.length;  } }if(backtestResult.outOfSampleFolds.length){ { svelteHTML.createElement("div", { "class":`result-strip`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});backtestResult.outOfSampleFolds.filter((fold) => fold.totalReturnPct > 0).length;  backtestResult.outOfSampleFolds.length; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});signedPct(Math.min(...backtestResult.outOfSampleFolds.map((fold) => fold.totalReturnPct))); } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});number(medianValue(backtestResult.outOfSampleFolds.map((fold) => fold.totalReturnPct)), 3);  } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});backtestResult.outOfSampleFolds[0].candlesEvaluated; } } } { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});  for(let fold of __sveltets_2_ensureArray(backtestResult.outOfSampleFolds)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {}); { svelteHTML.createElement("strong", {}); fold.foldIndex; } } { svelteHTML.createElement("td", {});isoDateTimeLabel(fold.startMs);  isoDateTimeLabel(fold.endMs); } { svelteHTML.createElement("td", {  });fold.totalReturnPct >= 0;fold.totalReturnPct < 0;signedPct(fold.totalReturnPct); } { svelteHTML.createElement("td", {});number(fold.maxDrawdownPct, 3);  } { svelteHTML.createElement("td", {});number(fold.sharpeRatio, 3); } { svelteHTML.createElement("td", {});fold.tradeCount; } }} } } } { svelteHTML.createElement("p", { "class":`table-footnote`,});                                 }}else{ { svelteHTML.createElement("div", { "class":`empty-state`,}); { svelteHTML.createElement("strong", {});      } { svelteHTML.createElement("span", {});                                } }} }
            }else{ { svelteHTML.createElement("section", { "class":`panel empty-result`,}); { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_lacinoCksalF6C = __sveltets_2_ensureComponent(FlaskConical); new $$_lacinoCksalF6C({ target: __sveltets_2_any(), props: { "size":28,}});} { svelteHTML.createElement("strong", {});     } { svelteHTML.createElement("span", {});                 } } }}

          } else if (activeNav === 'Research Lab'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});    } { svelteHTML.createElement("h1", {});  } { svelteHTML.createElement("p", {});             } } { svelteHTML.createElement("span", {    "class":`status-tag`,});researchIntegrity?.valid;!researchIntegrity?.valid;researchIntegrity?.valid ? 'LEDGER VERIFIED' : researchWorkspaceBusy ? 'VERIFYING' : 'NOT VERIFIED'; } }
             { svelteHTML.createElement("div", { "class":`notice-banner`,});  { const $$_kcehCdleihS5C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS5C({ target: __sveltets_2_any(), props: { "size":18,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});      } { svelteHTML.createElement("p", {});                                  } } }
            if(researchErrorMessage){ { svelteHTML.createElement("div", {   "class":`warnings-list`,"role":`alert`,}); { svelteHTML.createElement("div", {});  { const $$_trelAdleihS6C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS6C({ target: __sveltets_2_any(), props: { "size":14,}});} { svelteHTML.createElement("span", {});researchErrorMessage; } } }}
            if(researchNoticeMessage){ { svelteHTML.createElement("div", { "class":`notice-banner`,});  { const $$_2elcriCkcehC5C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC5C({ target: __sveltets_2_any(), props: { "size":17,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});  } { svelteHTML.createElement("p", {});researchNoticeMessage; } } }}
             { svelteHTML.createElement("section", { "class":`panel config-panel`,});
               { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});    } { svelteHTML.createElement("p", {});                } } { svelteHTML.createElement("span", { "class":`version-tag`,});researchHypotheses.length;    } }
               { svelteHTML.createElement("div", { "class":`settings-grid`,});
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-title`,});  }  { svelteHTML.createElement("input", {       "id":`research-title`,"maxlength":160,"bind:value":researchHypothesisTitle,"placeholder":`Volatility contraction continuation`,});/*Ωignore_startΩ*/() => researchHypothesisTitle = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});    } }
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-symbol`,});   } { svelteHTML.createElement("input", {    "id":`research-symbol`,"value":`BTCUSDT · 5m`,"disabled":true,});} { svelteHTML.createElement("small", {});          } }
                 { svelteHTML.createElement("div", {   "class":`config-field`,"style":`grid-column: 1 / -1;`,}); { svelteHTML.createElement("label", { "for":`research-thesis`,});  } { svelteHTML.createElement("textarea", {         "id":`research-thesis`,"rows":3,"maxlength":8000,"bind:value":researchHypothesisThesis,"placeholder":`State the observable condition, expected behaviour and how the hypothesis can be falsified.`,});/*Ωignore_startΩ*/() => researchHypothesisThesis = __sveltets_2_any(null);/*Ωignore_endΩ*/ } { svelteHTML.createElement("small", {});          } }
               }
               { svelteHTML.createElement("div", { "class":`config-bottom`,}); { svelteHTML.createElement("span", { "class":`muted-note`,});             } { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":createResearchHypothesisRecord,"disabled":researchWorkspaceBusy || researchHypothesisCreateBusy || !researchHypothesisTitle.trim() || !researchHypothesisThesis.trim(),});if(researchHypothesisCreateBusy){  { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {   "size":14,"class":`spin`,}});} }else{  { const $$_sulP7C = __sveltets_2_ensureComponent(Plus); new $$_sulP7C({ target: __sveltets_2_any(), props: { "size":14,}});}  } } }
             }
            if(researchHypotheses.some((hypothesis) => hypothesis.id === selectedResearchHypothesisId && hypothesis.status !== 'INVALIDATED')){
               { svelteHTML.createElement("section", { "class":`panel config-panel`,});
                 { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});               } } { svelteHTML.createElement("span", { "class":`mode-pill`,});   } }
                 { svelteHTML.createElement("div", { "class":`settings-grid`,});
                   { svelteHTML.createElement("div", {   "class":`config-field`,"style":`grid-column: 1 / -1;`,}); { svelteHTML.createElement("label", { "for":`research-invalidation-reason`,});   } { svelteHTML.createElement("textarea", {         "id":`research-invalidation-reason`,"rows":2,"maxlength":2000,"bind:value":researchInvalidationReason,"placeholder":`Document the evidence or methodology issue that invalidates this hypothesis.`,});/*Ωignore_startΩ*/() => researchInvalidationReason = __sveltets_2_any(null);/*Ωignore_endΩ*/ } { svelteHTML.createElement("small", {});              } }
                 }
                 { svelteHTML.createElement("div", { "class":`config-bottom`,}); { svelteHTML.createElement("span", { "class":`muted-note`,});  researchHypotheses.find((hypothesis) => hypothesis.id === selectedResearchHypothesisId)?.revision ?? '—'; } { svelteHTML.createElement("button", {     "class":`button-secondary compact`,"onclick":invalidateSelectedResearchHypothesis,"disabled":researchInvalidationBusy || researchWorkspaceBusy || !researchInvalidationReason.trim(),});if(researchInvalidationBusy){  { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {   "size":13,"class":`spin`,}});} }else{  { const $$_X7C = __sveltets_2_ensureComponent(X); new $$_X7C({ target: __sveltets_2_any(), props: { "size":13,}});}  } } }
               }
            }
             { svelteHTML.createElement("section", { "class":`panel config-panel`,});
               { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});                  } } { svelteHTML.createElement("span", { "class":`mode-pill`,});  { const $$_lacinoCksalF7C = __sveltets_2_ensureComponent(FlaskConical); new $$_lacinoCksalF7C({ target: __sveltets_2_any(), props: { "size":13,}});}   } }
               { svelteHTML.createElement("div", { "class":`settings-grid`,});
                 { svelteHTML.createElement("div", {   "class":`config-field`,"style":`grid-column: 1 / -1;`,}); { svelteHTML.createElement("label", { "for":`research-goal`,});  } { svelteHTML.createElement("textarea", {       "id":`research-goal`,"rows":2,"maxlength":2000,"bind:value":researchGoal,});/*Ωignore_startΩ*/() => researchGoal = __sveltets_2_any(null);/*Ωignore_endΩ*/ } { svelteHTML.createElement("small", {});           } }
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-hypothesis`,});  } { svelteHTML.createElement("select", {   "id":`research-hypothesis`,"bind:value":selectedResearchHypothesisId,});/*Ωignore_startΩ*/() => selectedResearchHypothesisId = __sveltets_2_any(null);/*Ωignore_endΩ*/ { svelteHTML.createElement("option", {"value":"",});   }  for(let hypothesis of __sveltets_2_ensureArray(researchHypotheses.filter((hypothesis) => hypothesis.status !== 'INVALIDATED'))){ { svelteHTML.createElement("option", { "value":hypothesis.id,});hypothesis.title;  hypothesis.revision; }} } { svelteHTML.createElement("small", {});      } }
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-fast`,});   }  { svelteHTML.createElement("input", {           "id":`research-fast`,"type":`number`,"min":`2`,"max":`299`,"step":`1`,"bind:value":researchFastPeriod,});/*Ωignore_startΩ*/() => researchFastPeriod = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});       } }
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-slow`,});   }  { svelteHTML.createElement("input", {           "id":`research-slow`,"type":`number`,"min":`3`,"max":`300`,"step":`1`,"bind:value":researchSlowPeriod,});/*Ωignore_startΩ*/() => researchSlowPeriod = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});     } }
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-capital`,});   }  { svelteHTML.createElement("input", {           "id":`research-capital`,"type":`number`,"min":`1`,"max":`1000000000000`,"step":`100`,"bind:value":researchInitialCapital,});/*Ωignore_startΩ*/() => researchInitialCapital = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});   } }
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-dd`,});   }  { svelteHTML.createElement("input", {           "id":`research-dd`,"type":`number`,"min":`0.1`,"max":`100`,"step":`0.1`,"bind:value":researchMaxDrawdownPct,});/*Ωignore_startΩ*/() => researchMaxDrawdownPct = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});       } }
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-oos`,});     }  { svelteHTML.createElement("input", {           "id":`research-oos`,"type":`number`,"min":`0`,"max":`1`,"step":`0.05`,"bind:value":researchMinimumPositiveOosRatio,});/*Ωignore_startΩ*/() => researchMinimumPositiveOosRatio = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});     } }
                 { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`research-sharpe`,});   }  { svelteHTML.createElement("input", {           "id":`research-sharpe`,"type":`number`,"min":`-10`,"max":`20`,"step":`0.1`,"bind:value":researchMinimumSharpeRatio,});/*Ωignore_startΩ*/() => researchMinimumSharpeRatio = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});     } }
                 { svelteHTML.createElement("div", {   "class":`config-field`,"style":`grid-column: 1 / -1;`,}); { svelteHTML.createElement("label", { "for":`research-tournament`,});  { svelteHTML.createElement("input", {     "id":`research-tournament`,"type":`checkbox`,"bind:checked":researchStrategyTournament,});/*Ωignore_startΩ*/() => researchStrategyTournament = __sveltets_2_any(null);/*Ωignore_endΩ*/}      } { svelteHTML.createElement("small", {});                                      } }
               }
               { svelteHTML.createElement("div", { "class":`config-bottom`,}); { svelteHTML.createElement("div", { "class":`assumption-chips`,}); { svelteHTML.createElement("span", {});   } { svelteHTML.createElement("span", {});chartSeriesResponse?.candles.length ?? 0;   } { svelteHTML.createElement("span", {}); number(snapshot.riskLimits.feeBps);  } { svelteHTML.createElement("span", {}); number(snapshot.riskLimits.slippageBps);  } } { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":executeResearchRun,"disabled":researchWorkflowBusy || researchWorkspaceBusy || !chartSeriesResponse || !researchGoal.trim() || Number(researchFastPeriod) < 2 || Number(researchSlowPeriod) <= Number(researchFastPeriod),});if(researchWorkflowBusy){  { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {   "size":14,"class":`spin`,}});}  }else{  { const $$_yalP7C = __sveltets_2_ensureComponent(Play); new $$_yalP7C({ target: __sveltets_2_any(), props: { "size":14,}});}   } } }
             }
             { svelteHTML.createElement("section", { "class":`panel table-panel`,});
               { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});researchIntegrity?.runCardCount ?? researchRunCards.length;      researchRunCards.length;               } } { svelteHTML.createElement("button", {     "class":`button-secondary compact`,"onclick":loadResearchWorkspace,"disabled":researchWorkspaceBusy,});  { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {   "size":13,"class":researchWorkspaceBusy ? 'spin' : '',}});}    } }
              if(researchWorkspaceBusy){ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_wChserfeR6C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR6C({ target: __sveltets_2_any(), props: {   "size":20,"class":`spin`,}});} { svelteHTML.createElement("span", {});   } }
              } else if (researchRunCards.length){ { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});  for(let record of __sveltets_2_ensureArray(researchRunCards)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {});new Date(record.card.createdAtMs).toLocaleString(); } { svelteHTML.createElement("td", {});record.card.goal.slice(0, 64);record.card.goal.length > 64 ? '…' : ''; } { svelteHTML.createElement("td", {}); record.card.backtest.fastPeriod; record.card.backtest.slowPeriod; } { svelteHTML.createElement("td", {  });record.card.backtest.totalReturnPct >= 0;record.card.backtest.totalReturnPct < 0;signedPct(record.card.backtest.totalReturnPct); } { svelteHTML.createElement("td", {});number(Math.abs(record.card.backtest.maxDrawdownPct), 2);  } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", {    "class":`status-tag`,});record.card.decision.eligibleForPaperValidation;!record.card.decision.eligibleForPaperValidation;record.card.decision.status; } } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("button", {   "class":`text-action`,"onclick":() => selectResearchRun(record.card.id),});record.card.id.slice(0, 12);  } } }} } } }
              }else{ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_nepOkooB6C = __sveltets_2_ensureComponent(BookOpen); new $$_nepOkooB6C({ target: __sveltets_2_any(), props: { "size":24,}});} { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("span", {});                } }}
             }
            if(selectedResearchRun){
               { svelteHTML.createElement("section", { "class":`panel result-panel`,});
                 { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});selectedResearchRun.card.symbol; selectedResearchRun.card.interval;  selectedResearchRun.card.backtest.strategyName;  new Date(selectedResearchRun.card.createdAtMs).toLocaleString(); } } { svelteHTML.createElement("span", {    "class":`status-tag`,});selectedResearchRun.card.decision.eligibleForPaperValidation;!selectedResearchRun.card.decision.eligibleForPaperValidation;selectedResearchRun.card.decision.status; } }
                 { svelteHTML.createElement("div", { "class":`result-strip`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});signedPct(selectedResearchRun.card.backtest.totalReturnPct); } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(Math.abs(selectedResearchRun.card.backtest.maxDrawdownPct), 2);  } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(selectedResearchRun.card.backtest.sharpeRatio, 3); } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});selectedResearchRun.card.backtest.tradeCount; } } }
                 { svelteHTML.createElement("details", {  "class":`dataset-provenance`,"open":true,}); { svelteHTML.createElement("summary", {});    } { svelteHTML.createElement("div", { "class":`dataset-provenance-grid`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("code", {});selectedResearchRun.card.backtest.datasetSha256; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});selectedResearchRun.card.backtest.dataSource; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});isoDateTimeLabel(selectedResearchRun.card.backtest.datasetStartMs);  isoDateTimeLabel(selectedResearchRun.card.backtest.datasetEndMs); } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("code", {});selectedResearchRun.contentSha256; } } } }
                 { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});          } } }
                 { svelteHTML.createElement("div", { "class":`event-list`,});  for(let node of __sveltets_2_ensureArray(selectedResearchRun.card.workflowGraph)){ { svelteHTML.createElement("div", { "class":`event-row`,}); { svelteHTML.createElement("span", { "class":`event-icon`,});  { const $$_2elcriCkcehC8C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC8C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});node.nodeId.replaceAll('_', ' ').toUpperCase();  node.status; } { svelteHTML.createElement("p", {});node.detail; } } { svelteHTML.createElement("time", {});new Date(node.completedAtMs).toLocaleTimeString(); } }} }
                 { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});          } } }
                 { svelteHTML.createElement("div", { "class":`result-strip`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});selectedResearchRun.card.outOfSampleReview.positiveFoldCount;  selectedResearchRun.card.outOfSampleReview.foldCount; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});selectedResearchRun.card.outOfSampleReview.medianReturnPct === null ? '—' : signedPct(selectedResearchRun.card.outOfSampleReview.medianReturnPct); } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});selectedResearchRun.card.blockBootstrap.available ? selectedResearchRun.card.blockBootstrap.simulations : 'Unavailable'; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});selectedResearchRun.card.blockBootstrap.maxDrawdownP95Pct === null ? '—' : `${number(selectedResearchRun.card.blockBootstrap.maxDrawdownP95Pct, 2)}%`; } } }
                if(selectedResearchRun.card.strategyTournament){
                   { svelteHTML.createElement("section", { "class":`ta-indicator-section tournament-section`,});
                     { svelteHTML.createElement("div", { "class":`ta-section-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});    } { svelteHTML.createElement("p", {});                  } } { svelteHTML.createElement("span", { "class":`ta-source-tag`,});selectedResearchRun.card.strategyTournament.algorithmVersion; } }
                     { svelteHTML.createElement("div", { "class":`result-strip`,});
                       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});selectedResearchRun.card.strategyTournament.candidateCount; } }
                       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});selectedResearchRun.card.strategyTournament.completedFolds;  selectedResearchRun.card.strategyTournament.requestedFolds; } }
                       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});selectedResearchRun.card.strategyTournament.positiveOosFoldRatio === null ? '—' : `${number(selectedResearchRun.card.strategyTournament.positiveOosFoldRatio * 100, 1)}%`; } }
                       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});selectedResearchRun.card.strategyTournament.parameterStabilityPct === null ? '—' : `${number(selectedResearchRun.card.strategyTournament.parameterStabilityPct, 1)}%`; } }
                     }
                     { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});
                        for(let fold of __sveltets_2_ensureArray(selectedResearchRun.card.strategyTournament.folds)){
                         { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {});fold.foldIndex;  fold.status; } { svelteHTML.createElement("td", {});isoDateTimeLabel(fold.trainingStartMs);  isoDateTimeLabel(fold.trainingEndMs); } { svelteHTML.createElement("td", {});isoDateTimeLabel(fold.testingStartMs);  isoDateTimeLabel(fold.testingEndMs); } { svelteHTML.createElement("td", {});fold.selectedFastPeriod === null ? 'No qualified pair' : `${fold.selectedFastPeriod}/${fold.selectedSlowPeriod}`; } { svelteHTML.createElement("td", {});fold.selectedTrainingSharpeRatio === null ? '—' : number(fold.selectedTrainingSharpeRatio, 3); } { svelteHTML.createElement("td", {  });(fold.outOfSampleReturnPct ?? 0) > 0;fold.outOfSampleReturnPct !== null && fold.outOfSampleReturnPct <= 0;fold.outOfSampleReturnPct === null ? '—' : signedPct(fold.outOfSampleReturnPct); } { svelteHTML.createElement("td", {});fold.outOfSampleMaxDrawdownPct === null ? '—' : `${number(fold.outOfSampleMaxDrawdownPct, 2)}%`; } { svelteHTML.createElement("td", {});fold.outOfSampleTradeCount ?? '—'; } }
                         { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", { "colspan":8,}); { svelteHTML.createElement("details", {}); { svelteHTML.createElement("summary", {});     fold.foldIndex; } { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});  for(let score of __sveltets_2_ensureArray(fold.candidateScores)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {});score.fastPeriod; } { svelteHTML.createElement("td", {});score.slowPeriod; } { svelteHTML.createElement("td", {});number(score.trainingSharpeRatio, 3); } { svelteHTML.createElement("td", {});signedPct(score.trainingReturnPct); } { svelteHTML.createElement("td", {});number(score.trainingMaxDrawdownPct, 2);  } { svelteHTML.createElement("td", {});score.trainingTradeCount; } { svelteHTML.createElement("td", {});score.qualifiesForSelection ? 'Yes' : 'No'; } }} } } } } } }
                      }
                     } } }
                    if(selectedResearchRun.card.strategyTournament.blockers.length){ { svelteHTML.createElement("div", { "class":`warnings-list`,}); { svelteHTML.createElement("h3", {});  }  for(let blocker of __sveltets_2_ensureArray(selectedResearchRun.card.strategyTournament.blockers)){ { svelteHTML.createElement("div", {});  { const $$_trelAdleihS8C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS8C({ target: __sveltets_2_any(), props: { "size":14,}});} { svelteHTML.createElement("span", {});blocker; } }} }}
                      for(let caveat of __sveltets_2_ensureArray(selectedResearchRun.card.strategyTournament.caveats)){ { svelteHTML.createElement("p", { "class":`table-footnote`,});caveat; }}
                   }
                }
                 { svelteHTML.createElement("div", { "class":`table-footnote`,}); { svelteHTML.createElement("strong", {});   }    number(selectedResearchRun.card.gateParameters.maximumDrawdownPct, 2);      number(selectedResearchRun.card.gateParameters.minimumPositiveOosRatio * 100, 1);    number(selectedResearchRun.card.gateParameters.minimumSharpeRatio, 2);           }
                 { svelteHTML.createElement("div", { "class":`event-list`,});  for(let review of __sveltets_2_ensureArray(selectedResearchRun.card.specialistReviews)){ { svelteHTML.createElement("div", { "class":`event-row`,}); { svelteHTML.createElement("span", { "class":`event-icon`,});  { const $$_kcehCdleihS8C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS8C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});review.role;  review.status; } { svelteHTML.createElement("p", {});review.summary; }  for(let blocker of __sveltets_2_ensureArray(review.blockers)){ { svelteHTML.createElement("p", { "class":`negative`,}); blocker; }}  for(let caveat of __sveltets_2_ensureArray(review.caveats)){ { svelteHTML.createElement("p", { "class":`muted-note`,});caveat; }} } }} }
                 { svelteHTML.createElement("div", { "class":`warnings-list`,}); { svelteHTML.createElement("h3", {});  }if(selectedResearchRun.card.decision.blockers.length){  for(let blocker of __sveltets_2_ensureArray(selectedResearchRun.card.decision.blockers)){ { svelteHTML.createElement("div", {});  { const $$_trelAdleihS7C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS7C({ target: __sveltets_2_any(), props: { "size":14,}});} { svelteHTML.createElement("span", {});blocker; } }}}else{ { svelteHTML.createElement("div", {});  { const $$_2elcriCkcehC7C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC7C({ target: __sveltets_2_any(), props: { "size":14,}});} { svelteHTML.createElement("span", {});          } }} }
                 { svelteHTML.createElement("p", { "class":`table-footnote`,});  selectedResearchRun.card.decision.policyVersion;                        }
               }
            }
             { svelteHTML.createElement("section", { "class":`ta-methodology-note`,});  { const $$_kcehCdleihS5C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS5C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("p", {}); { svelteHTML.createElement("strong", {});  }                                      } }

          } else if (activeNav === 'Technical Analysis'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});    } { svelteHTML.createElement("h1", {});   } { svelteHTML.createElement("p", {});              } } { svelteHTML.createElement("span", { "class":`mode-pill`,});  { const $$_3sreyaL6C = __sveltets_2_ensureComponent(Layers3); new $$_3sreyaL6C({ target: __sveltets_2_any(), props: { "size":13,}});} TECHNICAL_ANALYSIS_CATALOG.length;  } }
             { svelteHTML.createElement("div", { "class":`notice-banner`,});  { const $$_nepOkooB5C = __sveltets_2_ensureComponent(BookOpen); new $$_nepOkooB5C({ target: __sveltets_2_any(), props: { "size":18,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});         } { svelteHTML.createElement("p", {});                             } } }
             { svelteHTML.createElement("section", { "class":`ta-indicator-section`,});
               { svelteHTML.createElement("div", { "class":`ta-section-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});                 } } { svelteHTML.createElement("span", { "class":`ta-source-tag`,});candles.length;    } }
               { svelteHTML.createElement("div", { "class":`ta-reading-grid`,});
                  for(let reading of __sveltets_2_ensureArray(technicalReadings)){
                   { svelteHTML.createElement("article", { "class":`panel ta-reading-card`,}); { svelteHTML.createElement("div", { "class":`ta-reading-top`,}); { svelteHTML.createElement("span", {});reading.title; } { svelteHTML.createElement("span", {    "class":`status-tag`,});reading.available;!reading.available;reading.available ? 'CALCULATED' : 'NEEDS DATA'; } } { svelteHTML.createElement("strong", { });!reading.available;reading.value; } { svelteHTML.createElement("p", {});reading.detail; } }
                }
               }
             }
             { svelteHTML.createElement("section", { "class":`panel table-panel ta-engine-panel`,});
               { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});     } { svelteHTML.createElement("p", {});          } } { svelteHTML.createElement("span", { "class":`mode-pill`,});technicalEnvelope.engineVersion; } }
               { svelteHTML.createElement("div", { "class":`result-strip`,});
                 { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});technicalEnvelope.status.toUpperCase().replaceAll('_', ' '); } }
                 { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});technicalEnvelope.dataQuality.candleCount; } }
                 { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});technicalEnvelope.decision.coverage.accepted;  technicalEnvelope.decision.coverage.supplied; } }
                 { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});technicalEnvelope.decision.decision.replaceAll('_', ' '); } }
               }
               { svelteHTML.createElement("div", { "class":`table-footnote`,}); { svelteHTML.createElement("strong", {});  } technicalEnvelope.structure.bias;  technicalEnvelope.structure.pivots.length;    technicalEnvelope.structure.breaks.length;    }
               { svelteHTML.createElement("div", { "class":`table-footnote`,}); { svelteHTML.createElement("strong", {});    } technicalEnvelope.dataQuality.audit.status.toUpperCase();  technicalEnvelope.dataQuality.audit.hardIssues.length;    technicalEnvelope.dataQuality.audit.warnings.length;                    }
              if(technicalEnvelope.dataQuality.issues.length){
                 { svelteHTML.createElement("div", { "class":`warnings-list`,});  for(let issue of __sveltets_2_ensureArray((technicalEnvelope.dataQuality.issues.slice(0, 3)))){ { svelteHTML.createElement("div", {});  { const $$_trelAdleihS7C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS7C({ target: __sveltets_2_any(), props: { "size":14,}});} { svelteHTML.createElement("span", {});issue; } }} }
              }
              if(technicalEnvelope.decision.reasons.length){
                 { svelteHTML.createElement("div", { "class":`warnings-list`,});  for(let reason of __sveltets_2_ensureArray((technicalEnvelope.decision.reasons.slice(0, 3)))){ { svelteHTML.createElement("div", {});  { const $$_trelAdleihS7C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS7C({ target: __sveltets_2_any(), props: { "size":14,}});} { svelteHTML.createElement("span", {});reason; } }} }
              }
               { svelteHTML.createElement("p", { "class":`table-footnote`,});                                      }
             }
             { svelteHTML.createElement("section", { "class":`panel ta-catalog-panel`,});
               { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});filteredAnalysisCatalog.length;          } } { svelteHTML.createElement("span", { "class":`version-tag`,});  TECHNICAL_ANALYSIS_MASTER_LIST_VERSION; } }
               { svelteHTML.createElement("div", { "class":`ta-filters`,});
                 { svelteHTML.createElement("label", { "class":`ta-search`,});  { const $$_hcraeS7C = __sveltets_2_ensureComponent(Search); new $$_hcraeS7C({ target: __sveltets_2_any(), props: { "size":14,}});}  { svelteHTML.createElement("input", {       "bind:value":analysisQuery,"placeholder":`Search BOS, RSI, liquidity, drawdown…`,"aria-label":`Search technical analysis catalog`,"oninput":() => analysisLimit = 60,});/*Ωignore_startΩ*/() => analysisQuery = __sveltets_2_any(null);/*Ωignore_endΩ*/} }
                 { svelteHTML.createElement("select", {     "aria-label":`Filter technical analysis category`,"bind:value":analysisCategory,"onchange":() => { analysisGroup = 'All groups'; analysisLimit = 60; },});/*Ωignore_startΩ*/() => analysisCategory = __sveltets_2_any(null);/*Ωignore_endΩ*/ { svelteHTML.createElement("option", {});  }  for(let category of __sveltets_2_ensureArray(TECHNICAL_ANALYSIS_CATEGORIES)){ { svelteHTML.createElement("option", { "value":category,});category; }} }
                 { svelteHTML.createElement("select", {     "aria-label":`Filter technical analysis group`,"bind:value":analysisGroup,"onchange":() => analysisLimit = 60,});/*Ωignore_startΩ*/() => analysisGroup = __sveltets_2_any(null);/*Ωignore_endΩ*/  for(let group of __sveltets_2_ensureArray(analysisGroups)){ { svelteHTML.createElement("option", { "value":group,});group; }} }
                 { svelteHTML.createElement("select", {     "aria-label":`Filter implementation coverage`,"bind:value":analysisCoverage,"onchange":() => analysisLimit = 60,});/*Ωignore_startΩ*/() => analysisCoverage = __sveltets_2_any(null);/*Ωignore_endΩ*/ { svelteHTML.createElement("option", {});  } { svelteHTML.createElement("option", {});  } { svelteHTML.createElement("option", {});  } { svelteHTML.createElement("option", {});  } }
               }
               { svelteHTML.createElement("div", { "class":`ta-coverage-legend`,}); { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`ta-legend-dot ready`,}); }   analysisStatusCounts.calculated; } { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`ta-legend-dot ready`,}); }   analysisStatusCounts.workflow; } { svelteHTML.createElement("span", {}); { svelteHTML.createElement("i", { "class":`ta-legend-dot planned`,}); }   analysisStatusCounts.catalogOnly;    } }
              if(visibleAnalysisCatalog.length){
                 { svelteHTML.createElement("div", { "class":`ta-catalog-list`,});
                     for(let item of __sveltets_2_ensureArray(visibleAnalysisCatalog)){item.id;
                     { svelteHTML.createElement("article", { "class":`ta-catalog-item`,}); { svelteHTML.createElement("div", { "class":`ta-item-main`,}); { svelteHTML.createElement("span", { "class":`ta-item-category`,});item.category;  item.group; } { svelteHTML.createElement("strong", {});item.name; } } { svelteHTML.createElement("span", {    "class":`ta-coverage-label`,});item.available;!item.available;item.availability; } }
                  }
                 }
                 { svelteHTML.createElement("div", { "class":`panel-bottom-note`,}); { svelteHTML.createElement("span", {}); visibleAnalysisCatalog.length;  filteredAnalysisCatalog.length;   }if(visibleAnalysisCatalog.length < filteredAnalysisCatalog.length){ { svelteHTML.createElement("button", {   "class":`button-secondary compact`,"onclick":() => analysisLimit += 60,});   }} else if (analysisLimit > 60){ { svelteHTML.createElement("button", {   "class":`button-secondary compact`,"onclick":() => analysisLimit = 60,});  }} }
              }else{
                 { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_hcraeS6C = __sveltets_2_ensureComponent(Search); new $$_hcraeS6C({ target: __sveltets_2_any(), props: { "size":24,}});} { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});          } { svelteHTML.createElement("button", {   "class":`button-secondary compact`,"onclick":() => { analysisQuery = ''; analysisCategory = 'All categories'; analysisGroup = 'All groups'; analysisCoverage = 'All statuses'; analysisLimit = 60; },});  } }
              }
             }
             { svelteHTML.createElement("section", { "class":`ta-methodology-note`,});  { const $$_kcehCdleihS5C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS5C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("p", {}); { svelteHTML.createElement("strong", {});  }                                 } }

          } else if (activeNav === 'AI Agents'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});     } { svelteHTML.createElement("h1", {});   } { svelteHTML.createElement("p", {});                                       } } { svelteHTML.createElement("span", { "class":`mode-pill`,});  { const $$_toB6C = __sveltets_2_ensureComponent(Bot); new $$_toB6C({ target: __sveltets_2_any(), props: { "size":13,}});} snapshot.agents.filter((agent) => agent.status.startsWith('RUNNING')).length;  } }
             { svelteHTML.createElement("div", { "class":`notice-banner`,});  { const $$_kcehCdleihS5C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS5C({ target: __sveltets_2_any(), props: { "size":18,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("p", {});                                  } } }
             { const $$_lenaPemitnuRtnegA4C = __sveltets_2_ensureComponent(AgentRuntimePanel); new $$_lenaPemitnuRtnegA4C({ target: __sveltets_2_any(), props: {        "desktop":desktop,"symbol":activeCandleRequest.symbol,"timeframe":activeCandleRequest.interval,"marketReady":Boolean(chartSeriesResponse),}});}
             { svelteHTML.createElement("section", { "class":`agent-grid`,});  for(let agent of __sveltets_2_ensureArray(snapshot.agents)){ { svelteHTML.createElement("article", { "class":`panel agent-card`,}); { svelteHTML.createElement("div", { "class":`agent-card-top`,}); { svelteHTML.createElement("div", { "class":`agent-avatar`,});  { const $$_toB8C = __sveltets_2_ensureComponent(Bot); new $$_toB8C({ target: __sveltets_2_any(), props: { "size":22,}});} } { svelteHTML.createElement("div", { "class":`agent-title`,}); { svelteHTML.createElement("h2", {});agent.name; } { svelteHTML.createElement("p", {});agent.role; } } { svelteHTML.createElement("span", {    "class":`status-tag`,});agent.status.startsWith('RUNNING');!agent.status.startsWith('RUNNING');agent.status.startsWith('RUNNING') ? 'RUNNING' : 'STOPPED'; } } { svelteHTML.createElement("p", { "class":`agent-objective`,});agent.objective; } { svelteHTML.createElement("div", { "class":`agent-stats`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});agent.mode; } } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {  });agent.pnl >= 0;agent.pnl < 0;money(agent.pnl); } } } { svelteHTML.createElement("div", { "class":`agent-tools`,}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("div", {});  for(let tool of __sveltets_2_ensureArray(agent.tools)){ { svelteHTML.createElement("span", {});tool; }} } } { svelteHTML.createElement("div", { "class":`agent-last`,});  { const $$_3kcolC7C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC7C({ target: __sveltets_2_any(), props: { "size":13,}});} { svelteHTML.createElement("span", {});agent.lastDecision; } } { svelteHTML.createElement("button", {     "class":`button-secondary full-width`,"onclick":() => toggleAgent(agent),});agent.status.startsWith('RUNNING');if(agent.status.startsWith('RUNNING')){  { const $$_esuaP7C = __sveltets_2_ensureComponent(Pause); new $$_esuaP7C({ target: __sveltets_2_any(), props: { "size":14,}});}  }else{  { const $$_yalP7C = __sveltets_2_ensureComponent(Play); new $$_yalP7C({ target: __sveltets_2_any(), props: { "size":14,}});}  } } }} }
             { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});         } } } { svelteHTML.createElement("div", { "class":`event-list`,});  for(let event of __sveltets_2_ensureArray((snapshot.events.filter((event) => event.category === 'agent' || event.category === 'risk').slice(0, 10)))){ { svelteHTML.createElement("div", { "class":`event-row`,}); { svelteHTML.createElement("span", { "class":`event-icon`,});  { const $$_toB8C = __sveltets_2_ensureComponent(Bot); new $$_toB8C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});event.title; } { svelteHTML.createElement("p", {});event.detail; } } { svelteHTML.createElement("time", {});timeLabel(event.timestampMs); } }} } }

          } else if (activeNav === 'MT5 Bridge'){
             { const $$_lenaPegdirB5tM4C = __sveltets_2_ensureComponent(Mt5BridgePanel); new $$_lenaPegdirB5tM4C({ target: __sveltets_2_any(), props: {}});}
          } else if (activeNav === 'Live Trading'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});    } { svelteHTML.createElement("h1", {});  } { svelteHTML.createElement("p", {});         } } { svelteHTML.createElement("span", { "class":`locked-pill`,});  { const $$_elohyeKkcoL6C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL6C({ target: __sveltets_2_any(), props: { "size":13,}});}  } }
             { svelteHTML.createElement("section", { "class":`panel live-lock-panel`,}); { svelteHTML.createElement("div", { "class":`lock-orb`,});  { const $$_elohyeKkcoL6C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL6C({ target: __sveltets_2_any(), props: { "size":30,}});} } { svelteHTML.createElement("h2", {});     } { svelteHTML.createElement("p", {});                             } { svelteHTML.createElement("div", { "class":`lock-requirements`,}); { svelteHTML.createElement("div", {});  { const $$_2elcriCkcehC7C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});      } { svelteHTML.createElement("strong", {});  } } { svelteHTML.createElement("div", {});  { const $$_2elcriCkcehC7C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});      } { svelteHTML.createElement("strong", {});  } } { svelteHTML.createElement("div", {});  { const $$_3kcolC7C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});      } { svelteHTML.createElement("strong", {});  } } { svelteHTML.createElement("div", {});  { const $$_3kcolC7C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});     } { svelteHTML.createElement("strong", {});  } } { svelteHTML.createElement("div", {});  { const $$_3kcolC7C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});      } { svelteHTML.createElement("strong", {});  } } } { svelteHTML.createElement("button", {   "class":`button-secondary`,"onclick":() => { void navigate('Paper Trading'); },});    } }

          } else if (activeNav === 'Market Data'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});     } { svelteHTML.createElement("h1", {});  } { svelteHTML.createElement("p", {});                  } } { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":refreshSnapshot,"disabled":refreshBusy,});  { const $$_wChserfeR6C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR6C({ target: __sveltets_2_any(), props: {   "size":15,"class":refreshBusy ? 'spin' : '',}});}    } }
             { svelteHTML.createElement("section", { "class":`provider-card panel`,}); { svelteHTML.createElement("div", { "class":`provider-main-icon`,});  { const $$_esabataD6C = __sveltets_2_ensureComponent(Database); new $$_esabataD6C({ target: __sveltets_2_any(), props: { "size":22,}});} } { svelteHTML.createElement("div", { "class":`provider-main-copy`,}); { svelteHTML.createElement("h2", {});snapshot.providerStatus; } { svelteHTML.createElement("p", {});snapshot.providerSource; } { svelteHTML.createElement("small", {});  new Date(snapshot.providerUpdatedAtMs).toLocaleString();  ageLabel(snapshot.providerUpdatedAtMs, clockNow); } } { svelteHTML.createElement("span", {    "class":`status-tag`,});providerIsFresh;!providerIsFresh;providerIsFresh ? 'RECENT' : 'STALE'; } }
             { svelteHTML.createElement("section", { "class":`panel capability-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});               } } { svelteHTML.createElement("span", { "class":`version-tag`,});  } } { svelteHTML.createElement("div", { "class":`capability-list`,});  for(let series of __sveltets_2_ensureArray(snapshot.marketDataCapabilities.candleSeries)){ { svelteHTML.createElement("div", { "class":`capability-row`,}); { svelteHTML.createElement("div", { "class":`capability-primary`,}); { svelteHTML.createElement("strong", {});series.symbol;  series.interval; } { svelteHTML.createElement("span", {});series.note; } { svelteHTML.createElement("small", {});series.source;  series.candleCount; series.latestClosedBarAtMs ? ` · last closed ${new Date(series.latestClosedBarAtMs).toLocaleString()}` : '';series.retrievedAtMs ? ` · received ${new Date(series.retrievedAtMs).toLocaleString()}` : ''; } } { svelteHTML.createElement("span", {     "class":`status-tag`,});series.available && series.isLive;series.available && !series.isLive;!series.available;series.available ? series.isLive ? 'LIVE · AVAILABLE' : 'CACHED · NOT LIVE' : 'NOT AVAILABLE'; } { svelteHTML.createElement("button", {     "class":`button-secondary compact`,"disabled":!series.supported || !series.available || chartSeriesBusy,"onclick":() => { void openCandleSeries(series); },});  { const $$_trahCkcitseldnaC8C = __sveltets_2_ensureComponent(CandlestickChart); new $$_trahCkcitseldnaC8C({ target: __sveltets_2_any(), props: { "size":13,}});}   } }} } { svelteHTML.createElement("div", { "class":`panel-bottom-note`,}); { svelteHTML.createElement("span", {});  snapshot.marketDataCapabilities.quoteSymbols.length;          } { svelteHTML.createElement("span", { "class":`source-label`,});snapshot.marketDataCapabilities.quoteSymbols.join(', '); } } }
             { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});        } } { svelteHTML.createElement("div", { "class":`table-search`,});  { const $$_hcraeS7C = __sveltets_2_ensureComponent(Search); new $$_hcraeS7C({ target: __sveltets_2_any(), props: { "size":14,}});}  { svelteHTML.createElement("input", {   "bind:value":searchQuery,"placeholder":`Filter symbols`,});/*Ωignore_startΩ*/() => searchQuery = __sveltets_2_any(null);/*Ωignore_endΩ*/} } } { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});   } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});   } { svelteHTML.createElement("th", {}); } } } { svelteHTML.createElement("tbody", {});  for(let quote of __sveltets_2_ensureArray(filteredQuotes)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {}); { svelteHTML.createElement("button", {   "class":`asset-link`,"onclick":() => selectSymbol(quote.symbol),}); { svelteHTML.createElement("span", { "class":`mini-coin`,});quote.symbol.slice(0, 1); } { svelteHTML.createElement("strong", {});quote.symbol; } } } { svelteHTML.createElement("td", {});quoteIsFresh(quote, clockNow) ? money(quote.price) : 'Unavailable · stale quote'; } { svelteHTML.createElement("td", {  });quoteIsFresh(quote, clockNow) && quote.change24hPct >= 0;quoteIsFresh(quote, clockNow) && quote.change24hPct < 0;quoteIsFresh(quote, clockNow) ? signedPct(quote.change24hPct) : 'Unavailable'; } { svelteHTML.createElement("td", {});quoteIsFresh(quote, clockNow) ? money(quote.volume24h, true) : 'Unavailable'; } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", { "class":`source-label`,});quote.source; } } { svelteHTML.createElement("td", {});ageLabel(quote.updatedAtMs, clockNow);  ageLabel(quote.retrievedAtMs, clockNow); } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("button", {   "class":`table-action`,"onclick":() => selectSymbol(quote.symbol),});  } } }} } } } }
             { const $$_lenaPmroftalPataDtekraM4C = __sveltets_2_ensureComponent(MarketDataPlatformPanel); new $$_lenaPmroftalPataDtekraM4C({ target: __sveltets_2_any(), props: {}});}

          } else if (activeNav === 'Meme Coin Hunter'){
             { const $$_lenaPretnuHnioCemeM4C = __sveltets_2_ensureComponent(MemeCoinHunterPanel); new $$_lenaPretnuHnioCemeM4C({ target: __sveltets_2_any(), props: {}});}
          } else if (activeNav === 'Portfolio'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});   } { svelteHTML.createElement("h1", {});  } { svelteHTML.createElement("p", {});          } } { svelteHTML.createElement("span", { "class":`mode-pill`,});  { const $$_tellaW6C = __sveltets_2_ensureComponent(Wallet); new $$_tellaW6C({ target: __sveltets_2_any(), props: { "size":13,}});}   } }
             { svelteHTML.createElement("section", { "class":`portfolio-metrics metric-grid`,}); { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  }  { const $$_tellaW7C = __sveltets_2_ensureComponent(Wallet); new $$_tellaW7C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", { "class":`metric-value`,});portfolioMarksFresh ? money(snapshot.equity) : 'Unavailable'; } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});portfolioMarksFresh ? 'Cash + fresh marked positions' : 'Refresh every open-position mark'; } } } { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  }  { const $$_ytivitcA7C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA7C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", { "class":`metric-value`,});money(snapshot.cashBalance); } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});     } } } { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  }  { const $$_pUgnidnerT7C = __sveltets_2_ensureComponent(TrendingUp); new $$_pUgnidnerT7C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", {   "class":`metric-value`,});snapshot.realizedPnl >= 0;snapshot.realizedPnl < 0;money(snapshot.realizedPnl); } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});   } } } { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("div", { "class":`metric-head`,}); { svelteHTML.createElement("span", {});  }  { const $$_nwoDgnidnerT7C = __sveltets_2_ensureComponent(TrendingDown); new $$_nwoDgnidnerT7C({ target: __sveltets_2_any(), props: { "size":15,}});} } { svelteHTML.createElement("div", {   "class":`metric-value`,});snapshot.unrealizedPnl >= 0;snapshot.unrealizedPnl < 0;portfolioMarksFresh ? money(snapshot.unrealizedPnl) : 'Unavailable'; } { svelteHTML.createElement("div", { "class":`metric-foot`,}); { svelteHTML.createElement("span", {});portfolioMarksFresh ? 'Open paper positions' : 'One or more marks are stale or missing'; } } } }
             { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});            } } }if(snapshot.positions.length){ { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});  for(let position of __sveltets_2_ensureArray(snapshot.positions)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {}); { svelteHTML.createElement("strong", {});position.symbol; } } { svelteHTML.createElement("td", {});number(position.quantity, 8); } { svelteHTML.createElement("td", {});money(position.averageEntry); } { svelteHTML.createElement("td", {});positionMarkFresh(position.symbol, clockNow) ? money(position.markPrice) : 'Unavailable · stale mark'; } { svelteHTML.createElement("td", {});positionMarkFresh(position.symbol, clockNow) ? money(position.notional) : 'Unavailable'; } { svelteHTML.createElement("td", {  });position.unrealizedPnl >= 0;position.unrealizedPnl < 0;positionMarkFresh(position.symbol, clockNow) ? money(position.unrealizedPnl) : 'Unavailable'; } }} } } }}else{ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_tellaW6C = __sveltets_2_ensureComponent(Wallet); new $$_tellaW6C({ target: __sveltets_2_any(), props: { "size":24,}});} { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});      } }} }
             { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});           } } }if(snapshot.positions.length && portfolioMarksFresh){ { svelteHTML.createElement("div", { "class":`exposure-list`,});  for(let position of __sveltets_2_ensureArray(snapshot.positions)){ { svelteHTML.createElement("div", { "class":`exposure-row`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});position.symbol; } { svelteHTML.createElement("span", {});money(position.notional); } } { svelteHTML.createElement("div", { "class":`exposure-track`,}); { svelteHTML.createElement("span", { "style":`width:${Math.min(100, snapshot.equity > 0 ? position.notional / snapshot.equity * 100 : 0)}%`,}); } } { svelteHTML.createElement("small", {});number(snapshot.equity > 0 ? position.notional / snapshot.equity * 100 : 0);  } }} }} else if (snapshot.positions.length){ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_trelAdleihS6C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS6C({ target: __sveltets_2_any(), props: { "size":20,}});} { svelteHTML.createElement("strong", {});  } { svelteHTML.createElement("span", {});            } }}else{ { svelteHTML.createElement("div", { "class":`empty-small`,});     }} }

          } else if (activeNav === 'Audit Log'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});    } { svelteHTML.createElement("h1", {});  } { svelteHTML.createElement("p", {});             } } { svelteHTML.createElement("div", { "class":`heading-actions`,}); { svelteHTML.createElement("button", {     "class":`button-secondary`,"onclick":exportAuditCsv,"disabled":auditBusy || filteredAuditEvents.length === 0,});  { const $$_daolnwoD7C = __sveltets_2_ensureComponent(Download); new $$_daolnwoD7C({ target: __sveltets_2_any(), props: { "size":14,}});}   filteredAuditEvents.length;  } { svelteHTML.createElement("button", {     "class":`button-secondary`,"onclick":loadAuditHistory,"disabled":auditBusy,});  { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {   "size":14,"class":auditBusy ? 'spin' : '',}});}   } } }
             { svelteHTML.createElement("div", { "class":`notice-banner`,});  { const $$_esabataD5C = __sveltets_2_ensureComponent(Database); new $$_esabataD5C({ target: __sveltets_2_any(), props: { "size":18,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});       } { svelteHTML.createElement("p", {});                        } } }
             { svelteHTML.createElement("section", { "class":`panel table-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});filteredAuditEvents.length;    auditEvents.length;      } } { svelteHTML.createElement("div", { "class":`table-search`,});  { const $$_hcraeS7C = __sveltets_2_ensureComponent(Search); new $$_hcraeS7C({ target: __sveltets_2_any(), props: { "size":14,}});}  { svelteHTML.createElement("input", {   "bind:value":auditQuery,"placeholder":`Filter event details`,});/*Ωignore_startΩ*/() => auditQuery = __sveltets_2_any(null);/*Ωignore_endΩ*/} } }if(auditBusy){ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_wChserfeR6C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR6C({ target: __sveltets_2_any(), props: {   "size":20,"class":`spin`,}});} { svelteHTML.createElement("span", {});   } }} else if (filteredAuditEvents.length){ { svelteHTML.createElement("div", { "class":`table-wrap`,}); { svelteHTML.createElement("table", { "class":`data-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});  for(let event of __sveltets_2_ensureArray(filteredAuditEvents)){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {});new Date(event.timestampMs).toLocaleString(); } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", {     "class":`status-tag`,});event.level === 'ERROR';event.level === 'INFO';event.level === 'WARN';event.level; } } { svelteHTML.createElement("td", {});event.category; } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("strong", {});event.title; } } { svelteHTML.createElement("td", { "class":`audit-detail`,});event.detail; } { svelteHTML.createElement("td", { "class":`event-id-cell`,});event.id.slice(0, 12);  } }} } } }}else{ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_3kcolC6C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC6C({ target: __sveltets_2_any(), props: { "size":24,}});} { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("span", {});        } }} }

          } else if (activeNav === 'Tools & MCP'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});    } { svelteHTML.createElement("h1", {});   } { svelteHTML.createElement("p", {});         } } { svelteHTML.createElement("span", { "class":`locked-pill`,});  { const $$_elohyeKkcoL6C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL6C({ target: __sveltets_2_any(), props: { "size":13,}});}    } }
             { svelteHTML.createElement("section", { "class":`panel tool-security-card`,}); { svelteHTML.createElement("div", { "class":`tool-large-icon`,});  { const $$_krowteN6C = __sveltets_2_ensureComponent(Network); new $$_krowteN6C({ target: __sveltets_2_any(), props: { "size":24,}});} } { svelteHTML.createElement("h2", {});     } { svelteHTML.createElement("p", {});                        } { svelteHTML.createElement("div", { "class":`security-rule-list`,}); { svelteHTML.createElement("div", {});  { const $$_kcehCdleihS7C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});   } { svelteHTML.createElement("strong", {});  } } { svelteHTML.createElement("div", {});  { const $$_kcehCdleihS7C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});     } { svelteHTML.createElement("strong", {});  } } { svelteHTML.createElement("div", {});  { const $$_3kcolC7C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});      } { svelteHTML.createElement("strong", {});  } } { svelteHTML.createElement("div", {});  { const $$_3kcolC7C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC7C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("span", {});      } { svelteHTML.createElement("strong", {});  } } } }

          } else if (activeNav === 'Settings'){
             { svelteHTML.createElement("div", { "class":`page-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("div", { "class":`eyebrow`,});    } { svelteHTML.createElement("h1", {});  } { svelteHTML.createElement("p", {});            } } { svelteHTML.createElement("span", { "class":`mode-pill`,});  { const $$_sgnitteS6C = __sveltets_2_ensureComponent(Settings); new $$_sgnitteS6C({ target: __sveltets_2_any(), props: { "size":13,}});}  } }
             { svelteHTML.createElement("section", { "class":`panel settings-panel`,}); { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});            } }  { const $$_kcehCdleihS6C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS6C({ target: __sveltets_2_any(), props: { "size":17,}});} } { svelteHTML.createElement("div", { "class":`settings-grid`,}); { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`max-order`,});    }  { svelteHTML.createElement("input", {         "id":`max-order`,"type":`number`,"min":`1`,"step":`10`,"bind:value":riskForm.maxOrderNotional,});/*Ωignore_startΩ*/() => riskForm.maxOrderNotional = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});      } } { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`max-position`,});    }  { svelteHTML.createElement("input", {         "id":`max-position`,"type":`number`,"min":`1`,"step":`50`,"bind:value":riskForm.maxPositionNotional,});/*Ωignore_startΩ*/() => riskForm.maxPositionNotional = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});     } } { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`max-loss`,});    }  { svelteHTML.createElement("input", {         "id":`max-loss`,"type":`number`,"min":`1`,"step":`10`,"bind:value":riskForm.maxDailyLoss,});/*Ωignore_startΩ*/() => riskForm.maxDailyLoss = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});         } } { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`fee-bps`,});    }  { svelteHTML.createElement("input", {           "id":`fee-bps`,"type":`number`,"min":`0`,"max":`500`,"step":`1`,"bind:value":riskForm.feeBps,});/*Ωignore_startΩ*/() => riskForm.feeBps = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});     } } { svelteHTML.createElement("div", { "class":`config-field`,}); { svelteHTML.createElement("label", { "for":`slippage-bps`,});   }  { svelteHTML.createElement("input", {           "id":`slippage-bps`,"type":`number`,"min":`0`,"max":`500`,"step":`1`,"bind:value":riskForm.slippageBps,});/*Ωignore_startΩ*/() => riskForm.slippageBps = __sveltets_2_any(null);/*Ωignore_endΩ*/} { svelteHTML.createElement("small", {});    } } } { svelteHTML.createElement("div", { "class":`config-bottom`,}); { svelteHTML.createElement("p", { "class":`muted-note`,});                  } { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":saveRiskSettings,"disabled":riskBusy || !desktop,});if(riskBusy){  { const $$_wChserfeR7C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR7C({ target: __sveltets_2_any(), props: {   "size":14,"class":`spin`,}});} }else{  { const $$_2elcriCkcehC7C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC7C({ target: __sveltets_2_any(), props: { "size":14,}});}   } } } }
             { svelteHTML.createElement("section", { "class":`panel danger-zone`,}); { svelteHTML.createElement("div", { "class":`danger-title`,});  { const $$_trelAdleihS6C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS6C({ target: __sveltets_2_any(), props: { "size":18,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});              } } } { svelteHTML.createElement("button", {   "class":`button-danger`,"onclick":resetAccount,});   } }
             { svelteHTML.createElement("section", { "class":`panel provider-card`,}); { svelteHTML.createElement("div", { "class":`provider-main-icon`,});  { const $$_upC6C = __sveltets_2_ensureComponent(Cpu); new $$_upC6C({ target: __sveltets_2_any(), props: { "size":22,}});} } { svelteHTML.createElement("div", { "class":`provider-main-copy`,}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});          } { svelteHTML.createElement("small", {});                } } { svelteHTML.createElement("span", {    "class":`status-tag`,});desktop;!desktop;  } }
          }
         }

         { svelteHTML.createElement("aside", { "class":`right-rail`,});
           { svelteHTML.createElement("section", { "class":`rail-section watchlist-section`,}); { svelteHTML.createElement("div", { "class":`rail-heading`,}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("button", {     "class":`icon-button tiny`,"aria-label":`View market data`,"onclick":() => { void navigate('Market Data'); },});  { const $$_sulP7C = __sveltets_2_ensureComponent(Plus); new $$_sulP7C({ target: __sveltets_2_any(), props: { "size":16,}});} } } { svelteHTML.createElement("div", { "class":`watchlist-tabs`,}); { svelteHTML.createElement("span", {   "class":`selected`,"aria-current":`page`,});  } { svelteHTML.createElement("button", { "disabled":true,"title":`No Forex data provider is connected`,});  } { svelteHTML.createElement("button", { "disabled":true,"title":`No indices data provider is connected`,});  } { svelteHTML.createElement("button", { "disabled":true,"title":`No commodities data provider is connected`,});  } } { svelteHTML.createElement("div", { "class":`watchlist-header`,}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("span", {});  } }if(snapshot.quotes.length === 0){ { svelteHTML.createElement("div", { "class":`empty-small`,});      }}  for(let quote of __sveltets_2_ensureArray((snapshot.quotes.slice(0, 6)))){ { svelteHTML.createElement("button", {   "class":`watchlist-row`,"onclick":() => selectSymbol(quote.symbol),}); { svelteHTML.createElement("span", { "class":`watch-asset`,}); { svelteHTML.createElement("span", { "class":`watch-symbol-icon`,});quote.symbol.startsWith('BTC') ? '₿' : quote.symbol.startsWith('ETH') ? '◆' : quote.symbol.slice(0, 1); } { svelteHTML.createElement("strong", {});quote.symbol; } { svelteHTML.createElement("small", {});quoteIsFresh(quote, clockNow) ? 'Live' : 'Stale cached'; } } { svelteHTML.createElement("span", {});quoteIsFresh(quote, clockNow) ? number(quote.price, quote.price < 1 ? 5 : 2) : '—'; } { svelteHTML.createElement("span", {  });quoteIsFresh(quote, clockNow) && quote.change24hPct >= 0;quoteIsFresh(quote, clockNow) && quote.change24hPct < 0;quoteIsFresh(quote, clockNow) ? signedPct(quote.change24hPct) : '—'; } { svelteHTML.createElement("span", { "class":`watch-volume`,});quoteIsFresh(quote, clockNow) ? compactNumber(quote.volume24h) : '—'; } }} }
           { svelteHTML.createElement("section", { "class":`rail-section agents-rail`,}); { svelteHTML.createElement("div", { "class":`rail-heading`,}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("button", {   "class":`text-action`,"onclick":() => { void navigate('AI Agents'); },});  } }  for(let agent of __sveltets_2_ensureArray(snapshot.agents)){ { svelteHTML.createElement("div", { "class":`rail-agent`,}); { svelteHTML.createElement("div", { "class":`rail-agent-icon`,});  { const $$_toB7C = __sveltets_2_ensureComponent(Bot); new $$_toB7C({ target: __sveltets_2_any(), props: { "size":18,}});} } { svelteHTML.createElement("div", { "class":`rail-agent-copy`,}); { svelteHTML.createElement("strong", {});agent.name; } { svelteHTML.createElement("small", {});agent.role; } { svelteHTML.createElement("span", {   "class":`agent-status`,});agent.status.startsWith('RUNNING'); { svelteHTML.createElement("span", { "class":`small-dot`,}); }agent.status.startsWith('RUNNING') ? 'Running · observe-only' : 'Stopped'; } { svelteHTML.createElement("small", {    "class":`rail-agent-pnl`,});agent.pnl >= 0;agent.pnl < 0; money(agent.pnl); } } { svelteHTML.createElement("button", {       "class":`toggle-control`,"aria-label":`${agent.status.startsWith('RUNNING') ? 'Stop' : 'Start'} ${agent.name}`,"onclick":() => toggleAgent(agent),});agent.status.startsWith('RUNNING'); { svelteHTML.createElement("span", {}); } } }} }
           { svelteHTML.createElement("section", { "class":`rail-section events-rail`,}); { svelteHTML.createElement("div", { "class":`rail-heading`,}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("button", {   "class":`text-action`,"onclick":() => { void navigate('Audit Log'); },});  } }  for(let event of __sveltets_2_ensureArray((snapshot.events.slice(0, 6)))){ { svelteHTML.createElement("div", { "class":`rail-event`,}); { svelteHTML.createElement("div", {   "class":`event-level-icon`,});event.level === 'WARN';event.level === 'ERROR';if(event.category === 'risk'){  { const $$_trelAdleihS7C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS7C({ target: __sveltets_2_any(), props: { "size":14,}});}} else if (event.category === 'market-data'){  { const $$_ytivitcA7C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA7C({ target: __sveltets_2_any(), props: { "size":14,}});}} else if (event.category === 'agent'){  { const $$_toB7C = __sveltets_2_ensureComponent(Bot); new $$_toB7C({ target: __sveltets_2_any(), props: { "size":14,}});}}else{  { const $$_2elcriCkcehC7C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC7C({ target: __sveltets_2_any(), props: { "size":14,}});}} } { svelteHTML.createElement("div", { "class":`rail-event-copy`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("time", {});timeLabel(event.timestampMs); } { svelteHTML.createElement("strong", {});event.title; } } { svelteHTML.createElement("p", {});event.detail; } } }} }
           { svelteHTML.createElement("section", { "class":`rail-security-note`,}); { svelteHTML.createElement("div", {});  { const $$_kcehCdleihS6C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS6C({ target: __sveltets_2_any(), props: { "size":15,}});} { svelteHTML.createElement("strong", {});  } } { svelteHTML.createElement("p", {});                  } { svelteHTML.createElement("button", {   "class":`text-action`,"onclick":() => { void navigate('Settings'); },});    { const $$_thgiRpUworrA6C = __sveltets_2_ensureComponent(ArrowUpRight); new $$_thgiRpUworrA6C({ target: __sveltets_2_any(), props: { "size":12,}});} } }
         }
       }
       { svelteHTML.createElement("footer", { "class":`app-footer`,}); { svelteHTML.createElement("span", {});     } { svelteHTML.createElement("span", {}); { svelteHTML.createElement("span", { "class":`small-dot`,}); }    } { svelteHTML.createElement("span", {});       } }
    }else{
       { svelteHTML.createElement("section", {     "class":`runtime-load-failure`,"role":`alert`,"aria-labelledby":`runtime-load-failure-title`,});
         { const $$_trelAdleihS3C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS3C({ target: __sveltets_2_any(), props: {  "size":28,}});}
         { svelteHTML.createElement("h1", { "id":`runtime-load-failure-title`,});   }
         { svelteHTML.createElement("p", {});errorMessage || 'The native engine did not return a validated dashboard snapshot. No market data or decision result is available.'; }
         { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":loadSnapshot,"disabled":loading,});  { const $$_wChserfeR4C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR4C({ target: __sveltets_2_any(), props: {   "size":14,"class":loading ? 'spin' : '',}});}    }
       }
    }
   }
 }
}
};
return { props: {} as Record<string, never>, exports: {}, bindings: "", slots: {}, events: {} }}
const App__SvelteComponent_ = __sveltets_2_isomorphic_component(__sveltets_2_with_any_event($$render()));
/*Ωignore_startΩ*/type App__SvelteComponent_ = InstanceType<typeof App__SvelteComponent_>;
/*Ωignore_endΩ*/export default App__SvelteComponent_;