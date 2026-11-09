/**
 * @typedef {import('../../types').Candle} ResearchCandle
 * @typedef {{type:'swing_high'|'swing_low',price:number,pivotIndex:number,confirmedAt:number}} ConfirmedPivot
 * @typedef {ConfirmedPivot & {label:string|null}} ClassifiedPivot
 * @typedef {{type:string,level:number,levelPivotIndex:number,detectedAt:number,confirmation:'candle_close'}} StructureBreak
 * Deterministic, research-only SMC primitives. These functions never authorize trades.
 * Swing points are emitted only at their confirmation bar (rightBars later), avoiding
 * future leakage in event timestamps. They are deliberately not promoted in the master list.
 */
/** @param {unknown} candles @returns {asserts candles is ResearchCandle[]} */
function validateCandles(candles) {
  if (!Array.isArray(candles) || candles.length < 3) throw new TypeError('At least three candles are required.');
  for (const [i, c] of candles.entries()) {
    if (!c || !Number.isFinite(c.high) || !Number.isFinite(c.low) || c.high < c.low) {
      throw new TypeError(`Invalid OHLC candle at index ${i}.`);
    }
    if (c.open !== undefined && !Number.isFinite(c.open)) throw new TypeError(`Invalid open at index ${i}.`);
    if (c.close !== undefined && !Number.isFinite(c.close)) throw new TypeError(`Invalid close at index ${i}.`);
  }
}

/**
 * Returns confirmed pivots; `pivotIndex` is the extreme, `confirmedAt` is when it became knowable.
 * @param {ResearchCandle[]} candles
 * @param {{leftBars?:number,rightBars?:number}} [options={}]
 * @returns {ConfirmedPivot[]}
 */
export function detectConfirmedSwings(candles, { leftBars = 2, rightBars = 2 } = {}) {
  validateCandles(candles);
  if (!Number.isInteger(leftBars) || leftBars < 1 || !Number.isInteger(rightBars) || rightBars < 1) {
    throw new RangeError('leftBars and rightBars must be positive integers.');
  }
  /** @type {ConfirmedPivot[]} */
  const pivots = [];
  for (let i = leftBars; i < candles.length - rightBars; i += 1) {
    const window = candles.slice(i - leftBars, i + rightBars + 1);
    const high = candles[i].high;
    const low = candles[i].low;
    // Strictly unique extremes avoid ambiguous equal-high/low pivot selection.
    const isHigh = window.every((c, j) => j === leftBars || high > c.high);
    const isLow = window.every((c, j) => j === leftBars || low < c.low);
    if (isHigh) pivots.push({ type: 'swing_high', price: high, pivotIndex: i, confirmedAt: i + rightBars });
    if (isLow) pivots.push({ type: 'swing_low', price: low, pivotIndex: i, confirmedAt: i + rightBars });
  }
  return pivots;
}

/**
 * Labels confirmed swings HH/LH and HL/LL, using only prior confirmed pivots of same type.
 * @param {ConfirmedPivot[]} pivots
 * @returns {ClassifiedPivot[]}
 */
export function classifySwingStructure(pivots) {
  if (!Array.isArray(pivots)) throw new TypeError('pivots must be an array.');
  /** @type {{swing_high:ConfirmedPivot|null,swing_low:ConfirmedPivot|null}} */
  const previous = { swing_high: null, swing_low: null };
  return [...pivots].sort((a, b) => a.confirmedAt - b.confirmedAt || a.pivotIndex - b.pivotIndex).map((p) => {
    if (!['swing_high', 'swing_low'].includes(p.type) || !Number.isFinite(p.price) || !Number.isInteger(p.confirmedAt)) {
      throw new TypeError('Malformed confirmed pivot.');
    }
    const prev = previous[p.type];
    let label = null;
    if (prev !== null) label = p.type === 'swing_high' ? (p.price > prev.price ? 'HH' : p.price < prev.price ? 'LH' : 'EQH') : (p.price > prev.price ? 'HL' : p.price < prev.price ? 'LL' : 'EQL');
    previous[p.type] = p;
    return { ...p, label };
  });
}

/**
 * Detects close-based breaks of already-confirmed swing levels.
 * Emits structural events for research review only; no signal or trade recommendation.
 */
/**
 * @param {ResearchCandle[]} candles
 * @param {ClassifiedPivot[]} classifiedPivots
 * @returns {StructureBreak[]}
 */
export function detectStructureBreaks(candles, classifiedPivots) {
  validateCandles(candles);
  if (!Array.isArray(classifiedPivots)) throw new TypeError('classifiedPivots must be an array.');
  /** @type {StructureBreak[]} */
  const events = [];
  let latestHigh = /** @type {ClassifiedPivot|null} */ (null);
  let latestLow = /** @type {ClassifiedPivot|null} */ (null);
  let bias = 'unknown';
  /** @type {Map<number, ClassifiedPivot[]>} */
  const byConfirmation = new Map();
  /** @type {Set<string>} */
  const seenPivots = new Set();
  for (const p of classifiedPivots) {
    if (!p || !['swing_high', 'swing_low'].includes(p.type)
      || !Number.isFinite(p.price)
      || !Number.isInteger(p.pivotIndex) || p.pivotIndex < 0 || p.pivotIndex >= candles.length
      || !Number.isInteger(p.confirmedAt) || p.confirmedAt <= p.pivotIndex || p.confirmedAt >= candles.length) {
      throw new TypeError('Malformed or non-causal confirmed pivot.');
    }
    const key = `${p.type}:${p.pivotIndex}`;
    if (seenPivots.has(key)) throw new TypeError('Duplicate confirmed pivot identity.');
    seenPivots.add(key);
    const sourcePrice = p.type === 'swing_high' ? candles[p.pivotIndex].high : candles[p.pivotIndex].low;
    if (p.price !== sourcePrice) throw new TypeError('Confirmed pivot price does not match source candle provenance.');
    const confirmationBucket = byConfirmation.get(p.confirmedAt);
    if (confirmationBucket) confirmationBucket.push(p);
    else byConfirmation.set(p.confirmedAt, [p]);
  }
  // External callers may provide pivots in arbitrary order. Process simultaneous
  // confirmations deterministically so input order cannot choose the active level.
  for (const pivots of byConfirmation.values()) {
    pivots.sort((a, b) => a.pivotIndex - b.pivotIndex || a.type.localeCompare(b.type));
  }
  for (let i = 0; i < candles.length; i += 1) {
    // Pivots become available only on/after confirmation, never at their earlier extreme.
    for (const p of byConfirmation.get(i) ?? []) {
      if (p.type === 'swing_high') latestHigh = p;
      if (p.type === 'swing_low') latestLow = p;
    }
    const close = candles[i].close;
    if (!Number.isFinite(close)) continue;
    if (latestHigh && close > latestHigh.price) {
      const eventType = bias === 'bearish' ? 'CHoCH_bullish' : 'BOS_bullish';
      events.push({ type: eventType, level: latestHigh.price, levelPivotIndex: latestHigh.pivotIndex, detectedAt: i, confirmation: 'candle_close' });
      bias = 'bullish';
      latestHigh = null; // one event per confirmed level
    } else if (latestLow && close < latestLow.price) {
      const eventType = bias === 'bullish' ? 'CHoCH_bearish' : 'BOS_bearish';
      events.push({ type: eventType, level: latestLow.price, levelPivotIndex: latestLow.pivotIndex, detectedAt: i, confirmation: 'candle_close' });
      bias = 'bearish';
      latestLow = null;
    }
  }
  return events;
}
