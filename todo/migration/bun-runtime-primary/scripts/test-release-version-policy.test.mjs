import test from 'node:test';
import assert from 'node:assert/strict';
import { releaseTagVersionError } from './release-version-policy.mjs';

test('release tag must match the canonical app version rather than just a semver-looking prefix', () => {
  assert.equal(releaseTagVersionError('refs/tags/v0.9.6', '0.9.6'), null);
  assert.equal(releaseTagVersionError('refs/tags/v0.9.6-rc.1', '0.9.6-rc.1'), null);
  assert.match(releaseTagVersionError('refs/tags/v0.9.7', '0.9.6'), /does not match application manifest version/);
  assert.match(releaseTagVersionError('refs/tags/v0.9.6', '0.9.6+build.2'), /does not match application manifest version/);
});

test('malformed tag-shaped refs fail closed while non-tag CI refs remain valid', () => {
  assert.match(releaseTagVersionError('refs/tags/latest', '0.9.6'), /not a valid version tag/);
  assert.match(releaseTagVersionError('refs/tags/0.9.6', '0.9.6'), /not a valid version tag/);
  assert.equal(releaseTagVersionError('refs/heads/main', '0.9.6'), null);
  assert.equal(releaseTagVersionError(undefined, '0.9.6'), null);
  assert.match(releaseTagVersionError('refs/tags/v0.9.6', ''), /version is unavailable/);
});
