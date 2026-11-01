import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const read = (file) => fs.readFileSync(path.join(ROOT, file), 'utf8');

test('release readiness validates each invariant in the source module that owns it', () => {
  const workflow = read('.github/workflows/ci.yml');
  const readiness = read('scripts/check-release-readiness.mjs');
  const verifier = read('scripts/verify-release-attestation.mjs');
  const versionPolicy = read('scripts/release-version-policy.mjs');

  assert.match(workflow, /bun run scripts\/verify-release-attestation\.mjs/);
  assert.match(readiness, /read\('scripts\/verify-release-attestation\.mjs'\)/);
  assert.match(verifier, /'--deny-self-hosted-runners'/);
  assert.match(verifier, /https:\/\/slsa\.dev\/provenance\/v1/);
  assert.match(verifier, /https:\/\/spdx\.dev\/Document\/v2\.3/);
  assert.match(readiness, /findUnpinnedActions/);
  assert.match(readiness, /reviewed full 40-character commit SHAs/);
  assert.match(readiness, /releaseTagVersionError/);
  assert.match(readiness, /\['bun', \['--version'\]\]/);
  assert.match(versionPolicy, /Release tag v\$\{tagVersion\} does not match application manifest version/);
});

test('release preflight does not misreport source-owned policy tokens as missing workflow contracts', () => {
  const result = spawnSync('bun', ['run', 'scripts/check-release-readiness.mjs'], {
    cwd: ROOT,
    encoding: 'utf8',
    timeout: 45_000,
    windowsHide: true,
    env: { ...process.env, GITHUB_REF: 'refs/heads/main' },
  });
  assert.equal(result.error, undefined, result.error?.message);
  const output = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  assert.match(output, /Release preflight (?:BLOCKED|prerequisites passed)/);
  assert.doesNotMatch(output, /CI does not enforce the release contract/);
  assert.match(output, /all external GitHub Actions use reviewed full commit SHAs/);
  assert.doesNotMatch(output, /mutable refs remain/);

  assert.doesNotMatch(output, /Attestation verifier must reject self-hosted runners/);
  assert.doesNotMatch(output, /Attestation verifier must require both release evidence predicates/);
  assert.doesNotMatch(output, /Release tag-to-manifest version policy is not wired/);
  assert.doesNotMatch(output, /explicit manifest-aligned tag contract/);
});
