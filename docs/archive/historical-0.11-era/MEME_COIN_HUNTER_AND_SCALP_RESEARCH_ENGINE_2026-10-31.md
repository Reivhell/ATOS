# Meme Coin Hunter and Scalp Research Engine

**Implementation contract:** `meme-coin-hunter.v1`  
**Application version:** `0.11.0` (unchanged)  
**Execution authority:** none; discovery, risk screening and research ranking only.

## Scope

Meme Coin Hunter adds a native, multi-source discovery and short-horizon market-screening path to ATOS. It is intended to narrow a broad token universe into a ranked research queue for manual review. A `SETUP` label means only that the configured market and security heuristics passed for the captured observation. It is not a forecast, a probability of profit, a buy/sell instruction, a route quote, or authorization to trade.

The feature does not connect a wallet, derive or store private keys, sign transactions, submit swaps, approve token spend, or mutate any on-chain state. It has no transaction execution command.

## Data path

1. **Discovery:** four feeds are requested in parallel:
   - GeckoTerminal global new pools.
   - DEX Screener latest token profiles.
   - DEX Screener latest token boosts.
   - GeckoTerminal global trending pools.
2. **Identity validation:** network and token addresses are normalized against the supported chain registry. EVM addresses are validated as non-zero 20-byte hexadecimal values; Solana addresses must decode from base58 to exactly 32 bytes. GeckoTerminal base-token and quote-token relationships are both considered. Unsupported chain identifiers, malformed addresses and cross-network pool/token relationships are rejected.
3. **Market enrichment:** DEX Screener token-pair lookup is batched by chain, with at most 30 token addresses per request. For each token, the most liquid returned pair is selected. The candidate can be either the base or quote token; quote-side spot USD price is derived from the base USD price and native pair price, while price-change and directional buy/sell metrics remain unavailable because the feed does not provide a separately verified quote-token USD history or directional attribution. Quote-side candidates therefore cannot pass the momentum and buy-pressure scalp gates. Market-cap/FDV values are not attributed to the quote token.
4. **Security assessment:** one GoPlus request per supported chain is batched over the token identities with market pairs. Security evidence is associated with the exact token and selected pool. Missing fields, incomplete holder arrays, unverified LP attribution, API failures and missing credentials remain unknown/partial; they are never converted into a clean result.
5. **Scalp research gates:** short-window volume, transaction count, buy pressure, price movement, pool age, liquidity and security evidence determine `WATCH`, `SETUP` or `BLOCKED`.
6. **Persistence:** each successful report is serialized to a local SQLite history database. The database uses WAL mode, full synchronous commits and a 100-report retention limit. A SHA-256 fingerprint covers the canonical candidate dataset.

## Supported networks

Solana, Ethereum, Base, BNB Smart Chain, Arbitrum, Polygon, Optimism and Avalanche. The supported set is explicit; unknown network IDs are not guessed or silently remapped.

## Market screening profile

A candidate is eligible for `SETUP` only when all these configured gates pass:

| Gate | Current requirement |
|---|---:|
| Selected-pool liquidity | at least USD 100,000 |
| Five-minute quote volume | at least USD 15,000 |
| Five-minute volume velocity | 1.25x to 12x the average five-minute volume over the prior hour |
| Five-minute transactions | at least 40 combined buys and sells |
| Five-minute buy pressure | 56% to 82% |
| Five-minute price change | +0.25% to +7% |
| One-hour price change | -5% to +20% |
| Pool age | 1 hour to 30 days |
| Contract and pool security | complete verified evidence; no review-required conditions |
| Combined risk score | at most 35/100 |

A result failing one or more gates remains `WATCH`. A detected critical contract or token-control risk becomes `BLOCKED`. A candidate with unavailable or partial security evidence cannot become `SETUP` and its scalp score is capped at 40/100.

These thresholds are screening defaults, not optimized trading parameters or backtest-proven profit settings. They have not been calibrated against an exchange execution simulator, latency-adjusted fills, slippage, gas, MEV or realized outcomes.

## Security evidence rules

### EVM tokens

The GoPlus response is read using the documented EVM contract-security and trading-security fields. Honeypot, buy restriction, mint capability, reclaimable ownership, hidden owner, balance mutation, self-destruct, modifiable tax, pausable transfer, blacklist, personal slippage modification, airdrop scam and counterfeit-token detections are treated as critical flags. Proxy contracts, whitelist restrictions, cooldowns, anti-whale controls, partial-sell restrictions, external calls and additional potential-risk notes require review and prevent a verified setup.

GoPlus documents `lp_holders` at token-response level while DEX pools are listed separately. To avoid attributing a token-level LP snapshot to the wrong pool, the current gate accepts this evidence only when the response contains exactly one DEX pool and its pair address matches the selected market pair. Multiple or ambiguous pools remain partial.

The top-holder concentration calculation requires a complete top-ten list, or an explicit holder count no greater than the number of returned rows. Missing percentages, duplicate holder identities, impossible percentages or an incomplete list are treated as unknown. At least 40% top-ten concentration requires review; 70% or higher is blocked.

### Solana tokens

The parser uses the documented Solana fields, including `non_transferable`, `creator.malicious_address`, `mintable.status`, `freezable.status`, `closable.status`, transfer-hook, metadata mutability, balance-mutation authority and default-account state. Custom transfer hooks and mutable token controls require manual review.

Transfer fee screening considers both the current and scheduled fee rates. Fee rates are interpreted as parts per ten thousand and converted to percentage values. The higher known non-negative rate governs the gate; a scheduled increase is explicitly marked as review-required. If the current rate is missing, the scheduled rate is still used conservatively and the record remains partial. A current or scheduled fee of 10% or more is blocked. Invalid negative values are marked partial.

LP lock evidence must match the selected pool ID. The top-ten LP snapshot must contain ten rows, or a provider holder count must exactly match a shorter list; an incomplete list cannot prove locked liquidity. The top-ten LP locked percentage is a lower-bound observation based on returned LP holders, not a guarantee that all remaining liquidity is locked. At least 50% of total LP supply must be directly evidenced as locked for eligibility; missing or insufficient evidence remains partial.

## Operational safeguards

- One scan may run at a time; concurrent invocations are rejected.
- Refresh requests are limited to one scan per 60 seconds using a monotonic in-process clock and an atomic SQLite reservation, so restarting the desktop app does not bypass the cooldown. If the control database is unavailable or the wall clock moves backwards relative to the persisted reservation, scanning fails closed.
- HTTP redirects are disabled.
- Connection timeout is 4 seconds; total request timeout is 9 seconds.
- Provider response bodies are capped at 4 MiB, including streamed bodies without a reliable content-length header.
- GoPlus credentials are stored only in the operating-system credential vault and wrapped in zeroizing memory buffers, including per-chain request copies and the temporary Bearer value. The HTTP authorization header is marked sensitive. No plaintext configuration fallback exists; a missing or unavailable vault fails closed. Opaque printable ASCII token characters are accepted without imposing an undocumented token alphabet.
- Credentials are sent only by the native Rust process to the GoPlus API using the documented Bearer authorization header.
- Provider errors, unsupported chains, malformed responses and missing records are exposed as degraded/partial states instead of silently reusing old evidence as current. An unconfigured GoPlus provider has no fabricated security-check timestamp; an actual request that returns no token record is timestamped as an attempted but unavailable check.
- The most recent successful report remains available when a later scan fails; its original observation timestamp remains visible.

## Freshness and suitability

The discovery and market feeds are public REST endpoints, not a low-latency streaming feed. The report deliberately states that it is not suitable for sub-second execution. GeckoTerminal's current public API documentation states an approximate limit of 10 calls per minute; GoPlus support documents a 30-calls-per-minute default for its free Security API; DEX Screener documents 60 requests/minute for profile/boost feeds and 300 requests/minute for pair/token lookup, with up to 30 token addresses in the multi-token request. The 60-second scan lease and per-chain batching are designed to stay within the public limits during one desktop user's scan; this does not provide an SLA or guaranteed capacity for multi-user deployment.

A production trading workflow would still need independently verified, low-latency trade/quote data; exact router/venue quote; price-impact and depth checks; gas and fee estimates; transaction simulation; wallet/position risk controls; MEV-aware execution; kill-switch policy; and broker/venue acceptance. Those are intentionally not represented as implemented here.

## External API references

- DEX Screener API reference: https://docs.dexscreener.com/api/reference
- GeckoTerminal API documentation: https://api.geckoterminal.com/docs/index.html
- CoinGecko support note on GeckoTerminal public API rate limits: https://support.coingecko.com/articles/6867086195-does-geckoterminal-have-an-api
- GoPlus EVM token-security endpoint: https://docs.gopluslabs.io/reference/tokensecurityusingget_1
- GoPlus Solana token-security endpoint: https://docs.gopluslabs.io/reference/solanatokensecurityusingget
- GoPlus EVM response details: https://docs.gopluslabs.io/reference/response-details
- GoPlus Solana response details: https://docs.gopluslabs.io/reference/response-detail-1
- GoPlus rate-limit support: https://docs.gopluslabs.io/reference/support

## Release status

The source implementation, UI contract and Node repository regressions are present. Rust unit tests for provider parsing and security gates are included in the native module, but this environment does not contain `cargo`, `rustc`, `rustfmt` or Clippy; those tests have not been compiled or executed here. The Svelte production build also requires the project toolchain to be installed. No live provider acceptance test, target-OS keyring validation or broker/venue execution acceptance was possible in this environment.

Accordingly, this increment is an implemented research/screening capability with passing available checks, not a certified production trading/execution release. Application version remains `0.11.0`.
