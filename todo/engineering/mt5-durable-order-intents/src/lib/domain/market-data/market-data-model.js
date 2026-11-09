/**
 * Pure guards for the market-data boundary. Quote coverage must never be
 * mistaken for candle coverage, unsupported series never silently fall back,
 * and rendered candles must match the exact typed request.
 */
/** @typedef {import('../../types').CandleInterval} CandleInterval */
/** @typedef {import('../../types').CandleSeriesCapability} CandleSeriesCapability */
/** @typedef {import('../../types').MarketDataCapabilities} MarketDataCapabilities */
/** @typedef {import('../../types').CandleSeriesRequest} CandleSeriesRequest */
/** @typedef {import('../../types').CandleSeriesResponse} CandleSeriesResponse */
import { CANDLE_INTERVALS, CANDLE_INTERVAL_MS } from './timeframe-contract.js';
import { isFreshMarketTimestamp } from './market-data-policy.js';
export { CANDLE_INTERVALS } from './timeframe-contract.js';

const VALIDATED_CANDLE_SNAPSHOTS = new WeakSet();

/** @param {unknown} series @returns {boolean} */
function capabilityIsUsable(series) {
  if (!series || typeof series !== 'object') return false;
  const candidate = /** @type {Record<string, unknown>} */ (series);
  return candidate.supported === true
    && candidate.available === true
    && typeof candidate.symbol === 'string'
    && /^[A-Z0-9_-]{1,30}$/.test(candidate.symbol)
    && typeof candidate.interval === 'string'
    && CANDLE_INTERVALS.includes(/** @type {CandleInterval} */ (candidate.interval))
    && typeof candidate.candleCount === 'number'
    && Number.isInteger(candidate.candleCount)
    && candidate.candleCount >= 30
    && typeof candidate.latestCandleTimeMs === 'number'
    && Number.isSafeInteger(candidate.latestCandleTimeMs)
    && candidate.latestCandleTimeMs > 0
    && typeof candidate.latestClosedBarAtMs === 'number'
    && Number.isSafeInteger(candidate.latestClosedBarAtMs)
    && candidate.latestClosedBarAtMs > candidate.latestCandleTimeMs
    && typeof candidate.retrievedAtMs === 'number'
    && Number.isSafeInteger(candidate.retrievedAtMs)
    && candidate.retrievedAtMs >= 0
    && typeof candidate.source === 'string'
    && candidate.source.trim().length > 0;
}

/** @param {MarketDataCapabilities|null|undefined} capabilities @returns {CandleSeriesCapability[]} */
export function getAvailableCandleSeries(capabilities) {
  if (!capabilities || !Array.isArray(capabilities.candleSeries)) return [];
  return capabilities.candleSeries.filter(capabilityIsUsable);
}

/**
 * @param {MarketDataCapabilities|null|undefined} capabilities
 * @param {unknown} symbol
 * @param {unknown} interval
 * @returns {CandleSeriesRequest}
 */
export function buildCandleSeriesRequest(capabilities, symbol, interval) {
  const normalizedSymbol = typeof symbol === 'string' ? symbol.trim().toUpperCase() : '';
  if (!/^[A-Z0-9_-]{1,30}$/.test(normalizedSymbol)) {
    throw new Error('Candle-series symbol is malformed.');
  }
  if (typeof interval !== 'string' || !CANDLE_INTERVALS.includes(/** @type {CandleInterval} */ (interval))) {
    throw new Error(`Unsupported candle interval: ${String(interval)}.`);
  }
  const normalizedInterval = /** @type {CandleInterval} */ (interval);
  const capability = capabilities?.candleSeries?.find((series) =>
    series?.symbol === normalizedSymbol && series?.interval === normalizedInterval && series?.supported === true,
  );
  if (!capability) {
    throw new Error(`Unsupported candle series ${normalizedSymbol} ${interval}. No substitute series will be returned.`);
  }
  if (capability.available !== true) {
    throw new Error(capability.note || `Candle series ${normalizedSymbol} ${interval} is supported but not currently available.`);
  }
  if (!capabilityIsUsable(capability)) {
    const detail = typeof capability.note === 'string' && capability.note.trim() ? `: ${capability.note}` : '';
    throw new Error(`Candle series ${normalizedSymbol} ${interval} is not currently validated as available${detail}.`);
  }
  return { symbol: normalizedSymbol, interval: normalizedInterval };
}

/**
 * Validate a response at the UI boundary before any value reaches charts or indicators.
 * @param {import('../../types').CandleSeriesResponse} response
 * @param {import('../../types').CandleSeriesRequest} requested
 * @returns {import('../../types').ValidatedCandleSeriesSnapshot}
 */
export function validateCandleSeriesResponse(response, requested) {
  const symbol = typeof requested?.symbol === 'string' ? requested.symbol.trim().toUpperCase() : '';
  const interval = requested?.interval;
  if (!/^[A-Z0-9_-]{1,30}$/.test(symbol) || typeof interval !== 'string' || !CANDLE_INTERVALS.includes(/** @type {CandleInterval} */ (interval))) {
    throw new Error('The requested candle series is malformed.');
  }
  if (!response || response.request?.symbol !== symbol || response.request?.interval !== interval) {
    throw new Error(`Provider response did not match requested series ${symbol} ${interval}; no data was rendered.`);
  }
  if (typeof response.source !== 'string' || response.source.trim().length === 0
    || typeof response.providerStatus !== 'string' || response.providerStatus.trim().length === 0
    || !Number.isSafeInteger(response.updatedAtMs) || response.updatedAtMs < 0
    || !Number.isSafeInteger(response.retrievedAtMs) || response.retrievedAtMs < 0
    || response.updatedAtMs !== response.retrievedAtMs
    || typeof response.latestClosedBarAtMs !== 'number'
    || !Number.isSafeInteger(response.latestClosedBarAtMs) || response.latestClosedBarAtMs <= 0
    || typeof response.datasetSha256 !== 'string' || !/^[a-f0-9]{64}$/.test(response.datasetSha256)
    || typeof response.isLive !== 'boolean') {
    throw new Error('Candle-series response is missing valid source, status, timestamp, dataset fingerprint, or freshness metadata.');
  }
  const now = Date.now();
  if (response.retrievedAtMs > now + 30_000) {
    throw new Error('Candle-series retrieval timestamp is implausibly far in the future.');
  }
  if (response.isLive && (response.retrievedAtMs <= 0 || !response.providerStatus.startsWith('CONNECTED') || !isFreshMarketTimestamp(response.retrievedAtMs, now, 30_000))) {
    throw new Error('Candle series claims to be live without a connected and fresh provider receipt.');
  }
  if (!Array.isArray(response.candles) || response.candles.length < 30) {
    throw new Error(`Candle-series response contains fewer than 30 candles for ${symbol} ${interval}.`);
  }
  let previousTime = 0;
  const expectedIntervalMs = CANDLE_INTERVAL_MS[interval];
  for (const [index, candle] of response.candles.entries()) {
    const fields = /** @type {const} */ (['time', 'open', 'high', 'low', 'close', 'volume']);
    if (!candle || fields.some((field) => !Number.isFinite(candle[field])) || !Number.isSafeInteger(candle.time)) {
      throw new Error(`Candle ${index + 1} contains a missing or non-finite OHLCV field.`);
    }
    if (candle.time <= previousTime || candle.time <= 0) {
      throw new Error(`Candle timestamps are not strictly increasing at row ${index + 1}.`);
    }
    if (candle.time > now + 30_000) {
      throw new Error(`Candle timestamp at row ${index + 1} is implausibly far in the future.`);
    }
    if (candle.time % expectedIntervalMs !== 0) {
      throw new Error(`Candle timestamp at row ${index + 1} is not aligned to the ${interval} UTC interval grid; no data was rendered.`);
    }
    if (candle.open <= 0 || candle.high <= 0 || candle.low <= 0 || candle.close <= 0 || candle.volume < 0
      || candle.high < Math.max(candle.open, candle.close, candle.low)
      || candle.low > Math.min(candle.open, candle.close, candle.high)) {
      throw new Error(`Candle ${index + 1} violates OHLC price-range invariants.`);
    }
    if (previousTime > 0) {
      const observedIntervalMs = candle.time - previousTime;
      if (observedIntervalMs !== expectedIntervalMs) {
        const missingIntervals = observedIntervalMs / expectedIntervalMs;
        const detail = Number.isInteger(missingIntervals) && missingIntervals > 1
          ? ` (${missingIntervals - 1} candle interval(s) missing)`
          : '';
        throw new Error(`Candle cadence gap at row ${index + 1}: expected ${expectedIntervalMs}ms, received ${observedIntervalMs}ms${detail}; no data was rendered.`);
      }
    }
    previousTime = candle.time;
  }
  const newestCandleTime = response.candles[response.candles.length - 1].time;
  const expectedLatestClosedBarAtMs = newestCandleTime + expectedIntervalMs - 1;
  if (!Number.isSafeInteger(expectedLatestClosedBarAtMs) || response.latestClosedBarAtMs !== expectedLatestClosedBarAtMs) {
    throw new Error('Candle-series close-time metadata does not match the latest bar open and requested interval.');
  }
  if (response.latestClosedBarAtMs >= now) {
    throw new Error('Candle series includes a forming bar or a future close timestamp.');
  }
  if (response.isLive && now - response.latestClosedBarAtMs > expectedIntervalMs + 30_000) {
    throw new Error('Candle series claims to be live, but its latest fully closed bar is stale.');
  }
  // Validation must return the exact immutable snapshot consumers use. Returning the
  // original bridge object creates a time-of-check/time-of-use gap: another caller
  // could mutate candles after the checks but before chart/indicator consumption.
  const snapshot = Object.freeze({
    request: Object.freeze({ symbol, interval }),
    source: response.source,
    providerStatus: response.providerStatus,
    updatedAtMs: response.updatedAtMs,
    retrievedAtMs: response.retrievedAtMs,
    latestClosedBarAtMs: response.latestClosedBarAtMs,
    datasetSha256: response.datasetSha256,
    isLive: response.isLive,
    candles: Object.freeze(response.candles.map((candle) => Object.freeze({
      time: candle.time,
      open: candle.open,
      high: candle.high,
      low: candle.low,
      close: candle.close,
      volume: candle.volume,
    }))),
  });
  // A private WeakSet proves this exact in-memory object crossed this validator. A
  // caller cannot gain trusted status merely by imitating the snapshot's object shape.
  VALIDATED_CANDLE_SNAPSHOTS.add(snapshot);
  return snapshot;
}

/**
 * True only for the exact immutable snapshot emitted by this validator in this runtime.
 * @param {unknown} snapshot
 * @returns {snapshot is import('../../types').ValidatedCandleSeriesSnapshot}
 */
export function isValidatedCandleSeriesSnapshot(snapshot) {
  return !!snapshot && typeof snapshot === 'object' && VALIDATED_CANDLE_SNAPSHOTS.has(snapshot);
}


/**
 * Recheck a validated snapshot at the moment it is consumed, not only when it arrived.
 * @param {unknown} snapshot
 * @param {number} [nowMs]
 * @returns {boolean}
 */
export function isCurrentLiveCandleSeriesSnapshot(snapshot, nowMs = Date.now()) {
  if (!isValidatedCandleSeriesSnapshot(snapshot) || !Number.isSafeInteger(nowMs) || nowMs <= 0) return false;
  const intervalMs = CANDLE_INTERVAL_MS[snapshot.request.interval];
  const candleAgeMs = nowMs - snapshot.latestClosedBarAtMs;
  return snapshot.isLive === true
    && snapshot.providerStatus.startsWith('CONNECTED')
    && isFreshMarketTimestamp(snapshot.retrievedAtMs, nowMs)
    && candleAgeMs >= 0
    && candleAgeMs <= intervalMs + 30_000;
}

/** @param {unknown} orderSymbol @param {CandleSeriesRequest|null|undefined} candleSeriesRequest @returns {boolean} */
export function isOrderChartAligned(orderSymbol, candleSeriesRequest) {
  return typeof orderSymbol === 'string'
    && !!candleSeriesRequest
    && orderSymbol.trim().toUpperCase() === candleSeriesRequest.symbol;
}
