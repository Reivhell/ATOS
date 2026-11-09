/** Safe, bounded operational telemetry for the loopback research API. */
export const REQUEST_OBSERVABILITY_SCHEMA = 'local-research-api.request.v1';

const ALLOWED_METHODS = new Set(['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']);
const ALLOWED_OUTCOMES = new Set([
  'success',
  'request_context_rejected',
  'rate_limited',
  'unauthorized',
  'unsupported_media_type',
  'payload_too_large',
  'request_timeout',
  'invalid_request',
  'not_found',
  'server_error',
]);
const UUID_PATTERN = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export type RequestOutcome =
  | 'success'
  | 'request_context_rejected'
  | 'rate_limited'
  | 'unauthorized'
  | 'unsupported_media_type'
  | 'payload_too_large'
  | 'request_timeout'
  | 'invalid_request'
  | 'not_found'
  | 'server_error';

export interface RequestObservationInput extends Record<string, unknown> {
  requestId?: unknown;
  method?: unknown;
  pathname?: unknown;
  statusCode?: unknown;
  durationMs?: unknown;
  outcome?: unknown;
}

export interface RequestObservation {
  readonly schema: typeof REQUEST_OBSERVABILITY_SCHEMA;
  readonly event: 'http.request.completed';
  readonly requestId: string;
  readonly method: string;
  readonly route: '/health' | '/v1/analysis' | '/other';
  readonly statusCode: number;
  readonly durationMs: number;
  readonly outcome: RequestOutcome;
  readonly suppressedEventsSincePreviousEmission?: number;
}

export interface ObserverOptions {
  maxEvents?: number;
  windowMs?: number;
  now?: () => number;
}

export function buildRequestObservation(input: RequestObservationInput): RequestObservation {
  const statusCode = typeof input?.statusCode === 'number' && Number.isInteger(input.statusCode) && input.statusCode >= 100 && input.statusCode <= 599
    ? input.statusCode
    : 500;
  const duration = Number(input?.durationMs);
  const durationMs = Number.isFinite(duration) ? Math.max(0, Math.min(300_000, Math.round(duration))) : 0;
  const pathname = input?.pathname;
  const route = pathname === '/health' ? '/health' : pathname === '/v1/analysis' ? '/v1/analysis' : '/other';
  const method = typeof input?.method === 'string' && ALLOWED_METHODS.has(input.method.toUpperCase())
    ? input.method.toUpperCase()
    : 'OTHER';
  const outcome = typeof input?.outcome === 'string' && ALLOWED_OUTCOMES.has(input.outcome)
    ? input.outcome as RequestOutcome
    : 'server_error';

  return Object.freeze({
    schema: REQUEST_OBSERVABILITY_SCHEMA,
    event: 'http.request.completed',
    requestId: typeof input?.requestId === 'string' && UUID_PATTERN.test(input.requestId)
      ? input.requestId
      : 'invalid-request-id',
    method,
    route,
    statusCode,
    durationMs,
    outcome,
  });
}

/** Map only known internal error codes to finite, non-sensitive outcome labels. */
export function classifyRequestOutcome(code: string, statusCode: number): RequestOutcome {
  switch (code) {
    case 'request_context_rejected': return 'request_context_rejected';
    case 'rate_limited': return 'rate_limited';
    case 'unauthorized': return 'unauthorized';
    case 'unsupported_media_type': return 'unsupported_media_type';
    case 'body_too_large': return 'payload_too_large';
    case 'request_timeout': return 'request_timeout';
    case 'not_found': return 'not_found';
    case 'internal_error':
    case 'analysis_failed': return 'server_error';
    default:
      if (statusCode === 429) return 'rate_limited';
      if (statusCode >= 500) return 'server_error';
      if (statusCode === 404) return 'not_found';
      if (statusCode >= 400) return 'invalid_request';
      return 'success';
  }
}

/** Bound log output independently of request rate; sink failures never affect requests. */
export function createBoundedRequestObserver(
  write: (event: RequestObservation) => void,
  options: ObserverOptions = {},
): (event: RequestObservation) => void {
  if (typeof write !== 'function') throw new TypeError('A request-observation writer is required.');
  const maxEvents = options.maxEvents ?? 120;
  const windowMs = options.windowMs ?? 60_000;
  const now = options.now ?? (() => performance.now());
  if (!Number.isInteger(maxEvents) || maxEvents < 1 || maxEvents > 10_000) {
    throw new RangeError('maxEvents must be an integer between 1 and 10000.');
  }
  if (!Number.isFinite(windowMs) || windowMs < 1 || windowMs > 3_600_000) {
    throw new RangeError('windowMs must be between 1 and 3600000 milliseconds.');
  }
  let windowStart = now();
  let emittedInWindow = 0;
  let suppressedSinceEmission = 0;

  return (event: RequestObservation) => {
    const current = now();
    if (!Number.isFinite(current)) return;
    if (current < windowStart || current - windowStart >= windowMs) {
      windowStart = current;
      emittedInWindow = 0;
    }
    if (emittedInWindow >= maxEvents) {
      suppressedSinceEmission += 1;
      return;
    }
    const output: RequestObservation = suppressedSinceEmission > 0
      ? Object.freeze({ ...event, suppressedEventsSincePreviousEmission: suppressedSinceEmission })
      : event;
    try {
      write(output);
      emittedInWindow += 1;
      suppressedSinceEmission = 0;
    } catch {
      // Best-effort telemetry must never alter request processing.
    }
  };
}
