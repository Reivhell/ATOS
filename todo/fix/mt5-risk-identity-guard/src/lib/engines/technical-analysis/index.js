/** Public entry point for numerical indicators, causal research detectors, and their coordinator. */
export { calculateIndicatorReadings, calculateIndicatorSnapshot } from './indicators.js';
export { detectConfirmedSwings, classifySwingStructure, detectStructureBreaks } from './research-detectors.js';
export { buildTechnicalAnalysisSnapshot, TECHNICAL_ANALYSIS_ENGINE_CONTRACT, TECHNICAL_ANALYSIS_ENGINE_VERSION, MAX_ANALYSIS_CANDLES } from './analysis-engine.js';
