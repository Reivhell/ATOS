/**
 * Explicit layered API for internal modules and tests.
 * Legacy file-level entry points remain available while imports migrate gradually.
 */
export * as MarketDataDomain from './domain/market-data/market-data-model.js';
export * as TechnicalAnalysisCatalog from './domain/technical-analysis/catalog.js';
export * as DecisionContracts from './domain/decision/contracts.js';
export * as TechnicalAnalysisEngine from './engines/technical-analysis/indicators.js';
export * as TechnicalResearchDetectors from './engines/technical-analysis/research-detectors.js';
export * as TechnicalAnalysis from './engines/technical-analysis/index.js';
export * as TechnicalAnalysisOrchestrator from './engines/technical-analysis/analysis-engine.js';
export * as ResearchDecisionEngine from './engines/decision/research-decision-engine.js';
export * as StrictDecisionEngine from './engines/decision/strict-decision-gate.js';
export * as StrictAnalysisApplication from './application/strict-analysis-orchestrator.js';
export * as TechnicalAnalysisApplication from './application/technical-analysis-service.js';
export * as DashboardPresentation from './presentation/dashboard/dashboard-model.js';
export * as ExportInfrastructure from './infrastructure/serialization/exports.js';
