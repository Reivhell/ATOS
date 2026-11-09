/**
 * @typedef {import('../../types').Candle} Candle
 *
 * Small pure view-model helpers for the Home dashboard. They intentionally do not
 * create candles, prices, or volume data when the provider payload is missing.
 */
/** @param {unknown} candles */
export function summarizeLastCandle(candles) {
  if (!Array.isArray(candles) || candles.length === 0) return null;
  const candle = /** @type {Candle[]} */ (candles)[candles.length - 1];
  if (!candle || !Number.isFinite(candle.time) || !Number.isFinite(candle.open)
    || !Number.isFinite(candle.high) || !Number.isFinite(candle.low)
    || !Number.isFinite(candle.close) || !Number.isFinite(candle.volume)) return null;
  if (candle.open <= 0 || candle.high <= 0 || candle.low <= 0 || candle.close <= 0
    || candle.volume < 0 || candle.high < Math.max(candle.open, candle.close, candle.low)
    || candle.low > Math.min(candle.open, candle.close, candle.high)) return null;
  const changePct = ((candle.close / candle.open) - 1) * 100;
  if (!Number.isFinite(changePct)) return null;
  return {
    time: candle.time,
    open: candle.open,
    high: candle.high,
    low: candle.low,
    close: candle.close,
    volume: candle.volume,
    changePct,
    bullish: candle.close >= candle.open,
  };
}

/** @param {unknown} candles */
export function getMaximumVolume(candles) {
  if (!Array.isArray(candles)) return 0;
  let max = 0;
  for (const candle of /** @type {Candle[]} */ (candles)) {
    if (candle && Number.isFinite(candle.volume) && candle.volume >= 0) max = Math.max(max, candle.volume);
  }
  return max;
}

/** @param {number} volume @param {number} maxVolume @param {number} [maxHeight=34] @param {number} [minHeight=2] */
export function normalizedVolumeHeight(volume, maxVolume, maxHeight = 34, minHeight = 2) {
  if (!Number.isFinite(volume) || volume <= 0 || !Number.isFinite(maxVolume) || maxVolume <= 0
    || !Number.isFinite(maxHeight) || !Number.isFinite(minHeight) || maxHeight < minHeight || minHeight < 0) return 0;
  const ratio = Math.max(0, Math.min(1, volume / maxVolume));
  return Math.min(maxHeight, Math.max(minHeight, minHeight + ratio * (maxHeight - minHeight)));
}
