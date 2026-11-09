/**
 * Application boundary for technical-analysis requests. UI code submits a validated
 * candle snapshot here; it does not call engines directly or promote object-shaped data
 * to trusted input. This is frontend integrity plumbing, not a substitute for Rust
 * authority or a native risk receipt.
 */
import { isCurrentLiveCandleSeriesSnapshot, isValidatedCandleSeriesSnapshot } from '../domain/market-data/market-data-model.js';
import { calculateIndicatorReadings } from '../engines/technical-analysis/indicators.js';
import { buildTechnicalAnalysisSnapshot } from '../engines/technical-analysis/analysis-engine.js';

export const TECHNICAL_ANALYSIS_APPLICATION_CONTRACT = 'technical-analysis-application.v1';

/** @param {unknown} value @returns {string} */
function normalizeSymbol(value) {
  return typeof value === 'string' ? value.trim().toUpperCase() : '';
}

/**
 * @param {{validatedSeries?:import('../types').ValidatedCandleSeriesSnapshot|null,requestedSymbol?:string,requestedTimeframe?:string}} [request={}]
 */
export function createTechnicalAnalysisReport(request = {}) {
  const series = request?.validatedSeries ?? null;
  const trustedSnapshot = isValidatedCandleSeriesSnapshot(series);
  const requestedSymbol = normalizeSymbol(request?.requestedSymbol ?? series?.request?.symbol);
  const requestedTimeframe = request?.requestedTimeframe ?? series?.request?.interval ?? '';
  const seriesSymbol = trustedSnapshot ? series.request.symbol : '';
  const seriesTimeframe = trustedSnapshot ? series.request.interval : '';
  const scopeMatches = trustedSnapshot
    && requestedSymbol === seriesSymbol
    && requestedTimeframe === seriesTimeframe;
  const candles = trustedSnapshot ? [...series.candles] : [];
  const boundaryIssues = [];
  if (!trustedSnapshot) boundaryIssues.push('No immutable candle snapshot produced by the market-data validation boundary was supplied.');
  else if (!scopeMatches) boundaryIssues.push('Requested instrument or timeframe does not match the validated candle snapshot.');
  const liveAtStart = scopeMatches && isCurrentLiveCandleSeriesSnapshot(series);
  const researchEligibleAtStart = scopeMatches && liveAtStart;

  const asOfMs = trustedSnapshot ? series.latestClosedBarAtMs : null;
  const asOf = typeof asOfMs === 'number' && Number.isSafeInteger(asOfMs) && asOfMs > 0 ? new Date(asOfMs).toISOString() : null;
  const analysis = buildTechnicalAnalysisSnapshot({
    candles,
    symbol: requestedSymbol,
    timeframe: requestedTimeframe,
    marketData: {
      validated: researchEligibleAtStart,
      source: trustedSnapshot ? series.source : '',
      asOf,
      latestClosedBarAtMs: asOfMs,
      // The digest is issued by Rust over the exact OHLCV payload; the UI never recomputes or invents it.
      datasetSha256: trustedSnapshot ? series.datasetSha256 : null,
      symbol: trustedSnapshot ? seriesSymbol : null,
      timeframe: trustedSnapshot ? seriesTimeframe : null,
    },
    intent: 'research',
  });
  const liveSnapshot = scopeMatches && isCurrentLiveCandleSeriesSnapshot(series);
  const researchEligible = scopeMatches && liveSnapshot;
  if (scopeMatches && !liveSnapshot) boundaryIssues.push('The validated candle snapshot expired or became stale before the analysis result was committed.');
  const existingDecisionReasons = /** @type {{reasons?: readonly string[]}} */ (analysis.decision).reasons ?? [];
  const decision = researchEligible ? analysis.decision : Object.freeze({
    ...analysis.decision,
    decision: 'BLOCK',
    technicalBias: 'untrusted',
    reasons: Object.freeze([...new Set([
      ...existingDecisionReasons,
      'A current live provider-backed candle snapshot is required at the final decision boundary.',
    ])]),
    orderIntent: null,
    executionAuthority: 'none',
    liveExecutionAuthorized: false,
  });
  const issues = [...boundaryIssues, ...analysis.dataQuality.issues];
  const status = !researchEligible ? 'blocked' : analysis.status;
  const dataQuality = Object.freeze({
    ...analysis.dataQuality,
    valid: researchEligible && analysis.dataQuality.valid,
    issues: [...new Set(issues)],
  });
  return Object.freeze({
    ...analysis,
    applicationContractVersion: TECHNICAL_ANALYSIS_APPLICATION_CONTRACT,
    status,
    dataQuality,
    decision,
    indicatorReadings: calculateIndicatorReadings(candles),
    validationBoundary: Object.freeze({
      passed: scopeMatches,
      trustedSnapshot,
      scopeMatches,
      liveSnapshot,
      reason: boundaryIssues[0] ?? null,
    }),
    // A UI-only result must never be promoted into an order or execution grant.
    authority: Object.freeze({ orderIntent: null, executionAuthority: 'none', liveExecutionAuthorized: false }),
  });
}
