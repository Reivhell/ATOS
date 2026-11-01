import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { findUnpinnedActions } from './release-action-pin-policy.mjs';

const ROOT = process.cwd();

test('accepts reviewed full-length commit SHAs and ignores local action paths', () => {
  const workflow = [
    '      - uses: actions/checkout@0123456789abcdef0123456789abcdef01234567',
    '      - uses: ./.github/actions/local-check',
    '      - uses: docker://ghcr.io/example/action@sha256:abc',
  ].join('\n');
  assert.deepEqual(findUnpinnedActions(workflow), []);
});

test('rejects moving tags, branches and abbreviated hashes', () => {
  const workflow = [
    '      - uses: actions/checkout@v4',
    '      - uses: example/action@main',
    '      - uses: example/short-sha@0123456',
  ].join('\n');
  assert.deepEqual(findUnpinnedActions(workflow).map(({ target }) => target), [
    'actions/checkout@v4', 'example/action@main', 'example/short-sha@0123456',
  ]);
});

test('current release workflow pins every external action to a reviewed full commit SHA', () => {
  const workflow = fs.readFileSync(path.join(ROOT, '.github/workflows/ci.yml'), 'utf8');
  assert.deepEqual(findUnpinnedActions(workflow), []);
  const requiredPins = [
    'actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1',
    'actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9',
    'actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6',
    'Swatinem/rust-cache@6323deb102c322ba6fcbdcafc7e3dddab59af2b6',
    'dtolnay/rust-toolchain@4360b52568e2003a75bf9bc1d59f33a8e3fc893c',
    'oven-sh/setup-bun@0c5077e51419868618aeaa5fe8019c62421857d6',
    'anchore/sbom-action@66cbf4bc1f1c0d2edc94016e65bc221b6bb0ad6c',
  ];
  for (const pin of requiredPins) assert.ok(workflow.includes(`uses: ${pin}`), `missing reviewed action pin: ${pin}`);
});

test('the Rust compiler and formatter/linter components are pinned consistently for local builds and CI', () => {
  const workflow = fs.readFileSync(path.join(ROOT, '.github/workflows/ci.yml'), 'utf8');
  const toolchain = fs.readFileSync(path.join(ROOT, 'rust-toolchain.toml'), 'utf8');
  assert.match(toolchain, /^channel = "1\.99\.1"$/m);
  assert.match(toolchain, /components = \["rustfmt", "clippy"\]/);
  assert.equal((workflow.match(/toolchain: 1\.99\.1/g) ?? []).length, 2);
});


test('regular Bun and frontend CI jobs require committed frozen dependency lockfiles', () => {
  const workflow = fs.readFileSync(path.join(ROOT, '.github/workflows/ci.yml'), 'utf8');
  const job = (name, nextName) => {
    const start = workflow.indexOf(`\n  ${name}:`);
    const end = workflow.indexOf(`\n  ${nextName}:`, start + 1);
    assert.notEqual(start, -1, `missing ${name} job`);
    assert.notEqual(end, -1, `missing next job after ${name}`);
    return workflow.slice(start, end);
  };
  const bun = job('bun-compatibility', 'frontend');
  const frontend = job('frontend', 'rust');
  assert.match(bun, /run: bun install --frozen-lockfile/);
  assert.doesNotMatch(bun, /run: bun install\s*(?:\n|$)/);
  assert.match(frontend, /run: bun install --frozen-lockfile/);
  assert.doesNotMatch(frontend, /run: npm install/);
});
