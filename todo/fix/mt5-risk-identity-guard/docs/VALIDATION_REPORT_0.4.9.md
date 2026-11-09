# Validation report — Agentic Trading OS v0.4.9

Date: 2026-10-27  
Status: **engineering candidate; not production-certified**

## Change under review

This increment replaces repeated historical-pivot scans in the research-only SMC liquidity-sweep and structure-break detectors with ordered price indexes. Price-range queries enumerate only levels in the current candle's candidate interval. The swing detector uses forward-sliding monotonic deques so left and right extrema windows are evaluated in linear time. A canonical IEEE-754 sort key normalizes signed zero. These detectors fail closed when the configured one-million-event output ceiling would be exceeded rather than returning a truncated result.

Causal confirmation rules, close-crossing intervals, repeated sweep observations, independent swing-high/low lifecycles, same-candle break batching, and deterministic output ordering are preserved as the intended contract. Outputs remain research observations only.

## Executed verification

| Check | Result | Boundary |
|---|---|---|
| `npm test` | **PASS — 14 groups, 90 passed, 0 failed** | Includes application-level Node tests and Rust source-contract tests; does not compile or execute Rust. |
| `npm run verify:repo` | **PASS with release-blocking warnings** | Static repository check only. |
| Independent rolling-window reference experiment | **211,200 comparisons, 0 mismatches** | Local model of the sliding-deque logic compared against a naive scan; not execution of the Rust implementation. |
| Indexed event-semantics reference experiment | **5,000 generated OHLC cases, 0 mismatches** | Local models of indexed vs. naive sweep/break semantics; not native Rust execution. |
| ZIP integrity and SHA-256 | **PASS** — exact archive passes `unzip -t`; SHA-256 verified with `sha256sum -c` after packaging | The checksum sidecar accompanies the delivered ZIP. |

## Blocked verification / release gates

- `npm run build` **fails** because `svelte-check` is not installed (`sh: 1: svelte-check: not found`). Frontend type-check and production bundling were not completed.
- `cargo`, `rustc`, and `rustfmt` are unavailable in this environment. The native crate has **not** been compiled; Rust unit tests, Clippy, and formatting checks have **not** been executed. Native test cases in source are test definitions, not passing test evidence.
- `package-lock.json` and `src-tauri/Cargo.lock` are absent. `verify:repo` therefore warns that frontend/Rust dependency resolution is not pinned. An attempt to generate the npm lockfile timed out and did not produce a verified lockfile.
- The SMC detectors remain separate research primitives and are not wired as proof of completed specialist-agent execution. Runtime LLM provider/agent dispatch is not configured. The 364-entry catalog still has 6 `Calculated`, 1 `Workflow`, and 357 `Catalog only` entries; catalog-only blockers must not be promoted by research output.
- Live order execution remains disabled.

## Release decision

**Do not label this version production-ready or release-certified yet.** The algorithmic source change and dependency-free test suite have evidence, but native Rust compilation/tests, frontend production build, deterministic dependency lockfiles, and broader runtime acceptance are mandatory outstanding gates. CI should run `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --all -- --check`, the frontend install/build, and lockfile-backed repository checks before certification.
