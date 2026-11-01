#!/usr/bin/env node
/**
 * Fail-closed preflight for tagged desktop releases. It checks release evidence
 * prerequisites; it does not replace running the build, native tests, or acceptance.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { releaseTagVersionError } from './release-version-policy.mjs';
import { findUnpinnedActions } from './release-action-pin-policy.mjs';

const root = process.cwd();
const failures = [];
const checks = [];
const read = (file) => {
  try { return fs.readFileSync(path.join(root, file), 'utf8'); }
  catch { return null; }
};
const addFailure = (message) => failures.push(message);
const present = (file) => fs.existsSync(path.join(root, file));
const parseJson = (file) => {
  const raw = read(file);
  if (raw === null) return null;
  try { return JSON.parse(raw); }
  catch { addFailure(`${file} is not valid JSON`); return null; }
};
const commandWorks = (command, args = ['--version']) => {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', timeout: 10_000, windowsHide: true });
  return !result.error && result.status === 0;
};

const packageJson = parseJson('package.json');
const tauriConfig = parseJson('src-tauri/tauri.conf.json');
const packageLock = parseJson('package-lock.json');
const cargoToml = read('src-tauri/Cargo.toml');
const cargoLock = read('src-tauri/Cargo.lock');
const workflow = read('.github/workflows/ci.yml');

for (const file of ['package-lock.json', 'bun.lock', 'src-tauri/Cargo.lock']) {
  if (!present(file)) addFailure(`Missing ${file}; release dependency resolution is not pinned`);
  else checks.push(`present: ${file}`);
}

const cargoVersion = cargoToml?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const cargoLockedVersion = cargoLock?.match(/\[\[package\]\]\s*\nname\s*=\s*"agentic-trading-os"\s*\nversion\s*=\s*"([^"]+)"/m)?.[1];
const versions = [
  ['package.json', packageJson?.version],
  ['src-tauri/Cargo.toml', cargoVersion],
  ['src-tauri/tauri.conf.json', tauriConfig?.version],
];
if (versions.some(([, version]) => typeof version !== 'string' || !version)) {
  addFailure('Could not read a valid application version from all three manifests');
} else if (new Set(versions.map(([, version]) => version)).size !== 1) {
  addFailure(`Application version mismatch: ${versions.map(([name, version]) => `${name}=${version}`).join(', ')}`);
} else {
  checks.push(`manifest versions aligned: ${versions[0][1]}`);
}
const tagVersionError = releaseTagVersionError(process.env.GITHUB_REF, packageJson?.version);
if (tagVersionError) addFailure(tagVersionError);
else if (process.env.GITHUB_REF?.startsWith('refs/tags/')) checks.push(`release tag matches all application manifests: ${process.env.GITHUB_REF.slice('refs/tags/'.length)}`);
if (packageJson && packageLock) {
  if (packageLock.lockfileVersion !== 3) addFailure(`package-lock.json lockfileVersion must be 3; found ${packageLock.lockfileVersion}`);
  if (packageLock.packages?.['']?.name !== packageJson.name || packageLock.packages?.['']?.version !== packageJson.version) {
    addFailure('package-lock.json root package identity/version does not match package.json');
  } else checks.push('npm lock root identity/version matches package.json');
}
if (cargoLock && cargoVersion) {
  if (!cargoLockedVersion) addFailure('Cargo.lock is missing the agentic-trading-os root package entry');
  else if (cargoLockedVersion !== cargoVersion) addFailure(`Cargo.lock root version ${cargoLockedVersion} does not match Cargo.toml ${cargoVersion}`);
  else checks.push('Cargo.lock root package version matches Cargo.toml');
}

for (const executable of ['svelte-check', 'vite', 'tauri']) {
  const windows = process.platform === 'win32';
  const candidates = windows
    ? [`node_modules/.bin/${executable}.cmd`, `node_modules/.bin/${executable}.exe`]
    : [`node_modules/.bin/${executable}`];
  if (!candidates.some(present)) addFailure(`Required installed build executable is missing: ${executable} (run bun install --frozen-lockfile first)`);
  else checks.push(`installed frontend executable: ${executable}`);
}
for (const [command, args] of [
  ['bun', ['--version']],
  ['cargo', ['--version']], ['rustfmt', ['--version']], ['cargo', ['clippy', '--version']],
]) {
  if (!commandWorks(command, args)) addFailure(`Required release toolchain command failed: ${command} ${args.join(' ')}`);
  else checks.push(`toolchain available: ${command} ${args.join(' ')}`);
}

if (!packageJson?.scripts?.build || !packageJson?.scripts?.test || !packageJson?.scripts?.tauri) {
  addFailure('package.json must define build, test, and tauri scripts');
}
if (!packageJson?.scripts?.['eval:agents'] || !present('scripts/agent-release-evaluation.mjs') || !present('scripts/test-agent-release-evaluation.test.mjs')) {
  addFailure('Integrated AI Agent release qualification command or tests are missing');
} else {
  const result = spawnSync('bun', ['run', 'scripts/agent-release-evaluation.mjs'], { cwd: root, encoding: 'utf8', timeout: 30_000, windowsHide: true });
  if (result.error || result.status !== 0) addFailure('AI Agent release qualification gate failed; inspect bun run eval:agents output');
  else checks.push('AI Agent release qualification gate passed against the application boundary');
}
if (!packageJson?.scripts?.['release:evidence'] || !packageJson?.scripts?.['release:evidence:verify']) {
  addFailure('package.json must define release evidence generation and verification scripts');
}
if (!present('scripts/release-evidence.mjs') || !present('scripts/test-release-evidence.test.mjs')) {
  addFailure('Release artifact evidence implementation/tests are missing');
}
if (!packageJson?.scripts?.['release:stage'] || !packageJson?.scripts?.['release:attestation:verify']) {
  addFailure('package.json must define strict installer staging and signed attestation verification commands');
}
for (const file of ['scripts/prepare-release-artifacts.mjs', 'scripts/test-release-artifact-staging.test.mjs', 'scripts/verify-release-attestation.mjs', 'scripts/test-release-attestation.test.mjs', 'scripts/release-version-policy.mjs', 'scripts/test-release-version-policy.test.mjs']) {
  if (!present(file)) addFailure(`Release supply-chain implementation/test is missing: ${file}`);
}
if (!workflow) addFailure('CI workflow is missing or unreadable');
else {
  const unpinnedActions = findUnpinnedActions(workflow);
  if (unpinnedActions.length) {
    const names = [...new Set(unpinnedActions.map(({ target }) => target))].sort();
    addFailure(`GitHub Actions must use reviewed full 40-character commit SHAs; mutable refs remain (${names.length}): ${names.join(', ')}`);
  } else checks.push('all external GitHub Actions use reviewed full commit SHAs');
  // Check a contract only in the file that actually owns it. The workflow
  // invokes the verifier and preflight; their internal policy tokens belong
  // to their source modules, not to the workflow text itself.
  const workflowContracts = [
    'bun run release:preflight',
    'bun install --frozen-lockfile', 'bun test api/test/app.integration.test.ts',
    'bun run build', 'bun run test',
    'cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check',
    'cargo test --locked', 'cargo clippy --locked', 'bun run tauri build',
    'bun run release:evidence', 'bun run release:evidence:verify',
    'bun run scripts/prepare-release-artifacts.mjs',
    'scripts/verify-release-attestation.mjs', 'actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6',
    'anchore/sbom-action@66cbf4bc1f1c0d2edc94016e65bc221b6bb0ad6c', 'sbom-path:', '--signer-workflow',
    '--source-ref', 'atos-release-staging/**', 'bun run eval:agents',
  ];
  for (const contract of workflowContracts) {
    if (!workflow.includes(contract)) addFailure(`CI does not enforce the release contract: ${contract}`);
  }
}
const attestationVerifier = read('scripts/verify-release-attestation.mjs');
if (!attestationVerifier?.includes("'--deny-self-hosted-runners'")) {
  addFailure('Attestation verifier must reject self-hosted runners');
}
for (const predicate of ['https://slsa.dev/provenance/v1', 'https://spdx.dev/Document/v2.3']) {
  if (!attestationVerifier?.includes(predicate)) addFailure(`Attestation verifier must require both release evidence predicates: ${predicate}`);
}
const releasePolicy = read('scripts/release-version-policy.mjs');
const readinessPolicy = read('scripts/check-release-readiness.mjs');
if (!releasePolicy?.includes('releaseTagVersionError') || !readinessPolicy?.includes('releaseTagVersionError')) {
  addFailure('Release tag-to-manifest version policy is not wired into release preflight');
}
if (!readinessPolicy?.includes('release tag matches all application manifests')) {
  addFailure('Release preflight is missing its explicit manifest-aligned tag contract');
}

if (failures.length) {
  console.error(`Release preflight BLOCKED (${failures.length} issue(s))`);
  for (const item of checks) console.log(`  PASS ${item}`);
  for (const item of failures) console.error(`  BLOCK ${item}`);
  console.error('This preflight is intentionally fail-closed and cannot certify a release by itself.');
  process.exit(1);
}
console.log(`Release preflight prerequisites passed for v${packageJson.version}.`);
for (const item of checks) console.log(`  PASS ${item}`);
console.log('Run the full CI build/tests and desktop acceptance before calling the release production-ready.');
