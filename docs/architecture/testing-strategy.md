# Testing Strategy

## Purpose

This document defines the testing strategy for the wallet architecture. Its role is to turn “we should test this” into explicit test surfaces that future plans can reference without repeating the same reasoning.

## Testing Principles

- Test by architectural risk, not by file count.
- Prefer testing behavior at subsystem boundaries, not only helper functions.
- Every new system layer must define:
  - what can fail
  - what must remain true
  - how failure is surfaced
- Use the smallest test shape that can prove the architectural claim.

## Test Categories

The wallet architecture should be tested across four categories.

### 1. Unit Tests

Purpose:

- validate pure logic
- validate type conversion and status mapping
- validate policy and lifecycle rules

Examples:

- transaction status enum round-trip
- freshness state mapping
- signer operation policy checks
- chain-independent transformation logic

### 2. Integration Tests

Purpose:

- validate subsystem contracts
- validate persistence behavior
- validate command surfaces

Examples:

- wallet creation persists expected metadata
- keystore abstraction stores and retrieves expected secret kinds
- wallet-state command returns the expected typed shape
- sync engine updates persistence and metadata together

### 3. Lifecycle Tests

Purpose:

- validate multi-step state transitions
- prevent semantic drift across BTC and EVM

Examples:

- send -> broadcasted -> pending -> confirmed
- send -> broadcasted -> failed
- transaction replacement becomes `replaced`
- stale balance becomes fresh after sync
- dashboard state transitions from cached to refreshed

### 4. Failure-Path Tests

Purpose:

- validate timeout, retry, fallback, and partial-failure semantics
- prove the system fails honestly, not silently

Examples:

- BTC explorer primary fails and secondary succeeds
- EVM WSS fails and HTTP fallback succeeds
- CoinGecko fresh request fails and stale cache is used
- dashboard recompute runs with partial chain refresh failure
- BTC price command unavailable does not silently appear as fresh state

## Test Surface By Subsystem

### Security Boundary

Must test:

- secret persistence routing
- unlock session expiry
- export policy checks
- signer operation checks
- refusal behavior when locked

### Wallet State Model

Must test:

- freshness shape
- partial failure shape
- price state shape
- transaction lifecycle state shape
- compatibility defaults for old persisted rows

### Sync Engine

Must test:

- sync reason and target handling
- persistence updates after sync
- metadata updates after sync
- transaction/receipt refresh coordination
- explicit handling of deferred approval refresh if not yet implemented

### Transaction Lifecycle

Must test:

- BTC and EVM use the same lifecycle vocabulary
- broadcast does not equal confirmed
- pending and confirmed semantics are consistent
- replaced and dropped semantics do not regress

### UI Consumption Layer

Must test:

- dashboard no longer performs hidden consistency repair
- freshness state is visible in presentation components
- stale or fallback price state is distinguishable from fresh state
- transaction lifecycle labels render correctly

## Minimum Required Coverage For “Foundation Hardened”

The wallet cannot be considered “foundation hardened” unless all of the following are covered:

1. secret access boundary tests
2. wallet-state freshness tests
3. transaction lifecycle transition tests
4. at least one failure-path test per external dependency class:
   - Bitcoin explorer
   - EVM RPC / WSS
   - CoinGecko
   - swap-market integration where applicable

## Test Tooling Expectations

### Rust

Use Rust tests for:

- state types
- security logic
- sync engine behavior
- transaction lifecycle rules
- persistence-level integration checks where feasible

### Frontend

At minimum:

- `npm run build` must pass

When freshness and lifecycle UI surfaces are introduced, add component-level tests for:

- dashboard state display
- transaction status display
- fallback / unavailable state rendering

## Test Exit Criteria Per Phase

### Security Phase Exit

- security unit tests pass
- compile passes

### State Model Phase Exit

- freshness and partial-failure tests pass
- persistence compatibility path is covered

### Sync / Lifecycle Phase Exit

- lifecycle tests pass
- sync engine integration tests pass
- no chain-family-specific semantic drift remains

### UI Phase Exit

- build passes
- UI no longer hides freshness or fallback state

## What This Document Prevents

Without this document, plans tend to regress into:

- compile-check-only validation
- no explicit failure-path coverage
- no lifecycle semantic coverage
- no shared definition of what “done” means

This document exists to keep future implementation plans honest.
