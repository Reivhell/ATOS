/**
 * @typedef {import('../../types').Candle} Candle
 * @typedef {{type:string,level:number,levelPivotIndex:number,detectedAt:number,confirmation:string}} StructureBreak
 * @typedef {{domain:'trend'|'momentum'|'structure'|'location',direction:'bullish'|'bearish'|'neutral',signalStrength:number,evidenceRefs:string[],asOf?:string,datasetSha256?:string|null}} EngineSignal
 * @typedef {{validated?:boolean,source?:string,asOf?:string|null,latestClosedBarAtMs?:number|null,datasetSha256?:string|null,symbol?:string|null,timeframe?:string|null}} AnalysisMarketData
 * @typedef {{passed?:boolean,source?:string,receiptId?:string}} RiskEvidence
 * @typedef {{candles?:unknown,symbol?:unknown,timeframe?:unknown,marketData?:AnalysisMarketData,risk?:RiskEvidence,intent?:unknown}} AnalysisInput
 * Layered deterministic analysis engine.
 * Coordinates numeric indicators and causal structure observations, then delegates the
 * research-only verdict to the independent decision engine. It has no I/O or order authority.
 */
import { calculateIndicatorSnapshot } from './indicators.js';
import { auditCandleSeries } from '../../domain/market-data/candle-series-audit.js';
import { getLatestClosedBarAtMs } from '../../domain/market-data/timeframe-contract.js';
import { detectConfirmedSwings, classifySwingStructure, detectStructureBreaks } from './research-detectors.js';
import { evaluateResearchDecision } from '../decision/research-decision-engine.js';

export const TECHNICAL_ANALYSIS_ENGINE_CONTRACT = 'technical-analysis-envelope.v2';
export const TECHNICAL_ANALYSIS_ENGINE_VERSION = 'layered-ta-engine.v2';
export const MAX_ANALYSIS_CANDLES = 10_000;

/** @param {Candle[]} candles @param {string} timeframe @returns {string|null} */
function asOfIso(candles, timeframe) {
  const time = getLatestClosedBarAtMs(candles, timeframe);
  if (time === null) return null;
  const date = new Date(time);
  return Number.isFinite(date.getTime()) ? date.toISOString() : null;
}

/** @param {StructureBreak|null} lastBreak @param {string} asOf @param {string|null} datasetSha256 @returns {EngineSignal} */
function signalForStructure(lastBreak, asOf, datasetSha256) {
  const direction = lastBreak?.type?.endsWith('_bullish') ? 'bullish'
    : lastBreak?.type?.endsWith('_bearish') ? 'bearish' : 'neutral';
  const evidenceRefs = lastBreak
    ? [`structure-event:${lastBreak.type}:${lastBreak.detectedAt}`, `level-pivot:${lastBreak.levelPivotIndex}`]
    : ['structure:no-confirmed-close-break-as-of-cutoff'];
  return {
    domain: 'structure', direction, signalStrength: lastBreak ? 0.72 : 0.55,
    evidenceRefs, asOf, datasetSha256,
  };
}

/** @param {Omit<EngineSignal, 'asOf'|'datasetSha256'>} signal @param {string} asOf @param {string|null} datasetSha256 @returns {EngineSignal} */
function enrichSignal(signal, asOf, datasetSha256) {
  return { ...signal, asOf, datasetSha256 };
}

/** @param {string} message @param {AnalysisInput} [input={}] */
function errorEnvelope(message, input = {}) {
  return {
    contractVersion: TECHNICAL_ANALYSIS_ENGINE_CONTRACT,
    engineVersion: TECHNICAL_ANALYSIS_ENGINE_VERSION,
    status: 'blocked',
    mode: 'research_only',
    identity: {
      symbol: typeof input?.symbol === 'string' ? input.symbol : null,
      timeframe: typeof input?.timeframe === 'string' ? input.timeframe : null,
      asOf: null,
      datasetSha256: null,
    },
    dataQuality: {
      valid: false, candleCount: Array.isArray(input?.candles) ? input.candles.length : 0, issues: [message],
      audit: auditCandleSeries([], input?.timeframe),
    },
    indicators: null,
    structure: { pivots: [], breaks: [], latestBreak: null, bias: 'unknown' },
    signals: [],
    decision: evaluateResearchDecision({ marketData: {}, signals: [], risk: input?.risk, intent: typeof input?.intent === 'string' ? input.intent : undefined }),
    authority: { orderIntent: null, executionAuthority: 'none', liveExecutionAuthorized: false },
  };
}

/**
 * Run a repeatable, bounded analysis over one immutable caller-provided candle snapshot.
 * `marketData.validated` must only be set after the caller's boundary validator passes.
 * @param {AnalysisInput} [input={}]
 */
export function buildTechnicalAnalysisSnapshot(input = {}) {
  const candidateCandles = input?.candles;
  if (!Array.isArray(candidateCandles)) return errorEnvelope('Candles must be supplied as an array.', input);
  if (candidateCandles.length > MAX_ANALYSIS_CANDLES) {
    return errorEnvelope(`Analysis input exceeds the ${MAX_ANALYSIS_CANDLES}-candle safety limit.`, input);
  }

  const candles = /** @type {Candle[]} */ (candidateCandles);
  const audit = auditCandleSeries(candles, input?.timeframe);
  const rawIndicators = calculateIndicatorSnapshot(candles);
  const auditIssues = audit.hardIssues.map((issue) => issue.message);
  // Temporal integrity is a core-engine invariant, not merely an adapter promise.
  // When the cadence/timeframe contract fails, suppress numeric indicators and signals
  // rather than calculating bar-count indicators over an irregular time series.
  const indicators = audit.status === 'invalid' && rawIndicators.status !== 'invalid_data'
    ? { ...rawIndicators, status: 'invalid_data', issues: [...new Set([...rawIndicators.issues, ...auditIssues])], metrics: null, signals: [] }
    : rawIndicators;
  const cutoff = asOfIso(candles, typeof input?.timeframe === 'string' ? input.timeframe : '');
  const datasetSha256 = typeof input?.marketData?.datasetSha256 === 'string'
    ? input.marketData.datasetSha256.toLowerCase()
    : null;
  const symbol = typeof input?.symbol === 'string' ? input.symbol.trim().toUpperCase() : '';
  const timeframe = typeof input?.timeframe === 'string' ? input.timeframe : '';

  /** @type {Array<import('./research-detectors.js').ClassifiedPivot>} */
  let pivots = [];
  /** @type {StructureBreak[]} */
  let breaks = [];
  /** @type {StructureBreak|null} */
  let latestBreak = null;
  /** @type {string[]} */
  let structureIssues = [];
  if (indicators.status !== 'invalid_data' && candles.length >= 3) {
    try {
      pivots = classifySwingStructure(detectConfirmedSwings(candles, { leftBars: 2, rightBars: 2 }));
      breaks = detectStructureBreaks(candles, pivots);
      latestBreak = breaks.at(-1) ?? null;
    } catch (error) {
      structureIssues = [error instanceof Error ? error.message : 'Market-structure calculation failed.'];
    }
  } else if (indicators.status !== 'invalid_data') {
    structureIssues = ['At least three valid candles are required for causal swing/structure research.'];
  }

  const dataQualityValid = indicators.status !== 'invalid_data' && audit.status !== 'invalid';
  const asOf = cutoff;
  const signals = dataQualityValid && asOf
    ? [
        ...indicators.signals.map((signal) => enrichSignal(signal, asOf, datasetSha256)),
        ...(candles.length >= 3 && structureIssues.length === 0 ? [signalForStructure(latestBreak, asOf, datasetSha256)] : []),
      ]
    : [];

  const suppliedMarketData = input?.marketData ?? {};
  const suppliedCloseAtMs = suppliedMarketData.latestClosedBarAtMs;
  const cutoffMismatch = (suppliedMarketData.asOf != null && suppliedMarketData.asOf !== asOf)
    || (suppliedCloseAtMs != null && suppliedCloseAtMs !== getLatestClosedBarAtMs(candles, timeframe))
    || (suppliedMarketData.validated === true && suppliedCloseAtMs == null);
  const scopeMismatch = (typeof suppliedMarketData.symbol === 'string'
      && suppliedMarketData.symbol.trim().toUpperCase() !== symbol)
    || (typeof suppliedMarketData.timeframe === 'string'
      && suppliedMarketData.timeframe !== timeframe);
  const canonicalMarketData = {
    // Metadata conflicts are integrity failures, not warnings to be silently corrected.
    validated: suppliedMarketData.validated === true && dataQualityValid && !cutoffMismatch && !scopeMismatch,
    source: suppliedMarketData.source,
    asOf,
    datasetSha256,
    symbol,
    timeframe,
  };
  const evaluatedDecision = evaluateResearchDecision({
    marketData: canonicalMarketData,
    signals,
    risk: input?.risk,
    intent: typeof input?.intent === 'string' ? input.intent : 'research',
  });
  // Keep the candle observations visible, but do not allow a statistically suspicious
  // latest bar to advance to a research candidate without an external review process.
  // The audit warning is not proof that the price is false; it is a handoff barrier.
  const decision = audit.warnings.length > 0
    ? Object.freeze({
        ...evaluatedDecision,
        decision: 'BLOCK',
        technicalBias: 'untrusted',
        reasons: Object.freeze([...new Set([
          ...evaluatedDecision.reasons,
          'Latest-bar anomaly warning requires review; research candidate handoff is blocked while the warning remains.',
        ])]),
        orderIntent: null,
        executionAuthority: 'none',
        liveExecutionAuthorized: false,
      })
    : evaluatedDecision;

  const issues = [...indicators.issues, ...structureIssues, ...auditIssues, ...audit.warnings.map((warning) => warning.message)];
  if (cutoffMismatch) {
    issues.push('Provider cutoff differs from the latest fully closed candle timestamp or supplied close metadata is missing; the snapshot is blocked rather than silently re-pinned.');
  }
  if (scopeMismatch) {
    issues.push('Provider symbol or timeframe scope differs from the requested analysis scope; the snapshot is blocked.');
  }
  const status = !dataQualityValid ? 'invalid_data'
    : audit.warnings.length > 0 ? 'partial'
      : candles.length < 34 ? 'partial'
      : structureIssues.length > 0 ? 'partial' : 'ready';

  return {
    contractVersion: TECHNICAL_ANALYSIS_ENGINE_CONTRACT,
    engineVersion: TECHNICAL_ANALYSIS_ENGINE_VERSION,
    status,
    mode: 'research_only',
    identity: Object.freeze({
      symbol: symbol || null,
      timeframe: timeframe || null,
      asOf,
      datasetSha256,
      source: typeof suppliedMarketData.source === 'string' ? suppliedMarketData.source : null,
    }),
    dataQuality: Object.freeze({ valid: dataQualityValid, candleCount: candles.length, issues: [...new Set(issues)], audit }),
    indicators,
    structure: Object.freeze({
      pivots: pivots.slice(-40),
      breaks: breaks.slice(-40),
      latestBreak,
      bias: latestBreak?.type?.endsWith('_bullish') ? 'bullish'
        : latestBreak?.type?.endsWith('_bearish') ? 'bearish' : 'unknown',
    }),
    signals,
    decision,
    authority: Object.freeze({ orderIntent: null, executionAuthority: 'none', liveExecutionAuthorized: false }),
  };
}
