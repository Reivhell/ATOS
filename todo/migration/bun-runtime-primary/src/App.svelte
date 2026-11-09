<script lang="ts">
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
  $: activeChartQuote = quoteFor(activeCandleRequest.symbol);
  $: activeChartQuoteFresh = positionMarkFresh(activeCandleRequest.symbol, clockNow);
  function selectSymbol(symbol: string) { selectedSymbol = symbol; activeNav = 'Trading'; if (!isOrderChartAligned(symbol, activeCandleRequest)) noticeMessage = `${symbol} has quote data but no loaded candle series; the chart remains ${activeCandleRequest.symbol} ${activeCandleRequest.interval}. The paper order will use the ${symbol} quote only.`; }

  $: currentQuote = selectedSymbol ? quoteFor(selectedSymbol) : undefined;
  $: currentQuoteFresh = quoteIsFresh(currentQuote, clockNow);
  $: portfolioMarksFresh = Number.isFinite(clockNow) && (snapshot?.positions ?? []).every((position) => positionMarkFresh(position.symbol, clockNow));
  $: freshQuotes = (snapshot?.quotes ?? []).filter((quote) => quoteIsFresh(quote, clockNow));
  $: filteredQuotes = (snapshot?.quotes ?? []).filter((quote) => quote.symbol.toLowerCase().includes(searchQuery.toLowerCase()));
  $: filteredAuditEvents = auditEvents.filter((event) => `${event.level} ${event.category} ${event.title} ${event.detail}`.toLowerCase().includes(auditQuery.toLowerCase()));
  // Screener rows are displayed as current evidence only while the provider snapshot is connected,
  // timestamp-valid, and within the same bounded freshness window as market quotes.
  $: screenerSnapshotFresh = !!snapshot
    && snapshot.screenerProviderStatus.startsWith('CONNECTED')
    && isFreshMarketTimestamp(snapshot.screenerProviderUpdatedAtMs, clockNow);
  $: if (!screenerSnapshotFresh) selectedScreenerToken = null;
  $: sortedScreener = [...(screenerSnapshotFresh ? (snapshot?.screener ?? []) : [])]
    .filter((token) => `${token.symbol} ${token.name} ${token.chain}`.toLowerCase().includes(screenerQuery.toLowerCase()))
    .sort((a, b) => compareScreenerTokens(a, b, screenerSort));
  $: candles = chartSeriesResponse?.candles ?? [];
  $: chartMin = candles.length ? Math.min(...candles.map((c) => c.low)) : 0;
  $: chartMax = candles.length ? Math.max(...candles.map((c) => c.high)) : 1;
  $: chartRange = Math.max(chartMax - chartMin, 0.000001);
  $: orderQuantity = Number(quantityText);
  $: estimatedNotional = currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? currentQuote.price * orderQuantity : 0;
  $: estimatedFee = estimatedNotional * ((snapshot?.riskLimits.feeBps ?? 10) / 10000);
  $: closePath = candles.map((c, i) => `${i === 0 ? 'M' : 'L'} ${chartX(i)} ${priceY(c.close)}`).join(' ');
  $: maFastPath = movingAveragePath(9);
  $: maSlowPath = movingAveragePath(21);
  $: providerIsFresh = !!snapshot && snapshot.providerStatus.startsWith('CONNECTED') && isFreshMarketTimestamp(snapshot.providerUpdatedAtMs, clockNow);
  $: technicalEnvelope = createTechnicalAnalysisReport({
    validatedSeries: chartSeriesResponse,
    requestedSymbol: activeCandleRequest.symbol,
    requestedTimeframe: activeCandleRequest.interval,
  });
  $: technicalReadings = technicalEnvelope.indicatorReadings;
  $: lastCandleSummary = summarizeLastCandle(candles);
  $: orderChartAligned = isOrderChartAligned(selectedSymbol, activeCandleRequest);
  $: activeSeriesCapability = snapshot?.marketDataCapabilities.candleSeries.find((series) => series.symbol === activeCandleRequest.symbol && series.interval === activeCandleRequest.interval);
  $: maxCandleVolume = getMaximumVolume(candles);
  $: analysisGroups = ['All groups', ...new Set(TECHNICAL_ANALYSIS_CATALOG.filter((item) => analysisCategory === 'All categories' || item.category === analysisCategory).map((item) => item.group))];
  $: analysisStatusCounts = {
    calculated: TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Calculated now').length,
    workflow: TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Workflow available').length,
    catalogOnly: TECHNICAL_ANALYSIS_CATALOG.filter((item) => item.availability === 'Catalog only').length,
  };
  $: filteredAnalysisCatalog = TECHNICAL_ANALYSIS_CATALOG.filter((item) => {
    const search = analysisQuery.trim().toLowerCase();
    const matchesQuery = !search || `${item.name} ${item.group} ${item.category} ${item.kind}`.toLowerCase().includes(search);
    const matchesCategory = analysisCategory === 'All categories' || item.category === analysisCategory;
    const matchesGroup = analysisGroup === 'All groups' || item.group === analysisGroup;
    const matchesCoverage = analysisCoverage === 'All statuses' || item.availability === analysisCoverage;
    return matchesQuery && matchesCategory && matchesGroup && matchesCoverage;
  });
  $: visibleAnalysisCatalog = filteredAnalysisCatalog.slice(0, analysisLimit);

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

</script>

<svelte:head>
  <title>QuantDinger AI Trading OS</title>
  <meta name="description" content="QuantDinger AI Trading OS — market research, backtesting, and paper trading with a local risk gateway." />
</svelte:head>

{#if !desktop}
  <main class="runtime-required-screen" role="alert" aria-labelledby="runtime-required-title">
    <div class="runtime-required-mark"><LockKeyhole size={28} /></div>
    <p class="runtime-required-eyebrow">NATIVE RUNTIME BOUNDARY</p>
    <h1 id="runtime-required-title">Installed desktop runtime required</h1>
    <p class="runtime-required-copy">This application has no browser data mode. Market observations, AI provider operations, model registry access, the audit journal, analytics, and paper-risk operations require the Rust/Tauri runtime.</p>
    <div class="runtime-required-details">
      <div><ShieldCheck size={16} /><span>No synthetic quotes, candles, token rows, or audit events are supplied.</span></div>
      <div><Database size={16} /><span>No decision or market-data result is treated as available without a native response.</span></div>
      <div><Activity size={16} /><span>Launch the installed desktop application to access its configured providers and durable local state.</span></div>
    </div>
    <p class="runtime-required-footnote">No native command, provider request, or trading action was sent.</p>
  </main>
{:else}
<div class="app-shell">
  <aside class:mobile-open={showMobileNav} class="sidebar">
    <div class="brand">
      <div class="brand-mark"><Activity size={22} strokeWidth={2.4} /></div>
      <div><div class="brand-name">QuantDinger</div><div class="brand-sub">AI TRADING OS</div></div>
      <button class="icon-button mobile-close" aria-label="Close navigation" onclick={() => showMobileNav = false}><X size={17} /></button>
    </div>

    <div class="nav-caption">WORKSPACE</div>
    <nav class="nav-list" aria-label="Primary navigation">
      {#each navItems as item}
        <button class:active={activeNav === item.label} class="nav-item" onclick={() => { void navigate(item.label); }}>
          <svelte:component this={item.icon} size={17} strokeWidth={1.8} />
          <span>{item.label}</span>
          {#if item.locked}<LockKeyhole size={13} class="nav-lock" />{/if}
          {#if item.label === 'Meme Coin Hunter'}<span class="new-tag">NEW</span>{/if}
        </button>
      {/each}
    </nav>

    <div class="sidebar-bottom">
      <div class="quick-panel">
        <div class="panel-label">QUICK ACTIONS</div>
        <button onclick={() => { void navigate('Strategy Builder'); }}><Plus size={14} /> New strategy</button>
        <button onclick={() => { void navigate('Backtesting'); }}><FlaskConical size={14} /> Run backtest</button>
        <button onclick={() => { void navigate('AI Agents'); }}><Play size={14} /> Manage agents</button>
        <button onclick={() => { void navigate('Tools & MCP'); }}><Terminal size={14} /> Open tools</button>
      </div>
      <div class="system-state"><span class="pulse-dot"></span><div><strong>Local engine</strong><small>Rust runtime connected</small></div><span class="system-version">v0.11.0</span></div>
    </div>
  </aside>

  <div class="app-main">
    <header class="topbar">
      <button class="icon-button mobile-menu" aria-label="Open navigation" onclick={() => showMobileNav = true}><Menu size={19} /></button>
      <div class="global-search"><Search size={16} /><input bind:value={searchQuery} placeholder="Search symbol, strategy, agent..." aria-label="Search market symbols" /><kbd><Command size={11} /> K</kbd></div>
      <div class="topbar-spacer"></div>
      <div class="runtime-chip"><span class="pulse-dot"></span>LOCAL ENGINE</div>
      <button class="icon-button" aria-label="Latest events" title="Latest events" onclick={() => { void navigate('Audit Log'); }}><Bell size={17} /></button>
      <button class="icon-button" aria-label="Refresh market data" title="Refresh market data" disabled={refreshBusy} onclick={refreshSnapshot}><RefreshCw size={17} class={refreshBusy ? 'spin' : ''} /></button>
      <div class="top-divider"></div>
      <button class="icon-button" aria-label="Settings" onclick={() => { void navigate('Settings'); }}><Settings size={17} /></button>
      <div class="profile-chip"><div class="avatar">T</div><div><strong>Trader</strong><small>LOCAL WORKSPACE</small></div><ChevronDown size={13} /></div>
    </header>

    {#if errorMessage && snapshot}
      <div class="flash flash-error" role="alert"><ShieldAlert size={17} /><span>{errorMessage}</span><button aria-label="Dismiss error" onclick={() => errorMessage = ''}><X size={15} /></button></div>
    {/if}
    {#if noticeMessage}
      <div class="flash flash-info" role="status"><CheckCircle2 size={17} /><span>{noticeMessage}</span><button aria-label="Dismiss notice" onclick={() => noticeMessage = ''}><X size={15} /></button></div>
    {/if}

    {#if loading}
      <div class="loading-state"><RefreshCw class="spin" size={24} /><p>Connecting to local trading runtime…</p></div>
    {:else if snapshot}
      <div class="content-grid">
        <main class="content-main">
          {#if activeNav === 'Home'}
            <div class="page-heading">
              <div><div class="eyebrow">OVERVIEW / LOCAL SESSION</div><h1>Good morning, Trader</h1><p>Inspect markets, strategies, and paper-execution health from one workspace.</p></div>
              <div class="heading-actions"><span class="date-label"><Clock3 size={13} /> {new Date().toLocaleDateString([], { weekday: 'short', month: 'short', day: 'numeric' })}</span><button class="button-secondary" onclick={refreshSnapshot} disabled={refreshBusy}><RefreshCw size={14} /> Sync data</button></div>
            </div>
            <div class="overview-strip">
              <span class="mode-pill"><span class="small-dot"></span>{snapshot.mode}</span>
              <span class:healthy={providerIsFresh} class:stale={!providerIsFresh} class="provider-pill"><span class="small-dot"></span>{snapshot.providerStatus}</span>
              <span class="muted-note">Updated {ageLabel(snapshot.providerUpdatedAtMs, clockNow)} · {snapshot.providerSource}</span>
              <button class="kill-button" class:engaged={snapshot.killSwitchEngaged} onclick={toggleKillSwitch}><ShieldAlert size={14} /> {snapshot.killSwitchEngaged ? 'KILL SWITCH ON' : 'Engage kill switch'}</button>
            </div>

            <section class="metric-grid">
              <article class="metric-card"><div class="metric-head"><span>Total portfolio value</span><Wallet size={15} /></div><div class="metric-value">{portfolioMarksFresh ? money(snapshot.equity) : 'Unavailable'}</div><div class="metric-foot portfolio-foot"><span class:positive={snapshot.dayChangePct >= 0} class:negative={snapshot.dayChangePct < 0}>{portfolioMarksFresh ? `${signedPct(snapshot.dayChangePct)} (UTC day)` : 'Latest portfolio marks are stale'}</span><span>Realized {money(snapshot.realizedPnl)} · Unrealized {portfolioMarksFresh ? money(snapshot.unrealizedPnl) : 'Unavailable'}</span></div></article>
              <article class="metric-card"><div class="metric-head"><span>Win rate (paper)</span><Activity size={15} /></div><div class="metric-value">{snapshot.closedTradeCount > 0 && snapshot.winRatePct !== null ? number(snapshot.winRatePct) : '—'}{#if snapshot.closedTradeCount > 0 && snapshot.winRatePct !== null}<small>%</small>{/if}</div><div class="metric-foot"><span class="neutral-value">{snapshot.closedTradeCount}</span><span>{snapshot.closedTradeCount === 1 ? 'closed paper trade' : 'closed paper trades'}</span></div></article>
              <article class="metric-card"><div class="metric-head"><span>Max drawdown</span><TrendingDown size={15} /></div><div class="metric-value" class:negative={snapshot.maxDrawdownPct < 0}>{number(snapshot.maxDrawdownPct)}<small>%</small></div><div class="metric-foot"><span>Peak-equity reference</span></div></article>
              <article class="metric-card"><div class="metric-head"><span>Active agents</span><Bot size={15} /></div><div class="metric-value">{snapshot.agents.filter((agent) => agent.status.startsWith('RUNNING')).length}<small> / {snapshot.agents.length}</small></div><div class="metric-foot"><span class="status-text">{snapshot.agents.length > 0 ? 'Agent state reported by runtime' : 'No agent sessions configured'}</span></div></article>
            </section>

            <section class="chart-order-grid">
              <article class="panel chart-panel">
                <div class="panel-heading chart-heading"><div class="asset-heading"><div class="asset-icon">₿</div><div><h2>{activeCandleRequest.symbol.replace('USDT', ' / USDT')}</h2><p>Spot market · {activeCandleRequest.interval} candles</p></div></div><div class="chart-tools"><button class="timeframe unavailable-timeframe" title="Only 5m candles are loaded" onclick={() => noticeMessage = '1m candles are not loaded yet; the chart remains on the verified 5m dataset.'}>1m</button><span class="timeframe active" title="Loaded candle interval">5m</span><button class="timeframe unavailable-timeframe" title="Only 5m candles are loaded" onclick={() => noticeMessage = '15m candles are not loaded yet; the chart remains on the verified 5m dataset.'}>15m</button><button class="timeframe unavailable-timeframe" title="Only 5m candles are loaded" onclick={() => noticeMessage = '1h candles are not loaded yet; the chart remains on the verified 5m dataset.'}>1h</button><button class="timeframe unavailable-timeframe" title="Only 5m candles are loaded" onclick={() => noticeMessage = '4h candles are not loaded yet; the chart remains on the verified 5m dataset.'}>4h</button><button class="timeframe unavailable-timeframe" title="Only 5m candles are loaded" onclick={() => noticeMessage = '1D candles are not loaded yet; the chart remains on the verified 5m dataset.'}>1D</button><button class="icon-button tiny" title="Technical indicators" aria-label="Technical indicators" onclick={() => { void navigate('Technical Analysis'); }}><Activity size={14} /></button><button class="icon-button tiny" title="Market data" aria-label="Market data" onclick={() => { void navigate('Market Data'); }}><Database size={14} /></button></div></div>
                <div class="chart-price-row"><div>{#if activeChartQuote && activeChartQuoteFresh}<strong>{money(activeChartQuote.price)}</strong><span class:positive={activeChartQuote.change24hPct >= 0} class:negative={activeChartQuote.change24hPct < 0}>{signedPct(activeChartQuote.change24hPct)}</span>{:else}<strong>Unavailable</strong><span class="muted-note">{activeChartQuote ? 'Stale quote · refresh required' : 'No validated quote'}</span>{/if}</div><div class="chart-source"><span class="small-dot" class:green-dot={chartSeriesResponse?.isLive}></span>{chartSeriesResponse?.source || chartSeriesError || 'No candle-series source'}</div></div>
                {#if lastCandleSummary}
                  <div class="chart-ohlc-row" aria-label={`Latest validated ${activeCandleRequest.interval} candle`}>
                    <span>O <strong>{number(lastCandleSummary.open, 2)}</strong></span><span>H <strong>{number(lastCandleSummary.high, 2)}</strong></span><span>L <strong>{number(lastCandleSummary.low, 2)}</strong></span><span>C <strong class:positive={lastCandleSummary.bullish} class:negative={!lastCandleSummary.bullish}>{number(lastCandleSummary.close, 2)}</strong></span><span>Vol <strong>{number(lastCandleSummary.volume, 2)}</strong></span><time>{new Date(lastCandleSummary.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</time>
                  </div>
                {:else}
                  <div class="chart-ohlc-row chart-ohlc-empty">No validated candle available.</div>
                {/if}
                <div class="chart-canvas">
                  {#if candles.length > 0}
                  <svg viewBox="0 0 1000 310" preserveAspectRatio="none" role="img" aria-label={`${activeCandleRequest.symbol} candle close chart. Price data source shown above.`}>
                    {#each [0, 1, 2, 3, 4] as tick}
                      <line x1="0" x2="1000" y1={20 + tick * 62} y2={20 + tick * 62} class="chart-gridline" />
                    {/each}
                    {#each [0, 1, 2, 3, 4, 5, 6] as tick}
                      <line x1={tick * 166} x2={tick * 166} y1="0" y2="300" class="chart-gridline vertical" />
                    {/each}
                    <path d={closePath} class="price-area" />
                    <path d={maSlowPath} class="ma-line ma-slow" />
                    <path d={maFastPath} class="ma-line ma-fast" />
                    {#each candles as candle, i}
                      <line x1={chartX(i)} x2={chartX(i)} y1={priceY(candle.high)} y2={priceY(candle.low)} class:up-candle={candle.close >= candle.open} class:down-candle={candle.close < candle.open} class="candle-wick" />
                      <rect x={chartX(i) - Math.max(1.3, 340 / candles.length / 2)} y={Math.min(priceY(candle.open), priceY(candle.close))} width={Math.max(2.2, 340 / candles.length)} height={Math.max(1.3, Math.abs(priceY(candle.open) - priceY(candle.close)))} class:up-candle={candle.close >= candle.open} class:down-candle={candle.close < candle.open} class="candle-body" />
                    {/each}
                  </svg>
                  <div class="chart-y-labels"><span>{money(chartMax, true)}</span><span>{money((chartMax + chartMin) / 2, true)}</span><span>{money(chartMin, true)}</span></div>
                  {:else}
                    <div class="empty-chart-state"><Database size={21}/><strong>{chartSeriesBusy ? 'Loading requested candle series…' : 'Candle series unavailable'}</strong><span>{chartSeriesError || activeSeriesCapability?.note || 'No candle data is loaded for this exact symbol and interval.'}</span></div>
                  {/if}
                </div>
                <div class="chart-volume-panel" aria-label="Volume bars from loaded candle data">
                  <div><span>Volume</span><small>Loaded OHLCV · {activeCandleRequest.interval}</small></div>
                  <svg viewBox="0 0 1000 40" preserveAspectRatio="none" role="img" aria-label={`${activeCandleRequest.interval} candle volume histogram`}>
                    {#each candles as candle, i}
                      <rect x={chartX(i) - Math.max(1.3, 350 / candles.length / 2)} y={38 - normalizedVolumeHeight(candle.volume, maxCandleVolume, 34, 2)} width={Math.max(2.1, 350 / candles.length)} height={normalizedVolumeHeight(candle.volume, maxCandleVolume, 34, 2)} class:up-candle={candle.close >= candle.open} class:down-candle={candle.close < candle.open} />
                    {/each}
                  </svg>
                </div>
                <div class="chart-footer"><div class="chart-legend"><span><i class="legend-dot fast"></i> SMA 9</span><span><i class="legend-dot slow"></i> SMA 21</span><span><i class="legend-dot price"></i> Close</span></div><div class="chart-times">{#if candles.length > 0}<span>{new Date(candles[0].time).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</span><span>{new Date(candles[Math.floor((candles.length - 1) / 2)].time).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</span><span>{new Date(candles[candles.length - 1].time).toLocaleString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}</span>{:else}<span>Time range unavailable</span>{/if}</div></div>
              </article>

              <article class="panel order-panel">
                <div class="panel-heading"><div><h2>Paper order</h2><p>All orders pass through Rust risk checks</p></div><span class="paper-badge">PAPER</span></div>
                {#if snapshot.killSwitchEngaged}<div class="risk-banner"><ShieldAlert size={15} /><span>Kill switch engaged. New orders are blocked.</span></div>{/if}
                <div class="side-switch"><button class:buy-active={orderSide === 'BUY'} onclick={() => orderSide = 'BUY'}>Buy / Long</button><button class:sell-active={orderSide === 'SELL'} onclick={() => orderSide = 'SELL'}>Sell / Reduce</button></div>
                <label class="field-label" for="order-instrument">Instrument</label>
                <select id="order-instrument" class="field-control" bind:value={selectedSymbol} aria-label="Order instrument" disabled={snapshot.quotes.length === 0}>
                  {#if snapshot.quotes.length === 0}<option value="">No validated quote available</option>{/if}
                  {#each snapshot.quotes as quote}<option value={quote.symbol}>{quote.symbol}</option>{/each}
                </select>
                {#if !orderChartAligned}<div class="chart-order-mismatch" role="status"><ShieldAlert size={13}/><span>Chart: {activeCandleRequest.symbol} {activeCandleRequest.interval}. {selectedSymbol} has no matching candle series. Any paper-order request is evaluated against the current validated {selectedSymbol} quote only.</span></div>{/if}
                <span class="field-label">Order type</span><div class="order-type-control" role="group" aria-label="Order type"><button class:chosen={orderType === 'Market'} onclick={() => orderType = 'Market'}>Market</button><button class="disabled-type" title="Limit orders are not supported by this broker">Limit <LockKeyhole size={11} /></button><button class="disabled-type" title="Stop orders are not supported by this broker">Stop <LockKeyhole size={11} /></button></div>
                <label class="field-label" for="quantity">Quantity</label><div class="input-with-suffix"><input id="quantity" type="number" min="0.00000001" step="any" bind:value={quantityText} /><span>{selectedSymbol.replace('USDT', '')}</span></div>
                <div class="quantity-presets"><button disabled={!currentQuoteFresh} onclick={() => { if (currentQuoteFresh && currentQuote) quantityText = String((snapshot?.riskLimits.maxOrderNotional ?? 500) * 0.25 / currentQuote.price); }}>25% cap</button><button disabled={!currentQuoteFresh} onclick={() => { if (currentQuoteFresh && currentQuote) quantityText = String((snapshot?.riskLimits.maxOrderNotional ?? 500) * 0.5 / currentQuote.price); }}>50% cap</button><button disabled={!currentQuoteFresh} onclick={() => { if (currentQuoteFresh && currentQuote) quantityText = String((snapshot?.riskLimits.maxOrderNotional ?? 500) * 0.75 / currentQuote.price); }}>75% cap</button><button disabled={!currentQuoteFresh} onclick={() => { if (currentQuoteFresh && currentQuote) quantityText = String((snapshot?.riskLimits.maxOrderNotional ?? 500) / currentQuote.price); }}>Max</button></div>
                <div class="estimate-list"><div><span>Reference price</span><strong>{currentQuoteFresh && currentQuote ? money(currentQuote.price) : 'Unavailable'}</strong></div><div><span>Estimated notional</span><strong>{currentQuoteFresh && currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? money(estimatedNotional) : 'Unavailable'}</strong></div><div><span>Estimated fee</span><strong>{currentQuoteFresh && currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? money(estimatedFee) : 'Unavailable'}</strong></div><div><span>Order cap</span><strong>{money(snapshot.riskLimits.maxOrderNotional)}</strong></div></div>
                <button class="submit-order" class:sell-submit={orderSide === 'SELL'} disabled={orderBusy || snapshot.killSwitchEngaged || !desktop || orderType !== 'Market' || !currentQuoteFresh || !Number.isFinite(orderQuantity) || orderQuantity <= 0} onclick={submitOrder}>{#if orderBusy}<RefreshCw size={15} class="spin" /> Processing…{:else}<Zap size={15} /> Simulate {orderSide} order{/if}</button>
                <div class="execution-hint"><ShieldCheck size={13} /> Paper broker only · No live order endpoint</div>
                {#if orderResult && showRiskChecks}<div class="risk-check-list"><div class="risk-check-title">Last order · {orderResult.status}</div>{#each orderResult.riskChecks as check}<div class="risk-check"><span class:check-pass={check.passed} class:check-fail={!check.passed}>{#if check.passed}<CheckCircle2 size={13} />{:else}<X size={13} />{/if}</span><div><strong>{check.rule}</strong><small>{check.detail}</small></div></div>{/each}</div>{/if}
              </article>
            </section>

            <section class="panel screener-panel">
              <div class="panel-heading screener-heading"><div class="section-title-with-icon"><div class="section-icon purple"><Sparkles size={16} /></div><div><h2>Promoted Token Feed</h2><p>{snapshot.screenerProviderStatus} · {ageLabel(snapshot.screenerProviderUpdatedAtMs, clockNow)} · promoted-feed bias applies</p></div></div><button class="button-secondary compact" onclick={() => { void navigate('Meme Coin Hunter'); }}>Open Meme Coin Hunter <ExternalLink size={13} /></button></div>
              <div class="provider-risk-disclosure"><ShieldAlert size={14} /><span>Discovery data is provider-backed but promotion-biased; risk scores are uncalibrated research heuristics, not safety certification or return probabilities.</span></div>
              <div class="table-wrap"><table class="data-table screener-table"><thead><tr><th>#</th><th>Asset</th><th>Chain</th><th>Price</th><th>24h</th><th>Volume</th><th>Momentum</th><th>Risk</th><th>Signal</th><th></th></tr></thead><tbody>
                {#if sortedScreener.length === 0}<tr><td colspan="10"><div class="empty-state"><Database size={22}/><strong>{screenerSnapshotFresh ? 'No validated screener observations' : 'Screener snapshot stale or disconnected'}</strong><span>{screenerSnapshotFresh ? 'Refresh the public feed to load provider-backed token observations. No sample rows are substituted.' : 'Stored rows are retained for audit/history only and are not presented as current evidence. Refresh the public feed and verify its timestamp before relying on screener values.'}</span></div></td></tr>{/if}
                {#each sortedScreener.slice(0, 5) as token, i}<tr><td class="rank-cell">{i + 1}</td><td><div class="coin-cell"><div class={`coin-logo coin-${token.symbol.toLowerCase()}`}>{token.symbol.slice(0, 1)}</div><div><strong>{token.symbol}</strong><small>{token.name}</small></div></div></td><td><span class="chain-tag">{token.chain}</span></td><td>{money(token.price)}</td><td class="positive">{signedPct(token.change24hPct)}</td><td>{money(token.volume24h, true)}</td><td><div class="score-cell"><strong>{token.momentumScore}</strong><div class="score-track"><span style={`width:${token.momentumScore}%`}></span></div></div></td><td><span class:low-risk={token.riskScore < 40} class:medium-risk={token.riskScore >= 40 && token.riskScore < 65} class:high-risk={token.riskScore >= 65} class="risk-score">{token.riskScore}</span></td><td><span class="signal-tag" class:signal-risk={token.riskScore >= 55}>{token.riskScore >= 55 ? 'High risk' : token.status}</span></td><td><button class="table-action" title={`Show ${token.symbol} evidence`} onclick={() => { void navigate('Meme Coin Hunter'); }}>Open Hunter</button></td></tr>{/each}
              </tbody></table></div>
              <div class="panel-bottom-note"><span><span class="small-dot amber-dot"></span>Unknown risk evidence is not equivalent to low risk.</span><button class="text-action" onclick={() => { void navigate('Meme Coin Hunter'); }}>View hunter analysis <ArrowUpRight size={13} /></button></div>
            </section>

            <section class="bottom-grid">
              <article class="panel mini-table-panel"><div class="panel-heading"><div><h2>Top performers (24h)</h2><p>Quotes available in the connected crypto universe</p></div><button class="icon-button tiny" onclick={() => { void navigate('Market Data'); }}><MoreHorizontal size={16} /></button></div><div class="mini-rows">{#if freshQuotes.length === 0}<div class="empty-small"><Database size={18}/><span>No fresh validated quote observations.</span></div>{:else}{#each [...freshQuotes].sort((a, b) => b.change24hPct - a.change24hPct).slice(0, 5) as quote}<button class="mini-row" onclick={() => selectSymbol(quote.symbol)}><span class="mini-coin">{quote.symbol.slice(0, 1)}</span><span class="mini-row-name"><strong>{quote.symbol}</strong><small>{money(quote.price)}</small></span><span class:positive={quote.change24hPct >= 0} class:negative={quote.change24hPct < 0}>{signedPct(quote.change24hPct)}</span></button>{/each}{/if}</div></article>
              <article class="panel mini-table-panel"><div class="panel-heading"><div><h2>Recent paper trades</h2><p>Newest simulated fills and rejects</p></div><button class="text-action" onclick={() => { void navigate('Paper Trading'); }}>All orders</button></div>{#if snapshot.orders.length}<div class="trade-list">{#each snapshot.orders.slice(0, 5) as order}<div class="trade-row"><span class:positive={order.side === 'BUY'} class:negative={order.side === 'SELL'} class="trade-side">{order.side}</span><span><strong>{order.symbol}</strong><small>{timeLabel(order.createdAtMs)} · {order.status}</small></span><span class="trade-value">{money(order.notional)}</span></div>{/each}</div>{:else}<div class="empty-small"><Activity size={18} /><span>No paper orders yet.</span></div>{/if}</article>
              <article class="panel mini-table-panel performance-panel"><div class="panel-heading"><div><h2>Strategy performance</h2><p>{backtestResult ? `${backtestResult.strategyName} · historical simulation` : 'No backtest curve loaded'}</p></div><Gauge size={16} class="muted-icon" /></div>
                {#if backtestResult && backtestResult.equityCurve.length > 1}
                  <div class="strategy-performance-value"><strong class:positive={backtestResult.totalReturnPct >= 0} class:negative={backtestResult.totalReturnPct < 0}>{signedPct(backtestResult.totalReturnPct)}</strong><span>Return</span><strong>{number(backtestResult.sharpeRatio)}</strong><span>Sharpe</span><strong class:negative={backtestResult.maxDrawdownPct < 0}>{signedPct(backtestResult.maxDrawdownPct)}</strong><span>Max DD</span></div>
                  <div class="strategy-plot"><svg viewBox="0 0 300 80" preserveAspectRatio="none" role="img" aria-label="Backtest strategy equity compared with buy and hold benchmark"><line x1="0" y1="14" x2="300" y2="14"/><line x1="0" y1="40" x2="300" y2="40"/><line x1="0" y1="66" x2="300" y2="66"/><path d={backtestCurvePath('benchmark')} class="benchmark-curve"/><path d={backtestCurvePath('equity')} class="strategy-curve"/></svg><div class="strategy-legend"><span><i class="legend-dot fast"></i> Strategy equity</span><span><i class="legend-dot slow"></i> Buy & hold</span></div></div>
                {:else}
                  <div class="strategy-empty"><FlaskConical size={21}/><strong>No strategy curve yet</strong><span>Run a backtest to compare strategy equity with the buy-and-hold benchmark.</span><button class="text-action" onclick={() => { void navigate('Backtesting'); }}>Open backtesting <ArrowUpRight size={12}/></button></div>
                {/if}
              </article>
            </section>

          {:else if activeNav === 'Trading' || activeNav === 'Paper Trading'}
            <div class="page-heading"><div><div class="eyebrow">EXECUTION / SIMULATION</div><h1>{activeNav === 'Trading' ? 'Trading terminal' : 'Paper trading'}</h1><p>Market orders are simulated locally; every intent is evaluated by the deterministic Rust risk gateway.</p></div><span class="mode-pill"><span class="small-dot"></span> PAPER ONLY</span></div>
            <div class="notice-banner"><ShieldCheck size={18} /><div><strong>No real funds are connected</strong><p>Limit and stop orders are not implemented. Market orders use the latest quote plus configured slippage and fees.</p></div></div>
            <section class="chart-order-grid full-trading-grid"><article class="panel chart-panel"><div class="panel-heading"><div><h2>Market chart · {activeCandleRequest.symbol} · {activeCandleRequest.interval}</h2><p>{chartSeriesResponse?.source || chartSeriesError || snapshot.providerSource}</p></div><span class="paper-badge">{providerIsFresh ? 'FRESH' : 'STALE'}</span></div><div class="chart-price-row">{#if activeChartQuote && activeChartQuoteFresh}<strong>{money(activeChartQuote.price)}</strong><span class:positive={activeChartQuote.change24hPct >= 0} class:negative={activeChartQuote.change24hPct < 0}>{signedPct(activeChartQuote.change24hPct)}</span>{:else}<strong>Unavailable</strong><span class="muted-note">{activeChartQuote ? 'Stale quote · refresh required' : 'No validated quote'}</span>{/if}</div><div class="chart-canvas large-chart">{#if candles.length > 0}<svg viewBox="0 0 1000 310" preserveAspectRatio="none" role="img" aria-label={`${activeCandleRequest.symbol} ${activeCandleRequest.interval} market close chart`}><path d={closePath} class="price-area"/><path d={maSlowPath} class="ma-line ma-slow"/><path d={maFastPath} class="ma-line ma-fast"/>{#each candles as candle, i}<line x1={chartX(i)} x2={chartX(i)} y1={priceY(candle.high)} y2={priceY(candle.low)} class:up-candle={candle.close >= candle.open} class:down-candle={candle.close < candle.open} class="candle-wick"/><rect x={chartX(i) - 1.6} y={Math.min(priceY(candle.open), priceY(candle.close))} width="3.2" height={Math.max(1.5, Math.abs(priceY(candle.open) - priceY(candle.close)))} class:up-candle={candle.close >= candle.open} class:down-candle={candle.close < candle.open} class="candle-body"/>{/each}</svg>{:else}<div class="empty-chart-state"><Database size={21}/><strong>{chartSeriesBusy ? 'Loading requested candle series…' : 'Candle series unavailable'}</strong><span>{chartSeriesError || activeSeriesCapability?.note || 'No candle data is loaded for this exact symbol and interval.'}</span></div>{/if}</div><div class="chart-footer"><div class="chart-legend"><span><i class="legend-dot fast"></i> SMA 9</span><span><i class="legend-dot slow"></i> SMA 21</span></div><span class="muted-note">Candles: {candles.length} · source is shown above</span></div></article>
              <article class="panel order-panel expanded-order"><div class="panel-heading"><div><h2>New paper order</h2><p>Estimated exposure and fees before submit</p></div><ShieldCheck size={17} /></div><div class="side-switch"><button class:buy-active={orderSide === 'BUY'} onclick={() => orderSide = 'BUY'}>Buy</button><button class:sell-active={orderSide === 'SELL'} onclick={() => orderSide = 'SELL'}>Sell</button></div><label class="field-label" for="order-instrument-2">Instrument</label><select id="order-instrument-2" class="field-control" bind:value={selectedSymbol} disabled={snapshot.quotes.length === 0}>{#if snapshot.quotes.length === 0}<option value="">No validated quote available</option>{/if}{#each snapshot.quotes as quote}<option value={quote.symbol}>{quote.symbol} · {quoteIsFresh(quote, clockNow) ? money(quote.price) : 'quote unavailable'}</option>{/each}</select>{#if !orderChartAligned}<div class="chart-order-mismatch" role="status"><ShieldAlert size={13}/><span>Chart is {activeCandleRequest.symbol} {activeCandleRequest.interval}; {selectedSymbol} has quote-only coverage. Paper order price uses {selectedSymbol} quote, not the chart candle series.</span></div>{/if}<label class="field-label" for="quantity-2">Quantity</label><input id="quantity-2" class="field-control" type="number" min="0.00000001" step="any" bind:value={quantityText}/><div class="estimate-list"><div><span>Notional</span><strong>{currentQuoteFresh && currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? money(estimatedNotional) : 'Unavailable'}</strong></div><div><span>Fee estimate</span><strong>{currentQuoteFresh && currentQuote && Number.isFinite(orderQuantity) && orderQuantity > 0 ? money(estimatedFee) : 'Unavailable'}</strong></div></div><button class="submit-order" disabled={orderBusy || snapshot.killSwitchEngaged || !desktop || !currentQuoteFresh || !Number.isFinite(orderQuantity) || orderQuantity <= 0} onclick={submitOrder}><Zap size={15}/> Submit paper order</button>{#if !currentQuoteFresh}<p class="last-order-label quote-blocked-note">No fresh validated quote is available for the selected instrument. Refresh verified public market data before submitting.</p>{/if}{#if orderResult}<p class="last-order-label">Last result: <strong>{orderResult.status}</strong></p>{/if}</article></section>
            <section class="panel table-panel"><div class="panel-heading"><div><h2>Open positions</h2><p>Current marks require a live quote within 120 seconds; historical cached values are not shown as current.</p></div></div>{#if snapshot.positions.length}<div class="table-wrap"><table class="data-table"><thead><tr><th>Symbol</th><th>Quantity</th><th>Average entry</th><th>Mark</th><th>Notional</th><th>Unrealized PnL</th></tr></thead><tbody>{#each snapshot.positions as position}<tr><td><strong>{position.symbol}</strong></td><td>{number(position.quantity, 8)}</td><td>{money(position.averageEntry)}</td><td>{positionMarkFresh(position.symbol, clockNow) ? money(position.markPrice) : 'Unavailable · stale mark'}</td><td>{positionMarkFresh(position.symbol, clockNow) ? money(position.notional) : 'Unavailable'}</td><td class:positive={position.unrealizedPnl >= 0} class:negative={position.unrealizedPnl < 0}>{positionMarkFresh(position.symbol, clockNow) ? money(position.unrealizedPnl) : 'Unavailable'}</td></tr>{/each}</tbody></table></div>{:else}<div class="empty-state"><Wallet size={24}/><strong>No open positions</strong><span>Paper fills will appear here after successful orders.</span></div>{/if}</section>
            <section class="panel table-panel"><div class="panel-heading"><div><h2>Order ledger</h2><p>Recent persisted paper orders with attached risk evaluations</p></div></div>{#if snapshot.orders.length}<div class="table-wrap"><table class="data-table"><thead><tr><th>Time</th><th>Symbol</th><th>Side</th><th>Qty</th><th>Fill price</th><th>Notional</th><th>Fee</th><th>Status</th></tr></thead><tbody>{#each snapshot.orders as order}<tr><td>{timeLabel(order.createdAtMs)}</td><td><strong>{order.symbol}</strong></td><td class:positive={order.side === 'BUY'} class:negative={order.side === 'SELL'}>{order.side}</td><td>{number(order.quantity, 8)}</td><td>{order.fillPrice ? money(order.fillPrice) : '—'}</td><td>{money(order.notional)}</td><td>{money(order.fee)}</td><td><span class:status-good={order.status.startsWith('FILLED')} class:status-bad={order.status === 'REJECTED'} class="status-tag">{order.status}</span></td></tr>{/each}</tbody></table></div>{:else}<div class="empty-state"><Activity size={24}/><strong>No orders recorded</strong><span>Rejected intents also appear here with their risk checks.</span></div>{/if}</section>

          {:else if activeNav === 'Strategy Builder' || activeNav === 'Backtesting'}
            <div class="page-heading"><div><div class="eyebrow">RESEARCH / REPRODUCIBILITY</div><h1>{activeNav === 'Backtesting' ? 'Backtesting laboratory' : 'Strategy builder'}</h1><p>Configure an auditable baseline strategy and evaluate it on the available candle set.</p></div><span class="mode-pill"><FlaskConical size={13}/> HISTORICAL SIMULATION</span></div>
            <div class="notice-banner"><FlaskConical size={18}/><div><strong>Baseline strategy: simple moving-average crossover</strong><p>The Rust runner now automatically adds fee/slippage shock scenarios and chronological out-of-sample windows. This is a robustness check, not an optimizer or profit guarantee; each out-of-sample window starts from fresh simulated capital.</p></div></div>
            <section class="panel config-panel"><div class="panel-heading"><div><h2>Run configuration</h2><p>Input parameters are included in the result and warnings.</p></div><span class="version-tag">sma-crossover / v1</span></div><div class="config-grid"><div class="config-field"><label for="bt-symbol">Instrument</label><select id="bt-symbol" disabled><option>BTCUSDT</option></select><small>Only BTCUSDT candles are stored in this build.</small></div><div class="config-field"><label for="fast">Fast SMA period</label><input id="fast" type="number" min="2" max="100" bind:value={fastPeriod}/><small>Minimum 2 candles.</small></div><div class="config-field"><label for="slow">Slow SMA period</label><input id="slow" type="number" min="3" max="300" bind:value={slowPeriod}/><small>Must exceed fast period.</small></div><div class="config-field"><label for="capital">Initial capital (USD)</label><input id="capital" type="number" min="1" step="100" bind:value={initialCapital}/><small>Simulated account starting value.</small></div></div><div class="config-bottom"><div class="assumption-chips"><span>Fee {number(snapshot.riskLimits.feeBps)} bps</span><span>Slippage {number(snapshot.riskLimits.slippageBps)} bps</span><span>{candles.length} candles available</span></div><button class="button-primary" onclick={executeBacktest} disabled={backtestBusy || !desktop}>{#if backtestBusy}<RefreshCw size={15} class="spin"/> Running…{:else}<Play size={15}/> Run backtest{/if}</button></div></section>
            {#if backtestResult}
              <section class="metric-grid backtest-metrics"><article class="metric-card"><div class="metric-head"><span>Total return</span><TrendingUp size={15}/></div><div class="metric-value" class:positive={backtestResult.totalReturnPct >= 0} class:negative={backtestResult.totalReturnPct < 0}>{signedPct(backtestResult.totalReturnPct)}</div><div class="metric-foot"><span>Final equity {money(backtestResult.finalEquity)}</span></div></article><article class="metric-card"><div class="metric-head"><span>Max drawdown</span><TrendingDown size={15}/></div><div class="metric-value">{number(backtestResult.maxDrawdownPct)}<small>%</small></div><div class="metric-foot"><span>From prior equity peak</span></div></article><article class="metric-card"><div class="metric-head"><span>Win rate (paper)</span><Activity size={15}/></div><div class="metric-value">{backtestResult.tradeCount > 0 && backtestResult.winRatePct !== null ? number(backtestResult.winRatePct) : '—'}{#if backtestResult.tradeCount > 0 && backtestResult.winRatePct !== null}<small>%</small>{/if}</div><div class="metric-foot"><span>{backtestResult.tradeCount} closed trades</span></div></article><article class="metric-card"><div class="metric-head"><span>Profit factor</span><Gauge size={15}/></div><div class="metric-value">{backtestResult.grossProfit > 0 && backtestResult.grossLoss === 0 ? '∞' : backtestResult.profitFactor == null ? '—' : number(backtestResult.profitFactor, 3)}</div><div class="metric-foot"><span>Gross wins / gross losses</span></div></article></section>
              <section class="panel result-panel"><div class="panel-heading"><div><h2>Backtest report</h2><p>{backtestResult.strategyName} · {backtestResult.candlesUsed} candles · {backtestResult.dataSource}</p></div><div class="heading-actions"><span class="status-good status-tag">COMPLETED</span><button class="button-secondary compact" onclick={exportBacktestJson}><Download size={13}/> Export JSON</button></div></div><div class="result-strip"><div><small>Sharpe (rough)</small><strong>{number(backtestResult.sharpeRatio, 3)}</strong></div><div><small>Fees</small><strong>{number(backtestResult.feeBps)} bps</strong></div><div><small>Slippage</small><strong>{number(backtestResult.slippageBps)} bps</strong></div><div><small>Run ID</small><strong class="run-id">{backtestResult.id.slice(0, 12)}…</strong></div></div><details class="dataset-provenance"><summary>Dataset fingerprint and input range</summary><div class="dataset-provenance-grid"><div><small>SHA-256 · canonical encoding v{backtestResult.datasetEncodingVersion}</small><code>{backtestResult.datasetSha256}</code></div><div><small>Input candle range (UTC)</small><strong>{isoDateTimeLabel(backtestResult.datasetStartMs)} — {isoDateTimeLabel(backtestResult.datasetEndMs)}</strong></div><div><small>Median candle interval</small><strong>{number(backtestResult.medianCandleIntervalMs)} ms</strong></div></div><p>Matching fingerprints identify identical candle bytes under this encoding; they do not prove provider authenticity or make the run fully replayable without the original candles.</p></details><div class="warnings-list"><h3>Interpretation and caveats</h3>{#each backtestResult.warnings as warning}<div><ShieldAlert size={14}/><span>{warning}</span></div>{/each}</div></section>
              <section class="panel table-panel"><div class="panel-heading"><div><h2>Execution-cost sensitivity</h2><p>Same signal logic and candle set; only simulated fees/slippage change. Return delta is measured in percentage points versus the baseline.</p></div><span class="mode-pill">2 COST SHOCKS</span></div>{#if backtestResult.costStressScenarios.length}<div class="table-wrap"><table class="data-table"><thead><tr><th>Scenario</th><th>Fee</th><th>Slippage</th><th>Total return</th><th>Δ vs baseline</th><th>Max drawdown</th><th>Trades</th></tr></thead><tbody>{#each backtestResult.costStressScenarios as scenario}<tr><td><strong>{scenario.name}</strong>{#if scenario.feeCapReached || scenario.slippageCapReached}<small class="negative"> · cost cap reached</small>{/if}</td><td>{number(scenario.feeBps, 2)} bps</td><td>{number(scenario.slippageBps, 2)} bps</td><td class:positive={scenario.totalReturnPct >= 0} class:negative={scenario.totalReturnPct < 0}>{signedPct(scenario.totalReturnPct)}</td><td class:positive={scenario.returnDeltaPercentagePoints >= 0} class:negative={scenario.returnDeltaPercentagePoints < 0}>{scenario.returnDeltaPercentagePoints > 0 ? '+' : ''}{number(scenario.returnDeltaPercentagePoints, 3)} pp</td><td>{number(scenario.maxDrawdownPct, 3)}%</td><td>{scenario.tradeCount}</td></tr>{/each}</tbody></table></div>{:else}<div class="empty-state"><strong>No cost stress data returned</strong><span>Check the validation report before interpreting this run.</span></div>{/if}<p class="table-footnote">Shocks use deterministic fee/slippage floors and multipliers capped by the engine's supported limits. They do not model bid/ask spread, market impact, order-book depth, latency, or exchange-specific fill quality.</p></section>

              <section class="panel table-panel"><div class="panel-heading"><div><h2>Out-of-sample stability</h2><p>Chronological fixed-size windows; prior bars are used only to warm up the indicators. No positions carry over between windows.</p></div><span class="mode-pill">{backtestResult.outOfSampleFolds.length} FOLDS</span></div>{#if backtestResult.outOfSampleFolds.length}<div class="result-strip"><div><small>Positive windows</small><strong>{backtestResult.outOfSampleFolds.filter((fold) => fold.totalReturnPct > 0).length} / {backtestResult.outOfSampleFolds.length}</strong></div><div><small>Worst window</small><strong>{signedPct(Math.min(...backtestResult.outOfSampleFolds.map((fold) => fold.totalReturnPct)))}</strong></div><div><small>Median window return</small><strong>{number(medianValue(backtestResult.outOfSampleFolds.map((fold) => fold.totalReturnPct)), 3)}%</strong></div><div><small>Bars / window</small><strong>{backtestResult.outOfSampleFolds[0].candlesEvaluated}</strong></div></div><div class="table-wrap"><table class="data-table"><thead><tr><th>Fold</th><th>UTC range</th><th>Return</th><th>Max drawdown</th><th>Sharpe*</th><th>Trades</th></tr></thead><tbody>{#each backtestResult.outOfSampleFolds as fold}<tr><td><strong>OOS {fold.foldIndex}</strong></td><td>{isoDateTimeLabel(fold.startMs)} — {isoDateTimeLabel(fold.endMs)}</td><td class:positive={fold.totalReturnPct >= 0} class:negative={fold.totalReturnPct < 0}>{signedPct(fold.totalReturnPct)}</td><td>{number(fold.maxDrawdownPct, 3)}%</td><td>{number(fold.sharpeRatio, 3)}</td><td>{fold.tradeCount}</td></tr>{/each}</tbody></table></div><p class="table-footnote">*Sharpe is a rough estimate on each short window. The partial tail after the last complete window is excluded; only the most recent 12 complete windows are reported. Fold results are not parameter-optimized.</p>{:else}<div class="empty-state"><strong>Not enough history for out-of-sample windows</strong><span>At least two complete test windows of max(2 × slow SMA period, 30) bars plus indicator warm-up are required. The main backtest alone should not be treated as evidence of future robustness.</span></div>{/if}</section>
            {:else}<section class="panel empty-result"><div class="empty-state"><FlaskConical size={28}/><strong>No backtest in this session</strong><span>Set the parameters and run a test. The native Rust backtest runner did not return a result.</span></div></section>{/if}

          {:else if activeNav === 'Research Lab'}
            <div class="page-heading"><div><div class="eyebrow">RESEARCH / REPRODUCIBLE EXPERIMENTS</div><h1>Research Lab</h1><p>Persistent hypotheses, native historical evidence, independent robustness checks, and a deterministic research gate.</p></div><span class:status-good={researchIntegrity?.valid} class:status-bad={!researchIntegrity?.valid} class="status-tag">{researchIntegrity?.valid ? 'LEDGER VERIFIED' : researchWorkspaceBusy ? 'VERIFYING' : 'NOT VERIFIED'}</span></div>
            <div class="notice-banner"><ShieldCheck size={18}/><div><strong>Research approval is not execution approval</strong><p>Only the validated BTCUSDT 5m native candle series is currently supported. Rust executes the SMA backtest, walk-forward review, moving-block bootstrap and risk gate. No LLM output can place an order or change account state.</p></div></div>
            {#if researchErrorMessage}<div class="warnings-list" role="alert"><div><ShieldAlert size={14}/><span>{researchErrorMessage}</span></div></div>{/if}
            {#if researchNoticeMessage}<div class="notice-banner"><CheckCircle2 size={17}/><div><strong>Research workspace</strong><p>{researchNoticeMessage}</p></div></div>{/if}
            <section class="panel config-panel">
              <div class="panel-heading"><div><h2>Register a falsifiable hypothesis</h2><p>Hypothesis revisions are recorded separately from the account ledger and cannot mutate orders, balances or positions.</p></div><span class="version-tag">{researchHypotheses.length} / 200 RECORDS</span></div>
              <div class="settings-grid">
                <div class="config-field"><label for="research-title">Hypothesis title</label><input id="research-title" maxlength="160" bind:value={researchHypothesisTitle} placeholder="Volatility contraction continuation"/><small>Required, max 160 characters.</small></div>
                <div class="config-field"><label for="research-symbol">Instrument / timeframe</label><input id="research-symbol" value="BTCUSDT · 5m" disabled/><small>Fixed to the only exact native candle series currently supported.</small></div>
                <div class="config-field" style="grid-column: 1 / -1;"><label for="research-thesis">Testable thesis</label><textarea id="research-thesis" rows="3" maxlength="8000" bind:value={researchHypothesisThesis} placeholder="State the observable condition, expected behaviour and how the hypothesis can be falsified."></textarea><small>Required, max 8,000 characters. Do not encode execution instructions here.</small></div>
              </div>
              <div class="config-bottom"><span class="muted-note">Storage: isolated SQLite research workspace · immutable run cards · SHA-256 event chain</span><button class="button-primary" onclick={createResearchHypothesisRecord} disabled={researchWorkspaceBusy || researchHypothesisCreateBusy || !researchHypothesisTitle.trim() || !researchHypothesisThesis.trim()}>{#if researchHypothesisCreateBusy}<RefreshCw size={14} class="spin"/> Saving…{:else}<Plus size={14}/> Save hypothesis{/if}</button></div>
            </section>
            {#if researchHypotheses.some((hypothesis) => hypothesis.id === selectedResearchHypothesisId && hypothesis.status !== 'INVALIDATED')}
              <section class="panel config-panel">
                <div class="panel-heading"><div><h2>Hypothesis lifecycle control</h2><p>Invalidation is revision-checked, audited and irreversible. It does not delete or rewrite prior run evidence.</p></div><span class="mode-pill">DESTRUCTIVE / AUDITED</span></div>
                <div class="settings-grid">
                  <div class="config-field" style="grid-column: 1 / -1;"><label for="research-invalidation-reason">Reason for invalidation</label><textarea id="research-invalidation-reason" rows="2" maxlength="2000" bind:value={researchInvalidationReason} placeholder="Document the evidence or methodology issue that invalidates this hypothesis."></textarea><small>Required, max 2,000 characters. The reason is written to the append-only research event ledger.</small></div>
                </div>
                <div class="config-bottom"><span class="muted-note">Selected revision: r{researchHypotheses.find((hypothesis) => hypothesis.id === selectedResearchHypothesisId)?.revision ?? '—'}</span><button class="button-secondary compact" onclick={invalidateSelectedResearchHypothesis} disabled={researchInvalidationBusy || researchWorkspaceBusy || !researchInvalidationReason.trim()}>{#if researchInvalidationBusy}<RefreshCw size={13} class="spin"/> Invalidating…{:else}<X size={13}/> Invalidate hypothesis{/if}</button></div>
              </section>
            {/if}
            <section class="panel config-panel">
              <div class="panel-heading"><div><h2>Execute research committee</h2><p>Runs the production Rust backtest on native stored candles. Goal text is metadata, not an AI-generated strategy compiler.</p></div><span class="mode-pill"><FlaskConical size={13}/> DETERMINISTIC DAG</span></div>
              <div class="settings-grid">
                <div class="config-field" style="grid-column: 1 / -1;"><label for="research-goal">Research objective</label><textarea id="research-goal" rows="2" maxlength="2000" bind:value={researchGoal}></textarea><small>Describe the claim to evaluate; strategy logic remains explicitly parameterized below.</small></div>
                <div class="config-field"><label for="research-hypothesis">Linked hypothesis</label><select id="research-hypothesis" bind:value={selectedResearchHypothesisId}><option value="">No linked hypothesis</option>{#each researchHypotheses.filter((hypothesis) => hypothesis.status !== 'INVALIDATED') as hypothesis}<option value={hypothesis.id}>{hypothesis.title} · r{hypothesis.revision}</option>{/each}</select><small>Instrument and interval must match exactly.</small></div>
                <div class="config-field"><label for="research-fast">Fast SMA period</label><input id="research-fast" type="number" min="2" max="299" step="1" bind:value={researchFastPeriod}/><small>Must be lower than the slow period.</small></div>
                <div class="config-field"><label for="research-slow">Slow SMA period</label><input id="research-slow" type="number" min="3" max="300" step="1" bind:value={researchSlowPeriod}/><small>At least fast + 1.</small></div>
                <div class="config-field"><label for="research-capital">Initial capital (USD)</label><input id="research-capital" type="number" min="1" max="1000000000000" step="100" bind:value={researchInitialCapital}/><small>Backtest-only starting notional.</small></div>
                <div class="config-field"><label for="research-dd">Maximum drawdown (%)</label><input id="research-dd" type="number" min="0.1" max="100" step="0.1" bind:value={researchMaxDrawdownPct}/><small>Used by observed and bootstrap risk gates.</small></div>
                <div class="config-field"><label for="research-oos">Minimum positive OOS fold ratio</label><input id="research-oos" type="number" min="0" max="1" step="0.05" bind:value={researchMinimumPositiveOosRatio}/><small>Fraction between 0 and 1.</small></div>
                <div class="config-field"><label for="research-sharpe">Minimum Sharpe ratio</label><input id="research-sharpe" type="number" min="-10" max="20" step="0.1" bind:value={researchMinimumSharpeRatio}/><small>Uses the existing backtest metric.</small></div>
                <div class="config-field" style="grid-column: 1 / -1;"><label for="research-tournament"><input id="research-tournament" type="checkbox" bind:checked={researchStrategyTournament}/> Run nested walk-forward parameter tournament</label><small>Each of 3 chronological folds selects from a bounded SMA neighbourhood using training data only, then evaluates the selected parameters on the next unseen block. Candidate search is capped at 25 configurations and a 50-million candle-evaluation work budget.</small></div>
              </div>
              <div class="config-bottom"><div class="assumption-chips"><span>BTCUSDT · 5m</span><span>{chartSeriesResponse?.candles.length ?? 0} candles visible</span><span>Fees {number(snapshot.riskLimits.feeBps)} bps</span><span>Slippage {number(snapshot.riskLimits.slippageBps)} bps</span></div><button class="button-primary" onclick={executeResearchRun} disabled={researchWorkflowBusy || researchWorkspaceBusy || !chartSeriesResponse || !researchGoal.trim() || Number(researchFastPeriod) < 2 || Number(researchSlowPeriod) <= Number(researchFastPeriod)}>{#if researchWorkflowBusy}<RefreshCw size={14} class="spin"/> Running committee…{:else}<Play size={14}/> Execute research workflow{/if}</button></div>
            </section>
            <section class="panel table-panel">
              <div class="panel-heading"><div><h2>Immutable run registry</h2><p>{researchIntegrity?.runCardCount ?? researchRunCards.length} total run card(s) · latest {researchRunCards.length} loaded · newest first · each card stores its input fingerprint and gate evidence.</p></div><button class="button-secondary compact" onclick={loadResearchWorkspace} disabled={researchWorkspaceBusy}><RefreshCw size={13} class={researchWorkspaceBusy ? 'spin' : ''}/> Verify / refresh</button></div>
              {#if researchWorkspaceBusy}<div class="empty-state"><RefreshCw size={20} class="spin"/><span>Verifying research ledger…</span></div>
              {:else if researchRunCards.length}<div class="table-wrap"><table class="data-table"><thead><tr><th>Created</th><th>Goal</th><th>Strategy</th><th>Return</th><th>Drawdown</th><th>Gate</th><th>Run ID</th></tr></thead><tbody>{#each researchRunCards as record}<tr><td>{new Date(record.card.createdAtMs).toLocaleString()}</td><td>{record.card.goal.slice(0, 64)}{record.card.goal.length > 64 ? '…' : ''}</td><td>SMA {record.card.backtest.fastPeriod}/{record.card.backtest.slowPeriod}</td><td class:positive={record.card.backtest.totalReturnPct >= 0} class:negative={record.card.backtest.totalReturnPct < 0}>{signedPct(record.card.backtest.totalReturnPct)}</td><td>{number(Math.abs(record.card.backtest.maxDrawdownPct), 2)}%</td><td><span class:status-good={record.card.decision.eligibleForPaperValidation} class:status-bad={!record.card.decision.eligibleForPaperValidation} class="status-tag">{record.card.decision.status}</span></td><td><button class="text-action" onclick={() => selectResearchRun(record.card.id)}>{record.card.id.slice(0, 12)}…</button></td></tr>{/each}</tbody></table></div>
              {:else}<div class="empty-state"><BookOpen size={24}/><strong>No research run cards</strong><span>Register a hypothesis or execute an unlinked research run. This view does not seed sample records.</span></div>{/if}
            </section>
            {#if selectedResearchRun}
              <section class="panel result-panel">
                <div class="panel-heading"><div><h2>Research run dossier</h2><p>{selectedResearchRun.card.symbol} {selectedResearchRun.card.interval} · {selectedResearchRun.card.backtest.strategyName} · {new Date(selectedResearchRun.card.createdAtMs).toLocaleString()}</p></div><span class:status-good={selectedResearchRun.card.decision.eligibleForPaperValidation} class:status-bad={!selectedResearchRun.card.decision.eligibleForPaperValidation} class="status-tag">{selectedResearchRun.card.decision.status}</span></div>
                <div class="result-strip"><div><small>Total return</small><strong>{signedPct(selectedResearchRun.card.backtest.totalReturnPct)}</strong></div><div><small>Max drawdown</small><strong>{number(Math.abs(selectedResearchRun.card.backtest.maxDrawdownPct), 2)}%</strong></div><div><small>Sharpe</small><strong>{number(selectedResearchRun.card.backtest.sharpeRatio, 3)}</strong></div><div><small>Trades</small><strong>{selectedResearchRun.card.backtest.tradeCount}</strong></div></div>
                <details class="dataset-provenance" open><summary>Input provenance and reproducibility</summary><div class="dataset-provenance-grid"><div><small>Dataset SHA-256</small><code>{selectedResearchRun.card.backtest.datasetSha256}</code></div><div><small>Data source</small><strong>{selectedResearchRun.card.backtest.dataSource}</strong></div><div><small>Input range UTC</small><strong>{isoDateTimeLabel(selectedResearchRun.card.backtest.datasetStartMs)} — {isoDateTimeLabel(selectedResearchRun.card.backtest.datasetEndMs)}</strong></div><div><small>Run card SHA-256</small><code>{selectedResearchRun.contentSha256}</code></div></div></details>
                <div class="panel-heading"><div><h2>Workflow execution graph</h2><p>Independent robustness branches operate over the same immutable backtest evidence.</p></div></div>
                <div class="event-list">{#each selectedResearchRun.card.workflowGraph as node}<div class="event-row"><span class="event-icon"><CheckCircle2 size={15}/></span><div><strong>{node.nodeId.replaceAll('_', ' ').toUpperCase()} · {node.status}</strong><p>{node.detail}</p></div><time>{new Date(node.completedAtMs).toLocaleTimeString()}</time></div>{/each}</div>
                <div class="panel-heading"><div><h2>Independent statistical reviews</h2><p>Bootstrap is a resampled sensitivity distribution, not a market forecast.</p></div></div>
                <div class="result-strip"><div><small>OOS positive folds</small><strong>{selectedResearchRun.card.outOfSampleReview.positiveFoldCount} / {selectedResearchRun.card.outOfSampleReview.foldCount}</strong></div><div><small>OOS median return</small><strong>{selectedResearchRun.card.outOfSampleReview.medianReturnPct === null ? '—' : signedPct(selectedResearchRun.card.outOfSampleReview.medianReturnPct)}</strong></div><div><small>Bootstrap paths</small><strong>{selectedResearchRun.card.blockBootstrap.available ? selectedResearchRun.card.blockBootstrap.simulations : 'Unavailable'}</strong></div><div><small>Bootstrap p95 drawdown</small><strong>{selectedResearchRun.card.blockBootstrap.maxDrawdownP95Pct === null ? '—' : `${number(selectedResearchRun.card.blockBootstrap.maxDrawdownP95Pct, 2)}%`}</strong></div></div>
                {#if selectedResearchRun.card.strategyTournament}
                  <section class="ta-indicator-section tournament-section">
                    <div class="ta-section-heading"><div><h2>Nested walk-forward parameter tournament</h2><p>Training-only selection across a bounded SMA period grid; each selected pair is scored on the following unseen period.</p></div><span class="ta-source-tag">{selectedResearchRun.card.strategyTournament.algorithmVersion}</span></div>
                    <div class="result-strip">
                      <div><small>Candidate configurations</small><strong>{selectedResearchRun.card.strategyTournament.candidateCount}</strong></div>
                      <div><small>Completed OOS folds</small><strong>{selectedResearchRun.card.strategyTournament.completedFolds} / {selectedResearchRun.card.strategyTournament.requestedFolds}</strong></div>
                      <div><small>Positive OOS ratio</small><strong>{selectedResearchRun.card.strategyTournament.positiveOosFoldRatio === null ? '—' : `${number(selectedResearchRun.card.strategyTournament.positiveOosFoldRatio * 100, 1)}%`}</strong></div>
                      <div><small>Parameter stability</small><strong>{selectedResearchRun.card.strategyTournament.parameterStabilityPct === null ? '—' : `${number(selectedResearchRun.card.strategyTournament.parameterStabilityPct, 1)}%`}</strong></div>
                    </div>
                    <div class="table-wrap"><table class="data-table"><thead><tr><th>Fold</th><th>Training period</th><th>Unseen period</th><th>Selected SMA</th><th>Training Sharpe</th><th>OOS return</th><th>OOS drawdown</th><th>OOS trades</th></tr></thead><tbody>
                      {#each selectedResearchRun.card.strategyTournament.folds as fold}
                        <tr><td>{fold.foldIndex} · {fold.status}</td><td>{isoDateTimeLabel(fold.trainingStartMs)} — {isoDateTimeLabel(fold.trainingEndMs)}</td><td>{isoDateTimeLabel(fold.testingStartMs)} — {isoDateTimeLabel(fold.testingEndMs)}</td><td>{fold.selectedFastPeriod === null ? 'No qualified pair' : `${fold.selectedFastPeriod}/${fold.selectedSlowPeriod}`}</td><td>{fold.selectedTrainingSharpeRatio === null ? '—' : number(fold.selectedTrainingSharpeRatio, 3)}</td><td class:positive={(fold.outOfSampleReturnPct ?? 0) > 0} class:negative={fold.outOfSampleReturnPct !== null && fold.outOfSampleReturnPct <= 0}>{fold.outOfSampleReturnPct === null ? '—' : signedPct(fold.outOfSampleReturnPct)}</td><td>{fold.outOfSampleMaxDrawdownPct === null ? '—' : `${number(fold.outOfSampleMaxDrawdownPct, 2)}%`}</td><td>{fold.outOfSampleTradeCount ?? '—'}</td></tr>
                        <tr><td colspan="8"><details><summary>Training candidate scores · fold {fold.foldIndex}</summary><div class="table-wrap"><table class="data-table"><thead><tr><th>Fast</th><th>Slow</th><th>Training Sharpe</th><th>Return</th><th>Drawdown</th><th>Trades</th><th>Selection eligible</th></tr></thead><tbody>{#each fold.candidateScores as score}<tr><td>{score.fastPeriod}</td><td>{score.slowPeriod}</td><td>{number(score.trainingSharpeRatio, 3)}</td><td>{signedPct(score.trainingReturnPct)}</td><td>{number(score.trainingMaxDrawdownPct, 2)}%</td><td>{score.trainingTradeCount}</td><td>{score.qualifiesForSelection ? 'Yes' : 'No'}</td></tr>{/each}</tbody></table></div></details></td></tr>
                      {/each}
                    </tbody></table></div>
                    {#if selectedResearchRun.card.strategyTournament.blockers.length}<div class="warnings-list"><h3>Tournament blockers</h3>{#each selectedResearchRun.card.strategyTournament.blockers as blocker}<div><ShieldAlert size={14}/><span>{blocker}</span></div>{/each}</div>{/if}
                    {#each selectedResearchRun.card.strategyTournament.caveats as caveat}<p class="table-footnote">{caveat}</p>{/each}
                  </section>
                {/if}
                <div class="table-footnote"><strong>Recorded research gates:</strong> max drawdown ≤ {number(selectedResearchRun.card.gateParameters.maximumDrawdownPct, 2)}% · positive OOS folds ≥ {number(selectedResearchRun.card.gateParameters.minimumPositiveOosRatio * 100, 1)}% · Sharpe ≥ {number(selectedResearchRun.card.gateParameters.minimumSharpeRatio, 2)}. These exact thresholds are stored in the immutable run card.</div>
                <div class="event-list">{#each selectedResearchRun.card.specialistReviews as review}<div class="event-row"><span class="event-icon"><ShieldCheck size={15}/></span><div><strong>{review.role} · {review.status}</strong><p>{review.summary}</p>{#each review.blockers as blocker}<p class="negative">Blocker: {blocker}</p>{/each}{#each review.caveats as caveat}<p class="muted-note">{caveat}</p>{/each}</div></div>{/each}</div>
                <div class="warnings-list"><h3>Committee blockers</h3>{#if selectedResearchRun.card.decision.blockers.length}{#each selectedResearchRun.card.decision.blockers as blocker}<div><ShieldAlert size={14}/><span>{blocker}</span></div>{/each}{:else}<div><CheckCircle2 size={14}/><span>No research gate blockers. Eligibility means supervised paper validation only.</span></div>{/if}</div>
                <p class="table-footnote">Gate policy {selectedResearchRun.card.decision.policyVersion}. Passing this research gate does not authorise live or paper order submission; a separately supervised validation process and account risk controls remain mandatory.</p>
              </section>
            {/if}
            <section class="ta-methodology-note"><ShieldCheck size={16}/><p><strong>Integrity:</strong> hypotheses use optimistic revisions; run cards are immutable and content-hashed; the research event ledger is hash-chained and checked at startup. This is tamper-evident local evidence, not a remote notarisation service or protection against a privileged local attacker.</p></section>

          {:else if activeNav === 'Technical Analysis'}
            <div class="page-heading"><div><div class="eyebrow">RESEARCH / TECHNICAL TAXONOMY</div><h1>Technical analysis library</h1><p>Master catalog for SMC/ICT, price action, indicators, order flow, macro context, and risk evaluation.</p></div><span class="mode-pill"><Layers3 size={13}/> {TECHNICAL_ANALYSIS_CATALOG.length} CONCEPTS</span></div>
            <div class="notice-banner"><BookOpen size={18}/><div><strong>Catalog coverage is not the same as implemented detection</strong><p>Only the six calculations below are numerically computed in this build. Other entries are indexed for research and have not been implemented as detectors, signals, or validated trading rules.</p></div></div>
            <section class="ta-indicator-section">
              <div class="ta-section-heading"><div><h2>Available indicator readings</h2><p>Calculated from the currently loaded OHLCV candle series; unavailable when the sample is too short or malformed.</p></div><span class="ta-source-tag">{candles.length} candles · snapshot.providerSource</span></div>
              <div class="ta-reading-grid">
                {#each technicalReadings as reading}
                  <article class="panel ta-reading-card"><div class="ta-reading-top"><span>{reading.title}</span><span class:status-good={reading.available} class:status-idle={!reading.available} class="status-tag">{reading.available ? 'CALCULATED' : 'NEEDS DATA'}</span></div><strong class:ta-unavailable={!reading.available}>{reading.value}</strong><p>{reading.detail}</p></article>
                {/each}
              </div>
            </section>
            <section class="panel table-panel ta-engine-panel">
              <div class="panel-heading"><div><h2>Layered analysis & decision engine</h2><p>Deterministic, evidence-bound research diagnostics. This panel has no order authority.</p></div><span class="mode-pill">{technicalEnvelope.engineVersion}</span></div>
              <div class="result-strip">
                <div><small>Technical engine</small><strong>{technicalEnvelope.status.toUpperCase().replaceAll('_', ' ')}</strong></div>
                <div><small>Loaded candles</small><strong>{technicalEnvelope.dataQuality.candleCount}</strong></div>
                <div><small>Signal domains</small><strong>{technicalEnvelope.decision.coverage.accepted} / {technicalEnvelope.decision.coverage.supplied}</strong></div>
                <div><small>Research verdict</small><strong>{technicalEnvelope.decision.decision.replaceAll('_', ' ')}</strong></div>
              </div>
              <div class="table-footnote"><strong>Structure:</strong> {technicalEnvelope.structure.bias} · {technicalEnvelope.structure.pivots.length} recent pivots · {technicalEnvelope.structure.breaks.length} recent close-confirmed breaks.</div>
              <div class="table-footnote"><strong>Temporal & anomaly audit:</strong> {technicalEnvelope.dataQuality.audit.status.toUpperCase()} · {technicalEnvelope.dataQuality.audit.hardIssues.length} integrity fault(s) · {technicalEnvelope.dataQuality.audit.warnings.length} advisory warning(s). Latest-bar warnings are review prompts, not proof of bad data; candidate handoff stays blocked while warnings remain.</div>
              {#if technicalEnvelope.dataQuality.issues.length}
                <div class="warnings-list">{#each technicalEnvelope.dataQuality.issues.slice(0, 3) as issue}<div><ShieldAlert size={14}/><span>{issue}</span></div>{/each}</div>
              {/if}
              {#if technicalEnvelope.decision.reasons.length}
                <div class="warnings-list">{#each technicalEnvelope.decision.reasons.slice(0, 3) as reason}<div><ShieldAlert size={14}/><span>{reason}</span></div>{/each}</div>
              {/if}
              <p class="table-footnote">The chart adapter has no canonical dataset fingerprint or native Rust risk receipt for this view, so the decision engine must block candidate handoff. Indicator agreement is heuristic strength, not a profit probability. Live execution is never authorized.</p>
            </section>
            <section class="panel ta-catalog-panel">
              <div class="panel-heading"><div><h2>Master list</h2><p>{filteredAnalysisCatalog.length} matching concepts · category and implementation status are explicit</p></div><span class="version-tag">MASTER LIST v{TECHNICAL_ANALYSIS_MASTER_LIST_VERSION}</span></div>
              <div class="ta-filters">
                <label class="ta-search"><Search size={14}/><input bind:value={analysisQuery} placeholder="Search BOS, RSI, liquidity, drawdown…" aria-label="Search technical analysis catalog" oninput={() => analysisLimit = 60}/></label>
                <select aria-label="Filter technical analysis category" bind:value={analysisCategory} onchange={() => { analysisGroup = 'All groups'; analysisLimit = 60; }}><option>All categories</option>{#each TECHNICAL_ANALYSIS_CATEGORIES as category}<option value={category}>{category}</option>{/each}</select>
                <select aria-label="Filter technical analysis group" bind:value={analysisGroup} onchange={() => analysisLimit = 60}>{#each analysisGroups as group}<option value={group}>{group}</option>{/each}</select>
                <select aria-label="Filter implementation coverage" bind:value={analysisCoverage} onchange={() => analysisLimit = 60}><option>All statuses</option><option>Calculated now</option><option>Workflow available</option><option>Catalog only</option></select>
              </div>
              <div class="ta-coverage-legend"><span><i class="ta-legend-dot ready"></i> Calculated now: {analysisStatusCounts.calculated}</span><span><i class="ta-legend-dot ready"></i> Workflow available: {analysisStatusCounts.workflow}</span><span><i class="ta-legend-dot planned"></i> Catalog only: {analysisStatusCounts.catalogOnly} · not implemented</span></div>
              {#if visibleAnalysisCatalog.length}
                <div class="ta-catalog-list">
                  {#each visibleAnalysisCatalog as item (item.id)}
                    <article class="ta-catalog-item"><div class="ta-item-main"><span class="ta-item-category">{item.category} / {item.group}</span><strong>{item.name}</strong></div><span class:ta-coverage-ready={item.available} class:ta-coverage-planned={!item.available} class="ta-coverage-label">{item.availability}</span></article>
                  {/each}
                </div>
                <div class="panel-bottom-note"><span>Showing {visibleAnalysisCatalog.length} of {filteredAnalysisCatalog.length} matching items.</span>{#if visibleAnalysisCatalog.length < filteredAnalysisCatalog.length}<button class="button-secondary compact" onclick={() => analysisLimit += 60}>Load 60 more</button>{:else if analysisLimit > 60}<button class="button-secondary compact" onclick={() => analysisLimit = 60}>Show fewer</button>{/if}</div>
              {:else}
                <div class="empty-state"><Search size={24}/><strong>No matching concepts</strong><span>Try a broader search or reset one of the filters.</span><button class="button-secondary compact" onclick={() => { analysisQuery = ''; analysisCategory = 'All categories'; analysisGroup = 'All groups'; analysisCoverage = 'All statuses'; analysisLimit = 60; }}>Reset filters</button></div>
              {/if}
            </section>
            <section class="ta-methodology-note"><ShieldCheck size={16}/><p><strong>Research integrity:</strong> these readings describe the loaded dataset and are not trade recommendations. Indicator thresholds are conventions, not guarantees. No order is generated from this page, and catalog presence does not imply historical validation.</p></section>

          {:else if activeNav === 'AI Agents'}
            <div class="page-heading"><div><div class="eyebrow">SUPERVISION / STRICT RESEARCH CONTRACT</div><h1>Agent control room</h1><p>One bounded provider-backed research analyst is available in the native desktop runtime. The nine-specialist, 364-item workflow remains plan-only; no specialist dispatch or per-concept receipts are active. Agent prose is advisory; validated Rust-derived evidence alone enters the deterministic decision layer.</p></div><span class="mode-pill"><Bot size={13}/> {snapshot.agents.filter((agent) => agent.status.startsWith('RUNNING')).length} SUPERVISED</span></div>
            <div class="notice-banner"><ShieldCheck size={18}/><div><strong>Execution authority remains disabled</strong><p>Providers use bounded read-only requests and OS credential storage. Imported model files are fingerprinted and registered but are not executed by this app; no model can issue orders or override the Rust risk boundary.</p></div></div>
            <AgentRuntimePanel desktop={desktop} symbol={activeCandleRequest.symbol} timeframe={activeCandleRequest.interval} marketReady={Boolean(chartSeriesResponse)} />
            <section class="agent-grid">{#each snapshot.agents as agent}<article class="panel agent-card"><div class="agent-card-top"><div class="agent-avatar"><Bot size={22}/></div><div class="agent-title"><h2>{agent.name}</h2><p>{agent.role}</p></div><span class:status-good={agent.status.startsWith('RUNNING')} class:status-idle={!agent.status.startsWith('RUNNING')} class="status-tag">{agent.status.startsWith('RUNNING') ? 'RUNNING' : 'STOPPED'}</span></div><p class="agent-objective">{agent.objective}</p><div class="agent-stats"><div><small>Mode</small><strong>{agent.mode}</strong></div><div><small>Paper PnL</small><strong class:positive={agent.pnl >= 0} class:negative={agent.pnl < 0}>{money(agent.pnl)}</strong></div></div><div class="agent-tools"><small>READ-ONLY TOOL ALLOWLIST</small><div>{#each agent.tools as tool}<span>{tool}</span>{/each}</div></div><div class="agent-last"><Clock3 size={13}/><span>{agent.lastDecision}</span></div><button class:button-danger={agent.status.startsWith('RUNNING')} class="button-secondary full-width" onclick={() => toggleAgent(agent)}>{#if agent.status.startsWith('RUNNING')}<Pause size={14}/> Stop agent{:else}<Play size={14}/> Start observe-only{/if}</button></article>{/each}</section>
            <section class="panel table-panel"><div class="panel-heading"><div><h2>Agent event trace</h2><p>Local audit events; no hidden tool calls are implied.</p></div></div><div class="event-list">{#each snapshot.events.filter((event) => event.category === 'agent' || event.category === 'risk').slice(0, 10) as event}<div class="event-row"><span class="event-icon"><Bot size={15}/></span><div><strong>{event.title}</strong><p>{event.detail}</p></div><time>{timeLabel(event.timestampMs)}</time></div>{/each}</div></section>

          {:else if activeNav === 'MT5 Bridge'}
            <Mt5BridgePanel />
          {:else if activeNav === 'Live Trading'}
            <div class="page-heading"><div><div class="eyebrow">EXECUTION / LIVE VENUES</div><h1>Live trading</h1><p>This capability is intentionally unavailable in the current build.</p></div><span class="locked-pill"><LockKeyhole size={13}/> LOCKED</span></div>
            <section class="panel live-lock-panel"><div class="lock-orb"><LockKeyhole size={30}/></div><h2>Live execution adapter not installed</h2><p>The app currently has no exchange order endpoint, credential vault, order reconciliation, venue-side idempotency or preflight approval workflow. The interface will not pretend that a simulated order is live.</p><div class="lock-requirements"><div><CheckCircle2 size={16}/><span>Rust risk gateway for paper intents</span><strong>Present</strong></div><div><CheckCircle2 size={16}/><span>Paper order accounting and local state</span><strong>Present</strong></div><div><Clock3 size={16}/><span>Trade-only credential vault + permission checks</span><strong>Not implemented</strong></div><div><Clock3 size={16}/><span>Venue adapter, idempotency and reconciliation</span><strong>Not implemented</strong></div><div><Clock3 size={16}/><span>Live preflight, confirmation and incident runbook</span><strong>Not implemented</strong></div></div><button class="button-secondary" onclick={() => { void navigate('Paper Trading'); }}>Return to paper trading</button></section>

          {:else if activeNav === 'Market Data'}
            <div class="page-heading"><div><div class="eyebrow">MARKET DATA / PROVIDER HEALTH</div><h1>Market data</h1><p>Quotes include the source and last refresh state. A price is not presented as live without provider metadata.</p></div><button class="button-primary" onclick={refreshSnapshot} disabled={refreshBusy}><RefreshCw size={15} class={refreshBusy ? 'spin' : ''}/> Refresh public feed</button></div>
            <section class="provider-card panel"><div class="provider-main-icon"><Database size={22}/></div><div class="provider-main-copy"><h2>{snapshot.providerStatus}</h2><p>{snapshot.providerSource}</p><small>Last update: {new Date(snapshot.providerUpdatedAtMs).toLocaleString()} · {ageLabel(snapshot.providerUpdatedAtMs, clockNow)}</small></div><span class:status-good={providerIsFresh} class:status-bad={!providerIsFresh} class="status-tag">{providerIsFresh ? 'RECENT' : 'STALE'}</span></section>
            <section class="panel capability-panel"><div class="panel-heading"><div><h2>Candle-series capabilities</h2><p>Quote support and candle support are different. Exact symbol/timeframe requests never silently substitute another dataset.</p></div><span class="version-tag">TYPED API</span></div><div class="capability-list">{#each snapshot.marketDataCapabilities.candleSeries as series}<div class="capability-row"><div class="capability-primary"><strong>{series.symbol} · {series.interval}</strong><span>{series.note}</span><small>{series.source} · {series.candleCount} candle(s){series.latestClosedBarAtMs ? ` · last closed ${new Date(series.latestClosedBarAtMs).toLocaleString()}` : ''}{series.retrievedAtMs ? ` · received ${new Date(series.retrievedAtMs).toLocaleString()}` : ''}</small></div><span class:status-good={series.available && series.isLive} class:status-idle={series.available && !series.isLive} class:status-bad={!series.available} class="status-tag">{series.available ? series.isLive ? 'LIVE · AVAILABLE' : 'CACHED · NOT LIVE' : 'NOT AVAILABLE'}</span><button class="button-secondary compact" disabled={!series.supported || !series.available || chartSeriesBusy} onclick={() => { void openCandleSeries(series); }}><CandlestickChart size={13}/> Open chart</button></div>{/each}</div><div class="panel-bottom-note"><span>Quote symbols ({snapshot.marketDataCapabilities.quoteSymbols.length}) do not imply candle history exists for each one.</span><span class="source-label">{snapshot.marketDataCapabilities.quoteSymbols.join(', ')}</span></div></section>
            <section class="panel table-panel"><div class="panel-heading"><div><h2>Spot quote watchlist</h2><p>Public market data; prices can move between refreshes.</p></div><div class="table-search"><Search size={14}/><input bind:value={searchQuery} placeholder="Filter symbols"/></div></div><div class="table-wrap"><table class="data-table"><thead><tr><th>Instrument</th><th>Last price</th><th>24h change</th><th>24h quote volume</th><th>Source</th><th>Observed / received</th><th></th></tr></thead><tbody>{#each filteredQuotes as quote}<tr><td><button class="asset-link" onclick={() => selectSymbol(quote.symbol)}><span class="mini-coin">{quote.symbol.slice(0, 1)}</span><strong>{quote.symbol}</strong></button></td><td>{quoteIsFresh(quote, clockNow) ? money(quote.price) : 'Unavailable · stale quote'}</td><td class:positive={quoteIsFresh(quote, clockNow) && quote.change24hPct >= 0} class:negative={quoteIsFresh(quote, clockNow) && quote.change24hPct < 0}>{quoteIsFresh(quote, clockNow) ? signedPct(quote.change24hPct) : 'Unavailable'}</td><td>{quoteIsFresh(quote, clockNow) ? money(quote.volume24h, true) : 'Unavailable'}</td><td><span class="source-label">{quote.source}</span></td><td>{ageLabel(quote.updatedAtMs, clockNow)} / {ageLabel(quote.retrievedAtMs, clockNow)}</td><td><button class="table-action" onclick={() => selectSymbol(quote.symbol)}>Trade</button></td></tr>{/each}</tbody></table></div></section>
            <MarketDataPlatformPanel />

          {:else if activeNav === 'Meme Coin Hunter'}
            <MemeCoinHunterPanel />
          {:else if activeNav === 'Portfolio'}
            <div class="page-heading"><div><div class="eyebrow">ACCOUNT / EXPOSURE</div><h1>Portfolio</h1><p>Paper balances and marks based on the latest stored quote.</p></div><span class="mode-pill"><Wallet size={13}/> SIMULATED ACCOUNT</span></div>
            <section class="portfolio-metrics metric-grid"><article class="metric-card"><div class="metric-head"><span>Equity</span><Wallet size={15}/></div><div class="metric-value">{portfolioMarksFresh ? money(snapshot.equity) : 'Unavailable'}</div><div class="metric-foot"><span>{portfolioMarksFresh ? 'Cash + fresh marked positions' : 'Refresh every open-position mark'}</span></div></article><article class="metric-card"><div class="metric-head"><span>Available cash</span><Activity size={15}/></div><div class="metric-value">{money(snapshot.cashBalance)}</div><div class="metric-foot"><span>After simulated fills and fees</span></div></article><article class="metric-card"><div class="metric-head"><span>Realized PnL</span><TrendingUp size={15}/></div><div class="metric-value" class:positive={snapshot.realizedPnl >= 0} class:negative={snapshot.realizedPnl < 0}>{money(snapshot.realizedPnl)}</div><div class="metric-foot"><span>Closed paper trades</span></div></article><article class="metric-card"><div class="metric-head"><span>Unrealized PnL</span><TrendingDown size={15}/></div><div class="metric-value" class:positive={snapshot.unrealizedPnl >= 0} class:negative={snapshot.unrealizedPnl < 0}>{portfolioMarksFresh ? money(snapshot.unrealizedPnl) : 'Unavailable'}</div><div class="metric-foot"><span>{portfolioMarksFresh ? 'Open paper positions' : 'One or more marks are stale or missing'}</span></div></article></section>
            <section class="panel table-panel"><div class="panel-heading"><div><h2>Current exposure</h2><p>Current marks are unavailable unless each position has a fresh live quote.</p></div></div>{#if snapshot.positions.length}<div class="table-wrap"><table class="data-table"><thead><tr><th>Instrument</th><th>Quantity</th><th>Average entry</th><th>Market price</th><th>Exposure</th><th>Unrealized PnL</th></tr></thead><tbody>{#each snapshot.positions as position}<tr><td><strong>{position.symbol}</strong></td><td>{number(position.quantity, 8)}</td><td>{money(position.averageEntry)}</td><td>{positionMarkFresh(position.symbol, clockNow) ? money(position.markPrice) : 'Unavailable · stale mark'}</td><td>{positionMarkFresh(position.symbol, clockNow) ? money(position.notional) : 'Unavailable'}</td><td class:positive={position.unrealizedPnl >= 0} class:negative={position.unrealizedPnl < 0}>{positionMarkFresh(position.symbol, clockNow) ? money(position.unrealizedPnl) : 'Unavailable'}</td></tr>{/each}</tbody></table></div>{:else}<div class="empty-state"><Wallet size={24}/><strong>Portfolio is flat</strong><span>There are no open simulated positions.</span></div>{/if}</section>
            <section class="panel table-panel"><div class="panel-heading"><div><h2>Exposure by instrument</h2><p>Notional share is available only while all position marks are fresh.</p></div></div>{#if snapshot.positions.length && portfolioMarksFresh}<div class="exposure-list">{#each snapshot.positions as position}<div class="exposure-row"><div><strong>{position.symbol}</strong><span>{money(position.notional)}</span></div><div class="exposure-track"><span style={`width:${Math.min(100, snapshot.equity > 0 ? position.notional / snapshot.equity * 100 : 0)}%`}></span></div><small>{number(snapshot.equity > 0 ? position.notional / snapshot.equity * 100 : 0)}%</small></div>{/each}</div>{:else if snapshot.positions.length}<div class="empty-state"><ShieldAlert size={20}/><strong>Exposure unavailable</strong><span>Refresh market data for every open position before displaying current exposure percentages.</span></div>{:else}<div class="empty-small">No active exposure to visualize.</div>{/if}</section>

          {:else if activeNav === 'Audit Log'}
            <div class="page-heading"><div><div class="eyebrow">OBSERVABILITY / APPEND-ONLY JOURNAL</div><h1>Audit log</h1><p>Persistent local events from market-data refreshes, risk decisions, paper fills, and operator controls.</p></div><div class="heading-actions"><button class="button-secondary" onclick={exportAuditCsv} disabled={auditBusy || filteredAuditEvents.length === 0}><Download size={14}/> Export CSV ({filteredAuditEvents.length})</button><button class="button-secondary" onclick={loadAuditHistory} disabled={auditBusy}><RefreshCw size={14} class={auditBusy ? 'spin' : ''}/> Refresh journal</button></div></div>
            <div class="notice-banner"><Database size={18}/><div><strong>Stored in the local SQLite audit table</strong><p>This view is read-only. Details are sourced from the Rust engine’s persisted audit events; it does not imply any unrecorded agent or exchange activity.</p></div></div>
            <section class="panel table-panel"><div class="panel-heading"><div><h2>Event journal</h2><p>{filteredAuditEvents.length} events shown from {auditEvents.length} loaded records · newest first</p></div><div class="table-search"><Search size={14}/><input bind:value={auditQuery} placeholder="Filter event details"/></div></div>{#if auditBusy}<div class="empty-state"><RefreshCw size={20} class="spin"/><span>Loading persisted journal…</span></div>{:else if filteredAuditEvents.length}<div class="table-wrap"><table class="data-table"><thead><tr><th>Timestamp</th><th>Level</th><th>Category</th><th>Event</th><th>Details</th><th>Event ID</th></tr></thead><tbody>{#each filteredAuditEvents as event}<tr><td>{new Date(event.timestampMs).toLocaleString()}</td><td><span class:status-bad={event.level === 'ERROR'} class:status-idle={event.level === 'INFO'} class:status-good={event.level === 'WARN'} class="status-tag">{event.level}</span></td><td>{event.category}</td><td><strong>{event.title}</strong></td><td class="audit-detail">{event.detail}</td><td class="event-id-cell">{event.id.slice(0, 12)}…</td></tr>{/each}</tbody></table></div>{:else}<div class="empty-state"><Clock3 size={24}/><strong>No matching audit events</strong><span>Change the search query or refresh the journal.</span></div>{/if}</section>

          {:else if activeNav === 'Tools & MCP'}
            <div class="page-heading"><div><div class="eyebrow">EXTENSIONS / LEAST PRIVILEGE</div><h1>Tools & MCP</h1><p>External tool execution is intentionally not enabled by default.</p></div><span class="locked-pill"><LockKeyhole size={13}/> DISABLED BY DEFAULT</span></div>
            <section class="panel tool-security-card"><div class="tool-large-icon"><Network size={24}/></div><h2>Tool registry is not connected</h2><p>No MCP server, shell execution tool, filesystem tool, or model-provider adapter is currently registered. This page is not a control surface for arbitrary commands.</p><div class="security-rule-list"><div><ShieldCheck size={16}/><span>Default-deny tool allowlist</span><strong>Policy</strong></div><div><ShieldCheck size={16}/><span>Rust-controlled order and risk boundary</span><strong>Policy</strong></div><div><Clock3 size={16}/><span>Tool timeout, budget, and per-call trace</span><strong>Not implemented</strong></div><div><Clock3 size={16}/><span>Network destination allowlist and MCP sandbox</span><strong>Not implemented</strong></div></div></section>

          {:else if activeNav === 'Settings'}
            <div class="page-heading"><div><div class="eyebrow">CONFIGURATION / RISK POLICY</div><h1>Settings</h1><p>Set hard limits in the local Rust core, not in agent-generated prompts.</p></div><span class="mode-pill"><Settings size={13}/> LOCAL</span></div>
            <section class="panel settings-panel"><div class="panel-heading"><div><h2>Risk gateway limits</h2><p>Values are applied by the native Rust engine to subsequent paper-order intents.</p></div><ShieldCheck size={17}/></div><div class="settings-grid"><div class="config-field"><label for="max-order">Max order notional (USD)</label><input id="max-order" type="number" min="1" step="10" bind:value={riskForm.maxOrderNotional}/><small>Maximum notional of any new order.</small></div><div class="config-field"><label for="max-position">Max position notional (USD)</label><input id="max-position" type="number" min="1" step="50" bind:value={riskForm.maxPositionNotional}/><small>Maximum marked exposure per instrument.</small></div><div class="config-field"><label for="max-loss">Daily loss guard (USD)</label><input id="max-loss" type="number" min="1" step="10" bind:value={riskForm.maxDailyLoss}/><small>New orders should be blocked past the loss threshold.</small></div><div class="config-field"><label for="fee-bps">Trading fee (basis points)</label><input id="fee-bps" type="number" min="0" max="500" step="1" bind:value={riskForm.feeBps}/><small>Applied to each simulated fill.</small></div><div class="config-field"><label for="slippage-bps">Slippage (basis points)</label><input id="slippage-bps" type="number" min="0" max="500" step="1" bind:value={riskForm.slippageBps}/><small>Fixed adverse execution assumption.</small></div></div><div class="config-bottom"><p class="muted-note">These controls are for simulation. They do not cap external exchange risk because live execution is not implemented.</p><button class="button-primary" onclick={saveRiskSettings} disabled={riskBusy || !desktop}>{#if riskBusy}<RefreshCw size={14} class="spin"/> Saving…{:else}<CheckCircle2 size={14}/> Save risk policy{/if}</button></div></section>
            <section class="panel danger-zone"><div class="danger-title"><ShieldAlert size={18}/><div><h2>Paper account maintenance</h2><p>Resetting clears local paper orders and positions. This cannot reverse or affect real transactions.</p></div></div><button class="button-danger" onclick={resetAccount}>Reset paper account</button></section>
            <section class="panel provider-card"><div class="provider-main-icon"><Cpu size={22}/></div><div class="provider-main-copy"><h2>Runtime status</h2><p>Tauri desktop shell detected; commands route into the Rust backend.</p><small>Rust owns paper accounting and risk checks. Frontend is a command surface, not an execution authority.</small></div><span class:status-good={desktop} class:status-idle={!desktop} class="status-tag">NATIVE</span></section>
          {/if}
        </main>

        <aside class="right-rail">
          <section class="rail-section watchlist-section"><div class="rail-heading"><h2>Watchlist</h2><button class="icon-button tiny" aria-label="View market data" onclick={() => { void navigate('Market Data'); }}><Plus size={16}/></button></div><div class="watchlist-tabs"><span class="selected" aria-current="page">Crypto</span><button disabled title="No Forex data provider is connected">Forex</button><button disabled title="No indices data provider is connected">Indices</button><button disabled title="No commodities data provider is connected">Commodities</button></div><div class="watchlist-header"><span>Symbol</span><span>Price</span><span>24h</span><span>Vol</span></div>{#if snapshot.quotes.length === 0}<div class="empty-small">No validated quote observations are loaded.</div>{/if}{#each snapshot.quotes.slice(0, 6) as quote}<button class="watchlist-row" onclick={() => selectSymbol(quote.symbol)}><span class="watch-asset"><span class="watch-symbol-icon">{quote.symbol.startsWith('BTC') ? '₿' : quote.symbol.startsWith('ETH') ? '◆' : quote.symbol.slice(0, 1)}</span><strong>{quote.symbol}</strong><small>{quoteIsFresh(quote, clockNow) ? 'Live' : 'Stale cached'}</small></span><span>{quoteIsFresh(quote, clockNow) ? number(quote.price, quote.price < 1 ? 5 : 2) : '—'}</span><span class:positive={quoteIsFresh(quote, clockNow) && quote.change24hPct >= 0} class:negative={quoteIsFresh(quote, clockNow) && quote.change24hPct < 0}>{quoteIsFresh(quote, clockNow) ? signedPct(quote.change24hPct) : '—'}</span><span class="watch-volume">{quoteIsFresh(quote, clockNow) ? compactNumber(quote.volume24h) : '—'}</span></button>{/each}</section>
          <section class="rail-section agents-rail"><div class="rail-heading"><h2>Agent supervisors</h2><button class="text-action" onclick={() => { void navigate('AI Agents'); }}>View all</button></div>{#each snapshot.agents as agent}<div class="rail-agent"><div class="rail-agent-icon"><Bot size={18}/></div><div class="rail-agent-copy"><strong>{agent.name}</strong><small>{agent.role}</small><span class:positive={agent.status.startsWith('RUNNING')} class="agent-status"><span class="small-dot"></span>{agent.status.startsWith('RUNNING') ? 'Running · observe-only' : 'Stopped'}</span><small class:positive={agent.pnl >= 0} class:negative={agent.pnl < 0} class="rail-agent-pnl">PnL {money(agent.pnl)}</small></div><button class:toggle-on={agent.status.startsWith('RUNNING')} class="toggle-control" aria-label={`${agent.status.startsWith('RUNNING') ? 'Stop' : 'Start'} ${agent.name}`} onclick={() => toggleAgent(agent)}><span></span></button></div>{/each}</section>
          <section class="rail-section events-rail"><div class="rail-heading"><h2>Latest events</h2><button class="text-action" onclick={() => { void navigate('Audit Log'); }}>Audit</button></div>{#each snapshot.events.slice(0, 6) as event}<div class="rail-event"><div class="event-level-icon" class:level-warn={event.level === 'WARN'} class:level-error={event.level === 'ERROR'}>{#if event.category === 'risk'}<ShieldAlert size={14}/>{:else if event.category === 'market-data'}<Activity size={14}/>{:else if event.category === 'agent'}<Bot size={14}/>{:else}<CheckCircle2 size={14}/>{/if}</div><div class="rail-event-copy"><div><time>{timeLabel(event.timestampMs)}</time><strong>{event.title}</strong></div><p>{event.detail}</p></div></div>{/each}</section>
          <section class="rail-security-note"><div><ShieldCheck size={15}/><strong>Risk boundary</strong></div><p>Order logic and risk checks are enforced in Rust. Agent switches only enable observe-only mode in this build.</p><button class="text-action" onclick={() => { void navigate('Settings'); }}>Review policy <ArrowUpRight size={12}/></button></section>
        </aside>
      </div>
      <footer class="app-footer"><span>QuantDinger · AI Trading OS</span><span><span class="small-dot"></span>Rust engine · local-first</span><span>Not connected to a live execution venue</span></footer>
    {:else}
      <section class="runtime-load-failure" role="alert" aria-labelledby="runtime-load-failure-title">
        <ShieldAlert size={28} />
        <h1 id="runtime-load-failure-title">Local runtime unavailable</h1>
        <p>{errorMessage || 'The native engine did not return a validated dashboard snapshot. No market data or decision result is available.'}</p>
        <button class="button-primary" onclick={loadSnapshot} disabled={loading}><RefreshCw size={14} class={loading ? 'spin' : ''}/> Retry native connection</button>
      </section>
    {/if}
  </div>
</div>
{/if}
