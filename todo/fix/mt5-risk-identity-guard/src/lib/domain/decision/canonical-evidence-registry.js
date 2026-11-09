/**
 * Canonical, content-addressed evidence registry.
 *
 * The registry is pure domain code: no filesystem, network, provider, or runtime authority.
 * Hashes use Web Crypto SHA-256 and deterministic JSON normalization. Only registries created
 * by this module are accepted by the in-process decision engine; deserialized snapshots must
 * be rebuilt through createCanonicalEvidenceRegistry before use.
 */
const SHA256 = /^[a-f0-9]{64}$/;
const ISO_UTC = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.\d{1,9})?Z$/;
export const CANONICAL_EVIDENCE_REGISTRY_VERSION = 'canonical-evidence-registry.v1';
export const MAX_EVIDENCE_RECORDS = 512;
export const MAX_EVIDENCE_PAYLOAD_BYTES = 16_384;
const MAX_REGISTRY_BYTES = 1_048_576;
const MAX_PARENT_REFS = 16;
const MAX_DEPTH = 12;
const MAX_ARRAY_ITEMS = 256;
const MAX_OBJECT_KEYS = 128;
const MAX_STRING_LENGTH = 4096;

const registryState = new WeakMap();

/** @typedef {{canonicalId:string,contentSha256:string,source:string,observedAt:string,datasetSha256:string,symbol:string,timeframe:string,payload:unknown,parentEvidenceIds:readonly string[]}} CanonicalEvidenceRecord */
/** @typedef {{schemaVersion:string,registrySha256:string,recordCount:number,records:readonly CanonicalEvidenceRecord[]}} CanonicalEvidenceRegistry */
/** @typedef {{ref:string,source:string,observedAt:string,datasetSha256:string,symbol:string,timeframe:string,payload:unknown,parentRefs:string[]}} CanonicalDescriptor */

/** @param {unknown} condition @param {string} message @returns {asserts condition} */
function assert(condition, message) {
  if (!condition) throw new TypeError(message);
}

/** @param {unknown} value @returns {value is string} */
function validTimestamp(value) {
  if (typeof value !== 'string') return false;
  const match = ISO_UTC.exec(value);
  if (!match) return false;
  const timestamp = Date.parse(value);
  if (!Number.isFinite(timestamp)) return false;
  const date = new Date(timestamp);
  return date.getUTCFullYear() === Number(match[1])
    && date.getUTCMonth() + 1 === Number(match[2])
    && date.getUTCDate() === Number(match[3])
    && date.getUTCHours() === Number(match[4])
    && date.getUTCMinutes() === Number(match[5])
    && date.getUTCSeconds() === Number(match[6]);
}

/** @param {unknown} value @returns {value is Record<string, unknown>} */
function isPlainObject(value) {
  if (value === null || typeof value !== 'object') return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

/** @param {unknown} value @param {number} [depth=0] @param {WeakSet<object>} [seen] @returns {unknown} */
function normalizeJson(value, depth = 0, seen = new WeakSet()) {
  assert(depth <= MAX_DEPTH, `Evidence payload exceeds maximum nesting depth ${MAX_DEPTH}.`);
  if (value === null || typeof value === 'boolean') return value;
  if (typeof value === 'string') {
    assert(value.length <= MAX_STRING_LENGTH, `Evidence string exceeds ${MAX_STRING_LENGTH} characters.`);
    return value;
  }
  if (typeof value === 'number') {
    assert(Number.isFinite(value), 'Evidence payload cannot contain non-finite numbers.');
    return Object.is(value, -0) ? 0 : value;
  }
  assert(typeof value === 'object', 'Evidence payload must contain JSON-compatible values only.');
  assert(!seen.has(value), 'Evidence payload must not contain cycles.');
  seen.add(value);
  try {
    if (Array.isArray(value)) {
      assert(value.length <= MAX_ARRAY_ITEMS, `Evidence arrays are limited to ${MAX_ARRAY_ITEMS} items.`);
      const ownKeys = Object.keys(value);
      assert(ownKeys.length === value.length && ownKeys.every((key, index) => key === String(index)), 'Evidence arrays must be dense and must not contain extra properties.');
      const normalized = [];
      for (let index = 0; index < value.length; index += 1) {
        const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
        assert(descriptor && Object.hasOwn(descriptor, 'value'), 'Evidence arrays must not contain accessor properties.');
        normalized.push(normalizeJson(descriptor.value, depth + 1, seen));
      }
      return normalized;
    }
    assert(isPlainObject(value), 'Evidence payload objects must be plain objects.');
    const keys = Object.keys(value);
    assert(keys.length <= MAX_OBJECT_KEYS, `Evidence objects are limited to ${MAX_OBJECT_KEYS} keys.`);
    const result = Object.create(null);
    for (const key of keys.sort()) {
      assert(key.length > 0 && key.length <= 128 && key !== '__proto__' && key !== 'constructor' && key !== 'prototype', 'Evidence payload contains a forbidden object key.');
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      assert(descriptor && Object.hasOwn(descriptor, 'value'), 'Evidence objects must not contain accessor properties.');
      result[key] = normalizeJson(descriptor.value, depth + 1, seen);
    }
    return result;
  } finally {
    seen.delete(value);
  }
}

/** @param {unknown} value @returns {string} */
function canonicalJson(value) {
  // normalizeJson sorts keys recursively and removes unsupported JS object semantics.
  return JSON.stringify(normalizeJson(value));
}

/** Return the registry's bounded canonical JSON representation for comparing native payload mirrors.
 * This is normalization, not authentication: callers must still verify the native digest.
 * @param {unknown} value
 * @returns {string}
 */
export function canonicalizeEvidencePayload(value) {
  return canonicalJson(value);
}

/** @param {unknown} value @returns {unknown} */
function deepFreeze(value) {
  if (value && typeof value === 'object' && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) deepFreeze(child);
    Object.freeze(value);
  }
  return value;
}

/** @param {string} text @returns {Promise<string>} */
async function sha256Hex(text) {
  const cryptoApi = globalThis.crypto;
  assert(cryptoApi?.subtle && typeof cryptoApi.subtle.digest === 'function', 'Web Crypto SHA-256 is unavailable; canonical evidence registration fails closed.');
  const bytes = new TextEncoder().encode(text);
  const digest = await cryptoApi.subtle.digest('SHA-256', bytes);
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** @param {unknown} descriptor @param {number} index @returns {CanonicalDescriptor} */
function validateDescriptor(descriptor, index) {
  assert(isPlainObject(descriptor), `Evidence descriptor ${index + 1} must be a plain object.`);
  assert(typeof descriptor.ref === 'string' && descriptor.ref.trim().length > 0 && descriptor.ref.length <= 256, `Evidence descriptor ${index + 1} requires a bounded, non-empty source reference.`);
  assert(typeof descriptor.source === 'string' && descriptor.source.trim().length > 0 && descriptor.source.length <= 128, `Evidence descriptor ${index + 1} requires a bounded source identifier.`);
  assert(validTimestamp(descriptor.observedAt), `Evidence descriptor ${index + 1} requires an explicit UTC observation timestamp.`);
  assert(typeof descriptor.datasetSha256 === 'string' && SHA256.test(descriptor.datasetSha256.toLowerCase()), `Evidence descriptor ${index + 1} requires a SHA-256 dataset fingerprint.`);
  assert(typeof descriptor.symbol === 'string' && /^[A-Z0-9_-]{1,30}$/.test(descriptor.symbol), `Evidence descriptor ${index + 1} has an invalid instrument scope.`);
  assert(typeof descriptor.timeframe === 'string' && /^[A-Za-z0-9_-]{1,16}$/.test(descriptor.timeframe), `Evidence descriptor ${index + 1} has an invalid timeframe scope.`);
  const rawParentRefs = descriptor.parentRefs ?? [];
  assert(Array.isArray(rawParentRefs), `Evidence descriptor ${index + 1} parentRefs must be an array.`);
  assert(rawParentRefs.length <= MAX_PARENT_REFS, `Evidence descriptor ${index + 1} exceeds ${MAX_PARENT_REFS} parent references.`);
  assert(rawParentRefs.every((ref) => typeof ref === 'string' && ref.length > 0 && ref.length <= 256), `Evidence descriptor ${index + 1} has malformed parent references.`);
  const payload = deepFreeze(normalizeJson(descriptor.payload));
  const payloadText = JSON.stringify(payload);
  assert(new TextEncoder().encode(payloadText).byteLength <= MAX_EVIDENCE_PAYLOAD_BYTES, `Evidence descriptor ${index + 1} payload exceeds ${MAX_EVIDENCE_PAYLOAD_BYTES} bytes.`);
  return {
    ref: descriptor.ref,
    source: descriptor.source.trim(),
    observedAt: descriptor.observedAt,
    datasetSha256: descriptor.datasetSha256.toLowerCase(),
    symbol: descriptor.symbol,
    timeframe: descriptor.timeframe,
    payload,
    parentRefs: [.../** @type {string[]} */ (rawParentRefs)],
  };
}

/**
 * Build a bounded registry from descriptors. parentRefs refer to descriptor.ref aliases and
 * are resolved topologically; cycles and unresolved parents fail closed. Identical canonical
 * content receives the same ID even if callers give it different source-reference aliases.
 * @param {unknown[]} descriptors
 */
/** @param {unknown[]} descriptors @returns {Promise<CanonicalEvidenceRegistry>} */
export async function createCanonicalEvidenceRegistry(descriptors) {
  assert(Array.isArray(descriptors), 'Evidence descriptors must be an array.');
  assert(descriptors.length > 0 && descriptors.length <= MAX_EVIDENCE_RECORDS, `Evidence registry must contain 1..${MAX_EVIDENCE_RECORDS} descriptors.`);
  const normalized = descriptors.map(validateDescriptor);
  const aliasMap = new Map();
  for (const item of normalized) {
    assert(!aliasMap.has(item.ref), `Duplicate evidence source reference alias: ${item.ref}.`);
    aliasMap.set(item.ref, item);
  }
  const resolved = new Map();
  const resolving = new Set();
  const recordsById = new Map();
  const aliasToId = new Map();

  /** @param {CanonicalDescriptor} item @returns {Promise<string>} */
  async function resolve(item) {
    if (resolved.has(item.ref)) return resolved.get(item.ref);
    assert(!resolving.has(item.ref), `Evidence derivation cycle detected at ${item.ref}.`);
    resolving.add(item.ref);
    const parentIds = [];
    for (const parentRef of item.parentRefs) {
      const parent = aliasMap.get(parentRef);
      assert(parent, `Evidence parent reference is unresolved: ${parentRef}.`);
      parentIds.push(await resolve(parent));
    }
    parentIds.sort();
    const canonicalContent = {
      schemaVersion: CANONICAL_EVIDENCE_REGISTRY_VERSION,
      source: item.source,
      observedAt: item.observedAt,
      datasetSha256: item.datasetSha256,
      symbol: item.symbol,
      timeframe: item.timeframe,
      payload: item.payload,
      parentEvidenceIds: parentIds,
    };
    const canonicalText = canonicalJson(canonicalContent);
    const digest = await sha256Hex(canonicalText);
    const canonicalId = `ev1:${digest}`;
    const existing = recordsById.get(canonicalId);
    if (existing) {
      assert(existing.canonicalContent === canonicalText, 'SHA-256 collision detected while registering evidence.');
    } else {
      recordsById.set(canonicalId, {
        canonicalId,
        contentSha256: digest,
        source: item.source,
        observedAt: item.observedAt,
        datasetSha256: item.datasetSha256,
        symbol: item.symbol,
        timeframe: item.timeframe,
        payload: item.payload,
        parentEvidenceIds: parentIds,
        canonicalContent: canonicalText,
      });
    }
    aliasToId.set(item.ref, canonicalId);
    resolved.set(item.ref, canonicalId);
    resolving.delete(item.ref);
    return canonicalId;
  }

  for (const item of normalized) await resolve(item);
  const records = [...recordsById.values()].sort((a, b) => a.canonicalId.localeCompare(b.canonicalId)).map((record) => Object.freeze({
    canonicalId: record.canonicalId,
    contentSha256: record.contentSha256,
    source: record.source,
    observedAt: record.observedAt,
    datasetSha256: record.datasetSha256,
    symbol: record.symbol,
    timeframe: record.timeframe,
    payload: record.payload,
    parentEvidenceIds: Object.freeze([...record.parentEvidenceIds]),
  }));
  const registryCanonical = canonicalJson(records);
  assert(new TextEncoder().encode(registryCanonical).byteLength <= MAX_REGISTRY_BYTES, `Canonical evidence registry exceeds ${MAX_REGISTRY_BYTES} bytes.`);
  const registrySha256 = await sha256Hex(registryCanonical);
  const registry = Object.freeze({
    schemaVersion: CANONICAL_EVIDENCE_REGISTRY_VERSION,
    registrySha256,
    recordCount: records.length,
    records: Object.freeze(records),
  });
  registryState.set(registry, {
    recordsById: new Map(records.map((record) => [record.canonicalId, record])),
    aliasToId,
  });
  return registry;
}

/** @param {unknown} registry */
/** @param {unknown} registry @returns {registry is CanonicalEvidenceRegistry} */
export function isVerifiedCanonicalEvidenceRegistry(registry) {
  return Boolean(registry && typeof registry === 'object' && registryState.has(registry));
}

/** @param {unknown} registry @param {unknown} sourceRef */
/** @param {unknown} registry @param {unknown} sourceRef @returns {string|null} */
export function resolveCanonicalEvidenceId(registry, sourceRef) {
  if (!registry || typeof registry !== 'object') return null;
  return registryState.get(registry)?.aliasToId.get(sourceRef) ?? null;
}

/** @param {unknown} registry @param {unknown} canonicalId */
/** @param {unknown} registry @param {unknown} canonicalId @returns {CanonicalEvidenceRecord|null} */
export function getCanonicalEvidenceRecord(registry, canonicalId) {
  if (!registry || typeof registry !== 'object') return null;
  return registryState.get(registry)?.recordsById.get(canonicalId) ?? null;
}

/** @param {unknown} registry @returns {string|null} */
export function getCanonicalEvidenceRegistryDigest(registry) {
  return isVerifiedCanonicalEvidenceRegistry(registry) ? registry.registrySha256 : null;
}
