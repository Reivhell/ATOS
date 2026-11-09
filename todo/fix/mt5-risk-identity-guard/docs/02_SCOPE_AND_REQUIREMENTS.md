# 02 — Scope and Requirements

## MVP must-have
- Market-data connector interface and one working provider.
- Canonical market event schema and timestamp/source metadata.
- Strategy lifecycle: create/configure/start/stop/version.
- Backtesting with fees, slippage assumptions, and clear limitations.
- Paper broker with order, fill, balance, and position simulation.
- Central risk gateway for all order intents.
- Agent runtime with allowlisted tools, timeouts, budgets, and audit traces.
- Dashboard for system status, charts, positions, orders, logs, strategies, and agents.
- Meme Coin Screener with configurable filters, transparent scoring, risk flags, and watchlists.
- Persistent event/audit history.
- Secrets stored outside source control and masked in logs.

## Should-have after the vertical slice
- More exchange and chain adapters.
- Multiple AI providers.
- Notifications and configurable alerts.
- Strategy comparison and parameter sweeps.
- Exportable trade and backtest reports.
- Portfolio-level risk views.
- Plugin/tool registry and MCP integration, with explicit allowlists.

## Later
- Live execution on selected venues.
- Multi-user and remote deployment.
- Advanced distributed runtime.
- Strategy marketplace or social features.
- Automated on-chain transaction signing.

## Functional requirements
### Market data
- Ingest trades, candles, order-book snapshots, and token metadata where available.
- Normalize symbol/chain identifiers.
- Track event time and receive time separately.
- Detect stale feeds, gaps, duplicate events, and provider disconnects.
- Never silently substitute missing values.

### Strategies
- Have immutable versions and explicit parameters.
- Declare required data and execution permissions.
- Support deterministic replay where possible.
- Emit typed signals/order intents, not direct exchange requests.

### Agents
- Have explicit objectives, allowed tools, model/provider, token/time budgets, and action permissions.
- Support observe-only, paper-action, and (later) live-action modes.
- Record prompt/template version, tool calls, outputs, errors, and decisions subject to secret redaction.
- Be stoppable immediately.

### Orders and risk
- All order intents pass through the risk gateway.
- Validate available balance, exposure, max order size, allowed symbols, market status, and configured loss limits.
- Use idempotency keys and explicit order states.
- Treat uncertain order status as a reconciliation problem, not as permission to resubmit blindly.

### Screener
- Support filters by chain, age, liquidity, market cap/FDV, volume, holder concentration, and risk flags where data exists.
- Show data source and last-updated time.
- Separate momentum score from risk score.
- Provide drill-down evidence and missing-data warnings.
- Never label a token “guaranteed,” “safe,” or “will pump.”

## Non-functional requirements
- **Reliability:** recover cleanly from provider disconnects and process restarts.
- **Performance:** keep UI responsive while ingestion/backtests run in the background.
- **Security:** least privilege, secret redaction, dependency review, and no withdrawal permissions for trading keys.
- **Auditability:** trace signal → risk decision → order intent → simulated/live order → fill.
- **Maintainability:** modular adapters, versioned contracts, automated tests, clear error types.
- **Portability:** prioritize Linux development; packaging target to be confirmed.
- **Observability:** structured logs, health state, metrics, trace IDs, and actionable errors.
