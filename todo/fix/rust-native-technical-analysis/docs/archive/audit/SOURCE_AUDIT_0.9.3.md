# Source audit scope — Agentic Trading OS v0.9.3

## Scope inspected by automated source scan

- `src/`
- `src-tauri/src/`
- `api/src/`
- `scripts/`

Total: **90 code files, 18,221 lines** across JavaScript, MJS, CJS, TypeScript, Svelte, and Rust files.

## Pattern checks

| Check | Matches |
|---|---:|
| TODO/FIXME markers | 0 |
| `eval(...)` or `new Function(...)` | 0 |
| Explicit `.innerHTML =` assignments | 0 |
| Credential-adjacent logging patterns | 0 |

JavaScript/MJS/CJS parser checks passed for files under `src`, `api`, and `scripts`. The canonical Node test suite and layer typecheck passed.

## Interpretation and limits

This is a reproducible automated scan of the listed patterns, not a proof of absence of vulnerabilities and not a claim that every line was manually reviewed by a human. It does not scan compiled dependencies or replace SAST/DAST, dependency auditing, threat modeling, Rust compilation, desktop runtime acceptance, or cross-platform packaging.

Occurrences of terms such as `synthetic`, `fixture`, or `DEMO FIXTURE` in source are limited to user-facing statements that no synthetic data is supplied, migration logic that removes legacy persisted synthetic rows, input warnings, and regression tests. No browser fallback snapshot generator was found by the repository guardrails.
