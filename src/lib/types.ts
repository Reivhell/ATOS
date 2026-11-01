export interface TechnicalIndicatorReading {
  key: string;
  title: string;
  value: string;
  detail: string;
  available: boolean;
}

export interface Candle {
  time: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
}

export interface Quote {
  symbol: string;
  price: number;
  change24hPct: number;
  volume24h: number;
  source: string;
  updatedAtMs: number;
  retrievedAtMs: number;
  isLive: boolean;
}

export interface Position {
  symbol: string;
  quantity: number;
  averageEntry: number;
  markPrice: number;
  notional: number;
  unrealizedPnl: number;
}

export interface RiskCheck {
  rule: string;
  passed: boolean;
  detail: string;
}

export interface PaperOrder {
  id: string;
  clientOrderId?: string | null;
  symbol: string;
  side: string;
  quantity: number;
  requestedPrice: number;
  requestedLimitPrice?: number | null;
  fillPrice?: number | null;
  notional: number;
  fee: number;
  realizedPnl: number;
  status: string;
  createdAtMs: number;
  riskChecks: RiskCheck[];
}

export interface Agent {
  id: string;
  name: string;
  role: string;
  status: string;
  mode: string;
  objective: string;
  lastDecision: string;
  pnl: number;
  tools: string[];
  updatedAtMs: number;
}

export interface AuditEvent {
  id: string;
  timestampMs: number;
  level: string;
  category: string;
  title: string;
  detail: string;
}

export interface ScreenerToken {
  symbol: string;
  name: string;
  chain: string;
  price: number;
  change1hPct: number;
  change24hPct: number;
  volume24h: number;
  liquidity: number;
  momentumScore: number;
  riskScore: number;
  status: string;
  source: string;
  updatedAtMs: number;
  riskFlags: string[];
  tokenAddress?: string | null;
  pairAddress?: string | null;
  pairUrl?: string | null;
  priceChange5mPct?: number | null;
  marketCapUsd?: number | null;
  fdvUsd?: number | null;
  ageMinutes?: number | null;
  buys24h?: number | null;
  sells24h?: number | null;
  boostActive?: number | null;
  rulesetVersion: string;
}

export interface MemeCoinHunterSecurityStatus {
  credentialConfigured: boolean;
  secureStoreAvailable: boolean;
  provider: string;
  note: string;
}

export interface MemeCoinHunterSourceStatus {
  provider: string;
  state: string;
  discoveredCount: number;
  receivedAtMs: number | null;
  detail: string;
}

export interface MemeCoinHunterCandidate {
  chain: string;
  tokenAddress: string;
  pairAddress: string;
  pairUrl: string | null;
  symbol: string;
  name: string;
  priceUsd: number;
  marketCapUsd: number | null;
  fdvUsd: number | null;
  liquidityUsd: number;
  volume5mUsd: number | null;
  volume1hUsd: number | null;
  volume24hUsd: number;
  priceChange5mPct: number | null;
  priceChange1hPct: number | null;
  priceChange24hPct: number | null;
  buys5m: number | null;
  sells5m: number | null;
  buys1h: number | null;
  sells1h: number | null;
  buyPressure5mPct: number | null;
  volumeVelocity5m: number | null;
  poolAgeMinutes: number | null;
  holderTop10Pct: number | null;
  liquidityLockedTop10Pct: number | null;
  buyTaxPct: number | null;
  sellTaxPct: number | null;
  securityState: string;
  securityCheckedAtMs: number | null;
  securityFlags: string[];
  discoverySources: string[];
  riskScore: number;
  scalpScore: number;
  scalpState: string;
  scalpReasons: string[];
  receivedAtMs: number;
}

export interface MemeCoinHunterReport {
  contractVersion: string;
  runId: string;
  scannedAtMs: number;
  providerState: string;
  securityProviderConfigured: boolean;
  discoveredCount: number;
  candidateCount: number;
  scalpSetupCount: number;
  blockedCount: number;
  datasetFingerprintSha256: string;
  sources: MemeCoinHunterSourceStatus[];
  candidates: MemeCoinHunterCandidate[];
  executionAuthority: string;
  decisionLimitations: string[];
}

export interface RiskLimits {
  maxOrderNotional: number;
  maxPositionNotional: number;
  maxDailyLoss: number;
  feeBps: number;
  slippageBps: number;
}

export type CandleInterval = '1m' | '5m' | '15m' | '1h' | '4h' | '1d';

export interface CreateResearchHypothesisRequest {
  title: string;
  thesis: string;
  symbol: string;
  interval: CandleInterval;
}

export interface ResearchHypothesis {
  id: string;
  title: string;
  thesis: string;
  symbol: string;
  interval: CandleInterval;
  status: string;
  revision: number;
  createdAtMs: number;
  updatedAtMs: number;
}

export interface ResearchWorkflowRequest {
  goal: string;
  hypothesisId?: string | null;
  symbol: string;
  interval: CandleInterval;
  fastPeriod: number;
  slowPeriod: number;
  initialCapital: number;
  maxDrawdownPct: number;
  minimumPositiveOosRatio: number;
  minimumSharpeRatio: number;
  strategyTournament: boolean;
}

export interface ResearchWorkflowNode {
  nodeId: string;
  dependsOn: string[];
  status: string;
  detail: string;
  completedAtMs: number;
}

export interface ResearchOutOfSampleReview {
  foldCount: number;
  positiveFoldCount: number;
  positiveFoldRatio: number | null;
  medianReturnPct: number | null;
  worstDrawdownPct: number | null;
  passed: boolean;
  blockers: string[];
}

export interface ResearchBlockBootstrapSummary {
  method: string;
  available: boolean;
  simulations: number;
  observations: number;
  blockLength: number;
  seedSha256: string;
  returnP05Pct: number | null;
  returnMedianPct: number | null;
  returnP95Pct: number | null;
  maxDrawdownP95Pct: number | null;
  unavailableReason: string | null;
  caveat: string;
}

export interface ResearchSpecialistReview {
  role: string;
  status: string;
  summary: string;
  metrics: Record<string, unknown>;
  blockers: string[];
  caveats: string[];
}

export interface ResearchBacktestEvidence {
  resultId: string;
  strategyName: string;
  fastPeriod: number;
  slowPeriod: number;
  initialCapital: number;
  finalEquity: number;
  totalReturnPct: number;
  maxDrawdownPct: number;
  winRatePct: number | null;
  profitFactor: number | null;
  grossProfit: number;
  grossLoss: number;
  sharpeRatio: number;
  tradeCount: number;
  feeBps: number;
  slippageBps: number;
  candlesUsed: number;
  dataSource: string;
  datasetSha256: string;
  datasetEncodingVersion: number;
  datasetStartMs: number;
  datasetEndMs: number;
  medianCandleIntervalMs: number;
  costStressScenarios: BacktestCostStressScenario[];
  outOfSampleFolds: BacktestOutOfSampleFold[];
  warnings: string[];
}

export interface ResearchGateParameters {
  maximumDrawdownPct: number;
  minimumPositiveOosRatio: number;
  minimumSharpeRatio: number;
}

export interface ResearchGateDecision {
  policyVersion: string;
  status: string;
  eligibleForPaperValidation: boolean;
  blockers: string[];
  caveats: string[];
}

export interface ResearchRunCard {
  schemaVersion: number;
  id: string;
  hypothesisId: string | null;
  hypothesisRevision: number | null;
  goal: string;
  symbol: string;
  interval: CandleInterval;
  createdAtMs: number;
  riskLimitsContext: RiskLimits;
  gateParameters: ResearchGateParameters;
  workflowGraph: ResearchWorkflowNode[];
  backtest: ResearchBacktestEvidence;
  outOfSampleReview: ResearchOutOfSampleReview;
  blockBootstrap: ResearchBlockBootstrapSummary;
  strategyTournament?: StrategyParameterTournament | null;
  specialistReviews: ResearchSpecialistReview[];
  decision: ResearchGateDecision;
}


export interface TournamentCandidateScore {
  fastPeriod: number;
  slowPeriod: number;
  trainingSharpeRatio: number;
  trainingReturnPct: number;
  trainingMaxDrawdownPct: number;
  trainingTradeCount: number;
  qualifiesForSelection: boolean;
}

export interface WalkForwardTournamentFold {
  foldIndex: number;
  trainingStartMs: number;
  trainingEndMs: number;
  testingStartMs: number;
  testingEndMs: number;
  selectedFastPeriod: number | null;
  selectedSlowPeriod: number | null;
  selectedTrainingSharpeRatio: number | null;
  outOfSampleReturnPct: number | null;
  outOfSampleMaxDrawdownPct: number | null;
  outOfSampleSharpeRatio: number | null;
  outOfSampleTradeCount: number | null;
  candidateScores: TournamentCandidateScore[];
  status: string;
}

export interface StrategyParameterTournament {
  algorithmVersion: string;
  selectionMetric: string;
  candidateCount: number;
  candidateEvaluations: number;
  requestedFolds: number;
  completedFolds: number;
  positiveOosFoldRatio: number | null;
  medianOosReturnPct: number | null;
  worstOosMaxDrawdownPct: number | null;
  parameterStabilityPct: number | null;
  datasetSha256: string;
  datasetStartMs: number;
  datasetEndMs: number;
  folds: WalkForwardTournamentFold[];
  blockers: string[];
  caveats: string[];
}

export interface ResearchRunCardRecord {
  card: ResearchRunCard;
  contentSha256: string;
}

export interface ResearchIntegrityStatus {
  valid: boolean;
  eventCount: number;
  runCardCount: number;
  chainHeadSha256: string;
  verifiedAtMs: number;
}

export interface CandleSeriesRequest {
  symbol: string;
  interval: CandleInterval;
}

export interface CandleSeriesResponse {
  request: CandleSeriesRequest;
  source: string;
  providerStatus: string;
  updatedAtMs: number;
  retrievedAtMs: number;
  latestClosedBarAtMs: number;
  datasetSha256: string;
  isLive: boolean;
  candles: Candle[];
}

/** Immutable frontend snapshot returned only after the market-data domain validator. */
export interface ValidatedCandleSeriesSnapshot {
  readonly request: Readonly<CandleSeriesRequest>;
  readonly source: string;
  readonly providerStatus: string;
  readonly updatedAtMs: number;
  readonly retrievedAtMs: number;
  readonly latestClosedBarAtMs: number;
  readonly datasetSha256: string;
  readonly isLive: boolean;
  readonly candles: ReadonlyArray<Readonly<Candle>>;
}

export interface CandleSeriesCapability {
  symbol: string;
  interval: CandleInterval;
  supported: boolean;
  available: boolean;
  isLive: boolean;
  candleCount: number;
  latestCandleTimeMs: number | null;
  latestClosedBarAtMs: number | null;
  retrievedAtMs: number;
  source: string;
  note: string;
}

export interface MarketDataCapabilities {
  quoteSymbols: string[];
  candleSeries: CandleSeriesCapability[];
}

export interface DashboardSnapshot {
  mode: string;
  initialBalance: number;
  cashBalance: number;
  equity: number;
  realizedPnl: number;
  unrealizedPnl: number;
  dayChangePct: number;
  maxDrawdownPct: number;
  winRatePct: number | null;
  closedTradeCount: number;
  positions: Position[];
  orders: PaperOrder[];
  agents: Agent[];
  events: AuditEvent[];
  quotes: Quote[];
  candles: Candle[];
  marketDataCapabilities: MarketDataCapabilities;
  screener: ScreenerToken[];
  screenerProviderStatus: string;
  screenerProviderSource: string;
  screenerProviderUpdatedAtMs: number;
  killSwitchEngaged: boolean;
  providerStatus: string;
  providerSource: string;
  providerUpdatedAtMs: number;
  riskLimits: RiskLimits;
}

export interface PlaceOrderRequest {
  symbol: string;
  side: 'BUY' | 'SELL';
  quantity: number;
  limitPrice?: number | null;
  clientOrderId?: string | null;
}

export interface BacktestRequest {
  symbol: string;
  fastPeriod: number;
  slowPeriod: number;
  initialCapital: number;
  feeBps: number;
  slippageBps: number;
}

export interface BacktestPoint {
  time: number;
  equity: number;
  benchmark: number;
}

export interface BacktestCostStressScenario {
  name: string;
  feeBps: number;
  slippageBps: number;
  feeCapReached: boolean;
  slippageCapReached: boolean;
  finalEquity: number;
  totalReturnPct: number;
  returnDeltaPercentagePoints: number;
  maxDrawdownPct: number;
  sharpeRatio: number;
  tradeCount: number;
}

export interface BacktestOutOfSampleFold {
  foldIndex: number;
  startMs: number;
  endMs: number;
  candlesEvaluated: number;
  finalEquity: number;
  totalReturnPct: number;
  maxDrawdownPct: number;
  sharpeRatio: number;
  tradeCount: number;
}

export interface BacktestResult {
  id: string;
  symbol: string;
  strategyName: string;
  fastPeriod: number;
  slowPeriod: number;
  initialCapital: number;
  finalEquity: number;
  totalReturnPct: number;
  maxDrawdownPct: number;
  winRatePct: number | null;
  profitFactor: number | null;
  grossProfit: number;
  grossLoss: number;
  sharpeRatio: number;
  tradeCount: number;
  feeBps: number;
  slippageBps: number;
  candlesUsed: number;
  dataSource: string;
  datasetSha256: string;
  datasetEncodingVersion: number;
  datasetStartMs: number;
  datasetEndMs: number;
  medianCandleIntervalMs: number;
  equityCurve: BacktestPoint[];
  costStressScenarios: BacktestCostStressScenario[];
  outOfSampleFolds: BacktestOutOfSampleFold[];
  warnings: string[];
}

export type StrictAssessmentStatus = 'evaluated' | 'insufficient_data' | 'not_applicable' | 'unsupported';
export type StrictTradingIntent = 'research' | 'paper_trade';

export interface StrictCatalogEntry {
  id: string;
  category: string;
  group: string;
  name: string;
  availability: string;
  kind: string;
}

export interface StrictAnalysisTask {
  agentId: string;
  category: string;
  knowledgePackPath: string;
  requiredAssessmentCount: number;
  catalogIds: string[];
}

export interface CreateStrictWorkPlanRequest {
  symbol: string;
  timeframe: string;
  asOf: string;
  datasetSha256: string;
}

export interface StrictAnalysisWorkPlan {
  workPlanId: string;
  schemaVersion: string;
  decisionGateContractVersion: string;
  assessmentContractVersion: string;
  masterListVersion: string;
  symbol: string;
  timeframe: string;
  asOf: string;
  datasetSha256: string;
  expectedAssessmentCount: number;
  inventoryIdentity: string;
  dispatchStatus: 'work_plan_created_not_dispatched';
  liveExecutionAuthorized: false;
  entries: StrictCatalogEntry[];
  tasks: StrictAnalysisTask[];
}

export interface StrictAssessment {
  contractVersion: string;
  masterListVersion: string;
  catalogId: string;
  category: string;
  group: string;
  name: string;
  agentId: string;
  status: StrictAssessmentStatus;
  asOf: string;
  evidenceRefs?: string[];
  result?: unknown;
  reason?: string;
  lookAheadSafe?: boolean;
  implementationVersion?: string;
  dataSnapshotId?: string;
}

export interface StrictMarketDataContext {
  validated: boolean;
  source: string;
  asOf: string;
  datasetSha256: string;
  symbol: string;
  timeframe: string;
  candles: Candle[];
}

export interface StrictRiskApproval {
  passed: boolean;
  reason?: string;
}

export interface StrictCompletionRequest {
  workPlan: StrictAnalysisWorkPlan;
  assessments: StrictAssessment[];
  /** Compatibility mirror only; native Rust overwrites these fields from trusted state. */
  marketData: StrictMarketDataContext;
  /** Untrusted compatibility field; native Rust ignores caller-supplied approval and derives preflight itself. */
  risk: StrictRiskApproval;
  intent: StrictTradingIntent;
}

export interface StrictCategoryCoverage {
  required: number;
  received: number;
  evaluated: number;
  unsupported: number;
  insufficientData: number;
  notApplicable: number;
  missing: number;
}

export interface StrictCoverage {
  required: number;
  received: number;
  assessed: number;
  missing: number;
  duplicate: number;
  malformed: number;
  unsupported: number;
  insufficientData: number;
  notApplicable: number;
  evaluated: number;
  catalogOnly: number;
  byCategory: Record<string, StrictCategoryCoverage>;
}

export interface StrictNextLayerPayload {
  contractVersion: string;
  assessmentContractVersion: string;
  masterListVersion: string;
  assessmentCoverage: string;
  symbol: string;
  timeframe: string;
  marketDataSource: string;
  marketDataAsOf: string;
  datasetSha256: string;
  riskApproved: boolean;
  intent: StrictTradingIntent;
  liveExecutionAuthorized: false;
}

export interface StrictAuditChainStatus {
  valid: boolean;
  eventCount: number;
  lastSequence: number;
  headSha256: string;
}

export interface StrictAuditReceipt {
  schemaVersion: 'strict-analysis-audit.v1';
  workPlanId: string;
  masterListVersion: string;
  symbol: string;
  timeframe: string;
  asOf: string;
  datasetSha256: string;
  assessmentBundleSha256: string;
  categoryAssessmentSha256: Record<string, string>;
  coverageRequired: number;
  coverageReceived: number;
  coverageAssessed: number;
  coverageMissing: number;
  coverageDuplicate: number;
  coverageMalformed: number;
  coverageUnsupported: number;
  coverageInsufficientData: number;
  coverageNotApplicable: number;
  coverageEvaluated: number;
  coverageCatalogOnly: number;
  decision: 'BLOCK' | 'ALLOW_RESEARCH' | 'ALLOW_TO_NEXT_LAYER';
  reasonsSha256: string;
  generatedAt: string;
  receiptSha256: string;
  liveExecutionAuthorized: false;
}

export interface StrictCompletionResult {
  allowed: boolean;
  decision: 'BLOCK' | 'ALLOW_RESEARCH' | 'ALLOW_TO_NEXT_LAYER';
  reasons: string[];
  coverage: StrictCoverage;
  auditReceipt: StrictAuditReceipt;
  nextLayerPayload: StrictNextLayerPayload | null;
  liveExecutionAuthorized: false;
}

export interface Mt5AccountSnapshot {
  login: string;
  server: string;
  currency: string;
  balance: number;
  equity: number;
  credit: number;
  profit: number;
  margin: number;
  freeMargin: number;
  marginLevelPct: number;
  marginInitial: number;
  marginMaintenance: number;
  marginCallLevel: number;
  stopOutLevel: number;
  marginStopoutMode: number;
  marginMode: number;
  accountHedgeAllowed: boolean;
  accountLimitOrders: number;
  leverage: number;
  tradeAllowed: boolean;
  accountTradeAllowed: boolean;
  terminalTradeAllowed: boolean;
}

export interface Mt5TerminalSnapshot {
  name: string;
  company: string;
  build: number;
  connected: boolean;
  algoTradingAllowed: boolean;
  serverTimeMs: number;
}

export interface Mt5SymbolSnapshot {
  name: string;
  bid: number;
  ask: number;
  point: number;
  tickSize: number;
  tickValue: number;
  volumeMin: number;
  volumeMax: number;
  volumeStep: number;
  digits: number;
  tradeMode: number;
  quoteTimeMs: number;
  contractSize: number;
  calculationMode: number;
  executionMode: number;
  fillingMode: number;
  orderMode: number;
  tickValueProfit: number;
  tickValueLoss: number;
  volumeLimit: number;
  stopsLevelPoints: number;
  freezeLevelPoints: number;
  currencyBase: string;
  currencyProfit: string;
  currencyMargin: string;
  swapMode: number;
  swapLong: number;
  swapShort: number;
}

export interface Mt5InstrumentSpecificationRecord {
  brokerServer: string;
  accountLogin: string;
  specification: Omit<Mt5SymbolSnapshot, 'name' | 'bid' | 'ask' | 'quoteTimeMs'> & { symbol: string };
  fingerprintSha256: string;
  revision: number;
  firstObservedAtMs: number;
  lastObservedAtMs: number;
  changedAtMs: number | null;
  completeness: 'broker_snapshot_v1_partial' | string;
}


export interface Mt5RiskSizingRequest {
  brokerServer: string;
  accountLogin: string;
  symbol: string;
  side: 'BUY' | 'SELL';
  stopLossPrice: number;
  riskPercent: number;
}

export interface Mt5RiskDecisionReceipt {
  decisionId: string;
  brokerServer: string;
  accountLogin: string;
  symbol: string;
  decision: 'SIZED_WITH_LIMITATIONS' | 'BLOCKED' | string;
  sequence: number;
  decidedAtMs: number;
  requestSha256: string;
  resultSha256: string;
  snapshotSequence: number;
  specificationRevision: number;
  specificationFingerprintSha256: string;
  previousSha256: string;
  eventSha256: string;
}

export interface Mt5RiskDecisionLedgerStatus {
  valid: boolean;
  brokerServer: string;
  accountLogin: string;
  eventCount: number;
  headSha256: string;
  firstInvalidSequence: number | null;
  integrityError: string | null;
}

export interface Mt5RiskSizingResult {
  decision: 'SIZED_WITH_LIMITATIONS' | 'BLOCKED' | string;
  reasons: string[];
  limitations: string[];
  brokerServer: string;
  accountLogin: string;
  symbol: string;
  side: 'BUY' | 'SELL';
  accountCurrency: string;
  equity: number;
  riskPercent: number;
  riskBudget: number;
  effectiveRiskBudget: number;
  accountAvailableRiskBudget: number;
  entryPrice: number;
  stopLossPrice: number;
  stopDistance: number;
  tickSize: number;
  tickValueLossPerLot: number;
  estimatedLossPerLot: number;
  rawVolume: number;
  proposedVolume: number;
  normalizedVolume: number;
  estimatedLoss: number;
  estimatedRiskPercent: number;
  volumeMin: number;
  volumeMax: number;
  volumeStep: number;
  existingSymbolVolume: number;
  pendingSymbolVolume: number;
  totalDirectionalSymbolVolume: number;
  volumeLimit: number;
  accountStopRisk: number;
  accountStopRiskPercent: number;
  unprotectedPositionCount: number;
  unprotectedPendingOrderCount: number;
  unresolvedPositionRiskCount: number;
  unresolvedPendingOrderRiskCount: number;
  maxAccountRiskPercent: number;
  maxAccountRiskBudget: number;
  projectedAccountStopRisk: number;
  candidateMargin: number | null;
  currentMargin: number | null;
  projectedMargin: number | null;
  projectedFreeMargin: number | null;
  projectedMarginLevelPct: number | null;
  brokerMarginCallLevel: number | null;
  brokerStopOutLevel: number | null;
  brokerStopoutMode: number | null;
  marginPreflightAtMs: number | null;
  marginEstimation: string;
  commissionEstimation: string;
  quoteAgeMs: number;
  specificationRevision: number;
  specificationFingerprintSha256: string;
  contractCompleteness: string;
  calculatedAtMs: number;
  snapshotSequence: number;
  auditReceipt: Mt5RiskDecisionReceipt;
}

export interface Mt5SessionWindow {
  index: number;
  fromSeconds: number;
  toSeconds: number;
}

export interface Mt5SessionDay {
  dayOfWeek: number;
  sessions: Mt5SessionWindow[];
}

export interface Mt5SessionCalendarRecord {
  brokerServer: string;
  accountLogin: string;
  schedule: { symbol: string; capturedAtMs: number; days: Mt5SessionDay[] };
  fingerprintSha256: string;
  revision: number;
  firstObservedAtMs: number;
  lastObservedAtMs: number;
  changedAtMs: number | null;
  timeBasis: string;
}

export interface Mt5PositionSnapshot {
  ticket: string;
  symbol: string;
  positionType: number;
  volume: number;
  priceOpen: number;
  priceCurrent: number;
  profit: number;
  swap: number;
  stopLoss: number;
  takeProfit: number;
  openedAtMs: number;
  magic: string;
  riskStatus: 'UNPROTECTED' | 'CALCULATED' | 'CALCULATION_FAILED';
  riskAtStopLoss: number | null;
}

export interface Mt5PendingOrderSnapshot {
  ticket: string;
  symbol: string;
  orderType: number;
  state: number;
  volumeInitial: number;
  volumeCurrent: number;
  priceOpen: number;
  priceCurrent: number;
  priceStopLimit: number;
  stopLoss: number;
  takeProfit: number;
  setupAtMs: number;
  expirationAtMs: number;
  magic: string;
  riskStatus: 'UNPROTECTED' | 'CALCULATED' | 'CALCULATION_FAILED';
  riskAtStopLoss: number | null;
}

export interface Mt5AccountRiskInventory {
  accountCurrency: string;
  equity: number;
  currentStopRisk: number;
  currentStopRiskPercent: number;
  protectedPositionCount: number;
  protectedPendingOrderCount: number;
  unprotectedPositionCount: number;
  unprotectedPendingOrderCount: number;
  unresolvedPositionRiskCount: number;
  unresolvedPendingOrderRiskCount: number;
  marginUsed: number;
  initialMargin: number;
  maintenanceMargin: number;
  freeMargin: number;
  marginLevelPct: number;
  marginCallLevel: number;
  stopOutLevel: number;
  marginStopoutMode: number;
}

export interface Mt5Snapshot {
  account: Mt5AccountSnapshot;
  terminal: Mt5TerminalSnapshot;
  symbol: Mt5SymbolSnapshot;
  positions: Mt5PositionSnapshot[];
  pendingOrders: Mt5PendingOrderSnapshot[];
}

export interface Mt5BridgeStatus {
  protocolVersion: number;
  listening: boolean;
  address: string;
  connected: boolean;
  stale: boolean;
  quoteStale: boolean;
  clientId: string | null;
  accountLogin: string | null;
  accountServer: string | null;
  symbol: string | null;
  lastHeartbeatAtMs: number | null;
  heartbeatAgeMs: number | null;
  lastSequence: number;
  lastError: string | null;
  snapshot: Mt5Snapshot | null;
  marketDataStorage: Mt5MarketDataStatus | null;
  accountRisk: Mt5AccountRiskInventory | null;
}

export interface Mt5MarketDataStatus {
  databasePath: string;
  ticksTotal: number;
  barsTotal: number;
  symbolsTotal: number;
  lastIngestAtMs: number | null;
  storageAvailable: boolean;
}

export interface Mt5SeriesQuery {
  brokerServer: string;
  accountLogin: string;
  symbol: string;
  timeframe: string;
  fromMs: number;
  toMs: number;
  afterMs: number | null;
  limit: number;
}

export interface Mt5StoredBar {
  symbol: string;
  timeframe: string;
  timeMs: number;
  open: number;
  high: number;
  low: number;
  close: number;
  tickVolume: number;
  spread: number;
  realVolume: number;
  payloadSha256: string;
  revision: number;
  firstReceivedAtMs: number;
  lastReceivedAtMs: number;
}

export interface Mt5SeriesGapCandidate {
  previousBarTimeMs: number;
  nextBarTimeMs: number;
  elapsedMs: number;
  nominalIntervalsElapsed: number;
}

export interface Mt5SeriesQuality {
  barsTotal: number;
  firstBarTimeMs: number | null;
  lastBarTimeMs: number | null;
  revisionsTotal: number;
  nominalIntervalMs: number;
  intervalAnomalyCount: number;
  largestIntervalMs: number | null;
  gapCandidates: Mt5SeriesGapCandidate[];
  continuityInterpretation: string;
  sessionCalendarRevision: number | null;
  sessionCalendarFingerprintSha256: string | null;
  sessionCalendarTimeBasis: string;
}

export interface Mt5SeriesQueryResponse {
  brokerServer: string;
  accountLogin: string;
  symbol: string;
  timeframe: string;
  fromMs: number;
  toMs: number;
  datasetFingerprintSha256: string;
  bars: Mt5StoredBar[];
  quality: Mt5SeriesQuality;
  hasMore: boolean;
  nextAfterMs: number | null;
}

export interface Mt5BridgeSetup {
  host: string;
  port: number;
  protocolVersion: number;
  pairingToken: string;
  expertAdvisorPath: string;
  transportSecurity: string;
}

export type MarketDataPurpose = 'research' | 'backtest' | 'brokerContext';
export type MarketDataAuthority = 'brokerTerminal' | 'publicExchange';
export type MarketDataProviderState = 'connected' | 'cached' | 'disconnected' | 'notLoaded' | 'degraded';
export type MarketDataFreshness = 'fresh' | 'stale' | 'futureDated' | 'unavailable';
export type MarketDataCapability = 'quoteSnapshot' | 'candleHistory' | 'instrumentSpecification' | 'sessionCalendar';
export type MarketDataVolumeBasis = 'baseAsset' | 'brokerTickCount' | 'brokerRealVolume';

export interface MarketDataProviderCatalogEntry {
  id: string;
  displayName: string;
  authority: MarketDataAuthority;
  venue: string;
  capabilities: MarketDataCapability[];
  supportedPurposes: MarketDataPurpose[];
  state: MarketDataProviderState;
  observedAtMs: number | null;
  limitations: string[];
}

export interface UnifiedMarketDataQuery {
  symbol: string;
  timeframe: string;
  fromMs: number;
  toMs: number;
  afterMs: number | null;
  limit: number;
  providerId?: string | null;
  purpose: MarketDataPurpose;
  maxDataAgeMs: number;
  allowStale: boolean;
}

export interface UnifiedMarketDataBar {
  timeMs: number;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
  volumeBasis: MarketDataVolumeBasis;
  tickVolume: number | null;
  realVolume: number | null;
  revision: number | null;
}

export interface UnifiedMarketDataResult {
  schemaVersion: number;
  request: {
    symbol: string;
    timeframe: string;
    fromMs: number;
    toMs: number;
    afterMs: number | null;
    limit: number;
    providerId: string;
    purpose: MarketDataPurpose;
    maxDataAgeMs: number;
    allowStale: boolean;
  };
  provenance: {
    providerId: string;
    providerName: string;
    authority: MarketDataAuthority;
    venue: string;
    retrievedAtMs: number;
    latestObservationAtMs: number | null;
    latestReceivedAtMs: number | null;
    datasetFingerprintSha256: string;
    scopeFingerprintSha256: string | null;
    isLive: boolean;
    sourceDetails: string;
  };
  quality: {
    freshness: MarketDataFreshness;
    freshnessAgeMs: number | null;
    freshnessLimitMs: number;
    candleCount: number;
    revisionCount: number;
    intervalAnomalyCount: number;
    gapCandidates: Array<{
      previousBarTimeMs: number;
      nextBarTimeMs: number;
      elapsedMs: number;
      nominalIntervalsElapsed: number;
    }>;
    hasMore: boolean;
    continuityInterpretation: string;
  };
  bars: UnifiedMarketDataBar[];
  nextAfterMs: number | null;
}
