# Release Artifact Attestation Pipeline — v0.9.6

## Goal and trust boundary

The v0.9.5 release evidence is an unsigned local hash inventory. It can detect byte drift when its expected manifest is trusted, but it cannot authenticate the publisher. v0.9.6 adds a signed CI provenance layer for distributable installers and publishes an SPDX Software Bill of Materials (SBOM). The signed attestation is generated with GitHub Artifact Attestations/Sigstore and uses short-lived OIDC-backed signing credentials; no long-lived signing key is stored in this repository.

## Release state machine

1. **Build** — run locked npm/Bun dependency installs, the complete source suite, AI Agent qualification, frontend build, Rust formatting/tests/Clippy, and Tauri desktop packaging.
2. **Stage** — inspect only platform-approved package output directories and copy distributable installer files into a new, flat staging directory. Linux requires DEB, RPM and AppImage; Windows requires MSI and NSIS EXE; macOS requires DMG and may include PKG. Raw app directories, detached signature files, and build intermediates are not promoted. Missing required formats or a non-empty/unsafe condition fails the release.
3. **Inventory** — generate release evidence over the exact staged installer files. The evidence directory is separate from the payload directory. A pre-upload verification ensures the payload hashes and inventory still match.
4. **SBOM** — After the release workflow confirms committed lockfiles and performs frozen dependency installs, Syft scans the source/dependency workspace into an SPDX JSON document. CI keeps the SBOM beside the staged payload.
5. **Attest** — `actions/attest@v4` creates one SLSA provenance attestation and one SBOM attestation for all staged installer subjects.
6. **Verify identity** — the GitHub CLI verifies each installer and fails unless its cryptographic provenance matches the expected repository, exact version tag, `.github/workflows/ci.yml` signer workflow, GitHub OIDC issuer, and non-self-hosted runner policy.
7. **Promote** — only the staged installers, their release evidence, and the SBOM are uploaded. The raw Tauri bundle directory is not a published release payload.

Every transition is fail-closed. Missing installers, an unexpected file in staging, missing attestation service permissions, a signer/ref mismatch, failed cryptographic verification, or a failed hash comparison prevents artifact upload.

## Consumer verification

Check out the exact release tag in a trusted environment, install and authenticate the official GitHub CLI (`gh`), then download the workflow artifact and unpack it so `artifacts/`, `evidence/`, and `sbom.spdx.json` remain siblings. Verify the local inventory:

```sh
npm run release:evidence:verify -- --bundle-dir ./artifacts --evidence-dir ./evidence
```

Verify cryptographic provenance for each installer:

```sh
npm run release:attestation:verify -- \
  --repo OWNER/REPOSITORY \
  --signer-workflow OWNER/REPOSITORY/.github/workflows/ci.yml \
  --source-ref refs/tags/v0.9.6 \
  --bundle-dir ./artifacts
```

The verifier refuses unknown files, directories and symlinks in the installer stage, and invokes `gh attestation verify` for every file and each required predicate with `--deny-self-hosted-runners` and the GitHub Actions OIDC issuer. A valid SLSA statement alone is not enough when the separate SPDX SBOM attestation is absent. It is not enough to verify one file in a multi-installer release.

## GitHub prerequisites

The workflow requires `contents: read`, `id-token: write`, `attestations: write`, and `artifact-metadata: write` in the tagged release job. GitHub documents Artifact Attestations as available for public repositories on current plans and for private/internal repositories on GitHub Enterprise Cloud; other private plan configurations may be unsupported. A service or permission failure blocks release rather than downgrading to unsigned hashes.


## Immutable action-reference policy

Release preflight also inspects every external `uses:` reference in the CI workflow. Release is blocked unless each external action is pinned to a reviewed, full 40-character commit SHA; a mutable tag such as `@v4`, a branch, or an abbreviated hash does not satisfy the rule. This v0.9.6 workspace currently retains some version-tag references because full commit IDs for every action could not be independently verified from the available offline Git metadata; they are intentionally reported as a release blocker rather than replaced with guessed hashes. Before production certification, review each action release and pin its official commit SHA. Upgrades should be reviewed explicitly and then update the SHA and regression fixtures together.

## Threat model and limits

- The signature authenticates the origin claims captured by GitHub/Sigstore and binds the digest to the attestation. The verifier constrains repository, ref and workflow identity; it does not certify source correctness.
- The SBOM is dependency inventory evidence, not a vulnerability-free certificate. Review its packages, licenses and scan findings.
- This pipeline does not yet sign/notarize each OS vendor package at the platform level (for example, Apple Developer ID notarization or Windows Authenticode). GitHub provenance is separate from OS trust prompts.
- The hash-chain inside a local SQLite database remains tamper-evident, not tamper-proof. A local administrator with control of both process and data storage may be able to rewrite the database and recompute local hashes. This release attestation protects installer provenance; it does not externally anchor every runtime state commit.
- A release is not production-certified until tagged CI actually passes on all three OS targets and the platform acceptance gates, not merely because the workflow contains the right steps.

## Test coverage

`npm run test:release-artifact-staging` exercises package completeness, format filtering, digest checking, symlink rejection, and collisions using temporary files. `npm run test:release-attestation` exercises CLI policy enforcement and proves that every staged installer is verified for both SLSA and SPDX predicates with the required identity constraints. A regression also proves that release is rejected if SLSA succeeds but the SPDX SBOM attestation is missing. These tests do not create a real Sigstore attestation; only the tagged GitHub runner can validate that service interaction.
