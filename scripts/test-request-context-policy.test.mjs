import test from 'node:test';
import assert from 'node:assert/strict';
import { isLoopbackHostAuthority, validateRequestContext } from '../api/src/request-origin-policy.ts';

test('loopback Host authority accepts only canonical loopback hosts and valid explicit ports', () => {
  for (const host of ['localhost', 'LOCALHOST:8787', '127.0.0.1', '127.0.0.1:8787', '[::1]', '[::1]:8787']) {
    assert.equal(isLoopbackHostAuthority(host), true, host);
  }
  for (const host of [undefined, null, '', ' localhost', 'localhost ', 'localhost.', 'evil.example', '127.0.0.1.evil.example', '127.1', '0x7f000001', '127.0.0.1:0', '127.0.0.1:65536', '127.0.0.1:08787', '[::1]:abc', '::1:8787', 'localhost@evil.example']) {
    assert.equal(isLoopbackHostAuthority(host), false, String(host));
  }
  assert.equal(isLoopbackHostAuthority('127.0.0.1:8787', 8787), true);
  assert.equal(isLoopbackHostAuthority('127.0.0.1:8788', 8787), false);
  assert.equal(isLoopbackHostAuthority('127.0.0.1', 8787), false);
});

test('all explicit browser Origins are rejected while this API remains intentionally no-CORS', () => {
  for (const origin of ['https://evil.example', 'null', 'http://127.0.0.1:1420', 'tauri://localhost', '']) {
    assert.deepEqual(validateRequestContext({ host: '127.0.0.1:8787', origin }), { allowed: false, reason: 'browser_origin_not_supported' });
  }
});

test('cross-site and same-site browser requests without Origin are rejected', () => {
  assert.deepEqual(validateRequestContext({ host: 'localhost:8787', secFetchSite: 'cross-site' }), { allowed: false, reason: 'cross_site_browser_request' });
  assert.deepEqual(validateRequestContext({ host: 'localhost:8787', secFetchSite: 'same-site' }), { allowed: false, reason: 'cross_site_browser_request' });
});

test('same-origin and user-initiated browser metadata do not replace bearer authentication', () => {
  assert.deepEqual(validateRequestContext({ host: '127.0.0.1:8787', secFetchSite: 'same-origin' }), { allowed: true });
  assert.deepEqual(validateRequestContext({ host: '127.0.0.1:8787', secFetchSite: 'none' }), { allowed: true });
});

test('non-browser clients may omit Origin and Fetch Metadata but cannot spoof Host', () => {
  assert.deepEqual(validateRequestContext({ host: '127.0.0.1:8787' }), { allowed: true });
  assert.deepEqual(validateRequestContext({ host: 'api.attacker.example' }), { allowed: false, reason: 'invalid_host' });
});

test('unknown Fetch Metadata values fail closed', () => {
  assert.deepEqual(validateRequestContext({ host: '127.0.0.1:8787', secFetchSite: 'cross-origin-ish' }), { allowed: false, reason: 'cross_site_browser_request' });
});

test('invalid Host is rejected before other request metadata is considered', () => {
  assert.deepEqual(validateRequestContext({ host: 'rebinding.attacker.example', origin: 'https://evil.example' }), { allowed: false, reason: 'invalid_host' });
});

test('Fetch Metadata value comparison is case-insensitive', () => {
  assert.deepEqual(validateRequestContext({ host: 'localhost:8787', secFetchSite: 'CROSS-SITE' }), { allowed: false, reason: 'cross_site_browser_request' });
});
