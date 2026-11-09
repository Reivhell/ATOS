/**
 * Deterministic market-data integrity and latest-bar anomaly audit.
 * Hard structural/temporal violations invalidate the series. Statistical anomalies
 * are advisory only: genuine market shocks must not be silently discarded.
 */
import { getCandleIntervalMs } from './timeframe-contract.js';

export const CANDLE_SERIES_AUDIT_CONTRACT = 'candle-series-audit.v1';
export const MAX_CANDLE_SERIES_AUDIT_CANDLES = 10_000;
const MAX_RECORDED_ISSUES = 64;
const ROBUST_HISTORY = 64;
const MIN_ROBUST_SAMPLES = 20;
const ROBUST_Z_LIMIT = 8;
const MIN_ABSOLUTE_LOG_RETURN = Math.log(1.03);
const MIN_VOLUME_MULTIPLE = 8;
const MAD_TO_SIGMA = 1.4826;

/** @typedef {import('../../types').Candle} Candle */

/** @param {number[]} values @returns {number|null} */
function median(values) {
  if (values.length === 0) return null;
  const ordered = [...values].sort((a, b) => a - b);
  const middle = Math.floor(ordered.length / 2);
  return ordered.length % 2 === 0
    ? ordered[middle - 1] / 2 + ordered[middle] / 2
    : ordered[middle];
}

/** @param {number[]} values @param {number} center @returns {number|null} */
function medianAbsoluteDeviation(values, center) {
  return median(values.map((value) => Math.abs(value - center)));
}

/** @param {unknown} candle @returns {candle is Candle} */
function hasValidOhlcv(candle) {
  if (!candle || typeof candle !== 'object' || Array.isArray(candle)) return false;
  const row = /** @type {Record<string, unknown>} */ (candle);
  if (!['open', 'high', 'low', 'close', 'volume'].every((key) => typeof row[key] === 'number' && Number.isFinite(row[key]))) return false;
  const { open, high, low, close, volume } = /** @type {Candle} */ (candle);
  return open > 0 && high > 0 && low > 0 && close > 0 && volume >= 0
    && high >= Math.max(open, close, low)
    && low <= Math.min(open, close, high);
}

/**
 * @param {unknown} candles
 * @param {unknown} timeframe
 * @returns {{contractVersion:string,status:'invalid'|'warning'|'clean',timeframe:string|null,intervalMs:number|null,candleCount:number,hardIssues:readonly {code:string,message:string,index?:number}[],warnings:readonly {code:string,message:string,index?:number}[],metrics:Readonly<{firstCandleTimeMs:number|null,lastCandleTimeMs:number|null,expectedSpanMs:number|null,latestCloseReturnPct:number|null,latestReturnRobustZ:number|null,latestVolumeToMedianMultiple:number|null}>}}
 */
export function auditCandleSeries(candles, timeframe) {
  const rows = Array.isArray(candles) ? /** @type {Candle[]} */ (candles) : [];
  const intervalMs = getCandleIntervalMs(timeframe);
  /** @type {Array<{code:string,message:string,index?:number}>} */
  const hardIssues = [];
  /** @type {Array<{code:string,message:string,index?:number}>} */
  const warnings = [];
  /** @param {Array<{code:string,message:string,index?:number}>} target @param {{code:string,message:string,index?:number}} issue */
  const addBounded = (target, issue) => {
    if (target.length < MAX_RECORDED_ISSUES) target.push(Object.freeze(issue));
  };

  if (!Array.isArray(candles)) {
    addBounded(hardIssues, { code: 'candles_not_array', message: 'Candles must be supplied as an array.' });
  } else if (rows.length === 0) {
    addBounded(hardIssues, { code: 'empty_series', message: 'No candles are available for a temporal integrity audit.' });
  }
  if (rows.length > MAX_CANDLE_SERIES_AUDIT_CANDLES) {
    addBounded(hardIssues, { code: 'series_too_large', message: `Candle audit exceeds the ${MAX_CANDLE_SERIES_AUDIT_CANDLES}-candle safety limit.` });
  }
  if (intervalMs === null) {
    addBounded(hardIssues, { code: 'unsupported_timeframe', message: `Unsupported candle timeframe: ${String(timeframe)}.` });
  }

  let previousTime = null;
  const inspectCount = Math.min(rows.length, MAX_CANDLE_SERIES_AUDIT_CANDLES);
  for (let index = 0; index < inspectCount; index += 1) {
    const candle = rows[index];
    const row = index + 1;
    if (!hasValidOhlcv(candle)) {
      addBounded(hardIssues, { code: 'invalid_ohlcv', message: `Candle ${row} violates finite OHLCV, positive-price, non-negative-volume, or high/low range invariants.`, index });
    }
    const time = candle && typeof candle === 'object' ? /** @type {Record<string, unknown>} */ (/** @type {unknown} */ (candle)).time : null;
    if (typeof time !== 'number' || !Number.isSafeInteger(time) || time <= 0) {
      addBounded(hardIssues, { code: 'invalid_timestamp', message: `Candle ${row} timestamp must be a positive safe integer in milliseconds.`, index });
      continue;
    }
    if (intervalMs !== null && time % intervalMs !== 0) {
      addBounded(hardIssues, { code: 'unaligned_timestamp', message: `Candle ${row} timestamp is not aligned to the ${String(timeframe)} UTC interval grid.`, index });
    }
    if (previousTime !== null && intervalMs !== null) {
      if (time <= previousTime) {
        addBounded(hardIssues, { code: 'non_increasing_timestamp', message: `Candle timestamps are duplicate or out of order at row ${row}.`, index });
      } else if (time - previousTime !== intervalMs) {
        const difference = time - previousTime;
        const missing = difference / intervalMs;
        const detail = Number.isInteger(missing) && missing > 1 ? ` (${missing - 1} interval(s) missing)` : '';
        addBounded(hardIssues, { code: 'cadence_gap', message: `Candle cadence violation at row ${row}: expected ${intervalMs}ms, received ${difference}ms${detail}.`, index });
      }
    }
    previousTime = time;
  }

  const firstTime = rows[0]?.time;
  const lastTime = rows.length > 0 ? rows[rows.length - 1].time : undefined;
  const validTimes = typeof firstTime === 'number' && Number.isSafeInteger(firstTime) && firstTime > 0
    && typeof lastTime === 'number' && Number.isSafeInteger(lastTime) && lastTime > 0;
  /** @type {{firstCandleTimeMs:number|null,lastCandleTimeMs:number|null,expectedSpanMs:number|null,latestCloseReturnPct:number|null,latestReturnRobustZ:number|null,latestVolumeToMedianMultiple:number|null}} */
  const metrics = {
    firstCandleTimeMs: validTimes ? firstTime : null,
    lastCandleTimeMs: validTimes ? lastTime : null,
    expectedSpanMs: validTimes && intervalMs !== null ? Math.max(0, rows.length - 1) * intervalMs : null,
    latestCloseReturnPct: null,
    latestReturnRobustZ: null,
    latestVolumeToMedianMultiple: null,
  };

  // Statistical checks only run over fully valid, bounded series. They are advisory,
  // causal to the latest bar, and never alter historical candles or make trades.
  if (hardIssues.length === 0 && rows.length >= MIN_ROBUST_SAMPLES + 2) {
    const latest = /** @type {Candle} */ (rows[rows.length - 1]);
    const previous = /** @type {Candle} */ (rows[rows.length - 2]);
    const currentLogReturn = Math.log(latest.close) - Math.log(previous.close);
    const returnPercent = Math.expm1(currentLogReturn) * 100;
    metrics.latestCloseReturnPct = Number.isFinite(returnPercent) ? Number(returnPercent.toFixed(6)) : null;

    const allReturns = [];
    for (let index = 1; index < rows.length; index += 1) {
      allReturns.push(Math.log(rows[index].close) - Math.log(rows[index - 1].close));
    }
    const priorReturns = allReturns.slice(Math.max(0, allReturns.length - ROBUST_HISTORY - 1), -1);
    if (priorReturns.length >= MIN_ROBUST_SAMPLES) {
      const returnMedian = median(priorReturns);
      const returnMad = returnMedian === null ? null : medianAbsoluteDeviation(priorReturns, returnMedian);
      if (returnMedian !== null && returnMad !== null) {
        const robustScale = Math.max(MAD_TO_SIGMA * returnMad, 0.0005);
        const robustZ = Math.abs(currentLogReturn - returnMedian) / robustScale;
        metrics.latestReturnRobustZ = Number.isFinite(robustZ) ? Number(robustZ.toFixed(4)) : null;
        if (Math.abs(currentLogReturn) >= MIN_ABSOLUTE_LOG_RETURN && robustZ >= ROBUST_Z_LIMIT) {
          addBounded(warnings, {
            code: 'latest_price_return_outlier',
            message: `Latest candle close return is ${metrics.latestCloseReturnPct}% and ${robustZ.toFixed(1)} robust deviations from the prior return baseline; review as a possible genuine shock or data anomaly.`,
            index: rows.length - 1,
          });
        }
      }
    }

    const priorVolumes = rows.slice(Math.max(0, rows.length - 21), -1).map((row) => row.volume);
    if (priorVolumes.length >= MIN_ROBUST_SAMPLES) {
      const volumeMedian = median(priorVolumes);
      const volumeMad = volumeMedian === null ? null : medianAbsoluteDeviation(priorVolumes, volumeMedian);
      if (volumeMedian !== null && volumeMad !== null && volumeMedian > 0) {
        const ratio = latest.volume / volumeMedian;
        metrics.latestVolumeToMedianMultiple = Number.isFinite(ratio) ? Number(ratio.toFixed(4)) : null;
        const robustZ = (latest.volume - volumeMedian) / Math.max(MAD_TO_SIGMA * volumeMad, volumeMedian * 0.01, 1e-12);
        if (ratio >= MIN_VOLUME_MULTIPLE && robustZ >= ROBUST_Z_LIMIT) {
          addBounded(warnings, {
            code: 'latest_volume_outlier',
            message: `Latest candle volume is ${ratio.toFixed(1)}× the prior 20-candle median; review liquidity and provider consistency.`,
            index: rows.length - 1,
          });
        }
      }
    }

    const lastTwenty = rows.slice(-20);
    const flatPrice = lastTwenty.length === 20 && lastTwenty.every((candle) =>
      candle.open === latest.close && candle.high === latest.close && candle.low === latest.close && candle.close === latest.close,
    );
    if (flatPrice) {
      addBounded(warnings, {
        code: 'latest_price_flatline',
        message: 'The latest 20 candles are identical flat-price bars; verify that the feed is not replaying placeholder data.',
        index: rows.length - 1,
      });
    }
  }

  return Object.freeze({
    contractVersion: CANDLE_SERIES_AUDIT_CONTRACT,
    status: hardIssues.length > 0 ? 'invalid' : warnings.length > 0 ? 'warning' : 'clean',
    timeframe: typeof timeframe === 'string' ? timeframe : null,
    intervalMs,
    candleCount: rows.length,
    hardIssues: Object.freeze(hardIssues),
    warnings: Object.freeze(warnings),
    metrics: Object.freeze(metrics),
  });
}
