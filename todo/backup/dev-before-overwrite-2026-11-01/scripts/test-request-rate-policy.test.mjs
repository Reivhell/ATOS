import test from 'node:test';
import assert from 'node:assert/strict';
import { createResearchApiRateLimits, DEFAULT_RESEARCH_RATE_POLICY } from '../api/src/request-rate-policy.ts';

 test('failed authentication consumes only its own fixed-memory budget', () => {
  let now = 1_000;
  const limits = createResearchApiRateLimits({ now: () => now });
  assert.deepEqual(DEFAULT_RESEARCH_RATE_POLICY, {
    authenticatedRequests: 30,
    failedAuthenticationAttempts: 10,
    windowMs: 60_000,
  });

  for (let i = 0; i < 10; i++) assert.equal(limits.consumeFailedAuthentication().allowed, true);
  const blockedFailure = limits.consumeFailedAuthentication();
  assert.equal(blockedFailure.allowed, false);
  assert.equal(blockedFailure.retryAfterSeconds, 60);

  // Even a sustained invalid-token burst leaves the independent valid-client budget empty.
  for (let i = 0; i < 100; i++) assert.equal(limits.consumeFailedAuthentication().allowed, false);
  assert.equal(limits.consumeAuthenticatedRequest().allowed, true);
});

test('authenticated traffic is capped independently of authentication failures', () => {
  const limits = createResearchApiRateLimits({ now: () => 5 });
  for (let i = 0; i < 30; i++) assert.equal(limits.consumeAuthenticatedRequest().allowed, true);
  const blocked = limits.consumeAuthenticatedRequest();
  assert.equal(blocked.allowed, false);
  assert.equal(blocked.retryAfterSeconds, 60);
  assert.equal(limits.consumeFailedAuthentication().allowed, true);
});

test('both budgets expire at the same exact sliding-window boundary', () => {
  let now = 10;
  const limits = createResearchApiRateLimits({ authenticatedLimit: 1, failedAuthenticationLimit: 1, windowMs: 100, now: () => now });
  assert.equal(limits.consumeAuthenticatedRequest().allowed, true);
  assert.equal(limits.consumeFailedAuthentication().allowed, true);
  now = 109;
  assert.equal(limits.consumeAuthenticatedRequest().allowed, false);
  assert.equal(limits.consumeFailedAuthentication().allowed, false);
  now = 110;
  assert.equal(limits.consumeAuthenticatedRequest().allowed, true);
  assert.equal(limits.consumeFailedAuthentication().allowed, true);
});

test('invalid rate policy configuration fails closed', () => {
  assert.throws(() => createResearchApiRateLimits({ authenticatedLimit: 0 }), /limit/);
  assert.throws(() => createResearchApiRateLimits({ failedAuthenticationLimit: 10_001 }), /limit/);
  assert.throws(() => createResearchApiRateLimits({ windowMs: 0 }), /window/);
  assert.throws(() => createResearchApiRateLimits({ now: 1 }), /Rate clock/);
});
