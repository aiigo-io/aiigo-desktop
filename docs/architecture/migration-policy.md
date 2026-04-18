# Migration Policy

## Purpose

This document defines how architecture changes may evolve the local wallet runtime without breaking existing user data.

Because this is a local-first wallet, migration policy is not a secondary deployment concern. It is part of the wallet’s trust model.

## Core Rule

No architectural change may assume a fresh install.

Every change must be safe for an existing local database that already contains:

- wallets
- secret backing records
- balances
- transactions
- dashboard rows
- portfolio history

## Migration Principles

### 1. Additive First

Prefer:

- new nullable columns
- new tables
- new metadata rows

Avoid:

- destructive renames
- in-place semantic rewrites
- silent drops of fields or tables

### 2. Lazy Backfill Over Eager Rewrite

If exact historical truth is not recoverable, do not invent it.

Examples:

- old rows with unknown freshness may initialize as `cached`
- old rows with unknown sync timestamps may remain null until next refresh
- derived dashboard state may be recomputed on next refresh rather than bulk migrated

### 3. Old State Must Remain Readable

Existing rows must continue to deserialize even when:

- new transaction lifecycle states are added
- new freshness fields are introduced
- new sync metadata is introduced

### 4. Compatibility Is Part Of Done

A phase is not complete if:

- an existing DB fails to open
- existing wallets disappear
- old transaction rows fail to parse
- dashboard or balance reads fail because new metadata is absent

## Migration Categories

### Category A: Schema Additions

Examples:

- new sync metadata columns
- new wallet-state metadata table
- new session state table

Policy:

- additive only
- safe defaults or nullability required

### Category B: Semantic Expansion

Examples:

- transaction lifecycle enum grows from three states to six
- freshness model becomes explicit

Policy:

- old rows map to safe legacy-compatible states
- unknown old values must not crash deserialization

### Category C: Secret Handling Changes

Examples:

- secret persistence routed through keystore abstraction
- stronger encryption introduced later

Policy:

- must preserve the ability to read existing secret backing
- may introduce re-write-on-next-use or staged migration
- must never silently orphan existing wallets

## Backfill Rules

### Wallet Rows

- existing wallet rows remain valid without re-import

### Balance Rows

- existing balance rows may be treated as cached until refreshed

### Price Rows

- old price-related values may be treated as stale or cached depending on known provenance

### Transaction Rows

- old lifecycle values must map safely into the new lifecycle vocabulary
- no historical transaction row should become unreadable because the status model expanded

### Derived Rows

- dashboard and portfolio rows may be lazily recomputed
- avoid bulk recomputation migrations unless absolutely necessary

## Rollback Rules

Rollback is allowed only if:

- schema changes are additive and older code can tolerate extra columns/tables
- or a forward-only migration has a documented recovery path

The architecture should prefer forward-safe changes over rollback-dependent changes.

In practice, this means:

- design migrations so a partially upgraded DB is still structurally readable
- keep migration steps small
- avoid coupling schema changes and broad semantic rewrites in one step

## Required Validation For Migration-Bearing Changes

Any plan or PR that changes persistence semantics must validate:

1. fresh database initialization still works
2. existing local database still opens
3. wallet list still loads
4. transaction history still loads
5. dashboard and balance reads still return usable values

## Relationship To Plans

Implementation plans may describe migration steps, but this document is the reusable policy that every plan must reference.

If a future plan proposes a migration that breaks these rules, that plan must explicitly justify the exception.
