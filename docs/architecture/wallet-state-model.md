# Wallet State Model

## Purpose

This document defines the intended wallet state model, sources of truth, and freshness semantics.

## Canonical Domain Entities

The current and target wallet architectures should be reasoned about in terms of a small set of domain entities.

- `Wallet`
  A locally managed account container with metadata such as label, type, and address.
- `SecretMaterial`
  Mnemonic or private-key material associated with a wallet.
- `ChainBalance`
  A balance on a particular chain for a native asset or token.
- `AssetBalance`
  A tracked asset with raw quantity, display quantity, and optional valuation.
- `PriceQuote`
  A price observation for an asset, including provenance and freshness.
- `PortfolioSnapshot`
  A derived point-in-time aggregate valuation used for dashboard and history presentation.
- `TransactionRecord`
  A locally persisted record of a chain transaction and its interpreted status.
- `SessionOrApprovalState`
  Future state for dApp sessions, permissions, and approvals.

## Sources Of Truth

The current implementation already has multiple truth sources. The main problem is that they are not yet explicitly modeled as such.

### Current Source Categories

- `Local durable truth`
  Wallet metadata and secret material persisted in SQLite.
- `External chain truth`
  Bitcoin explorer results, EVM RPC balance lookups, and chain transaction status.
- `External pricing truth`
  CoinGecko price reads and price cache refreshes.
- `Derived local truth`
  Dashboard aggregates, USD values, and portfolio history snapshots written into SQLite.

### Intended Source Rules

- Secret material truth belongs to the key security layer.
- Balance truth belongs to chain adapters plus synchronized local cache.
- Price truth belongs to the pricing subsystem.
- Portfolio truth is always derived, never primary.
- UI truth should come from one explicit wallet state model rather than direct ad hoc recomputation.

## Wallet State Categories

The wallet state should be divided into the following categories:

- `Identity state`
  Wallet list, address, wallet type, and unlock status.
- `Balance state`
  BTC balance, EVM native balance, token balances, and their refresh times.
- `Price state`
  Current asset price, cached age, source, and fallback condition.
- `Portfolio state`
  Aggregate USD value, BTC equivalent, historical snapshots, and allocation.
- `Transaction state`
  Pending, confirmed, failed, replaced, dropped, confirmations, and receipt data.
- `Capability state`
  Future dApp permissions, approvals, active chain, and session scope.
- `Health state`
  Which subsystems are stale, partially failed, or unavailable.

## Balance State Model

### Current Model

- Bitcoin balance is persisted as a single numeric field per wallet in `bitcoin_wallets.balance`.
- EVM balances are persisted as rows in `evm_asset_balances` with:
  - raw balance string
  - display balance float
  - cached USD price
  - cached USD value

### Intended Balance Fields

The UI-facing balance model should eventually include:

- `asset_id`
- `wallet_id`
- `chain_id`
- `raw_amount`
- `display_amount`
- `balance_source`
- `balance_updated_at`
- `balance_status`
  - fresh
  - stale
  - failed
  - partial

### Current Problem

Balance data exists, but it does not yet have explicit freshness or failure semantics beyond ad hoc UI state.

## Price State Model

### Current Model

- Prices are fetched from CoinGecko.
- Prices are cached in-process in the EVM price manager.
- Stablecoins are synthetically fixed at `1.0`.
- Bitcoin price is separately requested from the frontend, but the command registration is currently inconsistent with that expectation.

### Intended Price Fields

- `asset_id`
- `price_usd`
- `price_source`
- `price_updated_at`
- `price_status`
  - fresh
  - stale
  - unavailable
  - synthetic

### Current Problem

The application currently uses price values as if they were normal product state, but does not yet surface:

- whether they are cached
- whether they are synthetic
- whether the latest refresh failed
- whether a fallback value is being shown

## Portfolio State Model

### Current Model

- Dashboard aggregates are stored in `dashboard_stats`.
- Portfolio history snapshots are stored in `portfolio_history`.
- EVM asset rows also persist per-asset USD values.

### Intended Portfolio Fields

- `portfolio_value_usd`
- `portfolio_value_btc`
- `portfolio_snapshot_at`
- `portfolio_sources`
- `portfolio_status`
  - fresh
  - stale
  - partial

### Current Problem

Portfolio state is currently derived from a mix of:

- wallet balances from SQLite
- token asset rows from SQLite
- in-process cached prices
- frontend reconciliation logic

This means portfolio state exists, but it is not yet produced by a single explicit valuation model with explicit freshness semantics.

## Transaction State Model

### Current Model

The current transaction enum only models:

- `pending`
- `confirmed`
- `failed`

### Current Problems

- Bitcoin sends are stored as `pending`.
- EVM sends are stored as `confirmed`.
- The application does not yet have one unified updater that advances transaction state after broadcast.
- There is no first-class concept for:
  - replaced
  - dropped
  - nonce conflict
  - receipt missing
  - finality depth

### Intended Transaction Fields

- `tx_hash`
- `chain_id`
- `tx_type`
- `status`
- `status_updated_at`
- `confirmations`
- `receipt_state`
- `broadcast_at`
- `finalized_at`
- `failure_reason`

## Approval And Session State Model

This state is largely missing today, but it should exist as a first-class domain when the wallet grows into a full Web3 wallet.

### Intended Future State

- active chain
- exposed accounts per dApp
- active dApp sessions
- granted permissions
- token approvals
- approval scope
  - one-time
  - bounded
  - unlimited
- typed data review state

### Current Reality

- Swap flows query allowance externally but do not expose a generalized approval or session state model.
- There is no EIP-1193 provider or WalletConnect session state in the current architecture.

## Freshness Semantics

Freshness is the missing glue between backend truth and user trust.

The wallet state model should explicitly support:

- `fresh`
  The value was synchronized recently enough to satisfy the relevant UI surface.
- `cached`
  The value comes from previously persisted or in-memory state.
- `stale`
  The value exists, but it is older than the current freshness threshold.
- `unavailable`
  The subsystem could not refresh the value.
- `partial`
  Some sources succeeded and others failed.

Every UI-facing aggregate should eventually carry:

- `updated_at`
- `status`
- `source_count`
- `failed_sources`

## Partial Failure Semantics

Partial failure should be treated as a first-class state rather than a logging detail.

Examples:

- one EVM chain refreshed while another failed
- prices refreshed for some symbols but not all
- dashboard stats were recomputed using stale values
- transaction history fetched for one chain but not another

The target state model should allow the UI to say:

- this section is partially refreshed
- these chains failed
- this value is derived from mixed freshness inputs

## Sync Triggers

The target wallet state model should be updated through explicit sync triggers:

- app startup
- wallet creation or import
- manual refresh
- transaction broadcast
- transaction confirmation
- periodic background refresh
- chain change
- dApp session events

Today, these triggers are spread across multiple components rather than coordinated by one sync engine.

## UI State Contract

The UI should not need to guess what is true. The wallet state model should eventually be the only supported input contract for wallet screens.

### UI Should Receive

- value
- source
- updated_at
- freshness status
- partial failure details
- actionable status for transactions

### UI Should Not Need To Do

- infer consistency by recomputing totals locally
- silently substitute fallback values
- guess whether a value is chain-fresh or DB-cached
- guess whether a transaction state is final

## Invariants

The target wallet state model should preserve the following invariants:

- Secret material is never treated as ordinary application state.
- Every displayed balance has a known chain scope and update timestamp.
- Every displayed price has a source and freshness state.
- Every displayed portfolio aggregate is explicitly derived.
- Every transaction has a lifecycle status that is monotonic or explicitly replaced.
- UI consistency is achieved through modeled state, not hidden frontend reconciliation.

## Known Violations In Current Code

### 1. Dashboard Consistency Is Partially Reconstructed In The Frontend

The dashboard hook recomputes total USD from allocation and infers BTC equivalent from previously returned values. This indicates the state contract is not yet explicit enough.

### 2. Bitcoin Price Fallback Is Silent

The Bitcoin frontend applies a default price fallback without surfacing that it is a fallback or stale value.

### 3. Transaction Status Semantics Diverge By Chain Type

Bitcoin send flow and EVM send flow do not currently use one consistent transaction lifecycle model.

### 4. Freshness Is Mostly Local UI State

Refresh timestamps are tracked in individual components, but not carried as a general backend state model.

### 5. Derived State And Cached State Are Mixed

SQLite currently stores both raw wallet state and derived dashboard state, but the state model does not yet classify them clearly for the UI.

## Open Questions

- Should SQLite be treated as the canonical local read model for wallet state, or only as a persistence cache behind a separate in-memory state engine?
- What freshness thresholds should exist for balances, prices, and portfolio totals?
- Should transaction lifecycle state be unified across BTC and EVM before any Web3 interaction surface is added?
- How should partial failure be represented in Tauri command responses?
- Which state should be computed only in Rust, and which state may be safely derived in the frontend?
