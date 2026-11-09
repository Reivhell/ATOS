#!/usr/bin/env node
/**
 * Stage only real, installable Tauri release payloads. Build intermediates, app-bundle
 * internals, detached signatures, and unrelated files are deliberately not publishable
 * through this path. This script is a fail-closed release boundary, not a bundler.
 */
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import { constants } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { fileURLToPath } from 'node:url';

export const RELEASE_PLATFORM_LAYOUT = Object.freeze({
  linux: Object.freeze([
    Object.freeze({ directory: 'deb', extension: '.deb', required: true }),
    Object.freeze({ directory: 'rpm', extension: '.rpm', required: true }),
    Object.freeze({ directory: 'appimage', extension: '.appimage', required: true }),
  ]),
  windows: Object.freeze([
    Object.freeze({ directory: 'msi', extension: '.msi', required: true }),
    Object.freeze({ directory: 'nsis', extension: '.exe', required: true }),
  ]),
  macos: Object.freeze([
    Object.freeze({ directory: 'dmg', extension: '.dmg', required: true }),
    Object.freeze({ directory: 'macos', extension: '.pkg', required: false }),
  ]),
});

const SHA256_RE = /^[a-f0-9]{64}$/;
const MAX_HEADER_BYTES = 64 * 1024;
const COPY_CHUNK_BYTES = 1024 * 1024;
const SOURCE_OPEN_FLAGS = constants.O_RDONLY
  | (constants.O_NOFOLLOW ?? 0)
  | (constants.O_NONBLOCK ?? 0);
const DESTINATION_OPEN_FLAGS = constants.O_RDWR
  | constants.O_CREAT
  | constants.O_EXCL
  | (constants.O_NOFOLLOW ?? 0);

function sameObject(left, right) {
  if (!left.isFile() || !right.isFile()) return false;
  if (left.dev !== right.dev || left.mode !== right.mode) return false;
  if (left.ino !== 0n && right.ino !== 0n) return left.ino === right.ino;
  return left.size === right.size
    && left.mtimeNs === right.mtimeNs
    && left.ctimeNs === right.ctimeNs;
}

function sameContentVersion(left, right) {
  return sameObject(left, right)
    && left.size === right.size
    && left.mtimeNs === right.mtimeNs
    && left.ctimeNs === right.ctimeNs;
}

async function readAt(handle, position, length) {
  const buffer = Buffer.alloc(length);
  let bytesRead = 0;
  while (bytesRead < length) {
    const result = await handle.read(buffer, bytesRead, length - bytesRead, position + bytesRead);
    if (result.bytesRead === 0) break;
    bytesRead += result.bytesRead;
  }
  return buffer.subarray(0, bytesRead);
}

async function hashHandle(handle, expectedSize) {
  const hash = crypto.createHash('sha256');
  const buffer = Buffer.allocUnsafe(COPY_CHUNK_BYTES);
  let position = 0;
  while (position < expectedSize) {
    const length = Math.min(buffer.length, expectedSize - position);
    const { bytesRead } = await handle.read(buffer, 0, length, position);
    if (bytesRead === 0) throw new Error('Artifact ended before its validated size was read.');
    hash.update(buffer.subarray(0, bytesRead));
    position += bytesRead;
  }
  return hash.digest('hex');
}

async function assertPathStillReferencesFile(filePath, expectedStat, description) {
  const current = await fs.lstat(filePath, { bigint: true }).catch((error) => {
    if (error.code === 'ENOENT') throw new Error(`${description} disappeared during release staging: ${filePath}`);
    throw error;
  });
  if (current.isSymbolicLink() || !sameObject(current, expectedStat)) {
    throw new Error(`${description} was replaced during release staging: ${filePath}`);
  }
  return current;
}

async function assertDirectoryStillStable(directoryPath, expectedStat, description) {
  const current = await fs.lstat(directoryPath, { bigint: true }).catch((error) => {
    if (error.code === 'ENOENT') throw new Error(`${description} disappeared during release staging: ${directoryPath}`);
    throw error;
  });
  const identityChanged = current.dev !== expectedStat.dev
    || (current.ino !== 0n && expectedStat.ino !== 0n && current.ino !== expectedStat.ino)
    || ((current.ino === 0n || expectedStat.ino === 0n)
      && (current.mtimeNs !== expectedStat.mtimeNs || current.ctimeNs !== expectedStat.ctimeNs));
  if (current.isSymbolicLink() || !current.isDirectory() || identityChanged) {
    throw new Error(`${description} changed during release staging: ${directoryPath}`);
  }
}

async function validateInstallerPayload(candidate, stat, handle) {
  const { name, category } = candidate;
  const fail = (format) => { throw new Error(`Installer payload does not match ${format} structure: ${name}`); };
  const head = await readAt(handle, 0, MAX_HEADER_BYTES);
  const ext = path.extname(name).toLowerCase();
  const size = Number(stat.size);

  if (ext === '.deb') {
    if (head.length < 8 || head.subarray(0, 8).toString('ascii') !== '!<arch>\n') fail('Debian ar archive');
  } else if (ext === '.rpm') {
    if (head.length < 4 || !head.subarray(0, 4).equals(Buffer.from([0xed, 0xab, 0xee, 0xdb]))) fail('RPM');
  } else if (ext === '.appimage') {
    if (head.length < 20 || head.subarray(0, 4).toString('binary') !== '\x7fELF' || head[4] < 1 || head[4] > 2) fail('ELF AppImage');
    if ((stat.mode & 0o111n) === 0n) throw new Error(`AppImage must have at least one executable permission bit: ${name}`);
  } else if (ext === '.msi') {
    if (head.length < 8 || !head.subarray(0, 8).equals(Buffer.from([0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]))) fail('OLE compound MSI');
  } else if (ext === '.exe') {
    if (head.length < 64 || head.subarray(0, 2).toString('ascii') !== 'MZ') fail('Windows PE executable');
    const peOffset = head.readUInt32LE(0x3c);
    if (peOffset < 64 || peOffset + 4 > size) fail('Windows PE executable');
    const peSignature = await readAt(handle, peOffset, 4);
    if (peSignature.length !== 4 || peSignature.toString('binary') !== 'PE\0\0') fail('Windows PE executable');
  } else if (ext === '.dmg') {
    if (size < 512) fail('Apple UDIF disk image');
    const trailer = await readAt(handle, size - 512, 4);
    if (trailer.toString('ascii') !== 'koly') fail('Apple UDIF disk image');
  } else if (ext === '.pkg') {
    if (head.length < 8 || head.subarray(0, 4).toString('ascii') !== 'xar!') fail('XAR package');
    const headerSize = head.readUInt16BE(4);
    if (headerSize < 28 || headerSize > head.length) fail('XAR package');
  } else {
    throw new Error(`No installer payload validator is registered for ${category}/${ext}`);
  }
}

function isInside(parent, candidate) {
  const relative = path.relative(parent, candidate);
  return relative === '' || (!relative.startsWith(`..${path.sep}`) && relative !== '..' && !path.isAbsolute(relative));
}

async function requireRealDirectory(directory, description) {
  const stat = await fs.lstat(directory, { bigint: true }).catch((error) => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  if (!stat) return null;
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`${description} must be a real directory, not a symlink or special entry: ${directory}`);
  }
  return stat;
}

export async function openValidatedArtifact(candidate) {
  if (candidate.bundleRoot && candidate.bundleStat) {
    await assertDirectoryStillStable(candidate.bundleRoot, candidate.bundleStat, 'Tauri bundle root');
  }
  if (candidate.categoryDir && candidate.categoryStat) {
    await assertDirectoryStillStable(candidate.categoryDir, candidate.categoryStat, `Tauri ${candidate.category} package directory`);
  }
  const entryBefore = await fs.lstat(candidate.sourcePath, { bigint: true });
  if (entryBefore.isSymbolicLink() || !entryBefore.isFile()) {
    throw new Error(`Installer candidate must be a regular, non-symlink file: ${candidate.name}`);
  }
  if (entryBefore.size <= 0n || entryBefore.size > BigInt(Number.MAX_SAFE_INTEGER)) {
    throw new Error(`Installer candidate is empty or has an invalid size: ${candidate.name}`);
  }

  let handle;
  try {
    handle = await fs.open(candidate.sourcePath, SOURCE_OPEN_FLAGS);
    const openedStat = await handle.stat({ bigint: true });
    if (!sameObject(entryBefore, openedStat)) {
      throw new Error(`Installer candidate changed while it was being opened: ${candidate.name}`);
    }
    await assertPathStillReferencesFile(candidate.sourcePath, openedStat, 'Installer candidate');
    await validateInstallerPayload(candidate, openedStat, handle);
    const validatedSha256 = await hashHandle(handle, Number(openedStat.size));
    const stableStat = await handle.stat({ bigint: true });
    if (!sameContentVersion(openedStat, stableStat)) {
      throw new Error(`Installer candidate content changed during validation: ${candidate.name}`);
    }
    await assertPathStillReferencesFile(candidate.sourcePath, stableStat, 'Installer candidate');
    if (candidate.bundleRoot && candidate.bundleStat) {
      await assertDirectoryStillStable(candidate.bundleRoot, candidate.bundleStat, 'Tauri bundle root');
    }
    if (candidate.categoryDir && candidate.categoryStat) {
      await assertDirectoryStillStable(candidate.categoryDir, candidate.categoryStat, `Tauri ${candidate.category} package directory`);
    }
    return {
      ...candidate,
      handle,
      fingerprint: stableStat,
      validatedSha256,
      sizeBytes: Number(stableStat.size),
      mode: Number(stableStat.mode & 0o777n),
    };
  } catch (error) {
    await handle?.close().catch(() => {});
    throw error;
  }
}

async function writeAll(handle, buffer, position) {
  let offset = 0;
  while (offset < buffer.length) {
    const { bytesWritten } = await handle.write(buffer, offset, buffer.length - offset, position + offset);
    if (bytesWritten === 0) throw new Error('Release artifact staging destination stopped accepting data.');
    offset += bytesWritten;
  }
}

export async function copyValidatedArtifact(candidate, destination) {
  if (candidate.bundleRoot && candidate.bundleStat) {
    await assertDirectoryStillStable(candidate.bundleRoot, candidate.bundleStat, 'Tauri bundle root');
  }
  if (candidate.categoryDir && candidate.categoryStat) {
    await assertDirectoryStillStable(candidate.categoryDir, candidate.categoryStat, `Tauri ${candidate.category} package directory`);
  }
  const sourceBefore = await candidate.handle.stat({ bigint: true });
  if (!sameContentVersion(candidate.fingerprint, sourceBefore)) {
    throw new Error(`Installer candidate changed after validation: ${candidate.name}`);
  }
  await assertPathStillReferencesFile(candidate.sourcePath, candidate.fingerprint, 'Installer candidate');

  let destinationHandle;
  try {
    destinationHandle = await fs.open(destination, DESTINATION_OPEN_FLAGS, 0o600);
    const sourceHash = crypto.createHash('sha256');
    const buffer = Buffer.allocUnsafe(COPY_CHUNK_BYTES);
    let position = 0;
    while (position < candidate.sizeBytes) {
      const length = Math.min(buffer.length, candidate.sizeBytes - position);
      const { bytesRead } = await candidate.handle.read(buffer, 0, length, position);
      if (bytesRead === 0) throw new Error(`Installer candidate was truncated while staging: ${candidate.name}`);
      const chunk = buffer.subarray(0, bytesRead);
      sourceHash.update(chunk);
      await writeAll(destinationHandle, chunk, position);
      position += bytesRead;
    }

    const copiedDigest = sourceHash.digest('hex');
    const sourceAfter = await candidate.handle.stat({ bigint: true });
    if (!sameContentVersion(candidate.fingerprint, sourceAfter)) {
      throw new Error(`Installer candidate changed while it was being staged: ${candidate.name}`);
    }
    await assertPathStillReferencesFile(candidate.sourcePath, candidate.fingerprint, 'Installer candidate');
    if (candidate.bundleRoot && candidate.bundleStat) {
      await assertDirectoryStillStable(candidate.bundleRoot, candidate.bundleStat, 'Tauri bundle root');
    }
    if (candidate.categoryDir && candidate.categoryStat) {
      await assertDirectoryStillStable(candidate.categoryDir, candidate.categoryStat, `Tauri ${candidate.category} package directory`);
    }
    if (copiedDigest !== candidate.validatedSha256) {
      throw new Error(`Staged bytes differ from the exact installer payload validated earlier: ${candidate.name}`);
    }

    await destinationHandle.chmod(candidate.mode);
    await destinationHandle.sync();
    const destinationStat = await destinationHandle.stat({ bigint: true });
    if (!destinationStat.isFile() || destinationStat.size !== candidate.fingerprint.size) {
      throw new Error(`Staged artifact size verification failed for ${candidate.name}`);
    }
    const stagedDigest = await hashHandle(destinationHandle, candidate.sizeBytes);
    if (!SHA256_RE.test(stagedDigest) || stagedDigest !== candidate.validatedSha256) {
      throw new Error(`Staged artifact digest does not match the validated source for ${candidate.name}`);
    }
    await validateInstallerPayload(candidate, destinationStat, destinationHandle);
    const destinationPathStat = await fs.lstat(destination, { bigint: true });
    if (destinationPathStat.isSymbolicLink() || !sameObject(destinationPathStat, destinationStat)) {
      throw new Error(`Staged artifact path was replaced during verification: ${candidate.name}`);
    }
    return { sha256: stagedDigest, sizeBytes: Number(destinationStat.size) };
  } finally {
    await destinationHandle?.close().catch(() => {});
  }
}

async function syncDirectory(directoryPath) {
  if (process.platform === 'win32') return;
  const directoryHandle = await fs.open(directoryPath, constants.O_RDONLY);
  try {
    await directoryHandle.sync();
  } finally {
    await directoryHandle.close();
  }
}

/**
 * Copy the required platform installer formats into a clean, flat release stage.
 * Source descriptors remain open from validation through copying so path replacement
 * cannot redirect the copy to a different file after validation.
 */
export async function stageReleaseArtifacts({ platform, bundleDir, outputDir }) {
  const layout = RELEASE_PLATFORM_LAYOUT[platform];
  if (!layout) throw new Error(`Unsupported release platform: ${platform || '(missing)'}`);
  if (typeof bundleDir !== 'string' || !bundleDir.trim() || typeof outputDir !== 'string' || !outputDir.trim()) {
    throw new Error('Both --bundle-dir and --output-dir are required.');
  }

  const bundleRoot = path.resolve(bundleDir);
  const outputRoot = path.resolve(outputDir);
  const bundleStat = await requireRealDirectory(bundleRoot, 'Tauri bundle root');
  if (!bundleStat) throw new Error(`Tauri bundle root does not exist: ${bundleRoot}`);
  if (isInside(bundleRoot, outputRoot) || isInside(outputRoot, bundleRoot)) {
    throw new Error('Release staging output and source bundle must be disjoint directories.');
  }
  const existingOutput = await fs.lstat(outputRoot).catch((error) => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  if (existingOutput) throw new Error(`Release staging output already exists; refusing to reuse or overwrite it: ${outputRoot}`);

  const candidates = [];
  const openCandidates = [];
  const categoryCounts = new Map();
  let outputCreated = false;
  try {
    for (const category of layout) {
      const categoryDir = path.join(bundleRoot, category.directory);
      const categoryStat = await requireRealDirectory(categoryDir, `Tauri ${category.directory} package directory`);
      if (!categoryStat) {
        if (category.required) throw new Error(`Required ${platform} release package directory is missing: ${category.directory}`);
        continue;
      }

      const children = await fs.readdir(categoryDir, { withFileTypes: true });
      children.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
      let count = 0;
      for (const child of children) {
        if (path.extname(child.name).toLowerCase() !== category.extension) continue;
        const sourcePath = path.join(categoryDir, child.name);
        const candidate = await openValidatedArtifact({
          sourcePath,
          name: child.name,
          category: category.directory,
          bundleRoot,
          bundleStat,
          categoryDir,
          categoryStat,
        });
        openCandidates.push(candidate);
        candidates.push(candidate);
        count += 1;
      }
      categoryCounts.set(category.directory, count);
      if (category.required && count === 0) {
        throw new Error(`Required ${platform} installer format ${category.extension} is missing from ${category.directory}/.`);
      }
    }

    if (!candidates.length) throw new Error(`No installable ${platform} release artifacts were found.`);
    if (candidates.length > 64) throw new Error(`Release artifact count exceeds the safety limit of 64: ${candidates.length}`);
    const names = new Set();
    for (const candidate of candidates) {
      const key = candidate.name.toLocaleLowerCase('en-US');
      if (names.has(key)) throw new Error(`Installer filename collision in flat release staging directory: ${candidate.name}`);
      names.add(key);
    }

    await assertDirectoryStillStable(bundleRoot, bundleStat, 'Tauri bundle root');
    await fs.mkdir(outputRoot, { recursive: false });
    outputCreated = true;
    const artifactsDir = path.join(outputRoot, 'artifacts');
    try {
      await fs.mkdir(artifactsDir, { recursive: false });
      const artifactsDirStat = await fs.lstat(artifactsDir, { bigint: true });
      if (!artifactsDirStat.isDirectory() || artifactsDirStat.isSymbolicLink()) {
        throw new Error('Release artifact output must be a newly created real directory.');
      }
      const artifacts = [];
      for (const candidate of candidates) {
        await assertDirectoryStillStable(bundleRoot, bundleStat, 'Tauri bundle root');
        await assertDirectoryStillStable(candidate.categoryDir, candidate.categoryStat, `Tauri ${candidate.category} package directory`);
        const destination = path.join(artifactsDir, candidate.name);
        const staged = await copyValidatedArtifact(candidate, destination);
        artifacts.push({
          name: candidate.name,
          category: candidate.category,
          sizeBytes: staged.sizeBytes,
          sha256: staged.sha256,
        });
      }
      await syncDirectory(artifactsDir);
      await syncDirectory(outputRoot);
      artifacts.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
      return {
        platform,
        outputDir: outputRoot,
        artifactsDir,
        artifactCount: artifacts.length,
        artifacts,
        categoryCounts: Object.fromEntries(categoryCounts),
      };
    } catch (error) {
      await fs.rm(outputRoot, { recursive: true, force: true }).catch(() => {});
      outputCreated = false;
      throw error;
    }
  } finally {
    await Promise.all(openCandidates.map((candidate) => candidate.handle.close().catch(() => {})));
    if (outputCreated) await syncDirectory(path.dirname(outputRoot)).catch(() => {});
  }
}

function parseCli(args) {
  const options = {};
  const allowed = new Map([
    ['--platform', 'platform'],
    ['--bundle-dir', 'bundleDir'],
    ['--output-dir', 'outputDir'],
  ]);
  for (let index = 0; index < args.length; index += 1) {
    const flag = args[index];
    const key = allowed.get(flag);
    const value = args[index + 1];
    if (!key || !value || value.startsWith('--')) throw new Error(`Unknown or incomplete option: ${flag}`);
    options[key] = value;
    index += 1;
  }
  return options;
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : '';
if (invokedPath && invokedPath === fileURLToPath(import.meta.url)) {
  try {
    const result = await stageReleaseArtifacts(parseCli(process.argv.slice(2)));
    console.log(`Staged ${result.artifactCount} verified ${result.platform} installer(s) at ${result.artifactsDir}.`);
    for (const artifact of result.artifacts) console.log(`  ${artifact.name} (${artifact.sizeBytes} bytes, sha256:${artifact.sha256})`);
  } catch (error) {
    console.error(`Release artifact staging BLOCKED: ${error.message}`);
    process.exitCode = 1;
  }
}
