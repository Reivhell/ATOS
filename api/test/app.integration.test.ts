import { describe, expect, test } from 'bun:test';
import { createResearchApi } from '../src/app.ts';

const TOKEN = 'integration-test-token-that-is-longer-than-32-characters';
const makeCandles = (count = 40) => Array.from({ length: count }, (_, index) => {
  const close = 1.1 + index * 0.0001;
  return { time: Math.floor(Date.now() / 60_000) * 60_000 - (count - index - 1) * 60_000, open: close - 0.00005, high: close + 0.0001, low: close - 0.0001, close, volume: 100 };
});
const request = (app: ReturnType<typeof createResearchApi>, path: string, init: RequestInit = {}) => app.request(path, init);

 describe('Bun/Hono local research API', () => {
  test('assigns a server-generated request ID to successful and rejected responses', async () => {
    const app = createResearchApi({ token: TOKEN });
    const rejected = await request(app, '/health');
    const accepted = await request(app, '/health', { headers: { authorization: `Bearer ${TOKEN}` } });
    const rejectedId = rejected.headers.get('x-request-id');
    const acceptedId = accepted.headers.get('x-request-id');
    expect(rejectedId).toMatch(/^[0-9a-f-]{36}$/i);
    expect(acceptedId).toMatch(/^[0-9a-f-]{36}$/i);
    expect(acceptedId).not.toBe(rejectedId);
    expect(rejected.status).toBe(401);
    expect(accepted.status).toBe(200);
  });

  test('emits correlated safe observations without logging caller IDs, query secrets, or credentials', async () => {
    const records: Record<string, unknown>[] = [];
    let clock = 1_000;
    const app = createResearchApi({
      token: TOKEN,
      observationNow: () => clock += 2,
      observe: (event) => records.push(event as unknown as Record<string, unknown>),
    });
    const callerId = 'client-controlled-request-id';
    const secretQuery = 'query-secret-must-not-be-logged';
    const rejected = await request(app, `/private/customer?apiKey=${secretQuery}`, {
      headers: { 'x-request-id': callerId, authorization: 'Bearer invalid-credentials' },
    });
    expect(rejected.status).toBe(401);
    const returnedId = rejected.headers.get('x-request-id');
    expect(returnedId).toMatch(/^[0-9a-f-]{36}$/i);
    expect(returnedId).not.toBe(callerId);
    const accepted = await request(app, '/health', { headers: { authorization: `Bearer ${TOKEN}` } });
    expect(accepted.status).toBe(200);
    expect(records).toHaveLength(2);
    expect(records[0].requestId).toBe(returnedId);
    expect(records[0].route).toBe('/other');
    expect(records[0].outcome).toBe('unauthorized');
    expect(records[0].statusCode).toBe(401);
    expect(records[1].route).toBe('/health');
    expect(records[1].outcome).toBe('success');
    const serialized = JSON.stringify(records);
    expect(serialized).not.toContain(secretQuery);
    expect(serialized).not.toContain(TOKEN);
    expect(serialized).not.toContain('invalid-credentials');
    expect(serialized).not.toContain('/private/customer');
  });

  test('observer exceptions cannot alter an otherwise successful API response', async () => {
    const app = createResearchApi({ token: TOKEN, observe: () => { throw new Error('sensitive observer failure'); } });
    const response = await request(app, '/health', { headers: { authorization: `Bearer ${TOKEN}` } });
    expect(response.status).toBe(200);
    expect((await response.json()).executionAuthority).toBe('none');
  });

  test('observation clock faults do not become request-availability failures', async () => {
    const app = createResearchApi({ token: TOKEN, observationNow: () => { throw new Error('clock unavailable'); } });
    const response = await request(app, '/health', { headers: { authorization: `Bearer ${TOKEN}` } });
    expect(response.status).toBe(200);
  });

  test('requires bearer authentication for health and analysis routes', async () => {
    const app = createResearchApi({ token: TOKEN });
    const response = await request(app, '/health');
    expect(response.status).toBe(401);
    expect((await response.json()).error.code).toBe('unauthorized');
  });

  test('failed authentication floods cannot consume the authenticated request budget', async () => {
    const app = createResearchApi({ token: TOKEN });
    let throttled = 0;
    for (let i = 0; i < 40; i++) {
      const response = await request(app, '/health', { headers: { authorization: 'Bearer wrong-token' } });
      if (response.status === 429) {
        throttled++;
        expect(response.headers.get('retry-after')).toBeTruthy();
      } else {
        expect(response.status).toBe(401);
      }
    }
    expect(throttled).toBe(30);
    const legitimate = await request(app, '/health', { headers: { authorization: `Bearer ${TOKEN}` } });
    expect(legitimate.status).toBe(200);
    expect((await legitimate.json()).executionAuthority).toBe('none');
  });

  test('rejects hostile browser origins before they can consume the shared request budget', async () => {
    const app = createResearchApi({ token: TOKEN });
    for (let i = 0; i < 35; i++) {
      const hostile = await request(app, '/health', { headers: { origin: 'https://evil.example', 'sec-fetch-site': 'cross-site' } });
      expect(hostile.status).toBe(403);
      expect((await hostile.json()).error.code).toBe('request_context_rejected');
    }
    const safeHeaders = { authorization: `Bearer ${TOKEN}` };
    for (let i = 0; i < 30; i++) expect((await request(app, '/health', { headers: safeHeaders })).status).toBe(200);
    const limited = await request(app, '/health', { headers: safeHeaders });
    expect(limited.status).toBe(429);
  });

  test('rejects untrusted Host authorities, all browser Origins, and cross-site Fetch Metadata', async () => {
    const app = createResearchApi({ token: TOKEN });
    const badHost = await request(app, '/health', { headers: { host: 'research.attacker.example' } });
    expect(badHost.status).toBe(403);
    const crossSite = await request(app, '/health', { headers: { 'sec-fetch-site': 'cross-site' } });
    expect(crossSite.status).toBe(403);
    const browserOrigin = await request(app, '/health', { headers: { authorization: `Bearer ${TOKEN}`, origin: 'tauri://localhost', 'sec-fetch-site': 'cross-site' } });
    expect(browserOrigin.status).toBe(403);
  });

  test('accepts a valid bounded analysis request but never promotes client data to trusted decision or execution authority', async () => {
    const app = createResearchApi({ token: TOKEN });
    const response = await request(app, '/v1/analysis', {
      method: 'POST',
      headers: { authorization: `Bearer ${TOKEN}`, 'content-type': 'application/json' },
      body: JSON.stringify({ symbol: 'EURUSD', timeframe: '1m', candles: makeCandles() }),
    });
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.trust.providerAuthenticated).toBe(false);
    expect(body.trust.datasetFingerprintComputed).toBe(true);
    expect(body.trust.datasetFingerprintVerified).toBe(false);
    expect(body.trust.datasetSha256).toMatch(/^[a-f0-9]{64}$/);
    expect(body.trust.researchCandidateHandoffAllowed).toBe(false);
    expect(body.trust.executionAuthority).toBe('none');
    expect(body.report.authority.liveExecutionAuthorized).toBe(false);
  });

  test('rejects malformed input and exposes no order routes', async () => {
    const app = createResearchApi({ token: TOKEN });
    const headers = { authorization: `Bearer ${TOKEN}`, 'content-type': 'application/json' };
    const invalid = await request(app, '/v1/analysis', { method: 'POST', headers, body: JSON.stringify({ symbol: 'EURUSD', timeframe: '1m', candles: [] }) });
    expect(invalid.status).toBe(400);
    const orders = await request(app, '/v1/orders', { method: 'POST', headers, body: '{}' });
    expect(orders.status).toBe(404);
    const wrongType = await request(app, '/v1/analysis', { method: 'POST', headers: { authorization: `Bearer ${TOKEN}`, 'content-type': 'text/plain' }, body: '{}' });
    expect(wrongType.status).toBe(415);
    const wrongCharset = await request(app, '/v1/analysis', { method: 'POST', headers: { ...headers, 'content-type': 'application/json; charset=iso-8859-1' }, body: '{}' });
    expect(wrongCharset.status).toBe(415);
    const utf8 = await request(app, '/v1/analysis', { method: 'POST', headers: { ...headers, 'content-type': 'application/json; charset=utf-8' }, body: JSON.stringify({ symbol: 'EURUSD', timeframe: '1m', candles: makeCandles() }) });
    expect(utf8.status).toBe(200);
  });

  test('applies the request budget to failed authentication attempts and emits security headers', async () => {
    const app = createResearchApi({ token: TOKEN });
    let response: Response | undefined;
    for (let i = 0; i < 31; i++) response = await request(app, '/health');
    expect(response?.status).toBe(429);
    expect(response?.headers.get('cache-control')).toContain('no-store');
    expect(response?.headers.get('x-content-type-options')).toBe('nosniff');
  });

  test('rejects compressed JSON and oversized declared bodies before parsing', async () => {
    const app = createResearchApi({ token: TOKEN });
    const headers = { authorization: `Bearer ${TOKEN}`, 'content-type': 'application/json' };
    const compressed = await request(app, '/v1/analysis', { method: 'POST', headers: { ...headers, 'content-encoding': 'gzip' }, body: '{}' });
    expect(compressed.status).toBe(415);
    const oversized = await request(app, '/v1/analysis', { method: 'POST', headers: { ...headers, 'content-length': String(2 * 1024 * 1024 + 1) }, body: '{}' });
    expect(oversized.status).toBe(413);
  });

  test('sliding-window rate limiting prevents a burst across minute boundaries', async () => {
    let wallNow = 1_800_000_000_000;
    let rateNow = 59_000;
    const app = createResearchApi({ token: TOKEN, now: () => wallNow, rateNow: () => rateNow });
    const headers = { authorization: `Bearer ${TOKEN}` };
    for (let i = 0; i < 30; i++) expect((await request(app, '/health', { headers })).status).toBe(200);
    rateNow = 60_000;
    const blocked = await request(app, '/health', { headers });
    expect(blocked.status).toBe(429);
    expect(blocked.headers.get('retry-after')).toBe('59');
    rateNow = 119_000;
    wallNow += 60_000;
    expect((await request(app, '/health', { headers })).status).toBe(200);
  });

  test('enforces process-wide rate limit and bounded body handling', async () => {
    let now = 1_800_000_000_000;
    const app = createResearchApi({ token: TOKEN, now: () => now, rateNow: () => now });
    const headers = { authorization: `Bearer ${TOKEN}`, 'content-type': 'application/json' };
    let finalResponse: Response | undefined;
    for (let i = 0; i < 31; i++) finalResponse = await request(app, '/health', { headers });
    expect(finalResponse?.status).toBe(429);
    expect(finalResponse?.headers.get('retry-after')).toBe('60');
    now += 60_001;
    const reset = await request(app, '/health', { headers });
    expect(reset.status).toBe(200);
  });
  test('rejects a stalled analysis body with a bounded 408 response', { timeout: 15_000 }, async () => {
    const app = createResearchApi({ token: TOKEN });
    const stalled = new ReadableStream<Uint8Array>({ pull() { return new Promise(() => {}); }, cancel() {} });
    const response = await app.request('/v1/analysis', {
      method: 'POST',
      headers: { authorization: `Bearer ${TOKEN}`, 'content-type': 'application/json' },
      body: stalled,
      // Bun's Request implementation supports streaming request bodies.
      duplex: 'half',
    } as RequestInit);
    expect(response.status).toBe(408);
    expect((await response.json()).error.code).toBe('request_timeout');
    expect(response.headers.get('x-request-id')).toMatch(/^[0-9a-f-]{36}$/i);
  });

});
