<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { confirm, open } from '@tauri-apps/plugin-dialog';
  import { evaluateAgentResearchReport } from '../application/ai-agent-orchestrator.js';

  export let desktop = false;
  export let symbol = 'BTCUSDT';
  export let timeframe = '5m';
  export let marketReady = false;

  type ProviderStatus = { id: string; label: string; credentialConfigured: boolean; secureStoreAvailable: boolean; keyRequired: boolean; connectionMode: string };
  type ImportedModel = { id: string; filename: string; format: string; sizeBytes: number; sha256: string; importedAt: string; executionStatus: string };
  type RuntimeStatus = { providers: ProviderStatus[]; importedModels: ImportedModel[]; agentRunBudget: { hourlyLimit: number; hourlyUsed: number; dailyLimit: number; dailyUsed: number }; audit: { valid: boolean; eventCount: number; headSha256: string; firstInvalidSequence: number | null }; localModelInferenceAvailable: boolean; executionAuthority: string };

  let runtimeStatus: RuntimeStatus | null = null;
  let selectedProvider = 'openai';
  let apiKey = '';
  let modelName = '';
  let busyAction = '';
  let errorText = '';
  let statusText = '';
  let connectionResult: any = null;
  let researchReport: any = null;
  let strictResult: any = null;
  let strictError = '';
  let selectedModelId = '';
  let expectedModelSha256 = '';

  $: selectedProviderStatus = runtimeStatus?.providers.find((provider) => provider.id === selectedProvider) ?? null;

  onMount(() => { if (desktop) void refreshRuntime(); });

  function explain(error: unknown): string {
    return error instanceof Error ? error.message : String(error || 'Unknown operation error.');
  }

  async function refreshRuntime() {
    if (!desktop) return;
    busyAction = 'refresh'; errorText = '';
    try {
      runtimeStatus = await invoke<RuntimeStatus>('get_ai_agent_runtime_status');
    } catch (error) {
      errorText = explain(error);
    } finally { busyAction = ''; }
  }

  function changeProvider(providerId: string) {
    selectedProvider = providerId;
    apiKey = '';
    connectionResult = null;
    statusText = '';
    errorText = '';
    modelName = '';
  }

  async function saveKey() {
    if (!desktop || !selectedProviderStatus?.keyRequired) return;
    busyAction = 'save-key'; errorText = ''; statusText = '';
    try {
      await invoke<ProviderStatus>('save_ai_provider_key', { providerId: selectedProvider, apiKey });
      apiKey = '';
      await refreshRuntime();
      statusText = 'Credential saved to the operating-system credential store. No plaintext fallback is used.';
    } catch (error) { errorText = explain(error); }
    finally { busyAction = ''; }
  }

  async function removeKey() {
    if (!desktop || !selectedProviderStatus?.keyRequired) return;
    busyAction = 'remove-key'; errorText = ''; statusText = '';
    try {
      await invoke<ProviderStatus>('remove_ai_provider_key', { providerId: selectedProvider });
      apiKey = '';
      connectionResult = null;
      await refreshRuntime();
      statusText = 'Stored credential removed from the operating-system credential store.';
    } catch (error) { errorText = explain(error); }
    finally { busyAction = ''; }
  }

  async function testProvider() {
    if (!desktop) return;
    busyAction = 'test-provider'; errorText = ''; statusText = ''; connectionResult = null;
    try {
      connectionResult = await invoke('test_ai_provider_connection', { providerId: selectedProvider });
      await refreshRuntime();
      statusText = connectionResult.connected ? 'Provider connectivity check succeeded; no prompt was sent.' : 'Provider connectivity check failed.';
    } catch (error) { errorText = explain(error); }
    finally { busyAction = ''; }
  }

  async function importModel() {
    if (!desktop) return;
    errorText = ''; statusText = '';
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: 'Import a local model artifact',
        filters: [{ name: 'Supported model artifacts', extensions: ['gguf', 'onnx', 'safetensors'] }],
      });
      if (!selected || Array.isArray(selected)) return;
      const expectedDigest = expectedModelSha256.trim();
      if (expectedDigest && !/^[a-fA-F0-9]{64}$/.test(expectedDigest)) {
        errorText = 'Expected SHA-256 must contain exactly 64 hexadecimal characters.';
        return;
      }
      busyAction = 'import-model';
      const imported = await invoke<ImportedModel>('import_local_ai_model', { sourcePath: selected, expectedSha256: expectedDigest || null });
      expectedModelSha256 = '';
      selectedModelId = imported.id;
      await refreshRuntime();
      statusText = `Imported and fingerprinted ${imported.filename}. The artifact is registered, not executable by this application yet.`;
    } catch (error) { errorText = explain(error); }
    finally { busyAction = ''; }
  }

  async function removeModel(model: ImportedModel) {
    if (!desktop) return;
    const approved = await confirm(`Remove ${model.filename} from the managed model registry? This cannot guarantee physical erasure on SSDs or copy-on-write filesystems.`, {
      title: 'Remove imported model',
      kind: 'warning',
      okLabel: 'Remove model',
      cancelLabel: 'Cancel',
    });
    if (!approved) return;
    busyAction = `remove:${model.id}`; errorText = ''; statusText = '';
    try {
      const result = await invoke<{ id: string; filename: string; sha256: string; fileDeleted: boolean }>('remove_imported_ai_model', { modelId: model.id });
      await refreshRuntime();
      statusText = result.fileDeleted
        ? `Removed ${result.filename} from the registry and model directory. Physical erasure is not guaranteed.`
        : `Removed ${result.filename} from the registry, but file cleanup is still pending; inspect application storage before importing more models.`;
      if (selectedModelId === model.id) selectedModelId = '';
    } catch (error) { errorText = explain(error); }
    finally { busyAction = ''; }
  }

  async function verifyModel(model: ImportedModel) {
    if (!desktop) return;
    busyAction = `verify:${model.id}`; errorText = ''; statusText = '';
    try {
      const verified = await invoke<ImportedModel>('verify_imported_ai_model', { modelId: model.id });
      selectedModelId = verified.id;
      statusText = `Integrity verified: SHA-256 ${verified.sha256}. Local inference is still disabled for imported files.`;
      await refreshRuntime();
    } catch (error) { errorText = explain(error); }
    finally { busyAction = ''; }
  }

  async function verifyAuditChain() {
    if (!desktop) return;
    busyAction = 'verify-audit'; errorText = ''; statusText = '';
    try {
      const audit = await invoke<any>('verify_ai_agent_audit_chain');
      if (runtimeStatus) runtimeStatus = { ...runtimeStatus, audit };
      statusText = audit.valid ? `Audit chain verified: ${audit.eventCount} append-only events.` : `Audit-chain integrity failure at sequence ${audit.firstInvalidSequence}.`;
      if (!audit.valid) errorText = statusText;
    } catch (error) { errorText = explain(error); }
    finally { busyAction = ''; }
  }

  async function runAnalyst() {
    if (!desktop || !marketReady || !modelName.trim()) return;
    busyAction = 'run-agent'; errorText = ''; statusText = ''; researchReport = null; strictResult = null; strictError = '';
    try {
      const result = await invoke<any>('run_ai_research_agent', {
        request: { providerId: selectedProvider, model: modelName.trim(), symbol, timeframe },
      });
      researchReport = result;
      try {
        strictResult = await evaluateAgentResearchReport(result);
        statusText = strictResult.decision.decision === 'BLOCK'
          ? 'Agent report received; strict decision gate blocked candidate handoff.'
          : 'Agent report received; strict evaluation completed using Rust-derived evidence only.';
      } catch (error) {
        strictError = explain(error);
        statusText = 'Agent commentary is available, but strict evaluation rejected the evidence envelope.';
      }
      await refreshRuntime();
    } catch (error) { errorText = explain(error); }
    finally { busyAction = ''; }
  }

  function formatBytes(value: number): string {
    if (!Number.isFinite(value) || value < 0) return 'unknown size';
    const units = ['B', 'KiB', 'MiB', 'GiB'];
    let size = value; let unit = 0;
    while (size >= 1024 && unit < units.length - 1) { size /= 1024; unit += 1; }
    return `${size.toFixed(unit === 0 ? 0 : 2)} ${units[unit]}`;
  }
</script>

<section class="agent-runtime panel">
  <div class="runtime-head">
    <div>
      <div class="eyebrow">AGENT RUNTIME / PROVIDERS / MODEL REGISTRY</div>
      <h2>AI research runtime</h2>
      <p>Provider calls are read-only, bounded, and routed through the native Rust boundary. Model-generated prose never sets the deterministic decision score.</p>
    </div>
    <button class="button-secondary compact" onclick={refreshRuntime} disabled={!desktop || busyAction !== ''}>Refresh runtime</button>
  </div>

  {#if !desktop}
    <div class="runtime-warning"><strong>Native runtime required</strong><span>This capability is unavailable until the Rust/Tauri runtime is connected; no substitute data or credentials are used.</span></div>
  {:else}
    <div class="runtime-grid">
      <section class="runtime-block">
        <div class="runtime-block-title"><h3>Provider & credentials</h3><span class="status-tag" class:status-good={selectedProviderStatus?.credentialConfigured || selectedProvider === 'ollama'} class:status-idle={!selectedProviderStatus?.credentialConfigured && selectedProvider !== 'ollama'}>{selectedProviderStatus?.credentialConfigured ? 'KEY STORED' : selectedProvider === 'ollama' ? 'LOCAL' : 'NO KEY'}</span></div>
        <label class="runtime-field">Provider
          <select value={selectedProvider} onchange={(event) => changeProvider(event.currentTarget.value)} disabled={busyAction !== ''}>
            <option value="openai">OpenAI · HTTPS</option>
            <option value="anthropic">Anthropic · HTTPS</option>
            <option value="openrouter">OpenRouter · HTTPS</option>
            <option value="ollama">Ollama · 127.0.0.1 only</option>
          </select>
        </label>
        {#if selectedProviderStatus?.keyRequired}
          <label class="runtime-field">API key <input type="password" bind:value={apiKey} autocomplete="off" autocapitalize="off" spellcheck="false" placeholder={selectedProviderStatus?.credentialConfigured ? 'Saved securely · enter to rotate key' : 'Paste provider API key'} disabled={busyAction !== ''} /></label>
          <div class="runtime-actions">
            <button class="button-primary" onclick={saveKey} disabled={busyAction !== '' || !apiKey.trim() || selectedProviderStatus?.secureStoreAvailable !== true}>Save key securely</button>
            <button class="button-secondary" onclick={removeKey} disabled={busyAction !== '' || !selectedProviderStatus?.credentialConfigured}>Remove key</button>
          </div>
          {#if selectedProviderStatus && !selectedProviderStatus.secureStoreAvailable}<p class="runtime-error-inline">OS credential store is unavailable. Saving/testing is blocked; no insecure file fallback exists.</p>{/if}
        {:else}
          <div class="runtime-local-note">No API key is needed. Ollama is contacted only at the hard-coded loopback address 127.0.0.1:11434; arbitrary URLs and remote HTTP endpoints are not accepted.</div>
        {/if}
        <label class="runtime-field">Provider model ID
          <input bind:value={modelName} autocomplete="off" spellcheck="false" placeholder={selectedProvider === 'ollama' ? 'e.g. name of an installed Ollama model' : 'Enter exact model identifier'} disabled={busyAction !== ''} />
        </label>
        <div class="runtime-actions">
          <button class="button-secondary" onclick={testProvider} disabled={busyAction !== '' || (selectedProviderStatus?.keyRequired === true && selectedProviderStatus?.credentialConfigured !== true)}>{busyAction === 'test-provider' ? 'Testing…' : 'Test provider'}</button>
          <button class="button-primary" onclick={runAnalyst} disabled={busyAction !== '' || !modelName.trim() || !marketReady || !selectedProviderStatus || (selectedProviderStatus.keyRequired && (selectedProviderStatus.credentialConfigured !== true || selectedProviderStatus.secureStoreAvailable !== true)) || (runtimeStatus?.agentRunBudget.hourlyUsed ?? 0) >= (runtimeStatus?.agentRunBudget.hourlyLimit ?? 0) || (runtimeStatus?.agentRunBudget.dailyUsed ?? 0) >= (runtimeStatus?.agentRunBudget.dailyLimit ?? 0)}>{busyAction === 'run-agent' ? 'Analyzing…' : 'Run research analyst'}</button>
        </div>
        <p class="runtime-note">Research scope: <strong>{symbol} · {timeframe}</strong>. Dataset identity is recomputed by Rust. The model has no tools, order endpoint, or execution permission.</p>
      </section>

      <section class="runtime-block model-registry-block">
        <div class="runtime-block-title"><h3>Local model artifact registry</h3><span class="status-tag status-idle">NO LOCAL INFERENCE</span></div>
        <p class="runtime-copy">Import GGUF, ONNX, or SafeTensors weights into the managed app directory. Each artifact is copied under a generated filename and streamed through SHA-256; import does not execute or load untrusted model code.</p>
        <label class="runtime-field">Expected artifact SHA-256 (optional)
          <input bind:value={expectedModelSha256} inputmode="text" autocomplete="off" autocapitalize="off" spellcheck="false" placeholder="64-character trusted digest" disabled={busyAction !== ''} />
        </label>
        <p class="runtime-note">When a trusted digest is available, supply it before selecting the file; a mismatch aborts registration.</p>
        <button class="button-secondary" onclick={importModel} disabled={busyAction !== ''}>{busyAction === 'import-model' ? 'Importing…' : 'Import model file'}</button>
        {#if runtimeStatus?.importedModels?.length}
          <div class="model-list">
            {#each runtimeStatus.importedModels as model}
              <article class="model-row">
                <div class="model-row-main"><strong>{model.filename}</strong><span>{model.format} · {formatBytes(model.sizeBytes)} · {new Date(model.importedAt).toLocaleString()}</span><code>{model.sha256}</code><small>{model.executionStatus.replaceAll('_', ' ')}</small></div>
                <div class="model-actions"><button class="button-secondary compact" onclick={() => verifyModel(model)} disabled={busyAction !== ''}>{busyAction === `verify:${model.id}` ? 'Verifying…' : 'Verify'}</button><button class="button-secondary compact model-remove" onclick={() => removeModel(model)} disabled={busyAction !== ''}>{busyAction === `remove:${model.id}` ? 'Removing…' : 'Remove'}</button></div>
              </article>
            {/each}
          </div>
        {:else}
          <div class="runtime-empty">No local model artifacts registered.</div>
        {/if}
        <p class="runtime-note">Local inference is not implemented in this build. For local inference now, run Ollama separately and select its already-installed model; imported files are catalogued and integrity-verifiable only.</p>
      </section>
    </div>
  {/if}

  {#if runtimeStatus?.agentRunBudget}
    <div class="runtime-budget"><strong>Provider abuse / cost budget</strong><span>{runtimeStatus.agentRunBudget.hourlyUsed}/{runtimeStatus.agentRunBudget.hourlyLimit} runs per hour · {runtimeStatus.agentRunBudget.dailyUsed}/{runtimeStatus.agentRunBudget.dailyLimit} per 24 hours</span><small>Failed runs consume the same persisted budget. Each provider completion is capped at 1,200 output tokens; this is a request-count guard, not a billing estimate.</small></div>
  {/if}
  {#if errorText}<div class="runtime-result error" role="alert">{errorText}</div>{/if}
  {#if statusText}<div class="runtime-result" role="status">{statusText}</div>{/if}
  {#if connectionResult}<div class="connection-result" class:failed={!connectionResult.connected}><strong>{connectionResult.connected ? 'Provider reachable' : 'Provider unavailable'}</strong><span>{connectionResult.message}</span><small>{connectionResult.checkedAt}{#if connectionResult.modelCount !== null && connectionResult.modelCount !== undefined} · {connectionResult.modelCount} model entries returned{/if}</small></div>{/if}

  {#if researchReport}
    <section class="agent-report-block">
      <div class="runtime-block-title"><h3>Research analyst output</h3><span class="status-tag status-idle">COMMENTARY ONLY</span></div>
      <p class="agent-summary">{researchReport.summary}</p>
      <div class="report-meta"><span>Provider: {researchReport.providerId} / {researchReport.model}</span><span>Scope: {researchReport.marketContext.symbol} {researchReport.marketContext.timeframe}</span><span>Closed-bar cutoff: {researchReport.marketContext.asOf}</span><span>Snapshot received: {new Date(researchReport.marketContext.retrievedAtMs).toLocaleString()}</span><span>Freshness: {researchReport.marketContext.isFresh ? 'FRESH' : 'STALE / UNVERIFIED'}</span></div>
      <div class="observation-grid">
        {#each researchReport.observations as observation}
          <article class="observation-item"><div><strong>{observation.domain.toUpperCase()}</strong><span>{observation.evidenceRefs.join(', ')}</span></div><p>{observation.assessment}</p>{#if observation.caveats.length}<small>{observation.caveats.join(' · ')}</small>{/if}</article>
        {/each}
      </div>
      {#if researchReport.disagreements.length}<div class="report-sublist"><strong>Evidence disagreements</strong>{#each researchReport.disagreements as issue}<p>{issue}</p>{/each}</div>{/if}
      {#if researchReport.followUpChecks.length}<div class="report-sublist"><strong>Recommended verification</strong>{#each researchReport.followUpChecks as check}<p>{check}</p>{/each}</div>{/if}
      <div class="report-digest"><span>Dataset SHA-256</span><code>{researchReport.marketContext.datasetSha256}</code><span>Output SHA-256</span><code>{researchReport.outputSha256}</code><span>Evidence envelope SHA-256</span><code>{researchReport.evidenceEnvelopeSha256}</code><span>Run ID</span><code>{researchReport.runId}</code></div>
      {#if strictError}<div class="runtime-result error">Strict gate rejected the envelope: {strictError}</div>{:else if strictResult}
        <div class="strict-verdict" class:blocked={strictResult.decision.decision === 'BLOCK'}>
          <div class="runtime-block-title"><h3>Strict deterministic decision gate</h3><span class="status-tag" class:status-good={strictResult.decision.decision !== 'BLOCK'} class:status-bad={strictResult.decision.decision === 'BLOCK'}>{strictResult.decision.decision.replaceAll('_', ' ')}</span></div>
          <p>Score inputs: <strong>Rust-derived deterministic evidence only</strong>. AI prose, labels, and confidence do not change the score. Registry: {strictResult.registryRecordCount} records · {strictResult.registrySha256}</p>
          {#if strictResult.decision.reasons.length}<ul>{#each strictResult.decision.reasons as reason}<li>{reason}</li>{/each}</ul>{/if}
          <div class="authority-note">Execution authority: NONE · Order intent: NONE · Live execution authorized: NO</div>
        </div>
      {/if}
      <p class="runtime-note">Model output is untrusted commentary. A research candidate is not a trade recommendation guarantee, paper-order approval, or live execution grant.</p>
    </section>
  {/if}

  <section class="agent-audit-footer">
    <div><strong>Append-only agent journal</strong><span>{runtimeStatus?.audit?.eventCount ?? 0} events · {runtimeStatus?.audit?.valid ? 'chain verified' : 'verification required'}</span><code>{runtimeStatus?.audit?.headSha256 ?? 'Not loaded'}</code></div>
    <button class="button-secondary compact" onclick={verifyAuditChain} disabled={!desktop || busyAction !== ''}>{busyAction === 'verify-audit' ? 'Verifying…' : 'Verify audit chain'}</button>
  </section>
</section>

<style>
  .agent-runtime{padding:15px;margin:0 0 13px;min-width:0}.runtime-head{display:flex;justify-content:space-between;align-items:flex-start;gap:12px;padding-bottom:13px;border-bottom:1px solid var(--border)}.runtime-head h2{font-size:13px;margin:4px 0 5px;color:#e1edfc}.runtime-head p,.runtime-copy,.runtime-note{margin:0;color:#839bb8;font-size:9px;line-height:1.65}.runtime-head p{max-width:760px}.runtime-grid{display:grid;grid-template-columns:minmax(0,.95fr) minmax(0,1.05fr);gap:12px;margin-top:13px}.runtime-block{min-width:0;border:1px solid #1b3048;border-radius:8px;background:#081625;padding:12px}.runtime-block-title{display:flex;align-items:center;justify-content:space-between;gap:10px;margin-bottom:10px}.runtime-block-title h3{margin:0;font-size:10px;color:#deebfc}.runtime-field{display:flex;flex-direction:column;gap:6px;margin:10px 0;color:#93abc7;font-size:9px;font-weight:600}.runtime-field input,.runtime-field select{width:100%;min-width:0;height:34px;border:1px solid #27415d;border-radius:6px;background:#091b2e;color:#dce9fa;padding:0 9px;outline:none}.runtime-field input:focus,.runtime-field select:focus{border-color:#3683bf}.runtime-actions{display:flex;gap:7px;flex-wrap:wrap;margin-top:9px}.runtime-local-note,.runtime-warning{padding:10px;border:1px solid #51412c;border-radius:6px;background:#201d17;color:#e5c68d;font-size:9px;line-height:1.6;margin:10px 0}.runtime-warning{display:flex;gap:10px;flex-direction:column}.runtime-note{margin-top:10px}.runtime-budget{display:flex;flex-direction:column;gap:5px;margin-top:12px;padding:10px;border:1px solid #28415b;border-radius:6px;background:#081827}.runtime-budget strong{font-size:9px;color:#b5d7fb}.runtime-budget span,.runtime-budget small{font-size:8px;line-height:1.55;color:#90a8c5}.runtime-error-inline{color:#ffb1bb;font-size:9px;line-height:1.5}.runtime-copy{margin-bottom:10px}.runtime-empty{padding:15px 8px;border:1px dashed #28415b;border-radius:6px;color:#7088a6;text-align:center;font-size:9px;margin:11px 0}.model-list{display:flex;flex-direction:column;gap:7px;margin-top:10px}.model-row{display:flex;align-items:flex-start;gap:9px;padding:9px;border:1px solid #1d3149;border-radius:6px;background:#0a192a}.model-row-main{min-width:0;flex:1;display:flex;flex-direction:column;gap:4px}.model-actions{display:flex;gap:5px;flex-wrap:wrap}.model-remove{border-color:#67404b;color:#ffc2ca}.model-row-main strong{color:#dceafd;font-size:9px;overflow-wrap:anywhere}.model-row-main span,.model-row-main small{font-size:8px;color:#829ab7;line-height:1.45}.model-row-main code,.report-digest code,.agent-audit-footer code{overflow-wrap:anywhere;white-space:normal;font:8px var(--mono);color:#8bbaf0}.runtime-result{margin-top:10px;padding:9px 11px;border:1px solid #1c6656;background:#0c2a26;color:#98ebd2;border-radius:6px;font-size:9px;line-height:1.6}.runtime-result.error{border-color:#783948;background:#321b24;color:#ffb7c0}.connection-result{display:flex;flex-direction:column;gap:4px;margin-top:10px;padding:10px;border:1px solid #1d6655;background:#0b2924;border-radius:6px;color:#9cebd3;font-size:9px}.connection-result.failed{border-color:#783948;background:#321b24;color:#ffb7c0}.connection-result span,.connection-result small{font-size:8px;line-height:1.5}.agent-report-block{border-top:1px solid var(--border);margin-top:14px;padding-top:13px}.agent-summary{font-size:10px;line-height:1.7;color:#dceafa;margin:0 0 10px}.report-meta{display:flex;gap:7px;flex-wrap:wrap;margin-bottom:10px}.report-meta span{background:#0d2135;border:1px solid #213951;border-radius:5px;padding:5px 7px;font-size:8px;color:#92abca}.observation-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px}.observation-item{min-width:0;padding:10px;border:1px solid #1b3048;border-radius:6px;background:#0a1929}.observation-item>div{display:flex;align-items:center;justify-content:space-between;gap:8px}.observation-item strong{color:#85c4ff;font-size:8px;letter-spacing:.5px}.observation-item span{font:7px var(--mono);color:#6e88a7;overflow-wrap:anywhere;text-align:right}.observation-item p{font-size:9px;line-height:1.6;color:#d0e0f2;margin:9px 0}.observation-item small{font-size:8px;line-height:1.55;color:#d4b983}.report-sublist{border-top:1px solid #1d3148;padding-top:9px;margin-top:10px}.report-sublist strong{font-size:9px;color:#93b5d9}.report-sublist p{font-size:8px;color:#8ca4c0;line-height:1.55;margin:5px 0}.report-digest{display:grid;grid-template-columns:110px minmax(0,1fr);gap:6px 9px;padding:9px;margin-top:10px;border-radius:6px;background:#071422;border:1px solid #192e45}.report-digest span{font-size:8px;color:#7189a6}.strict-verdict{margin-top:11px;padding:11px;border:1px solid #225d50;border-radius:7px;background:#0a221f}.strict-verdict.blocked{border-color:#66472a;background:#211b13}.strict-verdict p{font-size:8px;line-height:1.6;color:#9ab0c8;overflow-wrap:anywhere}.strict-verdict ul{padding-left:17px;color:#d5bd8b;font-size:8px;line-height:1.6}.authority-note{padding-top:8px;border-top:1px solid #294437;color:#74d9b7;font-size:8px;font-weight:800;letter-spacing:.4px}.agent-audit-footer{display:flex;align-items:center;justify-content:space-between;gap:12px;border-top:1px solid var(--border);margin-top:14px;padding-top:11px}.agent-audit-footer>div{display:grid;grid-template-columns:auto minmax(0,1fr);gap:4px 9px;min-width:0;flex:1}.agent-audit-footer strong{font-size:9px;color:#cbdcf0}.agent-audit-footer span{font-size:8px;color:#829ab7}.agent-audit-footer code{grid-column:1/-1}.status-tag.status-bad{border-color:#783948;background:#321b24;color:#ffb5c0}
  @media(max-width:920px){.runtime-grid{grid-template-columns:1fr}}@media(max-width:600px){.runtime-head{flex-direction:column}.runtime-head>button{width:100%}.observation-grid{grid-template-columns:1fr}.agent-audit-footer{align-items:flex-start;flex-direction:column}.agent-audit-footer>button{width:100%}.runtime-actions>*{flex:1}.model-row{align-items:flex-start;flex-direction:column}.report-digest{grid-template-columns:1fr;gap:3px}.report-digest code{margin-bottom:6px}}
</style>
