<script lang="ts">
  import { onMount } from 'svelte';
  import { AlertTriangle, Activity, CheckCircle2, Database, ExternalLink, LockKeyhole, RefreshCw, Search, ShieldAlert, ShieldCheck, Trash2 } from 'lucide-svelte';
  import {
    getLatestMemeCoinHunterReport, getMemeCoinHunterSecurityStatus, removeMemeCoinHunterSecurityToken,
    runMemeCoinHunter, saveMemeCoinHunterSecurityToken
  } from '../adapters/tauri/bridge';
  import type { MemeCoinHunterCandidate, MemeCoinHunterReport, MemeCoinHunterSecurityStatus } from '../types';

  let report: MemeCoinHunterReport | null = null;
  let securityStatus: MemeCoinHunterSecurityStatus | null = null;
  let securityToken = '';
  let selected: MemeCoinHunterCandidate | null = null;
  let query = '';
  let sortBy: 'scalp' | 'risk' | 'liquidity' = 'scalp';
  let loading = false;
  let credentialBusy = false;
  let errorMessage = '';
  let noticeMessage = '';

  onMount(() => { void loadState(); });

  async function loadState(): Promise<void> {
    loading = true;
    errorMessage = '';
    try {
      const [savedReport, savedSecurityStatus] = await Promise.all([
        getLatestMemeCoinHunterReport(), getMemeCoinHunterSecurityStatus()
      ]);
      report = savedReport;
      securityStatus = savedSecurityStatus;
      selected = report?.candidates[0] ?? null;
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      loading = false;
    }
  }

  async function scan(): Promise<void> {
    loading = true;
    errorMessage = '';
    noticeMessage = '';
    try {
      report = await runMemeCoinHunter();
      selected = report.candidates[0] ?? null;
      noticeMessage = `Scan ${report.runId} committed. ${report.scalpSetupCount} setup candidate(s), ${report.blockedCount} blocked candidate(s).`;
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
      const savedReport = await getLatestMemeCoinHunterReport().catch(() => null);
      if (savedReport) report = savedReport;
    } finally {
      loading = false;
    }
  }

  async function saveCredential(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    credentialBusy = true;
    errorMessage = '';
    noticeMessage = '';
    try {
      securityStatus = await saveMemeCoinHunterSecurityToken(securityToken);
      securityToken = '';
      noticeMessage = 'GoPlus access token saved securely. Run a new scan to apply security checks to candidates.';
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      credentialBusy = false;
    }
  }

  async function removeCredential(): Promise<void> {
    credentialBusy = true;
    errorMessage = '';
    noticeMessage = '';
    try {
      securityStatus = await removeMemeCoinHunterSecurityToken();
      noticeMessage = 'GoPlus credential removed from the operating-system credential vault.';
    } catch (error) {
      errorMessage = error instanceof Error ? error.message : String(error);
    } finally {
      credentialBusy = false;
    }
  }

  $: candidates = [...(report?.candidates ?? [])]
    .filter((candidate) => `${candidate.symbol} ${candidate.name} ${candidate.chain} ${candidate.tokenAddress}`.toLowerCase().includes(query.toLowerCase()))
    .sort((left, right) => sortBy === 'risk' ? left.riskScore - right.riskScore
      : sortBy === 'liquidity' ? right.liquidityUsd - left.liquidityUsd
        : right.scalpScore - left.scalpScore);

  function money(value: number | null, compact = false): string {
    if (value == null || !Number.isFinite(value)) return '—';
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', notation: compact ? 'compact' : 'standard', maximumFractionDigits: value < 0.01 ? 8 : 2 }).format(value);
  }

  function percent(value: number | null): string {
    if (value == null || !Number.isFinite(value)) return '—';
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
  }

  function ageLabel(timestamp: number | null | undefined): string {
    if (timestamp == null || !Number.isSafeInteger(timestamp) || timestamp <= 0) return 'Unavailable';
    const age = Date.now() - timestamp;
    if (age < 0) return 'Clock skew';
    if (age < 60_000) return `${Math.floor(age / 1000)}s ago`;
    if (age < 3_600_000) return `${Math.floor(age / 60_000)}m ago`;
    return `${(age / 3_600_000).toFixed(1)}h ago`;
  }

  function addressLabel(value: string): string {
    return value.length > 18 ? `${value.slice(0, 8)}…${value.slice(-6)}` : value;
  }
</script>

<section class="hunter-heading">
  <div>
    <div class="eyebrow">ON-CHAIN DISCOVERY / SHORT-HORIZON RESEARCH</div>
    <h1>Meme Coin Hunter</h1>
    <p>Multi-source token discovery, contract-risk screening and short-window scalp setup ranking. No transaction signing or execution authority.</p>
  </div>
  <div class="heading-actions">
    <span class="authority-pill"><LockKeyhole size={13}/> RESEARCH ONLY</span>
    <button class="primary-button" type="button" onclick={scan} disabled={loading}>
      <RefreshCw size={14} class={loading ? 'spin' : ''}/>{loading ? 'Scanning…' : 'Run hunter scan'}
    </button>
  </div>
</section>

<section class="hunter-grid">
  <article class="metric-card"><span>Discovered</span><strong>{report?.discoveredCount ?? '—'}</strong><small>Unique token identities</small></article>
  <article class="metric-card"><span>Market candidates</span><strong>{report?.candidateCount ?? '—'}</strong><small>Validated DEX pair observations</small></article>
  <article class="metric-card"><span>Scalp setups</span><strong>{report?.scalpSetupCount ?? '—'}</strong><small>All configured gates required</small></article>
  <article class="metric-card"><span>Hard-blocked</span><strong>{report?.blockedCount ?? '—'}</strong><small>Critical contract or token-control risk detected</small></article>
</section>

{#if errorMessage}<div class="message error" role="alert"><AlertTriangle size={15}/>{errorMessage}</div>{/if}
{#if noticeMessage}<div class="message notice" role="status"><CheckCircle2 size={15}/>{noticeMessage}</div>{/if}

<section class="panel source-panel">
  <div class="section-header">
    <div><h2>Discovery and risk providers</h2><p>{report?.providerState ?? 'NOT SCANNED'} · {report ? `Last scan ${ageLabel(report.scannedAtMs)}` : 'No scan report stored'}</p></div>
    {#if report}<code>{report.datasetFingerprintSha256.slice(0, 20)}…</code>{/if}
  </div>
  <div class="source-grid">
    {#each report?.sources ?? [] as source (source.provider)}
      <article class="source-entry">
        <div><strong>{source.provider}</strong><span class:connected={source.state === 'connected'} class:degraded={source.state !== 'connected'}>{source.state.toUpperCase()}</span></div>
        <small>{source.discoveredCount} observation(s) · {ageLabel(source.receivedAtMs)}</small>
        <p>{source.detail}</p>
      </article>
    {/each}
    {#if !report}<div class="empty-state"><Database size={20}/>Run a native scan to load source-backed token observations. No sample rows are injected.</div>{/if}
  </div>
</section>

<section class="panel security-panel">
  <div class="section-header">
    <div><h2>Contract security gate</h2><p>GoPlus token-security records are checked natively; credentials remain outside frontend storage.</p></div>
    <span class:connected={securityStatus?.credentialConfigured} class:degraded={!securityStatus?.credentialConfigured} class="security-state">{securityStatus?.credentialConfigured ? 'CREDENTIAL STORED' : 'NOT CONFIGURED'}</span>
  </div>
  <div class="security-row">
    <ShieldCheck size={18}/>
    <div><strong>{securityStatus?.secureStoreAvailable ? 'Operating-system credential vault available' : 'Credential vault unavailable'}</strong><p>{securityStatus?.note ?? 'Security credential status has not been loaded.'}</p></div>
  </div>
  <form class="credential-form" onsubmit={saveCredential}>
    <label>GoPlus bearer access token
      <input type="password" bind:value={securityToken} minlength="32" maxlength="4096" autocomplete="new-password" spellcheck="false" placeholder="Paste GoPlus access token" disabled={credentialBusy || !securityStatus?.secureStoreAvailable}/>
    </label>
    <button type="submit" disabled={credentialBusy || securityToken.length < 32 || !securityStatus?.secureStoreAvailable}>Save securely</button>
    <button class="secondary-button" type="button" onclick={removeCredential} disabled={credentialBusy || !securityStatus?.credentialConfigured}><Trash2 size={13}/> Remove</button>
  </form>
  <small class="credential-note">Token is only sent from the native Rust process to api.gopluslabs.io. It is never returned to the UI or written to plaintext configuration files.</small>
</section>

<section class="panel candidates-panel">
  <div class="section-header candidate-header">
    <div><h2>Scalp candidate matrix</h2><p>Signal score is a deterministic heuristic, not a win probability or a buy/sell instruction.</p></div>
    <div class="table-controls">
      <label><Search size={14}/><input bind:value={query} placeholder="Search token / chain / address"/></label>
      <select bind:value={sortBy} aria-label="Sort candidates"><option value="scalp">Scalp score</option><option value="risk">Lowest risk</option><option value="liquidity">Liquidity</option></select>
    </div>
  </div>
  <div class="table-scroll">
    <table>
      <thead><tr><th>Token</th><th>5m</th><th>5m volume</th><th>Buy pressure</th><th>Velocity</th><th>Liquidity</th><th>Age</th><th>Security</th><th>Risk</th><th>Scalp</th><th></th></tr></thead>
      <tbody>
        {#each candidates as candidate (candidate.chain + ':' + candidate.tokenAddress)}
          <tr class:selected={selected?.tokenAddress === candidate.tokenAddress && selected?.chain === candidate.chain}>
            <td><div class="asset"><strong>{candidate.symbol}</strong><small>{candidate.chain.toUpperCase()} · {candidate.name}</small></div><code>{addressLabel(candidate.tokenAddress)}</code></td>
            <td class:positive={(candidate.priceChange5mPct ?? 0) >= 0} class:negative={(candidate.priceChange5mPct ?? 0) < 0}>{percent(candidate.priceChange5mPct)}</td>
            <td>{money(candidate.volume5mUsd, true)}</td>
            <td>{candidate.buyPressure5mPct == null ? '—' : `${candidate.buyPressure5mPct.toFixed(1)}%`}</td>
            <td>{candidate.volumeVelocity5m == null ? '—' : `${candidate.volumeVelocity5m.toFixed(2)}x`}</td>
            <td>{money(candidate.liquidityUsd, true)}</td>
            <td>{candidate.poolAgeMinutes == null ? '—' : `${Math.floor(candidate.poolAgeMinutes / 60)}h`}</td>
            <td><span class:good={candidate.securityState === 'verified'} class:bad={candidate.securityState === 'blocked' || candidate.securityState === 'unavailable'} class:warn={candidate.securityState === 'partial'} class="state-pill">{candidate.securityState}</span></td>
            <td><span class="risk-number" class:bad={candidate.riskScore >= 65} class:warn={candidate.riskScore >= 35 && candidate.riskScore < 65}>{candidate.riskScore}/100</span></td>
            <td><span class="setup-state" class:setup={candidate.scalpState === 'SETUP'} class:blocked={candidate.scalpState === 'BLOCKED'}>{candidate.scalpState}</span><small>{candidate.scalpScore}/100</small></td>
            <td><button class="inspect-button" type="button" onclick={() => selected = candidate}>Inspect</button></td>
          </tr>
        {/each}
        {#if candidates.length === 0}<tr><td colspan="11"><div class="empty-state"><Database size={19}/>{report ? 'No candidate matches the current filter.' : 'No scan observations are available.'}</div></td></tr>{/if}
      </tbody>
    </table>
  </div>
</section>

{#if selected}
  <section class="panel detail-panel">
    <div class="section-header"><div><h2>{selected.symbol} · Evidence detail</h2><p>{selected.chain.toUpperCase()} · {selected.name} · {addressLabel(selected.tokenAddress)}</p></div>{#if selected.pairUrl}<a href={selected.pairUrl} target="_blank" rel="noopener noreferrer">Open DEX pair <ExternalLink size={12}/></a>{/if}</div>
    <div class="detail-grid">
      <div><span>Price</span><strong>{money(selected.priceUsd)}</strong></div>
      <div><span>5m / 1h / 24h</span><strong>{percent(selected.priceChange5mPct)} / {percent(selected.priceChange1hPct)} / {percent(selected.priceChange24hPct)}</strong></div>
      <div><span>Volume 1h / 24h</span><strong>{money(selected.volume1hUsd, true)} / {money(selected.volume24hUsd, true)}</strong></div>
      <div><span>Top 10 holders</span><strong>{selected.holderTop10Pct == null ? 'Unknown' : `${selected.holderTop10Pct.toFixed(1)}%`}</strong></div>
      <div><span>Top 10 LP locked</span><strong>{selected.liquidityLockedTop10Pct == null ? 'Unknown' : `${selected.liquidityLockedTop10Pct.toFixed(1)}%`}</strong></div>
      <div><span>Buy / sell tax</span><strong>{selected.buyTaxPct == null ? 'Unknown' : `${selected.buyTaxPct.toFixed(2)}%`} / {selected.sellTaxPct == null ? 'Unknown' : `${selected.sellTaxPct.toFixed(2)}%`}</strong></div>
    </div>
    <div class="evidence-columns">
      <div><h3><ShieldAlert size={14}/> Contract security evidence</h3>{#if selected.securityFlags.length}<ul>{#each selected.securityFlags as flag}<li>{flag}</li>{/each}</ul>{:else}<p>No risk flags returned. This is not a safety guarantee.</p>{/if}</div>
      <div><h3><Activity size={14}/> Scalp gate evaluation</h3>{#if selected.scalpReasons.length}<ul>{#each selected.scalpReasons as reason}<li>{reason}</li>{/each}</ul>{:else}<p>No missing gate explanation was returned.</p>{/if}</div>
    </div>
    <div class="provenance-note">Discovery: {selected.discoverySources.join(' · ')} · Market observation received {ageLabel(selected.receivedAtMs)} · Security evidence {ageLabel(selected.securityCheckedAtMs)}.</div>
  </section>
{/if}

<section class="limitations-panel">
  <div><AlertTriangle size={16}/><strong>Execution limitations</strong></div>
  <ul>{#each report?.decisionLimitations ?? [
    'Market data has not yet been loaded.',
    'A scalp setup is not an execution approval or guarantee of return.',
    'Route quote, transaction simulation, price impact, gas and wallet risk checks are not performed by this scanner.'
  ] as limitation}<li>{limitation}</li>{/each}</ul>
  <p>Provider refresh is limited to one scan per 60 seconds. GeckoTerminal public endpoints are cached and rate-limited; this scanner is not a sub-second execution feed.</p>
</section>

<style>
  .hunter-heading{display:flex;align-items:flex-end;justify-content:space-between;gap:18px;margin-bottom:18px}
  .eyebrow{font-size:9px;font-weight:750;letter-spacing:.12em;color:var(--accent,#6ea8fe)}
  .hunter-heading h1{margin:5px 0;font-size:24px}
  .hunter-heading p,.section-header p{margin:0;color:var(--muted,#929bad);font-size:12px;line-height:1.5;max-width:740px}
  .heading-actions{display:flex;align-items:center;gap:10px;flex-shrink:0}
  .authority-pill,.security-state,.state-pill,.setup-state{display:inline-flex;align-items:center;gap:5px;border-radius:6px;padding:5px 7px;font-size:9px;font-weight:750;letter-spacing:.04em;background:var(--surface-2,#182233);color:var(--muted,#a7b1c2)}
  .primary-button,.credential-form button,.secondary-button,.inspect-button{display:inline-flex;align-items:center;justify-content:center;gap:6px;padding:8px 11px;border-radius:7px;border:1px solid var(--border,#2c394d);background:var(--accent,#225cbd);color:#fff;font:inherit;font-size:11px;font-weight:650;cursor:pointer}
  button:disabled{opacity:.48;cursor:not-allowed}
  .hunter-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:10px;margin-bottom:14px}
  .metric-card{padding:14px;border:1px solid var(--border,#283244);border-radius:9px;background:var(--surface,#111a28);display:grid;gap:5px}
  .metric-card span,.detail-grid span{font-size:10px;color:var(--muted,#929bad)}
  .metric-card strong{font-size:22px;font-variant-numeric:tabular-nums}
  .metric-card small{font-size:10px;color:var(--muted,#929bad)}
  .panel{padding:16px;margin-bottom:13px;border:1px solid var(--border,#283244);border-radius:10px;background:var(--surface,#111a28)}
  .section-header{display:flex;align-items:flex-start;justify-content:space-between;gap:14px;margin-bottom:13px}
  .section-header h2{margin:0 0 4px;font-size:14px}
  .section-header code{font-size:10px;color:var(--muted,#929bad)}
  .source-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:9px}
  .source-entry{padding:11px;border:1px solid var(--border,#283244);border-radius:8px;min-width:0}
  .source-entry>div{display:flex;align-items:center;justify-content:space-between;gap:8px}
  .source-entry strong{font-size:11px}
  .source-entry span{font-size:9px;font-weight:750;color:#e0c17a}
  .source-entry span.connected{color:#86d4aa}.source-entry span.degraded{color:#e0a16f}
  .source-entry small,.source-entry p{font-size:10px;color:var(--muted,#929bad);line-height:1.45}
  .source-entry small{display:block;margin-top:7px}.source-entry p{margin:5px 0 0}
  .security-state.connected{background:#153729;color:#86d4aa}.security-state.degraded{background:#3a2b1d;color:#e0c17a}
  .security-row{display:flex;align-items:flex-start;gap:10px;color:var(--accent,#77aaff)}
  .security-row strong{font-size:11px;color:var(--text,#edf1f7)}
  .security-row p{margin:3px 0;color:var(--muted,#929bad);font-size:10px}
  .credential-form{display:grid;grid-template-columns:minmax(0,1fr) auto auto;gap:8px;align-items:end;margin-top:12px}
  .credential-form label{display:grid;gap:5px;color:var(--muted,#929bad);font-size:10px}
  .credential-form input,.table-controls input,.table-controls select{min-width:0;padding:8px;border:1px solid var(--border,#283244);border-radius:6px;background:var(--surface-2,#182233);color:var(--text,#edf1f7);font:inherit;font-size:11px}
  .credential-form .secondary-button{background:transparent;color:var(--text,#edf1f7)}
  .credential-note{display:block;margin-top:8px;color:var(--muted,#929bad);font-size:9px;line-height:1.5}
  .table-controls{display:flex;align-items:center;gap:8px}
  .table-controls label{display:flex;align-items:center;gap:6px;padding-left:8px;border:1px solid var(--border,#283244);border-radius:6px;color:var(--muted,#929bad)}
  .table-controls input{border:0;background:transparent;width:180px}
  .table-controls select{min-width:125px}
  .table-scroll{overflow:auto}
  table{width:100%;min-width:940px;border-collapse:collapse;font-size:10px}
  th,td{padding:9px 8px;border-bottom:1px solid var(--border,#283244);text-align:right;white-space:nowrap}
  th{font-size:9px;color:var(--muted,#929bad);font-weight:650}
  td:first-child,th:first-child{text-align:left}
  tbody tr.selected{background:rgba(64,121,205,.09)}
  .asset{display:grid;gap:2px}.asset strong{font-size:11px}.asset small{font-size:9px;color:var(--muted,#929bad)}
  td code{font-size:9px;color:var(--muted,#929bad)}
  .positive{color:#79d7a2}.negative{color:#e58e9a}
  .state-pill.good{background:#153729;color:#86d4aa}.state-pill.warn{background:#3a2b1d;color:#e0c17a}.state-pill.bad{background:#40242c;color:#ed9da7}
  .risk-number{font-weight:750}.risk-number.bad{color:#ed9da7}.risk-number.warn{color:#e0c17a}
  .setup-state{background:#263348;color:#c1ccdc}.setup-state.setup{background:#153729;color:#86d4aa}.setup-state.blocked{background:#40242c;color:#ed9da7}
  td .setup-state{display:inline-flex}td>small{display:block;margin-top:3px;color:var(--muted,#929bad);font-size:9px}
  .inspect-button{background:transparent;color:var(--text,#edf1f7);padding:5px 7px;font-size:10px}
  .detail-panel a{display:inline-flex;align-items:center;gap:5px;color:var(--accent,#77aaff);font-size:10px;text-decoration:none}
  .detail-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px}
  .detail-grid>div{display:grid;gap:5px;padding:10px;border:1px solid var(--border,#283244);border-radius:7px;min-width:0}
  .detail-grid strong{font-size:11px;overflow-wrap:anywhere}
  .evidence-columns{display:grid;grid-template-columns:1fr 1fr;gap:12px;margin-top:13px}
  .evidence-columns>div{padding:11px;border:1px solid var(--border,#283244);border-radius:8px;min-width:0}
  .evidence-columns h3{display:flex;align-items:center;gap:6px;margin:0 0 8px;font-size:11px}
  .evidence-columns ul,.limitations-panel ul{margin:0;padding-left:16px;color:var(--muted,#a2acbd);font-size:10px;line-height:1.55}
  .evidence-columns li+li,.limitations-panel li+li{margin-top:4px}
  .evidence-columns p,.provenance-note{font-size:10px;color:var(--muted,#929bad);line-height:1.5}
  .provenance-note{margin-top:11px;padding-top:10px;border-top:1px solid var(--border,#283244)}
  .limitations-panel{padding:12px;border:1px solid #59452b;border-radius:9px;background:rgba(96,67,27,.12)}
  .limitations-panel>div{display:flex;align-items:center;gap:7px;margin-bottom:7px;color:#e0c17a;font-size:11px}
  .limitations-panel p{margin:8px 0 0;color:var(--muted,#929bad);font-size:9px;line-height:1.5}
  .message{display:flex;align-items:flex-start;gap:8px;padding:10px;margin-bottom:12px;border-radius:7px;font-size:11px;line-height:1.5}
  .message.error{background:#40242c;color:#ed9da7}.message.notice{background:#153729;color:#86d4aa}
  .empty-state{display:flex;align-items:center;gap:8px;padding:16px;color:var(--muted,#929bad);font-size:11px}
  @media(max-width:900px){.hunter-grid{grid-template-columns:repeat(2,minmax(0,1fr))}.hunter-heading{align-items:flex-start;flex-direction:column}.credential-form{grid-template-columns:1fr auto}.credential-form label{grid-column:1/-1}.candidate-header{flex-direction:column}.table-controls{width:100%}.table-controls label{flex:1}.table-controls input{width:100%}}
  @media(max-width:600px){.source-grid,.detail-grid,.evidence-columns{grid-template-columns:1fr}.panel{padding:12px}.hunter-heading h1{font-size:21px}.heading-actions{width:100%;justify-content:space-between}}
</style>
