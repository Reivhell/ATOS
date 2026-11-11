# MT5 Bridge Hardening — 2026-10-31

## Change intent

This tranche hardens the existing MT5 telemetry boundary. It does not add an order route, simulated broker, mock account, or live-trading toggle. The repository version is intentionally unchanged: the work is a structural and validation correction, not a version-only release.

## Repository packaging correction

The Expert Advisor is now stored at the canonical repository path:

`MQL5/Experts/ATOS_Bridge.mq5`

The archive preserves this directory hierarchy so the file can be copied directly into the MetaTrader terminal data directory's `MQL5/Experts/` folder. Native setup metadata, UI installation instructions, operations documentation, and contract tests reference the same path.

## Native snapshot integrity changes

`src-tauri/src/mt5_bridge.rs` now rejects snapshots when:

- the terminal reports a disconnected state;
- the account leverage is zero or negative;
- aggregate trading permission differs from the conjunction of account, terminal, and EA permissions;
- the symbol trade mode is outside the documented MT5 enum range 0–4;
- point size or tick size is non-positive;
- minimum volume, maximum volume, and volume step do not form a positive, consistent broker contract.

These checks prevent malformed broker metadata from becoming a downstream risk input. A disabled trade mode remains valid telemetry; the validator does not require that the symbol be tradable.

## Regression coverage

Added native unit cases for disconnected terminals, inconsistent permission flags, invalid point/tick sizes, inconsistent volume constraints, and invalid leverage. Added repository contract checks for the canonical EA path and the new snapshot invariants.

## Validation results in this environment

- `npm test`: 281 tests passed, 0 failed.
- `npm run typecheck:layers`: passed.
- `npm run verify:repo`: passed with reproducibility warnings.
- `npm run release:preflight`: blocked by 9 release prerequisites.

Release blockers are unchanged in kind: absent `package-lock.json`, `bun.lock`, and `src-tauri/Cargo.lock`; absent `svelte-check`, Vite, and Tauri CLI executables; and unavailable Cargo, Rustfmt, and Clippy commands.

## Verification boundary

The JavaScript/Node regression suite and static repository checks passed. This environment does not provide a Rust compiler/toolchain, Cargo, MetaEditor, or an MT5 terminal. Therefore this tranche has **not** been Rust-compiled, Clippy-checked, MQL5-compiled, or exercised against a live broker terminal. Static contract tests are not substitutes for those checks.

The bridge remains read-only. Its loopback TCP transport is not encrypted in transit and must remain on the same trusted computer. No live order execution is authorized or implemented by this change.
