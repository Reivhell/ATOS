# Validation Report — 0.2.1

Date: 2026-10-27

## Changes

- Added `src/lib/strict-catalog-registry.js`, a code-owned 364-entry registry that pins catalog ID, category, name, and availability.
- Decision gate compares caller-provided catalog data against the trusted registry; agent payloads cannot self-promote `Catalog only` entries by changing availability.
- `Catalog only` entries must remain `unsupported` until implementation, tests, and a reviewed code-owned registry promotion exist. Unsupported entries block downstream handoff.
- Evaluated results require valid timestamp, evidence references, explicit result, and `lookAheadSafe: true`.
- Insufficient-data and unsupported states require meaningful reasons; intent must be explicitly `research` or `paper_trade`.
- Added tests for registry/fixture exact identity, payload spoofing, catalog tampering, assessment coverage, provenance, risk approval, and look-ahead safety.
- Added CI coverage test proving all 364 IDs are present exactly once across nine specialist knowledge packs.
- Updated package, Tauri, Rust package, and public-provider user-agent versions to 0.2.1.

## Tests executed

- Export tests: 5/5 passed.
- Dataset provenance tests: 2/2 passed.
- Technical analysis tests: 9/9 passed.
- Dashboard model tests: 4/4 passed.
- Market-data contract tests: 7/7 passed.
- Strict decision-gate tests: 12/12 passed.
- Agent knowledge-pack coverage tests: 2/2 passed.
- Total Node tests: 41/41 passed.
- `npm run verify:repo`: passed with missing-lockfile warnings.

## Remaining release blockers

- `package-lock.json` is absent; dependency resolution is not reproducible.
- `src-tauri/Cargo.lock` is absent; Rust dependency resolution is not pinned.
- `npm run build` cannot run successfully in the current environment because `svelte-check` is not installed and registry access previously failed with `EAI_AGAIN`.
- Rust tests, formatting, and Clippy cannot be executed here because `cargo` is not installed.
- Native Tauri builds, signed packaging, UI acceptance, and cross-platform persistence tests remain unverified.
- The decision gate is a tested contract module, not yet wired into a production AI-agent runtime. It intentionally blocks because 357 entries remain catalog-only.

## Release decision

**NOT PRODUCTION READY.** This revision closes the catalog self-promotion gap but does not implement all detectors or complete end-to-end runtime integration. Do not enable live trading. Release requires committed lockfiles, locked builds, Rust tests/Clippy, agent-runtime integration tests, and native packaging/acceptance across supported platforms.
