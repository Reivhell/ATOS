/**
 * Adapts a native, data-scoped AI analyst report to the existing deterministic research gate.
 * Model-authored prose is deliberately excluded from scoring: only the Rust-derived evidence
 * bundle can enter the canonical registry and the policy engine. This function has no I/O or
 * execution authority; paper/live execution remains outside this application service.
 */
import { canonicalizeEvidencePayload, createCanonicalEvidenceRegistry, MAX_EVIDENCE_PAYLOAD_BYTES, resolveCanonicalEvidenceId } from '../domain/decision/canonical-evidence-registry.js';
import { evaluateResearchDecision } from '../engines/decision/research-decision-engine.js';

const DOMAINS = Object.freeze(['trend', 'momentum', 'structure', 'location']);
const SHA256 = /^[a-f0-9]{64}$/;
const ISO_UTC = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z$/;
const SCORE_SCALE = 1_000_000;
const PROVIDERS = Object.freeze(['openai', 'anthropic', 'openrouter', 'ollama']);
const MAX_SUMMARY_CHARS = 1800;
const MAX_ASSESSMENT_CHARS = 900;
const MAX_CAVEAT_CHARS = 240;
const MAX_LIST_ITEM_CHARS = 280;
const MAX_DISAGREEMENTS = 4;
const MAX_FOLLOW_UP_CHECKS = 6;
const MAX_MARKET_RECEIPT_AGE_MS = 120_000;

/** @typedef {Record<string, unknown>} UnknownRecord */
/** @typedef {{ref:string,domain:string,direction:string,signalStrength:number,source:string,observedAt:string,datasetSha256:string,symbol:string,timeframe:string,payload:UnknownRecord,payloadCanonicalJson:string,payloadSha256:string}} AgentEvidence */
/** @typedef {{symbol:string,timeframe:string,asOf:string,latestClosedBarAtMs:number,retrievedAtMs:number,datasetSha256:string,source:string,providerStatus:string,isLive:boolean,isFresh:boolean,candleCount:number,freshnessLimitMs:number}} AgentMarketContext */

/** @param {unknown} condition @param {string} message @returns {asserts condition} */
function assert(condition, message) {
  if (!condition) throw new TypeError(message);
}

/** @param {unknown} value @param {string} message @returns {UnknownRecord} */
function asRecord(value, message) {
  assert(value !== null && typeof value === 'object' && !Array.isArray(value), message);
  return /** @type {UnknownRecord} */ (value);
}

/** @param {unknown} value @returns {value is string} */
function validIsoUtc(value) {
  if (typeof value !== 'string' || !ISO_UTC.test(value)) return false;
  const parsed = Date.parse(value);
  if (!Number.isFinite(parsed)) return false;
  const date = new Date(parsed);
  const match = ISO_UTC.exec(value);
  return Boolean(match)
    && date.getUTCFullYear() === Number(value.slice(0, 4))
    && date.getUTCMonth() + 1 === Number(value.slice(5, 7))
    && date.getUTCDate() === Number(value.slice(8, 10))
    && date.getUTCHours() === Number(value.slice(11, 13))
    && date.getUTCMinutes() === Number(value.slice(14, 16))
    && date.getUTCSeconds() === Number(value.slice(17, 19));
}

/** @param {string} value @returns {Promise<string>} */
async function sha256Text(value) {
  const cryptoApi = globalThis.crypto;
  assert(cryptoApi?.subtle && typeof cryptoApi.subtle.digest === 'function', 'Web Crypto SHA-256 is unavailable; agent evidence validation fails closed.');
  const digest = await cryptoApi.subtle.digest('SHA-256', new TextEncoder().encode(value));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** @param {unknown} value @returns {Promise<string>} */
async function sha256CanonicalEnvelope(value) {
  return sha256Text(JSON.stringify(value));
}

/** Validate untrusted model commentary independently of the Rust schema boundary. @param {UnknownRecord} root */
function validateCommentary(root) {
  assert(typeof root.providerId === 'string' && PROVIDERS.includes(root.providerId), 'Agent provider identity is unsupported.');
  assert(typeof root.model === 'string' && root.model.trim().length > 0 && root.model.length <= 128, 'Agent model identity is malformed.');
  assert(validIsoUtc(root.generatedAt), 'Agent report generation timestamp is malformed.');
  assert(typeof root.summary === 'string' && root.summary.trim().length > 0 && root.summary.length <= MAX_SUMMARY_CHARS, 'Agent summary exceeds the strict content contract.');

  const observations = root.observations;
  assert(Array.isArray(observations) && observations.length === DOMAINS.length, 'Agent report must contain exactly four observations.');
  const seen = new Set();
  for (const [index, raw] of /** @type {unknown[]} */ (observations).entries()) {
    const item = asRecord(raw, `Agent observation ${index + 1} is malformed.`);
    assert(typeof item.domain === 'string' && DOMAINS.includes(item.domain) && !seen.has(item.domain), 'Agent observations contain a duplicate or unknown domain.');
    seen.add(item.domain);
    assert(typeof item.assessment === 'string' && item.assessment.trim().length > 0 && item.assessment.length <= MAX_ASSESSMENT_CHARS, `Agent assessment for ${item.domain} violates the length contract.`);
    assert(Array.isArray(item.evidenceRefs) && item.evidenceRefs.length === 1 && item.evidenceRefs[0] === `native:${item.domain}`, `Agent observation for ${item.domain} cites evidence outside its assigned domain.`);
    assert(Array.isArray(item.caveats) && item.caveats.length <= 4 && item.caveats.every((entry) => typeof entry === 'string' && entry.trim().length > 0 && entry.length <= MAX_CAVEAT_CHARS), `Agent caveats for ${item.domain} violate the content contract.`);
  }
  assert(seen.size === DOMAINS.length, 'Agent report omits one or more required domains.');
  assert(Array.isArray(root.disagreements) && root.disagreements.length <= MAX_DISAGREEMENTS && /** @type {unknown[]} */ (root.disagreements).every((entry) => typeof entry === 'string' && entry.trim().length > 0 && entry.length <= MAX_LIST_ITEM_CHARS), 'Agent disagreement list violates the content contract.');
  assert(Array.isArray(root.followUpChecks) && root.followUpChecks.length <= MAX_FOLLOW_UP_CHECKS && /** @type {unknown[]} */ (root.followUpChecks).every((entry) => typeof entry === 'string' && entry.trim().length > 0 && entry.length <= MAX_LIST_ITEM_CHARS), 'Agent follow-up list violates the content contract.');
  assert(root.decisionNote === 'LLM text is commentary only. The separate deterministic decision evaluation uses Rust-derived evidence; no order intent or execution permission is emitted.', 'Agent decision note does not match the fixed safety contract.');
}

/** Recompute the Rust-produced digest over the exact, bounded commentary document. @param {UnknownRecord} root */
async function verifyCommentaryDigest(root) {
  const document = {
    summary: root.summary,
    observations: /** @type {UnknownRecord[]} */ (root.observations).map((item) => ({
      domain: item.domain,
      assessment: item.assessment,
      evidenceRefs: item.evidenceRefs,
      caveats: item.caveats,
    })),
    disagreements: root.disagreements,
    followUpChecks: root.followUpChecks,
  };
  const actual = await sha256CanonicalEnvelope(document);
  assert(actual === root.outputSha256, 'Agent commentary digest mismatch; report content changed after the native validation boundary.');
}

/**
 * @param {unknown} report
 * @returns {Promise<{decision:object,registrySha256:string,registryRecordCount:number,decisionInputSource:string,executionAuthority:'none',liveExecutionAuthorized:false,orderIntent:null,aiNarrativeExcludedFromScoring:true}>}
 */
export async function evaluateAgentResearchReport(report) {
  const root = asRecord(report, 'Agent report must be a non-array object.');
  assert(root.contractVersion === 'agent-research-runtime.v2', 'Unsupported agent report contract.');
  assert(typeof root.runId === 'string' && root.runId.length >= 20 && root.runId.length <= 64, 'Agent report run identity is malformed.');
  assert(root.executionAuthority === 'none', 'Agent report cannot grant execution authority.');
  assert(root.liveExecutionAuthorized === false, 'Live execution authorization must remain false.');
  assert(!Object.hasOwn(root, 'orderIntent') || root.orderIntent === null, 'Agent report cannot include an order intent.');
  assert(typeof root.outputSha256 === 'string' && SHA256.test(root.outputSha256), 'Agent output digest is malformed.');
  validateCommentary(root);
  await verifyCommentaryDigest(root);

  const rawContext = asRecord(root.marketContext, 'Agent report is missing its native market context.');
  const symbol = rawContext.symbol;
  const timeframe = rawContext.timeframe;
  const asOf = rawContext.asOf;
  const latestClosedBarAtMs = rawContext.latestClosedBarAtMs;
  const retrievedAtMs = rawContext.retrievedAtMs;
  const datasetSha256 = rawContext.datasetSha256;
  const source = rawContext.source;
  const providerStatus = rawContext.providerStatus;
  const isLive = rawContext.isLive;
  const isFresh = rawContext.isFresh;
  const candleCount = rawContext.candleCount;
  const freshnessLimitMs = rawContext.freshnessLimitMs;

  assert(typeof symbol === 'string' && /^[A-Z0-9_-]{1,30}$/.test(symbol), 'Market instrument scope is malformed.');
  assert(typeof timeframe === 'string' && /^[A-Za-z0-9_-]{1,16}$/.test(timeframe), 'Market timeframe scope is malformed.');
  assert(validIsoUtc(asOf), 'Market snapshot cutoff is not a valid UTC timestamp.');
  assert(typeof latestClosedBarAtMs === 'number' && Number.isSafeInteger(latestClosedBarAtMs) && latestClosedBarAtMs > 0 && Date.parse(asOf) === latestClosedBarAtMs, 'Market snapshot cutoff does not match the native latest-closed-bar timestamp.');
  assert(typeof retrievedAtMs === 'number' && Number.isSafeInteger(retrievedAtMs) && retrievedAtMs >= latestClosedBarAtMs, 'Native market snapshot receipt timestamp is malformed.');
  assert(typeof datasetSha256 === 'string' && SHA256.test(datasetSha256), 'Native dataset fingerprint is malformed.');
  assert(typeof source === 'string' && source.trim().length > 0 && source.length <= 256, 'Native market-data source is missing.');
  assert(typeof providerStatus === 'string' && providerStatus.length <= 256, 'Market-data provider status is malformed.');
  assert(typeof isLive === 'boolean' && typeof isFresh === 'boolean', 'Market-data live/freshness flags are malformed.');
  assert(Number.isSafeInteger(candleCount) && Number(candleCount) >= 30, 'Native market context does not have sufficient validated candles.');
  assert(typeof freshnessLimitMs === 'number' && Number.isSafeInteger(freshnessLimitMs) && freshnessLimitMs > 0, 'Native snapshot freshness bound is malformed.');

  const rawBundle = root.evidenceBundle;
  assert(Array.isArray(rawBundle) && rawBundle.length === 4, 'Native evidence bundle must contain exactly four analytical domains.');
  /** @type {Set<string>} */
  const seenDomains = new Set();
  /** @type {AgentEvidence[]} */
  const evidenceBundle = await Promise.all(rawBundle.map(async (rawItem, index) => {
    const item = asRecord(rawItem, `Native evidence item ${index + 1} is malformed.`);
    const domain = item.domain;
    const ref = item.ref;
    const direction = item.direction;
    const signalStrength = item.signalStrength;
    const itemSource = item.source;
    const observedAt = item.observedAt;
    const itemDatasetSha = item.datasetSha256;
    const itemSymbol = item.symbol;
    const itemTimeframe = item.timeframe;
    const payload = asRecord(item.payload, 'Evidence payload must be a structured object.');
    const payloadCanonicalJson = item.payloadCanonicalJson;
    const payloadSha256 = item.payloadSha256;
    assert(typeof payloadCanonicalJson === 'string' && payloadCanonicalJson.length > 0
      && new TextEncoder().encode(payloadCanonicalJson).byteLength <= MAX_EVIDENCE_PAYLOAD_BYTES,
    'Native evidence canonical payload is missing or exceeds the per-record byte limit.');
    assert(typeof payloadSha256 === 'string' && SHA256.test(payloadSha256), 'Native evidence payload SHA-256 is malformed.');
    const payloadDigest = await sha256Text(payloadCanonicalJson);
    assert(payloadDigest === payloadSha256, 'Native evidence payload digest mismatch; payload changed after the Rust boundary.');
    let canonicalPayloadValue;
    try { canonicalPayloadValue = JSON.parse(payloadCanonicalJson); }
    catch { throw new TypeError('Native evidence canonical payload is invalid JSON.'); }
    assert(canonicalizeEvidencePayload(canonicalPayloadValue) === canonicalizeEvidencePayload(payload),
      'Native evidence payload digest mismatch; payload changed after the Rust boundary.');
    assert(typeof domain === 'string' && DOMAINS.includes(domain) && !seenDomains.has(domain), 'Native evidence includes a duplicate or unregistered domain.');
    seenDomains.add(domain);
    assert(typeof ref === 'string' && ref === `native:${domain}`, `Native evidence reference does not match its domain (${domain}).`);
    assert(itemSource === 'rust-native-derived-evidence.v1', 'Evidence source is not the registered Rust analytics producer.');
    assert(observedAt === asOf, 'Evidence observation time does not match the market snapshot cutoff.');
    assert(itemDatasetSha === datasetSha256, 'Evidence fingerprint does not match the market snapshot.');
    assert(itemSymbol === symbol && itemTimeframe === timeframe, 'Evidence instrument/timeframe scope does not match the market snapshot.');
    assert(typeof direction === 'string' && ['bullish', 'bearish', 'neutral'].includes(direction), 'Evidence direction is outside the deterministic policy domain.');
    assert(typeof signalStrength === 'number' && Number.isFinite(signalStrength) && signalStrength >= 0 && signalStrength <= 1, 'Evidence strength must be a finite 0..1 value.');
    assert(payload.domain === domain && payload.direction === direction && payload.signalStrength === signalStrength, 'Evidence payload fields do not match the native scoring envelope.');
    return {
      ref, domain, direction, signalStrength, source: itemSource, observedAt,
      datasetSha256: itemDatasetSha, symbol: itemSymbol, timeframe: itemTimeframe, payload,
      payloadCanonicalJson, payloadSha256,
    };
  }));
  assert(seenDomains.size === 4, 'One or more native analytical domains are missing.');

  assert(typeof root.evidenceEnvelopeSha256 === 'string' && SHA256.test(root.evidenceEnvelopeSha256), 'Native evidence scoring-envelope digest is malformed.');
  const orderedEnvelope = DOMAINS.map((domain) => {
    const item = evidenceBundle.find((entry) => entry.domain === domain);
    assert(item, `Native evidence domain ${domain} is missing from the scoring envelope.`);
    return [item.ref, item.domain, item.direction, Math.round(item.signalStrength * SCORE_SCALE), item.source,
      item.observedAt, item.datasetSha256, item.symbol, item.timeframe, item.payloadSha256];
  });
  const computedEnvelopeSha256 = await sha256CanonicalEnvelope(orderedEnvelope);
  assert(computedEnvelopeSha256 === root.evidenceEnvelopeSha256, 'Native scoring envelope digest mismatch; evidence changed after the Rust boundary.');

  const descriptors = evidenceBundle.map((item) => ({
    ref: item.ref, source: item.source, observedAt: item.observedAt, datasetSha256: item.datasetSha256,
    symbol: item.symbol, timeframe: item.timeframe, payload: item.payload, parentRefs: [],
  }));
  const evidenceRegistry = await createCanonicalEvidenceRegistry(descriptors);
  const signals = evidenceBundle.map((item) => ({
    domain: item.domain,
    direction: item.direction,
    signalStrength: item.signalStrength,
    evidenceRefs: [resolveCanonicalEvidenceId(evidenceRegistry, item.ref)],
    asOf,
    datasetSha256,
  }));
  const evaluatedAtMs = Date.now();
  const candleAgeAtEvaluationMs = evaluatedAtMs - latestClosedBarAtMs;
  const receiptAgeAtEvaluationMs = evaluatedAtMs - retrievedAtMs;
  const timestampsRemainFresh = Number.isSafeInteger(evaluatedAtMs)
    && candleAgeAtEvaluationMs >= 0 && candleAgeAtEvaluationMs <= freshnessLimitMs
    && receiptAgeAtEvaluationMs >= 0 && receiptAgeAtEvaluationMs <= MAX_MARKET_RECEIPT_AGE_MS;
  const isTrustedFreshSnapshot = isFresh === true && isLive === true && providerStatus.startsWith('CONNECTED') && timestampsRemainFresh;
  const decision = evaluateResearchDecision({
    intent: 'research',
    marketData: { validated: isTrustedFreshSnapshot, source, asOf, datasetSha256, symbol, timeframe },
    signals,
    evidenceRegistry,
  });
  return Object.freeze({
    decision,
    registrySha256: evidenceRegistry.registrySha256,
    registryRecordCount: evidenceRegistry.recordCount,
    decisionInputSource: 'RUST_DERIVED_DETERMINISTIC_EVIDENCE_ONLY',
    executionAuthority: 'none',
    liveExecutionAuthorized: false,
    orderIntent: null,
    aiNarrativeExcludedFromScoring: true,
  });
}
