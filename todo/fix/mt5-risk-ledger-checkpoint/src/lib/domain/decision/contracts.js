/**
 * Versioned domain contracts and reviewed policy constants for analysis decisions.
 * Keep this module framework-free and immutable; UI/provider payloads cannot alter it.
 */
export const ASSESSMENT_CONTRACT_VERSION = 'strict-ta-assessment.v1';
export const DECISION_GATE_CONTRACT_VERSION = 'strict-ta-decision-gate.v4';
export const ASSESSMENT_STATUSES = Object.freeze([
  'evaluated',
  'insufficient_data',
  'not_applicable',
  'unsupported',
]);

export const REVIEWED_IMPLEMENTATION_VERSION_BY_CATALOG_ID = Object.freeze({
  'Indikator Teknikal::Trend::SMA (Simple Moving Average)': "technical-analysis-calculators.v1",
  'Indikator Teknikal::Trend::EMA (Exponential Moving Average)': "technical-analysis-calculators.v1",
  'Indikator Teknikal::Momentum::RSI (Relative Strength Index)': "technical-analysis-calculators.v1",
  'Indikator Teknikal::Momentum::MACD (Moving Average Convergence Divergence)': "technical-analysis-calculators.v1",
  'Indikator Teknikal::Volatility::ATR (Average True Range)': "technical-analysis-calculators.v1",
  'Indikator Teknikal::Volatility::Bollinger Bands': "technical-analysis-calculators.v1",
  'Risk Management & Evaluation::Strategy Validation & Journal::Backtesting': "rust-backtest-engine.v1",
});

export const SPECIALIST_AGENT_BY_CATEGORY = Object.freeze({
  'SMC & ICT': 'agent-01-smc-ict',
  'Price Action': 'agent-02-price-action',
  'Support, Resistance & Trend': 'agent-03-support-resistance-trend',
  'Indikator Teknikal': 'agent-04-technical-indicators',
  'Order Flow & Market Profile': 'agent-05-order-flow-market-profile',
  'Divergence & Intermarket Analysis': 'agent-06-divergence-intermarket',
  'Metode Analisis Lain': 'agent-07-other-methods',
  'Fundamental & Makro': 'agent-08-fundamental-macro',
  'Risk Management & Evaluation': 'agent-09-risk-management-evaluation',
});


export const DECISION_POLICY = Object.freeze({
  requiredCatalogEntries: 364,
  minimumReasonLength: 12,
  allowedIntents: Object.freeze(['research', 'paper_trade']),
  liveExecutionAuthorized: false,
  requiresNativeAuthorization: true,
});
