# 07 — API and Python SDK Contract (Draft)

**Implementation status:** there is no remote HTTP control API and no shipped Python SDK. The current desktop app uses local Tauri IPC commands and a Rust-only trading runtime. Everything below is a future contract proposal, not a claim about implemented endpoints. Python remains optional until a concrete research need justifies the extra runtime and security boundary.

## API design principles
- Version public endpoints (`/api/v1/...`).
- Typed request/response schemas and stable error codes.
- Idempotency for state-changing commands.
- Read endpoints may return freshness and source metadata.
- Commands return operation IDs when execution is asynchronous.
- Authentication and authorization are required before remote deployment; local-only mode still protects secrets and destructive actions.

## Candidate endpoints
- `GET /api/v1/health`
- `GET /api/v1/market/instruments`
- `GET /api/v1/market/candles?instrument_id=...&timeframe=...`
- `GET /api/v1/strategies`
- `POST /api/v1/strategies`
- `POST /api/v1/strategies/{id}/runs`
- `POST /api/v1/backtests`
- `GET /api/v1/backtests/{id}`
- `GET /api/v1/agents`
- `POST /api/v1/agents/{id}/start`
- `POST /api/v1/agents/{id}/stop`
- `GET /api/v1/orders`
- `GET /api/v1/positions`
- `GET /api/v1/screener/tokens`
- `GET /api/v1/screener/tokens/{chain}/{address}`
- `POST /api/v1/watchlists`
- `POST /api/v1/alerts`

These are proposed contracts, not implemented endpoints.

## Python strategy contract
A strategy should receive typed market events and return typed signals or order intents. It should not receive exchange API secrets or make direct network calls to order endpoints.

Illustrative interface:

```python
class Strategy:
    def on_start(self, context): ...
    def on_market_event(self, event, context):
        # Return zero or more typed signals/order intents.
        return []
    def on_stop(self, context): ...
```

## Agent tool contract
Each tool declares:
- stable tool name and version;
- input/output schema;
- required permission;
- timeout and resource budget;
- whether it is read-only or mutating;
- audit requirements.

Default-deny all tools. The model can only invoke explicitly allowed tools. Order placement is not exposed as a direct unrestricted model tool; agents submit proposals to the core risk gateway.

## Error categories
- `VALIDATION_ERROR`
- `NOT_FOUND`
- `CONFLICT`
- `RATE_LIMITED`
- `PROVIDER_UNAVAILABLE`
- `STALE_DATA`
- `RISK_REJECTED`
- `PERMISSION_DENIED`
- `EXECUTION_UNKNOWN`
- `INTERNAL_ERROR`
