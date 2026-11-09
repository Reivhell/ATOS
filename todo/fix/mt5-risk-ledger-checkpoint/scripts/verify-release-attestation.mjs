#!/usr/bin/env node
/**
 * Strict consumer-side verification for release installers. The GitHub CLI verifies
 * the Sigstore signature and provenance; this wrapper pins the expected repository,
 * tag ref, workflow identity, OIDC issuer, and hosted-runner policy.
 */
import fs from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

const INSTALLER_EXTENSIONS = new Set(['.deb', '.rpm', '.appimage', '.msi', '.exe', '.dmg', '.pkg']);
const WORKFLOW_PATH = '.github/workflows/ci.yml';
const OIDC_ISSUER = 'https://token.actions.githubusercontent.com';
const MAX_ARTIFACTS = 64;
const REQUIRED_PREDICATES = Object.freeze([
  Object.freeze({ label: 'SLSA provenance', type: 'https://slsa.dev/provenance/v1' }),
  Object.freeze({ label: 'SPDX SBOM', type: 'https://spdx.dev/Document/v2.3' }),
]);

function validateRepo(value) {
  if (typeof value !== 'string' || !/^[A-Za-z0-9-]+\/[A-Za-z0-9_.-]+$/.test(value)) {
    throw new Error('Repository must use the exact <owner>/<repo> format.');
  }
  return value;
}

function validateSourceRef(value) {
  if (typeof value !== 'string' || !/^refs\/tags\/v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value)) {
    throw new Error('Source ref must be a version tag such as refs/tags/v0.11.0.');
  }
  return value;
}

function validateSignerWorkflow(value, repo) {
  const expected = `${repo}/${WORKFLOW_PATH}`;
  if (value !== expected) throw new Error(`Signer workflow must be exactly ${expected}.`);
  return value;
}

export function parseAttestationVerificationArgs(args) {
  const options = {};
  const allowed = new Map([
    ['--repo', 'repo'],
    ['--source-ref', 'sourceRef'],
    ['--signer-workflow', 'signerWorkflow'],
    ['--bundle-dir', 'bundleDir'],
  ]);
  for (let index = 0; index < args.length; index += 1) {
    const flag = args[index];
    const key = allowed.get(flag);
    const value = args[index + 1];
    if (!key || !value || value.startsWith('--')) throw new Error(`Unknown or incomplete option: ${flag}`);
    if (Object.hasOwn(options, key)) throw new Error(`Duplicate option: ${flag}`);
    options[key] = value;
    index += 1;
  }

  for (const key of ['repo', 'sourceRef', 'signerWorkflow', 'bundleDir']) {
    if (!options[key]) throw new Error(`Missing required option: --${key.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)}`);
  }
  options.repo = validateRepo(options.repo);
  options.sourceRef = validateSourceRef(options.sourceRef);
  options.signerWorkflow = validateSignerWorkflow(options.signerWorkflow, options.repo);
  options.bundleDir = path.resolve(options.bundleDir);
  return options;
}

/**
 * Require a flat, exact installer-only directory. Unexpected files and directory
 * entries fail closed so a wildcard attestation cannot silently miss a release payload.
 */
export async function inventoryAttestationSubjects(bundleDir) {
  const root = path.resolve(bundleDir);
  const rootStat = await fs.lstat(root).catch((error) => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  if (!rootStat?.isDirectory() || rootStat.isSymbolicLink()) {
    throw new Error(`Installer stage must be a real directory: ${root}`);
  }

  const children = await fs.readdir(root, { withFileTypes: true });
  children.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
  if (!children.length) throw new Error('Installer stage is empty; there is nothing to attest or verify.');
  if (children.length > MAX_ARTIFACTS) throw new Error(`Installer stage exceeds ${MAX_ARTIFACTS} entries.`);

  const subjects = [];
  for (const child of children) {
    const filePath = path.join(root, child.name);
    const stat = await fs.lstat(filePath);
    if (stat.isSymbolicLink() || !stat.isFile()) {
      throw new Error(`Installer stage must contain only regular files; found an unexpected entry: ${child.name}`);
    }
    if (!INSTALLER_EXTENSIONS.has(path.extname(child.name).toLowerCase())) {
      throw new Error(`Installer stage contains a non-distributable file: ${child.name}`);
    }
    if (!Number.isSafeInteger(stat.size) || stat.size <= 0) {
      throw new Error(`Installer is empty or has an invalid size: ${child.name}`);
    }
    subjects.push({ name: child.name, path: filePath, sizeBytes: stat.size });
  }
  return subjects;
}

export async function verifyReleaseAttestations({
  repo,
  sourceRef,
  signerWorkflow,
  bundleDir,
  ghCommand = 'gh',
  runCommand = execFileSync,
}) {
  const expectedRepo = validateRepo(repo);
  const expectedRef = validateSourceRef(sourceRef);
  const expectedWorkflow = validateSignerWorkflow(signerWorkflow, expectedRepo);
  if (typeof ghCommand !== 'string' || !ghCommand.trim()) throw new Error('GitHub CLI command must be a non-empty executable name.');

  const subjects = await inventoryAttestationSubjects(bundleDir);
  for (const subject of subjects) {
    for (const predicate of REQUIRED_PREDICATES) {
      const args = [
        'attestation', 'verify', subject.path,
        '--predicate-type', predicate.type,
        '--repo', expectedRepo,
        '--signer-workflow', expectedWorkflow,
        '--source-ref', expectedRef,
        '--cert-oidc-issuer', OIDC_ISSUER,
        '--deny-self-hosted-runners',
      ];
      try {
        runCommand(ghCommand, args, {
          encoding: 'utf8',
          stdio: ['ignore', 'pipe', 'pipe'],
          timeout: 120_000,
          windowsHide: true,
          maxBuffer: 8 * 1024 * 1024,
        });
      } catch (error) {
        const code = Number.isInteger(error?.status) ? ` (exit ${error.status})` : '';
        throw new Error(`Cryptographic ${predicate.label} attestation verification failed for ${subject.name}${code}; release rejected.`);
      }
    }
  }
  return {
    repo: expectedRepo,
    sourceRef: expectedRef,
    signerWorkflow: expectedWorkflow,
    verifiedArtifactCount: subjects.length,
    verifiedPredicateCount: subjects.length * REQUIRED_PREDICATES.length,
    subjects,
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  try {
    const options = parseAttestationVerificationArgs(process.argv.slice(2));
    const result = await verifyReleaseAttestations(options);
    console.log(`Verified ${result.verifiedArtifactCount} installer attestation(s).`);
    console.log(`Expected repository: ${result.repo}`);
    console.log(`Expected source ref: ${result.sourceRef}`);
    console.log(`Expected signer workflow: ${result.signerWorkflow}`);
    for (const subject of result.subjects) console.log(`  VERIFIED ${subject.name} (${subject.sizeBytes} bytes)`);
  } catch (error) {
    console.error(`Release attestation verification BLOCKED: ${error.message}`);
    process.exitCode = 1;
  }
}
