import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

const guardPath = path.join(process.cwd(), 'scripts/check-rust-first-architecture.mjs');

function writeFiles(root, files) {
  for (const [file, source] of Object.entries(files)) {
    const target = path.join(root, file);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, source);
  }
}

function git(root, args) {
  execFileSync('git', args, { cwd: root, stdio: 'ignore' });
}

function runGuard(baseline, changed) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'rust-first-guard-'));
  try {
    git(root, ['init', '-b', 'main']);
    git(root, ['config', 'user.email', 'guard-test@example.invalid']);
    git(root, ['config', 'user.name', 'Architecture guard test']);
    writeFiles(root, { 'docs/architecture/RUST_FIRST_MIGRATION.md': '# Policy\n', ...baseline });
    git(root, ['add', '-A']);
    git(root, ['commit', '-m', 'baseline']);
    writeFiles(root, changed);
    git(root, ['add', '-A']);
    git(root, ['commit', '-m', 'candidate']);
    return spawnSync(process.execPath, [guardPath], { cwd: root, encoding: 'utf8' });
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
}

test('comment-only JavaScript changes do not count as business logic', () => {
  const result = runGuard(
    { 'src/lib/domain/risk.js': 'export const risk = 1;\n' },
    { 'src/lib/domain/risk.js': '// Explains the risk boundary.\nexport const risk = 1;\n' },
  );
  assert.equal(result.status, 0, result.stderr);
});

test('type declarations and type annotations do not count as runtime logic', () => {
  const result = runGuard(
    { 'src/lib/domain/risk.ts': 'export function calculate(value) { return value; }\n' },
    { 'src/lib/domain/risk.ts': 'export interface RiskSnapshot { volume: number; }\nexport function calculate(value: number): number { return value; }\n' },
  );
  assert.equal(result.status, 0, result.stderr);
});

test('new executable JavaScript production logic is blocked', () => {
  const result = runGuard(
    { 'src/lib/domain/risk.js': 'export const risk = 1;\n' },
    { 'src/lib/domain/risk.js': 'export const risk = 1;\nexport function evaluate() { return 2; }\n' },
  );
  assert.equal(result.status, 1);
  assert.match(result.stderr, /executable production JavaScript/);
});

test('new executable TypeScript core logic is blocked', () => {
  const result = runGuard(
    { 'src/lib/domain/risk.ts': 'export const risk = 1;\n' },
    { 'src/lib/domain/risk.ts': 'export const risk = 1;\nexport function evaluate() { return risk * 2; }\n' },
  );
  assert.equal(result.status, 1);
  assert.match(result.stderr, /executable TypeScript core logic/);
});

test('typed UI presentation code is outside the core-logic gate', () => {
  const result = runGuard(
    {},
    { 'src/lib/presentation/format.ts': 'export function formatRisk(value: number) { return String(value); }\n' },
  );
  assert.equal(result.status, 0, result.stderr);
});

test('generated files and tests are excluded from production logic checks', () => {
  const result = runGuard(
    {},
    {
      'src/lib/domain/generated/schema.generated.js': 'export function generated() { return 1; }\n',
      'src/lib/domain/__tests__/risk.test.js': 'export function fixture() { return 1; }\n',
    },
  );
  assert.equal(result.status, 0, result.stderr);
});
