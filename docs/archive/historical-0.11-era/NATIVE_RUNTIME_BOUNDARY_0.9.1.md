# Native runtime boundary and evidence consistency — v0.9.1

## Runtime boundary

All application capabilities that depend on local account state or trusted observations require the installed Tauri runtime. The browser bridge raises `NATIVE_RUNTIME_REQUIRED`; it never substitutes a synthetic market snapshot, chart candles, screener rows, agent sessions, or audit events. `App.svelte` does not mount the trading workspace without the native runtime.

## Startup and state migration

New/recovered local paper state starts with empty quote, candle, screener and agent collections, while the paper-account cash balance is the configured paper capital, not a market observation. The kill switch starts engaged. Before exposing persisted state, startup removes known legacy bundled-synthetic provider observations and placeholder agent IDs; when cleanup changes state, the kill switch is engaged and a migration event is recorded. If a newer schema is detected, startup refuses to overwrite it.

This migration is marker-based against known legacy source/status labels; it is not a general content classifier for arbitrary malicious data. Persistent-state integrity and the UI are not a substitute for access control over the local application data directory.

## AI evidence and freshness

Every native evidence record includes its native-derived structured payload, serialized payload bytes and SHA-256. The JS boundary checks the exact payload bytes, parsed/canonical semantic payload, decision-bearing fields and ordered envelope before invoking the canonical registry and decision engine. After provider processing, Rust re-reads the current candle context and rejects the report if freshness, dataset fingerprint, timestamp, source, scope, or candle count changed. Provider narrative is still commentary-only.

Hashes detect inconsistent bytes but do not prove provider authenticity, source truth, or statistical independence. Market snapshots are not digitally signed.

## Truthful presentation and order boundary

Portfolio trend illustrations that had no historical source were removed. Chart time labels derive from loaded candles. Missing quotes are rendered unavailable, not as zero dollars; an order uses only the quote for its selected symbol and the CTA requires a live/fresh quote plus a positive finite quantity. One shared quote-freshness predicate gates the order selector, market-data table, watchlist, and top-performer ranking: provider status must be connected, the quote must be live with a finite positive price, its timestamp must not be in the future, and age must be at most 120 seconds. Closed-trade win rate is represented as `null` until there is a denominator; backtest win rate follows the same rule. Backtest output also includes gross profit and gross loss, letting the UI distinguish a truly unbounded profit factor (positive gross profit and no losses) from an undefined one (no non-zero PnL). The native Rust risk gateway remains authoritative and the kill switch stays engaged on fresh account initialization. Risk-increasing orders are additionally blocked when any existing position lacks a positive, live quote no older than 120 seconds; a stored notional fallback must not be used to pass a new-risk decision. Risk-reducing sells remain available if their own instrument passes its quote and inventory checks. Dashboard equity, day change, unrealized PnL, position marks, exposure percentages and the current-price label are shown as unavailable while relevant open positions or the selected chart quote lack a fresh quote; stale persisted marks are not presented as current valuation. A failed provider refresh transitions the engine provider status away from `CONNECTED`; cached quote age alone cannot authorize a new paper-order intent, even if the cached quote is under 120 seconds old. Dashboard equity, day change, unrealized PnL, position marks and exposure percentages are presented as unavailable while any open position lacks a fresh quote, to avoid promoting stale persisted values as current market valuation.

## Verification limits

The Node tests and JS/TS layer typecheck can validate source contracts, but they do not compile Rust or Svelte. Production certification still requires locked dependency resolution, `svelte-check`/Vite build, `cargo fmt`, native Rust tests and Clippy, provider/keychain integration tests on supported OSes, and signed desktop packaging. Live execution is not implemented or authorized.
