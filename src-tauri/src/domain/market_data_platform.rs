use serde::{Deserialize, Serialize};

pub const MARKET_DATA_PLATFORM_SCHEMA_VERSION: u16 = 1;
pub const MAX_MARKET_DATA_RANGE_MS: i64 = 366 * 24 * 60 * 60 * 1_000;
pub const MAX_MARKET_DATA_PAGE_SIZE: usize = 5_000;
pub const MAX_MARKET_DATA_AGE_MS: i64 = 7 * 24 * 60 * 60 * 1_000;

fn default_purpose() -> MarketDataPurpose {
    MarketDataPurpose::Research
}
fn default_max_data_age_ms() -> i64 {
    300_000
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarketDataPurpose {
    #[default]
    Research,
    Backtest,
    BrokerContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarketDataAuthority {
    BrokerTerminal,
    PublicExchange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarketDataProviderState {
    Connected,
    Cached,
    Disconnected,
    NotLoaded,
    Degraded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarketDataFreshness {
    Fresh,
    Stale,
    FutureDated,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarketDataCapability {
    QuoteSnapshot,
    CandleHistory,
    InstrumentSpecification,
    SessionCalendar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MarketDataVolumeBasis {
    BaseAsset,
    BrokerTickCount,
    BrokerRealVolume,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketDataQuery {
    pub symbol: String,
    pub timeframe: String,
    pub from_ms: i64,
    pub to_ms: i64,
    #[serde(default)]
    pub after_ms: Option<i64>,
    pub limit: usize,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default = "default_purpose")]
    pub purpose: MarketDataPurpose,
    #[serde(default = "default_max_data_age_ms")]
    pub max_data_age_ms: i64,
    #[serde(default)]
    pub allow_stale: bool,
}

impl MarketDataQuery {
    pub fn validate_and_normalize(mut self) -> Result<Self, String> {
        self.symbol = self.symbol.trim().to_ascii_uppercase();
        self.timeframe = self.timeframe.trim().to_ascii_uppercase();

        if self.symbol.is_empty()
            || self.symbol.len() > 64
            || !self
                .symbol
                .chars()
                .all(|value| value.is_ascii_alphanumeric() || matches!(value, '_' | '-' | '.'))
        {
            return Err("Market-data symbol is malformed.".into());
        }
        if !matches!(
            self.timeframe.as_str(),
            "M1" | "M2"
                | "M3"
                | "M4"
                | "M5"
                | "M6"
                | "M10"
                | "M12"
                | "M15"
                | "M20"
                | "M30"
                | "H1"
                | "H2"
                | "H3"
                | "H4"
                | "H6"
                | "H8"
                | "H12"
                | "D1"
                | "W1"
        ) {
            return Err(
                "Market-data timeframe is unsupported or has no fixed interval duration.".into(),
            );
        }
        if self.from_ms <= 0
            || self.from_ms > 9_007_199_254_740_991
            || self.to_ms < self.from_ms
            || self.to_ms > 9_007_199_254_740_991
            || self.to_ms.saturating_sub(self.from_ms) > MAX_MARKET_DATA_RANGE_MS
        {
            return Err("Market-data query range is invalid or exceeds 366 days.".into());
        }
        if self.after_ms.is_some_and(|cursor| {
            cursor < self.from_ms.saturating_sub(1)
                || cursor > self.to_ms
                || cursor > 9_007_199_254_740_991
        }) {
            return Err("Market-data pagination cursor is outside the requested range.".into());
        }
        if !(1..=MAX_MARKET_DATA_PAGE_SIZE).contains(&self.limit) {
            return Err(format!(
                "Market-data page size must be between 1 and {MAX_MARKET_DATA_PAGE_SIZE}."
            ));
        }
        if !(1_000..=MAX_MARKET_DATA_AGE_MS).contains(&self.max_data_age_ms) {
            return Err(
                "Market-data freshness limit must be between one second and seven days.".into(),
            );
        }
        if let Some(provider_id) = self.provider_id.as_mut() {
            *provider_id = provider_id.trim().to_ascii_lowercase();
            if provider_id.is_empty()
                || provider_id.len() > 48
                || !provider_id.chars().all(|value| {
                    value.is_ascii_lowercase()
                        || value.is_ascii_digit()
                        || matches!(value, '-' | '_')
                })
            {
                return Err("Market-data provider identifier is malformed.".into());
            }
        }
        if self.purpose == MarketDataPurpose::BrokerContext {
            if self.max_data_age_ms > 600_000 {
                return Err(
                    "Broker-context market data freshness cannot exceed ten minutes.".into(),
                );
            }
            if self
                .provider_id
                .as_deref()
                .is_some_and(|provider| provider != "mt5-terminal")
            {
                return Err(
                    "Broker context may only use the authenticated MT5 terminal provider.".into(),
                );
            }
            if self.allow_stale {
                return Err("Stale data cannot be explicitly allowed for broker context.".into());
            }
            self.provider_id = Some("mt5-terminal".into());
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataProviderCatalogEntry {
    pub id: String,
    pub display_name: String,
    pub authority: MarketDataAuthority,
    pub venue: String,
    pub capabilities: Vec<MarketDataCapability>,
    pub supported_purposes: Vec<MarketDataPurpose>,
    pub state: MarketDataProviderState,
    pub observed_at_ms: Option<i64>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataRequestSummary {
    pub symbol: String,
    pub timeframe: String,
    pub from_ms: i64,
    pub to_ms: i64,
    pub after_ms: Option<i64>,
    pub limit: usize,
    pub provider_id: String,
    pub purpose: MarketDataPurpose,
    pub max_data_age_ms: i64,
    pub allow_stale: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataBar {
    pub time_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub volume_basis: MarketDataVolumeBasis,
    pub tick_volume: Option<u64>,
    pub real_volume: Option<u64>,
    pub revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataProvenance {
    pub provider_id: String,
    pub provider_name: String,
    pub authority: MarketDataAuthority,
    pub venue: String,
    pub retrieved_at_ms: i64,
    pub latest_observation_at_ms: Option<i64>,
    pub latest_received_at_ms: Option<i64>,
    pub dataset_fingerprint_sha256: String,
    pub scope_fingerprint_sha256: Option<String>,
    pub is_live: bool,
    pub source_details: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataQuality {
    pub freshness: MarketDataFreshness,
    pub freshness_age_ms: Option<i64>,
    pub freshness_limit_ms: i64,
    pub candle_count: usize,
    pub revision_count: u64,
    pub interval_anomaly_count: u64,
    pub gap_candidates: Vec<MarketDataGap>,
    pub has_more: bool,
    pub continuity_interpretation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataGap {
    pub previous_bar_time_ms: i64,
    pub next_bar_time_ms: i64,
    pub elapsed_ms: i64,
    pub nominal_intervals_elapsed: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnifiedMarketDataResult {
    pub schema_version: u16,
    pub request: MarketDataRequestSummary,
    pub provenance: MarketDataProvenance,
    pub quality: MarketDataQuality,
    pub bars: Vec<MarketDataBar>,
    pub next_after_ms: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::{MarketDataPurpose, MarketDataQuery};

    fn query() -> MarketDataQuery {
        MarketDataQuery {
            symbol: " btcusdt ".into(),
            timeframe: "m5".into(),
            from_ms: 1_700_000_000_000,
            to_ms: 1_700_100_000_000,
            after_ms: None,
            limit: 500,
            provider_id: None,
            purpose: MarketDataPurpose::Research,
            max_data_age_ms: 300_000,
            allow_stale: false,
        }
    }

    #[test]
    fn query_normalizes_symbol_timeframe_and_provider() {
        let mut request = query();
        request.provider_id = Some(" BINANCE-PUBLIC ".into());
        let normalized = request.validate_and_normalize().unwrap();
        assert_eq!(normalized.symbol, "BTCUSDT");
        assert_eq!(normalized.timeframe, "M5");
        assert_eq!(normalized.provider_id.as_deref(), Some("binance-public"));
    }

    #[test]
    fn broker_context_is_pinned_to_mt5_and_cannot_allow_stale_data() {
        let mut request = query();
        request.purpose = MarketDataPurpose::BrokerContext;
        request.provider_id = Some("binance-public".into());
        assert!(request.validate_and_normalize().is_err());

        let mut request = query();
        request.purpose = MarketDataPurpose::BrokerContext;
        request.allow_stale = true;
        assert!(request.validate_and_normalize().is_err());

        let mut request = query();
        request.purpose = MarketDataPurpose::BrokerContext;
        let normalized = request.validate_and_normalize().unwrap();
        assert_eq!(normalized.provider_id.as_deref(), Some("mt5-terminal"));
    }

    #[test]
    fn malformed_range_cursor_page_and_freshness_policy_are_rejected() {
        let mut request = query();
        request.to_ms = request.from_ms - 1;
        assert!(request.validate_and_normalize().is_err());

        let mut request = query();
        request.after_ms = Some(request.to_ms + 1);
        assert!(request.validate_and_normalize().is_err());

        let mut request = query();
        request.limit = 5_001;
        assert!(request.validate_and_normalize().is_err());

        let mut request = query();
        request.max_data_age_ms = 999;
        assert!(request.validate_and_normalize().is_err());
    }

    #[test]
    fn variable_calendar_month_bars_are_not_accepted_as_fixed_grid_requests() {
        let mut request = query();
        request.timeframe = "MN1".into();
        assert!(request.validate_and_normalize().is_err());
    }
}
