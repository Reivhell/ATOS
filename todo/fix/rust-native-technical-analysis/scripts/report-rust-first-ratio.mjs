import fs from 'node:fs';
import path from 'node:path';
import ts from 'typescript';

const root = process.cwd();
const roots = [
  { language: 'Rust', directories: ['src-tauri/src/application', 'src-tauri/src/domain', 'src-tauri/src/infrastructure', 'src-tauri/src/runtime'], extensions: new Set(['.rs']) },
  { language: 'TypeScript/JavaScript', directories: ['src/lib/application', 'src/lib/domain', 'src/lib/engines', 'src/lib/infrastructure'], extensions: new Set(['.ts', '.tsx', '.js']) },
];

function sourceFiles(directory, extensions) {
  const absolute = path.join(root, directory);
  if (!fs.existsSync(absolute)) return [];
  return fs.readdirSync(absolute, { withFileTypes: true }).flatMap((entry) => {
    const relative = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      if (/^(test|tests|__tests__|generated|gen)$/i.test(entry.name)) return [];
      return sourceFiles(relative, extensions);
    }
    if (!entry.isFile() || !extensions.has(path.extname(entry.name))) return [];
    if (/(^|[\\/])(test|tests|__tests__|generated|gen)([\\/]|$)|\.test\.[^.]+$|\.(?:generated|gen)\.[^.]+$|\.d\.ts$/i.test(relative)) return [];
    return [relative];
  });
}

function countRustLines(source) {
  let inBlockComment = false;
  return source.split(/\r?\n/).reduce((count, raw) => {
    const line = raw.trim();
    if (!line) return count;
    if (inBlockComment) {
      if (line.includes('*/')) inBlockComment = false;
      return count;
    }
    if (line.startsWith('/*')) {
      if (!line.includes('*/')) inBlockComment = true;
      return count;
    }
    return line.startsWith('//') ? count : count + 1;
  }, 0);
}

function countExecutableLines(file) {
  const source = fs.readFileSync(path.join(root, file), 'utf8');
  if (file.endsWith('.rs')) return countRustLines(source);
  const executable = file.endsWith('.js')
    ? source
    : ts.transpileModule(source, {
      fileName: file,
      compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext, removeComments: true, verbatimModuleSyntax: true },
    }).outputText.replace(/^\s*export\s*\{\s*\};?\s*$/gm, '');
  return executable.split(/\r?\n/).filter((line) => line.trim() && !line.trim().startsWith('//') && !line.trim().startsWith('/*') && !line.trim().startsWith('*')).length;
}

const totals = roots.map(({ language, directories, extensions }) => {
  const files = directories.flatMap((directory) => sourceFiles(directory, extensions));
  return { language, files: files.length, lines: files.reduce((sum, file) => sum + countExecutableLines(file), 0) };
});
const totalLines = totals.reduce((sum, item) => sum + item.lines, 0);
const rust = totals.find((item) => item.language === 'Rust');
const share = totalLines ? (rust.lines / totalLines) * 100 : 0;
console.log('Production core-logic inventory (executable source lines; UI, adapters, types, generated files, tests, docs, and tooling excluded)');
for (const item of totals) console.log(`${item.language}: ${item.lines} lines across ${item.files} files`);
console.log(`Rust share: ${share.toFixed(2)}%`);
console.log(`80% Rust target: ${share >= 80 ? 'reached' : 'not reached'}`);
if (process.argv.includes('--check') && share < 80) process.exitCode = 1;
