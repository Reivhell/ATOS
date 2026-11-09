# Release artifact descriptor-bound staging — 2026-10-31

## Problem

The previous staging implementation validated installer signatures, hashed source files, and copied them through separate path opens. A concurrent file or directory replacement could redirect a later path operation to bytes other than the exact file observed during validation. Rehashing the destination reduced corruption risk but did not bind the earlier validation read to the bytes being staged.

## Implemented control flow

1. Reject non-regular, symbolic-link, empty, oversized-for-safe-offset, and unsupported installer candidates.
2. Open each candidate once with read-only, no-follow and non-blocking flags where the host runtime exposes those semantics.
3. Compare the directory entry with the opened descriptor and recheck the path after opening.
4. Validate the installer header from that descriptor, hash the complete descriptor contents, and compare descriptor identity and content metadata before and after validation.
5. Keep the descriptor open for the complete staging operation. Revalidate source file and directory identities before copying.
6. Stream bytes from the held descriptor into an exclusive, newly created destination descriptor. Compute the copy digest during that exact read and compare it with the digest captured during validation.
7. Recheck source descriptor metadata and source path identity after copying. Reject in-place modification, truncation, replacement, or source-directory substitution.
8. Apply only the source permission bits, flush the destination file, re-read and hash the destination through its open descriptor, and validate its format again.
9. Flush staged directories on POSIX systems. Any failed copy or verification removes the partial stage; output reuse remains prohibited.

## Regression coverage

The focused suite verifies valid platform staging, required-format enforcement, extension spoofing, AppImage executable permissions, symlink rejection, output reuse/overlap rejection, case-insensitive filename collisions, atomic source-path replacement, same-inode content mutation, truncation, and package-directory replacement after inventory discovery.

The source-level staging suite now passes **11/11 tests** on the current Linux runner.

## Frontend type safety

- The strict decision gate and work-plan orchestrator now have explicit checked contracts and pass strict TypeScript/JSDoc validation.
- The duplicate `Candle` type export was removed from the compatibility barrel collision without replacing the existing module entry points.
- `tsconfig.json` now includes all `src/**/*.js` modules with `allowJs` and `checkJs` enabled, rather than leaving most JavaScript application modules outside project type checking.
- The strict decision gate and orchestrator are now included in the direct layer typecheck command.
- Node type definitions are declared explicitly for Vite's `process.env` configuration usage. The dependency lockfiles have not been regenerated in this environment.

A standalone strict check of all 31 frontend JavaScript modules passes. The full project TypeScript command still cannot resolve the installed `@tauri-apps/api`, Svelte, Svelte component, Vite/plugin and Node type packages because dependencies are not installed in this workspace. The Svelte semantic check and production build have not run.

## Security and acceptance limits

This change closes the identified file-path reopen gap for the validated source file and adds identity checks around package directories. It is not a claim of complete cross-platform race freedom: Node's standard filesystem API does not provide a portable directory-descriptor-relative `openat`/`openat2` path for every target OS, and Windows directory durability semantics differ from POSIX directory `fsync`. A native platform-specific helper or equivalent hardened build-runner primitive is still needed to eliminate every parent-directory race under a hostile concurrent filesystem actor.

The format validators remain bounded signature/header plausibility checks. They do not yet parse every package table/member, validate all internal compressed payloads, verify vendor publisher signatures, verify Apple notarization, or prove that each package installs successfully. The GitHub attestation pipeline proves the build workflow's identity only when its verifier succeeds; it does not replace platform package-signature verification.

The staging code is source-tested on Linux only in this continuation. Windows and macOS runs must be executed by the matching hosted CI runners before treating the cross-platform path as accepted.
