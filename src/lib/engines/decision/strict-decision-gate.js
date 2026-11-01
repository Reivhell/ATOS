/** Pure fail-closed strict decision policy engine; never authorizes orders. */
import { STRICT_CATALOG_REGISTRY, STRICT_MASTER_LIST_VERSION } from '../../domain/decision/strict-catalog-registry.js';
import {
  ASSESSMENT_CONTRACT_VERSION,
  DECISION_GATE_CONTRACT_VERSION,
  ASSESSMENT_STATUSES,
  REVIEWED_IMPLEMENTATION_VERSION_BY_CATALOG_ID,
  SPECIALIST_AGENT_BY_CATEGORY,
  DECISION_POLICY,
} from '../../domain/decision/contracts.js';

/** @type {Map<string, StrictCatalogEntry>} */
const TRUSTED_BY_ID = new Map(STRICT_CATALOG_REGISTRY.map((item) => [item.id, item]));
export {
  ASSESSMENT_CONTRACT_VERSION,
  DECISION_GATE_CONTRACT_VERSION,
  ASSESSMENT_STATUSES,
  REVIEWED_IMPLEMENTATION_VERSION_BY_CATALOG_ID,
  SPECIALIST_AGENT_BY_CATEGORY,
  DECISION_POLICY,
} from '../../domain/decision/contracts.js';

/** The code-owned registry is authoritative; caller payloads cannot promote concepts.
 * @typedef {typeof STRICT_CATALOG_REGISTRY[number]} StrictCatalogEntry
 */
/** @typedef {{catalogId?:unknown,contractVersion?:unknown,masterListVersion?:unknown,category?:unknown,group?:unknown,name?:unknown,agentId?:unknown,status?:unknown,asOf?:unknown,evidenceRefs?:unknown,result?:unknown,lookAheadSafe?:unknown,implementationVersion?:unknown,dataSnapshotId?:unknown,reason?:unknown}} StrictAssessment */
/** @typedef {{asOf?:unknown,datasetSha256?:unknown,validated?:unknown,source?:unknown,symbol?:unknown,timeframe?:unknown}} StrictMarketData */
/** @typedef {{passed?:unknown,reason?:unknown}} StrictRisk */
/** @typedef {{catalog?:unknown,assessments?:unknown,marketData?:StrictMarketData|null,risk?:StrictRisk|null,intent?:unknown}|undefined} StrictDecisionInput */
/** @typedef {{required:number,received:number,evaluated:number,unsupported:number,insufficientData:number,notApplicable:number,missing:number}} CategoryCoverage */
/** @typedef {{allowed:boolean,decision:string,nativeRuntimeRequired:true,reasons:string[],coverage:{required:number,received:number,assessed:number,missing:number,duplicate:number,malformed:number,unsupported:number,insufficientData:number,notApplicable:number,evaluated:number,catalogOnly:number,byCategory:Record<string,CategoryCoverage>}}} StrictDecisionResult */

/** @param {unknown} value */
function validTimestamp(value) {
  return typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:\d{2})$/.test(value) && Number.isFinite(Date.parse(value));
}

/** @param {unknown} value */
function meaningfulReason(value) {
  return typeof value === 'string' && value.trim().length >= DECISION_POLICY.minimumReasonLength;
}

/** @param {unknown} value */
function nonEmptyString(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

/** @param {unknown} candidate */
function sameTrustedCatalog(candidate) {
  if (!Array.isArray(candidate) || candidate.length !== DECISION_POLICY.requiredCatalogEntries) return false;
  const seen = new Set();
  for (const item of candidate) {
    if (!item || typeof item !== 'object' || typeof item.id !== 'string' || seen.has(item.id)) return false;
    seen.add(item.id);
    const trusted = TRUSTED_BY_ID.get(item.id);
    if (!trusted || item.category !== trusted.category || item.group !== trusted.group ||
        item.name !== trusted.name || item.availability !== trusted.availability || item.kind !== trusted.kind) return false;
  }
  return seen.size === TRUSTED_BY_ID.size && TRUSTED_BY_ID.size === DECISION_POLICY.requiredCatalogEntries;
}

/** @param {StrictAssessment} assessment @param {StrictCatalogEntry} trusted */
function identityMatches(assessment, trusted) {
  const agentByCategory = /** @type {Record<string, string>} */ (SPECIALIST_AGENT_BY_CATEGORY);
  return assessment.contractVersion === ASSESSMENT_CONTRACT_VERSION &&
    assessment.masterListVersion === STRICT_MASTER_LIST_VERSION &&
    assessment.catalogId === trusted.id &&
    assessment.category === trusted.category &&
    assessment.group === trusted.group &&
    assessment.name === trusted.name &&
    assessment.agentId === agentByCategory[trusted.category];
}

/** @param {unknown} value */
function validSha256(value) {
  return typeof value === 'string' && /^[a-f0-9]{64}$/i.test(value);
}

/** Evidence references are scoped to one exact dataset; this JS gate remains non-authoritative. */
function canonicalEvidenceRefMatchesSnapshot(reference, snapshotSha256) {
  if (typeof reference !== 'string' || typeof snapshotSha256 !== 'string') return false;
  const match = /^snapshot:([a-fA-F0-9]{64}):(binance|mt5|rust-indicator|rust-structure|backtest):([A-Za-z0-9._~/-]+)$/.exec(reference);
  return Boolean(match)
    && match[1].toLowerCase() === snapshotSha256.toLowerCase()
    && !match[3].includes('..')
    && match[3].split('/').every((segment) => segment.length > 0 && !segment.startsWith('.'));
}

function hasCanonicalEvidenceRefs(refs, snapshotSha256) {
  return Array.isArray(refs) && refs.length > 0 && refs.length <= 32
    && refs.every((ref) => typeof ref === 'string' && ref.length <= 256 && ref.trim() === ref
      && canonicalEvidenceRefMatchesSnapshot(ref, snapshotSha256))
    && new Set(refs).size === refs.length;
}

/** @returns {CategoryCoverage} */
function newCategoryCoverage() {
  return { required: 0, received: 0, evaluated: 0, unsupported: 0, insufficientData: 0, notApplicable: 0, missing: 0 };
}

/**
 * @param {StrictDecisionInput} input
 * @returns {StrictDecisionResult}
 */
export function evaluateStrictDecisionGate(input) {
  const reasons = [];
  const suppliedCatalog = Array.isArray(input?.catalog) ? input.catalog : [];
  /** @type {StrictAssessment[]} */
  const assessments = Array.isArray(input?.assessments) ? input.assessments : [];
  const marketData = input?.marketData;
  const catalogMatchesTrustedRegistry = sameTrustedCatalog(suppliedCatalog);
  /** @type {Record<string, CategoryCoverage>} */
  const byCategory = Object.fromEntries(Object.keys(SPECIALIST_AGENT_BY_CATEGORY).map((category) => [category, newCategoryCoverage()]));

  for (const trusted of STRICT_CATALOG_REGISTRY) byCategory[trusted.category].required += 1;
  if (!catalogMatchesTrustedRegistry) {
    reasons.push('Supplied catalog differs from the code-owned Strict Master List v1.0.0 registry, including category/group/name/status/kind identity.');
  }
  if (assessments.length !== DECISION_POLICY.requiredCatalogEntries) {
    reasons.push('Assessment count must equal exactly 364.');
  }

  /** @type {Map<string, StrictAssessment>} */
  const byId = new Map();
  let duplicate = 0;
  let malformed = 0;
  for (const assessment of assessments) {
    if (!assessment || typeof assessment.catalogId !== 'string' || !TRUSTED_BY_ID.has(assessment.catalogId)) {
      malformed += 1;
      reasons.push('Assessment contains an unknown or malformed catalogId.');
      continue;
    }
    const trusted = TRUSTED_BY_ID.get(assessment.catalogId);
    if (!trusted) {
      malformed += 1;
      reasons.push('Assessment catalog identity is unavailable in the trusted registry.');
      continue;
    }
    byCategory[trusted.category].received += 1;
    if (byId.has(assessment.catalogId)) {
      duplicate += 1;
      reasons.push(`Duplicate assessment for ${assessment.catalogId}.`);
      continue;
    }
    byId.set(assessment.catalogId, assessment);
  }

  let unsupported = 0;
  let insufficientData = 0;
  let notApplicable = 0;
  let evaluated = 0;
  let catalogOnly = 0;

  for (const trusted of STRICT_CATALOG_REGISTRY) {
    const assessment = byId.get(trusted.id);
    if (!assessment) {
      byCategory[trusted.category].missing += 1;
      reasons.push(`Missing assessment: ${trusted.id}.`);
      continue;
    }
    if (!identityMatches(assessment, trusted)) {
      reasons.push(`Assessment identity/contract/agent routing mismatch: ${trusted.id}.`);
    }
    const status = typeof assessment.status === 'string' ? assessment.status : '';
    if (!ASSESSMENT_STATUSES.includes(/** @type {typeof ASSESSMENT_STATUSES[number]} */ (status))) {
      reasons.push(`Invalid assessment status for ${trusted.id}.`);
      continue;
    }
    if (!validTimestamp(assessment.asOf)) reasons.push(`Missing or invalid assessment timestamp for ${trusted.id}.`);
    if (assessment.asOf !== marketData?.asOf) reasons.push(`Assessment timestamp does not match the exact market-data snapshot cutoff: ${trusted.id}.`);

    if (trusted.availability === 'Catalog only') {
      catalogOnly += 1;
      if (status !== 'unsupported') {
        reasons.push(`Catalog-only concept must remain unsupported until reviewed promotion: ${trusted.id}.`);
      }
    }

    if (status === 'evaluated') {
      evaluated += 1;
      byCategory[trusted.category].evaluated += 1;
      if (!hasCanonicalEvidenceRefs(assessment.evidenceRefs, marketData?.datasetSha256)) {
        reasons.push(`Evaluated concept evidence references must be unique, canonical, and bound to the market-data snapshot: ${trusted.id}.`);
      }
      if (assessment.result === undefined || assessment.result === null) reasons.push(`Evaluated concept lacks an explicit result: ${trusted.id}.`);
      if (assessment.lookAheadSafe !== true) reasons.push(`Evaluated concept lacks explicit look-ahead safety confirmation: ${trusted.id}.`);
      const reviewedImplementations = /** @type {Record<string, string>} */ (REVIEWED_IMPLEMENTATION_VERSION_BY_CATALOG_ID);
      const reviewedImplementationVersion = reviewedImplementations[trusted.id];
      if (!reviewedImplementationVersion) reasons.push(`No code-owned reviewed implementation version is registered for evaluated concept: ${trusted.id}.`);
      else if (assessment.implementationVersion !== reviewedImplementationVersion) reasons.push(`Required implementation version does not match the code-owned reviewed version: ${trusted.id}.`);
      if (!validSha256(assessment.dataSnapshotId)) reasons.push(`Evaluated concept lacks a valid SHA-256 data snapshot identifier: ${trusted.id}.`);
      const assessmentSnapshotId = assessment.dataSnapshotId;
      const marketDatasetSha256 = marketData?.datasetSha256;
      if (typeof assessmentSnapshotId === 'string' && typeof marketDatasetSha256 === 'string'
        && validSha256(assessmentSnapshotId) && validSha256(marketDatasetSha256)
        && assessmentSnapshotId.toLowerCase() !== marketDatasetSha256.toLowerCase()) {
        reasons.push(`Evaluated concept data snapshot does not match the validated market-data dataset: ${trusted.id}.`);
      }
    } else if (status === 'not_applicable') {
      notApplicable += 1;
      byCategory[trusted.category].notApplicable += 1;
      if (!meaningfulReason(assessment.reason)) reasons.push(`Not-applicable status requires a meaningful rationale: ${trusted.id}.`);
      if (!hasCanonicalEvidenceRefs(assessment.evidenceRefs, marketData?.datasetSha256)) {
        reasons.push(`Not-applicable evidence references must be unique, canonical, and bound to the market-data snapshot: ${trusted.id}.`);
      }
    } else if (status === 'insufficient_data') {
      insufficientData += 1;
      byCategory[trusted.category].insufficientData += 1;
      if (!meaningfulReason(assessment.reason)) reasons.push(`Insufficient-data status requires a reason: ${trusted.id}.`);
      reasons.push(`Insufficient data blocks downstream decisions: ${trusted.id}.`);
    } else {
      unsupported += 1;
      byCategory[trusted.category].unsupported += 1;
      if (!meaningfulReason(assessment.reason)) reasons.push(`Unsupported status requires a reason: ${trusted.id}.`);
      reasons.push(`Unsupported concept blocks downstream decisions: ${trusted.id}.`);
    }
  }

  if (marketData?.validated !== true) reasons.push('Market data has not passed the validated-data contract.');
  if (!nonEmptyString(marketData?.source)) reasons.push('Market-data source provenance is missing.');
  if (!validTimestamp(marketData?.asOf)) reasons.push('Market-data timestamp is missing or invalid.');
  if (!validSha256(marketData?.datasetSha256)) reasons.push('Market-data SHA-256 snapshot fingerprint is missing or invalid.');
  if (!nonEmptyString(marketData?.symbol)) reasons.push('Market-data symbol/instrument scope is missing.');
  if (!nonEmptyString(marketData?.timeframe)) reasons.push('Market-data timeframe scope is missing.');
  if (input?.risk?.passed !== true) reasons.push(`Risk gateway did not approve the handoff${input?.risk?.reason ? `: ${String(input.risk.reason)}` : '.'}`);
  if (typeof input?.intent !== 'string' || !DECISION_POLICY.allowedIntents.includes(input.intent)) reasons.push('Intent must be explicitly research or paper_trade.');

  const mirrorRulesSatisfied = reasons.length === 0 && catalogMatchesTrustedRegistry && byId.size === DECISION_POLICY.requiredCatalogEntries;
  if (mirrorRulesSatisfied) reasons.push('JavaScript gate is non-authoritative; native Rust completion is required before downstream handoff.');
  return {
    allowed: false,
    decision: mirrorRulesSatisfied ? 'NATIVE_RUST_GATE_REQUIRED' : 'BLOCK',
    nativeRuntimeRequired: true,
    reasons,
    coverage: {
      required: DECISION_POLICY.requiredCatalogEntries,
      received: assessments.length,
      assessed: byId.size,
      missing: Math.max(0, DECISION_POLICY.requiredCatalogEntries - byId.size),
      duplicate,
      malformed,
      unsupported,
      insufficientData,
      notApplicable,
      evaluated,
      catalogOnly,
      byCategory,
    },
  };
}
