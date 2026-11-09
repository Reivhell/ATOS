/**
 * @typedef {'trend'|'momentum'|'structure'|'location'} EvidenceDomain
 * @typedef {'bullish'|'bearish'|'neutral'} EvidenceDirection
 * @typedef {{domain:EvidenceDomain,direction:EvidenceDirection,signalStrength:number,evidenceRefs:string[],asOf:string,datasetSha256:string}} ValidatedEvidence
 * @typedef {{id:string,type:string,attributes:Record<string,unknown>}} ProvenanceNode
 * @typedef {{from:string,to:string,relationship:string}} ProvenanceEdge
 * @typedef {{validated?:boolean,source?:string,asOf?:string|null,datasetSha256?:string|null,symbol?:string|null,timeframe?:string|null}} DecisionMarketData
 * @typedef {{passed?:boolean,source?:string,receiptId?:string}} RiskContext
 * @typedef {{marketData?:DecisionMarketData,signals?:unknown[],risk?:RiskContext,intent?:string,evidenceRegistry?:unknown}} DecisionInput
 * Deterministic, fail-closed research verdict engine.
 * It evaluates evidence across independent technical domains and NEVER emits order authority.
 */
import { RESEARCH_DECISION_POLICY } from '../../domain/decision/research-policy.js';
import { getCanonicalEvidenceRecord, getCanonicalEvidenceRegistryDigest, isVerifiedCanonicalEvidenceRegistry } from '../../domain/decision/canonical-evidence-registry.js';

/** @type {Set<EvidenceDirection>} */
const VALID_DIRECTIONS = new Set(['bullish', 'bearish', 'neutral']);
const SHA256 = /^[a-f0-9]{64}$/i;
const ISO_UTC = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z$/;
const MAX_SIGNAL_INPUTS = 16;
const MAX_EVIDENCE_REFS_PER_SIGNAL = 32;
const MAX_EVIDENCE_REF_LENGTH = 256;

/** @param {unknown} value @returns {value is string} */
function validTimestamp(value) {
  return typeof value === 'string' && ISO_UTC.test(value) && Number.isFinite(Date.parse(value));
}

/** @param {unknown} value @returns {value is string} */
function nonEmptyString(value) {
  return typeof value === 'string' && value.trim().length > 0;
}

/** @param {unknown} value @returns {value is string} */
function validSymbol(value) {
  return typeof value === 'string' && /^[A-Z0-9_-]{1,30}$/.test(value);
}

/** @param {{marketData:DecisionMarketData|undefined,risk?:RiskContext,intent?:string}} input */
function evaluatePrerequisites({ marketData, risk, intent }) {
  const reasons = [];
  if (typeof intent !== 'string' || !RESEARCH_DECISION_POLICY.allowedIntents.includes(intent)) {
    reasons.push('Intent must be explicitly research or paper_trade; live intent is not supported.');
  }
  if (marketData?.validated !== true) reasons.push('Market data has not passed the validated-snapshot boundary.');
  if (!nonEmptyString(marketData?.source)) reasons.push('Market-data source provenance is missing.');
  if (!validTimestamp(marketData?.asOf)) reasons.push('Snapshot cutoff must be an explicit UTC ISO-8601 timestamp.');
  if (typeof marketData?.datasetSha256 !== 'string' || !SHA256.test(marketData.datasetSha256)) {
    reasons.push('A canonical SHA-256 fingerprint is required to bind signal evidence to one dataset.');
  }
  if (!validSymbol(marketData?.symbol)) reasons.push('Market-data instrument scope is malformed or missing.');
  if (!nonEmptyString(marketData?.timeframe)) reasons.push('Market-data timeframe scope is missing.');
  if (intent === 'paper_trade' && risk?.passed !== true) {
    reasons.push('Paper-trade intent requires a passing native risk-gateway result.');
  }
  if (intent === 'paper_trade' && risk?.source !== 'native_rust') {
    reasons.push('Paper-trade risk result must identify the native Rust risk gateway.');
  }
  if (intent === 'paper_trade' && !nonEmptyString(risk?.receiptId)) {
    reasons.push('Paper-trade risk approval requires a traceable native receipt identifier.');
  }
  return reasons;
}

/** @param {unknown} signals @param {DecisionMarketData|undefined} marketData */
/** @param {unknown} signals @param {DecisionMarketData|undefined} marketData @param {unknown} evidenceRegistry */
function validateSignals(signals, marketData, evidenceRegistry) {
  const reasons = [];
  /** @type {ValidatedEvidence[]} */
  const valid = [];
  /** @type {Set<EvidenceDomain>} */
  const seen = new Set();
  /** @type {Set<string>} */
  const seenEvidenceRefs = new Set();
  if (!Array.isArray(signals)) return { signals: valid, reasons: ['Signals must be supplied as an array.'] };
  if (!isVerifiedCanonicalEvidenceRegistry(evidenceRegistry)) {
    reasons.push('A canonical evidence registry created and verified by the registry boundary is required.');
  }
  if (signals.length > MAX_SIGNAL_INPUTS) {
    return { signals: valid, reasons: [`Signal input exceeds the hard limit of ${MAX_SIGNAL_INPUTS}; decision evaluation is blocked.`] };
  }

  for (const [index, signal] of signals.entries()) {
    const label = `Signal ${index + 1}`;
    if (!signal || typeof signal !== 'object' || Array.isArray(signal)) {
      reasons.push(`${label} is malformed.`);
      continue;
    }
    const raw = /** @type {Record<string, unknown>} */ (signal);
    if (typeof raw.domain !== 'string' || !Object.hasOwn(RESEARCH_DECISION_POLICY.domainWeights, raw.domain)) {
      reasons.push(`${label} uses an unregistered evidence domain.`);
      continue;
    }
    const domain = /** @type {EvidenceDomain} */ (raw.domain);
    if (seen.has(domain)) {
      reasons.push(`Duplicate signal domain ${domain}; one domain may contribute only once.`);
      continue;
    }
    seen.add(domain);
    if (typeof raw.direction !== 'string' || !VALID_DIRECTIONS.has(/** @type {EvidenceDirection} */ (raw.direction))) {
      reasons.push(`${label} direction must be bullish, bearish, or neutral.`);
      continue;
    }
    const direction = /** @type {EvidenceDirection} */ (raw.direction);
    if (typeof raw.signalStrength !== 'number' || !Number.isFinite(raw.signalStrength) || raw.signalStrength < 0 || raw.signalStrength > 1) {
      reasons.push(`${label} strength must be finite and within 0..1.`);
      continue;
    }
    const signalStrength = raw.signalStrength;
    if (!validTimestamp(raw.asOf) || raw.asOf !== marketData?.asOf) {
      reasons.push(`${label} cutoff does not match the canonical market-data snapshot.`);
      continue;
    }
    if (typeof raw.datasetSha256 !== 'string' || !SHA256.test(raw.datasetSha256)
      || raw.datasetSha256.toLowerCase() !== String(marketData?.datasetSha256 ?? '').toLowerCase()) {
      reasons.push(`${label} fingerprint does not match the canonical market-data dataset.`);
      continue;
    }
    if (!Array.isArray(raw.evidenceRefs) || raw.evidenceRefs.length === 0
      || raw.evidenceRefs.length > MAX_EVIDENCE_REFS_PER_SIGNAL
      || raw.evidenceRefs.some((reference) => !nonEmptyString(reference) || reference.length > MAX_EVIDENCE_REF_LENGTH)
      || new Set(raw.evidenceRefs).size !== raw.evidenceRefs.length) {
      reasons.push(`${label} must contain 1..${MAX_EVIDENCE_REFS_PER_SIGNAL} unique, non-empty evidence references of at most ${MAX_EVIDENCE_REF_LENGTH} characters.`);
      continue;
    }
    const evidenceRefs = /** @type {string[]} */ (raw.evidenceRefs);
    const unknownRefs = evidenceRefs.filter((reference) => {
      const record = getCanonicalEvidenceRecord(evidenceRegistry, reference);
      return !record || !/^ev1:[a-f0-9]{64}$/.test(reference)
        || record.canonicalId !== reference
        || record.contentSha256 !== reference.slice(4)
        || record.datasetSha256 !== String(marketData?.datasetSha256 ?? '').toLowerCase()
        || record.observedAt !== marketData?.asOf
        || record.symbol !== marketData?.symbol
        || record.timeframe !== marketData?.timeframe;
    });
    if (unknownRefs.length > 0) {
      reasons.push(`${label} contains evidence references absent from the verified canonical registry or outside the snapshot scope.`);
      continue;
    }
    const reusedRefs = evidenceRefs.filter((reference) => seenEvidenceRefs.has(reference));
    if (reusedRefs.length > 0) {
      reasons.push(`${label} reuses evidence already counted by another domain; cross-domain evidence reuse is blocked.`);
      continue;
    }
    for (const reference of evidenceRefs) seenEvidenceRefs.add(reference);
    valid.push({
      domain,
      direction,
      signalStrength,
      evidenceRefs: [...evidenceRefs],
      asOf: raw.asOf,
      datasetSha256: raw.datasetSha256.toLowerCase(),
    });
  }
  return { signals: valid, reasons };
}

/**
 * @typedef {object} ResearchDecisionCoverage
 * @property {number} supplied
 * @property {number} accepted
 * @property {number} directional
 *
 * @typedef {object} ResearchDecisionScores
 * @property {number} bullish
 * @property {number} bearish
 * @property {number} dominantShare
 * @property {number} weightedDominantShare
 * @property {number} meanSignalStrength
 *
 * @typedef {object} ResearchDecisionProvenance
 * @property {string} schemaVersion
 * @property {boolean} deterministicForIdenticalInputs
 * @property {string|null} snapshotAsOf
 * @property {string} policyVersion
 * @property {{nodes: readonly object[], edges: readonly object[]}} graph
 * @property {readonly object[]} contributions
 * @property {readonly object[]} gates
 * @property {object} integrity
 *
 * @typedef {object} ResearchDecision
 * @property {string} contractVersion
 * @property {string} decision
 * @property {string} technicalBias
 * @property {string|null} intent
 * @property {ResearchDecisionCoverage} coverage
 * @property {ResearchDecisionScores} scores
 * @property {readonly object[]} evidence
 * @property {ResearchDecisionProvenance} provenance
 * @property {readonly string[]} reasons
 * @property {null} orderIntent
 * @property {'none'} executionAuthority
 * @property {false} liveExecutionAuthorized
 * @property {string} policyVersion
 */

/**
 * @param {DecisionInput} [input={}]
 * @returns {ResearchDecision} Deterministic research verdict; never a tradable order or execution approval.
 */
export function evaluateResearchDecision(input = {}) {
  const marketData = input?.marketData;
  const intent = input?.intent;
  const prerequisiteReasons = evaluatePrerequisites({ marketData, risk: input?.risk, intent });
  const signalResult = validateSignals(input?.signals, marketData, input?.evidenceRegistry);
  const reasons = [...prerequisiteReasons, ...signalResult.reasons];
  const signals = signalResult.signals;
  const eligibleDomains = signals.length;
  const directional = signals.filter((signal) => signal.direction === 'bullish' || signal.direction === 'bearish');
  const bullish = directional.filter((signal) => signal.direction === 'bullish');
  const bearish = directional.filter((signal) => signal.direction === 'bearish');
  const bullishScore = bullish.reduce((sum, signal) => sum + RESEARCH_DECISION_POLICY.domainWeights[signal.domain] * signal.signalStrength, 0);
  const bearishScore = bearish.reduce((sum, signal) => sum + RESEARCH_DECISION_POLICY.domainWeights[signal.domain] * signal.signalStrength, 0);
  const totalDirectionalWeight = bullish.reduce((sum, signal) => sum + RESEARCH_DECISION_POLICY.domainWeights[signal.domain], 0)
    + bearish.reduce((sum, signal) => sum + RESEARCH_DECISION_POLICY.domainWeights[signal.domain], 0);
  const weightedDominantShare = totalDirectionalWeight > 0 ? Math.max(
    bullish.reduce((sum, signal) => sum + RESEARCH_DECISION_POLICY.domainWeights[signal.domain], 0),
    bearish.reduce((sum, signal) => sum + RESEARCH_DECISION_POLICY.domainWeights[signal.domain], 0),
  ) / totalDirectionalWeight : 0;
  // Agreement threshold is based on independent domain count, not duplicated weight.
  // Weight influences score separation only; a 2-vs-2 disagreement cannot become a candidate.
  const dominantShare = directional.length > 0 ? Math.max(bullish.length, bearish.length) / directional.length : 0;
  const meanStrength = directional.length > 0
    ? directional.reduce((sum, signal) => sum + signal.signalStrength, 0) / directional.length
    : 0;

  let decision = 'ABSTAIN_INSUFFICIENT_EVIDENCE';
  let technicalBias = 'unknown';
  if (reasons.length > 0) {
    decision = 'BLOCK';
    technicalBias = 'untrusted';
  } else if (eligibleDomains < RESEARCH_DECISION_POLICY.minimumDistinctDomains) {
    reasons.push(`At least ${RESEARCH_DECISION_POLICY.minimumDistinctDomains} independent signal domains are required.`);
  } else if (directional.length < RESEARCH_DECISION_POLICY.minimumDirectionalDomains) {
    reasons.push(`At least ${RESEARCH_DECISION_POLICY.minimumDirectionalDomains} directional domains are required; neutral or unavailable outputs do not count as votes.`);
  } else if (meanStrength < RESEARCH_DECISION_POLICY.minimumMeanSignalStrength) {
    decision = 'ABSTAIN_WEAK_EVIDENCE';
    reasons.push('Mean heuristic signal strength is below the reviewed decision threshold.');
  } else if (dominantShare < RESEARCH_DECISION_POLICY.minimumDirectionalAgreement) {
    decision = 'ABSTAIN_CONFLICTING_SIGNALS';
    reasons.push('Independent directional domains do not reach the required agreement threshold.');
  } else if (bullishScore - bearishScore >= 2) {
    decision = 'RESEARCH_LONG_CANDIDATE';
    technicalBias = 'bullish';
  } else if (bearishScore - bullishScore >= 2) {
    decision = 'RESEARCH_SHORT_CANDIDATE';
    technicalBias = 'bearish';
  } else {
    decision = 'ABSTAIN_CONFLICTING_SIGNALS';
    reasons.push('Weighted bullish/bearish evidence is not sufficiently separated.');
  }

  if (!technicalBias || technicalBias === 'unknown') {
    if (bullishScore > bearishScore) technicalBias = 'bullish-leaning';
    else if (bearishScore > bullishScore) technicalBias = 'bearish-leaning';
    else if (signals.some((signal) => signal.direction === 'neutral')) technicalBias = 'neutral';
  }

  const uniqueReasons = [...new Set(reasons)];
  const snapshotNodeId = `snapshot:${String(marketData?.datasetSha256 ?? 'missing').toLowerCase()}`;
  /** @type {ProvenanceNode[]} */
  const graphNodes = [{
    id: snapshotNodeId,
    type: 'market_snapshot',
    attributes: {
      source: nonEmptyString(marketData?.source) ? marketData.source : null,
      symbol: validSymbol(marketData?.symbol) ? marketData.symbol : null,
      timeframe: nonEmptyString(marketData?.timeframe) ? marketData.timeframe : null,
      asOf: validTimestamp(marketData?.asOf) ? marketData.asOf : null,
      datasetSha256: typeof marketData?.datasetSha256 === 'string' && SHA256.test(marketData.datasetSha256)
        ? marketData.datasetSha256.toLowerCase() : null,
      validated: marketData?.validated === true,
    },
  }];
  /** @type {ProvenanceEdge[]} */
  const graphEdges = [];
  const contributions = signals.map((signal) => {
    const weight = RESEARCH_DECISION_POLICY.domainWeights[signal.domain];
    const signedContribution = signal.direction === 'bullish' ? weight * signal.signalStrength
      : signal.direction === 'bearish' ? -weight * signal.signalStrength : 0;
    const signalNodeId = `signal:${signal.domain}`;
    graphNodes.push({
      id: signalNodeId,
      type: 'validated_signal',
      attributes: {
        domain: signal.domain,
        direction: signal.direction,
        signalStrength: signal.signalStrength,
        domainWeight: weight,
        signedContribution: Number(signedContribution.toFixed(6)),
        evidenceRefCount: signal.evidenceRefs.length,
      },
    });
    graphEdges.push({ from: snapshotNodeId, to: signalNodeId, relationship: 'evaluated_against_snapshot' });
    for (const reference of signal.evidenceRefs) {
      const evidenceNodeId = `evidence:${encodeURIComponent(reference)}`;
      if (!graphNodes.some((node) => node.id === evidenceNodeId)) {
        graphNodes.push({ id: evidenceNodeId, type: 'evidence_reference', attributes: { reference } });
      }
      graphEdges.push({ from: evidenceNodeId, to: signalNodeId, relationship: 'supports_signal' });
    }
    return Object.freeze({
      domain: signal.domain,
      direction: signal.direction,
      signalStrength: signal.signalStrength,
      domainWeight: weight,
      signedContribution: Number(signedContribution.toFixed(6)),
      evidenceRefs: Object.freeze([...signal.evidenceRefs]),
    });
  });
  /** @param {string} id @param {boolean} passed @param {unknown} detail */
  const gate = (id, passed, detail) => Object.freeze({ id, status: passed ? 'passed' : 'blocked', detail });
  const gateTrace = Object.freeze([
    gate('market_snapshot_prerequisites', prerequisiteReasons.length === 0, prerequisiteReasons.length ? prerequisiteReasons : ['Required market-data, intent, and risk prerequisites are satisfied.']),
    gate('signal_integrity', signalResult.reasons.length === 0, signalResult.reasons.length ? signalResult.reasons : ['Signal schema, snapshot binding, bounded inputs, and canonical-registry scope checks passed.']),
    gate('distinct_domain_coverage', eligibleDomains >= RESEARCH_DECISION_POLICY.minimumDistinctDomains, [`accepted=${eligibleDomains}`, `required=${RESEARCH_DECISION_POLICY.minimumDistinctDomains}`]),
    gate('directional_coverage', directional.length >= RESEARCH_DECISION_POLICY.minimumDirectionalDomains, [`directional=${directional.length}`, `required=${RESEARCH_DECISION_POLICY.minimumDirectionalDomains}`]),
    gate('mean_signal_strength', meanStrength >= RESEARCH_DECISION_POLICY.minimumMeanSignalStrength, [`observed=${Number(meanStrength.toFixed(6))}`, `required=${RESEARCH_DECISION_POLICY.minimumMeanSignalStrength}`]),
    gate('directional_agreement', dominantShare >= RESEARCH_DECISION_POLICY.minimumDirectionalAgreement, [`observed=${Number(dominantShare.toFixed(6))}`, `required=${RESEARCH_DECISION_POLICY.minimumDirectionalAgreement}`]),
    gate('score_separation', Math.abs(bullishScore - bearishScore) >= 2, [`bullish=${Number(bullishScore.toFixed(6))}`, `bearish=${Number(bearishScore.toFixed(6))}`, 'required absolute separation >= 2']),
  ]);
  const provenance = Object.freeze({
    schemaVersion: 'decision-provenance.v2',
    deterministicForIdenticalInputs: true,
    snapshotAsOf: validTimestamp(marketData?.asOf) ? marketData.asOf : null,
    policyVersion: 'research-decision-policy.v3',
    graph: Object.freeze({
      nodes: Object.freeze(graphNodes.map((node) => Object.freeze({ ...node, attributes: Object.freeze({ ...node.attributes }) }))),
      edges: Object.freeze(graphEdges.map((edge) => Object.freeze({ ...edge }))),
    }),
    contributions: Object.freeze(contributions),
    gates: gateTrace,
    integrity: Object.freeze({
      crossDomainEvidenceReuseDetected: uniqueReasons.some((reason) => reason.includes('cross-domain evidence reuse')),
      inputBoundsExceeded: uniqueReasons.some((reason) => reason.includes('hard limit')),
      acceptedEvidenceReferences: signals.reduce((count, signal) => count + signal.evidenceRefs.length, 0),
      canonicalRegistryVerified: isVerifiedCanonicalEvidenceRegistry(input?.evidenceRegistry),
      canonicalRegistrySha256: getCanonicalEvidenceRegistryDigest(input?.evidenceRegistry),
      graphIsDiagnosticNotCryptographicAttestation: true,
    }),
  });

  return Object.freeze({
    contractVersion: 'research-decision.v4',
    decision,
    technicalBias,
    intent: typeof intent === 'string' ? intent : null,
    coverage: Object.freeze({ supplied: Array.isArray(input?.signals) ? input.signals.length : 0, accepted: eligibleDomains, directional: directional.length }),
    scores: Object.freeze({ bullish: Number(bullishScore.toFixed(6)), bearish: Number(bearishScore.toFixed(6)), dominantShare: Number(dominantShare.toFixed(6)), weightedDominantShare: Number(weightedDominantShare.toFixed(6)), meanSignalStrength: Number(meanStrength.toFixed(6)) }),
    evidence: Object.freeze(signals),
    provenance,
    reasons: Object.freeze(uniqueReasons),
    orderIntent: null,
    executionAuthority: 'none',
    liveExecutionAuthorized: false,
    policyVersion: 'research-decision-policy.v3',
  });
}
