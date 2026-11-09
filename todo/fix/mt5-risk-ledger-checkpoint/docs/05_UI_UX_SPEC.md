# 05 — UI/UX Specification

## Visual direction
Professional trading terminal: deep navy/charcoal background, restrained cyan for focus, green/red for market movement, neutral typography, compact but readable tables, consistent spacing, clear hierarchy. Avoid decorative gradients, excessive glow, random charts, oversized KPI cards, and fabricated real-time numbers.

## Primary navigation
- Home / Overview
- Trading
- Strategy Builder
- AI Agents
- Backtesting
- Paper Trading
- Live Trading (locked/disabled until explicitly enabled)
- Market Data
- Meme Coin Screener
- Portfolio
- Tools & MCP
- Settings

## Overview
- Portfolio/PnL and risk KPIs with period labels.
- Market watchlist.
- Main chart with explicit timeframe and source.
- Agent status and last decision.
- Open positions/orders.
- Recent events and system health.
- Data freshness indicator always visible.

## Meme Coin Screener
- Filters stay visible and can be saved as presets.
- Table supports sorting, column selection, search, watchlist, and pagination/virtualization.
- Momentum and risk are visually separate; do not combine into one mysterious “AI score.”
- Clicking a row opens a detail drawer without losing current filters.
- Show timestamp and source freshness in the table and details.
- Distinguish observed values from model-derived features.
- Use plain-language risk explanations and direct links to source explorers where possible.
- Empty/error/stale states must explain what happened and how to recover.

## AI Agents
- Agent status: stopped, starting, running, degraded, error.
- Show objective, provider/model, allowed tools, action mode, cost/budget, last run, last decision.
- Show event trace: observation → tool call → result → proposal → risk decision → action.
- Stop button must be prominent; dangerous actions require confirmation.

## Backtesting
- Configuration summary, data coverage, assumptions, fees/slippage, strategy version.
- Equity curve, drawdown, trade list, metrics, and benchmark.
- Warnings for look-ahead bias, missing data, or unrealistic fills where detectable.
- Backtest results must be reproducible and exportable.

## Trading / order panel
- Show account/mode (paper/live) prominently.
- Display estimated notional, fees, exposure after order, and risk checks.
- Confirm order only after risk gateway approves.
- Never let visual color alone convey the only risk status.
- Provide a global kill switch and clear state after activation.

## Accessibility and usability
- Keyboard navigation and visible focus.
- Tooltips for financial metrics.
- Tables support readable density and column customization.
- Use consistent number formatting and timezone labels.
- Avoid flashing animation; respect reduced-motion preferences.
- Never use placeholder values that could be mistaken for live market data.
