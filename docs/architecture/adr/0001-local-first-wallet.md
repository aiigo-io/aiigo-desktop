# ADR 0001: Local-First Wallet

## Status

Accepted

## Context

`aiigo-desktop` is a desktop wallet application, not a custodial platform and not a server-led wallet product. The current codebase already persists wallet metadata, secret backing material, balances, transactions, dashboard aggregates, and portfolio snapshots locally through SQLite.

At the same time, blockchain state and market data come from external integrations:

- Bitcoin explorer APIs
- EVM RPC / WSS providers
- CoinGecko
- OpenOcean

This means the application naturally sits in a hybrid world:

- local device owns the wallet runtime and user-facing state
- external systems provide chain truth and market truth

The architecture needs one explicit answer to the question: what kind of wallet is this supposed to be?

Without that answer, future work can drift toward:

- custodial assumptions
- cloud-synced product-state assumptions
- Web3-provider behavior bolted onto an unstable core

## Decision

We treat `aiigo-desktop` as a **local-first wallet**.

This means:

1. The client device is the primary runtime owner of wallet state.
2. The application persists its operational wallet state locally.
3. External services may supply chain data, price data, and swap market data, but they do not become the application's canonical product-state store.
4. Secret material and signing authority remain local concerns, not remote concerns.
5. Any future Web3 wallet capability must be layered on top of this local-first foundation rather than replacing it.

### Operational Meaning

- wallet creation, import, export, signing, and wallet-state presentation are local-first behaviors
- chain balances and receipts are synchronized into local state rather than treated as UI-only fetches
- portfolio views are derived locally from synchronized state

### Explicit Non-Goals Implied By This Decision

This ADR excludes the following as the primary product model:

- custodial wallet architecture
- server-owned source of truth for balances and transaction state
- remote-first session orchestration
- smart-account or account-abstraction orchestration as the current runtime baseline

## Consequences

### Positive

- The architecture can center around local trust boundaries, which is correct for self-custody.
- Wallet state, freshness, and transaction semantics can be modeled consistently without depending on a remote application backend.
- Future Web3 wallet capabilities can be layered onto a stable local core.
- Offline-tolerant or partially connected behavior remains possible.

### Negative

- Local migrations and on-disk compatibility become first-class concerns.
- SQLite and local runtime state require stronger discipline, because they are not disposable implementation details.
- Secret handling mistakes have immediate local blast radius.
- There is no easy escape hatch into “the backend will fix it later” thinking.

### Architectural Implications

- security boundary must be local and explicit
- wallet-state model must distinguish local truth, synchronized chain truth, and derived truth
- sync engine must be treated as a core wallet subsystem

## Alternatives Considered

### 1. Custodial Or Semi-Custodial Backend-Centered Wallet

Rejected.

This would conflict with the product direction and would invert the trust model of the current codebase.

### 2. Cloud-Synced State As Primary Product Truth

Rejected as the primary architecture.

Cloud sync may exist later as an optional convenience feature, but not as the system's architectural center.

### 3. Browser-Extension-Style Web3 Wallet As The Immediate Core Model

Rejected for the current stage.

The current application is not yet ready to treat dApp session handling as its primary architectural center. The local wallet foundation must be stabilized first.
