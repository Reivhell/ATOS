import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {
  inventoryAttestationSubjects,
  parseAttestationVerificationArgs,
  verifyReleaseAttestations,
} from './verify-release-attestation.mjs';

const repo = 'acme/agentic-trading-os';
const ref = 'refs/tags/v0.9.6';
const workflow = `${repo}/.github/workflows/ci.yml`;

async function fixture(t) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'atos-attestation-'));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const bundle = path.join(root, 'artifacts');
  await fs.mkdir(bundle);
  return { root, bundle };
}

test('attestation CLI pins repository, release tag and the exact CI signer workflow', () => {
  const parsed = parseAttestationVerificationArgs([
    '--repo', repo,
    '--source-ref', ref,
    '--signer-workflow', workflow,
    '--bundle-dir', './release-staging/artifacts',
  ]);
  assert.equal(parsed.repo, repo);
  assert.equal(parsed.sourceRef, ref);
  assert.equal(parsed.signerWorkflow, workflow);
  assert.ok(path.isAbsolute(parsed.bundleDir));
  assert.throws(() => parseAttestationVerificationArgs(['--repo', repo]), /Missing required option/);
  assert.throws(() => parseAttestationVerificationArgs(['--repo', 'attacker.invalid/acme', '--source-ref', ref, '--signer-workflow', workflow, '--bundle-dir', '.']), /Repository must/);
  assert.throws(() => parseAttestationVerificationArgs(['--repo', repo, '--source-ref', 'refs/heads/main', '--signer-workflow', workflow, '--bundle-dir', '.']), /version tag/);
  assert.throws(() => parseAttestationVerificationArgs(['--repo', repo, '--source-ref', ref, '--signer-workflow', 'acme/other/.github/workflows/ci.yml', '--bundle-dir', '.']), /must be exactly/);
});

test('attestation verification checks every staged installer against strict GitHub provenance constraints', async (t) => {
  const f = await fixture(t);
  await fs.writeFile(path.join(f.bundle, 'installer.deb'), 'linux package bytes');
  await fs.writeFile(path.join(f.bundle, 'installer.rpm'), 'rpm package bytes');
  const calls = [];
  const result = await verifyReleaseAttestations({
    repo, sourceRef: ref, signerWorkflow: workflow, bundleDir: f.bundle,
    runCommand(command, args, options) {
      calls.push({ command, args, options });
      return Buffer.from('verified');
    },
  });
  assert.equal(result.verifiedArtifactCount, 2);
  assert.equal(result.verifiedPredicateCount, 4);
  assert.deepEqual(result.subjects.map((subject) => subject.name), ['installer.deb', 'installer.rpm']);
  assert.equal(calls.length, 4);
  const verifiedPairs = calls.map((call) => ({
    subject: path.basename(call.args[2]),
    predicate: call.args[call.args.indexOf('--predicate-type') + 1],
  }));
  assert.deepEqual(verifiedPairs, [
    { subject: 'installer.deb', predicate: 'https://slsa.dev/provenance/v1' },
    { subject: 'installer.deb', predicate: 'https://spdx.dev/Document/v2.3' },
    { subject: 'installer.rpm', predicate: 'https://slsa.dev/provenance/v1' },
    { subject: 'installer.rpm', predicate: 'https://spdx.dev/Document/v2.3' },
  ]);
  for (const call of calls) {
    assert.equal(call.command, 'gh');
    assert.equal(call.args[0], 'attestation');
    assert.equal(call.args[1], 'verify');
    assert.ok(call.args.includes('--repo') && call.args.includes(repo));
    assert.ok(call.args.includes('--signer-workflow') && call.args.includes(workflow));
    assert.ok(call.args.includes('--source-ref') && call.args.includes(ref));
    assert.ok(call.args.includes('--deny-self-hosted-runners'));
    assert.ok(call.args.includes('--cert-oidc-issuer') && call.args.includes('https://token.actions.githubusercontent.com'));
    assert.equal(call.options.timeout, 120_000);
  }
});

test('attestation inventory rejects empty stages, extra files, nested paths and installer symlinks', async (t) => {
  const f = await fixture(t);
  await assert.rejects(inventoryAttestationSubjects(f.bundle), /empty/);
  await fs.writeFile(path.join(f.bundle, 'installer.msi'), 'package');
  await fs.writeFile(path.join(f.bundle, 'README.txt'), 'unexpected extra payload');
  await assert.rejects(inventoryAttestationSubjects(f.bundle), /non-distributable/);
  await fs.rm(path.join(f.bundle, 'README.txt'));
  await fs.mkdir(path.join(f.bundle, 'nested'));
  await assert.rejects(inventoryAttestationSubjects(f.bundle), /only regular files/);
  await fs.rmdir(path.join(f.bundle, 'nested'));
  await fs.rm(path.join(f.bundle, 'installer.msi'));
  const real = path.join(f.root, 'outside.msi');
  await fs.writeFile(real, 'package');
  await fs.symlink(real, path.join(f.bundle, 'installer.msi')).catch((error) => {
    if (['EPERM', 'EACCES', 'ENOTSUP'].includes(error.code)) return t.skip('Symlink creation is unavailable on this runner.');
    throw error;
  });
  await assert.rejects(inventoryAttestationSubjects(f.bundle), /only regular files/);
});

test('attestation verification fails closed on any failed or unavailable GitHub CLI result', async (t) => {
  const f = await fixture(t);
  await fs.writeFile(path.join(f.bundle, 'installer.exe'), 'windows installer');
  await assert.rejects(verifyReleaseAttestations({
    repo, sourceRef: ref, signerWorkflow: workflow, bundleDir: f.bundle,
    runCommand() { const error = new Error('sensitive tool stderr must not escape'); error.status = 1; throw error; },
  }), /Cryptographic SLSA provenance attestation verification failed for installer.exe \(exit 1\); release rejected\./);
});

test('release is rejected when SLSA provenance verifies but the SPDX SBOM attestation is missing', async (t) => {
  const f = await fixture(t);
  await fs.writeFile(path.join(f.bundle, 'installer.dmg'), 'macOS installer');
  const predicates = [];
  await assert.rejects(verifyReleaseAttestations({
    repo, sourceRef: ref, signerWorkflow: workflow, bundleDir: f.bundle,
    runCommand(_command, args) {
      const type = args[args.indexOf('--predicate-type') + 1];
      predicates.push(type);
      if (type === 'https://spdx.dev/Document/v2.3') {
        const error = new Error('SBOM attestation unavailable');
        error.status = 1;
        throw error;
      }
      return Buffer.from('verified SLSA provenance');
    },
  }), /Cryptographic SPDX SBOM attestation verification failed for installer\.dmg \(exit 1\); release rejected\./);
  assert.deepEqual(predicates, [
    'https://slsa.dev/provenance/v1',
    'https://spdx.dev/Document/v2.3',
  ]);
});
