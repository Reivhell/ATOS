# Changelog

## 0.11.0 — Rust-native unified market-data platform (2026-10-31)

- Added a Rust-native provider registry inspired by the OpenBB core/provider/standard-model architecture. The implementation is compiled into ATOS; it does not load downloaded provider code or copy OpenBB source files.
- Added a versioned canonical market-data contract with explicit authority, provider capability, purpose, pagination, volume basis, freshness, quality, provenance, and dataset fingerprint fields.
- Reworked installer staging around a held source file descriptor: validation and full-file digest are bound to the same object that is copied, source and package-directory replacement or mutation fail closed, and the staged descriptor is flushed, re-read, hashed, and revalidated before publication.
- Extended strict JavaScript contract typing to the work-plan and decision-gate modules, resolved the duplicate `Candle` barrel type, and included all `src/**/*.js` modules in project-level `checkJs` coverage.
- Connected the existing authenticated MT5 bridge and durable account-bound SQLite candle store to the normalized query surface. Broker-context requests are pinned to MT5, require a fresh connected terminal, and revalidate both sequence and account identity after the query.
- Connected the existing validated Binance BTCUSDT M5 cache for research/backtest only. Public data cannot satisfy broker context and unsupported symbols/timeframes fail without cross-source substitution.
- Anchored freshness to market-observation timestamps rather than cache-ingestion timestamps. Missing timestamps remain unavailable and cannot be overridden as merely stale. Research/backtest stale override is explicit and echoed into the request summary.
- Added stable query-range dataset fingerprints for public data, pagination-independent provenance, gap/revision reporting for MT5 history, and preservation of broker tick-volume versus real-volume semantics.
- Added native Tauri commands, typed Svelte bridge contracts, and a production Market Data panel for provider discovery, explicit source selection, request policy, observation/receipt timestamps, freshness, volume basis, and provenance.
- Added release preflight enforcement for the Bun runtime because Bun/Hono is a required backend runtime in this repository.
- Added eight native-market-data source-contract tests and unified market-observation freshness across quote/candle ingestion, cache, risk admission, provider status, and UI. Reception time remains provenance and cannot refresh stale market observations.
- Corrected MT5 active-order state handling using named MQL5 enum constants: partial state 3 remains in the complete pending inventory, while canceled/filled/rejected/expired terminal states are rejected. Added regression coverage for the full active-versus-terminal state set and for unprotected partial remainder risk.
- Hardened broker volume normalization so the result is capped in whole `SYMBOL_VOLUME_STEP` units and cannot exceed `SYMBOL_VOLUME_MAX` or the raw risk-derived amount when the reported maximum is not step-aligned.
- Closed a risk-sizing time-of-check window: the response now uses completion-time quote/specification ages and is blocked if the authenticated account frame advances, disconnects, or becomes stale after broker-native preflight.
- The current full Node regression passes 375/375 tests across 51 command groups; strict layer typecheck and repository guardrails pass; all 31 frontend JavaScript modules pass strict `tsc`, 92 JavaScript/MJS/CJS files and 13 TypeScript files pass syntax checks, and 18 JSON files parse successfully. The agent release evaluation passes 10/10 policy scenarios but does not exercise live provider integration.
- This is a substantial source milestone, not production certification. The full project typecheck is blocked by missing installed dependencies; Rust tests/build, Svelte/Vite/Tauri build, Bun runtime integration, MT5 MetaEditor compilation, and real broker acceptance remain unverified. Release preflight remains fail-closed with ten blockers, including the three absent dependency lockfiles and unavailable native/frontend toolchains.

## 0.10.1 — Nested walk-forward strategy tournament (2026-10-30)

- Added a Rust-native, expanding-window parameter tournament around explicit SMA anchor periods. Candidate configurations are selected from training-only scores and evaluated on the next unseen chronological test block.
- Added hard limits for candidate count, fold count, candle volume, minimum training trade count and estimated compute work; no automatic weakening of evidence requirements occurs when a dataset is too short.
- Persisted all candidate scores, selected parameter pairs, training/test ranges, OOS results, parameter stability, blockers and dataset provenance.
- Added run-card semantic verification for canonical candidate grids, eligibility, fold continuity, deterministic winner selection and independently recomputed aggregate OOS statistics.
- Added committee gate v2; incomplete folds, insufficient positive OOS ratio or excessive worst-fold drawdown block research promotion.
- Added Tauri/Svelte wiring and source-contract tests. The tournament is Rust-native, read-only on account state, and grants no paper/live execution authority.
- Pinned all eight external GitHub Actions to reviewed full 40-character upstream commit SHAs, including Node 24-compatible checkout, setup-node and upload-artifact lines; CI and `rust-toolchain.toml` now select Rust 1.99.1 with rustfmt and Clippy rather than moving `stable`. Release readiness enforces the exact reviewed attestation/SBOM action commits and compiler version.
- Changed ordinary CI dependency setup to `npm ci` and `bun install --frozen-lockfile`; non-release validation can no longer silently regenerate or drift dependency lockfiles.
- Full local regression: 269/269 Node tests across 41 command groups, strict layer typecheck, 85-file JavaScript syntax scan, repository guardrails and workflow YAML parse pass.
- Extended the MT5 protocol with server-issued per-symbol tick cursors and per-symbol/timeframe bar cursors sourced from durable SQLite state. EA reconnects now resume tick synchronization at the last committed broker timestamp and recover closed-bar backlogs oldest-first in bounded 64-bar chunks instead of silently jumping to the most recent 64 bars.
- Added bounded tick-history gap detection, exact closed-bar cursor probing before recovery, and periodic 60-second reconciliation of the latest three committed closed bars so recent broker corrections create auditable bar revisions. The EA advances all local cursors and its reconciliation timer only after the exact durable frame ACK.
- Added an explicit SQLite market-data schema version and fail-closed handling for databases created by a newer application version; added native Rust tests for cursor recovery, invalid subscription sets, and schema downgrade prevention.
- Bound every telemetry frame to the authenticated account login/server and symbol/timeframe allow-list; mismatched account identity, primary quote symbols, tick symbols, or bar series are rejected before durable commit. Durable status now reports storage availability rather than overstating it as full storage health, and corrupted negative counters fail visibly instead of being normalized to zero.
- Preserved the single-repository EA path at `MQL5/Experts/ATOS_Bridge.mq5`; the adapter remains read-only and no live order route or synthetic broker data was added.
- Current full Node regression: 289/289 subtests across 43 repository command groups; strict layer typecheck and repository guardrails pass. A source audit also confirms the 828-line MQL5 adapter has balanced lexical structure and no order-submission or position-mutation API calls. Release preflight remains blocked by the three missing lockfiles and unavailable Svelte/Vite/Tauri, Cargo, Rustfmt and Clippy executables. Rust unit tests, Svelte/Tauri production build, MetaEditor compilation, and real-broker MT5 recovery tests are not claimed as passed.
- Release remains blocked by three missing dependency lockfiles and six unavailable frontend/native build executables; Rust compilation/tests, Svelte/Vite/Tauri build, signed attestations and cross-platform acceptance remain unverified.


## 0.10.0 — Rust-native research orchestration and immutable run cards (2026-10-30)

- Added a dedicated Research Lab spanning hypothesis creation, revision-bound lifecycle, execution, run-card inspection, and startup/in-session integrity verification.
- Added a separate SQLite research workspace; it is deliberately outside the trading account state and the Rust order/risk gateway. No LLM output, research status, or run-card decision can create an order intent or grant execution authority.
- Added a validated DAG with deterministic data-integrity, native backtest, independent out-of-sample and moving-block bootstrap branches, drawdown review, and a final investment-committee gate. The current reviewers are deterministic Rust validators, not autonomous LLM agents; role names describe independent review responsibilities only.
- Added immutable evidence cards recording the dataset SHA-256, data source and period, exact strategy parameters, fee/slippage context, configured research thresholds, cost-stress scenarios, OOS folds, bootstrap seed and summary, reviewer findings, blockers, caveats, and content hash.
- Added an append-only SHA-256 event chain, exact SQL trigger verification, fail-closed database reopening, replay-based validation of mutable hypothesis lifecycle state, immutable hypothesis identity fields, transaction boundaries, optimistic revisions, bounded payloads, a 200-hypothesis cap, and a hard 100-run-card cap. The cap prevents a supposedly bounded workspace from accumulating unbounded immutable artifacts.
- Fixed the research database trigger verifier to compare normalized full trigger definitions rather than a case-sensitive SQL substring. Altered or weakened guards now prevent workspace use.
- Added an account-read path for research assumptions that does not call the dashboard snapshot method or cause incidental account valuation writes.
- Corrected statistical labelling and gating: the resampling routine is identified as moving-block bootstrap; return quantiles are no longer compared to a drawdown threshold. Observed and resampled drawdown are compared only with drawdown limits. Replaced the basic xorshift sampler with deterministic PCG-XSH-RR 64/32 and rejection-sampled indices to avoid modulo bias.
- Adopted the applicable Vibe-Trading research concepts (hypothesis-to-report workflow, inspectable run cards, DAG reviews, OOS/bootstrap validation). Multi-market loaders, broad alpha zoo, broker connectors, chat channels, schedulers, and shadow-account journal analysis are not copied into this release because the current ATOS market-data/account contracts do not safely support them.
- Added seven source-contract regression checks for native boundaries, data provenance, hash/immutability policy, DAG structure, Tauri wiring, hypothesis lifecycle and frontend evidence consumption.
- This is a substantial feature candidate, not a production signoff. Rust/frontend builds, native test execution, live integration, cross-platform packaging, and signed release attestations remain unverified in the current environment.

## 0.9.6 — Signed release provenance and strict installer staging (2026-10-30)

- Added a platform-aware release artifact staging boundary. Linux release jobs must produce DEB, RPM and AppImage installers; Windows must produce MSI and NSIS EXE installers; macOS must produce a DMG. Optional macOS PKG installers are included when present. Missing required formats, empty installers, symlink payloads, basename collisions, unsafe/overlapping directories, and output reuse block staging.
- Changed promotion to publish only staged installer files plus their inventory and SBOM; raw build intermediates and loose `.app` internals are no longer published as release payloads.
- Added SPDX SBOM generation with Syft and signed GitHub Artifact Attestations for both SLSA build provenance and the SBOM on every installer.
- Added consumer-side `release:attestation:verify`, enforcing the exact repository, release tag, CI workflow identity, GitHub OIDC issuer and prohibition on self-hosted runners. Every staged installer must pass two independent predicate checks: SLSA provenance and the SPDX 2.3 SBOM attestation, before CI uploads it.
- Fixed the release preflight so workflow contracts are tested against the workflow while policy flags are tested against their owning verifier module. Added an integration regression that executes the preflight to prevent false contract failures.
- Extended the fail-closed release preflight and repository guardrails to require installer staging, provenance/SBOM attestations, and strict verification wiring.
- Added a fail-closed release-action pin policy with regression tests: mutable action tags, branches, and abbreviated SHAs block release until all external actions are pinned to reviewed full commit SHAs. The current workspace cannot verify every full SHA offline, so this is an explicit blocker rather than guessed metadata.
- Added regression tests for platform package completeness, staged hash correctness, symlink/empty/collision rejection, provenance identity policy, unexpected payload files, both provenance/SBOM predicate checks per installer, and all-artifact fail-closed verification.
- This is CI/source implementation, not proof that a real signed release has been built. Artifact attestations require a supported GitHub repository/plan and the tagged workflow, frontend/Rust builds, committed lockfiles, and native acceptance gates remain required.


## 0.9.5 — Tamper-evident durable state commit chain (2026-10-29)

- Added a SHA-256 chained receipt for every accepted durable state commit, written atomically with the SQLite state payload and audit events.
- Verifies sequence continuity, previous-hash linkage, receipt hashes, and the latest receipt against the exact stored payload before accepting subsequent writes.
- Added append-only SQLite triggers for state-commit receipts and a pre-recovery startup verification so payload tampering cannot be silently treated as ordinary recoverable corruption once a chain exists. SQLite `user_version = 2` now marks chain activation; schema-v2 accounts fail closed if the receipt table or chain is missing.
- Existing schema-v1 databases without receipts are explicitly anchored on their next accepted commit with `LEGACY_BASELINE_UNVERIFIED`; historical integrity before that anchor is not claimed.
- Added a native regression test for receipt immutability and valid-JSON payload tampering. Rust tests remain unexecuted where the Rust toolchain is unavailable.

## 0.9.4 — Native state-commit decision layer and fail-closed account reset (2026-10-29)

- Added a Rust-native commit decision layer for every durable engine-state write. The gate reads the exact prior SQLite payload and evaluates the candidate inside the same write transaction before `app_state` can be updated.
- Classified the mutable state surface explicitly. Unknown changed fields fail closed; schema/account identity fields are immutable; market, screener, agent, risk-limit, order-ledger, and kill-switch transitions require matching fresh audit evidence.
- Made audit history append-only in two places: embedded history must remain ordered, immutable, and bounded; SQLite update/delete triggers protect the durable `audit_events` ledger.
- Bound paper fills atomically to one new order, unique non-empty risk checks, an execution audit event, cash change, and position-exposure change. Rejected orders require at least one failed risk check and a risk-gateway event.
- Validated derived equity peak, drawdown, UTC daily-baseline and quote-to-position marks against deterministic state equations, while distinguishing valuation updates from changes to position exposure.
- Fixed a real reset bug: `reset_paper_account` now always engages the kill switch, even if it had previously been released. A new regression test exercises the released-switch reset path.
- Kill-switch release now fails closed unless the change has explicit risk audit evidence and the provider snapshot and portfolio marks are fresh. This is a local paper-trading control only; live execution remains disabled.
- Added Rust unit tests plus Node source-contract tests. Rust compilation/tests and SQLite runtime trigger tests still require the native Rust toolchain, which is unavailable in this workspace; passing source checks are not reported as native test execution.

## 0.9.3 — Exclusive provider refresh leases (2026-10-29)

- Fixed a provider refresh race at its source: the previous timestamp throttle could permit a second market-data request after 3 seconds even while the first request was still running (request timeout up to 8 seconds). A late response could overwrite a newer observation.
- Added per-provider `AtomicBool` in-flight leases. Market-data and screener refreshes now reject overlapping calls for the full async request lifetime, while preserving their separate rate limits.
- Lease cleanup uses Rust `Drop`, so network/parse failures, early returns, and cancellation release the in-flight flag instead of permanently wedging refresh.
- Added native Rust tests for overlap rejection, throttle release behavior, and wall-clock rollback; added source-contract checks to the runnable Node suite.
- Node suite and layer typecheck pass in this environment. Native Rust tests are written but not executed because Cargo/rustfmt are unavailable; release preflight remains blocked by missing dependency lockfiles and build toolchains.
- No mock/demo data, broker execution, live order intent, or execution authority was added.

## 0.9.2 — Provider Session Lifecycle Hardening (2026-10-29)

- Fixed a restart-lifecycle defect: persisted `CONNECTED`/`isLive` metadata is invalidated at startup, including cached screener connection status; cached rows remain historical until a successful provider refresh in the new process.
- Unified native provider, quote, candle-series and portfolio-mark freshness checks around positive, non-future timestamps and a bounded age.
- Native market-data provenance now requires a recent connected provider observation, not just a stale persisted string prefix.
- Every process restart re-engages the paper-trading kill switch. A successful refresh alone cannot resume order entry; the operator must explicitly review controls and release it.
- The UI requires both safe-integer provider-snapshot and per-quote timestamps. Stale/disconnected screener rows remain in local history but are excluded from all current-evidence views.
- Disabled automatic redirects for fixed-origin public market-data/screener requests.
- Added Rust regression tests for restart revalidation, kill-switch re-arming, and future timestamp rejection, plus runnable source-contract checks for UI freshness and screener visibility.
- Release remains blocked until native/frontend build toolchains and dependency lockfiles are available and checked in CI.

## 0.9.1 — Native-only market boundary and truthful evidence presentation

- Removed the browser-generated dashboard snapshot path and required a real Tauri/Rust runtime for every dashboard, market-data, screener, audit, analysis, order and account operation. The browser displays an explicit unsupported-runtime state and no longer synthesizes quotes, candles, tokens, agent sessions or audit events.
- Replaced fresh/recovered engine state with an empty observation state, no preloaded agent/screener records, and an engaged kill switch. Startup migration removes previously persisted bundled/synthetic observations before the first snapshot and keeps controls fail-closed when cleanup changes state.
- Bound each native AI evidence payload to its SHA-256 and canonical JSON representation, then revalidated the exact market dataset and freshness after provider processing to close the provider-latency time-of-check/time-of-use window.
- Removed hardcoded dashboard equity/drawdown trend lines, removed zero-price fallback labels, aligned paper-order quote selection to the exact selected symbol, and disabled estimates/order controls without a fresh exact-symbol quote and a positive finite quantity.
- Risk-increasing paper orders now require positive, live, no-more-than-120-second marks for every open position; missing/stale marks cannot be replaced with saved notional to pass portfolio risk checks, while risk-reducing sells are not blocked solely by an unrelated unmarked position.
- Portfolio equity, day-change, unrealized PnL, current exposure and position mark cells are unavailable when any open position lacks a fresh live quote; stale chart quotes explicitly become unavailable after the freshness window.
- A provider refresh failure now blocks new risk-increasing orders even if the last quote is younger than 120 seconds. Cached quote age alone cannot override a disconnected/stale provider status; UI and Rust provenance checks enforce the same rule.
- Portfolio equity, day-change, unrealized PnL, current exposure and position mark cells are now marked unavailable if any open position lacks a fresh live quote; stale cached marks remain visible only as an explicit unavailable state, not a current valuation.
- Added an explicit closed-trade count to the dashboard contract so win rate displays as unavailable until a denominator exists. Replaced the misleading idle-agent “Running” label with the actual empty-session state and derived chart time labels from loaded candle timestamps.
- Added native-runtime boundary, no-fabricated-visual, empty-statistic, symbol-scope, and freshness contract regression checks to the canonical `npm test` sequence and layer typecheck.
- Unified UI quote validation behind one provider-aware rule (connected provider, live flag, finite positive price, non-future timestamp, and a 120-second freshness bound); stale quotes no longer enter top-performer rankings or appear as current in watchlist, order selector, or market-data metrics.
- Changed native paper-dashboard win rate to `null` until there is at least one closed trade. Backtest win rate is likewise nullable; gross profit/loss are now explicit so an actually unbounded profit factor is distinguished from an undefined metric when there are no gains/losses.
- This candidate remains not production-certified: frontend/native compilers, locked dependency resolution, actual provider/keychain acceptance, and cross-platform packaging are still required. No live venue execution or order intent is enabled.

## 0.9.0 — Integrated AI Agent release qualification and timeout containment

- Wired the reliability evaluator to the real JavaScript AI report boundary through a credential-free, deterministic release gate (`npm run eval:agents`).
- Added report scenarios for malformed provider identity, simulated provider timeout, authority escalation, evidence-envelope tampering, oversized commentary, repeat-run determinism, narrative-only metamorphism, and evidence-order permutation.
- The gate emits a machine-readable report with report and envelope SHA-256 digests, requires 100% pass rate and complete risk-category coverage, and is now part of CI; CI uploads its result as an artifact.
- Hardened evaluator deadline behavior: once a timeout occurs, the current repeat sequence stops and all later scenarios are blocked rather than continuing to launch potentially uncooperative adapter promises.
- Decision-equivalence fingerprints normalize object keys and order-independent output arrays, so evidence ordering changes do not masquerade as decision drift.
- No live provider, broker, order intent, or trading authority was introduced. The offline quality gate is not a substitute for native/frontend build, real transport integration, or production acceptance.

## 0.8.9 — AI agent reliability and adversarial evaluation framework

- Added a bounded, provider-agnostic reliability evaluation contract for AI Agent adapters.
- Added strict canonicalization that rejects cycles, accessors, sparse arrays, non-finite values, unsupported prototypes, forbidden keys, excessive nesting, and oversized payloads.
- Added scenario coverage gates for contract, provider failure, prompt injection, evidence integrity, determinism, and resource limits.
- Added per-scenario deadlines with cooperative AbortSignal cancellation, stable error-code assertions, output hashing, repeat-run determinism checks, and equivalence-group checks.
- Enforced immutable safety invariants in observed outputs: execution authority remains `none`, live execution remains disabled, and order intent remains `null`.
- The evaluation report is SHA-256 hashed and fails closed when required risk categories are missing or the pass-rate threshold is not met.
- No live trading, broker execution, or model-weight inference authority was added.
## 0.8.8 — AI commentary integrity and bounded report contract

- Added an independent JS-side validation boundary for AI report commentary: provider/model identity, UTC generation time, exact four-domain coverage, same-domain evidence citations, caveat/list bounds, and fixed safety note.
- Recompute and verify the native output SHA-256 over the normalized commentary document before evaluating any research evidence. Post-boundary edits or report-field substitution now fail closed.
- Added regression coverage for oversized text, cross-domain reference injection, modified narrative digests, unknown providers, invalid timestamps, and safety-note override attempts.
- This improves cross-layer integrity but does not make a hash a signature or establish source authenticity. Production certification still requires locked dependency installs, frontend/native builds, and runtime acceptance tests.

## 0.8.7 — AI Agent runtime, secure provider configuration, and strict analytics integration

- Added a native Tauri AI research runtime for allowlisted OpenAI, Anthropic, OpenRouter, and loopback-only Ollama endpoints. Remote API keys use the operating-system credential store with no plaintext fallback.
- Added bounded provider HTTP behavior (timeouts, redirects disabled, response caps), single-flight concurrency, persisted 12/hour and 50/day run budgets, token-output ceiling, and an append-only hash-chained SQLite audit journal.
- Derived bounded trend, momentum, structure, and location evidence from the validated native candle series. LLM content is schema-validated commentary only and is excluded from scoring; the JS orchestrator recomputes a cross-layer evidence-envelope digest and passes native evidence through the canonical registry and strict decision gate.
- Added import, format-validation, verification, and safe removal for GGUF, ONNX, and SafeTensors artifacts. Imported artifacts are registry-only and are never executed; local inference uses already-installed Ollama models only.
- Added UI workflows for provider configuration/probes, analyst runs, model artifact management, run budget visibility, and audit chain verification. Execution authority remains `none`, live trading remains disabled, and order intent stays absent.
- Release remains not certified: dependency lockfiles, Svelte/Tauri build, Rust tests/Clippy, real provider integration testing, and cross-platform acceptance are not verified in this workspace.

## 0.8.6 — Canonical evidence registry and lineage enforcement

- Added a bounded canonical evidence registry using deterministic JSON normalization and Web Crypto SHA-256 content addressing; external aliases are excluded from canonical identity so alias changes cannot manufacture independent evidence.
- Added derivation parent resolution with cycle/missing-parent rejection, per-record and total-registry size limits, strict JSON-value validation, immutable registry records, and process-local registry verification.
- Made the registry mandatory for accepted decision signals; the decision engine now requires canonical IDs, verifies evidence digest and market snapshot scope, and blocks cross-domain reuse of canonical evidence.
- Advanced the decision/provenance/policy contracts to v4/v2/v3 and added registry digest metadata to the provenance integrity section. No order intent or live-execution authority is emitted.
- Release certification remains blocked until dependency locks, frontend/native builds, runtime integration, and desktop acceptance pass in a pinned CI environment.

## 0.8.5 — Deterministic decision provenance graph

- Added a deterministic provenance graph linking the market snapshot, accepted signal domains, and evidence references.
- Added per-domain signed score contributions and explicit gate outcome traces to make decision behavior reconstructable.
- Namespaced/encoded evidence-node IDs to avoid collisions with internal graph identifiers.
- Added tests for deterministic traces, graph structure, score contribution metadata, and visible blocked-gate reasons.
- The graph is diagnostic, not cryptographic attestation; order intent remains null and live execution remains disabled.

## 0.8.4 — Decision-layer evidence independence and bounded inputs
- Hardened the research decision engine against evidence laundering: the same evidence reference cannot be counted as independent support across multiple technical domains.
- Added strict upper bounds for submitted signal batches, evidence references per signal, and reference length to keep decision evaluation resource-bounded.
- Added regression tests for cross-domain evidence reuse, oversized batches, oversized evidence lists, and overlong evidence references. Decision contract and policy identifiers now explicitly report v2.
- This remains a research candidate engine; it emits no order intent and grants no execution authority. Release certification remains blocked until locked installs, runtime integration, frontend/native builds, and desktop acceptance pass.


## 0.8.3 — Authentication-aware rate-limit isolation
- Split API rate budgets so failed bearer credentials use a separate 10-attempt/60-second budget, while authenticated requests retain a 30-request/60-second operational budget.
- Authenticate before consuming the authenticated-client budget, preventing invalid-token floods from spending all legitimate-client quota. Failed-auth bursts return controlled 429 responses with `Retry-After` after their separate budget is exhausted.
- Added pure Node policy tests and a Bun/Hono integration test for failed-auth flood isolation.
- Added CI job timeouts and cancellation of stale push/PR runs to limit hung/stale workload.
- Release remains uncertified until dependency lockfiles, Bun/Hono integration, frontend/native builds, and desktop acceptance pass.


## 0.8.2 — Request-body deadlines and slow-stream containment
- Added a 10-second deadline to streamed analysis request-body reads, alongside the existing 2 MiB byte cap. Stalled streams return a controlled HTTP 408 and are cancelled best-effort without waiting on a potentially hostile cancellation promise.
- Added timeout outcome telemetry and regression coverage for stalled streams and invalid timeout configuration.
- Kept the API read-only and fail-closed; this is not a live-trading or production-certification change.

## 0.8.1 — Bounded, redacted request observability

- Added versioned JSON request-completion events with server-generated request IDs, allowlisted route and method labels, status, bounded duration, and a finite outcome taxonomy. Query strings, arbitrary path segments, bodies, headers, tokens, client IPs, provider payloads, and exception text are excluded.
- Added a fixed-memory observation sink capped at 120 emitted events per 60-second window. Suppressed event volume is attached to a later emitted row so hostile request bursts cannot create an unbounded log stream.
- Logging is best-effort and isolated from request correctness; a failed observer cannot change the API response. Added deterministic tests for redaction, event schema, outcome classification, sampling limits, and sink faults, plus Bun/Hono integration coverage.
- Aligned application version manifests and public-data User-Agent strings to 0.8.1. Release remains blocked pending locked dependency installs, real Bun/Hono tests, frontend/native builds, and desktop acceptance.


## 0.7.0 — Local API request-boundary hardening

- Added a dedicated request-context policy for the optional local research API. Canonical loopback `Host` validation, including listener-port matching, rejects DNS-rebinding-style authorities, alternate numeric IP spellings, invalid ports, and attacker-controlled host suffixes.
- Explicitly reject all browser `Origin` requests while the API remains no-CORS/CLI-native only, and use Fetch Metadata to reject cross-site browser probes. This avoids implying that browser clients are supported before a preflight/CORS contract exists.
- Evaluated Host/Origin/Fetch Metadata before consuming the shared rate-limit budget, preventing hostile browser probes from exhausting legitimate clients' authentication quota. Bearer authentication, bounded bodies, and research-only/no-execution guarantees remain mandatory.
- Added dependency-free request-policy tests, Bun/Hono integration tests for cross-site budget isolation, repository guardrails, and security documentation.
- The candidate remains non-certified until the full release CI proves the frontend build, Bun/Hono HTTP integration, Rust compile/tests/Clippy, and cross-platform Tauri acceptance.


## 0.5.8 — Layered architecture and deterministic research decision engine

- Moved implementation logic from the flat `src/lib` surface into domain, engine, application, adapter, presentation, and infrastructure folders. Existing import paths are compatibility shims, not duplicate implementations.
- Split the technical-analysis taxonomy from numerical calculations; freeze the domain-owned catalog and retain explicit implementation-coverage states.
- Added a structured technical-analysis envelope with OHLCV invariant checks, bounded analysis input, numeric indicator metrics, causal market-structure observations, dataset-cutoff identity, and capped evidence output.
- Added an independent cross-domain research decision engine. It validates unique evidence domains, cutoff and dataset-fingerprint binding, explicit scope, and consensus. Conflicts abstain; missing provenance blocks. It cannot produce an order intent or authorize live execution.
- Added regression suites for analysis output, future-prefix invariance, malformed inputs, evidence conflicts, missing fingerprints, paper-risk prerequisites, and layer dependencies.
- Added a strict, repeatable `typecheck:layers` gate to the frontend CI path so changed domain/engine/application JavaScript modules are type-checked before Svelte production build.
- Tightened decision diagnostic wording to distinguish independent-domain agreement from the separate weighted score metric.
- Preserved compatibility imports to reduce consumer breakage. Bun/Hono migration was not promoted: Bun is unavailable in this workspace, and the current Tauri-to-Rust architecture does not require an HTTP server.
- Certification remains blocked pending locked dependency installation, Svelte typecheck/build, native Rust compile/tests/Clippy, desktop packaging, and runtime acceptance. See `VALIDATION_REPORT_0.5.8.md`.


## 0.5.7 — Backtest robustness suite
- Added a moderate and severe fee/slippage shock to each successful native backtest, exposing applied costs, cost ceilings, return delta versus baseline, drawdown, Sharpe estimate, and trade count.
- Added fixed-size chronological out-of-sample windows with preceding indicator warm-up, no warm-up trades, fresh capital/flat inventory per fold, and a cap of 12 reported windows. Insufficient history is clearly reported instead of fabricated.
- Added native regression tests for cost sensitivity outputs, window chronology, short-history handling, and invariance of earlier fold results to later candle-value changes.
- Added UI panels and included the reports in the existing complete-result JSON export. This is a diagnostic suite, not a parameter optimizer or live-trading authorization. Native Rust execution and frontend build still require CI verification.

## 0.5.6 — Native candle continuity and Bun compatibility gate
- Added one canonical Rust-owned OHLCV validator: minimum history, finite/positive price fields, valid range geometry, UTC interval-grid alignment, checked timestamp subtraction, and exact adjacent-bar continuity.
- Binance candle batches now fail closed on malformed rows, duplicates, ordering defects, irregular cadence, or internal gaps instead of sorting/deduplicating away evidence. A still-forming candle is excluded only when it is the final provider row.
- The native cache and dashboard capability checks use the same validator. Invalid candles are withheld from charts, indicators, strict analysis, and backtests while valid quote refreshes remain usable, with an audit warning.
- Added frontend interval-grid validation and regression coverage, plus native Rust unit cases for gaps, duplicates, ordering, alignment, invalid OHLCV, and insufficient history. Native Rust tests are authored but not executed in this environment.
- Added a separate Bun CI compatibility lane for dependency-free source tests and repository guardrails. Node remains the primary package/runtime path until a Bun lockfile can be generated and verified with frozen installs and the Tauri build; Hono was not added because the current desktop app already uses native Rust/Tauri commands and has no standalone JavaScript HTTP backend to replace.

## 0.5.5 — Consistent benchmark liquidation accounting
- Fixed a backtest accounting defect where benchmark exit costs were applied only when the strategy remained in a position at the end of the sample. The benchmark now applies final-close slippage and fees regardless of strategy position state.
- Added a native Rust regression test that independently computes the expected buy-and-hold benchmark after entry and exit costs in a strategy-flat scenario, plus a Node source-contract guard.
- This corrects relative-performance comparability; it does not validate strategy profitability or replace native Rust CI execution.


## 0.5.4 — Verifiable desktop release artifacts
- Added an unsigned machine-readable release manifest with source commit, clean-tree assertion, build target/toolchain metadata, and a deterministic complete inventory of packaged files and symlinks.
- Added a checksum map that covers each bundle artifact and the exact manifest bytes, plus a verifier that detects inventory drift, file tampering, unexpected outputs, and version misalignment.
- Tagged CI now generates, verifies, and uploads the release evidence alongside per-platform Tauri artifacts. Evidence is explicitly not a digital signature or proof of publisher identity.
- Added dependency-free tests for known file digests, full inventory, byte tampering, added files, checksum/manifest tampering, empty bundles, manifest mismatch, path safety, and symlink handling.

## 0.5.3 — Fail-closed backtest cadence integrity
- Replaced median-only timestamp spacing inference in the native backtest with exact adjacent-bar cadence enforcement, preventing internal missing bars from silently skewing annualized metrics.
- Added checked timestamp subtraction and explicit fail-closed handling for unrepresentable deltas.
- Added native regression tests for missing intervals, irregular spacing, and timestamp overflow, plus Node source-contract guards.
- This engine policy is for continuous markets; session-aware instruments require a trusted venue calendar. Rust tests remain unexecuted locally without the Rust toolchain.

## 0.5.2 — Immutable validated market-data snapshots
- Closed a time-of-check/time-of-use gap: UI rendering previously validated a bridge response but still retained the mutable original object.
- `validateCandleSeriesResponse` now returns a frozen defensive snapshot with a normalized request and explicitly copied OHLCV fields; the UI consumes this returned value.
- Added mutation-after-validation regression tests that prove the rendered snapshot cannot be altered through the original response reference.
- This hardens in-process data integrity only; it does not authenticate provider data or establish production readiness. Dependency lockfiles and native/frontend build verification remain release blockers.


## 0.5.1 — Fail-closed candle continuity validation
- Replaced median-only interval cadence validation with exact per-adjacent-candle interval checks at the market-data boundary.
- Missing candle intervals and irregular timestamp offsets now reject the full series before it reaches charts or indicators; the error identifies the row, expected interval, observed interval, and missing-interval count when derivable.
- Added regression cases for a single missing candle interval and a non-integral cadence gap.
- This strict policy is appropriate only for configured continuous-market series; session-aware market calendars are required before applying it to markets with scheduled closures.
- This does not change catalog coverage, agent dispatch status, or live-execution authority.

## 0.5.0 — Causal prefix-invariance verification for research detectors
- Added a deterministic native Rust regression suite that runs the same generated OHLCV sequence as every possible prefix and asserts that already-observed swing, FVG, FVG-lifecycle, liquidity-sweep, and structure-break events never change when future candles are appended.
- This directly tests the anti-lookahead contract across the detector family rather than relying only on source-pattern assertions or isolated confirmation-index checks.
- Added a dependency-free Node contract assertion so the causal regression cannot be silently removed from the Rust test module.
- The Rust regression is authored but not executed in this environment because the Rust toolchain is absent; native CI must run it before release. No catalog status is promoted and live execution remains disabled.

## 0.4.9 — Price-indexed research detectors
- Replaced all-active-pivot scans in liquidity sweep and close-confirmed structure-break detection with ordered price indexes and range queries.
- Preserved strict confirmation timing, multi-level same-candle batching, independent high/low level lifecycles, deterministic event ordering, and repeated sweep observations.
- Applied the one-million-event fail-closed output budget to swing, sweep, and structure-break outputs; no truncated partial result is returned.
- Reworked swing-window extrema evaluation into a correct forward-sliding monotonic-deque algorithm (O(n)); added deterministic generated-candle oracle coverage and signed-zero price-key ordering tests. Static contracts also guard the native test helper return type and indexed query architecture.
- Tightened CI to use `cargo fmt -- --check` and fail Clippy warnings with `-D warnings`, instead of silently formatting the working tree or tolerating warnings.
- Native Rust tests are authored but not run in this environment; this is not a production certification. Catalog status and live-execution restrictions remain unchanged.

## 0.4.8 — Fail-closed numerical geometry for FVG research
- Reject FVG zones when derived width is non-finite or not positive, even if source prices are individually finite.
- Use overflow-safer midpoint calculation and reject non-representable lifecycle reference geometry.
- Add native Rust regression coverage for finite input values that overflow during subtraction and a Node source-contract assertion.
- Remove accidentally nested prior-release source tree from the release workspace/package input.


## 0.4.7 — Fail-closed market-structure provenance

- Hardened the JavaScript research-only structure-break boundary against forged pivot prices, invalid/out-of-range indices, non-causal confirmation references, and duplicate pivot identities.
- Made simultaneous confirmed-pivot processing deterministic independent of caller array order.
- Added regression tests for provenance forgery, invalid causal references, and duplicate identities.
- No catalog statuses were promoted; no live trading capability was enabled. Full frontend/native build validation remains a release gate.

## 0.4.6 — Native compile-blocker correction and regression guard

- Fixed `issued_work_plans()` return type to match the three-field tuple stored by `ISSUED_WORK_PLANS` and consumed by the replay-protection logic.
- Added a regression guard for the static/accessor tuple contract.
- Native Rust compilation remains a mandatory release gate and was not run in this environment.

## 0.4.5 — Market-structure identity and timeline hardening

- Track consumed structure levels by `(pivot_index, swing_kind)` so an outside-bar pivot's high and low are not accidentally coupled.
- Pre-index confirmed swings and structure breaks by candle for linear-time snapshot assembly after detector output is available.
- Add a Rust regression test for independent outside-bar high/low break lifecycles.
- Native Rust compilation and tests remain a release blocker in environments without Rust tooling.


## 0.4.3 — Deterministic batch semantics for multi-level structure breaks

- Fixed a causal-state consistency defect: structure breaks discovered on one candle are now classified against a single pre-candle bias snapshot instead of mutating bias inside event iteration.
- Multiple confirmed levels crossed by the same close are preserved as distinct events, but their classifications no longer depend on sort order; directional bias advances once per candle batch.
- Added a Rust regression test using a candle that breaks two confirmed swing highs at once, asserting both events retain the same prior bias and initial-break classification.
- Kept BOS/CHoCH/MSS/MSB labels unpromoted: the detector remains a research primitive and does not claim universal ICT semantics. No catalog entry was promoted; live execution remains disabled.
- Rust tests are authored but require execution in native CI. This release is not production-certified.


## 0.4.2 — Durable specialist-dispatch receipts and strict audit chain

- Added a SQLite hash chain for work-plan issuance, single-use consumption, attempt start, and completion finalization. Every event hashes the previous event, and a separately stored chain head detects ordinary tail truncation.
- Cross-checks chain events against persisted work-plan JSON/digests, consumed markers, attempt rows, assessment bundle, serialized result, and receipt digest before decision-layer persistence operations proceed. Corruption fails closed.
- Added a native audit-chain verification command and TypeScript bridge API.
- Added SQLite schema v3 receipts for the nine specialist categories, bound to exact plan/task/output hashes and cross-linked to the append-only audit chain. The native receipt writer is not exposed to frontend callers.
- The strict gate blocks completion when any trusted specialist receipt is absent or mismatched; provider dispatch remains unconfigured and is never inferred from a work plan.
- Added schema-v1 migration that imports extant rows into the chain. Historical attempts whose plan records had already been removed by old TTL cleanup are explicitly snapshotted as legacy orphan attempts rather than silently discarded.
- Removed routine TTL deletion of historical work-plan rows; TTL now governs eligibility and pending capacity, not audit-history retention.
- Bumped decision-gate contract to v4 and added source-contract tests plus Rust fault tests for event tampering, tail truncation, migration, and blocked finalization.
- SHA-256 chain is tamper-evident against partial edits but is not a digital signature; secure external anchoring remains a production security requirement for a hostile-local-database threat model.
- Master-list status remains unchanged (6 calculated, 1 workflow, 357 catalog-only); live execution remains disabled.

## 0.4.1 — Durable strict-analysis replay protection

- Replaced the production work-plan replay authority with a dedicated SQLite-backed registry. The store persists issued plan JSON, a SHA-256 integrity digest, issuance time, and a consumed marker.
- Production Tauri commands now create and consume plans through the durable store; process-memory replay protection remains only for isolated unit tests/parity harnesses. A durable attempt ledger records `in_progress` claims and finalizes full per-concept assessment bundles, decisions, serialized results, and audit receipt digests before responses are returned.
- Issuance/capacity checks and one-time consumption use SQLite `BEGIN IMMEDIATE` transactions, a conditional update, a 5-second busy timeout, WAL journaling, and `synchronous=FULL`. Storage errors fail closed.
- Persistent replay protection is designed to survive fresh database connections and application restarts; tampered requests do not consume the original plan. A blocked assessment attempt still consumes the plan and requires a new plan to retry.
- The 364-concept strict gate is unchanged: 357 Catalog only concepts remain blockers and live execution remains disabled.
- Rust compilation/tests and frontend production build remain unverified in this environment because the Rust toolchain is unavailable and frontend dependencies are not installed.

- Added a Rust research timeline that combines confirmed swing levels and close-confirmed structure breaks into one auditable snapshot per candle.
- Added explicit active swing levels, bias, transition classification, and operational protected high/low levels.
- Conflicting bullish and bearish breaks on the same candle now produce an explicit `ambiguous_break` snapshot and freeze the state machine's bias/protected levels rather than arbitrarily selecting an order.
- Added Rust unit tests for snapshot cardinality, point-in-time swing availability, and invalid windows, plus Node source-contract tests.
- Documented limitations and release acceptance requirements. The new primitive remains research-only and does not promote catalog entries or authorize trading.

## 0.3.9 — Causal market-structure break primitives

- Added Rust research-only structure-break detection using confirmed swing levels and close-crossing semantics.
- Added explicit initial/continuation/reversal-candidate labels, deterministic event provenance, and wick-only rejection tests.
- Preserved fail-closed catalog inventory: no concept status was promoted and no live execution was enabled.
- Added static source-contract coverage; Rust compile/unit tests remain unverified until a Rust toolchain is available.

# Changelog

## 0.5.1 — Fail-closed candle continuity validation
- Replaced median-only interval cadence validation with exact per-adjacent-candle interval checks at the market-data boundary.
- Missing candle intervals and irregular timestamp offsets now reject the full series before it reaches charts or indicators; the error identifies the row, expected interval, observed interval, and missing-interval count when derivable.
- Added regression cases for a single missing candle interval and a non-integral cadence gap.
- This strict policy is appropriate only for configured continuous-market series; session-aware market calendars are required before applying it to markets with scheduled closures.
- This does not change catalog coverage, agent dispatch status, or live-execution authority.

## v0.3.8 — Single-use strict work plans

- Added atomic, process-local single-use consumption for Rust-issued strict work plans.
- Replay attempts are blocked; each completion attempt consumes its plan, including blocked attempts.
- Tampered work-plan submissions do not consume the authentic issued plan.
- Added native Rust unit tests and static contract coverage for replay and tampering.
- Explicit limitation: replay state is process-local and is not restart-safe until a durable transactional store is implemented.

## v0.3.7 — Verifiable audit receipt contract

- Added native Rust `verify_audit_receipt` to validate receipt schema/version, required identity, timestamp and digest shape, nine specialist digests, coverage invariants, disabled live authority, and self-hash integrity.
- The native completion path now verifies its own generated receipt before returning it.
- Added Rust unit tests for tampering, forbidden live authorization, mismatched master-list version, and missing specialist digest.
- Static Node source-contract tests are not a substitute for native Rust execution.


## v0.3.6 — Reproducible strict-analysis audit receipts

- Added a native Rust audit receipt to every strict-completion result, including blocked requests, with an integrity fingerprint over the receipt body.
- Added deterministic SHA-256 over sorted assessment bundles plus per-category assessment digests across all nine specialist categories. Input order no longer changes the overall bundle digest; changed content changes the digest.
- Bound receipt metadata to work-plan ID, catalog version, symbol/timeframe/asOf, dataset fingerprint, decision, coverage counters, and blocker digest.
- Added Rust unit tests for blocked-result receipts, nine-category digest coverage, tamper sensitivity, and assessment-order independence.
- No concept status was promoted. The decision gate remains fail-closed while any concept is unsupported or catalog-only; live execution stays disabled.
- Validation remains incomplete because this environment has no Rust toolchain and frontend dependencies are not installed.

## [0.3.5] — 2026-10-27

- Added deterministic Rust research classification of confirmed swings as HH/HL/LH/LL, with explicit initial/equal labels and prior-pivot comparison provenance.
- Labels carry pivot and confirmation indices/timestamps; the API derives pivots directly from validated candles and does not accept caller-forged swing lists.
- Added Rust unit-test source for HH/LH/HH and initial/LL/HL sequences plus explicit equal-high handling; added Node source-contract assertions.
- Did not promote catalog entries or connect research output to decision authorization. Full 364-concept fail-closed gate and disabled live execution are preserved.
- Validation limitations are documented in `docs/VALIDATION_REPORT_0.3.5.md`.

## [0.3.4] — 2026-10-27

- Added a research-only Rust confirmed swing-high/swing-low primitive with explicit confirmation indices/timestamps, strict unique-pivot semantics, bounded windows, and fail-closed candle validation.
- Added Rust unit-test source and Node source-contract coverage for pivot confirmation and tied-pivot rejection.
- Kept swing concepts catalog-only; no strict-gate promotion, agent-dispatch claim, or live execution authorization.
- Validation limitations are documented in `docs/VALIDATION_REPORT_0.3.4.md`.

## [0.3.3] - 2026-10-27

### Rust-first FVG lifecycle research

- Added a deterministic Rust FVG lifecycle evaluator that emits one-time `partial_fill` and `full_fill` observations and an explicitly non-authoritative `inversion_candidate` when a candle closes beyond the zone's far boundary.
- Lifecycle evaluation rejects malformed/forged gap references and only examines candles strictly after the gap formation candle; events carry observation indices/timestamps and are sorted chronologically.
- Added native Rust unit tests for partial-to-full progression, inversion-candidate semantics, and invalid gap provenance, plus Node static contract checks.
- FVG remains `Catalog only`; lifecycle events are research observations and are not connected to assessment promotion, decision authorization, or order execution.
- Version bumped to 0.3.3. Node suite and static repository checks were rerun; native Rust and frontend build limitations remain explicit in the validation report.

### Validation status

- `npm test`: 68/68 Node tests passed; this does not substitute for Rust execution.
- `npm run verify:repo`: static guardrails pass with missing-lockfile warnings.
- Rust compile/tests/Clippy and frontend build remain unverified/blocked in this environment.

## [0.3.1] - 2026-10-27

### Rust-first strict-analysis hardening

- Added process-issued strict work-plan IDs with a bounded registry (256 active plans), a 15-minute expiry, and exact payload/tamper checks.
- Added explicit canonical timeframe validation and rejection of incomplete candles at the snapshot cutoff.
- Rewired native Tauri commands to reconstruct market context from the connected, fresh Rust-owned Binance cache, recompute OHLCV SHA-256, and ignore caller-supplied risk approval in favor of Rust account-state preflight.
- Added a strict work-plan JSON Schema and static contract tests. All nine specialist files still route the complete 364-entry manifest, while the plan explicitly remains `work_plan_created_not_dispatched`.
- Kept the decision gate fail-closed: 357 catalog-only concepts continue to block downstream completion; live execution remains disabled.
- Added `docs/VALIDATION_REPORT_0.3.1.md` with exact verification results and release blockers.

### Validation status

- `npm test`: 63 tests passed.
- `npm run verify:repo`: static guardrails passed with missing-lockfile warnings.
- `npm run build`: blocked because `svelte-check` is not installed.
- Rust tests, formatting, Clippy, and native desktop packaging were not run because `cargo`/`rustc` are unavailable. No claim of production readiness is made.

## [0.3.0] - 2026-10-27

### Strict decision contract hardening

- Bound each assessment to its exact catalog ID, category, group, name, master-list version, and code-owned specialist routing identity.
- Added implementation version and SHA-256 dataset snapshot requirements for evaluated evidence.
- Required contextual evidence for `not_applicable` assessments and strengthened market-data scope checks with symbol and timeframe.
- Added deterministic assessment coverage by category and malformed-assessment visibility.
- Preserved fail-closed behavior: the 357 catalog-only concepts block any downstream handoff; live execution is never authorized.
- Added full contract details to all nine specialist knowledge packs and the orchestrator.

- Added a Rust-native strict analysis work-plan/completion gate using the embedded reviewed 364-entry manifest, with typed Tauri commands and native unit tests for identity, routing, evidence, snapshot binding, duplicates, and fail-closed handoff.
- Expanded all nine agent knowledge packs with domain-specific data requirements, confirmation timing, and anti-look-ahead rules.
- Explicitly labels work-plan generation as not dispatched; no real LLM/provider adapter is claimed or configured.

### Validation boundaries

- Node unit tests and static repository guardrails must pass before this package can be considered for further validation.
- This version is not declared production-ready until dependency lockfiles, Svelte build/type checks, Rust tests/Clippy, and native packaging/acceptance checks succeed in a pinned environment.

## 0.2.0 — strict analysis decision gate and specialist agent packs (2026-10-27)

- Added nine category-specific agent knowledge packs, each containing the full inventory for its domain, plus a central strict orchestrator contract.
- Added machine-readable `docs/STRICT_DECISION_LAYER_POLICY.json`.
- Added deterministic `evaluateStrictDecisionGate` contract and regression tests.
- Downstream handoff now fails closed on missing/duplicate/unknown assessments, unsupported or insufficient-data concepts, missing evidence/timestamps, invalid market-data provenance, or failed risk approval.
- Catalog-only concepts cannot self-promote to evaluated; promotion requires implementation, tests, and reviewed manifest changes.
- Live execution remains explicitly unauthorized.
- Bumped package/Tauri/Rust package version to 0.2.0.

## 0.1.9 — single-instance persistence hardening (2026-10-27)

- Added process-lifetime exclusive SQLite account lock to reject a second instance opening the same paper-account database.
- Added regression coverage for lock acquisition, duplicate instance rejection, and release on close.
- Preserved explicit release blockers for missing dependency lockfiles and unavailable local build toolchains.

## 0.2.1 — trusted catalog registry and anti-spoofing gate (2026-10-27)

- Added a code-owned 364-entry registry so agent-supplied catalog availability cannot promote an unimplemented concept.
- Decision gate now requires exact match of every catalog ID, category, name, and implementation status against the trusted registry.
- Catalog-only concepts must remain `unsupported` until reviewed implementation, tests, and registry promotion; this keeps strict handoff blocked while 357 concepts remain catalog-only.
- Evaluated results now require explicit look-ahead safety confirmation; insufficient-data and unsupported assessments require meaningful reasons; intent must be explicitly `research` or `paper_trade`.
- Added exhaustive knowledge-pack coverage tests and included them in CI.
- Bumped package/Tauri/Rust package version to 0.2.1.
- Release remains blocked pending lockfiles, frontend/Rust toolchain builds, and native acceptance tests.

## [0.3.2] - 2026-10-27

- Added research-only Rust primitives for three-candle FVG geometry and confirmed-pivot liquidity sweep observations.
- Added adversarial/known-vector Rust unit tests and Node source-contract checks.
- Kept FVG and Liquidity Sweep as `Catalog only`; no detector is promoted or connected to the strict decision handoff.
- Added `docs/RESEARCH_SMC_DETECTORS_0.3.2.md` and `docs/VALIDATION_REPORT_0.3.2.md`.
- Native Rust compilation and unit tests remain unverified because the environment has no Rust toolchain.

## 0.4.0 — Point-in-time market structure state machine

- Added a Rust research timeline that combines confirmed swing levels and close-confirmed structure breaks into one auditable snapshot per candle.
- Added explicit active swing levels, bias, transition classification, and operational protected high/low levels.
- Conflicting bullish and bearish breaks on the same candle now produce an explicit `ambiguous_break` snapshot and freeze the state machine's bias/protected levels rather than arbitrarily selecting an order.
- Added Rust unit tests for snapshot cardinality, point-in-time swing availability, and invalid windows, plus Node source-contract tests.
- Documented limitations and release acceptance requirements. The new primitive remains research-only and does not promote catalog entries or authorize trading.

## 0.5.9 — Bun/Hono research boundary (candidate; not release-certified)

- Added an isolated Bun/Hono loopback-only research API. The desktop Svelte/Tauri/Rust path remains the primary application architecture; the API is opt-in and is not required to launch the desktop app.
- Added bearer-token enforcement, bounded JSON input, strict OHLCV cadence validation, request rate limiting, safe error envelopes, and no order/execution routes.
- Client-submitted candles are explicitly treated as untrusted research input. The API can return analytical observations but cannot produce a trusted provider handoff or execution authority.
- Added runtime-independent tests for the API input validator and source-contract tests for security boundaries.
- Reproducibility: the local API now emits the canonical domain-separated OHLCV SHA-256 dataset identity matching the native Rust known vector; this improves replay/evidence binding but does not claim provider authenticity.
- Security hardening: added an incremental bounded UTF-8 stream reader that enforces the body cap even without a trustworthy `Content-Length`; rejects invalid UTF-8 and compressed bodies; applies the request budget before authentication so failed token guesses are rate-limited; adds no-store and browser hardening headers.
- Follow-up hardening (2026-10-28): replaced fixed-window request throttling with a bounded-memory sliding-window limiter, monotonic-clock rollback protection, and `Retry-After`; restricts the API bind to literal loopback IPs and JSON charset to UTF-8; distinguishes a computed dataset hash from a verified fingerprint; and wires the actual Bun/Hono HTTP integration suite into CI. Tagged release acceptance now requires npm, Bun, and Cargo lockfiles and uses frozen Bun install/testing. Five new deterministic limiter tests pass locally.
- Added tests for oversized streamed bodies without `Content-Length`, fragmented request chunks, invalid UTF-8, failed-auth request budget, and security headers. The Node suite exercises the actual stream reader; Bun/Hono route integration tests are authored but remain unrun locally.
- Release blockers remain until dependency lockfiles, Bun/Hono runtime tests, Svelte build, native Rust checks, and desktop packaging are verified in CI. This is a security-hardening continuation of the 0.5.9 candidate, not a version bump or a production certification.


## 0.6.0 — market-data assurance and engine-owned timeline gate (candidate; not release-certified)

- Consolidated the supported candle intervals into one domain contract shared by market-data validation, the technical-analysis engine, and the opt-in local API.
- Added a bounded OHLCV/timeline audit covering supported timeframe, safe integer timestamps, UTC-grid alignment, strictly increasing timestamps, and exact adjacent-candle cadence. Invalid series suppress indicator metrics and directional signals even if a direct caller marks its metadata validated.
- Added robust latest-bar diagnostics for extreme close returns, volume spikes against a prior median/MAD baseline, and 20 identical flat-price bars. They are advisory and do not automatically discard genuine market events, but pause candidate handoff while warnings remain.
- Added explicit audit status/reason codes and rendered audit status in the UI; updated the technical-analysis envelope to v2.
- `npm test`: 149/149 passed across 25 Node suites; strict layer typecheck and repository guardrails passed.
- Still not production-certified: lockfiles, Bun/Hono runtime verification, Svelte/Vite/Tauri build, native Rust tests/Clippy, and desktop/platform acceptance are not verified in this workspace. No live execution is enabled.
