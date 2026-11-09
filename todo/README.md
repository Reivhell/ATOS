# Archived branch work

These directories preserve complete repository snapshots from work branches that were not merged into the active `dev` source tree. They are stored here so the work can be continued locally without keeping separate working branches.

- `backup/dev-before-overwrite-2026-11-01/` — archive snapshot; preserve as historical recovery material.
- `engineering/mt5-durable-order-intents/` — MT5 durable order-intent work from the former engineering branch.
- `fix/mt5-risk-identity-guard/` — snapshot of the branch associated with merged PR #7.
- `fix/mt5-risk-ledger-checkpoint/` — snapshot of the branch associated with merged PR #8.
- `fix/rust-native-technical-analysis/` — Rust-native technical-analysis work from PR #9; not merged into the active source tree here.
- `migration/bun-runtime-primary/` — migration branch snapshot.

Each directory is a full source-tree snapshot at the recorded branch head, not a patch applied over the root of `dev`. The active `dev` files remain unchanged by these archived copies.
