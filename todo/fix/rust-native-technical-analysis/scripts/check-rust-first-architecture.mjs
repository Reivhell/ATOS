import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import ts from 'typescript';

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

function readRevision(revision, file) {
  const result = spawnSync('git', ['show', `${revision}:${file}`], { cwd: root, encoding: 'utf8' });
  return result.status === 0 ? result.stdout : '';
}

function runtimeAst(source, file, isTypeScript) {
  let executable = source;
  if (isTypeScript) {
    executable = ts.transpileModule(source, {
      fileName: file,
      compilerOptions: {
        target: ts.ScriptTarget.ES2022,
        module: ts.ModuleKind.ESNext,
        removeComments: true,
        verbatimModuleSyntax: true,
      },
    }).outputText.replace(/^\s*export\s*\{\s*\};?\s*$/gm, '');
  }
  const sourceFile = ts.createSourceFile(
    file,
    executable,
    ts.ScriptTarget.Latest,
    true,
    ts.ScriptKind.JS,
  );
  return ts.createPrinter({ removeComments: true }).printFile(sourceFile).trim();
}

function isExcluded(file) {
  return /(^|[\\/])(test|tests|__tests__|generated|gen)([\\/]|$)|\.test\.[^.]+$|\.(?:generated|gen)\.[^.]+$|\.d\.ts$/i.test(file);
}

function isProductionJavaScript(file) {
  return /^(?:src|api\/src)\/.+\.(?:js|jsx|mjs)$/i.test(file);
}

function isCoreTypeScript(file) {
  return /^src\/lib\/(?:domain|application|engines|infrastructure)\/.+\.tsx?$/i.test(file);
}

let changedFiles;
try {
  changedFiles = run(['diff', '--name-only', 'HEAD^', 'HEAD', '--', 'src', 'api/src'])
    .split('\n')
    .filter(Boolean);
} catch (error) {
  console.error(`Cannot inspect the parent commit for semantic Rust-first enforcement: ${error.message}`);
  process.exit(1);
}

const violations = [];
for (const file of changedFiles) {
  if (isExcluded(file) || (!isProductionJavaScript(file) && !isCoreTypeScript(file))) continue;
  const before = readRevision('HEAD^', file);
  const after = readRevision('HEAD', file);
  const isTypeScript = /\.tsx?$/i.test(file);
  if (runtimeAst(before, file, isTypeScript) !== runtimeAst(after, file, isTypeScript)) {
    violations.push(isProductionJavaScript(file)
      ? `${file}: executable production JavaScript changed; move deterministic business logic to Rust and keep presentation at the typed UI boundary.`
      : `${file}: executable TypeScript core logic changed; implement the business rule in Rust. Type-only and comment-only edits are allowed.`);
  }
}

if (violations.length) {
  console.error(`Rust-first architecture gate failed (${violations.length} semantic violation(s)):`);
  for (const violation of violations) console.error(`- ${violation}`);
  process.exit(1);
}

console.log('Rust-first architecture gate passed: no new executable JavaScript production logic or TypeScript core logic was introduced.');
