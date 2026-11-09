/**
 * Fail-closed policy for third-party GitHub Actions in release workflows.
 * Mutable refs (major tags, branches, or short SHAs) are not acceptable for a
 * release boundary; every external action must use a reviewed 40-character SHA.
 */
const FULL_COMMIT_SHA_RE = /^[a-f0-9]{40}$/i;

export function findUnpinnedActions(workflowText) {
  if (typeof workflowText !== 'string') throw new TypeError('Workflow text must be a string.');
  const results = [];
  for (const line of workflowText.split(/\r?\n/)) {
    const match = line.match(/^\s*(?:-\s*)?uses:\s*([^\s#]+)\s*(?:#.*)?$/);
    if (!match) continue;
    const target = match[1];
    // Local composite actions and Docker image references have different trust
    // boundaries. This workflow currently has neither; do not misclassify them.
    if (target.startsWith('./') || target.startsWith('docker://')) continue;
    const splitAt = target.lastIndexOf('@');
    if (splitAt <= 0) {
      results.push({ target, ref: null });
      continue;
    }
    const ref = target.slice(splitAt + 1);
    if (!FULL_COMMIT_SHA_RE.test(ref)) results.push({ target, ref });
  }
  return results;
}
