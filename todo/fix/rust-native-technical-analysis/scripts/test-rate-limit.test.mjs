import test from 'node:test';
import assert from 'node:assert/strict';
import { createSlidingWindowLimiter } from '../api/src/rate-limit.ts';

test('sliding-window limiter rejects boundary bursts across the wall-clock minute boundary', () => {
  let now = 59_000;
  const limiter = createSlidingWindowLimiter({ limit: 3, windowMs: 60_000, now: () => now });
  assert.equal(limiter.consume().allowed, true);
  assert.equal(limiter.consume().allowed, true);
  assert.equal(limiter.consume().allowed, true);
  now = 60_000;
  assert.deepEqual(limiter.consume(), { allowed: false, retryAfterSeconds: 59 });
  now = 119_000;
  assert.deepEqual(limiter.consume(), { allowed: true, retryAfterSeconds: 0 });
});

test('sliding-window limiter expires events exactly at the window boundary', () => {
  let now = 10;
  const limiter = createSlidingWindowLimiter({ limit: 1, windowMs: 1000, now: () => now });
  assert.equal(limiter.consume().allowed, true);
  now = 1009;
  assert.equal(limiter.consume().allowed, false);
  now = 1010;
  assert.equal(limiter.consume().allowed, true);
});

test('clock rollback cannot clear the request budget', () => {
  let now = 10_000;
  const limiter = createSlidingWindowLimiter({ limit: 2, windowMs: 1000, now: () => now });
  assert.equal(limiter.consume().allowed, true);
  now = 10_100;
  assert.equal(limiter.consume().allowed, true);
  now = 1;
  assert.deepEqual(limiter.consume(), { allowed: false, retryAfterSeconds: 1 });
  now = 11_000;
  assert.equal(limiter.consume().allowed, true);
});

test('invalid limiter configuration and invalid clocks fail closed', () => {
  assert.throws(() => createSlidingWindowLimiter({ limit: 0, windowMs: 1000 }), /limit/);
  assert.throws(() => createSlidingWindowLimiter({ limit: 1, windowMs: 0 }), /window/);
  const limiter = createSlidingWindowLimiter({ limit: 1, windowMs: 1000, now: () => Number.NaN });
  assert.throws(() => limiter.consume(), /non-finite/);
});

test('rejected requests do not expand limiter storage under sustained pressure', () => {
  let now = 5;
  const limiter = createSlidingWindowLimiter({ limit: 2, windowMs: 100_000, now: () => now });
  assert.equal(limiter.consume().allowed, true);
  now++;
  assert.equal(limiter.consume().allowed, true);
  for (let i = 0; i < 50_000; i++) assert.equal(limiter.consume().allowed, false);
  now = 100_005;
  assert.equal(limiter.consume().allowed, true);
});
