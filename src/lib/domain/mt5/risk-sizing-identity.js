/**
 * @param {import('../../types').Mt5BridgeStatus | null} status
 * @param {import('../../types').Mt5RiskSizingResult | null} result
 * @param {string} activeSymbol
 * @returns {boolean}
 */
export function isMt5RiskSizingResultStale(status, result, activeSymbol) {
  if (!result) return false;

  return !status?.connected
    || status.stale
    || status.quoteStale
    || status.lastSequence !== result.snapshotSequence
    || status.accountServer !== result.brokerServer
    || status.accountLogin !== result.accountLogin
    || status.symbol !== result.symbol
    || typeof activeSymbol !== 'string'
    || activeSymbol.trim() !== result.symbol;
}
