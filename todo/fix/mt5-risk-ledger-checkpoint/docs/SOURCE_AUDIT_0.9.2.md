# Source Audit — Agentic Trading OS v0.9.2

Date: 2026-10-29

## Scope

A static, file-by-file scan covered 90 logic/test source files and 18,168 lines in `src/`, `api/src/`, `src-tauri/src/`, and `scripts/`.

| Extension | Files | Lines |
|---|---:|---:|
| `.js` | 35 | 3,693 |
| `.mjs` | 39 | 4,419 |
| `.rs` | 8 | 8,180 |
| `.svelte` | 2 | 1,122 |
| `.ts` | 6 | 754 |
| **Total** | **90** | **18,168** |

## Checks performed and results

- `node --check` against every `.js` and `.mjs` in the scanned inventory: **0 failures**.
- Explicit dynamic execution and HTML sink patterns (`eval`, `new Function`, assignment to `innerHTML` / `outerHTML`, `dangerouslySetInnerHTML`, `document.write`): **0 matches** in production source.
- Production `TODO`, `FIXME`, `XXX`, or `HACK` markers: **0 matches**.
- The searched secret-adjacent logging patterns for API key / authorization / secret / password / token values: **0 matches**.
- Broad mock/demo-related source scan: two text-only matches, neither a demo-data path:
  - `src/App.svelte` explicitly states that synthetic quotes, candles, token rows and audit events are *not* supplied.
  - `src-tauri/src/backtest.rs` warns against interpreting an explicitly synthetic-labelled input dataset as a real strategy result. This is a validation warning, not a bundled synthetic market feed.
- Targeted review covered startup restoration, provider/screener liveness invalidation, kill-switch lifecycle, shared Rust freshness predicates, order and portfolio provenance, fixed-origin HTTP redirect policy, and frontend quote/screener freshness display.
- Full Node suite: **237/237 passed**.
- Strict layer typecheck: passed.
- AI Agent release evaluation: **10/10 passed**, policy passed, zero category gaps; provider-live integration remains untested.
- Repository guardrails: passed, with missing dependency lockfiles reported as warnings/blockers.

## Root-cause changes reviewed

1. Persisted provider `CONNECTED` and quote `is_live` claims no longer survive process restart as current evidence.
2. Every process restart re-engages the kill switch; fresh data refresh by itself cannot release it.
3. Future, zero, invalid, or older-than-120-second provider/quote observations fail freshness predicates.
4. Persisted screener rows are not presented as current unless the corresponding provider snapshot is freshly connected.
5. Fixed public market-data HTTP clients reject redirects.
6. The UI explicitly labels invalid or future timestamps instead of presenting them as recent observations.

## Important limits

This is a source-level static audit, not a formal proof or a complete dynamic security assessment. Rust tests/compilation, Svelte/Vite compilation, Bun/Hono integration, OS credential-store behavior, live provider transport tests, dependency vulnerability scanning, and cross-platform packaging were not run: the needed toolchains/dependencies and lockfiles are absent. No Rust test result is inferred from the Node contract tests. The source workspace was extracted from an archive rather than a Git checkout, so a Git diff check was unavailable.
