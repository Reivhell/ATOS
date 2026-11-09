/**
 * Canonical timeframe contract shared by the frontend, analysis engine and optional
 * local API. A candle series must use UTC-aligned open timestamps and exactly one
 * interval between adjacent candles; no layer may silently infer or repair gaps.
 */
/** @type {Record<import('../../types').CandleInterval, number>} */
export const CANDLE_INTERVAL_MS = Object.freeze({
  '1m': 60_000,
  '5m': 300_000,
  '15m': 900_000,
  '1h': 3_600_000,
  '4h': 14_400_000,
  '1d': 86_400_000,
});

/** @type {readonly import('../../types').CandleInterval[]} */
export const CANDLE_INTERVALS = Object.freeze(
  /** @type {import('../../types').CandleInterval[]} */ (Object.keys(CANDLE_INTERVAL_MS)),
);

/** @param {unknown} timeframe @returns {number|null} */
export function getCandleIntervalMs(timeframe) {
  if (typeof timeframe !== 'string' || !Object.hasOwn(CANDLE_INTERVAL_MS, timeframe)) return null;
  return CANDLE_INTERVAL_MS[/** @type {import('../../types').CandleInterval} */ (timeframe)];
}


/** @param {unknown} candles @param {unknown} timeframe @returns {number|null} */
export function getLatestClosedBarAtMs(candles, timeframe) {
  const intervalMs = getCandleIntervalMs(timeframe);
  if (!intervalMs || !Array.isArray(candles) || candles.length === 0) return null;
  const latest = candles[candles.length - 1];
  const openAtMs = latest && typeof latest === 'object' ? latest.time : null;
  if (typeof openAtMs !== 'number' || !Number.isSafeInteger(openAtMs) || openAtMs <= 0) return null;
  const closeAtMs = openAtMs + intervalMs - 1;
  return Number.isSafeInteger(closeAtMs) && closeAtMs > openAtMs ? closeAtMs : null;
}
