# Testing Strategy

## Purpose

This document defines the minimum reviewer-enforced test surfaces for the wallet MVP.

## Testing Rules

1. Test architectural risks, not file counts.
2. Prefer boundary behavior over helper-level coverage when tradeoffs are needed.
3. New runtime layers must define failure shape, invariants, and visibility.
4. Use the smallest test shape that proves the contract.

## Required Coverage

### Security boundary

Must cover:

- unlock session expiry
- refusal behavior while locked
- signer authorization checks
- export policy checks

### Wallet state model

Must cover:

- freshness status serialization
- partial-failure metadata shape
- price status serialization, including `Synthetic`
- balance state compatibility across Bitcoin and EVM
- portfolio aggregation degradation when any component is non-fresh
- legacy SQLite reads after additive migration

Rules for legacy compatibility:

1. Old rows must remain readable after migration.
2. Missing historical freshness metadata defaults to cached, not fresh.

### Sync boundary

Must cover:

- sync reason and target handling
- persistence updates after refresh
- freshness metadata updates after refresh
- partial-failure recording when external sources fail

### Transaction lifecycle

Must cover:

- shared BTC/EVM lifecycle vocabulary
- `broadcasted` is distinct from `confirmed`
- replacement and failure semantics do not collapse into success states

### UI consumption

Must cover when relevant UI surfaces change:

- freshness is visible, not hidden
- fallback or unavailable state is distinguishable from fresh state
- transaction lifecycle labels remain correct

## Minimum Validation Commands

- `npm run build`
- `bash scripts/check_task.sh phase5`

Add targeted Rust or frontend tests whenever a change touches security, state contracts, sync behavior, or lifecycle semantics.

## Review Gate

A wallet runtime change is not complete if it introduces any of the following:

1. hidden refresh inside a read path
2. fresh-looking state produced from unavailable inputs
3. legacy row breakage after additive migration
4. security-sensitive behavior without locked-state refusal coverage

## Why This Exists

Without this file, validation regresses toward compile-only checks. This document keeps review focused on wallet runtime truthfulness: freshness, lifecycle, migration safety, and locked-state behavior.