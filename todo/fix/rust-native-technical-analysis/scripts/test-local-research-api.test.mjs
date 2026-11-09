import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { MAX_API_CANDLES, MAX_API_BODY_BYTES, canonicalDatasetSha256, parseAnalysisRequest, readBoundedBody } from '../api/src/validation.ts';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = (file) => readFile(path.join(root, file), 'utf8');
const validRequest = (count = 34) => ({
  symbol: 'eurusd', timeframe: '1m',
  candles: Array.from({ length: count }, (_, i) => ({ time: Math.floor(Date.now() / 60_000) * 60_000 - (count - i - 1) * 60_000, open: 1.1, high: 1.2, low: 1.0, close: 1.15, volume: 10 })),
});

test('API request validator returns normalized immutable data for a valid contiguous series', () => {
  const result = parseAnalysisRequest(validRequest());
  assert.equal(result.symbol, 'EURUSD');
  assert.equal(result.candles.length, 34);
  assert.equal(Object.isFrozen(result), true);
  assert.equal(Object.isFrozen(result.candles[0]), true);
});

test('API request validator rejects unsupported scope, insufficient history, gaps, duplicates, and malformed OHLCV', () => {
  assert.equal(parseAnalysisRequest({ ...validRequest(), timeframe: '2m' }), null);
  assert.equal(parseAnalysisRequest(validRequest(33)), null);
  const gap = validRequest(); gap.candles[20].time += 60_000;
  assert.equal(parseAnalysisRequest(gap), null);
  const duplicate = validRequest(); duplicate.candles[20].time = duplicate.candles[19].time;
  assert.equal(parseAnalysisRequest(duplicate), null);
  const malformed = validRequest(); malformed.candles[4].high = 0.5;
  assert.equal(parseAnalysisRequest(malformed), null);
  assert.equal(parseAnalysisRequest(null), null);
  const future = validRequest(); future.candles[33].time = Math.floor(Date.now() / 60_000) * 60_000 + 120_000;
  assert.equal(parseAnalysisRequest(future), null);
});

test('API bounds request work and rejects excessive series length', () => {
  assert.equal(MAX_API_CANDLES, 2_000);
  assert.equal(MAX_API_BODY_BYTES, 2 * 1024 * 1024);
  assert.equal(parseAnalysisRequest(validRequest(MAX_API_CANDLES + 1)), null);
});

test('Hono adapter is loopback-only, authenticated, bounded, read-only, and has no order route', async () => {
  const app = await read('api/src/app.ts');
  const server = await read('api/src/server.ts');
  const rateLimiter = await read('api/src/rate-limit.ts');
  const ratePolicy = await read('api/src/request-rate-policy.ts');
  const workflow = await read('.github/workflows/ci.yml');
  assert.match(app, /const requestId = crypto\.randomUUID\(\)/, 'request IDs must be generated server-side');
  assert.match(app, /c\.header\('X-Request-ID', requestId\)/, 'server-generated request IDs must support incident correlation');
  assert.match(app, /buildRequestObservation/);
  assert.match(app, /options\.observe/);
  assert.match(app, /options\.token\.length < 32/);
  assert.match(app, /RATE_MAX_REQUESTS = 30/);
  assert.match(app, /createResearchApiRateLimits/);
  assert.match(app, /Retry-After/);
  const contextGuardIndex = app.indexOf('validateRequestContext({');
  const rateLimitIndex = app.indexOf('const budget = rateLimits.consumeAuthenticatedRequest();');
  const credentialComparisonIndex = app.indexOf('if (diff !== 0) {');
  assert.ok(contextGuardIndex >= 0 && contextGuardIndex < credentialComparisonIndex && credentialComparisonIndex < rateLimitIndex, 'context and authentication must be checked before the authenticated request budget');
  assert.match(app, /consumeFailedAuthentication\(\)/, 'failed authentication uses an isolated abuse budget');
  assert.match(await read('api/src/request-origin-policy.ts'), /browser_origin_not_supported/);
  assert.doesNotMatch(app, /Access-Control-Allow-Origin/i, 'the local API must not silently enable permissive CORS');
  assert.ok(app.includes('for (let i = 0; i < b.length; i++)'), 'Bearer comparison work loop must not scale with attacker-supplied header length');
  assert.match(rateLimiter, /new Float64Array\(limit\)/);
  assert.match(ratePolicy, /authenticatedRequests: 30/);
  assert.match(ratePolicy, /failedAuthenticationAttempts: 10/);
  assert.match(ratePolicy, /consumeAuthenticatedRequest/);
  assert.match(ratePolicy, /consumeFailedAuthentication/);
  assert.match(rateLimiter, /Math\.max\(observed, lastNow\)/);
  assert.match(workflow, /bun test api\/test\/app\.integration\.test\.ts/);
  assert.match(app, /readBoundedBody\(c\.req\.raw\)/);
  const validation = await read('api/src/validation.ts');
  assert.match(validation, /MAX_API_BODY_BYTES - total/);
  assert.match(validation, /timeoutMs \?\? 10_000/);
  assert.match(validation, /kind: 'timed_out'/);
  assert.match(app, /bounded.kind === 'timed_out'/);
  assert.doesNotMatch(validation, /await reader\.cancel\(/, 'request completion must never depend on an underlying stream cancellation hook');
  assert.match(validation, /releaseReaderLock = false/, 'timeout cleanup must not release a reader with a pending read request');
  assert.match(app, /'Cache-Control': 'no-store, max-age=0'/);
  assert.match(app, /c\.req\.header\('content-encoding'\)/);
  assert.match(app, /MAX_API_BODY_BYTES/);
  assert.match(app, /canonicalDatasetSha256\(request\.candles\)/);
  assert.match(app, /getLatestClosedBarAtMs\(request\.candles, request\.timeframe\)/);
  assert.match(app, /latestClosedBarAtMs, datasetSha256/);
  assert.doesNotMatch(app, /new Date\(request\.candles\.at\(-1\)!\.time\)/, 'API cutoffs must represent the end of the requested candle interval');
  assert.match(app, /datasetFingerprintComputed: true/);
  assert.match(app, /datasetFingerprintVerified: false/);
  assert.ok(app.includes('charset\\s*=\\s*(?:utf-8|utf8'), 'Only UTF-8 JSON charsets should be accepted');
  assert.match(app, /providerAuthenticated: false/);
  assert.match(app, /researchCandidateHandoffAllowed: false/);
  assert.match(app, /executionAuthority: 'none'/);
  assert.doesNotMatch(app, /app\.(?:post|put|patch|delete)\(['"]\/v\d+\/(?:orders|execute|trade)/i);
  assert.match(server, /host !== '127\.0\.0\.1'/);
  assert.match(server, /host !== '::1'/);
  assert.match(server, /expectedPort: port/);
  assert.doesNotMatch(server, /host !== 'localhost'/);
  assert.match(server, /maxRequestBodySize: 2 \* 1024 \* 1024/);
});


test('streaming body reader enforces the byte cap without Content-Length and rejects invalid UTF-8', async () => {
  const oversized = new Uint8Array(MAX_API_BODY_BYTES + 1);
  const stream = new ReadableStream({
    start(controller) {
      controller.enqueue(oversized.subarray(0, MAX_API_BODY_BYTES));
      controller.enqueue(oversized.subarray(MAX_API_BODY_BYTES));
    },
  });
  const result = await readBoundedBody(new Request('http://localhost/', { method: 'POST', body: stream, duplex: 'half' }));
  assert.deepEqual(result, { kind: 'too_large' });

  const hostileCancellation = new ReadableStream({
    start(controller) { controller.enqueue(new Uint8Array(MAX_API_BODY_BYTES + 1)); },
    cancel() { return new Promise(() => {}); },
  });
  const bounded = await Promise.race([
    readBoundedBody(new Request('http://localhost/', { method: 'POST', body: hostileCancellation, duplex: 'half' }), { timeoutMs: 20 }),
    new Promise((resolve) => setTimeout(() => resolve('hung'), 250)),
  ]);
  assert.deepEqual(bounded, { kind: 'too_large' }, 'a hanging cancellation hook must not defeat the request byte cap');

  const invalid = new ReadableStream({ start(controller) { controller.enqueue(new Uint8Array([0xc3, 0x28])); controller.close(); } });
  assert.deepEqual(await readBoundedBody(new Request('http://localhost/', { method: 'POST', body: invalid, duplex: 'half' })), { kind: 'invalid_utf8' });

  const fragmented = new ReadableStream({ start(controller) { for (const byte of new TextEncoder().encode('{"safe":true}')) controller.enqueue(new Uint8Array([byte])); controller.close(); } });
  assert.deepEqual(await readBoundedBody(new Request('http://localhost/', { method: 'POST', body: fragmented, duplex: 'half' })), { kind: 'ok', text: '{"safe":true}' });
});


test('canonical OHLCV fingerprint matches the cross-runtime Rust known vector', async () => {
  assert.equal(await canonicalDatasetSha256([{ time: 1, open: 1, high: 2, low: 0.5, close: 1.5, volume: 3 }]), 'f2372778376160ebe9f4759a6a81dc8e0b76020e21dcc8dafe48d1f76134dec6');
  const base = [{ time: 10, open: 10, high: 12, low: 9, close: 11, volume: 42 }];
  const digest = await canonicalDatasetSha256(base);
  assert.equal(await canonicalDatasetSha256([{ ...base[0] }]), digest);
  assert.notEqual(await canonicalDatasetSha256([{ ...base[0], close: 11.01 }]), digest);
});


test('streaming body reader times out stalled streams even when cancellation never settles', async () => {
  const stalled = new ReadableStream({
    pull() { return new Promise(() => {}); },
    cancel() { return new Promise(() => {}); },
  });
  const started = Date.now();
  const result = await Promise.race([
    readBoundedBody(new Request('http://localhost/', { method: 'POST', body: stalled, duplex: 'half' }), { timeoutMs: 15 }),
    new Promise((resolve) => setTimeout(() => resolve('hung'), 250)),
  ]);
  assert.deepEqual(result, { kind: 'timed_out' });
  assert.ok(Date.now() - started < 500, 'stalled request must not wait for the source cancellation hook');
  await assert.rejects(() => readBoundedBody(new Request('http://localhost/'), { timeoutMs: 0 }), /timeoutMs/);
});


test('Bun/Hono runtime implementation is TypeScript-only at the API boundary', async () => {
  const sourceFiles = await readdir(path.join(root, 'api/src'));
  assert.equal(sourceFiles.filter((file) => file.endsWith('.js')).length, 0, 'API runtime must not retain JavaScript implementation modules');
  for (const file of ['app.ts', 'server.ts', 'validation.ts', 'rate-limit.ts', 'request-rate-policy.ts', 'request-origin-policy.ts', 'request-observability.ts']) {
    assert.ok(sourceFiles.includes(file), `${file} must remain in the typed API runtime`);
  }
  const server = await read('api/src/server.ts');
  const app = await read('api/src/app.ts');
  const packageJson = JSON.parse(await read('package.json'));
  assert.match(server, /Bun\.serve\(/);
  assert.match(app, /from 'hono'/);
  assert.match(packageJson.scripts['dev:api'], /^bun --hot run api\/src\/server\.ts$/);
  assert.match(packageJson.scripts['start:api'], /^bun run api\/src\/server\.ts$/);
});
