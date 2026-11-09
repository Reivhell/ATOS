/**
 * Pure numerical indicator engine. No UI, network, persistence, or order authority.
 * @typedef {import('../../types').Candle} Candle
 * @typedef {'trend'|'momentum'|'location'} IndicatorDomain
 * @typedef {'bullish'|'bearish'|'neutral'} SignalDirection
 * @typedef {{domain:IndicatorDomain,direction:SignalDirection,signalStrength:number,evidenceRefs:string[]}} IndicatorSignal
 */
/** @param {number} value @param {number} [digits=4] */
function format(value, digits = 4) {
  if (!Number.isFinite(value)) return '—';
  const maxDigits = Math.abs(value) < 1 ? Math.min(8, Math.max(digits, 6)) : digits;
  return new Intl.NumberFormat('en-US', { maximumFractionDigits: maxDigits }).format(value);
}

/** @param {readonly Candle[]} candles @returns {number[]} */
function closeValues(candles) {
  return candles.map((candle) => Number(candle.close));
}

/** @param {number[]} values @param {number} period @returns {number|null} */
function sma(values, period) {
  if (values.length < period) return null;
  const slice = values.slice(-period);
  if (slice.some((value) => !Number.isFinite(value))) return null;
  return slice.reduce((total, value) => total + value, 0) / period;
}

/** @param {number[]} values @param {number} period @returns {Array<number|null>} */
function emaSeries(values, period) {
  if (values.length < period || values.some((value) => !Number.isFinite(value))) return [];
  const output = Array(values.length).fill(null);
  let current = values.slice(0, period).reduce((sum, value) => sum + value, 0) / period;
  output[period - 1] = current;
  const alpha = 2 / (period + 1);
  for (let i = period; i < values.length; i += 1) {
    current = values[i] * alpha + current * (1 - alpha);
    output[i] = current;
  }
  return output;
}

/** @param {number[]} values @param {number} [period=14] @returns {number|null} */
function rsi(values, period = 14) {
  if (values.length <= period || values.some((value) => !Number.isFinite(value))) return null;
  let gains = 0;
  let losses = 0;
  for (let i = 1; i <= period; i += 1) {
    const delta = values[i] - values[i - 1];
    gains += Math.max(delta, 0);
    losses += Math.max(-delta, 0);
  }
  let averageGain = gains / period;
  let averageLoss = losses / period;
  for (let i = period + 1; i < values.length; i += 1) {
    const delta = values[i] - values[i - 1];
    averageGain = (averageGain * (period - 1) + Math.max(delta, 0)) / period;
    averageLoss = (averageLoss * (period - 1) + Math.max(-delta, 0)) / period;
  }
  if (averageLoss === 0) return averageGain === 0 ? 50 : 100;
  const relativeStrength = averageGain / averageLoss;
  return 100 - 100 / (1 + relativeStrength);
}

/** @param {readonly Candle[]} candles @param {number} [period=14] @returns {number|null} */
function atr(candles, period = 14) {
  if (candles.length <= period) return null;
  const ranges = [];
  for (let i = 0; i < candles.length; i += 1) {
    const current = candles[i];
    const high = Number(current.high);
    const low = Number(current.low);
    if (![high, low].every(Number.isFinite) || high < low) return null;
    if (i === 0) {
      ranges.push(high - low);
      continue;
    }
    const previousClose = Number(candles[i - 1].close);
    if (!Number.isFinite(previousClose)) return null;
    ranges.push(Math.max(high - low, Math.abs(high - previousClose), Math.abs(low - previousClose)));
  }
  let current = ranges.slice(0, period).reduce((sum, value) => sum + value, 0) / period;
  for (let i = period; i < ranges.length; i += 1) current = (current * (period - 1) + ranges[i]) / period;
  return current;
}

/** @param {number[]} values @param {number} [fast=12] @param {number} [slow=26] @param {number} [signalPeriod=9] @returns {{line:number,signal:number,histogram:number}|null} */
function macd(values, fast = 12, slow = 26, signalPeriod = 9) {
  if (values.length < slow + signalPeriod - 1 || values.some((value) => !Number.isFinite(value))) return null;
  const fastEma = emaSeries(values, fast);
  const slowEma = emaSeries(values, slow);
  const macdValues = [];
  for (let i = slow - 1; i < values.length; i += 1) {
    const fastValue = fastEma[i];
    const slowValue = slowEma[i];
    if (typeof fastValue === 'number' && typeof slowValue === 'number') macdValues.push(fastValue - slowValue);
  }
  const signal = emaSeries(macdValues, signalPeriod);
  const line = macdValues.at(-1);
  const signalLine = signal.at(-1);
  if (line == null || signalLine == null) return null;
  return { line, signal: signalLine, histogram: line - signalLine };
}

/** @param {number[]} values @param {number} [period=20] @param {number} [deviations=2] @returns {{upper:number,middle:number,lower:number}|null} */
function bollinger(values, period = 20, deviations = 2) {
  if (values.length < period || values.some((value) => !Number.isFinite(value))) return null;
  const sample = values.slice(-period);
  const mean = sample.reduce((sum, value) => sum + value, 0) / period;
  const variance = sample.reduce((sum, value) => sum + (value - mean) ** 2, 0) / period;
  const deviation = Math.sqrt(variance);
  return { upper: mean + deviations * deviation, middle: mean, lower: mean - deviations * deviation };
}

/**
 * Single calculation path shared by human-readable and machine-readable indicator APIs.
 * This prevents numeric drift between dashboard cards and the research decision engine.
 * @param {readonly Candle[]} candles
 */
function computeIndicatorMetrics(candles) {
  const values = closeValues(candles);
  return {
    close: values.at(-1) ?? null,
    smaFast: sma(values, 9),
    smaSlow: sma(values, 21),
    ema20: emaSeries(values, 20).at(-1) ?? null,
    rsi14: rsi(values, 14),
    atr14: atr(candles, 14),
    macd: macd(values, 12, 26, 9),
    bollinger: bollinger(values, 20, 2),
  };
}

/**
 * Calculate a deliberately small, explicitly scoped set of common indicators.
 * @param {Candle[]} [candles=[]]
 */
export function calculateIndicatorReadings(candles = []) {
  const safeCandles = Array.isArray(candles) ? candles : [];
  const { smaFast: sma9, smaSlow: sma21, ema20, rsi14, atr14, macd: macdValue, bollinger: bands } = computeIndicatorMetrics(safeCandles);
  const isUsable = safeCandles.every((candle) => [candle.open, candle.high, candle.low, candle.close, candle.volume].every((value) => Number.isFinite(Number(value))) && Number(candle.high) >= Number(candle.low));
  /** @param {number|null} value @returns {number|null} */
  const guard = (value) => isUsable ? value : null;
  const effectiveSma9 = guard(sma9);
  const effectiveSma21 = guard(sma21);
  const effectiveEma20 = guard(ema20);
  const effectiveRsi14 = guard(rsi14);
  const effectiveAtr14 = guard(atr14);
  const effectiveMacd = isUsable ? macdValue : null;
  const effectiveBands = isUsable ? bands : null;
  /** @param {number} needed */
  const insufficient = (needed) => `Need at least ${needed} valid candles; ${safeCandles.length} loaded.`;
  return [
    {
      key: 'sma', title: 'SMA · 9 / 21', value: effectiveSma9 == null || effectiveSma21 == null ? 'Unavailable' : `${format(effectiveSma9)} / ${format(effectiveSma21)}`,
      detail: effectiveSma9 == null || effectiveSma21 == null ? insufficient(21) : `Fast ${effectiveSma9 >= effectiveSma21 ? 'above' : 'below'} slow · close-price simple averages.`,
      available: effectiveSma9 != null && effectiveSma21 != null,
    },
    {
      key: 'ema', title: 'EMA · 20', value: effectiveEma20 == null ? 'Unavailable' : format(effectiveEma20),
      detail: effectiveEma20 == null ? insufficient(20) : 'Exponential average seeded with the first 20-bar mean.',
      available: effectiveEma20 != null,
    },
    {
      key: 'rsi', title: 'RSI · 14', value: effectiveRsi14 == null ? 'Unavailable' : format(effectiveRsi14, 2),
      detail: effectiveRsi14 == null ? insufficient(15) : effectiveRsi14 >= 70 ? 'Above the conventional 70 threshold; not a standalone sell signal.' : effectiveRsi14 <= 30 ? 'Below the conventional 30 threshold; not a standalone buy signal.' : 'Between conventional 30 / 70 reference levels.',
      available: effectiveRsi14 != null,
    },
    {
      key: 'macd', title: 'MACD · 12 / 26 / 9', value: effectiveMacd == null ? 'Unavailable' : format(effectiveMacd.histogram),
      detail: effectiveMacd == null ? insufficient(34) : `Line ${format(effectiveMacd.line)} · signal ${format(effectiveMacd.signal)} · histogram shown above.`,
      available: effectiveMacd != null,
    },
    {
      key: 'atr', title: 'ATR · 14', value: effectiveAtr14 == null ? 'Unavailable' : format(effectiveAtr14),
      detail: effectiveAtr14 == null ? insufficient(15) : 'Wilder-smoothed true range; measures volatility, not direction.',
      available: effectiveAtr14 != null,
    },
    {
      key: 'bollinger', title: 'Bollinger Bands · 20 / 2σ', value: effectiveBands == null ? 'Unavailable' : `${format(effectiveBands.lower)} – ${format(effectiveBands.upper)}`,
      detail: effectiveBands == null ? insufficient(20) : `Middle ${format(effectiveBands.middle)} · population standard deviation over 20 closes.`,
      available: effectiveBands != null,
    },
  ];
}

/**
 * Machine-readable indicator output for the layered analysis engine.
 * Indicator strength is a deterministic heuristic, not a probability of profit.
 * @param {unknown} [candles=[]]
 */
export function calculateIndicatorSnapshot(candles = []) {
  const issues = [];
  if (!Array.isArray(candles)) {
    return {
      contractVersion: 'technical-indicator-snapshot.v1',
      status: 'invalid_data',
      candleCount: 0,
      asOfMs: null,
      issues: ['Candles must be supplied as an array.'],
      metrics: null,
      signals: [],
    };
  }

  const candleRows = /** @type {readonly Candle[]} */ (candles);
  let previousTime = null;
  for (const [index, candle] of candleRows.entries()) {
    const row = index + 1;
    if (!candle || !/** @type {readonly (keyof Candle)[]} */ (['open', 'high', 'low', 'close', 'volume']).every((field) => Number.isFinite(candle[field]))) {
      issues.push(`Candle ${row} contains a missing or non-finite OHLCV value.`);
      continue;
    }
    if (candle.open <= 0 || candle.high <= 0 || candle.low <= 0 || candle.close <= 0 || candle.volume < 0
      || candle.high < Math.max(candle.open, candle.close, candle.low)
      || candle.low > Math.min(candle.open, candle.close, candle.high)) {
      issues.push(`Candle ${row} violates positive-price, non-negative-volume, or OHLC range invariants.`);
    }
    if (candle.time !== undefined) {
      if (!Number.isSafeInteger(candle.time) || candle.time <= 0) {
        issues.push(`Candle ${row} timestamp must be a positive safe integer in milliseconds.`);
      } else if (previousTime !== null && candle.time <= previousTime) {
        issues.push(`Candle timestamps are not strictly increasing at row ${row}.`);
      }
      if (Number.isSafeInteger(candle.time) && candle.time > 0) previousTime = candle.time;
    }
  }

  const finalTime = candleRows.at(-1)?.time;
  const asOfMs = typeof finalTime === 'number' && Number.isSafeInteger(finalTime) ? finalTime : null;
  if (issues.length > 0) {
    return {
      contractVersion: 'technical-indicator-snapshot.v1', status: 'invalid_data',
      candleCount: candleRows.length, asOfMs, issues: [...new Set(issues)], metrics: null, signals: [],
    };
  }

  const { close: lastClose, smaFast, smaSlow, ema20, rsi14, atr14, macd: macdValue, bollinger: bands } = computeIndicatorMetrics(candleRows);
  const metrics = {
    close: lastClose,
    smaFast,
    smaSlow,
    ema20,
    rsi14,
    atr14,
    atrPct: atr14 != null && typeof lastClose === 'number' && lastClose > 0 ? (atr14 / lastClose) * 100 : null,
    macdLine: macdValue?.line ?? null,
    macdSignal: macdValue?.signal ?? null,
    macdHistogram: macdValue?.histogram ?? null,
    bollingerUpper: bands?.upper ?? null,
    bollingerMiddle: bands?.middle ?? null,
    bollingerLower: bands?.lower ?? null,
  };
  /** @param {IndicatorDomain} domain @param {SignalDirection} direction @param {number} strength @param {string[]} evidenceRefs @returns {IndicatorSignal} */
  const signal = (domain, direction, strength, evidenceRefs) => ({
    domain, direction, signalStrength: Math.max(0, Math.min(1, strength)), evidenceRefs,
  });
  const signals = [];
  if (smaFast != null && smaSlow != null && typeof lastClose === 'number' && lastClose > 0) {
    const spread = (smaFast - smaSlow) / lastClose;
    signals.push(signal('trend', spread > 0 ? 'bullish' : spread < 0 ? 'bearish' : 'neutral',
      Math.min(0.95, 0.55 + Math.abs(spread) * 8), [`sma9=${smaFast}`, `sma21=${smaSlow}`]));
  }
  if (macdValue && rsi14 != null && typeof lastClose === 'number' && lastClose > 0) {
    /** @type {SignalDirection} */
    let direction = 'neutral';
    if (macdValue.histogram > 0 && rsi14 >= 52) direction = 'bullish';
    else if (macdValue.histogram < 0 && rsi14 <= 48) direction = 'bearish';
    const strength = direction === 'neutral' ? 0.55
      : Math.min(0.9, 0.6 + Math.abs(rsi14 - 50) / 160 + Math.abs(macdValue.histogram / lastClose) * 10);
    signals.push(signal('momentum', direction, strength,
      [`macdHistogram=${macdValue.histogram}`, `rsi14=${rsi14}`]));
  }
  if (bands && typeof lastClose === 'number' && lastClose > 0) {
    const spread = (lastClose - bands.middle) / lastClose;
    signals.push(signal('location', spread > 0 ? 'bullish' : spread < 0 ? 'bearish' : 'neutral',
      Math.min(0.85, 0.55 + Math.abs(spread) * 8), [`close=${lastClose}`, `bollingerMiddle=${bands.middle}`]));
  }

  return {
    contractVersion: 'technical-indicator-snapshot.v1',
    status: candleRows.length >= 34 ? 'ready' : 'partial',
    candleCount: candleRows.length,
    asOfMs,
    issues: candleRows.length >= 34 ? [] : [`At least 34 candles are required for the full SMA/EMA/RSI/MACD/ATR/Bollinger reading set; ${candleRows.length} supplied.`],
    metrics,
    signals,
  };
}
