import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createReleaseEvidence, verifyReleaseEvidence, RELEASE_EVIDENCE_SCHEMA } from './release-evidence.mjs';

const COMMIT = 'a'.repeat(40);

async function makeProject(t, { version = '0.5.6' } = {}) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'atos-release-evidence-'));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  await fs.mkdir(path.join(root, 'src-tauri'), { recursive: true });
  await fs.writeFile(path.join(root, 'package.json'), JSON.stringify({ name: 'agentic-trading-os', version }));
  await fs.writeFile(path.join(root, 'src-tauri', 'tauri.conf.json'), JSON.stringify({ version }));
  await fs.writeFile(path.join(root, 'src-tauri', 'Cargo.toml'), `[package]\nname = "agentic-trading-os"\nversion = "${version}"\n`);
  const bundle = path.join(root, 'bundle');
  const evidence = path.join(root, 'evidence');
  await fs.mkdir(path.join(bundle, 'nested'), { recursive: true });
  await fs.writeFile(path.join(bundle, 'QuantDinger.AppImage'), Buffer.from([0, 1, 2, 3, 255]));
  await fs.writeFile(path.join(bundle, 'nested', 'manifest.txt'), 'packaged-file\n');
  const options = {
    projectRoot: root,
    bundleDir: bundle,
    evidenceDir: evidence,
    generatedAt: '2026-10-28T00:00:00.000Z',
    gitMetadata: { revision: COMMIT, workingTree: 'clean' },
    expectedSourceRevision: COMMIT,
    target: { runnerOS: 'test-os', platform: 'test-platform', architecture: 'test-arch' },
    toolchain: { node: 'test-node', npm: null, rustc: null, cargo: null },
    ci: { provider: 'test', runId: null, runAttempt: null, ref: null },
  };
  return { root, bundle, evidence, options };
}

test('release evidence hashes a sorted full artifact inventory and verifies cleanly', async (t) => {
  const fixture = await makeProject(t);
  const created = await createReleaseEvidence(fixture.options);
  assert.equal(created.manifest.schemaVersion, RELEASE_EVIDENCE_SCHEMA);
  assert.equal(created.manifest.version, '0.5.6');
  assert.equal(created.manifest.source.revision, COMMIT);
  assert.equal(created.manifest.artifactCount, 2);
  assert.deepEqual(created.manifest.artifacts.map((item) => item.path), ['QuantDinger.AppImage', 'nested/manifest.txt']);
  const checksums = JSON.parse(await fs.readFile(path.join(fixture.evidence, 'checksums.json'), 'utf8'));
  const expected = crypto.createHash('sha256').update(Buffer.from([0, 1, 2, 3, 255])).digest('hex');
  assert.equal(checksums.hashes['bundle/QuantDinger.AppImage'], expected);
  const verified = await verifyReleaseEvidence(fixture.options);
  assert.equal(verified.verifiedArtifactCount, 2);
});

test('release evidence rejects a source revision that differs from the expected CI commit', async (t) => {
  const fixture = await makeProject(t);
  await createReleaseEvidence(fixture.options);
  await assert.rejects(
    verifyReleaseEvidence({ ...fixture.options, expectedSourceRevision: 'b'.repeat(40) }),
    /does not match the expected source commit/,
  );
});

test('release evidence rejects changed package bytes after manifest generation', async (t) => {
  const fixture = await makeProject(t);
  await createReleaseEvidence(fixture.options);
  await fs.writeFile(path.join(fixture.bundle, 'QuantDinger.AppImage'), 'tampered');
  await assert.rejects(verifyReleaseEvidence(fixture.options), /Artifact hashes, file sizes, kinds, or symlink targets/);
});

test('release evidence rejects added or removed artifacts, even when existing files still hash correctly', async (t) => {
  const fixture = await makeProject(t);
  await createReleaseEvidence(fixture.options);
  await fs.writeFile(path.join(fixture.bundle, 'unexpected.bin'), 'extra');
  await assert.rejects(verifyReleaseEvidence(fixture.options), /Artifact inventory drift/);
});

test('release evidence rejects a tampered checksum map and changed manifest bytes', async (t) => {
  const fixture = await makeProject(t);
  await createReleaseEvidence(fixture.options);
  const checksumPath = path.join(fixture.evidence, 'checksums.json');
  const checksums = JSON.parse(await fs.readFile(checksumPath, 'utf8'));
  checksums.hashes['bundle/QuantDinger.AppImage'] = '0'.repeat(64);
  await fs.writeFile(checksumPath, JSON.stringify(checksums, null, 2) + '\n');
  await assert.rejects(verifyReleaseEvidence(fixture.options), /checksums.json does not exactly cover/);

  await createReleaseEvidence(fixture.options);
  await fs.appendFile(path.join(fixture.evidence, 'release-manifest.json'), ' ');
  await assert.rejects(verifyReleaseEvidence(fixture.options), /checksums.json does not exactly cover/);
});

test('release evidence rejects empty bundles, misaligned versions and unsafe nested evidence paths', async (t) => {
  const fixture = await makeProject(t);
  await assert.rejects(createReleaseEvidence({ ...fixture.options, evidenceDir: path.join(fixture.bundle, 'evidence') }), /outside the bundle/);
  await fs.rm(fixture.bundle, { recursive: true, force: true });
  await fs.mkdir(fixture.bundle);
  await assert.rejects(createReleaseEvidence(fixture.options), /contains no files/);

  const mismatched = await makeProject(t, { version: '9.9.9' });
  await fs.writeFile(path.join(mismatched.root, 'src-tauri', 'tauri.conf.json'), JSON.stringify({ version: '0.5.6' }));
  await assert.rejects(createReleaseEvidence(mismatched.options), /not version-aligned/);
});

test('release evidence records internal symlink targets without following them', async (t) => {
  const fixture = await makeProject(t);
  try {
    await fs.symlink('QuantDinger.AppImage', path.join(fixture.bundle, 'app-link'));
  } catch (error) {
    if (['EPERM', 'EACCES', 'ENOTSUP'].includes(error.code)) return t.skip('Symlink creation is unavailable on this runner.');
    throw error;
  }
  const created = await createReleaseEvidence(fixture.options);
  const link = created.manifest.artifacts.find((item) => item.path === 'app-link');
  assert.equal(link.kind, 'symlink');
  assert.equal(link.linkTarget, 'QuantDinger.AppImage');
  assert.equal((await verifyReleaseEvidence(fixture.options)).verifiedArtifactCount, 3);
});

test('release evidence refuses symlinks that escape the packaged artifact root', async (t) => {
  const fixture = await makeProject(t);
  const outside = path.join(fixture.root, 'outside-payload.txt');
  await fs.writeFile(outside, 'not part of the package');
  try {
    await fs.symlink(outside, path.join(fixture.bundle, 'external-link'));
  } catch (error) {
    if (['EPERM', 'EACCES', 'ENOTSUP'].includes(error.code)) return t.skip('Symlink creation is unavailable on this runner.');
    throw error;
  }
  await assert.rejects(createReleaseEvidence(fixture.options), /Symbolic link escapes the release bundle/);
});
