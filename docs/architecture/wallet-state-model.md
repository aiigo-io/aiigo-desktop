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

The repository now has one frozen balance contract and one wallet-specific EVM response surface.

The frozen Rust contract is:

- `raw_amount`
- `display_amount`
- `chain_id`
- `freshness`

The EVM wallet response additionally carries per-chain asset groups and wallet-level sync outcome.

Future extensions may still add richer balance metadata such as:

- `asset_id`
- `wallet_id`
- `balance_source`
- `balance_updated_at`
- `balance_status`
  - fresh
  - stale
  - unavailable
  - partial

### Current Problem

Bitcoin balance state now has explicit freshness semantics through `BalanceState`, and EVM wallet responses carry chain-level `FreshnessMetadata`. The remaining gap is that balance-related UI surfaces do not all consume the same frozen contract yet.

## Price State Model

### Current Model

- Prices are fetched from CoinGecko.
- Prices are cached in-process in the EVM price manager.
- Stablecoins are synthetically fixed at `1.0`.
- Bitcoin price is available through both `get_bitcoin_price` and `state_get_bitcoin_price_state`, each returning `PriceState`.

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

The application now surfaces explicit price state in the Bitcoin portfolio UI and dashboard-adjacent flows, including:

- whether they are cached
- whether they are synthetic
- whether the latest refresh failed

The remaining gap is that not every pricing consumer uses the same `PriceState` contract yet.

## Portfolio State Model

### Current Model

- Dashboard aggregates are stored in `dashboard_stats`.
- Portfolio history snapshots are stored in `portfolio_history`.
- EVM asset rows also persist per-asset USD values.
- `state_get_bitcoin_portfolio_state` exposes a frozen `PortfolioState` contract for BTC-only aggregation.
- `DashboardStats` now also carries freshness metadata for the dashboard aggregate.

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
- backend aggregation paths

This means portfolio state exists and freshness is explicit for BTC and dashboard views, but the repository still exposes more than one aggregate contract rather than one universal wallet-state facade.

## Transaction State Model

### Current Model

The current transaction lifecycle models:

- `broadcasted`
- `pending`
- `confirmed`
- `failed`
- `replaced`
- `dropped`

### Current Problems

- BTC and EVM now share the same six-state vocabulary through `LifecycleStatus` / `TransactionStatus`.
- Send paths insert `broadcasted` at send time and history/lifecycle refresh paths advance state later.
- The remaining gaps are around richer receipt-specific metadata and a single end-to-end lifecycle facade for every transaction surface.

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

Freshness is now an explicit backend contract.

The wallet state model explicitly supports:

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

The frozen freshness contract currently carries:

- `updated_at`
- `status`
- `failed_sources`

Some higher-level response types still add source-specific context outside this frozen struct.

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

### 1. State Contracts Are Still Split By Surface

The dashboard hook no longer reconstructs totals locally, but the repository still exposes multiple state shapes:

- frozen `BalanceState`, `PriceState`, and `PortfolioState`
- wallet-specific `EvmWalletBalancesResponse`
- dashboard-specific `DashboardStats`

### 2. EVM And BTC Do Not Yet Share One Read Facade

Bitcoin has dedicated state commands, while EVM freshness is surfaced through wallet-specific balance responses. The semantics are compatible, but the transport shape still differs.

### 3. Derived State And Cached State Are Still Exposed Through Multiple APIs

SQLite classification is documented in the architecture set, but command surfaces still expose BTC wallet state, EVM wallet balances, dashboard aggregates, and history through separate read paths instead of one consolidated wallet-state API.

## Open Questions

- Should SQLite be treated as the canonical local read model for wallet state, or only as a persistence cache behind a separate in-memory state engine?
- What freshness thresholds should exist for balances, prices, and portfolio totals?
- Should the frozen state contracts expand to cover EVM wallet and portfolio views directly?
- How should partial failure be represented across swap-market data as well as wallet-owned state?
- Which state should be computed only in Rust, and which state may be safely derived in the frontend?
