# Strict specialist dispatch receipts — v0.4.2

## Purpose

A specialist work-plan route or a populated assessment object is not proof that an AI specialist executed. The native strict gate now requires exactly one durable dispatch receipt for each of the nine trusted categories, bound to the same Rust-issued work plan. The current runtime has no configured AI-provider dispatcher; therefore ordinary completion attempts have missing receipts and must fail closed.

## Receipt contract

Each receipt records:

- the exact Rust-issued `workPlanId`;
- the trusted `agentId` and category mapping;
- SHA-256 of the canonical task input, including work plan and category knowledge pack;
- SHA-256 of the canonical, ID-sorted assessment output for the complete category;
- the number of unique covered concepts;
- the completion timestamp and receipt digest.

`persist_specialist_dispatch_receipt` is a crate-private native Rust function. It is deliberately not registered as a Tauri command and is not callable from browser/frontend code. A future provider adapter must call it only after an actual specialist invocation and after validating the complete category output. It is not implemented as an API for an LLM or caller to self-attest.

## Durable storage and integrity

SQLite schema version 3 stores receipts in `strict_specialist_dispatch_receipts`. Database constraints reject duplicate receipts for the same plan/agent and same plan/category. SQLite triggers prohibit ordinary update/delete. Each insert is committed in the same `BEGIN IMMEDIATE` transaction as a `specialist_receipt_recorded` append-only hash-chain event. On store open, the Rust code recomputes receipt digests and cross-checks every row with the audit event count and matching plan, category-to-agent routing, and timestamp. A mismatch aborts persistence and decision processing.

Before the strict gate can proceed, Rust recalculates the expected task hash and the expected category-output hash from the submitted assessment bundle. A missing or mismatched receipt becomes a blocking reason. All nine receipts must exist before the completion attempt consumes the plan; consumption is single-use and persisted. Receipts cannot be added to an already-consumed or expired plan.

## What this does not prove

- No provider dispatcher is configured in this release, so there is no real agent execution receipt in the current runtime. The intended result is `BLOCK`, not an assumed pass.
- The audit chain uses unkeyed SHA-256. It detects partial edits and inconsistent records but is not a digital signature and cannot authenticate the database against an attacker who can rewrite the entire database and chain head.
- A receipt is not proof that a trading concept is mathematically correct. The central gate still rejects unsupported/catalog-only concepts, insufficient market data, invalid provenance, failed risk preflight, and incomplete 364-entry coverage.
- No receipt or successful analysis authorizes live execution.

## Acceptance tests

Rust unit tests were added for the missing-nine-receipts block and for exact output binding plus append-only receipt storage. They must be run with `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` in CI; Node source-contract tests do not substitute for executing those Rust tests.
