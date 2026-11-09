# 04 — Meme Coin Screener Specification

## Implemented baseline (2026-10-25)

The current desktop integration discovers tokens from GeckoTerminal new/trending pools and DEX Screener token profiles/latest boosts, then enriches a bounded set of recent addresses across Solana, Ethereum, Base, BNB Chain, Arbitrum, Polygon, Optimism, and Avalanche. It ranks a token only when the provider reports that token as the pair's base asset, because `priceNative` is denominated in the native asset rather than a universally valid quote-token price. Known major quote assets are excluded from discovery candidates. The screen records available 5m/1h/24h changes, volume, liquidity, market cap/FDV, pool age, transaction flow, source URL, observation time, and contract-security evidence where configured.

The discovery feeds are not an unbiased token universe. GoPlus contract screening is integrated, but unknown, incomplete, or mismatched holder/LP evidence blocks setup eligibility. The screen does not inspect social attention, wallet clusters, route simulation, executable depth, or transaction outcomes. Data-source failure retains the last good report and original observation timestamp while showing the scan error; partial provider coverage is labelled `PARTIAL`. Quote-side-only candidates are rejected rather than assigned a price derived from `priceNative`, whose units do not support that inference.

### Current heuristic version

`momentum-risk-v1` calculates momentum from bounded 5m/1h/24h price changes, a capped `ln(1 + 24h volume / displayed liquidity)` activity term, and a bounded buy/sell transaction-flow term when at least 20 transactions are represented. Risk starts from a conservative base and adds penalties for low liquidity, young/unknown pool age, extreme turnover, missing market cap, incomplete price windows, and missing buy/sell counts. Explicit warning flags always include promotion bias, unverified contract/holder/liquidity-lock status, and unmodeled executable depth.

The scores are **heuristics, not probabilities**, and are not fitted or validated out of sample. The screen must be used for triage only. Hard-risk exclusions and third-party contract-risk providers remain future work.

## Purpose
Identify tokens worth further investigation by ranking observable momentum and activity while filtering obvious risks. The screener is a discovery and triage tool, not a price oracle.

## Core screen layout
- Header: chain selector, search, refresh timestamp, data-health status.
- Filter bar: token age, minimum liquidity, volume, market cap/FDV, minimum pool age, holder concentration, exclude known risky patterns.
- Main table: token, chain, age, price change (5m/1h/24h), volume, liquidity, market cap/FDV, buy/sell imbalance, holder growth, momentum score, risk score, data freshness, action.
- Detail drawer: score breakdown, chart, liquidity/pool details, holder concentration, top-wallet changes, contract flags, social signals with provenance, known limitations.
- Actions: watchlist, alert, paper-test rule, copy token address. No one-click live buy in MVP.

## Candidate signals
### Momentum / activity
- Relative volume increase compared with the token's recent baseline.
- Buy/sell transaction imbalance, with wash-trading caveats.
- Price acceleration over multiple windows rather than a single green candle.
- Liquidity growth and spread quality.
- Unique active wallets and holder growth, if the source provides trustworthy data.
- Cross-source social attention, weighted by source reliability and bot/spam filtering.

### Risk signals
- Liquidity not locked or lock status unverifiable.
- Concentrated ownership, deployer holdings, or suspicious wallet clusters.
- Mint/freeze/blacklist/tax permissions where relevant to the chain.
- Honeypot or sell simulation failure where supported.
- Thin liquidity, extreme price impact, abnormal spreads, or shallow pool depth.
- Sudden liquidity removal or suspicious holder distribution.
- Very new pool/token with insufficient evidence.
- Missing, stale, contradictory, or low-confidence data.
- Suspected wash trading, sybil activity, or inorganic social bursts.

## Scoring model
Keep **Momentum Score (0–100)** and **Risk Score (0–100)** separate.

Illustrative initial momentum weights (calibrate using historical data before relying on them):
- Relative volume acceleration: 25%
- Multi-window price momentum: 20%
- Buy/sell imbalance: 15%
- Liquidity quality/growth: 15%
- Holder/activity growth: 10%
- Social attention quality: 10%
- Market regime/context: 5%

Risk score should be a separate aggregation of explicit red flags and missing evidence. Some severe risks should be hard exclusions rather than weighted penalties. Do not convert these scores into a “probability it will pump” without out-of-sample calibration.

## Suggested status labels
- **Watch:** interesting signal; insufficient confirmation.
- **Momentum:** several independent activity signals agree.
- **High Risk:** one or more significant risk flags.
- **Insufficient Data:** source coverage or freshness is inadequate.
- **Excluded:** hard-risk rule triggered.

## Required transparency
Every score should show:
- individual feature values and contribution;
- source/provider and observation timestamp;
- missing-data markers;
- model/ruleset version;
- explanation of triggered risk flags;
- warning that signals can fail and liquidity can disappear.

## Alerts
Alerts should be configurable and rate-limited. Example: relative volume exceeds user threshold AND liquidity exceeds minimum AND no hard-risk rule is triggered. Do not alert solely because price rose sharply.

## Validation
- Replay historical snapshots where available.
- Measure precision/recall for clearly defined outcomes, not anecdotal “good calls.”
- Include delisted/rugged tokens to reduce survivorship bias.
- Evaluate slippage, spread, fees, and exit liquidity.
- Track false positives, source outages, and data latency.

## MVP boundary
Start with one chain and one reliable data provider. If contract-risk data is unavailable, show “unknown,” never “safe.” Do not implement automated on-chain buying in the screener MVP.
