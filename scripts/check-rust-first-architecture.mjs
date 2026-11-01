import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = process.cwd();
const run = (args) => {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || `git ${args.join(' ')} failed`);
  }
  return result.stdout;
};

const policyPath = path.join(root, 'docs/architecture/RUST_FIRST_MIGRATION.md');
if (!fs.existsSync(policyPath)) {
  console.error('Rust-first architecture policy is missing.');
  process.exit(1);
}

let diff;
try {
  diff = run(['diff', '--unified=0', '--find-renames', 'HEAD^1', 'HEAD', '--', 'src', 'api/src']);
} catch (error) {
  console.error(`Cannot inspect the commit diff for Rust-first architecture enforcement: ${error.message}`);
  process.exit(1);
}

const violations = [];
let currentPath = '';
for (const line of diff.split('\n')) {
  if (line.startsWith('+++ b/')) {
    currentPath = line.slice(6);
    continue;
  }
  if (!line.startsWith('+') || line.startsWith('+++')) continue;

  if (/^(src|api\/src)\/.+\.js$/i.test(currentPath)) {
    violations.push(`${currentPath}: added production JavaScript is blocked; move business logic to Rust or keep the UI boundary in TypeScript.`);
  }
  if (/^src\/lib\/(domain|application|engines|infrastructure)\/.+\.tsx?$/i.test(currentPath)) {
    violations.push(`${currentPath}: added TypeScript core logic is blocked; implement the business rule in Rust.`);
  }
}

if (violations.length) {
  console.error(`Rust-first architecture gate failed (${violations.length} violation(s)):`);
  for (const violation of new Set(violations)) console.error(`- ${violation}`);
  process.exit(1);
}

console.log('Rust-first architecture gate passed: no new JavaScript production logic or TypeScript core logic was added.');
