#!/usr/bin/env node
/**
 * Create and verify unsigned, machine-readable release evidence for packaged Tauri artifacts.
 * This detects accidental corruption and inventory drift; it is not a digital signature or attestation.
 */
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { createReadStream } from 'node:fs';

export const RELEASE_EVIDENCE_SCHEMA = 'agentic-trading-os.release-evidence.v1';
export const RELEASE_CHECKSUM_SCHEMA = 'agentic-trading-os.release-checksums.v1';
const MANIFEST_NAME = 'release-manifest.json';
const CHECKSUMS_NAME = 'checksums.json';
const SHA256_RE = /^[a-f0-9]{64}$/;

function sha256Bytes(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}

async function hashFile(filePath) {
  const hash = crypto.createHash('sha256');
  for await (const chunk of createReadStream(filePath)) hash.update(chunk);
  return hash.digest('hex');
}

function toPosixPath(filePath) {
  return filePath.split(path.sep).join('/');
}

function isInside(parent, candidate) {
  const relative = path.relative(parent, candidate);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}

async function inventoryDirectory(bundleDir) {
  const root = path.resolve(bundleDir);
  const rootStat = await fs.lstat(root).catch(() => null);
  if (!rootStat?.isDirectory() || rootStat.isSymbolicLink()) {
    throw new Error(`Bundle directory is missing or is not a real directory: ${root}`);
  }

  const entries = [];
  const pending = [''];
  while (pending.length) {
    const relativeDirectory = pending.pop();
    const absoluteDirectory = path.join(root, relativeDirectory);
    const children = await fs.readdir(absoluteDirectory, { withFileTypes: true });
    children.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
    for (const child of children) {
      const relative = relativeDirectory ? path.join(relativeDirectory, child.name) : child.name;
      const absolute = path.join(root, relative);
      const relativePosix = toPosixPath(relative);
      const stat = await fs.lstat(absolute);
      if (stat.isSymbolicLink()) {
        const target = await fs.readlink(absolute);
        const resolvedTarget = path.resolve(path.dirname(absolute), target);
        if (path.isAbsolute(target) || !isInside(root, resolvedTarget)) {
          throw new Error(`Symbolic link escapes the release bundle: ${relativePosix} -> ${target}`);
        }
        const targetBytes = Buffer.from(target, 'utf8');
        entries.push({ path: relativePosix, kind: 'symlink', sizeBytes: targetBytes.length, sha256: sha256Bytes(targetBytes), linkTarget: target });
      } else if (stat.isDirectory()) {
        pending.push(relative);
      } else if (stat.isFile()) {
        entries.push({ path: relativePosix, kind: 'file', sizeBytes: stat.size, sha256: await hashFile(absolute) });
      } else {
        throw new Error(`Unsupported filesystem entry in release bundle: ${relativePosix}`);
      }
    }
  }
  entries.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
  if (!entries.length) throw new Error('Release bundle contains no files or symlinks; refusing to create empty evidence.');
  return entries;
}

function readGitMetadata(projectRoot) {
  const runGit = (...args) => execFileSync('git', args, { cwd: projectRoot, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim();
  const revision = process.env.GITHUB_SHA || runGit('rev-parse', 'HEAD');
  if (!/^[a-f0-9]{40,64}$/i.test(revision)) throw new Error(`Source revision must be a full Git commit hash; received: ${revision || '(empty)'}`);
  const status = runGit('status', '--porcelain', '--untracked-files=normal');
  if (status) throw new Error('Release evidence requires a clean Git working tree; commit or remove source changes before packaging.');
  return { revision: revision.toLowerCase(), workingTree: 'clean' };
}

function commandVersion(command, arg = '--version') {
  try {
    return execFileSync(command, [arg], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 10_000 }).trim().split(/\r?\n/, 1)[0];
  } catch {
    return null;
  }
}

function applicationVersion(projectRoot) {
  const packageJson = JSON.parse(requireRead(path.join(projectRoot, 'package.json')));
  const tauri = JSON.parse(requireRead(path.join(projectRoot, 'src-tauri', 'tauri.conf.json')));
  const cargo = requireRead(path.join(projectRoot, 'src-tauri', 'Cargo.toml'));
  const cargoVersion = cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
  const versions = [packageJson.version, tauri.version, cargoVersion];
  if (versions.some((value) => typeof value !== 'string' || !value) || new Set(versions).size !== 1) {
    throw new Error(`Application manifests are not version-aligned: ${versions.join(', ')}`);
  }
  return { name: packageJson.name, version: packageJson.version };
}

function requireRead(filePath) {
  try { return requireReadSync(filePath); }
  catch { throw new Error(`Required release manifest is missing or unreadable: ${path.basename(filePath)}`); }
}

function requireReadSync(filePath) {
  return fsSync.readFileSync(filePath, 'utf8');
}

// Kept as a static import to make manifest validation synchronous and straightforward.
import fsSync from 'node:fs';

function ensureEvidenceDirectoryIsSeparate(bundleDir, evidenceDir) {
  const bundle = path.resolve(bundleDir);
  const evidence = path.resolve(evidenceDir);
  if (isInside(bundle, evidence)) throw new Error('Evidence directory must be outside the bundle directory to avoid self-referential manifests.');
  return { bundle, evidence };
}

async function assertEvidenceDirectoryShape(evidenceDir) {
  await fs.mkdir(evidenceDir, { recursive: true });
  const names = await fs.readdir(evidenceDir);
  const unexpected = names.filter((name) => ![MANIFEST_NAME, CHECKSUMS_NAME].includes(name));
  if (unexpected.length) throw new Error(`Evidence directory contains unexpected entries: ${unexpected.join(', ')}`);
}

function stableJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function assertInventoryPaths(entries) {
  if (!Array.isArray(entries) || entries.length === 0) throw new Error('Manifest artifact inventory is missing or empty.');
  const seen = new Set();
  for (const entry of entries) {
    if (!entry || typeof entry.path !== 'string' || !entry.path || entry.path.startsWith('/') || /^[a-z]:\//i.test(entry.path) || entry.path.includes('\\') || entry.path.split('/').some((part) => part === '..' || part === '' || part === '.')) {
      throw new Error('Manifest contains an unsafe or malformed artifact path.');
    }
    if (seen.has(entry.path)) throw new Error(`Manifest contains a duplicate artifact path: ${entry.path}`);
    seen.add(entry.path);
    if (!['file', 'symlink'].includes(entry.kind) || !Number.isSafeInteger(entry.sizeBytes) || entry.sizeBytes < 0 || !SHA256_RE.test(entry.sha256)) {
      throw new Error(`Manifest contains invalid metadata for artifact: ${entry.path}`);
    }
    if (entry.kind === 'symlink' && typeof entry.linkTarget !== 'string') throw new Error(`Manifest symlink target is missing: ${entry.path}`);
  }
}

/** Generate release-manifest.json and checksums.json for an already-built bundle. */
export async function createReleaseEvidence({
  bundleDir = 'src-tauri/target/release/bundle',
  evidenceDir = 'release-evidence',
  projectRoot = process.cwd(),
  generatedAt = new Date().toISOString(),
  gitMetadata = null,
  target = null,
  toolchain = null,
  ci = null,
} = {}) {
  const dirs = ensureEvidenceDirectoryIsSeparate(bundleDir, evidenceDir);
  const app = applicationVersion(projectRoot);
  const git = gitMetadata ?? readGitMetadata(projectRoot);
  if (!/^[a-f0-9]{40,64}$/i.test(git.revision ?? '') || git.workingTree !== 'clean') {
    throw new Error('Release evidence requires a full source commit and a clean working tree.');
  }
  if (!Number.isFinite(Date.parse(generatedAt)) || !generatedAt.endsWith('Z')) {
    throw new Error('generatedAt must be a valid UTC ISO-8601 timestamp ending in Z.');
  }
  const artifacts = await inventoryDirectory(dirs.bundle);
  assertInventoryPaths(artifacts);
  await assertEvidenceDirectoryShape(dirs.evidence);

  const manifest = {
    schemaVersion: RELEASE_EVIDENCE_SCHEMA,
    project: app.name,
    version: app.version,
    generatedAt,
    source: { revision: git.revision.toLowerCase(), workingTree: 'clean' },
    target: target ?? {
      runnerOS: process.env.RUNNER_OS ?? process.platform,
      platform: process.platform,
      architecture: process.arch,
    },
    toolchain: toolchain ?? {
      node: process.version,
      npm: commandVersion('npm'),
      rustc: commandVersion('rustc'),
      cargo: commandVersion('cargo'),
    },
    ci: ci ?? {
      provider: process.env.GITHUB_ACTIONS === 'true' ? 'github-actions' : 'unknown',
      runId: process.env.GITHUB_RUN_ID ?? null,
      runAttempt: process.env.GITHUB_RUN_ATTEMPT ?? null,
      ref: process.env.GITHUB_REF ?? null,
    },
    artifactCount: artifacts.length,
    artifacts,
    authenticity: 'unsigned-hashes-only',
  };
  const manifestBytes = Buffer.from(stableJson(manifest), 'utf8');
  const checksumMap = {};
  for (const entry of artifacts) checksumMap[`bundle/${entry.path}`] = entry.sha256;
  checksumMap[`evidence/${MANIFEST_NAME}`] = sha256Bytes(manifestBytes);
  const checksums = { schemaVersion: RELEASE_CHECKSUM_SCHEMA, hashes: checksumMap };

  await fs.writeFile(path.join(dirs.evidence, `${MANIFEST_NAME}.tmp`), manifestBytes);
  await fs.rename(path.join(dirs.evidence, `${MANIFEST_NAME}.tmp`), path.join(dirs.evidence, MANIFEST_NAME));
  await fs.writeFile(path.join(dirs.evidence, `${CHECKSUMS_NAME}.tmp`), stableJson(checksums));
  await fs.rename(path.join(dirs.evidence, `${CHECKSUMS_NAME}.tmp`), path.join(dirs.evidence, CHECKSUMS_NAME));
  return { manifest, checksums };
}

/** Verify the exact bundle inventory, every file/link digest, manifest schema, and checksum map. */
export async function verifyReleaseEvidence({
  bundleDir = 'src-tauri/target/release/bundle',
  evidenceDir = 'release-evidence',
  projectRoot = process.cwd(),
  expectedSourceRevision = process.env.GITHUB_SHA,
} = {}) {
  const dirs = ensureEvidenceDirectoryIsSeparate(bundleDir, evidenceDir);
  await assertEvidenceDirectoryShape(dirs.evidence);
  let manifest;
  let checksums;
  try {
    manifest = JSON.parse(await fs.readFile(path.join(dirs.evidence, MANIFEST_NAME), 'utf8'));
    checksums = JSON.parse(await fs.readFile(path.join(dirs.evidence, CHECKSUMS_NAME), 'utf8'));
  } catch {
    throw new Error('Release manifest or checksums file is missing or invalid JSON.');
  }
  if (manifest?.schemaVersion !== RELEASE_EVIDENCE_SCHEMA || checksums?.schemaVersion !== RELEASE_CHECKSUM_SCHEMA) {
    throw new Error('Unsupported release-evidence/checksum schema version.');
  }
  const app = applicationVersion(projectRoot);
  if (manifest.project !== app.name || manifest.version !== app.version) {
    throw new Error(`Evidence identity/version does not match current source manifests (${manifest.project}@${manifest.version} vs ${app.name}@${app.version}).`);
  }
  if (!/^[a-f0-9]{40,64}$/i.test(manifest.source?.revision ?? '') || manifest.source?.workingTree !== 'clean') {
    throw new Error('Release manifest does not identify a full commit from a clean source tree.');
  }
  if (expectedSourceRevision && manifest.source.revision !== expectedSourceRevision.toLowerCase()) {
    throw new Error('Release manifest source revision does not match the expected source commit.');
  }
  if (manifest.authenticity !== 'unsigned-hashes-only') throw new Error('Manifest authenticity label is missing or unsupported.');
  assertInventoryPaths(manifest.artifacts);
  if (manifest.artifactCount !== manifest.artifacts.length) throw new Error('Manifest artifactCount does not match its inventory.');

  const actual = await inventoryDirectory(dirs.bundle);
  const expectedBytes = stableJson(manifest.artifacts);
  if (stableJson(actual) !== expectedBytes) {
    const expectedPaths = new Set(manifest.artifacts.map((entry) => entry.path));
    const actualPaths = new Set(actual.map((entry) => entry.path));
    const added = [...actualPaths].filter((entry) => !expectedPaths.has(entry));
    const removed = [...expectedPaths].filter((entry) => !actualPaths.has(entry));
    if (added.length || removed.length) throw new Error(`Artifact inventory drift; added=[${added.join(', ')}], removed=[${removed.join(', ')}].`);
    throw new Error('Artifact hashes, file sizes, kinds, or symlink targets do not match the release manifest.');
  }

  const manifestBytes = await fs.readFile(path.join(dirs.evidence, MANIFEST_NAME));
  const expectedHashes = {};
  for (const entry of actual) expectedHashes[`bundle/${entry.path}`] = entry.sha256;
  expectedHashes[`evidence/${MANIFEST_NAME}`] = sha256Bytes(manifestBytes);
  const sortedActual = Object.fromEntries(Object.entries(checksums?.hashes ?? {}).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
  const sortedExpected = Object.fromEntries(Object.entries(expectedHashes).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)));
  if (JSON.stringify(sortedActual) !== JSON.stringify(sortedExpected)) {
    throw new Error('checksums.json does not exactly cover the current artifacts and release manifest.');
  }
  return { manifest, verifiedArtifactCount: actual.length };
}

function parseCli(args) {
  const [mode, ...rest] = args;
  if (!['create', 'verify'].includes(mode)) throw new Error('Usage: node scripts/release-evidence.mjs <create|verify> [--bundle-dir <path>] [--evidence-dir <path>]');
  const options = { mode };
  for (let i = 0; i < rest.length; i += 1) {
    const flag = rest[i];
    if (!['--bundle-dir', '--evidence-dir'].includes(flag) || !rest[i + 1] || rest[i + 1].startsWith('--')) {
      throw new Error(`Unknown or incomplete option: ${flag}`);
    }
    options[flag === '--bundle-dir' ? 'bundleDir' : 'evidenceDir'] = rest[++i];
  }
  return options;
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  try {
    const options = parseCli(process.argv.slice(2));
    if (options.mode === 'create') {
      const { manifest } = await createReleaseEvidence(options);
      console.log(`Release evidence created for ${manifest.project} v${manifest.version}: ${manifest.artifactCount} artifact entries.`);
      console.log(`Source revision: ${manifest.source.revision}; target: ${manifest.target.runnerOS}/${manifest.target.architecture}.`);
      console.log('Evidence is unsigned; hashes detect corruption but do not authenticate publisher identity.');
    } else {
      const result = await verifyReleaseEvidence(options);
      console.log(`Release evidence verified: ${result.verifiedArtifactCount} artifact entries for ${result.manifest.project} v${result.manifest.version}.`);
      console.log(`Source revision: ${result.manifest.source.revision}; evidence remains unsigned.`);
    }
  } catch (error) {
    console.error(`Release evidence ${process.argv[2] === 'verify' ? 'verification' : 'generation'} failed: ${error.message}`);
    process.exitCode = 1;
  }
}
