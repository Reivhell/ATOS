# Release artifact evidence contract — v0.5.4

## Purpose

Each tagged multi-platform desktop build must publish a machine-readable inventory of its packaged files and an unsigned checksum map. This provides evidence that the uploaded artifact bundle matches the exact files enumerated immediately after Tauri packaging. It makes added, missing, changed, or unexpected output files detectable before artifacts are consumed downstream.

## Files produced

- `release-evidence/release-manifest.json`: schema version, product version, UTC generation time, full Git commit, clean working-tree status, build target, available toolchain versions, CI run identity, and a sorted inventory of every file or symbolic link under the package bundle.
- `release-evidence/checksums.json`: SHA-256 digest for every bundle inventory entry and for the exact bytes of `release-manifest.json`.

Regular-file digests are calculated from file bytes. Symbolic links are recorded without following them; the digest covers the link target string. Inventory paths are relative POSIX paths, sorted deterministically. Unsupported filesystem entry types, empty bundles, manifest version disagreement, dirty source trees, and inventory drift fail closed.

## Required release workflow

After `npm run tauri build`, CI invokes `npm run release:evidence` and then `npm run release:evidence:verify`. Both the distributable bundle and evidence files are uploaded together. The verification step re-enumerates the package and checks the complete inventory rather than checking only the files already listed in the manifest.

## Security properties and limitations

The evidence is **unsigned**. SHA-256 provides content-integrity comparison against a trusted copy of the manifest/checksum file; it does not prove publisher identity, defend against an attacker who can rewrite both package and evidence, replace code signing, or constitute a signed SLSA/in-toto provenance attestation. The Git commit and CI run fields are claims recorded by the build environment, not cryptographically signed assertions. Signing and verification policies remain an additional release prerequisite.

The generator requires a clean Git working tree and a full commit hash. Unit tests inject synthetic metadata so the hashing, inventory, and tamper-detection logic can run without a Git checkout or desktop toolchain. These Node tests do not replace building and testing the native app on each target platform.
