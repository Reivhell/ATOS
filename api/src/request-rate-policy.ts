import { createSlidingWindowLimiter, type RateLimitDecision } from './rate-limit.ts';

export const DEFAULT_RESEARCH_RATE_POLICY = Object.freeze({
  authenticatedRequests: 30,
  failedAuthenticationAttempts: 10,
  windowMs: 60_000,
});

export interface ResearchApiRateLimitOptions {
  authenticatedLimit?: number;
  failedAuthenticationLimit?: number;
  windowMs?: number;
  now?: () => number;
}

export interface ResearchApiRateLimits {
  consumeAuthenticatedRequest(): RateLimitDecision;
  consumeFailedAuthentication(): RateLimitDecision;
}

/** Keep valid-client throughput separate from failed-authentication pressure. */
export function createResearchApiRateLimits(options: ResearchApiRateLimitOptions = {}): ResearchApiRateLimits {
  const authenticatedLimit = options.authenticatedLimit ?? DEFAULT_RESEARCH_RATE_POLICY.authenticatedRequests;
  const failedAuthenticationLimit = options.failedAuthenticationLimit ?? DEFAULT_RESEARCH_RATE_POLICY.failedAuthenticationAttempts;
  const windowMs = options.windowMs ?? DEFAULT_RESEARCH_RATE_POLICY.windowMs;
  const now = options.now ?? (() => performance.now());
  if (typeof now !== 'function') throw new TypeError('Rate clock must be a function.');

  const authenticated = createSlidingWindowLimiter({ limit: authenticatedLimit, windowMs, now });
  const failedAuthentication = createSlidingWindowLimiter({ limit: failedAuthenticationLimit, windowMs, now });

  return Object.freeze({
    consumeAuthenticatedRequest: () => authenticated.consume(),
    consumeFailedAuthentication: () => failedAuthentication.consume(),
  });
}
