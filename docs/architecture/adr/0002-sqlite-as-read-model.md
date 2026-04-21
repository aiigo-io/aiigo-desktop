# ADR 0002: SQLite As Read Model

## Status

Accepted

## Context

The current implementation already uses SQLite for a broad mix of concerns:

- wallet metadata
- secret backing material
- EVM asset balance rows
- transaction records
- dashboard aggregates
- portfolio history snapshots

This is useful, but architecturally ambiguous. Without a clear decision, SQLite risks being interpreted in mutually incompatible ways:

- as the canonical business truth for everything
- as a disposable cache
- as a mixed operational store with no explicit contract

That ambiguity creates downstream problems:

- freshness semantics become unclear
- derived state and synchronized external state become conflated
- future migrations become harder
- implementers make different assumptions in different modules

## Decision

We treat SQLite as the application's **durable local read model and persistence layer**, but **not** as the sole owner of business truth.

More precisely:

1. SQLite is the durable local store for wallet metadata and local operational state.
2. SQLite stores synchronized external state for balances, transactions, and portfolio-related data so the application can function as a local-first wallet.
3. SQLite may store derived state such as dashboard aggregates and portfolio snapshots, but derived state remains semantically derived.
4. SQLite does **not** redefine external chain truth; chain adapters and synchronized updates remain the authority on how chain state is interpreted.
5. SQLite does **not** define secret-handling policy by itself; the security boundary does.

### Classification Rule

Data written into SQLite must be understood as one of three categories:

- `durable local state`
  wallet metadata, secret backing metadata, sync metadata
- `synchronized external state`
  balances, transaction records, receipts, approval-related state
- `derived state`
  dashboard stats, portfolio snapshots, allocation summaries

## Consequences

### Positive

- The wallet can remain local-first without pretending SQLite is the chain.
- Synchronization and freshness semantics can be layered on top of SQLite consistently.
- Derived state can be cached locally without confusing it with primary state.
- Future migrations can be reasoned about by data category instead of table name alone.

### Negative

- Engineers must explicitly classify new persisted fields rather than treating all DB rows as equivalent.
- Sync metadata and freshness semantics become mandatory, not optional.
- The persistence layer remains powerful and must be documented carefully to avoid semantic drift.

### Architectural Implications

- wallet-state facade must interpret SQLite rows, not blindly expose them
- sync engine must own how synchronized external state gets refreshed and marked stale
- dashboard and portfolio code must stop pretending derived rows are the same as source rows

### Current Runtime Anchor

The current repository already reflects this decision in several concrete ways:

- wallet metadata and secrets live in SQLite tables, but secret-handling policy lives in `wallet/security/*`
- synchronized external state is persisted in wallet balance and transaction tables, then interpreted through freshness-aware state contracts
- derived state is persisted in `dashboard_stats` and `portfolio_history`, and dashboard freshness is surfaced explicitly rather than reconstructed in the frontend

## Alternatives Considered

### 1. SQLite As Pure Cache

Rejected.

The application already depends on SQLite for durable local wallet behavior. Treating it as disposable would misdescribe reality and weaken local-first guarantees.

### 2. SQLite As Canonical Truth For Everything

Rejected.

This would erase the distinction between chain truth, cached truth, and derived truth, which is exactly the ambiguity the architecture needs to remove.

### 3. Replace SQLite With A Fully In-Memory State Engine

Rejected for the current stage.

That would add complexity without solving the actual problem, which is semantic discipline, not just storage technology.
