///<reference types="svelte" />
;
import { onMount } from 'svelte';
import { invoke } from '@tauri-apps/api/core';
import { confirm, open } from '@tauri-apps/plugin-dialog';
import { evaluateAgentResearchReport } from '../application/ai-agent-orchestrator.js';
function $$render() {

  
  
  
  

   let desktop = false/*Ωignore_startΩ*/;desktop = __sveltets_2_any(desktop);/*Ωignore_endΩ*/;
   let symbol = 'BTCUSDT';
   let timeframe = '5m';
   let marketReady = false/*Ωignore_startΩ*/;marketReady = __sveltets_2_any(marketReady);/*Ωignore_endΩ*/;

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

  let  selectedProviderStatus = __sveltets_2_invalidate(() => runtimeStatus?.providers.find((provider) => provider.id === selectedProvider) ?? null);

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
;
async () => {

 { svelteHTML.createElement("section", { "class":`agent-runtime panel`,});
   { svelteHTML.createElement("div", { "class":`runtime-head`,});
     { svelteHTML.createElement("div", {});
       { svelteHTML.createElement("div", { "class":`eyebrow`,});       }
       { svelteHTML.createElement("h2", {});   }
       { svelteHTML.createElement("p", {});                    }
     }
     { svelteHTML.createElement("button", {     "class":`button-secondary compact`,"onclick":refreshRuntime,"disabled":!desktop || busyAction !== '',});  }
   }

  if(!desktop){
     { svelteHTML.createElement("div", { "class":`runtime-warning`,}); { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});                 } }
  }else{
     { svelteHTML.createElement("div", { "class":`runtime-grid`,});
       { svelteHTML.createElement("section", { "class":`runtime-block`,});
         { svelteHTML.createElement("div", { "class":`runtime-block-title`,}); { svelteHTML.createElement("h3", {});   } { svelteHTML.createElement("span", {   "class":`status-tag`,});selectedProviderStatus?.credentialConfigured || selectedProvider === 'ollama';!selectedProviderStatus?.credentialConfigured && selectedProvider !== 'ollama';selectedProviderStatus?.credentialConfigured ? 'KEY STORED' : selectedProvider === 'ollama' ? 'LOCAL' : 'NO KEY'; } }
         { svelteHTML.createElement("label", { "class":`runtime-field`,});
           { svelteHTML.createElement("select", {     "value":selectedProvider,"onchange":(event) => changeProvider(event.currentTarget.value),"disabled":busyAction !== '',});
             { svelteHTML.createElement("option", { "value":`openai`,});   }
             { svelteHTML.createElement("option", { "value":`anthropic`,});   }
             { svelteHTML.createElement("option", { "value":`openrouter`,});   }
             { svelteHTML.createElement("option", { "value":`ollama`,});    }
           }
         }
        if(selectedProviderStatus?.keyRequired){
           { svelteHTML.createElement("label", { "class":`runtime-field`,});   { svelteHTML.createElement("input", {              "type":`password`,"bind:value":apiKey,"autocomplete":`off`,"autocapitalize":`off`,"spellcheck":`false`,"placeholder":selectedProviderStatus?.credentialConfigured ? 'Saved securely · enter to rotate key' : 'Paste provider API key',"disabled":busyAction !== '',});/*Ωignore_startΩ*/() => apiKey = __sveltets_2_any(null);/*Ωignore_endΩ*/} }
           { svelteHTML.createElement("div", { "class":`runtime-actions`,});
             { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":saveKey,"disabled":busyAction !== '' || !apiKey.trim() || selectedProviderStatus?.secureStoreAvailable !== true,});   }
             { svelteHTML.createElement("button", {     "class":`button-secondary`,"onclick":removeKey,"disabled":busyAction !== '' || !selectedProviderStatus?.credentialConfigured,});  }
           }
          if(selectedProviderStatus && !selectedProviderStatus.secureStoreAvailable){ { svelteHTML.createElement("p", { "class":`runtime-error-inline`,});             }}
        }else{
           { svelteHTML.createElement("div", { "class":`runtime-local-note`,});                        }
        }
         { svelteHTML.createElement("label", { "class":`runtime-field`,});  
           { svelteHTML.createElement("input", {          "bind:value":modelName,"autocomplete":`off`,"spellcheck":`false`,"placeholder":selectedProvider === 'ollama' ? 'e.g. name of an installed Ollama model' : 'Enter exact model identifier',"disabled":busyAction !== '',});/*Ωignore_startΩ*/() => modelName = __sveltets_2_any(null);/*Ωignore_endΩ*/}
         }
         { svelteHTML.createElement("div", { "class":`runtime-actions`,});
           { svelteHTML.createElement("button", {     "class":`button-secondary`,"onclick":testProvider,"disabled":busyAction !== '' || (selectedProviderStatus?.keyRequired === true && selectedProviderStatus?.credentialConfigured !== true),});busyAction === 'test-provider' ? 'Testing…' : 'Test provider'; }
           { svelteHTML.createElement("button", {     "class":`button-primary`,"onclick":runAnalyst,"disabled":busyAction !== '' || !modelName.trim() || !marketReady || !selectedProviderStatus || (selectedProviderStatus.keyRequired && (selectedProviderStatus.credentialConfigured !== true || selectedProviderStatus.secureStoreAvailable !== true)) || (runtimeStatus?.agentRunBudget.hourlyUsed ?? 0) >= (runtimeStatus?.agentRunBudget.hourlyLimit ?? 0) || (runtimeStatus?.agentRunBudget.dailyUsed ?? 0) >= (runtimeStatus?.agentRunBudget.dailyLimit ?? 0),});busyAction === 'run-agent' ? 'Analyzing…' : 'Run research analyst'; }
         }
         { svelteHTML.createElement("p", { "class":`runtime-note`,});   { svelteHTML.createElement("strong", {});symbol;  timeframe; }                 }
       }

       { svelteHTML.createElement("section", { "class":`runtime-block model-registry-block`,});
         { svelteHTML.createElement("div", { "class":`runtime-block-title`,}); { svelteHTML.createElement("h3", {});    } { svelteHTML.createElement("span", { "class":`status-tag status-idle`,});   } }
         { svelteHTML.createElement("p", { "class":`runtime-copy`,});                                }
         { svelteHTML.createElement("label", { "class":`runtime-field`,});   
           { svelteHTML.createElement("input", {              "bind:value":expectedModelSha256,"inputmode":`text`,"autocomplete":`off`,"autocapitalize":`off`,"spellcheck":`false`,"placeholder":`64-character trusted digest`,"disabled":busyAction !== '',});/*Ωignore_startΩ*/() => expectedModelSha256 = __sveltets_2_any(null);/*Ωignore_endΩ*/}
         }
         { svelteHTML.createElement("p", { "class":`runtime-note`,});                }
         { svelteHTML.createElement("button", {     "class":`button-secondary`,"onclick":importModel,"disabled":busyAction !== '',});busyAction === 'import-model' ? 'Importing…' : 'Import model file'; }
        if(runtimeStatus?.importedModels?.length){
           { svelteHTML.createElement("div", { "class":`model-list`,});
              for(let model of __sveltets_2_ensureArray(runtimeStatus.importedModels)){
               { svelteHTML.createElement("article", { "class":`model-row`,});
                 { svelteHTML.createElement("div", { "class":`model-row-main`,}); { svelteHTML.createElement("strong", {});model.filename; } { svelteHTML.createElement("span", {});model.format;  formatBytes(model.sizeBytes);  new Date(model.importedAt).toLocaleString(); } { svelteHTML.createElement("code", {});model.sha256; } { svelteHTML.createElement("small", {});model.executionStatus.replaceAll('_', ' '); } }
                 { svelteHTML.createElement("div", { "class":`model-actions`,}); { svelteHTML.createElement("button", {     "class":`button-secondary compact`,"onclick":() => verifyModel(model),"disabled":busyAction !== '',});busyAction === `verify:${model.id}` ? 'Verifying…' : 'Verify'; } { svelteHTML.createElement("button", {     "class":`button-secondary compact model-remove`,"onclick":() => removeModel(model),"disabled":busyAction !== '',});busyAction === `remove:${model.id}` ? 'Removing…' : 'Remove'; } }
               }
            }
           }
        }else{
           { svelteHTML.createElement("div", { "class":`runtime-empty`,});     }
        }
         { svelteHTML.createElement("p", { "class":`runtime-note`,});                           }
       }
     }
  }

  if(runtimeStatus?.agentRunBudget){
     { svelteHTML.createElement("div", { "class":`runtime-budget`,}); { svelteHTML.createElement("strong", {});     } { svelteHTML.createElement("span", {});runtimeStatus.agentRunBudget.hourlyUsed; runtimeStatus.agentRunBudget.hourlyLimit;     runtimeStatus.agentRunBudget.dailyUsed; runtimeStatus.agentRunBudget.dailyLimit;    } { svelteHTML.createElement("small", {});                         } }
  }
  if(errorText){ { svelteHTML.createElement("div", {   "class":`runtime-result error`,"role":`alert`,});errorText; }}
  if(statusText){ { svelteHTML.createElement("div", {   "class":`runtime-result`,"role":`status`,});statusText; }}
  if(connectionResult){ { svelteHTML.createElement("div", {  "class":`connection-result`,});!connectionResult.connected; { svelteHTML.createElement("strong", {});connectionResult.connected ? 'Provider reachable' : 'Provider unavailable'; } { svelteHTML.createElement("span", {});connectionResult.message; } { svelteHTML.createElement("small", {});connectionResult.checkedAt;if(connectionResult.modelCount !== null && connectionResult.modelCount !== undefined){ connectionResult.modelCount;   } } }}

  if(researchReport){
     { svelteHTML.createElement("section", { "class":`agent-report-block`,});
       { svelteHTML.createElement("div", { "class":`runtime-block-title`,}); { svelteHTML.createElement("h3", {});   } { svelteHTML.createElement("span", { "class":`status-tag status-idle`,});  } }
       { svelteHTML.createElement("p", { "class":`agent-summary`,});researchReport.summary; }
       { svelteHTML.createElement("div", { "class":`report-meta`,}); { svelteHTML.createElement("span", {}); researchReport.providerId;  researchReport.model; } { svelteHTML.createElement("span", {}); researchReport.marketContext.symbol; researchReport.marketContext.timeframe; } { svelteHTML.createElement("span", {});  researchReport.marketContext.asOf; } { svelteHTML.createElement("span", {});  new Date(researchReport.marketContext.retrievedAtMs).toLocaleString(); } { svelteHTML.createElement("span", {}); researchReport.marketContext.isFresh ? 'FRESH' : 'STALE / UNVERIFIED'; } }
       { svelteHTML.createElement("div", { "class":`observation-grid`,});
          for(let observation of __sveltets_2_ensureArray(researchReport.observations)){
           { svelteHTML.createElement("article", { "class":`observation-item`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});observation.domain.toUpperCase(); } { svelteHTML.createElement("span", {});observation.evidenceRefs.join(', '); } } { svelteHTML.createElement("p", {});observation.assessment; }if(observation.caveats.length){ { svelteHTML.createElement("small", {});observation.caveats.join(' · '); }} }
        }
       }
      if(researchReport.disagreements.length){ { svelteHTML.createElement("div", { "class":`report-sublist`,}); { svelteHTML.createElement("strong", {});  }  for(let issue of __sveltets_2_ensureArray(researchReport.disagreements)){ { svelteHTML.createElement("p", {});issue; }} }}
      if(researchReport.followUpChecks.length){ { svelteHTML.createElement("div", { "class":`report-sublist`,}); { svelteHTML.createElement("strong", {});  }  for(let check of __sveltets_2_ensureArray(researchReport.followUpChecks)){ { svelteHTML.createElement("p", {});check; }} }}
       { svelteHTML.createElement("div", { "class":`report-digest`,}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("code", {});researchReport.marketContext.datasetSha256; } { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("code", {});researchReport.outputSha256; } { svelteHTML.createElement("span", {});   } { svelteHTML.createElement("code", {});researchReport.evidenceEnvelopeSha256; } { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("code", {});researchReport.runId; } }
      if(strictError){ { svelteHTML.createElement("div", { "class":`runtime-result error`,});     strictError; }} else if (strictResult){
         { svelteHTML.createElement("div", {  "class":`strict-verdict`,});strictResult.decision.decision === 'BLOCK';
           { svelteHTML.createElement("div", { "class":`runtime-block-title`,}); { svelteHTML.createElement("h3", {});    } { svelteHTML.createElement("span", {   "class":`status-tag`,});strictResult.decision.decision !== 'BLOCK';strictResult.decision.decision === 'BLOCK';strictResult.decision.decision.replaceAll('_', ' '); } }
           { svelteHTML.createElement("p", {});   { svelteHTML.createElement("strong", {});    }            strictResult.registryRecordCount;   strictResult.registrySha256; }
          if(strictResult.decision.reasons.length){ { svelteHTML.createElement("ul", {});  for(let reason of __sveltets_2_ensureArray(strictResult.decision.reasons)){ { svelteHTML.createElement("li", {});reason; }} }}
           { svelteHTML.createElement("div", { "class":`authority-note`,});            }
         }
      }
       { svelteHTML.createElement("p", { "class":`runtime-note`,});                    }
     }
  }

   { svelteHTML.createElement("section", { "class":`agent-audit-footer`,});
     { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});   } { svelteHTML.createElement("span", {});runtimeStatus?.audit?.eventCount ?? 0;   runtimeStatus?.audit?.valid ? 'chain verified' : 'verification required'; } { svelteHTML.createElement("code", {});runtimeStatus?.audit?.headSha256 ?? 'Not loaded'; } }
     { svelteHTML.createElement("button", {     "class":`button-secondary compact`,"onclick":verifyAuditChain,"disabled":!desktop || busyAction !== '',});busyAction === 'verify-audit' ? 'Verifying…' : 'Verify audit chain'; }
   }
 }


};
return { props: {desktop: desktop , symbol: symbol , timeframe: timeframe , marketReady: marketReady} as {desktop?: typeof desktop, symbol?: typeof symbol, timeframe?: typeof timeframe, marketReady?: typeof marketReady}, exports: {}, bindings: "", slots: {}, events: {} }}
const AgentRuntimePanel__SvelteComponent_ = __sveltets_2_isomorphic_component(__sveltets_2_with_any_event($$render()));
/*Ωignore_startΩ*/type AgentRuntimePanel__SvelteComponent_ = InstanceType<typeof AgentRuntimePanel__SvelteComponent_>;
/*Ωignore_endΩ*/export default AgentRuntimePanel__SvelteComponent_;