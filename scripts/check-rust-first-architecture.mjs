import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = process.cwd();
const run = (args) => {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || `git ${args.join(' ')} failed`);
  }
  return result.stdout.trim();
};

const policyPath = path.join(root, 'docs/architecture/RUST_FIRST_MIGRATION.md');
if (!fs.existsSync(policyPath)) {
  console.error('Rust-first architecture policy is missing.');
  process.exit(1);
}

let changed;
try {
  changed = run(['diff', '--name-status', '--find-renames', 'HEAD^1', 'HEAD'])
    .split('\n')
    .filter(Boolean)
    .map((line) => line.split('\t'));
} catch (error) {
  console.error(`Cannot inspect the commit diff for Rust-first architecture enforcement: ${error.message}`);
  process.exit(1);
}

const violations = [];
for (const entry of changed) {
  const status = entry[0];
  const file = entry.at(-1);
  if (!file) continue;
  const added = status === 'A' || status.startsWith('C');
  if (!added) continue;

  if (/^(src|api\/src)\/.+\.js$/i.test(file)) {
    violations.push(`${file}: new production JavaScript modules are not allowed; use Rust for business logic or TypeScript for UI/IPC.`);
  }
  if (/^src\/lib\/(domain|application|engines|infrastructure)\/.+\.tsx?$/i.test(file)) {
    violations.push(`${file}: new TypeScript modules in core logic layers are not allowed; implement the business rule in Rust.`);
  }
}

if (violations.length) {
  console.error(`Rust-first architecture gate failed (${violations.length} violation(s)):`);
  for (const violation of violations) console.error(`- ${violation}`);
  process.exit(1);
}

console.log('Rust-first architecture gate passed: no new JavaScript production modules or TypeScript core-logic modules were added.');
