/** Browser-to-loopback request boundary for the optional local research API. */
const LOOPBACK_HOST = /^(?:localhost|127\.0\.0\.1|\[::1\])(?::([0-9]{1,5}))?$/i;

export interface RequestContextHeaders {
  host?: unknown;
  origin?: unknown;
  secFetchSite?: unknown;
}

export interface RequestContextOptions {
  expectedPort?: number;
}

export type RequestContextDecision =
  | Readonly<{ allowed: true }>
  | Readonly<{ allowed: false; reason: 'invalid_host' | 'browser_origin_not_supported' | 'cross_site_browser_request' }>;

export function isLoopbackHostAuthority(value: unknown, expectedPort?: number): boolean {
  if (typeof value !== 'string' || value.length === 0 || value.trim() !== value) return false;
  const match = LOOPBACK_HOST.exec(value);
  if (!match) return false;
  if (match[1] === undefined) return expectedPort === undefined;
  if (match[1].length > 1 && match[1].startsWith('0')) return false;
  const port = Number(match[1]);
  if (!Number.isInteger(port) || port < 1 || port > 65535) return false;
  return expectedPort === undefined || port === expectedPort;
}

/** Evaluate request metadata before consuming the process-wide limiter. */
export function validateRequestContext(
  headers: RequestContextHeaders,
  options: RequestContextOptions = {},
): RequestContextDecision {
  if (options.expectedPort !== undefined && (!Number.isInteger(options.expectedPort) || options.expectedPort < 1 || options.expectedPort > 65535)) {
    return Object.freeze({ allowed: false, reason: 'invalid_host' });
  }
  if (!isLoopbackHostAuthority(headers?.host, options.expectedPort)) return Object.freeze({ allowed: false, reason: 'invalid_host' });
  if (headers?.origin !== undefined) return Object.freeze({ allowed: false, reason: 'browser_origin_not_supported' });

  const fetchSite = headers?.secFetchSite;
  if (typeof fetchSite === 'string' && fetchSite !== '') {
    const normalized = fetchSite.toLowerCase();
    if (!['same-origin', 'none', 'same-site', 'cross-site'].includes(normalized)) {
      return Object.freeze({ allowed: false, reason: 'cross_site_browser_request' });
    }
    if (normalized === 'cross-site' || normalized === 'same-site') {
      return Object.freeze({ allowed: false, reason: 'cross_site_browser_request' });
    }
  }

  return Object.freeze({ allowed: true });
}
