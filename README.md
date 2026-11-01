# Agentic Trading OS

**Local-first trading research desktop** — Rust-native market data, MT5 telemetry, paper trading, and fail-closed risk controls in a Svelte 5 / Tauri 2 shell.

[![npm test](https://img.shields.io/badge/npm%20test-375%20passing-brightgreen)](https://github.com)
[![cargo test](https://img.shields.io/badge/cargo%20test-239%20passing-brightgreen)](https://github.com)
[![build](https://img.shields.io/badge/build-passing-brightgreen)](https://github.com)
[![not certified](https://img.shields.io/badge/status-not%20production--certified-amber)](https://github.com)

---

## What it is

Agentic Trading OS is a desktop application for **trading research and paper execution**. It runs entirely on your machine — no cloud account, no remote execution, no live order submission. Market data comes from authenticated MT5 and public Binance caches; every price, candle, and order snapshot is validated before it reaches your screen.

**The app is research/paper-only.** It has no live execution authority and will never submit or mutate broker orders.

---

## Why it's different

| Most trading tools | Agentic Trading OS |
|---|---|
| Cloud-first, remote execution | Local-first, nothing leaves your machine |
| Live order execution by default | Read-only; paper trading only, no order submission |
| Synthetic demo data | Real MT5 + Binance data, validated freshness |
| Fail-open on bad data | Fail-closed: stale, NaN, or inconsistent data is rejected |
| No audit trail | SHA-256 hash-chained SQLite state commits |

---

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                    Svelte 5 / Tauri 2 UI                    │
│  Dashboard • Market Data • MT5 Bridge • Agent Runtime       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Tauri IPC commands (typed, read-only)          │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   domain/    │    │ application/ │    │ runtime/     │
│  models,     │    │ backtest,    │    │ trading_core │
│  market data │    │ risk sizing, │    │ state engine │
│  state       │    │ strict gate  │    │ SQLite       │
└──────────────┘    └──────────────┘    └──────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│ infrastructure/   adapters/tauri/ │
│ providers, MT5   commands, bootstrap│
│ bridge, keyring  │                    │
└──────────────┘    └──────────────┘    └──────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│  MT5 (read-only telemetry)  •  Binance (public cache)      │
│  keyring (OS credential store)  •  SQLite (audit chain)   │
└─────────────────────────────────────────────────────────────┘
```

---

## How to use

### Prerequisites

- Node.js ≥ 20
- Bun (for the Hono research API)
- Rust 1.99.1 (for the native backend)

### Install & build

```bash
# 1. Install all dependencies (frozen lockfiles)
npm ci --no-audit --fund=false
bun install --frozen-lockfile

# 2. Build the frontend (Svelte + Vite)
npm run build

# 3. Verify everything
npm test              # 375 tests, 52 suites
cargo test --all-targets   # 239 Rust tests
npm run verify:repo   # repository guardrails
```

### Run in development

```bash
# Frontend dev server
npm run dev

# Research API (Bun/Hono)
bun run api/src/server.ts
```

---

## Verification status (2026-11-01)

| Gate | Command | Result |
|---|---|---|
| Frontend build | `npm run build` | Passes (3434 modules) |
| Typecheck layers | `npm run typecheck:layers` | Passes |
| Rust format | `cargo fmt --check` | Clean |
| Rust clippy | `cargo clippy --all-targets` | No errors |
| Rust tests | `cargo test --all-targets` | 239 passed |
| Node tests | `npm test` | 375 passed / 52 suites |
| Repo guardrails | `npm run verify:repo` | Passes |
| Release preflight | `npm run release:preflight` | Passes |

**Lockfiles present:** `package-lock.json`, `bun.lock`, `src-tauri/Cargo.lock`

**External gates (not available in this workspace):** MetaEditor MQL5 compilation, real MT5 terminal, broker acceptance. The app cannot be production-certified until these pass on the appropriate platform.

---

## What's included

- **Rust-native market data platform** — provider registry, canonical models, typed result provenance, freshness anchoring
- **MT5 bridge (read-only)** — authenticated telemetry, durable receipts, per-series cursors, historical-series inspector
- **Account-wide risk control plane** — margin/stop-out policy, position and pending-order risk, 5% account-wide stop-risk ceiling, fail-closed sizing
- **Backtesting engine** — next-bar-open execution, out-of-sample windows, Sharpe annualization, dataset SHA-256 fingerprints
- **AI research runtime** — allowlisted providers (OpenAI, Anthropic, OpenRouter, Ollama), OS keyring storage, commentary-only output, no execution authority
- **Durable state integrity** — SHA-256 hash-chained SQLite commits, tamper-evident audit log
- **Svelte 5 / Tauri 2 desktop** — dashboard, market data panel, MT5 bridge panel, agent runtime panel

---

## Limits & safety

- **No live order execution.** The MT5 EA is read-only and does not submit or mutate orders.
- **Not production-certified.** Do not use real trading account credentials or enable live execution until all gates pass on pinned CI.
- **Local inference only.** GGUF/ONNX/SafeTensors imports are capped, hashed, and format-checked — never executed.
- **API is loopback-only.** The Bun/Hono research API binds to `127.0.0.1` and rejects cross-site browser origins.

---

## Documentation

| Topic | Document |
|---|---|
| Product requirements | `docs/01_PRD.md` |
| System architecture | `docs/03_SYSTEM_ARCHITECTURE.md` |
| Security & trading risk | `docs/08_SECURITY_AND_TRADING_RISK.md` |
| Testing & acceptance | `docs/09_TESTING_AND_ACCEPTANCE.md` |
| Roadmap | `docs/10_ROADMAP.md` |
| Release attestation pipeline | `docs/RELEASE_ATTESTATION_PIPELINE_0.9.6.md` |
| MT5 market data assurance | `docs/MT5_MARKET_DATA_ASSURANCE_AND_QUERY.md` |
| Archive (v0.5–v0.11 reports) | `docs/archive/` |

---

## License

See `SECURITY.md` for security policy and disclosure process.
