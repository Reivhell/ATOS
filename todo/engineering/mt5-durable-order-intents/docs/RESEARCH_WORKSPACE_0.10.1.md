# Rust-native Research Workspace — v0.10.1

## Purpose

v0.10.1 adds a bounded nested walk-forward parameter tournament to the existing Rust Research Lab. The change addresses a structural limitation in v0.10.0: a research run evaluated only one manually supplied SMA pair. The new workflow compares a bounded neighbourhood of SMA configurations independently inside each expanding training window, selects one configuration using training evidence only, and evaluates that selected configuration on the next chronological block that was not used for its selection.

The tournament is research-only. It does not emit orders, modify account state, grant paper or live execution permission, change a kill switch, or convert an LLM response into a trading parameter. All strategy calculations and policy gates execute in Rust.

## Workflow

When enabled, the Research Lab adds a `nested_parameter_tournament` node to the persisted workflow DAG:

1. Validate the exact native BTCUSDT 5-minute candle series and bind the run to its canonical SHA-256 fingerprint and observed date range.
2. Generate a deterministic SMA candidate neighbourhood around the explicitly selected anchor periods. The candidate grid is sorted, deduplicated, and capped at 25 pairs.
3. Divide the series into three chronological outer test windows and an expanding training prefix. Each fold uses the same fixed test-window length.
4. Run each candidate against only that fold's training prefix. A candidate is eligible only after at least three training trades.
5. Select the candidate with the highest training Sharpe ratio. Exact ties resolve by lower training drawdown, then lexicographically smaller `(fast, slow)` periods. The rule and all training scores are persisted.
6. Evaluate the selected candidate on the next unseen block. The backtest receives earlier candles only as indicator warm-up; its evaluation starts at the first candle of that test block. It starts flat and does not carry positions across fold boundaries.
7. Aggregate positive OOS fold ratio, median OOS return, worst OOS drawdown, and parameter stability. Missing candidates count as failed folds because the positive-fold ratio uses the requested fold count as its denominator.
8. Recompute every aggregate and verify candidate selection semantics before the immutable run card is persisted. The baseline backtest and tournament share one immutable native candle/provider snapshot; engine synchronization is held only while cloning that snapshot, not while running CPU-heavy simulations.

The first fold's candidate scores and selected pair are invariant to changes in the first test block and all later candles. Native unit tests cover that no-peek property. The work estimate is checked before execution.

## Explicit bounds

| Control | Policy |
|---|---:|
| Supported native data | BTCUSDT, 5-minute candles only |
| Dataset size | 150 to 2,000,000 candles |
| Candidate configurations | 1 to 25 |
| Outer folds | 2 to 5; Research Lab currently requests 3 |
| Minimum training trades per candidate | 3 |
| Estimated work budget | 50,000,000 candle-evaluations |
| Candidate parameter periods | fast >= 2, slow > fast, slow <= 300 |
| Candidate diagnostics | Training Sharpe, return, drawdown, trade count, eligibility |

The service returns a hard error when the required training or test window cannot be formed or the computed work budget is exceeded. It does not silently lower the number of candidates, folds, or evidence requirements.

## Evidence integrity

A tournament record includes its algorithm version, exact selection metric, candidate count, evaluation count, requested and completed fold counts, all per-candidate training scores, selected parameters, OOS results, fold dates, dataset fingerprint, aggregate statistics, blockers, and caveats.

Before persistence, the Research Workspace independently verifies:

- algorithm and selection-policy identity;
- dataset hash and date-range equality with the parent native backtest;
- canonical candidate ordering, uniqueness, eligibility, and identical candidate catalog across folds;
- chronological expanding-window boundaries and five-minute continuity;
- that the selected pair is the deterministic winner among qualifying training scores;
- that unqualified folds contain no selected pair or OOS metrics;
- evaluation counts and completed-fold counts;
- exact recomputation of positive-fold ratio, median OOS return, worst drawdown, and parameter stability;
- blockers and caveats are present and bounded when folds do not complete;
- the workflow DAG has the exact expected node ordering and dependency edges, consistent timestamps, bounded node details, valid status vocabulary, and status agreement with specialist/committee evidence.

The Research Lab's committee gate v2 additionally requires all requested folds to complete, the positive OOS fold ratio to meet the recorded user threshold, and the worst selected-candidate OOS drawdown to remain within the recorded research ceiling. Failure blocks promotion to supervised paper validation.

The workflow uses a separate SQLite research database with immutable run cards and a hash-chained event ledger, preserving the existing separation from account state and the order/risk gateway.

## Vibe-Trading adoption review

Reviewed the upstream [HKUDS/Vibe-Trading releases](https://github.com/HKUDS/Vibe-Trading/releases) on 2026-10-30. Relevant concepts adopted or reinforced:

- Strategy research that fixes parameters from earlier data and evaluates them on a later, untouched period. ATOS implements this as an expanding-window nested walk-forward tournament rather than running external strategy code.
- Evidence-grounded outputs that represent unavailable metrics as unavailable rather than plausible zeros or infinities. This behavior already exists in ATOS's backtest and research evidence contracts and is retained.
- Bounded execution and failure handling for agent/research workflows. ATOS applies explicit candidate/fold/work budgets and rejects incomplete tournament evidence at the committee gate.

Not imported because they do not fit the current ATOS execution contract or the verified data available here:

- Python agent, backtest, factor and loader stacks; the project remains Rust-native for deterministic trading and risk logic.
- Multi-market loader zoo, factor catalog, Alpha Zoo and external strategy child-process execution. The current Research Lab can only prove BTCUSDT 5m data and must not claim support for other markets/timeframes.
- Vibe-Trading's daily-bar paper-account workflow. ATOS already has a distinct Rust account/order/risk boundary; introducing a second paper ledger without a complete settlement, reconciliation and account-risk design would create ambiguous state ownership. A dedicated forward-only shadow account remains a separate design decision, not silently approximated here.
- Broker/MCP integrations, including MT5. The research tournament has no broker capabilities and the current market-data contract is not sufficient to validate a new broker adapter.

The upstream project's v0.1.18 release also describes connection-time custom-endpoint checks, stream-failure handling and on-demand tool schema loading. ATOS does not expose arbitrary remote model endpoints or a dynamic tool catalog through this research workflow; its existing provider allowlist, endpoint restrictions, bounded responses and no-order model contract remain in force. Those changes are not copied without the corresponding extension points.

## Operational limitations

The tournament is a deterministic research procedure, not a forecast, guarantee, or proof of a profitable strategy. Sharpe selection can overfit the training prefix; parameter stability is descriptive and is not a formal probability-of-backtest-overfitting statistic. OOS fold resets mean results do not model continuous position carry. Order-book depth, market impact, queue position, spread variation, latency and exchange-specific fills remain outside the current backtest assumptions.

A completed research gate means only that the stored evidence passed the configured research policy and may be considered for a separately supervised paper-validation process. It does not grant paper or live order authority.

## Release qualification

Source-level Node tests and the strict TypeScript layer check can run in this environment. Native Rust compilation/tests, rustfmt, Clippy, Svelte/Vite/Tauri packaging, provider/keychain acceptance, cross-platform acceptance, signed attestations and OS code signing have not been executed here. See `VALIDATION_REPORT_0.10.1.md`; the candidate must remain release-blocked until all release preflight blockers are resolved and verified in CI.
