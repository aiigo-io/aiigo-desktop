# Executable Wallet Runtime Blueprint

## Purpose

This is the live architecture note for the current wallet MVP. It defines the minimum runtime shape the repository should converge toward without a ground-up rewrite.

## Core Rules

1. The app stays local-first, Tauri-based, and SQLite-backed.
2. Bitcoin and EVM remain separate execution domains behind shared vocabulary.
3. UI read paths must not perform hidden refreshes or writes.
4. Refresh flows must be explicit and go through one sync boundary.
5. Secret material access must be isolated behind the security boundary.
6. Schema changes stay additive unless a task explicitly states otherwise.

## Runtime Boundaries

### `wallet/security/*`

Owns unlock session, signing authority, export policy, and secret custody access.

Must not own chain RPC, portfolio assembly, or refresh orchestration.

### `wallet/state/*`

Owns canonical UI-facing read models assembled from SQLite.

Must not own remote refresh, secret access, or transaction broadcast.

### `wallet/sync/*`

Owns refresh planning, refresh execution, freshness updates, and post-broadcast reconciliation.

Must not own UI formatting or secret export.

### `wallet/chain/*`

Owns chain adapter traits, provider error normalization, and source-health mapping.

Must not own persistence or unlock decisions.

### `wallet/bitcoin/*` and `wallet/evm/*`

Own chain-specific building, mapping, validation, and serialization.

Must not own IPC naming, unlock policy, or read-model orchestration.

## Command Surface

Use intent-revealing Tauri command names:

- `query_*`: read from local state only
- `refresh_*`: fetch or reconcile external state
- `mutate_*`: create, rename, delete, or otherwise change stored state
- `sign_*`: sign transactions or messages
- `export_*`: export mnemonic or private key material
- `session_*`: unlock, lock, or inspect signer session state

Rules:

1. `query_*` commands must be side-effect free.
2. `refresh_*` commands may update persistence and freshness metadata.
3. Signing and export commands must pass through the security boundary.
4. New Tauri commands must be registered in `src-tauri/src/lib.rs`.

## Data Model Direction

Keep existing wallet and transaction tables readable during rollout.

Additive schema work should support four needs:

1. secret custody metadata separated from wallet metadata
2. explicit sync freshness state
3. source health and partial-failure visibility
4. stable read projections for dashboard, portfolio, and wallet detail

Legacy rows with incomplete freshness metadata should default to cached semantics, not fresh semantics.

## Frontend Contract

The UI should converge on two explicit flow types:

1. query current local state
2. request refresh explicitly when newer remote data is needed

Practical frontend rules:

- centralize Tauri wallet calls under `src/lib/tauri.ts` or a dedicated wallet API wrapper
- avoid page-level hidden consistency repair
- expose freshness and partial-failure state in UI-facing types
- keep dashboard, wallet detail, and transaction views on the same freshness vocabulary

## Incremental Migration Order

1. Separate command entrypoints from domain logic.
2. Keep `security`, `state`, `sync`, and `chain` as the main backend seams.
3. Rename mixed-semantics commands toward `query_*` and `refresh_*` naming.
4. Additive-migrate schema for freshness, sync metadata, and projections.
5. Move frontend callers toward shared query and refresh helpers.

## Definition Of Done

The MVP architecture is in the intended shape when all of the following are true:

1. UI reads no longer trigger hidden refresh or mutation.
2. Sync-triggering behavior is explicit in both code and command names.
3. Secret material is never loaded outside the security boundary.
4. BTC and EVM surfaces share the same freshness and transaction lifecycle vocabulary.
5. Reviewer checks and tests align with this boundary model.