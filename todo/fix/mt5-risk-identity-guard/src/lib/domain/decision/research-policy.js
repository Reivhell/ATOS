/** Reviewed thresholds for the deterministic research verdict. Values are policy, not learned parameters. */
export const RESEARCH_DECISION_POLICY = Object.freeze({
  minimumDistinctDomains: 3,
  minimumDirectionalDomains: 3,
  minimumDirectionalAgreement: 2 / 3,
  minimumMeanSignalStrength: 0.6,
  domainWeights: Object.freeze({
    trend: 2,
    momentum: 1,
    structure: 2,
    location: 1,
  }),
  allowedIntents: Object.freeze(['research', 'paper_trade']),
  liveExecutionAuthorized: false,
  orderIntentEmissionEnabled: false,
});
