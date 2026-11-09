use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::{
    domain::{
        market_data_platform::{
            MarketDataAuthority, MarketDataBar, MarketDataCapability, MarketDataFreshness,
            MarketDataGap, MarketDataProvenance, MarketDataProviderCatalogEntry,
            MarketDataProviderState, MarketDataPurpose, MarketDataQuality, MarketDataQuery,
            MarketDataRequestSummary, MarketDataVolumeBasis, UnifiedMarketDataResult,
            MARKET_DATA_PLATFORM_SCHEMA_VERSION,
        },
        models::{latest_closed_bar_at_ms, CandleInterval, CandleSeriesRequest},
    },
    infrastructure::mt5::{bridge::Mt5Bridge, market_data::Mt5SeriesQuery},
    runtime::trading_core::TradingCore,
};

const MT5_PROVIDER_ID: &str = "mt5-terminal";
const BINANCE_PROVIDER_ID: &str = "binance-public";
const MAX_CLOCK_SKEW_MS: i64 = 60_000;

pub struct MarketDataProviderContext<'a> {
    pub core: &'a TradingCore,
    pub bridge: &'a Mt5Bridge,
}

trait NativeMarketDataProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn catalog_entry(
        &self,
        context: &MarketDataProviderContext<'_>,
    ) -> MarketDataProviderCatalogEntry;
    fn query(
        &self,
        context: &MarketDataProviderContext<'_>,
        request: &MarketDataQuery,
    ) -> Result<UnifiedMarketDataResult, String>;
}

struct Mt5TerminalProvider;
struct BinancePublicCacheProvider;

struct NativeMarketDataProviderRegistry {
    providers: BTreeMap<&'static str, Box<dyn NativeMarketDataProvider>>,
}

impl NativeMarketDataProviderRegistry {
    fn built_in() -> Result<Self, String> {
        let mut registry = Self {
            providers: BTreeMap::new(),
        };
        registry.register(Box::new(Mt5TerminalProvider))?;
        registry.register(Box::new(BinancePublicCacheProvider))?;
        Ok(registry)
    }

    fn register(&mut self, provider: Box<dyn NativeMarketDataProvider>) -> Result<(), String> {
        let id = provider.id();
        if id.is_empty()
            || id.len() > 48
            || !id.chars().all(|value| {
                value.is_ascii_lowercase() || value.is_ascii_digit() || matches!(value, '-' | '_')
            })
        {
            return Err(
                "A native market-data provider has an invalid registration identifier.".into(),
            );
        }
        if self.providers.contains_key(id) {
            return Err(format!(
                "Market-data provider '{id}' is already registered."
            ));
        }
        self.providers.insert(id, provider);
        Ok(())
    }

    fn catalog(
        &self,
        context: &MarketDataProviderContext<'_>,
    ) -> Vec<MarketDataProviderCatalogEntry> {
        self.providers
            .values()
            .map(|provider| provider.catalog_entry(context))
            .collect()
    }

    fn query(
        &self,
        context: &MarketDataProviderContext<'_>,
        request: MarketDataQuery,
    ) -> Result<UnifiedMarketDataResult, String> {
        let request = request.validate_and_normalize()?;
        let provider_id = request
            .provider_id
            .as_deref()
            .unwrap_or(BINANCE_PROVIDER_ID);
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| format!("Market-data provider '{provider_id}' is not registered; no fallback provider was selected."))?;
        provider.query(context, &request)
    }
}

pub fn provider_catalog(
    core: &TradingCore,
    bridge: &Mt5Bridge,
) -> Result<Vec<MarketDataProviderCatalogEntry>, String> {
    let registry = NativeMarketDataProviderRegistry::built_in()?;
    let context = MarketDataProviderContext { core, bridge };
    Ok(registry.catalog(&context))
}

pub fn query_market_data(
    core: &TradingCore,
    bridge: &Mt5Bridge,
    request: MarketDataQuery,
) -> Result<UnifiedMarketDataResult, String> {
    let registry = NativeMarketDataProviderRegistry::built_in()?;
    let context = MarketDataProviderContext { core, bridge };
    registry.query(&context, request)
}

impl NativeMarketDataProvider for Mt5TerminalProvider {
    fn id(&self) -> &'static str {
        MT5_PROVIDER_ID
    }

    fn catalog_entry(
        &self,
        context: &MarketDataProviderContext<'_>,
    ) -> MarketDataProviderCatalogEntry {
        let status = context.bridge.status();
        let storage = status.market_data_storage.as_ref();
        let state = if status.connected
            && !status.stale
            && storage.is_some_and(|value| value.storage_available)
        {
            MarketDataProviderState::Connected
        } else if storage.is_some_and(|value| value.storage_available && value.bars_total > 0) {
            MarketDataProviderState::Cached
        } else {
            MarketDataProviderState::Disconnected
        };
        MarketDataProviderCatalogEntry {
            id: MT5_PROVIDER_ID.into(),
            display_name: "MetaTrader 5 terminal".into(),
            authority: MarketDataAuthority::BrokerTerminal,
            venue: status.account_server.unwrap_or_else(|| "Authenticated broker account".into()),
            capabilities: vec![
                MarketDataCapability::CandleHistory,
                MarketDataCapability::InstrumentSpecification,
                MarketDataCapability::SessionCalendar,
            ],
            supported_purposes: vec![MarketDataPurpose::Research, MarketDataPurpose::Backtest, MarketDataPurpose::BrokerContext],
            state,
            observed_at_ms: storage.and_then(|value| value.last_ingest_at_ms),
            limitations: vec![
                "Historical closed bars are account-bound to the last authenticated terminal identity.".into(),
                "Broker context requires a connected, non-stale terminal; stored history alone is not execution authority.".into(),
                "Volume preserves broker tick-count and real-volume fields; it is not normalized to exchange base-asset units.".into(),
            ],
        }
    }

    fn query(
        &self,
        context: &MarketDataProviderContext<'_>,
        request: &MarketDataQuery,
    ) -> Result<UnifiedMarketDataResult, String> {
        let initial_bridge_status = context.bridge.status();
        let broker_context_sequence = if request.purpose == MarketDataPurpose::BrokerContext {
            if !initial_bridge_status.connected || initial_bridge_status.stale {
                return Err(
                    "Broker context requires a connected, non-stale MT5 terminal heartbeat.".into(),
                );
            }
            Some(initial_bridge_status.last_sequence)
        } else {
            None
        };
        let (broker_server, account_login) = context.bridge.authenticated_account_identity()?;
        let response = context.bridge.market_data_series_query(Mt5SeriesQuery {
            broker_server: broker_server.clone(),
            account_login: account_login.clone(),
            symbol: request.symbol.clone(),
            timeframe: request.timeframe.clone(),
            from_ms: request.from_ms,
            to_ms: request.to_ms,
            after_ms: request.after_ms,
            limit: request.limit,
        })?;
        if response.bars.is_empty() {
            return Err("MT5 returned no bars for the exact requested account, symbol, timeframe, and range.".into());
        }
        if let Some(sequence) = broker_context_sequence {
            let confirmed = context.bridge.status();
            let confirmed_identity = context.bridge.authenticated_account_identity()?;
            if !confirmed.connected
                || confirmed.stale
                || confirmed.last_sequence != sequence
                || confirmed_identity.0 != broker_server
                || confirmed_identity.1 != account_login
            {
                return Err("MT5 account identity or snapshot changed during the broker-context query; the result was discarded.".into());
            }
        }

        let now = Utc::now().timestamp_millis();
        let latest_received_at_ms = response
            .bars
            .iter()
            .map(|bar| bar.last_received_at_ms)
            .max();
        let latest_observation_at_ms = response.quality.last_bar_time_ms;
        let (freshness, freshness_age_ms) =
            freshness(latest_observation_at_ms, now, request.max_data_age_ms)?;
        enforce_freshness(freshness, request.allow_stale, request.purpose)?;
        let bars: Vec<MarketDataBar> = response
            .bars
            .into_iter()
            .map(|bar| {
                let has_real_volume = bar.real_volume > 0;
                MarketDataBar {
                    time_ms: bar.time_ms,
                    open: bar.open,
                    high: bar.high,
                    low: bar.low,
                    close: bar.close,
                    volume: if has_real_volume {
                        bar.real_volume as f64
                    } else {
                        bar.tick_volume as f64
                    },
                    volume_basis: if has_real_volume {
                        MarketDataVolumeBasis::BrokerRealVolume
                    } else {
                        MarketDataVolumeBasis::BrokerTickCount
                    },
                    tick_volume: Some(bar.tick_volume),
                    real_volume: Some(bar.real_volume),
                    revision: Some(bar.revision),
                }
            })
            .collect();
        let has_more = response.has_more;
        let next_after_ms = response.next_after_ms;
        let quality = MarketDataQuality {
            freshness,
            freshness_age_ms,
            freshness_limit_ms: request.max_data_age_ms,
            candle_count: bars.len(),
            revision_count: response.quality.revisions_total,
            interval_anomaly_count: response.quality.interval_anomaly_count,
            gap_candidates: response
                .quality
                .gap_candidates
                .into_iter()
                .map(|gap| MarketDataGap {
                    previous_bar_time_ms: gap.previous_bar_time_ms,
                    next_bar_time_ms: gap.next_bar_time_ms,
                    elapsed_ms: gap.elapsed_ms,
                    nominal_intervals_elapsed: gap.nominal_intervals_elapsed,
                })
                .collect(),
            has_more,
            continuity_interpretation: response.quality.continuity_interpretation,
        };
        Ok(UnifiedMarketDataResult {
            schema_version: MARKET_DATA_PLATFORM_SCHEMA_VERSION,
            request: request_summary(request, MT5_PROVIDER_ID),
            provenance: MarketDataProvenance {
                provider_id: MT5_PROVIDER_ID.into(),
                provider_name: "MetaTrader 5 terminal".into(),
                authority: MarketDataAuthority::BrokerTerminal,
                venue: broker_server,
                retrieved_at_ms: now,
                latest_observation_at_ms,
                latest_received_at_ms,
                dataset_fingerprint_sha256: response.dataset_fingerprint_sha256,
                scope_fingerprint_sha256: None,
                is_live: initial_bridge_status.connected && !initial_bridge_status.stale && freshness == MarketDataFreshness::Fresh,
                source_details: "Durable MT5 closed-bar history. The authenticated account identity is validated inside the native bridge and is not returned in this normalized result.".into(),
            },
            quality,
            bars,
            next_after_ms,
        })
    }
}

impl NativeMarketDataProvider for BinancePublicCacheProvider {
    fn id(&self) -> &'static str {
        BINANCE_PROVIDER_ID
    }

    fn catalog_entry(
        &self,
        context: &MarketDataProviderContext<'_>,
    ) -> MarketDataProviderCatalogEntry {
        let cached = context.core.get_candle_series(CandleSeriesRequest {
            symbol: "BTCUSDT".into(),
            interval: CandleInterval::FiveMinutes,
        });
        let (state, observed_at_ms) = match cached {
            Ok(series) if series.is_live => (
                MarketDataProviderState::Connected,
                Some(series.retrieved_at_ms),
            ),
            Ok(series) => (
                MarketDataProviderState::Cached,
                Some(series.retrieved_at_ms),
            ),
            Err(_) => (MarketDataProviderState::NotLoaded, None),
        };
        MarketDataProviderCatalogEntry {
            id: BINANCE_PROVIDER_ID.into(),
            display_name: "Binance public spot cache".into(),
            authority: MarketDataAuthority::PublicExchange,
            venue: "Binance Spot".into(),
            capabilities: vec![MarketDataCapability::QuoteSnapshot, MarketDataCapability::CandleHistory],
            supported_purposes: vec![MarketDataPurpose::Research, MarketDataPurpose::Backtest],
            state,
            observed_at_ms,
            limitations: vec![
                "The native candle cache currently supports BTCUSDT M5 only; other requests are rejected without symbol or timeframe substitution.".into(),
                "Public exchange observations are research/backtest inputs and are never accepted as broker context.".into(),
                "Provider data terms govern storage and redistribution; the application does not grant exchange data rights.".into(),
            ],
        }
    }

    fn query(
        &self,
        context: &MarketDataProviderContext<'_>,
        request: &MarketDataQuery,
    ) -> Result<UnifiedMarketDataResult, String> {
        if request.purpose == MarketDataPurpose::BrokerContext {
            return Err("Public exchange data cannot satisfy a broker-context query.".into());
        }
        if request.symbol != "BTCUSDT" || request.timeframe != "M5" {
            return Err("Binance public cache supports BTCUSDT M5 only; no substitute dataset was returned.".into());
        }
        let series = context.core.get_candle_series(CandleSeriesRequest {
            symbol: request.symbol.clone(),
            interval: CandleInterval::FiveMinutes,
        })?;
        let dataset: Vec<_> = series
            .candles
            .into_iter()
            .filter(|candle| candle.time >= request.from_ms && candle.time <= request.to_ms)
            .collect();
        if dataset.is_empty() {
            return Err("Binance cache contains no candles in the exact requested range.".into());
        }
        let latest_observation_at_ms =
            latest_closed_bar_at_ms(&dataset, CandleInterval::FiveMinutes).ok();
        let canonical = serde_json::to_vec(&dataset).map_err(|error| {
            format!("Unable to serialize the complete Binance query dataset: {error}")
        })?;
        let mut dataset_hasher = Sha256::new();
        dataset_hasher.update(b"ATOS-BINANCE-QUERY-DATASET-V1");
        for part in [request.symbol.as_bytes(), request.timeframe.as_bytes()] {
            dataset_hasher.update((part.len() as u64).to_be_bytes());
            dataset_hasher.update(part);
        }
        dataset_hasher.update(request.from_ms.to_be_bytes());
        dataset_hasher.update(request.to_ms.to_be_bytes());
        dataset_hasher.update(&canonical);
        let fingerprint = dataset_hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let eligible: Vec<_> = dataset
            .into_iter()
            .filter(|candle| request.after_ms.is_none_or(|cursor| candle.time > cursor))
            .collect();
        if eligible.is_empty() {
            return Err(
                "Binance dataset has no candles after the requested pagination cursor.".into(),
            );
        }
        let has_more = eligible.len() > request.limit;
        let page: Vec<_> = eligible.into_iter().take(request.limit).collect();
        let bars: Vec<MarketDataBar> = page
            .into_iter()
            .map(|candle| MarketDataBar {
                time_ms: candle.time,
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: candle.volume,
                volume_basis: MarketDataVolumeBasis::BaseAsset,
                tick_volume: None,
                real_volume: None,
                revision: None,
            })
            .collect();
        let now = Utc::now().timestamp_millis();
        let (freshness, freshness_age_ms) =
            freshness(latest_observation_at_ms, now, request.max_data_age_ms)?;
        enforce_freshness(freshness, request.allow_stale, request.purpose)?;
        let next_after_ms = if has_more {
            bars.last().map(|bar| bar.time_ms)
        } else {
            None
        };
        Ok(UnifiedMarketDataResult {
            schema_version: MARKET_DATA_PLATFORM_SCHEMA_VERSION,
            request: request_summary(request, BINANCE_PROVIDER_ID),
            provenance: MarketDataProvenance {
                provider_id: BINANCE_PROVIDER_ID.into(),
                provider_name: "Binance public spot cache".into(),
                authority: MarketDataAuthority::PublicExchange,
                venue: "Binance Spot".into(),
                retrieved_at_ms: now,
                latest_observation_at_ms,
                latest_received_at_ms: Some(series.retrieved_at_ms),
                dataset_fingerprint_sha256: fingerprint,
                scope_fingerprint_sha256: None,
                is_live: series.is_live && freshness == MarketDataFreshness::Fresh,
                source_details: format!("{}; {}", series.source, series.provider_status),
            },
            quality: MarketDataQuality {
                freshness,
                freshness_age_ms,
                freshness_limit_ms: request.max_data_age_ms,
                candle_count: bars.len(),
                revision_count: 0,
                interval_anomaly_count: 0,
                gap_candidates: Vec::new(),
                has_more,
                continuity_interpretation: "Canonical BTCUSDT M5 series passed native OHLCV cadence validation before caching.".into(),
            },
            bars,
            next_after_ms,
        })
    }
}

fn request_summary(request: &MarketDataQuery, provider_id: &str) -> MarketDataRequestSummary {
    MarketDataRequestSummary {
        symbol: request.symbol.clone(),
        timeframe: request.timeframe.clone(),
        from_ms: request.from_ms,
        to_ms: request.to_ms,
        after_ms: request.after_ms,
        limit: request.limit,
        provider_id: provider_id.into(),
        purpose: request.purpose,
        max_data_age_ms: request.max_data_age_ms,
        allow_stale: request.allow_stale,
    }
}

fn freshness(
    observation_at_ms: Option<i64>,
    now_ms: i64,
    maximum_age_ms: i64,
) -> Result<(MarketDataFreshness, Option<i64>), String> {
    let Some(observation_at_ms) = observation_at_ms else {
        return Ok((MarketDataFreshness::Unavailable, None));
    };
    if observation_at_ms > now_ms.saturating_add(MAX_CLOCK_SKEW_MS) {
        return Err("Market-data provider timestamp is too far in the future; clock-skew protection rejected it.".into());
    }
    let age_ms = now_ms.saturating_sub(observation_at_ms).max(0);
    let state = if age_ms <= maximum_age_ms {
        MarketDataFreshness::Fresh
    } else {
        MarketDataFreshness::Stale
    };
    Ok((state, Some(age_ms)))
}

fn enforce_freshness(
    state: MarketDataFreshness,
    allow_stale: bool,
    purpose: MarketDataPurpose,
) -> Result<(), String> {
    if state == MarketDataFreshness::Fresh {
        return Ok(());
    }
    if state == MarketDataFreshness::Stale
        && allow_stale
        && purpose != MarketDataPurpose::BrokerContext
    {
        return Ok(());
    }
    Err(format!("Market-data freshness gate rejected the provider response ({state:?}); only known-stale observations may be explicitly allowed for research/backtest."))
}

#[cfg(test)]
mod tests {
    use super::{enforce_freshness, freshness};
    use crate::domain::market_data_platform::{MarketDataFreshness, MarketDataPurpose};

    #[test]
    fn freshness_gate_detects_stale_and_future_dated_observations() {
        assert_eq!(
            freshness(Some(90_000), 100_000, 15_000).unwrap(),
            (MarketDataFreshness::Fresh, Some(10_000))
        );
        assert_eq!(
            freshness(Some(80_000), 100_000, 5_000).unwrap(),
            (MarketDataFreshness::Stale, Some(20_000))
        );
        assert!(freshness(Some(200_000), 100_000, 5_000).is_err());
    }

    #[test]
    fn freshness_is_derived_from_market_observation_not_recent_cache_ingestion() {
        let now = 200_000_000;
        let old_market_observation = 100_000_000;
        let recent_ingestion = now - 1_000;
        let (state, age) = freshness(Some(old_market_observation), now, 600_000).unwrap();
        assert_eq!(state, MarketDataFreshness::Stale);
        assert_eq!(age, Some(100_000_000));
        assert!(recent_ingestion > old_market_observation);
    }

    #[test]
    fn stale_data_is_never_accepted_for_broker_context() {
        assert!(enforce_freshness(
            MarketDataFreshness::Stale,
            true,
            MarketDataPurpose::BrokerContext
        )
        .is_err());
        assert!(enforce_freshness(
            MarketDataFreshness::Stale,
            true,
            MarketDataPurpose::Research
        )
        .is_ok());
        assert!(enforce_freshness(
            MarketDataFreshness::Stale,
            false,
            MarketDataPurpose::Backtest
        )
        .is_err());
        assert!(enforce_freshness(
            MarketDataFreshness::Unavailable,
            true,
            MarketDataPurpose::Research
        )
        .is_err());
    }
}
