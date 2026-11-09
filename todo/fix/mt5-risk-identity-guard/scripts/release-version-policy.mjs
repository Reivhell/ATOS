/** Release tags are a public identity claim and must match every application manifest. */
const VERSION_TAG_RE = /^refs\/tags\/v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

export function releaseTagVersionError(releaseRef, applicationVersion) {
  if (typeof releaseRef !== 'string' || !releaseRef.startsWith('refs/tags/')) return null;
  if (!VERSION_TAG_RE.test(releaseRef)) {
    return `Release ref is not a valid version tag: ${releaseRef}`;
  }
  if (typeof applicationVersion !== 'string' || !applicationVersion) {
    return 'Cannot validate release tag because the application version is unavailable.';
  }
  const tagVersion = releaseRef.slice('refs/tags/v'.length);
  if (tagVersion !== applicationVersion) {
    return `Release tag v${tagVersion} does not match application manifest version ${applicationVersion}.`;
  }
  return null;
}
