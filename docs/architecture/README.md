# Architecture Documentation

This directory contains the architecture documentation set for `aiigo-desktop`.

## Audience

- Engineers working on wallet architecture
- Engineers extending Bitcoin or EVM support
- Engineers working on synchronization, state, or security
- Reviewers who need a high-level map before reading code

## Reading Guide

1. Read [system-overview.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/system-overview.md) first.
2. Read [wallet-state-model.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/wallet-state-model.md) to understand sources of truth and freshness semantics.
3. Read [security-model.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/security-model.md) before changing secret handling, signing, or export behavior.
4. Read [integration-surfaces.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/integration-surfaces.md) before changing RPC, explorer, price, or swap integrations.
5. Read [testing-strategy.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/testing-strategy.md) before writing or reviewing implementation plans.
6. Read [migration-policy.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/migration-policy.md) before changing SQLite-backed state or lifecycle semantics.
7. Use the appendix documents for current-vs-target reference material and diagrams.
8. Use ADRs to understand why important architectural decisions were made.

## Document Map

- [system-overview.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/system-overview.md)
- [wallet-state-model.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/wallet-state-model.md)
- [security-model.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/security-model.md)
- [integration-surfaces.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/integration-surfaces.md)
- [testing-strategy.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/testing-strategy.md)
- [migration-policy.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/migration-policy.md)
- [appendices/current-architecture.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/appendices/current-architecture.md)
- [appendices/target-architecture.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/appendices/target-architecture.md)
- [adr/README.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/adr/README.md)

## Source Of Truth Rules

- Code remains the source of truth for runtime behavior.
- These docs describe intended architecture and known gaps.
- When code and docs diverge, update one or both in the same change whenever feasible.

## Current Scope

This document set currently focuses on the wallet architecture:

- Bitcoin wallet flows
- EVM wallet flows
- portfolio valuation and dashboard aggregation
- local persistence
- future Web3 wallet capabilities

## Known Gaps In The Current Implementation

- Secret storage and signing authority boundaries are not fully hardened.
- Wallet state freshness and synchronization semantics are not yet unified.
- Web3 interaction surfaces such as EIP-1193 and WalletConnect are not yet present.

## How To Add New ADRs

1. Create a new numbered file under `docs/architecture/adr/`.
2. Follow the standard ADR template in [adr/README.md](/Users/hhx/work/aiigo/aiigo-desktop/docs/architecture/adr/README.md).
3. Link the ADR from the relevant architecture document if it changes architectural guidance.

## Update Policy

- Update `system-overview.md` when subsystem boundaries or runtime topology change.
- Update `wallet-state-model.md` when freshness semantics or source-of-truth rules change.
- Update `security-model.md` when secret handling, unlock flow, export flow, or signing flow changes.
- Update `integration-surfaces.md` when external dependencies or failure semantics change.
- Update `testing-strategy.md` when a new subsystem introduces a new failure or lifecycle surface.
- Update `migration-policy.md` when persistence, schema, compatibility, or rollback assumptions change.
