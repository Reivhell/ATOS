<script lang="ts">
  import { onMount } from 'svelte';
  import { Activity, AlertTriangle, CheckCircle2, CircleSlash, Copy, Database, EyeOff, LockKeyhole, Network, RefreshCw, ShieldCheck, Wifi, WifiOff } from 'lucide-svelte';
  import { getMt5BridgeSetup, getMt5BridgeStatus, getMt5InstrumentSpecification, getMt5SessionCalendar, calculateMt5RiskSizing, queryMt5MarketDataSeries } from '../adapters/tauri/bridge';
  import type { Mt5BridgeSetup, Mt5BridgeStatus, Mt5InstrumentSpecificationRecord, Mt5SessionCalendarRecord, Mt5RiskSizingRequest, Mt5RiskSizingResult, Mt5SeriesQuery, Mt5SeriesQueryResponse, Mt5StoredBar } from '../types';

  let status: Mt5BridgeStatus | null = null;
  let setup: Mt5BridgeSetup | null = null;
  let loading = false;
  let setupLoading = false;
  let revealToken = false;
  let error = '';
  let notice = '';
  let seriesSymbol = '';
  let seriesTimeframe = 'M5';
  let seriesRangeDays = '7';
  let seriesLoading = false;
  let seriesError = '';
  let seriesResult: Mt5SeriesQueryResponse | null = null;
  let seriesActiveQuery: Mt5SeriesQuery | null = null;
  let seriesBars: Mt5StoredBar[] = [];
  let seriesFromMs: number | null = null;
  let seriesToMs: number | null = null;
  let instrumentSpecification: Mt5InstrumentSpecificationRecord | null = null;
  let instrumentSpecificationLoading = false;
  let instrumentSpecificationError = '';
  let sessionCalendar: Mt5SessionCalendarRecord | null = null;
  let sessionCalendarLoading = false;
  let sessionCalendarError = '';
  let riskSide: 'BUY' | 'SELL' = 'BUY';
  let riskStopLoss = '';
  let riskPercent = '1';
  let riskSizingLoading = false;
  let riskSizingError = '';
  let riskSizingResult: Mt5RiskSizingResult | null = null;
  let riskSizingResultStale = false;
  $: riskSizingResultStale = !!riskSizingResult && (
    !status?.connected || status.stale || status.quoteStale
    || status.lastSequence !== riskSizingResult.snapshotSequence
  );
  const weekdayNames = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  const sessionClock = (seconds: number) => seconds === 86400 ? '24:00' : `${String(Math.floor(seconds / 3600)).padStart(2, '0')}:${String(Math.floor((seconds % 3600) / 60)).padStart(2, '0')}`;

  const money = (value: number, currency: string) => {
    try {
      return new Intl.NumberFormat(undefined, { style: 'currency', currency: currency || 'USD', maximumFractionDigits: 2 }).format(value);
    } catch {
      return `${number(value, 2)} ${currency || ''}`.trim();
    }
  };
  const number = (value: number, maximumFractionDigits = 2) => new Intl.NumberFormat(undefined, { maximumFractionDigits }).format(value);
  const timestamp = (value: number | null) => value === null ? 'Unavailable' : new Date(value).toLocaleString();
  const age = (value: number | null) => value === null ? 'No heartbeat received' : `${(value / 1000).toFixed(1)}s ago`;

  async function refreshStatus() {
    if (loading) return;
    loading = true;
    try {
      status = await getMt5BridgeStatus();
      error = status.lastError ?? '';
      if (!seriesSymbol && status.symbol) seriesSymbol = status.symbol;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      loading = false;
    }
  }

  async function queryStoredSeries(afterMs: number | null = null) {
    if (seriesLoading) return;
    if (!status?.accountLogin || !status.accountServer || !seriesSymbol.trim()) {
      seriesError = 'A connected or previously authenticated MT5 account and a symbol are required.';
      return;
    }
    seriesLoading = true;
    seriesError = '';
    try {
      if (afterMs === null) {
        // MT5 bar timestamps follow the broker server clock. Anchor the query window to
        // that authenticated terminal snapshot instead of the workstation wall clock.
        seriesToMs = status.snapshot?.terminal.serverTimeMs ?? Date.now();
        seriesFromMs = seriesToMs - Number(seriesRangeDays) * 24 * 60 * 60 * 1000;
        seriesBars = [];
        seriesResult = null;
        seriesActiveQuery = {
          brokerServer: status.accountServer,
          accountLogin: status.accountLogin,
          symbol: seriesSymbol.trim(),
          timeframe: seriesTimeframe,
          fromMs: seriesFromMs,
          toMs: seriesToMs,
          afterMs: null,
          limit: 1000
        };
      }
      if (!seriesActiveQuery || seriesFromMs === null || seriesToMs === null) throw new Error('Series query range was not initialized.');
      const response = await queryMt5MarketDataSeries({ ...seriesActiveQuery, afterMs });
      if (afterMs !== null && (!seriesResult || response.datasetFingerprintSha256 !== seriesResult.datasetFingerprintSha256)) {
        throw new Error('The stored MT5 dataset changed while pages were loading. The current page was not appended; restart the query to obtain one consistent dataset snapshot.');
      }
      seriesResult = response;
      seriesBars = afterMs === null ? response.bars : [...seriesBars, ...response.bars];
    } catch (cause) {
      seriesError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      seriesLoading = false;
    }
  }

  async function inspectInstrumentSpecification() {
    if (instrumentSpecificationLoading) return;
    if (!status?.accountLogin || !status.accountServer || !seriesSymbol.trim()) {
      instrumentSpecificationError = 'An authenticated account and broker-native symbol are required.';
      return;
    }
    instrumentSpecificationLoading = true;
    instrumentSpecificationError = '';
    instrumentSpecification = null;
    try {
      instrumentSpecification = await getMt5InstrumentSpecification(
        status.accountServer, status.accountLogin, seriesSymbol.trim()
      );
    } catch (cause) {
      instrumentSpecificationError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      instrumentSpecificationLoading = false;
    }
  }

  async function inspectSessionCalendar() {
    if (sessionCalendarLoading) return;
    if (!status?.accountLogin || !status.accountServer || !seriesSymbol.trim()) {
      sessionCalendarError = 'An authenticated account and broker-native symbol are required.';
      return;
    }
    sessionCalendarLoading = true;
    sessionCalendarError = '';
    sessionCalendar = null;
    try {
      sessionCalendar = await getMt5SessionCalendar(status.accountServer, status.accountLogin, seriesSymbol.trim());
    } catch (cause) {
      sessionCalendarError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      sessionCalendarLoading = false;
    }
  }

  async function calculateRiskSizing() {
    if (riskSizingLoading) return;
    if (!status?.accountLogin || !status.accountServer || !seriesSymbol.trim()) {
      riskSizingError = 'An authenticated MT5 account and selected broker symbol are required.';
      return;
    }
    const stopLossPrice = Number(riskStopLoss);
    const riskPercentValue = Number(riskPercent);
    if (!Number.isFinite(stopLossPrice) || stopLossPrice <= 0 || !Number.isFinite(riskPercentValue)) {
      riskSizingError = 'Enter a valid positive stop-loss price and risk percentage.';
      return;
    }
    riskSizingLoading = true;
    riskSizingError = '';
    riskSizingResult = null;
    try {
      const request: Mt5RiskSizingRequest = {
        brokerServer: status.accountServer, accountLogin: status.accountLogin,
        symbol: seriesSymbol.trim(), side: riskSide, stopLossPrice, riskPercent: riskPercentValue
      };
      riskSizingResult = await calculateMt5RiskSizing(request);
    } catch (cause) {
      riskSizingError = cause instanceof Error ? cause.message : String(cause);
    } finally {
      riskSizingLoading = false;
    }
  }

  async function revealPairing() {
    if (setupLoading) return;
    setupLoading = true;
    error = '';
    notice = '';
    try {
      setup = await getMt5BridgeSetup();
      revealToken = true;
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      setupLoading = false;
    }
  }

  async function copyToken() {
    if (!setup) return;
    try {
      await navigator.clipboard.writeText(setup.pairingToken);
      notice = 'Pairing token copied. Treat it as a local access credential.';
    } catch {
      notice = 'Clipboard access is unavailable. Select and copy the revealed token manually.';
    }
  }

  function closeToken() {
    revealToken = false;
    setup = null;
  }

  onMount(() => {
    void refreshStatus();
    const timer = window.setInterval(() => void refreshStatus(), 1500);
    return () => window.clearInterval(timer);
  });

  $: snapshot = status?.snapshot ?? null;
  $: account = snapshot?.account ?? null;
  $: symbol = snapshot?.symbol ?? null;
  $: positions = snapshot?.positions ?? [];
  $: pendingOrders = snapshot?.pendingOrders ?? [];
  $: accountRisk = status?.accountRisk ?? null;
  $: marketStorage = status?.marketDataStorage ?? null;
  $: connectionLabel = status?.connected ? 'CONNECTED' : status?.stale ? 'STALE' : status?.listening ? 'WAITING FOR MT5' : 'BRIDGE UNAVAILABLE';

  function pendingOrderTypeLabel(orderType: number) {
    return ({ 2: 'BUY LIMIT', 3: 'SELL LIMIT', 4: 'BUY STOP', 5: 'SELL STOP', 6: 'BUY STOP LIMIT', 7: 'SELL STOP LIMIT' } as Record<number, string>)[orderType] ?? 'UNKNOWN';
  }

  function stopRiskLabel(statusValue: string) {
    return statusValue === 'CALCULATED' ? 'Calculated' : statusValue === 'UNPROTECTED' ? 'No stop loss' : 'Unresolved';
  }
</script>

<section class="mt5-page">
  <div class="page-heading">
    <div>
      <div class="eyebrow">BROKER CONNECTIVITY / METATrader 5</div>
      <h1>MT5 bridge</h1>
      <p>Native Rust service receives authenticated account telemetry, multi-symbol ticks, and closed bars from a read-only MQL5 Expert Advisor. Frames are committed to a separate SQLite market-data store before acknowledgement. No order submission is exposed by this bridge.</p>
    </div>
    <span class:good={status?.connected} class:warn={status?.stale || !status?.listening} class="connection-pill">
      {#if status?.connected}<Wifi size={14}/>{:else if status?.stale}<AlertTriangle size={14}/>{:else}<WifiOff size={14}/>{/if}
      {connectionLabel}
    </span>
  </div>

  {#if error}
    <div class="mt5-alert danger"><AlertTriangle size={17}/><div><strong>Bridge issue</strong><p>{error}</p></div></div>
  {/if}
  {#if notice}
    <div class="mt5-alert info"><ShieldCheck size={17}/><div><p>{notice}</p></div></div>
  {/if}

  <section class="mt5-grid">
    <article class="panel mt5-stat">
      <div class="mt5-stat-label"><Network size={15}/> Listener</div>
      <strong>{status?.listening ? 'Listening' : 'Unavailable'}</strong>
      <span>{status?.address ?? '127.0.0.1:47650'}</span>
    </article>
    <article class="panel mt5-stat">
      <div class="mt5-stat-label"><Activity size={15}/> Heartbeat</div>
      <strong>{age(status?.heartbeatAgeMs ?? null)}</strong>
      <span>{timestamp(status?.lastHeartbeatAtMs ?? null)}</span>
    </article>
    <article class="panel mt5-stat">
      <div class="mt5-stat-label"><Database size={15}/> Account</div>
      <strong>{status?.connected ? status.accountLogin : status?.accountLogin ? 'Stale snapshot' : 'Not connected'}</strong>
      <span>{status?.connected ? status.accountServer : status?.accountServer ? `Last known: ${status.accountServer}` : 'No broker metadata'}</span>
    </article>
    <article class="panel mt5-stat">
      <div class="mt5-stat-label"><Activity size={15}/> Position count</div>
      <strong>{status?.connected && snapshot ? positions.length : '—'}</strong>
      <span>{status?.connected ? `Accepted snapshot sequence ${status.lastSequence}` : 'No current position snapshot'}</span>
    </article>
    <article class="panel mt5-stat">
      <div class="mt5-stat-label"><Activity size={15}/> Pending orders</div>
      <strong>{status?.connected && snapshot ? pendingOrders.length : '—'}</strong>
      <span>{status?.connected ? 'Active broker orders in the accepted snapshot' : 'No current order inventory'}</span>
    </article>
    <article class="panel mt5-stat">
      <div class="mt5-stat-label"><Database size={15}/> Durable market data</div>
      <strong>{marketStorage?.storageAvailable ? `${number(marketStorage.ticksTotal, 0)} ticks` : 'Storage unavailable'}</strong>
      <span>{marketStorage ? `${number(marketStorage.barsTotal, 0)} closed bars · ${marketStorage.symbolsTotal} symbols` : 'No durable ingestion store'}</span>
    </article>
  </section>

  <section class="panel mt5-data-workbench">
    <div class="panel-heading">
      <div><h2>Stored market-data assurance</h2><p>Query the durable broker dataset by symbol and timeframe. The report includes revision provenance and interval anomalies for the complete selected window.</p></div>
      <span class="source-tag"><Database size={13}/> SQLITE DATASET</span>
    </div>
    <div class="mt5-query-controls">
      <label>Broker symbol<input bind:value={seriesSymbol} oninput={() => riskSizingResult = null} maxlength="64" placeholder="e.g. XAUUSD.a" autocomplete="off" /></label>
      <label>Timeframe<select bind:value={seriesTimeframe}><option value="M1">M1</option><option value="M5">M5</option><option value="M15">M15</option><option value="M30">M30</option><option value="H1">H1</option><option value="H4">H4</option><option value="D1">D1</option></select></label>
      <label>Window<select bind:value={seriesRangeDays}><option value="1">Last 24 hours</option><option value="7">Last 7 days</option><option value="30">Last 30 days</option><option value="90">Last 90 days</option><option value="180">Last 180 days</option><option value="365">Last 365 days</option></select></label>
      <button class="button-primary" onclick={() => queryStoredSeries(null)} disabled={seriesLoading || !status?.accountLogin || !status?.accountServer}><RefreshCw size={14}/>{seriesLoading ? 'Analyzing…' : 'Inspect stored series'}</button>
      <button class="button-secondary" onclick={inspectInstrumentSpecification} disabled={instrumentSpecificationLoading || !status?.accountLogin || !status?.accountServer}><Database size={14}/>{instrumentSpecificationLoading ? 'Loading contract…' : 'Inspect broker contract'}</button>
      <button class="button-secondary" onclick={inspectSessionCalendar} disabled={sessionCalendarLoading || !status?.accountLogin || !status?.accountServer}><Activity size={14}/>{sessionCalendarLoading ? 'Loading sessions…' : 'Inspect broker sessions'}</button>
    </div>
    <div class="mt5-risk-sizing">
      <div class="mt5-instrument-heading"><div><strong>Broker-native position risk sizing</strong><span>Uses live MT5 equity, executable bid/ask, broker loss tick value and volume constraints</span></div><span class="source-tag">CALCULATION ONLY</span></div>
      <div class="mt5-query-controls">
        <label>Direction<select bind:value={riskSide} onchange={() => riskSizingResult = null}><option value="BUY">BUY</option><option value="SELL">SELL</option></select></label>
        <label>Stop-loss price<input bind:value={riskStopLoss} oninput={() => riskSizingResult = null} inputmode="decimal" placeholder="Broker price, e.g. 2300.00" autocomplete="off" /></label>
        <label>Equity risk %<input bind:value={riskPercent} oninput={() => riskSizingResult = null} inputmode="decimal" placeholder="1.0" autocomplete="off" /></label>
        <button class="button-primary" onclick={calculateRiskSizing} disabled={riskSizingLoading || !status?.connected || !status?.accountLogin || !status?.accountServer || !seriesSymbol.trim()}>{riskSizingLoading ? 'Calculating…' : 'Calculate risk-sized volume'}</button>
      </div>
      {#if riskSizingError}<div class="mt5-alert danger"><AlertTriangle size={16}/><div><strong>Risk sizing blocked</strong><p>{riskSizingError}</p></div></div>{/if}
      {#if riskSizingResult}
        <div class="mt5-risk-result">
          <div class="mt5-instrument-heading"><div><strong>{riskSizingResultStale ? 'Historical sizing result' : riskSizingResult.decision === 'BLOCKED' ? 'Sizing rejected' : 'Risk-sized volume calculated'}</strong><span>{riskSizingResult.symbol} · {riskSizingResult.side} · {riskSizingResult.accountCurrency}</span></div><span class="source-tag" class:negative={riskSizingResult.decision === 'BLOCKED' || riskSizingResultStale}>{riskSizingResultStale ? 'STALE — RE-CALCULATE' : riskSizingResult.decision}</span></div>
          {#if riskSizingResultStale}<div class="mt5-alert danger"><AlertTriangle size={16}/><div><strong>Snapshot superseded</strong><p>The authenticated MT5 snapshot has advanced, disconnected, or become stale since this calculation. Its volume is historical only and must not be relied on. Recalculate against the current account inventory.</p></div></div>{/if}
          <div class="mt5-quality-grid">
            <div><small>{riskSizingResultStale ? 'Historical calculated volume' : 'Approved volume'}</small><strong>{riskSizingResultStale ? 'STALE — RE-CALCULATE' : `${number(riskSizingResult.normalizedVolume, 4)} lots`}</strong></div>
            <div><small>Proposed volume</small><strong>{number(riskSizingResult.proposedVolume, 4)} lots</strong></div>
            <div><small>Requested trade budget</small><strong>{money(riskSizingResult.riskBudget, riskSizingResult.accountCurrency)}</strong></div>
            <div><small>Effective trade budget</small><strong>{money(riskSizingResult.effectiveRiskBudget, riskSizingResult.accountCurrency)}</strong></div>
            <div><small>Remaining account capacity</small><strong>{money(riskSizingResult.accountAvailableRiskBudget, riskSizingResult.accountCurrency)}</strong></div>
            <div><small>Candidate stop-loss</small><strong>{money(riskSizingResult.estimatedLoss, riskSizingResult.accountCurrency)}</strong></div>
            <div><small>Current portfolio stop risk</small><strong>{money(riskSizingResult.accountStopRisk, riskSizingResult.accountCurrency)} ({number(riskSizingResult.accountStopRiskPercent, 3)}%)</strong></div>
            <div><small>Projected portfolio stop risk</small><strong>{money(riskSizingResult.projectedAccountStopRisk, riskSizingResult.accountCurrency)}</strong></div>
            <div><small>Open same-direction volume</small><strong>{number(riskSizingResult.existingSymbolVolume, 4)} lots</strong></div>
            <div><small>Pending same-direction volume</small><strong>{number(riskSizingResult.pendingSymbolVolume, 4)} lots</strong></div>
            <div><small>Combined / broker limit</small><strong>{number(riskSizingResult.totalDirectionalSymbolVolume, 4)} / {number(riskSizingResult.volumeLimit, 4)} lots</strong></div>
            <div><small>Native candidate margin</small><strong>{riskSizingResult.candidateMargin === null ? '—' : money(riskSizingResult.candidateMargin, riskSizingResult.accountCurrency)}</strong></div>
            <div><small>Projected free margin</small><strong>{riskSizingResult.projectedFreeMargin === null ? '—' : money(riskSizingResult.projectedFreeMargin, riskSizingResult.accountCurrency)}</strong></div>
            <div><small>Stressed margin level</small><strong>{riskSizingResult.projectedMarginLevelPct === null ? '—' : `${number(riskSizingResult.projectedMarginLevelPct)}%`}</strong></div>
            <div><small>Quote age / native check</small><strong>{number(riskSizingResult.quoteAgeMs, 0)} ms / {riskSizingResult.marginPreflightAtMs === null ? 'not completed' : timestamp(riskSizingResult.marginPreflightAtMs)}</strong></div>
          </div>
          {#if riskSizingResult.reasons.length > 0}<div class="mt5-alert danger"><AlertTriangle size={16}/><div><strong>Blocking conditions</strong>{#each riskSizingResult.reasons as reason}<p>{reason}</p>{/each}</div></div>{/if}
          <div class="mt5-manifest"><small>Account snapshot sequence {riskSizingResult.snapshotSequence} · contract revision {riskSizingResult.specificationRevision} · {riskSizingResult.contractCompleteness} · SHA-256</small><code>{riskSizingResult.specificationFingerprintSha256}</code></div>
          <p class="mt5-quality-note">{riskSizingResult.marginEstimation}. {riskSizingResult.commissionEstimation}. This result is not an order, does not reserve margin, and does not guarantee the realized loss. No live or paper order is submitted.</p>
          <div class="mt5-manifest"><small>Calculation limitations</small>{#each riskSizingResult.limitations as limitation}<span>{limitation}</span>{/each}</div>
        </div>
      {/if}
    </div>
    {#if instrumentSpecificationError}<div class="mt5-alert danger"><AlertTriangle size={16}/><div><strong>Instrument registry query failed</strong><p>{instrumentSpecificationError}</p></div></div>{/if}
    {#if instrumentSpecification}
      <div class="mt5-instrument-registry">
        <div class="mt5-instrument-heading"><div><strong>Broker-reported instrument specification</strong><span>{instrumentSpecification.brokerServer} · {instrumentSpecification.accountLogin} · {instrumentSpecification.specification.symbol}</span></div><span class="source-tag">REVISION {instrumentSpecification.revision}</span></div>
        <div class="mt5-quality-grid">
          <div><small>Contract size</small><strong>{number(instrumentSpecification.specification.contractSize, 6)}</strong></div>
          <div><small>Tick value · profit / loss</small><strong>{number(instrumentSpecification.specification.tickValueProfit, 6)} / {number(instrumentSpecification.specification.tickValueLoss, 6)}</strong></div>
          <div><small>Volume min / step / max</small><strong>{number(instrumentSpecification.specification.volumeMin, 4)} / {number(instrumentSpecification.specification.volumeStep, 4)} / {number(instrumentSpecification.specification.volumeMax, 4)}</strong></div>
          <div><small>Volume limit</small><strong>{instrumentSpecification.specification.volumeLimit > 0 ? number(instrumentSpecification.specification.volumeLimit, 4) : 'No broker limit reported'}</strong></div>
          <div><small>Currency base / profit / margin</small><strong>{[instrumentSpecification.specification.currencyBase, instrumentSpecification.specification.currencyProfit, instrumentSpecification.specification.currencyMargin].filter(Boolean).join(' / ') || 'Not reported'}</strong></div>
          <div><small>Stops / freeze level</small><strong>{instrumentSpecification.specification.stopsLevelPoints} / {instrumentSpecification.specification.freezeLevelPoints} points</strong></div>
          <div><small>Calculation / execution mode</small><strong>{instrumentSpecification.specification.calculationMode} / {instrumentSpecification.specification.executionMode}</strong></div>
          <div><small>Swap long / short</small><strong>{number(instrumentSpecification.specification.swapLong, 6)} / {number(instrumentSpecification.specification.swapShort, 6)}</strong></div>
        </div>
        <div class="mt5-manifest"><small>Specification fingerprint · {instrumentSpecification.completeness}</small><code>{instrumentSpecification.fingerprintSha256}</code></div>
        <p class="mt5-quality-note">This is a partial broker snapshot for the primary symbol. It is not a complete margin model or a live-order authorization. Revision history is persisted natively; this view displays the current revision.</p>
      </div>
    {/if}
    {#if sessionCalendarError}<div class="mt5-alert danger"><AlertTriangle size={16}/><div><strong>Session calendar query failed</strong><p>{sessionCalendarError}</p></div></div>{/if}
    {#if sessionCalendar}
      <div class="mt5-instrument-registry">
        <div class="mt5-instrument-heading"><div><strong>Broker-reported weekly trading sessions</strong><span>{sessionCalendar.brokerServer} · {sessionCalendar.accountLogin} · {sessionCalendar.schedule.symbol} · captured {timestamp(sessionCalendar.schedule.capturedAtMs)}</span></div><span class="source-tag">REVISION {sessionCalendar.revision}</span></div>
        <div class="mt5-session-grid">
          {#each sessionCalendar.schedule.days as day (day.dayOfWeek)}
            <div><small>{weekdayNames[day.dayOfWeek] ?? `Weekday ${day.dayOfWeek}`}</small>
              {#if day.sessions.length === 0}<strong>No session reported</strong>{:else}
                {#each day.sessions as session (session.index)}<strong>{sessionClock(session.fromSeconds)} – {sessionClock(session.toSeconds)}{session.toSeconds < session.fromSeconds ? ' · overnight' : ''}</strong>{/each}
              {/if}
            </div>
          {/each}
        </div>
        <div class="mt5-manifest"><small>Calendar fingerprint · {sessionCalendar.timeBasis}</small><code>{sessionCalendar.fingerprintSha256}</code></div>
        <p class="mt5-quality-note">This is the broker's recurring weekly schedule, not a dated holiday calendar. It does not resolve timezone offsets or daylight-saving transitions. ATOS retains interval anomalies as unclassified candidates and does not infer that a market was closed.</p>
      </div>
    {/if}
    {#if seriesError}<div class="mt5-alert danger"><AlertTriangle size={16}/><div><strong>Series query failed</strong><p>{seriesError}</p></div></div>{/if}
    {#if seriesResult}
      <div class="mt5-query-result-identity"><strong>{seriesResult.brokerServer} · Account {seriesResult.accountLogin} · {seriesResult.symbol} · {seriesResult.timeframe}</strong><span>{timestamp(seriesResult.fromMs)} — {timestamp(seriesResult.toMs)} · range anchored to the last authenticated broker server time</span></div>
      <div class="mt5-quality-grid">
        <div><small>Stored bars</small><strong>{number(seriesResult.quality.barsTotal, 0)}</strong></div>
        <div><small>Revision events</small><strong>{number(seriesResult.quality.revisionsTotal, 0)}</strong></div>
        <div><small>Interval anomalies</small><strong class:negative={seriesResult.quality.intervalAnomalyCount > 0}>{number(seriesResult.quality.intervalAnomalyCount, 0)}</strong></div>
        <div><small>Largest observed interval</small><strong>{seriesResult.quality.largestIntervalMs === null ? '—' : `${(seriesResult.quality.largestIntervalMs / 60000).toFixed(1)} min`}</strong></div>
      </div>
      <p class="mt5-quality-note">{seriesResult.quality.continuityInterpretation}</p>
      <div class="mt5-manifest"><small>Dataset manifest SHA-256 · reproducibility identifier</small><code>{seriesResult.datasetFingerprintSha256}</code></div>
      {#if seriesResult.quality.gapCandidates.length > 0}
        <div class="mt5-gap-list"><strong>First interval anomaly candidates</strong>{#each seriesResult.quality.gapCandidates.slice(0, 5) as gap}<span>{timestamp(gap.previousBarTimeMs)} → {timestamp(gap.nextBarTimeMs)} · {(gap.elapsedMs / 60000).toFixed(1)} min</span>{/each}</div>
      {:else}
        <p class="mt5-quality-note">No interval anomaly above the nominal 1.5× timeframe threshold was observed in this window. This is not a broker-session completeness certificate.</p>
      {/if}
      <div class="mt5-series-table-wrap">
        <table class="mt5-series-table"><thead><tr><th>Bar time</th><th>Open</th><th>High</th><th>Low</th><th>Close</th><th>Ticks</th><th>Revision</th><th>Payload SHA-256</th></tr></thead><tbody>
          {#each seriesBars.slice(-8) as bar}<tr><td>{timestamp(bar.timeMs)}</td><td>{number(bar.open, 6)}</td><td>{number(bar.high, 6)}</td><td>{number(bar.low, 6)}</td><td>{number(bar.close, 6)}</td><td>{number(bar.tickVolume, 0)}</td><td>{bar.revision}</td><td><code>{bar.payloadSha256.slice(0, 16)}…</code></td></tr>{/each}
        </tbody></table>
        {#if seriesBars.length === 0}<div class="mt5-empty"><strong>No stored bars in this range</strong><span>Check the broker symbol spelling, timeframe subscription, and ingestion history.</span></div>{/if}
      </div>
      <div class="mt5-query-footer"><span>Showing {number(seriesBars.length, 0)} of {number(seriesResult.quality.barsTotal, 0)} stored bars · oldest-first pages, 1,000 rows maximum</span>{#if seriesResult.hasMore && seriesResult.nextAfterMs !== null}<button class="button-secondary compact" onclick={() => queryStoredSeries(seriesResult?.nextAfterMs ?? null)} disabled={seriesLoading}>Load next page</button>{/if}</div>
    {/if}
    {#if !status?.accountLogin || !status?.accountServer}<p class="mt5-quality-note">Stored-series queries require an account identity learned from an authenticated MT5 terminal. No broker identity is guessed.</p>{/if}
  </section>

  <section class="panel mt5-setup">
    <div class="panel-heading">
      <div><h2>Connect a MetaTrader 5 terminal</h2><p>Pair the Expert Advisor with this local desktop instance.</p></div>
      <button class="button-secondary compact" onclick={refreshStatus} disabled={loading}><RefreshCw size={13}/> Refresh status</button>
    </div>
    <div class="mt5-steps">
      <div><span>01</span><div><strong>Reveal the pairing token</strong><p>The token is stored in the operating-system credential vault and returned only when requested here.</p></div></div>
      <div><span>02</span><div><strong>Install the Expert Advisor</strong><p>Copy <code>MQL5/Experts/ATOS_Bridge.mq5</code> into the terminal's <code>MQL5/Experts</code> folder and compile it in MetaEditor.</p></div></div>
      <div><span>03</span><div><strong>Attach it to a chart</strong><p>Set the host to <code>127.0.0.1</code>, port to <code>47650</code>, paste the token, and select the symbol to observe.</p></div></div>
      <div><span>04</span><div><strong>Allow local socket access</strong><p>In MT5 terminal settings, add <code>127.0.0.1</code> to the permitted addresses for MQL5 network functions. Keep the bridge on this computer.</p></div></div>
    </div>
    <div class="mt5-actions">
      {#if !revealToken}
        <button class="button-primary" onclick={revealPairing} disabled={setupLoading || !status?.listening}>{#if setupLoading}<RefreshCw size={14}/>{:else}<LockKeyhole size={14}/>{/if} Reveal EA pairing token</button>
      {:else}
        <button class="button-secondary" onclick={closeToken}><EyeOff size={14}/> Hide token</button>
      {/if}
      {#if status?.lastError}<span class="mt5-inline-warning"><AlertTriangle size={13}/> Setup remains blocked until listener and credential storage are available.</span>{/if}
    </div>
    {#if revealToken && setup}
      <div class="mt5-token-box">
        <div class="mt5-token-heading"><strong>Local pairing credential</strong><span>Do not share or publish</span></div>
        <code>{setup.pairingToken}</code>
        <button class="button-secondary compact" onclick={copyToken}><Copy size={13}/> Copy token</button>
        <p>{setup.transportSecurity}</p>
      </div>
    {/if}
  </section>

  {#if status?.connected && snapshot && account && symbol}
    <section class="panel mt5-account">
      <div class="panel-heading"><div><h2>Broker account snapshot</h2><p>Account data received from the connected MT5 terminal. Values are not inferred from the paper account.</p></div><span class="source-tag"><CheckCircle2 size={13}/> MT5 TELEMETRY</span></div>
      <div class="mt5-account-grid">
        <div><small>Balance</small><strong>{money(account.balance, account.currency)}</strong></div>
        <div><small>Equity</small><strong>{money(account.equity, account.currency)}</strong></div>
        <div><small>Floating P/L</small><strong class:positive={account.profit >= 0} class:negative={account.profit < 0}>{money(account.profit, account.currency)}</strong></div>
        <div><small>Used margin</small><strong>{money(account.margin, account.currency)}</strong></div>
        <div><small>Free margin</small><strong>{money(account.freeMargin, account.currency)}</strong></div>
        <div><small>Margin level</small><strong>{account.marginLevelPct > 0 ? `${number(account.marginLevelPct)}%` : '—'}</strong></div>
        <div><small>Initial / maintenance margin</small><strong>{money(account.marginInitial, account.currency)} / {money(account.marginMaintenance, account.currency)}</strong></div>
        <div><small>Broker margin call</small><strong>{number(account.marginCallLevel)}{account.marginStopoutMode === 0 ? '%' : ` ${account.currency}`}</strong></div>
        <div><small>Broker stop out</small><strong>{number(account.stopOutLevel)}{account.marginStopoutMode === 0 ? '%' : ` ${account.currency}`}</strong></div>
        <div><small>Margin policy</small><strong>{account.marginStopoutMode === 0 ? 'Percentage' : 'Money'} · {account.marginMode === 0 ? 'Retail netting' : account.marginMode === 1 ? 'Exchange' : 'Retail hedging'}</strong></div>
        <div><small>Leverage / pending-order cap</small><strong>1:{number(account.leverage, 0)} / {number(account.accountLimitOrders, 0)}</strong></div>
        <div><small>Trading permissions</small><strong>{account.tradeAllowed ? 'All permission flags enabled' : 'One or more permission flags disabled'}</strong></div>
      </div>
      {#if accountRisk}
        <div class="mt5-manifest">
          <small>Account-wide stop-loss risk inventory · {accountRisk.accountCurrency}</small>
          <div class="mt5-quality-grid">
            <div><small>Known stop risk</small><strong>{money(accountRisk.currentStopRisk, accountRisk.accountCurrency)} ({number(accountRisk.currentStopRiskPercent, 3)}% equity)</strong></div>
            <div><small>Protected exposure</small><strong>{accountRisk.protectedPositionCount} positions · {accountRisk.protectedPendingOrderCount} pending</strong></div>
            <div><small>Unprotected exposure</small><strong>{accountRisk.unprotectedPositionCount} positions · {accountRisk.unprotectedPendingOrderCount} pending</strong></div>
            <div><small>Unresolved stop risk</small><strong>{accountRisk.unresolvedPositionRiskCount} positions · {accountRisk.unresolvedPendingOrderRiskCount} pending</strong></div>
          </div>
          <p class="mt5-quality-note">Any unprotected or unresolved position/order blocks new risk sizing. Known stop-loss amounts are broker-calculated estimates, not guaranteed loss ceilings.</p>
        </div>
      {:else}
        <div class="mt5-alert danger"><AlertTriangle size={16}/><div><strong>Account risk inventory unavailable</strong><p>ATOS could not reconcile broker stop-loss risk from this snapshot. New risk sizing must remain blocked.</p></div></div>
      {/if}
      <div class="mt5-symbol-strip">
        <div><small>Watched symbol</small><strong>{symbol.name}</strong></div>
        <div><small>Bid / Ask</small><strong>{number(symbol.bid, symbol.digits)} / {number(symbol.ask, symbol.digits)}</strong></div>
        <div><small>Spread</small><strong>{number(symbol.ask - symbol.bid, symbol.digits)} ({symbol.digits} digits)</strong></div>
        <div><small>Lot constraints</small><strong>{number(symbol.volumeMin, 4)} – {number(symbol.volumeMax, 4)} · step {number(symbol.volumeStep, 4)}</strong></div>
        <div><small>Quote freshness</small><strong class:positive={!status?.quoteStale} class:negative={status?.quoteStale}>{status?.quoteStale ? 'STALE QUOTE' : 'Fresh observation'}</strong><small>{timestamp(symbol.quoteTimeMs)}</small></div>
      </div>
      <div class="mt5-permissions">
        <span class:pass={snapshot.terminal.connected}><CheckCircle2 size={13}/> Terminal connected</span>
        <span class:pass={account.accountTradeAllowed}><CheckCircle2 size={13}/> Account permission</span>
        <span class:pass={account.terminalTradeAllowed}><CheckCircle2 size={13}/> Terminal permission</span>
        <span class:pass={snapshot.terminal.algoTradingAllowed}><CheckCircle2 size={13}/> Algo trading permission</span>
      </div>
    </section>

    <section class="panel mt5-positions">
      <div class="panel-heading"><div><h2>Open positions</h2><p>Position data is a broker snapshot and may become stale when the bridge disconnects.</p></div><span class="mode-pill">{positions.length} POSITIONS</span></div>
      {#if positions.length}
        <div class="table-scroll"><table><thead><tr><th>Ticket</th><th>Symbol</th><th>Side</th><th>Volume</th><th>Open</th><th>Current</th><th>Floating P/L</th><th>SL / TP</th><th>Stop-loss risk</th></tr></thead><tbody>
          {#each positions as position (position.ticket)}
            <tr><td><code>{position.ticket}</code></td><td>{position.symbol}</td><td>{position.positionType === 0 ? 'BUY' : 'SELL'}</td><td>{number(position.volume, 4)}</td><td>{number(position.priceOpen, 8)}</td><td>{number(position.priceCurrent, 8)}</td><td class:positive={position.profit >= 0} class:negative={position.profit < 0}>{money(position.profit + position.swap, account.currency)}</td><td>{position.stopLoss > 0 ? number(position.stopLoss, 8) : '—'} / {position.takeProfit > 0 ? number(position.takeProfit, 8) : '—'}</td><td>{stopRiskLabel(position.riskStatus)}{position.riskAtStopLoss === null ? '' : ` · ${money(position.riskAtStopLoss, account.currency)}`}</td></tr>
          {/each}
        </tbody></table></div>
      {:else}
        <div class="mt5-empty"><CircleSlash size={20}/><strong>No open positions</strong><span>The connected terminal reported zero open positions.</span></div>
      {/if}
    </section>

    <section class="panel mt5-positions">
      <div class="panel-heading"><div><h2>Active pending orders</h2><p>Remaining volume and stop-loss exposure are included in account-wide and directional broker-limit checks.</p></div><span class="mode-pill">{pendingOrders.length} PENDING</span></div>
      {#if pendingOrders.length}
        <div class="table-scroll"><table><thead><tr><th>Ticket</th><th>Symbol</th><th>Order type</th><th>Remaining / initial</th><th>Entry / stop-limit</th><th>SL / TP</th><th>Stop-loss risk</th></tr></thead><tbody>
          {#each pendingOrders as order (order.ticket)}
            <tr><td><code>{order.ticket}</code></td><td>{order.symbol}</td><td>{pendingOrderTypeLabel(order.orderType)}</td><td>{number(order.volumeCurrent, 4)} / {number(order.volumeInitial, 4)}</td><td>{number(order.priceOpen, 8)}{order.priceStopLimit > 0 ? ` / ${number(order.priceStopLimit, 8)}` : ''}</td><td>{order.stopLoss > 0 ? number(order.stopLoss, 8) : '—'} / {order.takeProfit > 0 ? number(order.takeProfit, 8) : '—'}</td><td>{stopRiskLabel(order.riskStatus)}{order.riskAtStopLoss === null ? '' : ` · ${money(order.riskAtStopLoss, account.currency)}`}</td></tr>
          {/each}
        </tbody></table></div>
      {:else}
        <div class="mt5-empty"><CircleSlash size={20}/><strong>No active pending orders</strong><span>The connected terminal reported zero active pending orders.</span></div>
      {/if}
    </section>
  {:else}
    <section class="panel mt5-empty-panel">
      <WifiOff size={24}/>
      <strong>{status?.stale ? 'MT5 telemetry is stale' : 'Waiting for an authenticated terminal'}</strong>
      <span>{status?.stale ? 'The last broker snapshot is retained as historical context only. It is not shown as current account state.' : 'No account, quote, or position values are fabricated. Attach the EA after the local listener is ready.'}</span>
    </section>
  {/if}

  <section class="mt5-alert security"><ShieldCheck size={18}/><div><strong>Execution and risk boundary</strong><p>This integration is read-only. The MQL5 adapter contains no <code>OrderSend</code>, position modification, or pending-order functions. The broker-native position sizer is calculation-only. Live execution remains disabled until a separate Rust mandate, complete margin-aware risk gateway, durable order outbox, broker reconciliation, and fault-injection test suite are implemented and verified.</p></div></section>
</section>

<style>
  .mt5-page{display:grid;gap:20px}.connection-pill,.source-tag,.mt5-permissions span{display:inline-flex;align-items:center;gap:7px}.connection-pill{padding:8px 11px;border:1px solid var(--border);border-radius:999px;font-size:11px;font-weight:800;letter-spacing:.08em;color:var(--muted)}.connection-pill.good{color:var(--green);border-color:color-mix(in srgb,var(--green) 35%,var(--border))}.connection-pill.warn{color:var(--amber);border-color:color-mix(in srgb,var(--amber) 35%,var(--border))}.mt5-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px}.mt5-stat{padding:16px;display:grid;gap:9px}.mt5-stat-label{display:flex;gap:8px;align-items:center;color:var(--muted);font-size:11px;text-transform:uppercase;letter-spacing:.08em}.mt5-stat strong{font-size:17px}.mt5-stat>span{font-size:11px;color:var(--muted);overflow-wrap:anywhere}.panel-heading{display:flex;justify-content:space-between;align-items:center;gap:14px}.mt5-setup,.mt5-account,.mt5-positions{padding:20px}.mt5-steps{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px;margin:22px 0}.mt5-steps>div{display:flex;gap:13px;align-items:flex-start;padding:14px;border:1px solid var(--border);border-radius:12px;background:var(--surface-2)}.mt5-steps>div>span{display:grid;place-items:center;flex:0 0 28px;height:28px;border-radius:8px;background:var(--cyan-dark);color:var(--cyan);font-size:11px;font-weight:800}.mt5-steps strong{font-size:13px}.mt5-steps p,.mt5-token-box p{font-size:12px;line-height:1.55;color:var(--muted);margin:5px 0 0}.mt5-actions{display:flex;align-items:center;gap:12px;flex-wrap:wrap}.mt5-token-box{display:grid;gap:10px;padding:15px;margin-top:16px;border:1px solid var(--border);border-radius:12px;background:var(--surface-2)}.mt5-token-heading{display:flex;justify-content:space-between;gap:12px}.mt5-token-heading span{color:var(--amber);font-size:11px;font-weight:700}.mt5-token-box code{font-size:12px;overflow-wrap:anywhere;user-select:all;padding:10px;background:var(--surface);border:1px solid var(--border);border-radius:8px}.mt5-alert{display:flex;gap:12px;align-items:flex-start;padding:13px 15px;border:1px solid var(--border);border-radius:12px}.mt5-alert p{margin:4px 0 0;font-size:12px;line-height:1.55;color:var(--muted)}.mt5-alert.danger{border-color:color-mix(in srgb,var(--red) 35%,var(--border));color:var(--red)}.mt5-alert.info{color:var(--cyan)}.mt5-alert.security{color:var(--green);background:var(--surface-2)}.mt5-inline-warning{display:inline-flex;gap:6px;align-items:center;font-size:11px;color:var(--amber)}.mt5-account-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:15px;padding:18px 0}.mt5-account-grid>div{display:grid;gap:5px}.mt5-account-grid small,.mt5-symbol-strip small{font-size:10px;color:var(--muted);text-transform:uppercase;letter-spacing:.07em}.mt5-account-grid strong{font-size:15px}.mt5-symbol-strip{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px;padding:14px;border:1px solid var(--border);border-radius:10px;background:var(--surface-2)}.mt5-symbol-strip>div{display:grid;gap:5px}.mt5-symbol-strip strong{font-size:12px}.mt5-permissions{display:flex;gap:15px;flex-wrap:wrap;padding-top:15px;color:var(--muted);font-size:11px}.mt5-permissions span.pass{color:var(--green)}.mt5-positions .table-scroll{overflow-x:auto}.mt5-positions table{width:100%;border-collapse:collapse;font-size:12px;white-space:nowrap}.mt5-positions th,.mt5-positions td{padding:11px 9px;border-bottom:1px solid var(--border);text-align:left}.mt5-positions th{font-size:10px;color:var(--muted);text-transform:uppercase;letter-spacing:.07em}.mt5-empty,.mt5-empty-panel{display:flex;align-items:center;justify-content:center;flex-direction:column;gap:7px;padding:34px;color:var(--muted);text-align:center}.mt5-empty strong,.mt5-empty-panel strong{color:var(--text);font-size:13px}.mt5-empty span,.mt5-empty-panel span{font-size:12px}.mt5-empty-panel{min-height:150px;border:1px dashed var(--border);border-radius:14px}.positive{color:var(--green)}.negative{color:var(--red)}.source-tag{font-size:10px;letter-spacing:.08em;color:var(--green);font-weight:800}.mt5-page code{font-size:.92em}
  .mt5-data-workbench{padding:20px;display:grid;gap:16px}.mt5-query-controls{display:grid;grid-template-columns:minmax(180px,2fr) minmax(100px,.8fr) minmax(130px,1fr) auto;gap:12px;align-items:end}.mt5-query-controls label{display:grid;gap:6px;font-size:11px;color:var(--muted);font-weight:700}.mt5-query-controls input,.mt5-query-controls select{width:100%;min-width:0;padding:10px 11px;border:1px solid var(--border);border-radius:9px;background:var(--surface-2);color:var(--text);font:inherit}.mt5-query-result-identity{display:grid;gap:5px}.mt5-query-result-identity strong{font-size:12px}.mt5-query-result-identity span{font-size:10px;color:var(--muted)}.mt5-quality-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px}.mt5-quality-grid>div{display:grid;gap:6px;padding:13px;border:1px solid var(--border);border-radius:10px;background:var(--surface-2)}.mt5-quality-grid small{color:var(--muted);font-size:10px;text-transform:uppercase;letter-spacing:.07em}.mt5-quality-grid strong{font-size:16px}.mt5-quality-note{margin:0;color:var(--muted);font-size:11px;line-height:1.6}.mt5-manifest{display:grid;gap:6px;padding:11px;border:1px solid var(--border);border-radius:9px;background:var(--surface-2)}.mt5-manifest small{color:var(--muted);font-size:10px;text-transform:uppercase;letter-spacing:.06em}.mt5-manifest code{font-size:10px;overflow-wrap:anywhere}.mt5-gap-list{display:grid;gap:7px;padding:12px;border:1px solid color-mix(in srgb,var(--amber) 35%,var(--border));border-radius:10px}.mt5-gap-list strong{font-size:12px}.mt5-gap-list span{font-size:11px;color:var(--muted)}.mt5-series-table-wrap{overflow-x:auto;border:1px solid var(--border);border-radius:10px}.mt5-series-table{width:100%;border-collapse:collapse;white-space:nowrap;font-size:11px}.mt5-series-table th,.mt5-series-table td{padding:10px;border-bottom:1px solid var(--border);text-align:left}.mt5-series-table th{color:var(--muted);font-size:9px;text-transform:uppercase;letter-spacing:.07em}.mt5-series-table td code{font-size:10px;color:var(--muted)}.mt5-query-footer{display:flex;align-items:center;justify-content:space-between;gap:12px;color:var(--muted);font-size:11px}
  @media(max-width:900px){.mt5-grid{grid-template-columns:repeat(2,minmax(0,1fr))}.mt5-account-grid,.mt5-symbol-strip{grid-template-columns:repeat(2,minmax(0,1fr))}}
  @media(max-width:600px){.mt5-query-controls{grid-template-columns:1fr 1fr}.mt5-query-controls label:first-child{grid-column:1/-1}.mt5-quality-grid{grid-template-columns:repeat(2,minmax(0,1fr))}.mt5-query-footer{align-items:flex-start;flex-direction:column}.mt5-steps{grid-template-columns:1fr}.mt5-grid{grid-template-columns:1fr 1fr}.mt5-setup,.mt5-account,.mt5-positions{padding:14px}.mt5-account-grid{grid-template-columns:repeat(2,minmax(0,1fr))}}

  .mt5-risk-sizing { margin-top: 16px; padding: 16px; border: 1px solid var(--border, #30343b); border-radius: 12px; background: var(--surface-2, rgba(127,127,127,.04)); display: grid; gap: 12px; }
  .mt5-instrument-registry { margin-top: 16px; padding: 16px; border: 1px solid var(--border, #30343b); border-radius: 12px; background: var(--surface-2, rgba(127,127,127,.04)); }
  .mt5-instrument-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 14px; }
  .mt5-instrument-heading > div { display: grid; gap: 4px; }
  .mt5-instrument-heading span:not(.source-tag) { color: var(--muted, #9298a3); font-size: 12px; }
  .mt5-session-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 10px; margin-top: 12px; }
  .mt5-session-grid > div { display: flex; flex-direction: column; gap: 6px; padding: 10px; border: 1px solid var(--border-subtle, rgba(127,127,127,.2)); border-radius: 8px; min-width: 0; }
  .mt5-session-grid small { color: var(--text-muted); font-size: 11px; }
  .mt5-session-grid strong { font-size: 12px; font-weight: 600; }
</style>
