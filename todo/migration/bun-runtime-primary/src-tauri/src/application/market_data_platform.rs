use crate::{
    domain::market_data_platform::{
        MarketDataProviderCatalogEntry, MarketDataQuery, UnifiedMarketDataResult,
    },
    infrastructure::{mt5::bridge::Mt5Bridge, providers::registry},
    runtime::trading_core::TradingCore,
};

pub fn provider_catalog(
    core: &TradingCore,
    bridge: &Mt5Bridge,
) -> Result<Vec<MarketDataProviderCatalogEntry>, String> {
    registry::provider_catalog(core, bridge)
}

pub fn query(
    core: &TradingCore,
    bridge: &Mt5Bridge,
    request: MarketDataQuery,
) -> Result<UnifiedMarketDataResult, String> {
    registry::query_market_data(core, bridge, request)
}
