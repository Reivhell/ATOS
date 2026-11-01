/**
 * JavaScript-side planning and parity diagnostics for the strict 364-concept analysis.
 * This module never authorizes a downstream handoff. The authoritative work-plan/completion
 * commands live in Rust (`src-tauri/src/strict_analysis.rs`); no LLM dispatch is claimed here.
 */
import { evaluateStrictDecisionGate } from '../engines/decision/strict-decision-gate.js';
import { ASSESSMENT_CONTRACT_VERSION, DECISION_GATE_CONTRACT_VERSION, SPECIALIST_AGENT_BY_CATEGORY } from '../domain/decision/contracts.js';
import { STRICT_CATALOG_REGISTRY, STRICT_MASTER_LIST_VERSION } from '../domain/decision/strict-catalog-registry.js';

export const STRICT_WORK_PLAN_VERSION = 'strict-ta-work-plan.v1';
const EXPECTED_COUNT = 364;
const KNOWLEDGE_PACK_BY_CATEGORY = Object.freeze({
  'SMC & ICT': 'docs/agents/01-smc-ict-agent.md',
  'Price Action': 'docs/agents/02-price-action-agent.md',
  'Support, Resistance & Trend': 'docs/agents/03-support-resistance-trend-agent.md',
  'Indikator Teknikal': 'docs/agents/04-technical-indicators-agent.md',
  'Order Flow & Market Profile': 'docs/agents/05-order-flow-market-profile-agent.md',
  'Divergence & Intermarket Analysis': 'docs/agents/06-divergence-intermarket-agent.md',
  'Metode Analisis Lain': 'docs/agents/07-other-methods-agent.md',
  'Fundamental & Makro': 'docs/agents/08-fundamental-macro-agent.md',
  'Risk Management & Evaluation': 'docs/agents/09-risk-management-evaluation-agent.md',
});

/** @typedef {typeof STRICT_CATALOG_REGISTRY[number]} CatalogEntry */
/** @typedef {keyof typeof SPECIALIST_AGENT_BY_CATEGORY} SpecialistCategory */
/** @typedef {{agentId:string,category:SpecialistCategory,knowledgePackPath:string,requiredAssessmentCount:number,catalogIds:string[]}} SpecialistTask */
/** @typedef {{schemaVersion:string,decisionGateContractVersion:string,assessmentContractVersion:string,masterListVersion:string,symbol:string,timeframe:string,asOf:string,datasetSha256:string,expectedAssessmentCount:number,inventoryIdentity:string,dispatchStatus:string,liveExecutionAuthorized:false,entries:CatalogEntry[],tasks:SpecialistTask[]}} StrictWorkPlan */
/** @typedef {{symbol:string,timeframe:string,asOf:string,datasetSha256:string}} WorkPlanInput */
/** @typedef {{symbol?:string,timeframe?:string,asOf?:string,datasetSha256?:string}} MarketDataScope */
/** @typedef {{status?:string,dataSnapshotId?:string,catalogId?:string}} StrictAssessmentSummary */
/** @typedef {{passed?:boolean,reason?:string}} RiskSummary */
/** @typedef {{workPlan?:unknown,assessments?:StrictAssessmentSummary[],marketData?:MarketDataScope,risk?:RiskSummary,intent?:'research'|'paper_trade'}} CompletionInput */

/** @param {unknown} value */
function nonEmptyString(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

/** @param {unknown} value */
function validTimestamp(value) {
  return typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value) && Number.isFinite(Date.parse(value));
}

/** @param {unknown} value */
function validSha256(value) {
  return typeof value === 'string' && /^[a-f0-9]{64}$/i.test(value);
}

/** @param {readonly CatalogEntry[]} entries @returns {Map<SpecialistCategory, CatalogEntry[]>} */
function entriesByCategory(entries) {
  const grouped = new Map();
  for (const entry of entries) {
    const category = /** @type {SpecialistCategory} */ (entry.category);
    if (!grouped.has(category)) grouped.set(category, []);
    grouped.get(category).push(entry);
  }
  return grouped;
}

/** @param {unknown} entries @returns {entries is CatalogEntry[]} */
function sameInventory(entries) {
  if (!Array.isArray(entries) || entries.length !== EXPECTED_COUNT) return false;
  return entries.every((entry, index) => {
    const trusted = STRICT_CATALOG_REGISTRY[index];
    if (!entry || typeof entry !== 'object' || !trusted) return false;
    const candidate = /** @type {Partial<CatalogEntry>} */ (entry);
    return candidate.id === trusted.id && candidate.category === trusted.category &&
      candidate.group === trusted.group && candidate.name === trusted.name &&
      candidate.availability === trusted.availability && candidate.kind === trusted.kind;
  });
}

/** Build one deterministic task per specialist and assign every code-owned catalog entry once. */
/** @param {Partial<WorkPlanInput>} [input={}] */
export function createStrictAnalysisWorkPlan({ symbol, timeframe, asOf, datasetSha256 } = {}) {
  const normalizedSymbol = typeof symbol === 'string' ? symbol.trim() : '';
  const normalizedTimeframe = typeof timeframe === 'string' ? timeframe.trim() : '';
  if (!nonEmptyString(normalizedSymbol)) throw new TypeError('symbol is required to create a strict analysis work plan.');
  if (!nonEmptyString(normalizedTimeframe)) throw new TypeError('timeframe is required to create a strict analysis work plan.');
  if (!validTimestamp(asOf)) throw new TypeError('asOf must be a valid ISO-8601 timestamp.');
  if (!validSha256(datasetSha256)) throw new TypeError('datasetSha256 must be a 64-character SHA-256 hex digest.');

  const entries = STRICT_CATALOG_REGISTRY.map((entry) => ({ ...entry }));
  const grouped = entriesByCategory(entries);
  const tasks = [...grouped.entries()].map(([category, categoryEntries]) => ({
    agentId: SPECIALIST_AGENT_BY_CATEGORY[category],
    category,
    knowledgePackPath: KNOWLEDGE_PACK_BY_CATEGORY[category],
    requiredAssessmentCount: categoryEntries.length,
    catalogIds: categoryEntries.map((entry) => entry.id),
  }));

  return Object.freeze({
    schemaVersion: STRICT_WORK_PLAN_VERSION,
    decisionGateContractVersion: DECISION_GATE_CONTRACT_VERSION,
    assessmentContractVersion: ASSESSMENT_CONTRACT_VERSION,
    masterListVersion: STRICT_MASTER_LIST_VERSION,
    symbol: normalizedSymbol,
    timeframe: normalizedTimeframe,
    asOf: /** @type {string} */ (asOf),
    datasetSha256: /** @type {string} */ (datasetSha256).toLowerCase(),
    expectedAssessmentCount: EXPECTED_COUNT,
    inventoryIdentity: 'code-owned-strict-catalog-registry',
    dispatchStatus: 'work_plan_created_not_dispatched',
    liveExecutionAuthorized: false,
    entries,
    tasks,
  });
}

/** @param {unknown} workPlan */
function validateWorkPlan(workPlan) {
  const plan = workPlan && typeof workPlan === 'object'
    ? /** @type {Partial<StrictWorkPlan>} */ (workPlan)
    : undefined;
  const reasons = [];
  if (!plan || plan.schemaVersion !== STRICT_WORK_PLAN_VERSION) reasons.push('Missing or unsupported strict work-plan version.');
  if (plan?.decisionGateContractVersion !== DECISION_GATE_CONTRACT_VERSION) reasons.push('Work plan decision-gate contract version mismatch.');
  if (plan?.assessmentContractVersion !== ASSESSMENT_CONTRACT_VERSION) reasons.push('Work plan assessment contract version mismatch.');
  if (plan?.masterListVersion !== STRICT_MASTER_LIST_VERSION) reasons.push('Work plan master-list version mismatch.');
  if (!nonEmptyString(plan?.symbol) || !nonEmptyString(plan?.timeframe)) reasons.push('Work plan symbol/timeframe scope is missing.');
  if (!validTimestamp(plan?.asOf)) reasons.push('Work plan timestamp is missing or invalid.');
  if (!validSha256(plan?.datasetSha256)) reasons.push('Work plan SHA-256 snapshot fingerprint is missing or invalid.');
  if (plan?.expectedAssessmentCount !== EXPECTED_COUNT) reasons.push('Work plan must require exactly 364 assessments.');
  if (!sameInventory(plan?.entries)) reasons.push('Work plan inventory differs from the exact code-owned 364-entry registry.');

  const grouped = entriesByCategory(STRICT_CATALOG_REGISTRY);
  const expectedTasks = [...grouped.entries()].map(([category, categoryEntries]) => ({
    agentId: SPECIALIST_AGENT_BY_CATEGORY[category],
    category,
    knowledgePackPath: KNOWLEDGE_PACK_BY_CATEGORY[category],
    requiredAssessmentCount: categoryEntries.length,
    catalogIds: categoryEntries.map((entry) => entry.id),
  }));
  if (!Array.isArray(plan?.tasks) || plan.tasks.length !== expectedTasks.length ||
      JSON.stringify(plan.tasks) !== JSON.stringify(expectedTasks)) {
    reasons.push('Work plan specialist routing differs from the code-owned category assignment.');
  }
  return reasons;
}

/**
 * Completion barrier. `nextLayerPayload` exists only when every strict gate condition passes.
 * All output remains research/paper-only; the returned payload never authorizes live execution.
 * @param {CompletionInput} [input={}]
 */
export function evaluateStrictAnalysisCompletion({ workPlan, assessments, marketData, risk, intent } = {}) {
  const plan = workPlan && typeof workPlan === 'object'
    ? /** @type {Partial<StrictWorkPlan>} */ (workPlan)
    : undefined;
  const planReasons = validateWorkPlan(workPlan);
  const reasons = [...planReasons];
  if (marketData?.symbol !== plan?.symbol) reasons.push('Market-data symbol does not match the work-plan scope.');
  if (marketData?.timeframe !== plan?.timeframe) reasons.push('Market-data timeframe does not match the work-plan scope.');
  if (String(marketData?.datasetSha256 ?? '').toLowerCase() !== plan?.datasetSha256) reasons.push('Market-data fingerprint does not match the work-plan snapshot.');
  if (marketData?.asOf !== plan?.asOf) reasons.push('Market-data timestamp does not match the work-plan snapshot timestamp.');

  if (Array.isArray(assessments)) {
    for (const assessment of assessments) {
      if (assessment?.status === 'evaluated' && String(assessment.dataSnapshotId ?? '').toLowerCase() !== String(plan?.datasetSha256 ?? '').toLowerCase()) {
        reasons.push(`Assessment dataSnapshotId does not match the work-plan snapshot: ${assessment?.catalogId ?? 'unknown'}.`);
      }
    }
  }

  const gateResult = evaluateStrictDecisionGate({
    catalog: sameInventory(plan?.entries) ? plan.entries : [],
    assessments: Array.isArray(assessments) ? assessments : [],
    marketData,
    risk,
    intent,
  });
  reasons.push(...gateResult.reasons);
  const mirrorRulesSatisfied = planReasons.length === 0 && reasons.length === 0 && gateResult.decision.startsWith('ALLOW_');
  const decision = mirrorRulesSatisfied ? 'NATIVE_RUST_GATE_REQUIRED' : 'BLOCK';
  if (mirrorRulesSatisfied) reasons.push('JavaScript work-plan diagnostics cannot authorize handoff; call the native Rust completion command.');
  return {
    allowed: false,
    decision,
    reasons: [...new Set(reasons)],
    coverage: gateResult.coverage,
    workPlanVersion: STRICT_WORK_PLAN_VERSION,
    nativeRustGateRequired: true,
    liveExecutionAuthorized: false,
  };
}
