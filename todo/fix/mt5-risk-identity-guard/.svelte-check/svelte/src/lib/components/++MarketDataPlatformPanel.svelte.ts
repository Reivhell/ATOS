///<reference types="svelte" />
;
import { onMount } from 'svelte';
import { AlertTriangle, CheckCircle2, Clock3, Database, RefreshCw, ShieldCheck } from 'lucide-svelte';
import { getMarketDataProviderCatalog, queryUnifiedMarketData } from '../adapters/tauri/bridge';
import type { MarketDataProviderCatalogEntry, MarketDataProviderState, MarketDataPurpose, UnifiedMarketDataResult } from '../types';
function $$render() {

  
  
  
  

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
;
async () => {

 { svelteHTML.createElement("section", { "class":`registry-panel panel`,});
   { svelteHTML.createElement("div", { "class":`panel-heading`,});
     { svelteHTML.createElement("div", {});
       { svelteHTML.createElement("h2", {});   }
       { svelteHTML.createElement("p", {});         }
     }
     { svelteHTML.createElement("button", {         "class":`refresh-button`,"type":`button`,"onclick":loadProviders,"disabled":providersBusy,"aria-label":`Refresh provider registry`,});
        { const $$_wChserfeR3C = __sveltets_2_ensureComponent(RefreshCw); new $$_wChserfeR3C({ target: __sveltets_2_any(), props: {   "size":14,"class":providersBusy ? 'spin' : '',}});}
      
     }
   }
  if(providers.length === 0 && providersBusy){
     { svelteHTML.createElement("div", { "class":`registry-empty`,});  { const $$_esabataD2C = __sveltets_2_ensureComponent(Database); new $$_esabataD2C({ target: __sveltets_2_any(), props: { "size":19,}});}     }
  } else if (providers.length === 0){
     { svelteHTML.createElement("div", { "class":`registry-empty`,});  { const $$_elgnairTtrelA2C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA2C({ target: __sveltets_2_any(), props: { "size":18,}});}          }
  }else{
     { svelteHTML.createElement("div", { "class":`provider-grid`,});
         for(let provider of __sveltets_2_ensureArray(providers)){provider.id;
         { svelteHTML.createElement("article", { "class":`provider-entry`,});
           { svelteHTML.createElement("div", { "class":`provider-entry-head`,});
             { svelteHTML.createElement("div", { "class":`provider-icon`,});  { const $$_esabataD5C = __sveltets_2_ensureComponent(Database); new $$_esabataD5C({ target: __sveltets_2_any(), props: { "size":17,}});} }
             { svelteHTML.createElement("span", { "class":`provider-state ${providerStateClass(provider.state)}`,});providerStates[provider.state]; }
           }
           { svelteHTML.createElement("h3", {});provider.displayName; }
           { svelteHTML.createElement("p", { "class":`venue`,});provider.venue;  provider.authority === 'brokerTerminal' ? 'Broker-authoritative' : 'Public exchange'; }
           { svelteHTML.createElement("div", { "class":`capabilities`,});
              for(let capability of __sveltets_2_ensureArray(provider.capabilities)){ { svelteHTML.createElement("span", {});capability; }}
           }
           { svelteHTML.createElement("ul", {});
              for(let limitation of __sveltets_2_ensureArray(provider.limitations)){ { svelteHTML.createElement("li", {});limitation; }}
           }
           { svelteHTML.createElement("small", {});provider.observedAtMs ? `Last observation ${new Date(provider.observedAtMs).toLocaleString()}` : 'No provider observation timestamp'; }
         }
      }
     }
  }
 }

 { svelteHTML.createElement("section", { "class":`query-panel panel`,});
   { svelteHTML.createElement("div", { "class":`panel-heading`,});
     { svelteHTML.createElement("div", {});
       { svelteHTML.createElement("h2", {});   }
       { svelteHTML.createElement("p", {});                 }
     }
      { const $$_kcehCdleihS2C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS2C({ target: __sveltets_2_any(), props: { "size":18,}});}
   }
   { svelteHTML.createElement("form", {   "class":`query-form`,"onsubmit":runQuery,});
     { svelteHTML.createElement("label", {});
       { svelteHTML.createElement("select", {   "bind:value":selectedProviderId,"disabled":purpose === 'brokerContext' || providers.length === 0,});/*Ωignore_startΩ*/() => selectedProviderId = __sveltets_2_any(null);/*Ωignore_endΩ*/
          for(let provider of __sveltets_2_ensureArray(providers)){ { svelteHTML.createElement("option", { "value":provider.id,});provider.displayName; }}
       }
     }
     { svelteHTML.createElement("label", {});
        { svelteHTML.createElement("input", {       "bind:value":symbol,"maxlength":64,"required":true,"autocomplete":`off`,"spellcheck":`false`,});/*Ωignore_startΩ*/() => symbol = __sveltets_2_any(null);/*Ωignore_endΩ*/}
     }
     { svelteHTML.createElement("label", {});
       { svelteHTML.createElement("select", { "bind:value":timeframe,});/*Ωignore_startΩ*/() => timeframe = __sveltets_2_any(null);/*Ωignore_endΩ*/
         { svelteHTML.createElement("option", { "value":`M1`,});  } { svelteHTML.createElement("option", { "value":`M5`,});  } { svelteHTML.createElement("option", { "value":`M15`,});  }
         { svelteHTML.createElement("option", { "value":`M30`,});  } { svelteHTML.createElement("option", { "value":`H1`,});  } { svelteHTML.createElement("option", { "value":`H4`,});  }
         { svelteHTML.createElement("option", { "value":`D1`,});  } { svelteHTML.createElement("option", { "value":`W1`,});  }
       }
     }
     { svelteHTML.createElement("label", {});
       { svelteHTML.createElement("select", { "bind:value":purpose,});/*Ωignore_startΩ*/() => purpose = __sveltets_2_any(null);/*Ωignore_endΩ*/
         { svelteHTML.createElement("option", { "value":`research`,});  }
         { svelteHTML.createElement("option", { "value":`backtest`,});  }
         { svelteHTML.createElement("option", { "value":`brokerContext`,});     }
       }
     }
     { svelteHTML.createElement("label", {}); 
        { svelteHTML.createElement("input", {         "type":`number`,"min":`1`,"max":`5000`,"step":`1`,"bind:value":pageLimit,});/*Ωignore_startΩ*/() => pageLimit = __sveltets_2_any(null);/*Ωignore_endΩ*/}
     }
     { svelteHTML.createElement("button", {     "class":`query-button`,"type":`submit`,"disabled":queryBusy || !selectedProviderId || !symbol.trim(),});
        { const $$_esabataD3C = __sveltets_2_ensureComponent(Database); new $$_esabataD3C({ target: __sveltets_2_any(), props: { "size":14,}});}queryBusy ? 'Querying…' : 'Query last 7 days';
     }
   }
  if(purpose === 'brokerContext'){
     { svelteHTML.createElement("p", { "class":`policy-note`,});  { const $$_kcehCdleihS2C = __sveltets_2_ensureComponent(ShieldCheck); new $$_kcehCdleihS2C({ target: __sveltets_2_any(), props: { "size":14,}});}                   }
  }else{
     { svelteHTML.createElement("label", { "class":`stale-toggle`,});  { svelteHTML.createElement("input", {   "type":`checkbox`,"bind:checked":allowStale,});/*Ωignore_startΩ*/() => allowStale = __sveltets_2_any(null);/*Ωignore_endΩ*/}       }
     { svelteHTML.createElement("p", { "class":`policy-note`,});  { const $$_3kcolC2C = __sveltets_2_ensureComponent(Clock3); new $$_3kcolC2C({ target: __sveltets_2_any(), props: { "size":14,}});}                    }
  }
  if(errorMessage){
     { svelteHTML.createElement("div", {   "class":`query-error`,"role":`alert`,});  { const $$_elgnairTtrelA2C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA2C({ target: __sveltets_2_any(), props: { "size":15,}});}errorMessage; }
  }
  if(result){
     { svelteHTML.createElement("div", { "class":`result-summary`,});
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});result.provenance.providerName; } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", { });result.quality.freshness !== 'fresh';result.quality.freshness.toUpperCase();  ageLabel(result.quality.freshnessAgeMs); } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("strong", {});result.quality.candleCount;result.quality.hasMore ? '+' : ''; } }
       { svelteHTML.createElement("div", {}); { svelteHTML.createElement("span", {});   } { svelteHTML.createElement("strong", {});result.quality.revisionCount;  result.quality.intervalAnomalyCount; } }
     }
     { svelteHTML.createElement("div", { "class":`request-policy`,}); { svelteHTML.createElement("span", {}); result.request.purpose; } { svelteHTML.createElement("span", {});  result.request.allowStale ? 'enabled' : 'disabled'; } { svelteHTML.createElement("span", {});  ageLabel(result.request.maxDataAgeMs); } }
     { svelteHTML.createElement("div", { "class":`fingerprint`,}); { svelteHTML.createElement("span", {});  } { svelteHTML.createElement("code", {});result.provenance.datasetFingerprintSha256; } }
    if(result.quality.gapCandidates.length > 0){
       { svelteHTML.createElement("div", { "class":`gap-warning`,});  { const $$_elgnairTtrelA2C = __sveltets_2_ensureComponent(AlertTriangle); new $$_elgnairTtrelA2C({ target: __sveltets_2_any(), props: { "size":14,}});}result.quality.gapCandidates.length;    result.quality.continuityInterpretation; }
    }else{
       { svelteHTML.createElement("div", { "class":`continuity-note`,});  { const $$_2elcriCkcehC2C = __sveltets_2_ensureComponent(CheckCircle2); new $$_2elcriCkcehC2C({ target: __sveltets_2_any(), props: { "size":14,}});}result.quality.continuityInterpretation; }
    }
     { svelteHTML.createElement("div", { "class":`bar-table-wrap`,});
       { svelteHTML.createElement("table", {});
         { svelteHTML.createElement("thead", {}); { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } { svelteHTML.createElement("th", {});  } } }
         { svelteHTML.createElement("tbody", {});
             for(let bar of __sveltets_2_ensureArray([...result.bars].slice(-12).reverse())){bar.timeMs;
             { svelteHTML.createElement("tr", {}); { svelteHTML.createElement("td", {});new Date(bar.timeMs).toLocaleString(); } { svelteHTML.createElement("td", {});bar.open; } { svelteHTML.createElement("td", {});bar.high; } { svelteHTML.createElement("td", {});bar.low; } { svelteHTML.createElement("td", {});bar.close; } { svelteHTML.createElement("td", {});bar.volume; } { svelteHTML.createElement("td", {});volumeLabel(bar.volumeBasis); } }
          }
         }
       }
     }
     { svelteHTML.createElement("p", { "class":`source-disclosure`,}); result.provenance.venue;     result.provenance.latestObservationAtMs ? new Date(result.provenance.latestObservationAtMs).toLocaleString() : 'Unavailable';    result.provenance.latestReceivedAtMs ? new Date(result.provenance.latestReceivedAtMs).toLocaleString() : 'Unavailable';  result.provenance.sourceDetails;    result.provenance.scopeFingerprintSha256 ?? (result.provenance.authority === 'brokerTerminal' ? 'Authenticated scope verified internally' : 'Public dataset'); }
  }
 }


};
return { props: {} as Record<string, never>, exports: {}, bindings: "", slots: {}, events: {} }}
const MarketDataPlatformPanel__SvelteComponent_ = __sveltets_2_isomorphic_component(__sveltets_2_with_any_event($$render()));
/*Ωignore_startΩ*/type MarketDataPlatformPanel__SvelteComponent_ = InstanceType<typeof MarketDataPlatformPanel__SvelteComponent_>;
/*Ωignore_endΩ*/export default MarketDataPlatformPanel__SvelteComponent_;