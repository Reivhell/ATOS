///<reference types="svelte" />
;
import { onMount } from 'svelte';
import { AlertTriangle, Activity, CheckCircle2, Database, ExternalLink, LockKeyhole, RefreshCw, Search, ShieldAlert, ShieldCheck, Trash2 } from 'lucide-svelte';
import {
    getLatestMemeCoinHunterReport, getMemeCoinHunterSecurityStatus, removeMemeCoinHunterSecurityToken,
    runMemeCoinHunter, saveMemeCoinHunterSecurityToken
  } from '../adapters/tauri/bridge';
import type { MemeCoinHunterCandidate, MemeCoinHunterReport, MemeCoinHunterSecurityStatus } from '../types';
function $$render() {

  
  
  
  

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

  let  candidates = __sveltets_2_invalidate(() => [...(report?.candidates ?? [])]
    .filter((candidate) => `${candidate.symbol} ${candidate.name} ${candidate.chain} ${candidate.tokenAddress}`.toLowerCase().includes(query.toLowerCase()))
    .sort((left, right) => sortBy === 'risk' ? left.riskScore - right.riskScore
      : sortBy === 'liquidity' ? right.liquidityUsd - left.liquidityUsd
        : right.scalpScore - left.scalpScore));

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
;
async () => {

 { svelteHTML.createElement("section", { "class":`hunter-heading`,});
   { svelteHTML.createElement("div", {});
     { svelteHTML.createElement("div", { "class":`eyebrow`,});     }
     { svelteHTML.createElement("h1", {});   }
     { svelteHTML.createElement("p", {});                }
   }
   { svelteHTML.createElement("div", { "class":`heading-actions`,});
     { svelteHTML.createElement("span", { "class":`authority-pill`,});  { const $$_elohyeKkcoL3C = __sveltets_2_ensureComponent(LockKeyhole); new $$_elohyeKkcoL3C({ target: __sveltets_2_any(), props: { "size":13,}});}   }
     { svelteHTML.createElement("button", {       "class":`primary-button`,"type":`button`,"onclick":scan,"disabled":loading,});
        { const $$_wChserfeR3C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR3C({ target: __sveltets_2_any(), props: {   "size":14,"class":loading ? 'spin' : '',}});}loading ? 'Scanning…' : 'Run hunter scan';
     }
   }
 }

 { svelteHTML.createElement("section", { "class":`hunter-grid`,});
   { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});report?.discoveredCount ?? '—'; } { svelteHTML.createElement("small", {});   } }
   { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});report?.candidateCount ?? '—'; } { svelteHTML.createElement("small", {});    } }
   { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});report?.scalpSetupCount ?? '—'; } { svelteHTML.createElement("small", {});    } }
   { svelteHTML.createElement("article", { "class":`metric-card`,}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});report?.blockedCount ?? '—'; } { svelteHTML.createElement("small", {});      } }
 }

if(errorMessage){ { svelteHTML.createElement("div", {   "class":`message error`,"role":`alert`,});  { const $$_elgnairTtrelA1C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA1C({ target: __sveltets_2_any(), props: { "size":15,}});}errorMessage; }}
if(noticeMessage){ { svelteHTML.createElement("div", {   "class":`message notice`,"role":`status`,});  { const $$_2elcriCkcehC1C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC1C({ target: __sveltets_2_any(), props: { "size":15,}});}noticeMessage; }}

 { svelteHTML.createElement("section", { "class":`panel source-panel`,});
   { svelteHTML.createElement("div", { "class":`section-header`,});
     { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});    } { svelteHTML.createElement("p", {});report?.providerState ?? 'NOT SCANNED';  report ? `Last scan ${ageLabel(report.scannedAtMs)}` : 'No scan report stored'; } }
    if(report){ { svelteHTML.createElement("code", {});report.datasetFingerprintSha256.slice(0, 20);  }}
   }
   { svelteHTML.createElement("div", { "class":`source-grid`,});
       for(let source of __sveltets_2_ensureArray(report?.sources ?? [])){source.provider;
       { svelteHTML.createElement("article", { "class":`source-entry`,});
         { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});source.provider; } { svelteHTML.createElement("span", {  });source.state === 'connected';source.state !== 'connected';source.state.toUpperCase(); } }
         { svelteHTML.createElement("small", {});source.discoveredCount;   ageLabel(source.receivedAtMs); }
         { svelteHTML.createElement("p", {});source.detail; }
       }
    }
    if(!report){ { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_esabataD3C = __sveltets_2_ensureComponent(Database); new $$_esabataD3C({ target: __sveltets_2_any(), props: { "size":20,}});}              }}
   }
 }

 { svelteHTML.createElement("section", { "class":`panel security-panel`,});
   { svelteHTML.createElement("div", { "class":`section-header`,});
     { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});           } }
     { svelteHTML.createElement("span", {    "class":`security-state`,});securityStatus?.credentialConfigured;!securityStatus?.credentialConfigured;securityStatus?.credentialConfigured ? 'CREDENTIAL STORED' : 'NOT CONFIGURED'; }
   }
   { svelteHTML.createElement("div", { "class":`security-row`,});
      { const $$_kcehCdleihS2C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS2C({ target: __sveltets_2_any(), props: { "size":18,}});}
     { svelteHTML.createElement("div", {}); { svelteHTML.createElement("strong", {});securityStatus?.secureStoreAvailable ? 'Operating-system credential vault available' : 'Credential vault unavailable'; } { svelteHTML.createElement("p", {});securityStatus?.note ?? 'Security credential status has not been loaded.'; } }
   }
   { svelteHTML.createElement("form", {   "class":`credential-form`,"onsubmit":saveCredential,});
     { svelteHTML.createElement("label", {});   
        { svelteHTML.createElement("input", {               "type":`password`,"bind:value":securityToken,"minlength":32,"maxlength":4096,"autocomplete":`new-password`,"spellcheck":`false`,"placeholder":`Paste GoPlus access token`,"disabled":credentialBusy || !securityStatus?.secureStoreAvailable,});/*Ωignore_startΩ*/() => securityToken = __sveltets_2_any(null);/*Ωignore_endΩ*/}
     }
     { svelteHTML.createElement("button", {   "type":`submit`,"disabled":credentialBusy || securityToken.length < 32 || !securityStatus?.secureStoreAvailable,});  }
     { svelteHTML.createElement("button", {       "class":`secondary-button`,"type":`button`,"onclick":removeCredential,"disabled":credentialBusy || !securityStatus?.credentialConfigured,});  { const $$_2hsarT3C = __sveltets_2_ensureComponent(Trash2); new $$_2hsarT3C({ target: __sveltets_2_any(), props: { "size":13,}});}  }
   }
   { svelteHTML.createElement("small", { "class":`credential-note`,});                        }
 }

 { svelteHTML.createElement("section", { "class":`panel candidates-panel`,});
   { svelteHTML.createElement("div", { "class":`section-header candidate-header`,});
     { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});   } { svelteHTML.createElement("p", {});              } }
     { svelteHTML.createElement("div", { "class":`table-controls`,});
       { svelteHTML.createElement("label", {});  { const $$_hcraeS4C = __sveltets_2_ensureComponent(Search); new $$_hcraeS4C({ target: __sveltets_2_any(), props: { "size":14,}});}  { svelteHTML.createElement("input", {   "bind:value":query,"placeholder":`Search token / chain / address`,});/*Ωignore_startΩ*/() => query = __sveltets_2_any(null);/*Ωignore_endΩ*/} }
       { svelteHTML.createElement("select", {   "bind:value":sortBy,"aria-label":`Sort candidates`,});/*Ωignore_startΩ*/() => sortBy = __sveltets_2_any(null);/*Ωignore_endΩ*/ { svelteHTML.createElement("option", { "value":`scalp`,});  } { svelteHTML.createElement("option", { "value":`risk`,});  } { svelteHTML.createElement("option", { "value":`liquidity`,});  } }
     }
   }
   { svelteHTML.createElement("div", { "class":`table-scroll`,});
     { svelteHTML.createElement("table", {});
       { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {}); } } }
       { svelteHTML.createElement("tbody", {});
           for(let candidate of __sveltets_2_ensureArray(candidates)){candidate.chain + ':' + candidate.tokenAddress;
           { svelteHTML.createElement("tr", { });selected?.tokenAddress === candidate.tokenAddress && selected?.chain === candidate.chain;
             { svelteHTML.createElement("td", {}); { svelteHTML.createElement("div", { "class":`asset`,}); { svelteHTML.createElement("strong", {});candidate.symbol; } { svelteHTML.createElement("small", {});candidate.chain.toUpperCase();  candidate.name; } } { svelteHTML.createElement("code", {});addressLabel(candidate.tokenAddress); } }
             { svelteHTML.createElement("td", {  });(candidate.priceChange5mPct ?? 0) >= 0;(candidate.priceChange5mPct ?? 0) < 0;percent(candidate.priceChange5mPct); }
             { svelteHTML.createElement("td", {});money(candidate.volume5mUsd, true); }
             { svelteHTML.createElement("td", {});candidate.buyPressure5mPct == null ? '—' : `${candidate.buyPressure5mPct.toFixed(1)}%`; }
             { svelteHTML.createElement("td", {});candidate.volumeVelocity5m == null ? '—' : `${candidate.volumeVelocity5m.toFixed(2)}x`; }
             { svelteHTML.createElement("td", {});money(candidate.liquidityUsd, true); }
             { svelteHTML.createElement("td", {});candidate.poolAgeMinutes == null ? '—' : `${Math.floor(candidate.poolAgeMinutes / 60)}h`; }
             { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", {     "class":`state-pill`,});candidate.securityState === 'verified';candidate.securityState === 'blocked' || candidate.securityState === 'unavailable';candidate.securityState === 'partial';candidate.securityState; } }
             { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", {   "class":`risk-number`,});candidate.riskScore >= 65;candidate.riskScore >= 35 && candidate.riskScore < 65;candidate.riskScore;  } }
             { svelteHTML.createElement("td", {}); { svelteHTML.createElement("span", {   "class":`setup-state`,});candidate.scalpState === 'SETUP';candidate.scalpState === 'BLOCKED';candidate.scalpState; } { svelteHTML.createElement("small", {});candidate.scalpScore;  } }
             { svelteHTML.createElement("td", {}); { svelteHTML.createElement("button", {     "class":`inspect-button`,"type":`button`,"onclick":() => selected = candidate,});  } }
           }
        }
        if(candidates.length === 0){ { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", { "colspan":11,}); { svelteHTML.createElement("div", { "class":`empty-state`,});  { const $$_esabataD7C = __sveltets_2_ensureComponent(Database); new $$_esabataD7C({ target: __sveltets_2_any(), props: { "size":19,}});}report ? 'No candidate matches the current filter.' : 'No scan observations are available.'; } } }}
       }
     }
   }
 }

if(selected){
   { svelteHTML.createElement("section", { "class":`panel detail-panel`,});
     { svelteHTML.createElement("div", { "class":`section-header`,}); { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h2", {});selected.symbol;    } { svelteHTML.createElement("p", {});selected.chain.toUpperCase();  selected.name;  addressLabel(selected.tokenAddress); } }if(selected.pairUrl){ { svelteHTML.createElement("a", {     "href":selected.pairUrl,"target":`_blank`,"rel":`noopener noreferrer`,});     { const $$_kniLlanretxE3C = __sveltets_2_ensureComponent(ExternalLink); new $$_kniLlanretxE3C({ target: __sveltets_2_any(), props: { "size":12,}});} }} }
     { svelteHTML.createElement("div", { "class":`detail-grid`,});
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});money(selected.priceUsd); } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});     } { svelteHTML.createElement("strong", {});percent(selected.priceChange5mPct);  percent(selected.priceChange1hPct);  percent(selected.priceChange24hPct); } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});    } { svelteHTML.createElement("strong", {});money(selected.volume1hUsd, true);  money(selected.volume24hUsd, true); } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});   } { svelteHTML.createElement("strong", {});selected.holderTop10Pct == null ? 'Unknown' : `${selected.holderTop10Pct.toFixed(1)}%`; } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});    } { svelteHTML.createElement("strong", {});selected.liquidityLockedTop10Pct == null ? 'Unknown' : `${selected.liquidityLockedTop10Pct.toFixed(1)}%`; } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});    } { svelteHTML.createElement("strong", {});selected.buyTaxPct == null ? 'Unknown' : `${selected.buyTaxPct.toFixed(2)}%`;  selected.sellTaxPct == null ? 'Unknown' : `${selected.sellTaxPct.toFixed(2)}%`; } }
     }
     { svelteHTML.createElement("div", { "class":`evidence-columns`,});
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h3", {});  { const $$_trelAdleihS4C = __sveltets_2_ensureComponent(ShieldAlert); new $$_trelAdleihS4C({ target: __sveltets_2_any(), props: { "size":14,}});}    }if(selected.securityFlags.length){ { svelteHTML.createElement("ul", {});  for(let flag of __sveltets_2_ensureArray(selected.securityFlags)){ { svelteHTML.createElement("li", {});flag; }} }}else{ { svelteHTML.createElement("p", {});          }} }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("h3", {});  { const $$_ytivitcA4C = __sveltets_2_ensureComponent(Activity); new $$_ytivitcA4C({ target: __sveltets_2_any(), props: { "size":14,}});}    }if(selected.scalpReasons.length){ { svelteHTML.createElement("ul", {});  for(let reason of __sveltets_2_ensureArray(selected.scalpReasons)){ { svelteHTML.createElement("li", {});reason; }} }}else{ { svelteHTML.createElement("p", {});      }} }
     }
     { svelteHTML.createElement("div", { "class":`provenance-note`,}); selected.discoverySources.join(' · ');     ageLabel(selected.receivedAtMs);    ageLabel(selected.securityCheckedAtMs);  }
   }
}

 { svelteHTML.createElement("section", { "class":`limitations-panel`,});
   { svelteHTML.createElement("div", {});  { const $$_elgnairTtrelA2C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA2C({ target: __sveltets_2_any(), props: { "size":16,}});} { svelteHTML.createElement("strong", {});  } }
   { svelteHTML.createElement("ul", {});  for(let limitation of __sveltets_2_ensureArray((report?.decisionLimitations ?? [
    'Market data has not yet been loaded.',
    'A scalp setup is not an execution approval or guarantee of return.',
    'Route quote, transaction simulation, price impact, gas and wallet risk checks are not performed by this scanner.'
  ]))){ { svelteHTML.createElement("li", {});limitation; }} }
   { svelteHTML.createElement("p", {});                         }
 }


};
return { props: {} as Record<string, never>, exports: {}, bindings: "", slots: {}, events: {} }}
const MemeCoinHunterPanel__SvelteComponent_ = __sveltets_2_isomorphic_component(__sveltets_2_with_any_event($$render()));
/*Ωignore_startΩ*/type MemeCoinHunterPanel__SvelteComponent_ = InstanceType<typeof MemeCoinHunterPanel__SvelteComponent_>;
/*Ωignore_endΩ*/export default MemeCoinHunterPanel__SvelteComponent_;