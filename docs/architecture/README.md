# Architecture Documentation

This directory contains the active architecture documentation set for the
current wallet MVP.

## Read This First

1. Read [docs/architecture/executable-wallet-runtime-blueprint.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/executable-wallet-runtime-blueprint.md) for the current runtime shape, module boundaries, command surface, migration strategy, and frontend hook guidance.
2. Read [docs/architecture/testing-strategy.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/testing-strategy.md) for reviewer-enforced test expectations and subsystem coverage guidance.

## Active Document Map

- [docs/architecture/executable-wallet-runtime-blueprint.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/executable-wallet-runtime-blueprint.md)
- [docs/architecture/testing-strategy.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/testing-strategy.md)

## Scope

These active docs are intentionally narrow. They exist to support the current
wallet MVP cleanup path:

- command/query/refresh boundaries
- minimal session-gated wallet security
- explicit state and freshness semantics
- additive SQLite migration rules
- reviewer-enforced testing expectations

## Source Of Truth Rules

- Code remains the source of truth for runtime behavior.
- The MVP blueprint is the primary architecture guide for current work.
