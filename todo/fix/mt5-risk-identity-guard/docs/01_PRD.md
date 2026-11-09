# 01 — Product Requirements Document (PRD)

## 1. Product summary
Agentic Trading OS is a local-first, modular trading workspace for researching markets, building and testing strategies, supervising AI agents, and safely moving from backtesting to paper trading. Live trading is a later, explicitly gated capability.

The platform combines a Rust execution/runtime core with Python extension points and a desktop/web dashboard. It includes a Meme Coin Screener that ranks observable market opportunities while clearly exposing uncertainty and manipulation risk.

## 2. Problem statement
Trading workflows are fragmented across charting tools, scripts, exchange dashboards, backtest engines, and AI assistants. Users need one inspectable environment that connects data, strategy logic, agents, risk controls, execution, and audit trails without allowing an AI agent to bypass safety controls.

## 3. Product goals
1. Provide one consistent event-driven runtime for data, strategies, agents, risk, and orders.
2. Make strategy development and testing reproducible.
3. Make agent decisions inspectable: inputs, tools, rationale, output, and resulting actions.
4. Make paper trading the default route for validating a strategy.
5. Surface market opportunities and risk signals, including for meme coins.
6. Make failures, uncertainty, and risk visible rather than hiding them behind a single AI score.

## 4. Non-goals for MVP
- Guaranteed prediction of future prices or “sure winners.”
- Fully autonomous live trading without explicit user-configured limits.
- Supporting every broker, exchange, chain, or AI provider at launch.
- Copy-trading, social trading, or public strategy marketplace.
- Custody of user funds or collection of exchange withdrawal permissions.
- Recreating proprietary internals of another application without sufficient source/evidence.

## 5. Target users
- Individual traders who want to research, backtest, and paper-trade strategies.
- Developers building deterministic strategies or AI-assisted agents.
- Advanced users who need visibility into orders, positions, risk, and runtime health.

## 6. Primary user journeys
### Journey A — Strategy to paper trading
Create strategy → configure data/timeframe → backtest → inspect metrics and trades → run paper session → compare expected vs observed behavior → approve or reject promotion.

### Journey B — Agent supervision
Create agent → select allowed tools → configure model/provider → set action permissions and budget → run in observe-only mode → inspect traces → enable paper actions → stop or disable agent.

### Journey C — Meme coin discovery
Choose chain and liquidity/market-cap thresholds → filter launches and tokens → inspect score breakdown and raw signals → review liquidity/holder/contract risks → add to watchlist or paper-test a rule. The screener must not imply certainty.

### Journey D — Live trading (later phase)
Connect a trade-only API key → validate permissions → set hard risk limits → run a preflight check → require explicit confirmation → monitor orders and kill switch.

## 7. Product principles
- **Risk first:** risk checks are enforced outside the LLM and strategy code.
- **Explainable:** show why a signal exists and what data supports it.
- **Reproducible:** preserve strategy version, parameters, data range, and engine version for backtests.
- **Fail closed:** if risk state, data integrity, or order state is unknown, do not open new exposure.
- **Human controlled:** live execution is opt-in and separately permissioned.
- **Honest scoring:** a score ranks signals; it is not a probability of profit unless calibrated and validated as such.

## 8. Success metrics
- A user can run a full paper-trading session without manual database edits.
- Every simulated order can be traced to a strategy/agent decision and risk result.
- Backtests are reproducible with the same data, version, and configuration.
- Screener results expose component signals, timestamp, source freshness, and risk flags.
- No agent can bypass the risk gateway or trigger live orders without the required permissions.
- UI communicates stale data, failed providers, and disconnected exchanges clearly.

## 9. Product assumptions
- Initial target is a developer-oriented desktop dashboard, with local-first operation.
- Rust owns critical runtime and risk boundaries; Python is an extension interface, not the ultimate authority for order permission.
- Initial data/exchange provider will be selected during implementation planning.
- Initial screener support will target one chain/data source before multi-chain expansion.
