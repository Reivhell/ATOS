import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { copyValidatedArtifact, openValidatedArtifact, RELEASE_PLATFORM_LAYOUT, stageReleaseArtifacts } from './prepare-release-artifacts.mjs';

async function fixture(t) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'atos-release-stage-'));
  t.after(() => fs.rm(root, { recursive: true, force: true }));
  const bundle = path.join(root, 'bundle');
  await fs.mkdir(bundle);
  const output = path.join(root, 'stage');
  return { root, bundle, output };
}

function validPayload(filename) {
  const ext = path.extname(filename).toLowerCase();
  if (ext === '.deb') return Buffer.from('!<arch>\n' + 'valid-deb-payload');
  if (ext === '.rpm') return Buffer.concat([Buffer.from([0xed, 0xab, 0xee, 0xdb]), Buffer.alloc(128)]);
  if (ext === '.appimage') {
    const bytes = Buffer.alloc(128);
    Buffer.from([0x7f, 0x45, 0x4c, 0x46, 2, 1, 1]).copy(bytes);
    return bytes;
  }
  if (ext === '.msi') return Buffer.concat([Buffer.from([0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]), Buffer.alloc(128)]);
  if (ext === '.exe') {
    const bytes = Buffer.alloc(128);
    bytes.write('MZ', 0, 'ascii');
    bytes.writeUInt32LE(64, 0x3c);
    bytes.write('PE\0\0', 64, 'binary');
    return bytes;
  }
  if (ext === '.dmg') {
    const bytes = Buffer.alloc(1024);
    bytes.write('koly', 512, 'ascii');
    return bytes;
  }
  if (ext === '.pkg') {
    const bytes = Buffer.alloc(64);
    bytes.write('xar!', 0, 'ascii');
    bytes.writeUInt16BE(28, 4);
    return bytes;
  }
  return `installer:${filename}`;
}

async function writePackage(bundle, directory, filename, content) {
  const folder = path.join(bundle, directory);
  await fs.mkdir(folder, { recursive: true });
  const filePath = path.join(folder, filename);
  await fs.writeFile(filePath, content ?? validPayload(filename));
  if (path.extname(filename).toLowerCase() === '.appimage') await fs.chmod(filePath, 0o755);
}

async function populatePlatform(bundle, platform) {
  for (const category of RELEASE_PLATFORM_LAYOUT[platform]) {
    if (category.required) await writePackage(bundle, category.directory, `${platform}-package${category.extension}`);
  }
}

test('release staging only copies expected, non-empty installable files and records stable digests', async (t) => {
  const f = await fixture(t);
  await populatePlatform(f.bundle, 'linux');
  await writePackage(f.bundle, 'deb', 'package.deb.sig', 'detached signature must not be staged');
  await writePackage(f.bundle, 'deb', 'notes.txt', 'not an installer');
  await fs.mkdir(path.join(f.bundle, 'appimage', 'unrelated-bundle'), { recursive: true });
  await fs.writeFile(path.join(f.bundle, 'appimage', 'unrelated-bundle', 'helper'), 'build intermediate');

  const result = await stageReleaseArtifacts({ platform: 'linux', bundleDir: f.bundle, outputDir: f.output });
  assert.equal(result.artifactCount, 3);
  assert.deepEqual(result.artifacts.map((a) => a.name), ['linux-package.appimage', 'linux-package.deb', 'linux-package.rpm']);
  for (const artifact of result.artifacts) {
    const bytes = await fs.readFile(path.join(result.artifactsDir, artifact.name));
    assert.equal(crypto.createHash('sha256').update(bytes).digest('hex'), artifact.sha256);
    assert.equal(bytes.length, artifact.sizeBytes);
  }
  assert.deepEqual((await fs.readdir(result.artifactsDir)).sort(), ['linux-package.appimage', 'linux-package.deb', 'linux-package.rpm']);
});

test('release staging fails closed when any required installer format is missing', async (t) => {
  const f = await fixture(t);
  await writePackage(f.bundle, 'deb', 'only.deb');
  await writePackage(f.bundle, 'rpm', 'only.rpm');
  await assert.rejects(stageReleaseArtifacts({ platform: 'linux', bundleDir: f.bundle, outputDir: f.output }), /Required linux release package directory is missing: appimage|Required linux installer format \.appimage is missing/);
  await assert.rejects(fs.lstat(f.output), { code: 'ENOENT' }, 'failed validation must not leave a partial release stage');
});

test('release staging enforces platform-specific formats for Windows and macOS', async (t) => {
  const win = await fixture(t);
  await populatePlatform(win.bundle, 'windows');
  const winResult = await stageReleaseArtifacts({ platform: 'windows', bundleDir: win.bundle, outputDir: win.output });
  assert.deepEqual(winResult.artifacts.map((a) => path.extname(a.name).toLowerCase()).sort(), ['.exe', '.msi']);

  const mac = await fixture(t);
  await populatePlatform(mac.bundle, 'macos');
  await writePackage(mac.bundle, 'macos', 'optional.pkg');
  await fs.mkdir(path.join(mac.bundle, 'macos', 'QuantDinger.app', 'Contents'), { recursive: true });
  await fs.writeFile(path.join(mac.bundle, 'macos', 'QuantDinger.app', 'Contents', 'Info.plist'), 'app bundle internals are not published separately');
  const macResult = await stageReleaseArtifacts({ platform: 'macos', bundleDir: mac.bundle, outputDir: mac.output });
  assert.deepEqual(macResult.artifacts.map((a) => a.name).sort(), ['macos-package.dmg', 'optional.pkg']);
});

test('release staging rejects symlink installer candidates and filename collisions', async (t) => {
  const f = await fixture(t);
  await populatePlatform(f.bundle, 'windows');
  const real = path.join(f.bundle, 'msi', 'real.msi');
  await fs.writeFile(real, 'installer');
  await fs.symlink(real, path.join(f.bundle, 'msi', 'linked.msi')).catch((error) => {
    if (['EPERM', 'EACCES', 'ENOTSUP'].includes(error.code)) return t.skip('Symlink creation is unavailable on this runner.');
    throw error;
  });
  await assert.rejects(stageReleaseArtifacts({ platform: 'windows', bundleDir: f.bundle, outputDir: f.output }), /regular, non-symlink file/);

  const g = await fixture(t);
  await writePackage(g.bundle, 'msi', 'same.msi');
  await writePackage(g.bundle, 'msi', 'SAME.MSI');
  await writePackage(g.bundle, 'nsis', 'installer.exe');
  await assert.rejects(stageReleaseArtifacts({ platform: 'windows', bundleDir: g.bundle, outputDir: g.output }), /filename collision/i);
});


test('release staging rejects extension-spoofed installer payloads before creating output', async (t) => {
  const cases = [
    ['linux', 'deb', 'spoof.deb', 'plain text'],
    ['linux', 'rpm', 'spoof.rpm', 'plain text'],
    ['linux', 'appimage', 'spoof.appimage', Buffer.from('not ELF')],
    ['windows', 'msi', 'spoof.msi', 'plain text'],
    ['windows', 'nsis', 'spoof.exe', 'MZ but not a PE image'],
    ['macos', 'dmg', 'spoof.dmg', Buffer.alloc(1024)],
    ['macos', 'macos', 'spoof.pkg', 'plain text'],
  ];
  for (const [platform, directory, filename, payload] of cases) {
    const f = await fixture(t);
    await populatePlatform(f.bundle, platform);
    await writePackage(f.bundle, directory, filename, payload);
    if (filename.endsWith('.appimage')) await fs.chmod(path.join(f.bundle, directory, filename), 0o755);
    await assert.rejects(stageReleaseArtifacts({ platform, bundleDir: f.bundle, outputDir: f.output }), /does not match .* structure/);
    await assert.rejects(fs.lstat(f.output), { code: 'ENOENT' });
  }
});

test('release staging rejects AppImages without executable permissions', async (t) => {
  const f = await fixture(t);
  await populatePlatform(f.bundle, 'linux');
  await fs.chmod(path.join(f.bundle, 'appimage', 'linux-package.appimage'), 0o644);
  await assert.rejects(stageReleaseArtifacts({ platform: 'linux', bundleDir: f.bundle, outputDir: f.output }), /must have at least one executable permission bit/);
});

test('release staging rejects unsafe roots, overlapping directories, unknown platforms and output reuse', async (t) => {
  const f = await fixture(t);
  await populatePlatform(f.bundle, 'macos');
  await assert.rejects(stageReleaseArtifacts({ platform: 'android', bundleDir: f.bundle, outputDir: f.output }), /Unsupported release platform/);
  await assert.rejects(stageReleaseArtifacts({ platform: 'macos', bundleDir: f.bundle, outputDir: path.join(f.bundle, 'stage') }), /must be disjoint/);
  await fs.mkdir(f.output);
  await assert.rejects(stageReleaseArtifacts({ platform: 'macos', bundleDir: f.bundle, outputDir: f.output }), /already exists/);
  await assert.rejects(stageReleaseArtifacts({ platform: 'macos', bundleDir: path.join(f.root, 'missing'), outputDir: path.join(f.root, 'fresh') }), /does not exist/);
});


test('release staging refuses an atomic source-path replacement after descriptor validation', async (t) => {
  const f = await fixture(t);
  const source = path.join(f.root, 'candidate.deb');
  const replacement = path.join(f.root, 'replacement.deb');
  await fs.writeFile(source, validPayload('candidate.deb'));
  await fs.writeFile(replacement, validPayload('replacement.deb'));
  const opened = await openValidatedArtifact({ sourcePath: source, name: 'candidate.deb', category: 'deb' });
  try {
    await fs.rename(source, `${source}.original`);
    await fs.rename(replacement, source);
    await assert.rejects(copyValidatedArtifact(opened, path.join(f.root, 'staged.deb')), /changed after validation|replaced during release staging/);
    await assert.rejects(fs.lstat(path.join(f.root, 'staged.deb')), { code: 'ENOENT' });
  } finally {
    await opened.handle.close();
  }
});

test('release staging refuses same-inode content mutation after descriptor validation', async (t) => {
  const f = await fixture(t);
  const source = path.join(f.root, 'candidate.deb');
  await fs.writeFile(source, validPayload('candidate.deb'));
  const opened = await openValidatedArtifact({ sourcePath: source, name: 'candidate.deb', category: 'deb' });
  try {
    const changed = validPayload('candidate.deb');
    changed[changed.length - 1] ^= 0xff;
    await fs.writeFile(source, changed);
    await assert.rejects(copyValidatedArtifact(opened, path.join(f.root, 'staged.deb')), /changed after validation/);
    await assert.rejects(fs.lstat(path.join(f.root, 'staged.deb')), { code: 'ENOENT' });
  } finally {
    await opened.handle.close();
  }
});

test('release staging refuses truncation after descriptor validation', async (t) => {
  const f = await fixture(t);
  const source = path.join(f.root, 'candidate.deb');
  await fs.writeFile(source, validPayload('candidate.deb'));
  const opened = await openValidatedArtifact({ sourcePath: source, name: 'candidate.deb', category: 'deb' });
  try {
    await fs.truncate(source, 8);
    await assert.rejects(copyValidatedArtifact(opened, path.join(f.root, 'staged.deb')), /changed after validation/);
    await assert.rejects(fs.lstat(path.join(f.root, 'staged.deb')), { code: 'ENOENT' });
  } finally {
    await opened.handle.close();
  }
});

test('release staging rejects replacement of a package directory after inventory discovery', async (t) => {
  const f = await fixture(t);
  const categoryDir = path.join(f.bundle, 'deb');
  await fs.mkdir(categoryDir);
  const source = path.join(categoryDir, 'candidate.deb');
  await fs.writeFile(source, validPayload('candidate.deb'));
  const bundleStat = await fs.lstat(f.bundle, { bigint: true });
  const categoryStat = await fs.lstat(categoryDir, { bigint: true });
  await fs.rename(categoryDir, `${categoryDir}.original`);
  await fs.mkdir(categoryDir);
  await fs.writeFile(path.join(categoryDir, 'candidate.deb'), validPayload('candidate.deb'));

  await assert.rejects(openValidatedArtifact({
    sourcePath: path.join(categoryDir, 'candidate.deb'),
    name: 'candidate.deb',
    category: 'deb',
    bundleRoot: f.bundle,
    bundleStat,
    categoryDir,
    categoryStat,
  }), /package directory changed during release staging/);
});
