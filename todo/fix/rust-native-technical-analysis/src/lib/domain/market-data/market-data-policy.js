export const MARKET_DATA_OBSERVATION_MAX_AGE_MS = 120_000;

/** @param {unknown} timestamp @param {number} nowMs @param {number} [maximumFutureSkewMs] @returns {boolean} */
export function isFreshMarketTimestamp(timestamp, nowMs, maximumFutureSkewMs = 0) {
  return typeof timestamp === 'number'
    && Number.isSafeInteger(timestamp)
    && timestamp > 0
    && typeof nowMs === 'number'
    && Number.isSafeInteger(nowMs)
    && nowMs > 0
    && typeof maximumFutureSkewMs === 'number'
    && Number.isSafeInteger(maximumFutureSkewMs)
    && maximumFutureSkewMs >= 0
    && timestamp <= nowMs + maximumFutureSkewMs
    && nowMs - timestamp <= MARKET_DATA_OBSERVATION_MAX_AGE_MS;
}
