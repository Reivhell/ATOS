# 08 — Security and Trading Risk

## Security baseline
- Never commit credentials, API keys, private keys, seed phrases, or access tokens.
- Store secrets using the OS credential store or an encrypted secret facility; mask them in logs.
- For exchange keys, request read/trade permissions only as needed; disable withdrawals.
- Keep chain signing keys out of the agent runtime. Do not support autonomous wallet signing in MVP.
- Use least privilege for plugins, MCP servers, filesystem access, and subprocess execution.
- Pin and review dependencies; generate an SBOM for release builds when practical.
- Validate all external data and tool outputs as untrusted input.
- Restrict network destinations for tools where feasible.
- Add a kill switch, audit trail, and visible execution mode.

## Trading risk controls
- Max order notional.
- Max position/exposure by instrument and portfolio.
- Max daily realized/unrealized loss threshold.
- Max open orders and rate of order submission.
- Allowed instrument/venue allowlist.
- Minimum liquidity / maximum spread and slippage thresholds.
- Data freshness requirement.
- Duplicate-order/idempotency protection.
- Circuit breaker after repeated rejects, disconnects, or abnormal fills.
- Reconciliation before resuming after an uncertain execution state.

## AI-specific controls
- The model is not a trusted policy engine.
- Never rely on a model-generated confidence number as calibrated probability.
- Keep hard limits in deterministic code and configuration.
- Use tool allowlists, timeouts, call budgets, and output validation.
- Prompt injection from news, token descriptions, websites, and social posts must be treated as untrusted content.
- Require human confirmation for live mode activation and other high-impact settings.
- Log rationale and tool traces with secret redaction.

## Meme coin risk policy
- Unknown risk data is not equivalent to low risk.
- Hard-risk flags should exclude a token or require explicit override in a later release.
- Show liquidity and potential price impact, not just market cap.
- Warn when ownership/contract analysis is unavailable.
- No “guaranteed pump” or “safe token” language.
- Screener output is research support, not financial advice or a guarantee of outcome.

## Incident response
1. Stop new order intents.
2. Preserve logs and event IDs.
3. Reconcile venue state if applicable.
4. Rotate affected credentials.
5. Identify root cause and add regression tests before re-enabling.
