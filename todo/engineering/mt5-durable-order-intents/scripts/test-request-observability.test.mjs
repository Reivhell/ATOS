import test from 'node:test';
import assert from 'node:assert/strict';
import { buildRequestObservation, classifyRequestOutcome, createBoundedRequestObserver, REQUEST_OBSERVABILITY_SCHEMA } from '../api/src/request-observability.ts';
import { readFile } from 'node:fs/promises';

const ID = '7d8f9c12-45a6-4d78-8a9b-0123456789ab';

test('request observations use a finite schema and redact arbitrary paths and query strings', () => {
  const event = buildRequestObservation({
    requestId: ID,
    method: 'post',
    pathname: '/private/account/secret?token=never-log-this',
    statusCode: 201,
    durationMs: 12.7,
    outcome: 'success',
    authorization: 'Bearer never-log-this',
    body: { password: 'never-log-this' },
  });
  assert.deepEqual(event, {
    schema: REQUEST_OBSERVABILITY_SCHEMA,
    event: 'http.request.completed',
    requestId: ID,
    method: 'POST',
    route: '/other',
    statusCode: 201,
    durationMs: 13,
    outcome: 'success',
  });
  assert.equal(JSON.stringify(event).includes('never-log-this'), false);
  assert.equal(Object.isFrozen(event), true);
});

test('known route labels are stable and unknown values fail closed', () => {
  assert.equal(buildRequestObservation({ requestId: ID, method: 'GET', pathname: '/health', statusCode: 200, durationMs: 0, outcome: 'success' }).route, '/health');
  assert.equal(buildRequestObservation({ requestId: ID, method: 'GET', pathname: '/v1/analysis', statusCode: 200, durationMs: 0, outcome: 'success' }).route, '/v1/analysis');
  const fallback = buildRequestObservation({ requestId: 'client-controlled-id', method: 'TRACE', pathname: '/health?secret=1', statusCode: 99, durationMs: Infinity, outcome: 'attacker-controlled' });
  assert.equal(fallback.requestId, 'invalid-request-id');
  assert.equal(fallback.method, 'OTHER');
  assert.equal(fallback.statusCode, 500);
  assert.equal(fallback.durationMs, 0);
  assert.equal(fallback.outcome, 'server_error');
});

test('duration values are bounded to prevent corrupt clocks from producing huge log values', () => {
  assert.equal(buildRequestObservation({ requestId: ID, method: 'GET', pathname: '/', statusCode: 200, durationMs: -42, outcome: 'success' }).durationMs, 0);
  assert.equal(buildRequestObservation({ requestId: ID, method: 'GET', pathname: '/', statusCode: 200, durationMs: 999_999_999, outcome: 'success' }).durationMs, 300_000);
});

test('outcome classifier maps internal codes to finite non-sensitive labels', () => {
  assert.equal(classifyRequestOutcome('request_context_rejected', 403), 'request_context_rejected');
  assert.equal(classifyRequestOutcome('rate_limited', 429), 'rate_limited');
  assert.equal(classifyRequestOutcome('unauthorized', 401), 'unauthorized');
  assert.equal(classifyRequestOutcome('body_too_large', 413), 'payload_too_large');
  assert.equal(classifyRequestOutcome('request_timeout', 408), 'request_timeout');
  assert.equal(classifyRequestOutcome('anything-else', 400), 'invalid_request');
  assert.equal(classifyRequestOutcome('anything-else', 503), 'server_error');
});

test('bounded observer limits emitted rows and reports suppression count without attacker-keyed memory', () => {
  let now = 0;
  const output = [];
  const observe = createBoundedRequestObserver((event) => output.push(event), { maxEvents: 2, windowMs: 100, now: () => now });
  const event = buildRequestObservation({ requestId: ID, method: 'GET', pathname: '/health', statusCode: 200, durationMs: 1, outcome: 'success' });
  observe(event); observe(event); observe(event); observe(event); observe(event);
  assert.equal(output.length, 2, 'event sink must stop writing after configured cap');
  now = 100;
  observe(event);
  assert.equal(output.length, 3);
  assert.equal(output[2].suppressedEventsSincePreviousEmission, 3);
});

test('invalid observer configuration is rejected early', () => {
  assert.throws(() => createBoundedRequestObserver(() => {}, { maxEvents: 0 }), /maxEvents/);
  assert.throws(() => createBoundedRequestObserver(() => {}, { windowMs: 0 }), /windowMs/);
  assert.throws(() => createBoundedRequestObserver(null), /writer/);
});

test('API wires safe request observation through finally and production server uses the bounded sink', async () => {
  const app = await readFile(new URL('../api/src/app.ts', import.meta.url), 'utf8');
  const server = await readFile(new URL('../api/src/server.ts', import.meta.url), 'utf8');
  assert.match(app, /buildRequestObservation\(/);
  assert.match(app, /finally\s*\{/);
  assert.match(app, /options\.observe\(/);
  assert.match(server, /createBoundedRequestObserver/);
  assert.match(server, /maxEvents: 120, windowMs: 60_000/);
  assert.match(server, /console\.info\(JSON\.stringify\(event\)\)/);
  assert.doesNotMatch(server, /console\.(?:info|error)\([^\n]*(?:token|authorization|req\.raw\.text)/i);
});
