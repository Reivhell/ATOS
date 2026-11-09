/** Fixed-memory sliding-window limiter for a single-process local service. */
export interface SlidingWindowLimiterOptions {
  limit: number;
  windowMs: number;
  now?: () => number;
}

export interface RateLimitDecision {
  readonly allowed: boolean;
  readonly retryAfterSeconds: number;
}

export interface SlidingWindowLimiter {
  consume(): RateLimitDecision;
}

export function createSlidingWindowLimiter({
  limit,
  windowMs,
  now = () => performance.now(),
}: SlidingWindowLimiterOptions): SlidingWindowLimiter {
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > 10_000) {
    throw new TypeError('Rate limit must be an integer between 1 and 10000.');
  }
  if (!Number.isFinite(windowMs) || windowMs <= 0) {
    throw new TypeError('Rate window must be a positive finite number.');
  }
  if (typeof now !== 'function') throw new TypeError('Rate clock must be a function.');

  const timestamps = new Float64Array(limit);
  let head = 0;
  let size = 0;
  let lastNow = Number.NEGATIVE_INFINITY;

  return Object.freeze({
    consume(): RateLimitDecision {
      const observed = now();
      if (!Number.isFinite(observed)) throw new TypeError('Rate clock returned a non-finite value.');
      const current = Math.max(observed, lastNow);
      lastNow = current;

      while (size > 0 && timestamps[head] <= current - windowMs) {
        head = (head + 1) % limit;
        size--;
      }

      if (size >= limit) {
        const retryAfterMs = Math.max(0, timestamps[head] + windowMs - current);
        return Object.freeze({
          allowed: false,
          retryAfterSeconds: Math.max(1, Math.ceil(retryAfterMs / 1000)),
        });
      }

      timestamps[(head + size) % limit] = current;
      size++;
      return Object.freeze({ allowed: true, retryAfterSeconds: 0 });
    },
  });
}
