<script lang="ts">
  import { onMount } from 'svelte';
  import { AlertTriangle, CheckCircle2, Clock3, Database, RefreshCw, ShieldCheck } from 'lucide-svelte';
  import { getMarketDataProviderCatalog, queryUnifiedMarketData } from '../adapters/tauri/bridge';
  import type { MarketDataProviderCatalogEntry, MarketDataProviderState, MarketDataPurpose, UnifiedMarketDataResult } from '../types';

  let providers: MarketDataProviderCatalogEntry[] = [];
  let selectedProviderId = 'binance-public';
  let symbol = 'BTCUSDT';
  let timeframe = 'M5';
  let purpose: MarketDataPurpose = 'research';
  let allowStale = false;
  let pageLimit = 500;
  let providersBusy = false;
  let queryBusy = false;
  let errorMessage = '';
  let result: UnifiedMarketDataResult | null = null;

  const providerStates: Record<MarketDataProviderState, string> = {
    connected: 'CONNECTED',
    cached: 'CACHED',
    disconnected: 'DISCONNECTED',
    notLoaded: 'NOT LOADED',
    degraded: 'DEGRADED'
  };

  onMount(() => {
    void loadProviders();
  });

  async function loadProviders(): Promise<void> {
    providersBusy = true;
    errorMessage = '';
    try {
      providers = await getMarketDataProviderCatalog();
      if (!providers.some((provider) => provider.id === selectedProviderId)) {
        selectedProviderId = providers[0]?.id ?? '';
      }
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      providersBusy = false;
    }
  }

  async function runQuery(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    queryBusy = true;
    errorMessage = '';
    result = null;
    const toMs = Date.now();
    try {
      result = await queryUnifiedMarketData({
        symbol,
        timeframe,
        fromMs: toMs - 7 * 24 * 60 * 60 * 1000,
        toMs,
        afterMs: null,
        limit: pageLimit,
        providerId: purpose === 'brokerContext' ? 'mt5-terminal' : selectedProviderId,
        purpose,
        maxDataAgeMs: purpose === 'brokerContext' ? 600_000 : 300_000,
        allowStale: purpose !== 'brokerContext' && allowStale
      });
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      queryBusy = false;
    }
  }

  function providerStateClass(state: MarketDataProviderState): string {
    if (state === 'connected') return 'good';
    if (state === 'cached') return 'cached';
    return 'bad';
  }

  function ageLabel(value: number | null): string {
    if (value == null || !Number.isFinite(value) || value < 0) return 'Unknown';
    if (value < 1_000) return `${Math.floor(value)} ms`;
    if (value < 60_000) return `${Math.floor(value / 1_000)} sec`;
    if (value < 3_600_000) return `${Math.floor(value / 60_000)} min`;
    return `${(value / 3_600_000).toFixed(1)} hr`;
  }

  function volumeLabel(basis: string): string {
    if (basis === 'baseAsset') return 'Base asset';
    if (basis === 'brokerRealVolume') return 'Broker real volume';
    return 'Broker tick count';
  }
</script>

<section class="registry-panel panel">
  <div class="panel-heading">
    <div>
      <h2>Native provider registry</h2>
      <p>Typed provider capabilities, explicit venue authority, and source-specific limitations.</p>
    </div>
    <button class="refresh-button" type="button" onclick={loadProviders} disabled={providersBusy} aria-label="Refresh provider registry">
      <RefreshCw size={14} class={providersBusy ? 'spin' : ''}/>
      Refresh
    </button>
  </div>
  {#if providers.length === 0 && providersBusy}
    <div class="registry-empty"><Database size={19}/> Loading native provider capabilities…</div>
  {:else if providers.length === 0}
    <div class="registry-empty"><AlertTriangle size={18}/> No native providers were returned by the Rust runtime.</div>
  {:else}
    <div class="provider-grid">
      {#each providers as provider (provider.id)}
        <article class="provider-entry">
          <div class="provider-entry-head">
            <div class="provider-icon"><Database size={17}/></div>
            <span class={`provider-state ${providerStateClass(provider.state)}`}>{providerStates[provider.state]}</span>
          </div>
          <h3>{provider.displayName}</h3>
          <p class="venue">{provider.venue} · {provider.authority === 'brokerTerminal' ? 'Broker-authoritative' : 'Public exchange'}</p>
          <div class="capabilities">
            {#each provider.capabilities as capability}<span>{capability}</span>{/each}
          </div>
          <ul>
            {#each provider.limitations as limitation}<li>{limitation}</li>{/each}
          </ul>
          <small>{provider.observedAtMs ? `Last observation ${new Date(provider.observedAtMs).toLocaleString()}` : 'No provider observation timestamp'}</small>
        </article>
      {/each}
    </div>
  {/if}
</section>

<section class="query-panel panel">
  <div class="panel-heading">
    <div>
      <h2>Unified market-data query</h2>
      <p>One canonical response model; provider identity, freshness, revisions, gaps, and dataset fingerprint stay attached to the data.</p>
    </div>
    <ShieldCheck size={18}/>
  </div>
  <form class="query-form" onsubmit={runQuery}>
    <label>Provider
      <select bind:value={selectedProviderId} disabled={purpose === 'brokerContext' || providers.length === 0}>
        {#each providers as provider}<option value={provider.id}>{provider.displayName}</option>{/each}
      </select>
    </label>
    <label>Symbol
      <input bind:value={symbol} maxlength="64" required autocomplete="off" spellcheck="false"/>
    </label>
    <label>Timeframe
      <select bind:value={timeframe}>
        <option value="M1">M1</option><option value="M5">M5</option><option value="M15">M15</option>
        <option value="M30">M30</option><option value="H1">H1</option><option value="H4">H4</option>
        <option value="D1">D1</option><option value="W1">W1</option>
      </select>
    </label>
    <label>Purpose
      <select bind:value={purpose}>
        <option value="research">Research</option>
        <option value="backtest">Backtest</option>
        <option value="brokerContext">Broker context · MT5 only</option>
      </select>
    </label>
    <label>Page size
      <input type="number" min="1" max="5000" step="1" bind:value={pageLimit}/>
    </label>
    <button class="query-button" type="submit" disabled={queryBusy || !selectedProviderId || !symbol.trim()}>
      <Database size={14}/>{queryBusy ? 'Querying…' : 'Query last 7 days'}
    </button>
  </form>
  {#if purpose === 'brokerContext'}
    <p class="policy-note"><ShieldCheck size={14}/> Broker context is pinned to the authenticated MT5 account, rejects stale data, and is not an execution approval.</p>
  {:else}
    <label class="stale-toggle"><input type="checkbox" bind:checked={allowStale}/> Allow stale observations for this query</label>
    <p class="policy-note"><Clock3 size={14}/> Stale observations are rejected unless explicitly allowed here. Backtests and research results retain their stale-data status and source timestamps.</p>
  {/if}
  {#if errorMessage}
    <div class="query-error" role="alert"><AlertTriangle size={15}/>{errorMessage}</div>
  {/if}
  {#if result}
    <div class="result-summary">
      <div><span>Provider</span><strong>{result.provenance.providerName}</strong></div>
      <div><span>Freshness</span><strong class:stale={result.quality.freshness !== 'fresh'}>{result.quality.freshness.toUpperCase()} · {ageLabel(result.quality.freshnessAgeMs)}</strong></div>
      <div><span>Bars</span><strong>{result.quality.candleCount}{result.quality.hasMore ? '+' : ''}</strong></div>
      <div><span>Revisions / gaps</span><strong>{result.quality.revisionCount} / {result.quality.intervalAnomalyCount}</strong></div>
    </div>
    <div class="request-policy"><span>Purpose: {result.request.purpose}</span><span>Stale override: {result.request.allowStale ? 'enabled' : 'disabled'}</span><span>Freshness limit: {ageLabel(result.request.maxDataAgeMs)}</span></div>
    <div class="fingerprint"><span>Dataset SHA-256</span><code>{result.provenance.datasetFingerprintSha256}</code></div>
    {#if result.quality.gapCandidates.length > 0}
      <div class="gap-warning"><AlertTriangle size={14}/>{result.quality.gapCandidates.length} interval gap candidate(s). {result.quality.continuityInterpretation}</div>
    {:else}
      <div class="continuity-note"><CheckCircle2 size={14}/>{result.quality.continuityInterpretation}</div>
    {/if}
    <div class="bar-table-wrap">
      <table>
        <thead><tr><th>Timestamp</th><th>Open</th><th>High</th><th>Low</th><th>Close</th><th>Volume</th><th>Volume basis</th></tr></thead>
        <tbody>
          {#each [...result.bars].slice(-12).reverse() as bar (bar.timeMs)}
            <tr><td>{new Date(bar.timeMs).toLocaleString()}</td><td>{bar.open}</td><td>{bar.high}</td><td>{bar.low}</td><td>{bar.close}</td><td>{bar.volume}</td><td>{volumeLabel(bar.volumeBasis)}</td></tr>
          {/each}
        </tbody>
      </table>
    </div>
    <p class="source-disclosure">Venue: {result.provenance.venue} · Latest market observation: {result.provenance.latestObservationAtMs ? new Date(result.provenance.latestObservationAtMs).toLocaleString() : 'Unavailable'} · Latest receipt: {result.provenance.latestReceivedAtMs ? new Date(result.provenance.latestReceivedAtMs).toLocaleString() : 'Unavailable'} · {result.provenance.sourceDetails} · Scope fingerprint: {result.provenance.scopeFingerprintSha256 ?? (result.provenance.authority === 'brokerTerminal' ? 'Authenticated scope verified internally' : 'Public dataset')}</p>
  {/if}
</section>

<style>
  .registry-panel,.query-panel{padding:20px;margin-bottom:16px}
  .panel-heading{display:flex;align-items:flex-start;justify-content:space-between;gap:14px;margin-bottom:16px}
  .panel-heading h2{margin:0;font-size:15px;font-weight:650}
  .panel-heading p{margin:5px 0 0;color:var(--muted,#8993a5);font-size:12px;line-height:1.5}
  .refresh-button,.query-button{display:inline-flex;align-items:center;justify-content:center;gap:7px;border:1px solid var(--border,#283244);border-radius:8px;padding:8px 11px;background:var(--surface-2,#171f2d);color:var(--text,#eef2f8);font-size:12px;cursor:pointer}
  .refresh-button:disabled,.query-button:disabled{opacity:.5;cursor:not-allowed}
  .provider-grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(250px,1fr));gap:12px}
  .provider-entry{border:1px solid var(--border,#283244);border-radius:10px;padding:14px;min-width:0}
  .provider-entry-head{display:flex;align-items:center;justify-content:space-between}
  .provider-icon{display:grid;place-items:center;width:30px;height:30px;border-radius:8px;background:var(--surface-2,#171f2d)}
  .provider-entry h3{margin:12px 0 3px;font-size:13px}
  .provider-entry .venue,.provider-entry small{color:var(--muted,#8993a5);font-size:11px}
  .provider-state{font-size:9px;font-weight:700;letter-spacing:.06em;padding:4px 6px;border-radius:5px;background:#303744;color:#c1c8d4}
  .provider-state.good{background:#153a2b;color:#8fe0b4}.provider-state.cached{background:#39331e;color:#e4cf88}.provider-state.bad{background:#3b2227;color:#e9a0a8}
  .capabilities{display:flex;flex-wrap:wrap;gap:5px;margin:10px 0}
  .capabilities span{font-size:10px;padding:4px 6px;border-radius:5px;background:var(--surface-2,#171f2d);color:var(--text,#eef2f8)}
  .provider-entry ul{padding-left:16px;margin:8px 0 12px;color:var(--muted,#8993a5);font-size:11px;line-height:1.5}
  .provider-entry li+li{margin-top:5px}
  .registry-empty{display:flex;align-items:center;gap:9px;padding:16px;color:var(--muted,#8993a5);font-size:12px}
  .query-form{display:grid;grid-template-columns:1.2fr 1fr .7fr 1.2fr .7fr auto;align-items:end;gap:9px}
  .query-form label{display:grid;gap:6px;color:var(--muted,#8993a5);font-size:11px}
  .query-form input,.query-form select{width:100%;min-width:0;border:1px solid var(--border,#283244);border-radius:7px;padding:9px;background:var(--surface-2,#171f2d);color:var(--text,#eef2f8);font:inherit;font-size:12px}
  .query-button{height:36px;white-space:nowrap;background:var(--accent,#225cbd);border-color:transparent}
  .policy-note,.source-disclosure{display:flex;align-items:flex-start;gap:7px;margin:12px 0 0;color:var(--muted,#8993a5);font-size:11px;line-height:1.5}
  .stale-toggle{display:flex;align-items:center;gap:8px;margin-top:12px;color:var(--text,#eef2f8);font-size:11px}
  .stale-toggle input{accent-color:var(--accent,#225cbd)}
  .request-policy{display:flex;flex-wrap:wrap;gap:8px;margin-top:12px}
  .request-policy span{padding:5px 7px;border:1px solid var(--border,#283244);border-radius:6px;color:var(--muted,#8993a5);font-size:10px}
  .query-error,.gap-warning{display:flex;align-items:flex-start;gap:8px;padding:10px;margin-top:12px;border-radius:8px;background:#3b2227;color:#e9a0a8;font-size:12px;line-height:1.5}
  .result-summary{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:9px;margin-top:18px}
  .result-summary>div{display:grid;gap:5px;padding:11px;border:1px solid var(--border,#283244);border-radius:8px;min-width:0}
  .result-summary span,.fingerprint span{font-size:10px;color:var(--muted,#8993a5)}
  .result-summary strong{font-size:12px;overflow-wrap:anywhere}
  .result-summary strong.stale{color:#e4cf88}
  .fingerprint{display:grid;gap:5px;margin-top:12px;padding:10px;border-radius:8px;background:var(--surface-2,#171f2d)}
  .fingerprint code{font-size:10px;overflow-wrap:anywhere;color:var(--text,#eef2f8)}
  .continuity-note{display:flex;gap:7px;margin-top:11px;color:#8fe0b4;font-size:11px;line-height:1.5}
  .bar-table-wrap{overflow-x:auto;margin-top:12px}
  .bar-table-wrap table{width:100%;border-collapse:collapse;white-space:nowrap;font-size:11px}
  .bar-table-wrap th,.bar-table-wrap td{padding:9px 10px;border-bottom:1px solid var(--border,#283244);text-align:right}
  .bar-table-wrap th:first-child,.bar-table-wrap td:first-child,.bar-table-wrap th:last-child,.bar-table-wrap td:last-child{text-align:left}
  @media(max-width:900px){.query-form{grid-template-columns:repeat(2,minmax(0,1fr))}.query-button{width:100%}}
  @media(max-width:560px){.result-summary{grid-template-columns:repeat(2,minmax(0,1fr))}.registry-panel,.query-panel{padding:14px}}
</style>
