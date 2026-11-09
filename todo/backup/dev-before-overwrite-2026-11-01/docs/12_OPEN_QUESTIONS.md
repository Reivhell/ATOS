# 12 — Open Questions and Current Decisions

> This document began as a pre-implementation questionnaire. The choices below define the current research/paper-trading baseline; items explicitly left open require a product/operations decision before release.

## Resolved for the current baseline

- **Desktop target:** Tauri 2 desktop shell, with Svelte 5 + TypeScript UI.
- **Core runtime:** Rust owns market-data adapters, paper execution, risk checks, backtest calculation, persistence, and audit events. Python is deferred; there is no current feature that justifies another runtime and its deployment/isolation burden.
- **Initial data domains:** public Binance Spot read-only tickers for BTC, ETH, SOL, BNB, DOGE plus BTCUSDT 5-minute candles; DEX Screener latest boosted-token discovery feed for the screener.
- **Persistence:** local SQLite with WAL/FULL synchronous configuration and a schema-versioned serialized state plus audit ledger.
- **Execution target:** paper trading only. Live broker calls, account credentials, wallet signing, and automated trading agents are deliberately absent.
- **Initial risk defaults:** local paper account defaults are defined in the engine and are operator-configurable from the Risk UI. They are research defaults, not a recommended financial risk profile.
- **Agent behavior:** visible agent toggles are observe-only status controls; they do not run a scheduler, call an LLM, or place orders.

## Decisions still required before a release

- **Supported OS/release matrix:** Windows, macOS and Linux are targets to validate, not yet verified deliverables. Installer packaging and signing need platform-specific acceptance.
- **Provider suitability and licensing:** confirm provider rate limits, terms, availability expectations, and whether a paid/contract-audit provider is needed. The boosted-token feed is not a full token universe.
- **Unknown token risk:** treat missing contract/holder/lock data as unknown, never as safe. The present screener emits unresolved-risk flags but is not a security scanner.
- **Live-trading policy:** no default permission policy is accepted yet. Any future live mode needs explicit per-order approval policy, keychain-backed credentials, exchange sandbox tests, duplicate-order reconciliation, an independent kill path, and threat-model approval.
- **License:** no license is selected. Do not ship as open-source or add a license file until the owner chooses the rights and obligations.
- **Acceptance target:** the required test matrix is documented in `09_TESTING_AND_ACCEPTANCE.md`; cross-platform test evidence and reproducible dependency lockfiles are outstanding.


These should be answered before the architecture is treated as final.

## Product and platform
1. Is the first deliverable a desktop app, local web dashboard, or web app?
2. Is the main goal research/paper trading first, or is live trading a near-term requirement?
3. Should the UI be React + TypeScript, and should the desktop shell be Tauri?
4. Which OSes must be supported for the first release?

## Markets and data
5. Which first market is required: centralized-exchange crypto, on-chain DEX tokens, Forex, or XAUUSD?
6. For the meme coin screener, which chain comes first (e.g. Solana or EVM)?
7. Which provider is acceptable for token discovery, liquidity, holder data, contract-risk checks, and social signals?
8. What rate limits and data licensing constraints apply?

## Runtime
9. Which database should be used initially?
10. Should Python strategies run in-process, in a subprocess, or in an isolated worker?
11. Which AI providers must be supported in the first release?
12. Is MCP integration required in MVP or after the core vertical slice?

## Risk policy
13. What are the default limits for maximum position, order size, daily loss, and exposure?
14. What is the policy for unknown token-risk data?
15. Should live trading require a manual approval for every order, or only for activation/configuration?
16. What is the exact kill-switch behavior and recovery procedure?

## Delivery
17. What is the repository name and preferred workspace layout?
18. Do existing source code, reports, and the original repository need to be audited before implementation?
19. Which license should the new project use?
20. What is the acceptance target for the first demo?

## Current recommendation
Keep the current paper-only boundary. Resolve the release matrix, data-provider terms, unknown-token policy, and license decision before packaging a public release. Do not unlock live trading to close a roadmap gap.
