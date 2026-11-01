import { Hono } from 'hono';
import { createResearchApiRateLimits } from './request-rate-policy.ts';
import { validateRequestContext } from './request-origin-policy.ts';
import { buildRequestObservation, classifyRequestOutcome } from './request-observability.ts';
import { buildTechnicalAnalysisSnapshot } from '../../src/lib/engines/technical-analysis/analysis-engine.js';
import { getLatestClosedBarAtMs } from '../../src/lib/domain/market-data/timeframe-contract.js';
import { LOCAL_RESEARCH_API_CONTRACT, MAX_API_BODY_BYTES, canonicalDatasetSha256, parseAnalysisRequest, readBoundedBody } from './validation.ts';

const RATE_WINDOW_MS = 60_000;
const RATE_MAX_REQUESTS = 30;
const FAILED_AUTH_MAX_REQUESTS = 10;

function jsonError(code: string, message: string, status: 400 | 401 | 403 | 404 | 408 | 413 | 415 | 429 | 500) {
  return Response.json({ error: { code, message }, contractVersion: LOCAL_RESEARCH_API_CONTRACT }, { status, headers: securityHeaders() });
}

function securityHeaders(): HeadersInit {
  return {
    'Cache-Control': 'no-store, max-age=0',
    'Pragma': 'no-cache',
    'X-Content-Type-Options': 'nosniff',
    'X-Frame-Options': 'DENY',
    'Referrer-Policy': 'no-referrer',
  };
}

export function createResearchApi(options: { token: string; now?: () => number; rateNow?: () => number; observationNow?: () => number; observe?: (event: ReturnType<typeof buildRequestObservation>) => void; expectedPort?: number } ) {
  if (!options || typeof options.token !== 'string' || options.token.length < 32) {
    throw new Error('A LOCAL_RESEARCH_API_TOKEN of at least 32 characters is required.');
  }
  if (options.expectedPort !== undefined && (!Number.isInteger(options.expectedPort) || options.expectedPort < 1024 || options.expectedPort > 65535)) {
    throw new Error('expectedPort must be an integer between 1024 and 65535.');
  }
  const now = options.now ?? Date.now;
  const observationNow = options.observationNow ?? (() => performance.now());
  // Observability clocks are not allowed to become a request-availability dependency.
  const safeObservationNow = () => {
    try {
      const value = observationNow();
      return Number.isFinite(value) ? value : 0;
    } catch {
      return 0;
    }
  };
  // Process-wide and fixed-memory. Separate failed-auth pressure from legitimate clients:
  // bad credentials never consume the authenticated request budget.
  // Use a monotonic clock so wall-clock changes cannot reset either budget.
  const rateLimits = createResearchApiRateLimits({
    authenticatedLimit: RATE_MAX_REQUESTS,
    failedAuthenticationLimit: FAILED_AUTH_MAX_REQUESTS,
    windowMs: RATE_WINDOW_MS,
    now: options.rateNow ?? (options.now ? options.now : () => performance.now()),
  });
  const app = new Hono();

  app.use('*', async (c, next) => {
    const requestId = crypto.randomUUID();
    const startedAt = safeObservationNow();
    let statusCode = 500;
    let outcome = 'server_error';
    c.header('X-Request-ID', requestId);

    const reject = (code: string, message: string, status: 400 | 401 | 403 | 404 | 408 | 413 | 415 | 429 | 500) => {
      statusCode = status;
      outcome = classifyRequestOutcome(code, status);
      const response = jsonError(code, message, status);
      response.headers.set('X-Request-ID', requestId);
      return response;
    };

    try {
      // Reject hostile Host/origin metadata before the shared rate budget is consumed.
      // Cross-site browser probes must not exhaust legitimate local clients' quota.
      const context = validateRequestContext({
        host: c.req.header('host') ?? new URL(c.req.url).host,
        origin: c.req.header('origin'),
        secFetchSite: c.req.header('sec-fetch-site'),
      }, { expectedPort: options.expectedPort });
      if (!context.allowed) {
        return reject('request_context_rejected', 'Request host or browser origin is not allowed.', 403);
      }

      // Compare every byte of the expected value rather than exiting on the first mismatch.
      // The work loop is fixed to the configured secret length; this is not a cryptographic
      // primitive and does not eliminate runtime-level timing variation.
      const supplied = c.req.header('authorization') ?? '';
      const expected = `Bearer ${options.token}`;
      const encoder = new TextEncoder();
      const a = encoder.encode(supplied);
      const b = encoder.encode(expected);
      let diff = a.length ^ b.length;
      for (let i = 0; i < b.length; i++) diff |= (a[i] ?? 0) ^ b[i];
      if (diff !== 0) {
        const failedAuthBudget = rateLimits.consumeFailedAuthentication();
        if (!failedAuthBudget.allowed) {
          const response = reject('rate_limited', 'Too many failed authentication attempts; retry after the oldest attempt expires.', 429);
          response.headers.set('Retry-After', String(failedAuthBudget.retryAfterSeconds));
          return response;
        }
        return reject('unauthorized', 'A valid bearer token is required.', 401);
      }

      // Only authenticated clients consume the shared operational request budget.
      const budget = rateLimits.consumeAuthenticatedRequest();
      if (!budget.allowed) {
        const response = reject('rate_limited', 'Rate limit exceeded; retry after the oldest request expires.', 429);
        response.headers.set('Retry-After', String(budget.retryAfterSeconds));
        return response;
      }

      if (c.req.method === 'POST') {
        const contentType = c.req.header('content-type')?.toLowerCase() ?? '';
        if (!/^application\/json(?:\s*;\s*charset\s*=\s*(?:utf-8|utf8|"utf-8"))?\s*$/.test(contentType)) {
          return reject('unsupported_media_type', 'Content-Type must be application/json encoded as UTF-8.', 415);
        }
        const encoding = c.req.header('content-encoding')?.toLowerCase();
        if (encoding && encoding !== 'identity') {
          return reject('unsupported_media_type', 'Compressed request bodies are not supported.', 415);
        }
        const rawLength = c.req.header('content-length');
        if (rawLength !== undefined && (!/^\d+$/.test(rawLength) || Number(rawLength) > MAX_API_BODY_BYTES)) {
          const tooLarge = Number(rawLength) > MAX_API_BODY_BYTES;
          return reject(tooLarge ? 'body_too_large' : 'invalid_content_length', 'Invalid or excessive Content-Length.', tooLarge ? 413 : 400);
        }
      }

      for (const [name, value] of Object.entries(securityHeaders())) c.header(name, value);
      await next();
      statusCode = c.res.status;
      outcome = classifyRequestOutcome(statusCode >= 500 ? 'internal_error' : statusCode === 404 ? 'not_found' : 'response_completed', statusCode);
    } catch (error) {
      // Do not include the exception, its message, or request data in observability output.
      statusCode = 500;
      outcome = 'server_error';
      throw error;
    } finally {
      if (options.observe) {
        try {
          options.observe(buildRequestObservation({
            requestId,
            method: c.req.method,
            pathname: new URL(c.req.url).pathname,
            statusCode,
            durationMs: safeObservationNow() - startedAt,
            outcome,
          }));
        } catch {
          // A custom observer is best-effort and is never part of request correctness.
        }
      }
    }
  });

  app.get('/health', (c) => c.json({ status: 'ok', service: 'local-research-api', contractVersion: LOCAL_RESEARCH_API_CONTRACT, executionAuthority: 'none' }));
  app.post('/v1/analysis', async (c) => {
    const bounded = await readBoundedBody(c.req.raw);
    if (bounded.kind === 'too_large') return jsonError('body_too_large', 'Request body exceeds the 2 MiB limit.', 413);
    if (bounded.kind === 'timed_out') return jsonError('request_timeout', 'Request body was not received within the allowed time.', 408);
    if (bounded.kind === 'invalid_utf8') return jsonError('invalid_encoding', 'Request body must be valid UTF-8.', 400);
    if (bounded.kind === 'read_error') return jsonError('body_read_failed', 'Request body could not be read safely.', 400);
    let payload: unknown;
    try {
      payload = JSON.parse(bounded.text);
    } catch {
      return jsonError('invalid_json', 'Request body must be valid JSON.', 400);
    }
    const request = parseAnalysisRequest(payload, now());
    if (!request) return jsonError('invalid_analysis_request', 'Expected a supported symbol/timeframe and 34–2000 contiguous, UTC-grid-aligned OHLCV candles.', 400);
    try {
      // Fingerprinting enables reproducibility and evidence binding; it does not authenticate
      // who supplied the candles. Provider trust and execution authority remain disabled.
      const datasetSha256 = await canonicalDatasetSha256(request.candles);
      const latestClosedBarAtMs = getLatestClosedBarAtMs(request.candles, request.timeframe);
      if (latestClosedBarAtMs === null) return jsonError('invalid_candle_cutoff', 'Latest candle close timestamp could not be derived from the requested timeframe.', 400);
      const report = buildTechnicalAnalysisSnapshot({
        candles: request.candles,
        symbol: request.symbol,
        timeframe: request.timeframe,
        marketData: { validated: false, source: 'untrusted-local-api-input', asOf: new Date(latestClosedBarAtMs).toISOString(), latestClosedBarAtMs, datasetSha256, symbol: request.symbol, timeframe: request.timeframe },
        intent: 'research',
      });
      return c.json({ contractVersion: LOCAL_RESEARCH_API_CONTRACT, report, trust: { providerAuthenticated: false, datasetFingerprintComputed: true, datasetFingerprintVerified: false, datasetSha256, researchCandidateHandoffAllowed: false, executionAuthority: 'none' } });
    } catch {
      return jsonError('analysis_failed', 'Analysis failed; no decision or execution authority was produced.', 500);
    }
  });
  app.notFound(() => jsonError('not_found', 'Route does not exist.', 404));
  app.onError(() => jsonError('internal_error', 'Request could not be completed safely.', 500));
  return app;
}
