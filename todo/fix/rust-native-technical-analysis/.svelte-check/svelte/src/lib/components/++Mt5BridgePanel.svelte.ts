///<reference types="svelte" />
;
import { onMount } from 'svelte';
import { Activity, AlertTriangle, CheckCircle2, CircleSlash, Copy, Database, EyeOff, LockKeyhole, Network, RefreshCw, ShieldCheck, Wifi, WifiOff } from 'lucide-svelte';
import { getMt5BridgeSetup, getMt5BridgeStatus, getMt5InstrumentSpecification, getMt5SessionCalendar, calculateMt5RiskSizing, queryMt5MarketDataSeries } from '../adapters/tauri/bridge';
import type { Mt5BridgeSetup, Mt5BridgeStatus, Mt5InstrumentSpecificationRecord, Mt5SessionCalendarRecord, Mt5RiskSizingRequest, Mt5RiskSizingResult, Mt5SeriesQuery, Mt5SeriesQueryResponse, Mt5StoredBar } from '../types';
function $$render() {

  
  
  
  

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
  $: riskSizingResultStale = __sveltets_2_invalidate(() => !!riskSizingResult && (
    !status?.connected || status.stale || status.quoteStale
    || status.lastSequence !== riskSizingResult.snapshotSequence
  ));
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

  let  snapshot = __sveltets_2_invalidate(() => status?.snapshot ?? null);
  let  account = __sveltets_2_invalidate(() => snapshot?.account ?? null);
  let  symbol = __sveltets_2_invalidate(() => snapshot?.symbol ?? null);
  let  positions = __sveltets_2_invalidate(() => snapshot?.positions ?? []);
  let  pendingOrders = __sveltets_2_invalidate(() => snapshot?.pendingOrders ?? []);
  let  accountRisk = __sveltets_2_invalidate(() => status?.accountRisk ?? null);
  let  marketStorage = __sveltets_2_invalidate(() => status?.marketDataStorage ?? null);
  let  connectionLabel = __sveltets_2_invalidate(() => status?.connected ? 'CONNECTED' : status?.stale ? 'STALE' : status?.listening ? 'WAITING FOR MT5' : 'BRIDGE UNAVAILABLE');

  function pendingOrderTypeLabel(orderType: number) {
    return ({ 2: 'BUY LIMIT', 3: 'SELL LIMIT', 4: 'BUY STOP', 5: 'SELL STOP', 6: 'BUY STOP LIMIT', 7: 'SELL STOP LIMIT' } as Record<number, string>)[orderType] ?? 'UNKNOWN';
  }

  function stopRiskLabel(statusValue: string) {
    return statusValue === 'CALCULATED' ? 'Calculated' : statusValue === 'UNPROTECTED' ? 'No stop loss' : 'Unresolved';
  }
;
async () => {

 { svelteHTML.createElement("section", { "class":`mt5-page`,});
   { svelteHTML.createElement("div", { "class":`page-heading`,});
     { svelteHTML.createElement("div", {});
       { svelteHTML.createElement("div", { "class":`eyebrow`,});     }
       { svelteHTML.createElement("h1", {});  }
       { svelteHTML.createElement("p", {});                                     }
     }
     { svelteHTML.createElement("span", {    "class":`connection-pill`,});status?.connected;status?.stale || !status?.listening;
      if(status?.connected){  { const $$_ifiW3C = __sveltets_2_ensureComponent(Wifi); new $$_ifiW3C({ target: __sveltets_2_any(), props: { "size":14,}});}} else if (status?.stale){  { const $$_elgnairTtrelA3C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA3C({ target: __sveltets_2_any(), props: { "size":14,}});}}else{  { const $$_ffOifiW3C = __sveltets_2_ensureComponent(WifiOff); new $$_ffOifiW3C({ target: __sveltets_2_any(), props: { "size":14,}});}}
      connectionLabel;
     }
   }

  if(error){
     { svelteHTML.createElement("div", { "class":`mt5-alert danger`,});  { const $$_elgnairTtrelA2C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA2C({ target: __sveltets_2_any(), props: { "size":17,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});  } { svelteHTML.createElement("p", {});error; } } }
  }
  if(notice){
     { svelteHTML.createElement("div", { "class":`mt5-alert info`,});  { const $$_kcehCdleihS2C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS2C({ target: __sveltets_2_any(), props: { "size":17,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("p", {});notice; } } }
  }

   { svelteHTML.createElement("section", { "class":`mt5-grid`,});
     { svelteHTML.createElement("article", { "class":`panel mt5-stat`,});
       { svelteHTML.createElement("div", { "class":`mt5-stat-label`,});  { const $$_krowteN4C = __sveltets_2_ensureComponent(Network); new $$_krowteN4C({ target: __sveltets_2_any(), props: { "size":15,}});}  }
       { svelteHTML.createElement("strong", {});status?.listening ? 'Listening' : 'Unavailable'; }
       { svelteHTML.createElement("span", {});status?.address ?? '127.0.0.1:47650'; }
     }
     { svelteHTML.createElement("article", { "class":`panel mt5-stat`,});
       { svelteHTML.createElement("div", { "class":`mt5-stat-label`,});  { const $$_ytivitcA4C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA4C({ target: __sveltets_2_any(), props: { "size":15,}});}  }
       { svelteHTML.createElement("strong", {});age(status?.heartbeatAgeMs ?? null); }
       { svelteHTML.createElement("span", {});timestamp(status?.lastHeartbeatAtMs ?? null); }
     }
     { svelteHTML.createElement("article", { "class":`panel mt5-stat`,});
       { svelteHTML.createElement("div", { "class":`mt5-stat-label`,});  { const $$_esabataD4C = __sveltets_2_ensureComponent(Database); new $$_esabataD4C({ target: __sveltets_2_any(), props: { "size":15,}});}  }
       { svelteHTML.createElement("strong", {});status?.connected ? status.accountLogin : status?.accountLogin ? 'Stale snapshot' : 'Not connected'; }
       { svelteHTML.createElement("span", {});status?.connected ? status.accountServer : status?.accountServer ? `Last known: ${status.accountServer}` : 'No broker metadata'; }
     }
     { svelteHTML.createElement("article", { "class":`panel mt5-stat`,});
       { svelteHTML.createElement("div", { "class":`mt5-stat-label`,});  { const $$_ytivitcA4C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA4C({ target: __sveltets_2_any(), props: { "size":15,}});}   }
       { svelteHTML.createElement("strong", {});status?.connected && snapshot ? positions.length : '—'; }
       { svelteHTML.createElement("span", {});status?.connected ? `Accepted snapshot sequence ${status.lastSequence}` : 'No current position snapshot'; }
     }
     { svelteHTML.createElement("article", { "class":`panel mt5-stat`,});
       { svelteHTML.createElement("div", { "class":`mt5-stat-label`,});  { const $$_ytivitcA4C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA4C({ target: __sveltets_2_any(), props: { "size":15,}});}   }
       { svelteHTML.createElement("strong", {});status?.connected && snapshot ? pendingOrders.length : '—'; }
       { svelteHTML.createElement("span", {});status?.connected ? 'Active broker orders in the accepted snapshot' : 'No current order inventory'; }
     }
     { svelteHTML.createElement("article", { "class":`panel mt5-stat`,});
       { svelteHTML.createElement("div", { "class":`mt5-stat-label`,});  { const $$_esabataD4C = __sveltets_2_ensureComponent(Database); new $$_esabataD4C({ target: __sveltets_2_any(), props: { "size":15,}});}    }
       { svelteHTML.createElement("strong", {});marketStorage?.storageAvailable ? `${number(marketStorage.ticksTotal, 0)} ticks` : 'Storage unavailable'; }
       { svelteHTML.createElement("span", {});marketStorage ? `${number(marketStorage.barsTotal, 0)} closed bars · ${marketStorage.symbolsTotal} symbols` : 'No durable ingestion store'; }
     }
   }

   { svelteHTML.createElement("section", { "class":`panel mt5-data-workbench`,});
     { svelteHTML.createElement("div", { "class":`panel-heading`,});
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});                      } }
       { svelteHTML.createElement("span", { "class":`source-tag`,});  { const $$_esabataD4C = __sveltets_2_ensureComponent(Database); new $$_esabataD4C({ target: __sveltets_2_any(), props: { "size":13,}});}   }
     }
     { svelteHTML.createElement("div", { "class":`mt5-query-controls`,});
       { svelteHTML.createElement("label", {});  { svelteHTML.createElement("input", {          "bind:value":seriesSymbol,"oninput":() => riskSizingResult = null,"maxlength":64,"placeholder":`e.g. XAUUSD.a`,"autocomplete":`off`,});/*Ωignore_startΩ*/() => seriesSymbol = __sveltets_2_any(null);/*Ωignore_endΩ*/} }
       { svelteHTML.createElement("label", {});  { svelteHTML.createElement("select", { "bind:value":seriesTimeframe,});/*Ωignore_startΩ*/() => seriesTimeframe = __sveltets_2_any(null);/*Ωignore_endΩ*/ { svelteHTML.createElement("option", { "value":`M1`,});  } { svelteHTML.createElement("option", { "value":`M5`,});  } { svelteHTML.createElement("option", { "value":`M15`,});  } { svelteHTML.createElement("option", { "value":`M30`,});  } { svelteHTML.createElement("option", { "value":`H1`,});  } { svelteHTML.createElement("option", { "value":`H4`,});  } { svelteHTML.createElement("option", { "value":`D1`,});  } } }
       { svelteHTML.createElement("label", {});  { svelteHTML.createElement("select", { "bind:value":seriesRangeDays,});/*Ωignore_startΩ*/() => seriesRangeDays = __sveltets_2_any(null);/*Ωignore_endΩ*/ { svelteHTML.createElement("option", { "value":`1`,});   } { svelteHTML.createElement("option", { "value":`7`,});   } { svelteHTML.createElement("option", { "value":`30`,});   } { svelteHTML.createElement("option", { "value":`90`,});   } { svelteHTML.createElement("option", { "value":`180`,});   } { svelteHTML.createElement("option", { "value":`365`,});   } } }
       { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":() => queryStoredSeries(null),"disabled":seriesLoading || !status?.accountLogin || !status?.accountServer,});  { const $$_wChserfeR4C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR4C({ target: __sveltets_2_any(), props: { "size":14,}});}seriesLoading ? 'Analyzing…' : 'Inspect stored series'; }
       { svelteHTML.createElement("button", {     "class":`button-secondary`,"onclick":inspectInstrumentSpecification,"disabled":instrumentSpecificationLoading || !status?.accountLogin || !status?.accountServer,});  { const $$_esabataD4C = __sveltets_2_ensureComponent(Database); new $$_esabataD4C({ target: __sveltets_2_any(), props: { "size":14,}});}instrumentSpecificationLoading ? 'Loading contract…' : 'Inspect broker contract'; }
       { svelteHTML.createElement("button", {     "class":`button-secondary`,"onclick":inspectSessionCalendar,"disabled":sessionCalendarLoading || !status?.accountLogin || !status?.accountServer,});  { const $$_ytivitcA4C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA4C({ target: __sveltets_2_any(), props: { "size":14,}});}sessionCalendarLoading ? 'Loading sessions…' : 'Inspect broker sessions'; }
     }
     { svelteHTML.createElement("div", { "class":`mt5-risk-sizing`,});
       { svelteHTML.createElement("div", { "class":`mt5-instrument-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("span", {});             } } { svelteHTML.createElement("span", { "class":`source-tag`,});  } }
       { svelteHTML.createElement("div", { "class":`mt5-query-controls`,});
         { svelteHTML.createElement("label", {});  { svelteHTML.createElement("select", {   "bind:value":riskSide,"onchange":() => riskSizingResult = null,});/*Ωignore_startΩ*/() => riskSide = __sveltets_2_any(null);/*Ωignore_endΩ*/ { svelteHTML.createElement("option", { "value":`BUY`,});  } { svelteHTML.createElement("option", { "value":`SELL`,});  } } }
         { svelteHTML.createElement("label", {});  { svelteHTML.createElement("input", {          "bind:value":riskStopLoss,"oninput":() => riskSizingResult = null,"inputmode":`decimal`,"placeholder":`Broker price, e.g. 2300.00`,"autocomplete":`off`,});/*Ωignore_startΩ*/() => riskStopLoss = __sveltets_2_any(null);/*Ωignore_endΩ*/} }
         { svelteHTML.createElement("label", {});   { svelteHTML.createElement("input", {          "bind:value":riskPercent,"oninput":() => riskSizingResult = null,"inputmode":`decimal`,"placeholder":`1.0`,"autocomplete":`off`,});/*Ωignore_startΩ*/() => riskPercent = __sveltets_2_any(null);/*Ωignore_endΩ*/} }
         { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":calculateRiskSizing,"disabled":riskSizingLoading || !status?.connected || !status?.accountLogin || !status?.accountServer || !seriesSymbol.trim(),});riskSizingLoading ? 'Calculating…' : 'Calculate risk-sized volume'; }
       }
      if(riskSizingError){ { svelteHTML.createElement("div", { "class":`mt5-alert danger`,});  { const $$_elgnairTtrelA4C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA4C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("p", {});riskSizingError; } } }}
      if(riskSizingResult){
         { svelteHTML.createElement("div", { "class":`mt5-risk-result`,});
           { svelteHTML.createElement("div", { "class":`mt5-instrument-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});riskSizingResultStale ? 'Historical sizing result' : riskSizingResult.decision === 'BLOCKED' ? 'Sizing rejected' : 'Risk-sized volume calculated'; } { svelteHTML.createElement("span", {});riskSizingResult.symbol;  riskSizingResult.side;  riskSizingResult.accountCurrency; } } { svelteHTML.createElement("span", {  "class":`source-tag`,});riskSizingResult.decision === 'BLOCKED' || riskSizingResultStale;riskSizingResultStale ? 'STALE — RE-CALCULATE' : riskSizingResult.decision; } }
          if(riskSizingResultStale){ { svelteHTML.createElement("div", { "class":`mt5-alert danger`,});  { const $$_elgnairTtrelA5C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA5C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});  } { svelteHTML.createElement("p", {});                              } } }}
           { svelteHTML.createElement("div", { "class":`mt5-quality-grid`,});
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});riskSizingResultStale ? 'Historical calculated volume' : 'Approved volume'; } { svelteHTML.createElement("strong", {});riskSizingResultStale ? 'STALE — RE-CALCULATE' : `${number(riskSizingResult.normalizedVolume, 4)} lots`; } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(riskSizingResult.proposedVolume, 4);  } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});money(riskSizingResult.riskBudget, riskSizingResult.accountCurrency); } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});money(riskSizingResult.effectiveRiskBudget, riskSizingResult.accountCurrency); } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});money(riskSizingResult.accountAvailableRiskBudget, riskSizingResult.accountCurrency); } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});money(riskSizingResult.estimatedLoss, riskSizingResult.accountCurrency); } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {});money(riskSizingResult.accountStopRisk, riskSizingResult.accountCurrency); number(riskSizingResult.accountStopRiskPercent, 3);  } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {});money(riskSizingResult.projectedAccountStopRisk, riskSizingResult.accountCurrency); } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});number(riskSizingResult.existingSymbolVolume, 4);  } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});number(riskSizingResult.pendingSymbolVolume, 4);  } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {});number(riskSizingResult.totalDirectionalSymbolVolume, 4);  number(riskSizingResult.volumeLimit, 4);  } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});riskSizingResult.candidateMargin === null ? '—' : money(riskSizingResult.candidateMargin, riskSizingResult.accountCurrency); } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});riskSizingResult.projectedFreeMargin === null ? '—' : money(riskSizingResult.projectedFreeMargin, riskSizingResult.accountCurrency); } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});riskSizingResult.projectedMarginLevelPct === null ? '—' : `${number(riskSizingResult.projectedMarginLevelPct)}%`; } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});     } { svelteHTML.createElement("strong", {});number(riskSizingResult.quoteAgeMs, 0);   riskSizingResult.marginPreflightAtMs === null ? 'not completed' : timestamp(riskSizingResult.marginPreflightAtMs); } }
           }
          if(riskSizingResult.reasons.length > 0){ { svelteHTML.createElement("div", { "class":`mt5-alert danger`,});  { const $$_elgnairTtrelA5C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA5C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});  }  for(let reason of __sveltets_2_ensureArray(riskSizingResult.reasons)){ { svelteHTML.createElement("p", {});reason; }} } }}
           { svelteHTML.createElement("div", { "class":`mt5-manifest`,}); { svelteHTML.createElement("small", {});   riskSizingResult.snapshotSequence;    riskSizingResult.specificationRevision;  riskSizingResult.contractCompleteness;   } { svelteHTML.createElement("code", {});riskSizingResult.specificationFingerprintSha256; } }
           { svelteHTML.createElement("p", { "class":`mt5-quality-note`,});riskSizingResult.marginEstimation; riskSizingResult.commissionEstimation;                         }
           { svelteHTML.createElement("div", { "class":`mt5-manifest`,}); { svelteHTML.createElement("small", {});  }  for(let limitation of __sveltets_2_ensureArray(riskSizingResult.limitations)){ { svelteHTML.createElement("span", {});limitation; }} }
         }
      }
     }
    if(instrumentSpecificationError){ { svelteHTML.createElement("div", { "class":`mt5-alert danger`,});  { const $$_elgnairTtrelA3C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA3C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("p", {});instrumentSpecificationError; } } }}
    if(instrumentSpecification){
       { svelteHTML.createElement("div", { "class":`mt5-instrument-registry`,});
         { svelteHTML.createElement("div", { "class":`mt5-instrument-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});instrumentSpecification.brokerServer;  instrumentSpecification.accountLogin;  instrumentSpecification.specification.symbol; } } { svelteHTML.createElement("span", { "class":`source-tag`,}); instrumentSpecification.revision; } }
         { svelteHTML.createElement("div", { "class":`mt5-quality-grid`,});
           { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(instrumentSpecification.specification.contractSize, 6); } }
           { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});      } { svelteHTML.createElement("strong", {});number(instrumentSpecification.specification.tickValueProfit, 6);  number(instrumentSpecification.specification.tickValueLoss, 6); } }
           { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});      } { svelteHTML.createElement("strong", {});number(instrumentSpecification.specification.volumeMin, 4);  number(instrumentSpecification.specification.volumeStep, 4);  number(instrumentSpecification.specification.volumeMax, 4); } }
           { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});instrumentSpecification.specification.volumeLimit > 0 ? number(instrumentSpecification.specification.volumeLimit, 4) : 'No broker limit reported'; } }
           { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});      } { svelteHTML.createElement("strong", {});[instrumentSpecification.specification.currencyBase, instrumentSpecification.specification.currencyProfit, instrumentSpecification.specification.currencyMargin].filter(Boolean).join(' / ') || 'Not reported'; } }
           { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {});instrumentSpecification.specification.stopsLevelPoints;  instrumentSpecification.specification.freezeLevelPoints;  } }
           { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {});instrumentSpecification.specification.calculationMode;  instrumentSpecification.specification.executionMode; } }
           { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {});number(instrumentSpecification.specification.swapLong, 6);  number(instrumentSpecification.specification.swapShort, 6); } }
         }
         { svelteHTML.createElement("div", { "class":`mt5-manifest`,}); { svelteHTML.createElement("small", {});   instrumentSpecification.completeness; } { svelteHTML.createElement("code", {});instrumentSpecification.fingerprintSha256; } }
         { svelteHTML.createElement("p", { "class":`mt5-quality-note`,});                                }
       }
    }
    if(sessionCalendarError){ { svelteHTML.createElement("div", { "class":`mt5-alert danger`,});  { const $$_elgnairTtrelA3C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA3C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("p", {});sessionCalendarError; } } }}
    if(sessionCalendar){
       { svelteHTML.createElement("div", { "class":`mt5-instrument-registry`,});
         { svelteHTML.createElement("div", { "class":`mt5-instrument-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("span", {});sessionCalendar.brokerServer;  sessionCalendar.accountLogin;  sessionCalendar.schedule.symbol;   timestamp(sessionCalendar.schedule.capturedAtMs); } } { svelteHTML.createElement("span", { "class":`source-tag`,}); sessionCalendar.revision; } }
         { svelteHTML.createElement("div", { "class":`mt5-session-grid`,});
             for(let day of __sveltets_2_ensureArray(sessionCalendar.schedule.days)){day.dayOfWeek;
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});weekdayNames[day.dayOfWeek] ?? `Weekday ${day.dayOfWeek}`; }
              if(day.sessions.length === 0){ { svelteHTML.createElement("strong", {});   }}else{
                   for(let session of __sveltets_2_ensureArray(day.sessions)){session.index; { svelteHTML.createElement("strong", {});sessionClock(session.fromSeconds);  sessionClock(session.toSeconds);session.toSeconds < session.fromSeconds ? ' · overnight' : ''; }}
              }
             }
          }
         }
         { svelteHTML.createElement("div", { "class":`mt5-manifest`,}); { svelteHTML.createElement("small", {});   sessionCalendar.timeBasis; } { svelteHTML.createElement("code", {});sessionCalendar.fingerprintSha256; } }
         { svelteHTML.createElement("p", { "class":`mt5-quality-note`,});                                     }
       }
    }
    if(seriesError){ { svelteHTML.createElement("div", { "class":`mt5-alert danger`,});  { const $$_elgnairTtrelA3C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA3C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("p", {});seriesError; } } }}
    if(seriesResult){
       { svelteHTML.createElement("div", { "class":`mt5-query-result-identity`,}); { svelteHTML.createElement("strong", {});seriesResult.brokerServer;   seriesResult.accountLogin;  seriesResult.symbol;  seriesResult.timeframe; } { svelteHTML.createElement("span", {});timestamp(seriesResult.fromMs);  timestamp(seriesResult.toMs);           } }
       { svelteHTML.createElement("div", { "class":`mt5-quality-grid`,});
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(seriesResult.quality.barsTotal, 0); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(seriesResult.quality.revisionsTotal, 0); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", { });seriesResult.quality.intervalAnomalyCount > 0;number(seriesResult.quality.intervalAnomalyCount, 0); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});seriesResult.quality.largestIntervalMs === null ? '—' : `${(seriesResult.quality.largestIntervalMs / 60000).toFixed(1)} min`; } }
       }
       { svelteHTML.createElement("p", { "class":`mt5-quality-note`,});seriesResult.quality.continuityInterpretation; }
       { svelteHTML.createElement("div", { "class":`mt5-manifest`,}); { svelteHTML.createElement("small", {});      } { svelteHTML.createElement("code", {});seriesResult.datasetFingerprintSha256; } }
      if(seriesResult.quality.gapCandidates.length > 0){
         { svelteHTML.createElement("div", { "class":`mt5-gap-list`,}); { svelteHTML.createElement("strong", {});    }  for(let gap of __sveltets_2_ensureArray((seriesResult.quality.gapCandidates.slice(0, 5)))){ { svelteHTML.createElement("span", {});timestamp(gap.previousBarTimeMs);  timestamp(gap.nextBarTimeMs);  (gap.elapsedMs / 60000).toFixed(1);  }} }
      }else{
         { svelteHTML.createElement("p", { "class":`mt5-quality-note`,});                     }
      }
       { svelteHTML.createElement("div", { "class":`mt5-series-table-wrap`,});
         { svelteHTML.createElement("table", { "class":`mt5-series-table`,}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});
            for(let bar of __sveltets_2_ensureArray(seriesBars.slice(-8))){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {});timestamp(bar.timeMs); } { svelteHTML.createElement("td", {});number(bar.open, 6); } { svelteHTML.createElement("td", {});number(bar.high, 6); } { svelteHTML.createElement("td", {});number(bar.low, 6); } { svelteHTML.createElement("td", {});number(bar.close, 6); } { svelteHTML.createElement("td", {});number(bar.tickVolume, 0); } { svelteHTML.createElement("td", {});bar.revision; } { svelteHTML.createElement("td", {}); { svelteHTML.createElement("code", {});bar.payloadSha256.slice(0, 16);  } } }}
         } }
        if(seriesBars.length === 0){ { svelteHTML.createElement("div", { "class":`mt5-empty`,}); { svelteHTML.createElement("strong", {});      } { svelteHTML.createElement("span", {});          } }}
       }
       { svelteHTML.createElement("div", { "class":`mt5-query-footer`,}); { svelteHTML.createElement("span", {}); number(seriesBars.length, 0);  number(seriesResult.quality.barsTotal, 0);         }if(seriesResult.hasMore && seriesResult.nextAfterMs !== null){ { svelteHTML.createElement("button", {     "class":`button-secondary compact`,"onclick":() => queryStoredSeries(seriesResult?.nextAfterMs ?? null),"disabled":seriesLoading,});   }} }
    }
    if(!status?.accountLogin || !status?.accountServer){ { svelteHTML.createElement("p", { "class":`mt5-quality-note`,});                 }}
   }

   { svelteHTML.createElement("section", { "class":`panel mt5-setup`,});
     { svelteHTML.createElement("div", { "class":`panel-heading`,});
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});     } { svelteHTML.createElement("p", {});         } }
       { svelteHTML.createElement("button", {     "class":`button-secondary compact`,"onclick":refreshStatus,"disabled":loading,});  { const $$_wChserfeR4C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR4C({ target: __sveltets_2_any(), props: { "size":13,}});}   }
     }
     { svelteHTML.createElement("div", { "class":`mt5-steps`,});
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("p", {});               } } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("p", {});  { svelteHTML.createElement("code", {});  }     { svelteHTML.createElement("code", {});  }       } } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});     } { svelteHTML.createElement("p", {});     { svelteHTML.createElement("code", {});  }    { svelteHTML.createElement("code", {});  }          } } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("p", {});      { svelteHTML.createElement("code", {});  }               } } }
     }
     { svelteHTML.createElement("div", { "class":`mt5-actions`,});
      if(!revealToken){
         { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":revealPairing,"disabled":setupLoading || !status?.listening,});if(setupLoading){  { const $$_wChserfeR4C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR4C({ target: __sveltets_2_any(), props: { "size":14,}});}}else{  { const $$_elohyeKkcoL4C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL4C({ target: __sveltets_2_any(), props: { "size":14,}});}}     }
      }else{
         { svelteHTML.createElement("button", {   "class":`button-secondary`,"onclick":closeToken,});  { const $$_ffOeyE4C = __sveltets_2_ensureComponent(EyeOff); new $$_ffOeyE4C({ target: __sveltets_2_any(), props: { "size":14,}});}   }
      }
      if(status?.lastError){ { svelteHTML.createElement("span", { "class":`mt5-inline-warning`,});  { const $$_elgnairTtrelA4C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA4C({ target: __sveltets_2_any(), props: { "size":13,}});}           }}
     }
    if(revealToken && setup){
       { svelteHTML.createElement("div", { "class":`mt5-token-box`,});
         { svelteHTML.createElement("div", { "class":`mt5-token-heading`,}); { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});     } }
         { svelteHTML.createElement("code", {});setup.pairingToken; }
         { svelteHTML.createElement("button", {   "class":`button-secondary compact`,"onclick":copyToken,});  { const $$_ypoC4C = __sveltets_2_ensureComponent(Copy); new $$_ypoC4C({ target: __sveltets_2_any(), props: { "size":13,}});}   }
         { svelteHTML.createElement("p", {});setup.transportSecurity; }
       }
    }
   }

  if(status?.connected && snapshot && account && symbol){
     { svelteHTML.createElement("section", { "class":`panel mt5-account`,});
       { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});                } } { svelteHTML.createElement("span", { "class":`source-tag`,});  { const $$_2elcriCkcehC4C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC4C({ target: __sveltets_2_any(), props: { "size":13,}});}   } }
       { svelteHTML.createElement("div", { "class":`mt5-account-grid`,});
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});money(account.balance, account.currency); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});money(account.equity, account.currency); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {  });account.profit >= 0;account.profit < 0;money(account.profit, account.currency); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});money(account.margin, account.currency); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});money(account.freeMargin, account.currency); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});account.marginLevelPct > 0 ? `${number(account.marginLevelPct)}%` : '—'; } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {});money(account.marginInitial, account.currency);  money(account.marginMaintenance, account.currency); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});number(account.marginCallLevel);account.marginStopoutMode === 0 ? '%' : ` ${account.currency}`; } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});number(account.stopOutLevel);account.marginStopoutMode === 0 ? '%' : ` ${account.currency}`; } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});account.marginStopoutMode === 0 ? 'Percentage' : 'Money';  account.marginMode === 0 ? 'Retail netting' : account.marginMode === 1 ? 'Exchange' : 'Retail hedging'; } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});    } { svelteHTML.createElement("strong", {}); number(account.leverage, 0);  number(account.accountLimitOrders, 0); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});account.tradeAllowed ? 'All permission flags enabled' : 'One or more permission flags disabled'; } }
       }
      if(accountRisk){
         { svelteHTML.createElement("div", { "class":`mt5-manifest`,});
           { svelteHTML.createElement("small", {});     accountRisk.accountCurrency; }
           { svelteHTML.createElement("div", { "class":`mt5-quality-grid`,});
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});money(accountRisk.currentStopRisk, accountRisk.accountCurrency); number(accountRisk.currentStopRiskPercent, 3);  } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});accountRisk.protectedPositionCount;   accountRisk.protectedPendingOrderCount;  } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});accountRisk.unprotectedPositionCount;   accountRisk.unprotectedPendingOrderCount;  } }
             { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});accountRisk.unresolvedPositionRiskCount;   accountRisk.unresolvedPendingOrderRiskCount;  } }
           }
           { svelteHTML.createElement("p", { "class":`mt5-quality-note`,});                   }
         }
      }else{
         { svelteHTML.createElement("div", { "class":`mt5-alert danger`,});  { const $$_elgnairTtrelA3C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA3C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("p", {});                } } }
      }
       { svelteHTML.createElement("div", { "class":`mt5-symbol-strip`,});
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});symbol.name; } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});   } { svelteHTML.createElement("strong", {});number(symbol.bid, symbol.digits);  number(symbol.ask, symbol.digits); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(symbol.ask - symbol.bid, symbol.digits); symbol.digits;  } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {});number(symbol.volumeMin, 4);  number(symbol.volumeMax, 4);   number(symbol.volumeStep, 4); } }
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("small", {});  } { svelteHTML.createElement("strong", {  });!status?.quoteStale;status?.quoteStale;status?.quoteStale ? 'STALE QUOTE' : 'Fresh observation'; } { svelteHTML.createElement("small", {});timestamp(symbol.quoteTimeMs); } }
       }
       { svelteHTML.createElement("div", { "class":`mt5-permissions`,});
         { svelteHTML.createElement("span", { });snapshot.terminal.connected;  { const $$_2elcriCkcehC4C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC4C({ target: __sveltets_2_any(), props: { "size":13,}});}   }
         { svelteHTML.createElement("span", { });account.accountTradeAllowed;  { const $$_2elcriCkcehC4C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC4C({ target: __sveltets_2_any(), props: { "size":13,}});}   }
         { svelteHTML.createElement("span", { });account.terminalTradeAllowed;  { const $$_2elcriCkcehC4C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC4C({ target: __sveltets_2_any(), props: { "size":13,}});}   }
         { svelteHTML.createElement("span", { });snapshot.terminal.algoTradingAllowed;  { const $$_2elcriCkcehC4C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC4C({ target: __sveltets_2_any(), props: { "size":13,}});}    }
       }
     }

     { svelteHTML.createElement("section", { "class":`panel mt5-positions`,});
       { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});  } { svelteHTML.createElement("p", {});              } } { svelteHTML.createElement("span", { "class":`mode-pill`,});positions.length;  } }
      if(positions.length){
         { svelteHTML.createElement("div", { "class":`table-scroll`,}); { svelteHTML.createElement("table", {}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});   } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});
             for(let position of __sveltets_2_ensureArray(positions)){position.ticket;
             { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {}); { svelteHTML.createElement("code", {});position.ticket; } } { svelteHTML.createElement("td", {});position.symbol; } { svelteHTML.createElement("td", {});position.positionType === 0 ? 'BUY' : 'SELL'; } { svelteHTML.createElement("td", {});number(position.volume, 4); } { svelteHTML.createElement("td", {});number(position.priceOpen, 8); } { svelteHTML.createElement("td", {});number(position.priceCurrent, 8); } { svelteHTML.createElement("td", {  });position.profit >= 0;position.profit < 0;money(position.profit + position.swap, account.currency); } { svelteHTML.createElement("td", {});position.stopLoss > 0 ? number(position.stopLoss, 8) : '—';  position.takeProfit > 0 ? number(position.takeProfit, 8) : '—'; } { svelteHTML.createElement("td", {});stopRiskLabel(position.riskStatus);position.riskAtStopLoss === null ? '' : ` · ${money(position.riskAtStopLoss, account.currency)}`; } }
          }
         } } }
      }else{
         { svelteHTML.createElement("div", { "class":`mt5-empty`,});  { const $$_hsalSelcriC3C = __sveltets_2_ensureComponent(CircleSlash); new $$_hsalSelcriC3C({ target: __sveltets_2_any(), props: { "size":20,}});} { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});       } }
      }
     }

     { svelteHTML.createElement("section", { "class":`panel mt5-positions`,});
       { svelteHTML.createElement("div", { "class":`panel-heading`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});             } } { svelteHTML.createElement("span", { "class":`mode-pill`,});pendingOrders.length;  } }
      if(pendingOrders.length){
         { svelteHTML.createElement("div", { "class":`table-scroll`,}); { svelteHTML.createElement("table", {}); { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});   } { svelteHTML.createElement("th", {});   } { svelteHTML.createElement("th", {});   } { svelteHTML.createElement("th", {});  } } } { svelteHTML.createElement("tbody", {});
             for(let order of __sveltets_2_ensureArray(pendingOrders)){order.ticket;
             { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {}); { svelteHTML.createElement("code", {});order.ticket; } } { svelteHTML.createElement("td", {});order.symbol; } { svelteHTML.createElement("td", {});pendingOrderTypeLabel(order.orderType); } { svelteHTML.createElement("td", {});number(order.volumeCurrent, 4);  number(order.volumeInitial, 4); } { svelteHTML.createElement("td", {});number(order.priceOpen, 8);order.priceStopLimit > 0 ? ` / ${number(order.priceStopLimit, 8)}` : ''; } { svelteHTML.createElement("td", {});order.stopLoss > 0 ? number(order.stopLoss, 8) : '—';  order.takeProfit > 0 ? number(order.takeProfit, 8) : '—'; } { svelteHTML.createElement("td", {});stopRiskLabel(order.riskStatus);order.riskAtStopLoss === null ? '' : ` · ${money(order.riskAtStopLoss, account.currency)}`; } }
          }
         } } }
      }else{
         { svelteHTML.createElement("div", { "class":`mt5-empty`,});  { const $$_hsalSelcriC3C = __sveltets_2_ensureComponent(CircleSlash); new $$_hsalSelcriC3C({ target: __sveltets_2_any(), props: { "size":20,}});} { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("span", {});        } }
      }
     }
  }else{
     { svelteHTML.createElement("section", { "class":`panel mt5-empty-panel`,});
        { const $$_ffOifiW2C = __sveltets_2_ensureComponent(WifiOff); new $$_ffOifiW2C({ target: __sveltets_2_any(), props: { "size":24,}});}
       { svelteHTML.createElement("strong", {});status?.stale ? 'MT5 telemetry is stale' : 'Waiting for an authenticated terminal'; }
       { svelteHTML.createElement("span", {});status?.stale ? 'The last broker snapshot is retained as historical context only. It is not shown as current account state.' : 'No account, quote, or position values are fabricated. Attach the EA after the local listener is ready.'; }
     }
  }

   { svelteHTML.createElement("section", { "class":`mt5-alert security`,});  { const $$_kcehCdleihS2C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS2C({ target: __sveltets_2_any(), props: { "size":18,}});} { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});    } { svelteHTML.createElement("p", {});          { svelteHTML.createElement("code", {});  }                                      } } }
 }


};
return { props: {} as Record<string, never>, exports: {}, bindings: "", slots: {}, events: {} }}
const Mt5BridgePanel__SvelteComponent_ = __sveltets_2_isomorphic_component(__sveltets_2_with_any_event($$render()));
/*Ωignore_startΩ*/type Mt5BridgePanel__SvelteComponent_ = InstanceType<typeof Mt5BridgePanel__SvelteComponent_>;
/*Ωignore_endΩ*/export default Mt5BridgePanel__SvelteComponent_;