//! Application-facing market-data query boundary.
//! The durable store remains an infrastructure implementation; this facade keeps UI/IPC
//! callers from depending on storage connection and schema details.
use crate::infrastructure::mt5::{
    bridge::Mt5Bridge,
    market_data::{
        Mt5InstrumentSpecificationRecord, Mt5MarketDataStatus, Mt5SeriesQuery,
        Mt5SeriesQueryResponse, Mt5SessionCalendarRecord,
    },
};

pub fn status(bridge: &Mt5Bridge) -> Result<Mt5MarketDataStatus, String> {
    bridge.market_data_status()
}

pub fn query(
    bridge: &Mt5Bridge,
    request: Mt5SeriesQuery,
) -> Result<Mt5SeriesQueryResponse, String> {
    bridge.market_data_series_query(request)
}

pub fn instrument_specification(
    bridge: &Mt5Bridge,
    broker_server: &str,
    account_login: &str,
    symbol: &str,
) -> Result<Mt5InstrumentSpecificationRecord, String> {
    bridge.instrument_specification(broker_server, account_login, symbol)
}

pub fn session_calendar(
    bridge: &Mt5Bridge,
    broker_server: &str,
    account_login: &str,
    symbol: &str,
) -> Result<Mt5SessionCalendarRecord, String> {
    bridge.session_calendar(broker_server, account_login, symbol)
}
