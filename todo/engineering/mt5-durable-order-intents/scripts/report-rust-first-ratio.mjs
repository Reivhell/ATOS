import fs from 'node:fs';
import path from 'node:path';

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
    if (entry.isDirectory()) return sourceFiles(relative, extensions);
    if (!entry.isFile() || !extensions.has(path.extname(entry.name))) return [];
    if (/(^|[\\/])(test|tests|__tests__)([\\/]|$)|\.test\.[^.]+$/i.test(relative)) return [];
    return [relative];
  });
}

function countLogicLines(file) {
  const lines = fs.readFileSync(path.join(root, file), 'utf8').split(/\r?\n/);
  let inBlockComment = false;
  let count = 0;
  for (const raw of lines) {
    const line = raw.trim();
    if (!line) continue;
    if (inBlockComment) {
      if (line.includes('*/')) inBlockComment = false;
      continue;
    }
    if (line.startsWith('/*') || line.startsWith('/**')) {
      if (!line.includes('*/')) inBlockComment = true;
      continue;
    }
    if (line.startsWith('//') || line.startsWith('*')) continue;
    count += 1;
  }
  return count;
}

const totals = roots.map(({ language, directories, extensions }) => {
  const files = directories.flatMap((directory) => sourceFiles(directory, extensions));
  return { language, files: files.length, lines: files.reduce((sum, file) => sum + countLogicLines(file), 0) };
});
const totalLines = totals.reduce((sum, item) => sum + item.lines, 0);
const rust = totals.find((item) => item.language === 'Rust');
const share = totalLines ? (rust.lines / totalLines) * 100 : 0;
console.log('Production core-logic source inventory (nonblank, non-comment lines; UI, adapters, tests, and tooling excluded)');
for (const item of totals) console.log(`${item.language}: ${item.lines} lines across ${item.files} files`);
console.log(`Rust share: ${share.toFixed(2)}%`);
console.log(`80% Rust target: ${share >= 80 ? 'reached' : 'not reached'}`);
if (process.argv.includes('--check') && share < 80) process.exitCode = 1;
