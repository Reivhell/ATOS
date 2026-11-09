import { getCandleIntervalMs } from '../../src/lib/domain/market-data/timeframe-contract.js';
import type { CandleInterval } from '../../src/lib/types.ts';

export const LOCAL_RESEARCH_API_CONTRACT = 'local-research-api.v1';
export const MAX_API_CANDLES = 2_000;
export const MAX_API_BODY_BYTES = 2 * 1024 * 1024;
const FINGERPRINT_DOMAIN = new TextEncoder().encode('agentic-trading-os.ohlcv.canonical.v1\0');

export interface ApiCandle {
  readonly time: number;
  readonly open: number;
  readonly high: number;
  readonly low: number;
  readonly close: number;
  readonly volume: number;
}

export interface NormalizedAnalysisRequest {
  readonly symbol: string;
  readonly timeframe: CandleInterval;
  readonly candles: readonly ApiCandle[];
}

export type BoundedBodyResult =
  | Readonly<{ kind: 'ok'; text: string }>
  | Readonly<{ kind: 'too_large' }>
  | Readonly<{ kind: 'invalid_utf8' }>
  | Readonly<{ kind: 'read_error' }>
  | Readonly<{ kind: 'timed_out' }>;

export interface BoundedBodyOptions {
  timeoutMs?: number;
}

/** Cross-runtime canonical SHA-256 identity; this is not provider authentication. */
export async function canonicalDatasetSha256(candles: readonly ApiCandle[]): Promise<string> {
  if (!Array.isArray(candles) || candles.length > MAX_API_CANDLES) throw new TypeError('Invalid canonical candle series.');
  const bytes = new Uint8Array(FINGERPRINT_DOMAIN.byteLength + 8 + candles.length * 48);
  bytes.set(FINGERPRINT_DOMAIN, 0);
  const view = new DataView(bytes.buffer);
  let offset = FINGERPRINT_DOMAIN.byteLength;
  view.setBigUint64(offset, BigInt(candles.length), true);
  offset += 8;
  for (const candle of candles) {
    if (!candle || !Number.isSafeInteger(candle.time) || ![candle.open, candle.high, candle.low, candle.close, candle.volume].every(Number.isFinite)) {
      throw new TypeError('Invalid candle in canonical fingerprint input.');
    }
    view.setBigInt64(offset, BigInt(candle.time), true);
    offset += 8;
    for (const key of ['open', 'high', 'low', 'close', 'volume'] as const) {
      view.setFloat64(offset, candle[key], true);
      offset += 8;
    }
  }
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** Bounded UTF-8 request-body reader; never buffers more than the configured maximum. */
export async function readBoundedBody(request: Request, options: BoundedBodyOptions = {}): Promise<BoundedBodyResult> {
  const timeoutMs = options.timeoutMs ?? 10_000;
  if (!Number.isFinite(timeoutMs) || timeoutMs < 1 || timeoutMs > 120_000) throw new RangeError('timeoutMs must be between 1 and 120000 milliseconds.');
  const body = request.body;
  if (!body) return { kind: 'ok', text: '' };
  const reader = body.getReader();
  let bytes = new Uint8Array(Math.min(16 * 1024, MAX_API_BODY_BYTES));
  let total = 0;
  let timeoutHandle: ReturnType<typeof setTimeout> | undefined;
  let releaseReaderLock = true;
  const timeoutMarker = Symbol('body-read-timeout');
  const timeout = new Promise<typeof timeoutMarker>((resolve) => {
    timeoutHandle = setTimeout(() => resolve(timeoutMarker), timeoutMs);
  });
  try {
    while (true) {
      const result = await Promise.race([reader.read(), timeout]);
      if (result === timeoutMarker) {
        // Cleanup is best-effort; a stuck underlying reader cannot hold request completion.
        releaseReaderLock = false;
        void reader.cancel().catch(() => undefined);
        return { kind: 'timed_out' };
      }
      const { done, value } = result;
      if (done) break;
      if (value.byteLength > MAX_API_BODY_BYTES - total) {
        void reader.cancel().catch(() => undefined);
        return { kind: 'too_large' };
      }
      const required = total + value.byteLength;
      if (required > bytes.byteLength) {
        let capacity = bytes.byteLength;
        while (capacity < required) capacity = Math.min(MAX_API_BODY_BYTES, capacity * 2);
        const grown = new Uint8Array(capacity);
        grown.set(bytes.subarray(0, total));
        bytes = grown;
      }
      bytes.set(value, total);
      total = required;
    }
    try {
      return { kind: 'ok', text: new TextDecoder('utf-8', { fatal: true }).decode(bytes.subarray(0, total)) };
    } catch {
      return { kind: 'invalid_utf8' };
    }
  } catch {
    void reader.cancel().catch(() => undefined);
    return { kind: 'read_error' };
  } finally {
    if (timeoutHandle !== undefined) clearTimeout(timeoutHandle);
    if (releaseReaderLock) reader.releaseLock();
  }
}

/** Validate untrusted API input without treating it as provider-authenticated data. */
export function parseAnalysisRequest(value: unknown, now = Date.now()): NormalizedAnalysisRequest | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const input = value as Record<string, unknown>;
  const symbol = typeof input.symbol === 'string' ? input.symbol.trim().toUpperCase() : '';
  if (!/^[A-Z0-9_-]{1,30}$/.test(symbol)) return null;
  if (typeof input.timeframe !== 'string') return null;
  const step = getCandleIntervalMs(input.timeframe);
  if (step === null) return null;
  if (!Array.isArray(input.candles) || input.candles.length < 34 || input.candles.length > MAX_API_CANDLES) return null;
  let previous = 0;
  const candles: ApiCandle[] = [];
  for (const candidate of input.candles as unknown[]) {
    if (!candidate || typeof candidate !== 'object' || Array.isArray(candidate)) return null;
    const row = candidate as Record<string, unknown>;
    const { time, open, high, low, close, volume } = row;
    if (typeof time !== 'number' || !Number.isSafeInteger(time) || time <= 0 || time % step !== 0 || time <= previous || time > now + 30_000) return null;
    if (![open, high, low, close, volume].every((n) => typeof n === 'number' && Number.isFinite(n))) return null;
    if ((open as number) <= 0 || (high as number) <= 0 || (low as number) <= 0 || (close as number) <= 0 || (volume as number) < 0) return null;
    if ((high as number) < Math.max(open as number, close as number, low as number) || (low as number) > Math.min(open as number, close as number, high as number)) return null;
    if (previous && time - previous !== step) return null;
    previous = time;
    candles.push(Object.freeze({
      time,
      open: open as number,
      high: high as number,
      low: low as number,
      close: close as number,
      volume: volume as number,
    }));
  }
  return Object.freeze({ symbol, timeframe: input.timeframe as CandleInterval, candles: Object.freeze(candles) });
}
