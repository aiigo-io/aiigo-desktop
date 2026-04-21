# Integration Surfaces

## Purpose

This document describes the internal-to-external integration surfaces used by the wallet.

## Frontend To Tauri Command Surface

The current application uses Tauri IPC as the primary frontend-to-backend integration surface for wallet operations.

### Current Command Categories

- wallet creation and import
- wallet export
- wallet lookup and balance refresh
- transaction send and history fetch
- dashboard read and refresh
- state read commands for BTC balance, price, and portfolio
- security session commands for unlock and lock state
- swap-side transaction actions

### High-Level Concern

The command surface currently mixes:

- read-oriented commands
- secret-export commands
- signing commands

This is functional, but it is not yet expressed as a strongly separated permission model.

The important current nuance is that send, approve, and export commands are now unlock-gated through the security subsystem even though they still travel over the same Tauri IPC channel as read commands.

## Bitcoin External Dependencies

Bitcoin support currently depends on external explorer-style APIs and Bitcoin-specific transaction libraries.

### Balance And History Dependencies

- Blockstream address APIs
- Blockchain.info address APIs

### Transaction Dependencies

- Blockstream transaction and fee-related APIs
- Electrum / Bitcoin transaction stack used by the send flow

### Current Characteristics

- explorer-backed lookup
- timeout-based HTTP requests
- retry behavior in some read paths
- explorer-style finality interpretation rather than a local full node

## EVM RPC And Provider Surface

EVM support is built on configured chain adapters and provider abstractions.

### Current Chain Support

- Ethereum
- Arbitrum
- Optimism
- Polygon
- Binance Smart Chain
- Ethereum Sepolia

### Current Provider Model

The provider layer supports:

- HTTP RPC as baseline
- optional WSS pool
- WSS health checks
- fallback from WSS to HTTP
- per-chain env override with built-in public RPC defaults

### Current Architectural Role

This layer is responsible for:

- native balance lookup
- token balance lookup through contract calls
- gas estimation
- transaction broadcast
- receipt lookup

## Price Provider Surface

The wallet currently uses CoinGecko as the price provider for tracked symbols.

### Current Behavior

- symbol-to-CoinGecko mapping in Rust
- in-process cached prices
- stablecoin synthetic price handling
- periodic refresh in the background
- stale-cache fallback when fresh requests fail

### Current Architectural Concern

Price integration is already separated from chain balance integration, and the resulting freshness state is now surfaced to the UI on dashboard, BTC wallet, and EVM chain-balance surfaces. The remaining gap is that swap market data does not participate in the same freshness contract.

## Swap Provider Surface

Swap support currently relies on OpenOcean from the frontend side.

### Current OpenOcean Usage

- quote lookup
- swap quote lookup
- allowance lookup
- spender discovery
- gas price lookup

### Current Integration Shape

The swap UI uses OpenOcean directly for market-side information, but still uses Tauri commands for wallet-side approval and send actions.

This creates a split integration model:

- frontend-owned swap market integration
- backend-owned wallet authority and signing

## Persistence Surface

SQLite is the current local persistence surface for:

- wallet metadata
- secret material
- asset balances
- transaction records
- dashboard aggregates
- portfolio history

### Integration Concern

SQLite is currently used as both:

- a durable local store
- a cached read model
- a derived aggregate store

This makes the persistence surface powerful, but also means each integration must clearly specify whether it is writing:

- canonical local state
- cached external state
- derived state

## Future Web3 Wallet Surfaces

The following integration surfaces are not yet present, but are necessary for a full Web3 wallet:

- EIP-1193 provider interface
- WalletConnect transport and session handling
- dApp account exposure rules
- chain-switch request handling
- message signing and typed-data signing
- approval scope and simulation support

These are future surfaces, not current product guarantees.

## Session And Permission Boundaries

### Current Reality

The wallet currently has no generalized dApp session layer.

This means there is currently no explicit model for:

- which app is requesting access
- which accounts are exposed
- which chain context is active for a session
- which permissions are granted

### Current Partial Equivalent

The closest existing approximation is swap allowance and approval behavior, but this is still flow-specific rather than a general permission system.

Separately, local product-owned actions already use a security boundary with unlock TTL and per-operation authorization labels. That boundary is not a dApp session layer, but it is the boundary future Web3 session code must call into rather than bypass.

### Target Direction

The future architecture should treat session and permission boundaries as first-class integration concerns, not UI-only concerns.

## Failure Semantics Per Integration

Each external dependency already has different failure characteristics.

### Bitcoin Explorer Failure

- HTTP timeout
- explorer unavailability
- malformed or partial response
- fallback to secondary provider

### EVM Provider Failure

- WSS pool unavailable
- WSS query failure with HTTP fallback
- RPC timeout or provider-level error
- per-chain partial failure

### Price Failure

- request timeout
- HTTP error
- parse failure
- stale-cache fallback
- synthetic stablecoin fallback

### Swap Failure

- OpenOcean API errors
- quote unavailability
- allowance data absence
- gas fallback paths

## Timeouts Retries And Fallbacks

The current system already includes a patchwork of resilience behaviors.

### Bitcoin

- explorer requests use HTTP timeouts
- balance lookup retries across providers

### EVM

- provider layer can fall back from WSS to HTTP
- balance lookups use retry logic
- chain RPC URLs can come from env vars with default fallbacks

### Prices

- price fetch uses timeout
- retries on failed requests
- stale cache fallback
- stablecoin synthetic fallback

### Swap

- frontend fallback gas price defaults exist for some chains

### Architectural Gap

These resilience mechanisms exist, and wallet-owned surfaces now expose freshness and partial-failure metadata in several places. The remaining architectural gap is that swap market data and some history surfaces still do not share one unified health contract.

## Testing Strategy For Integrations

The architecture should eventually distinguish between:

- unit tests for mapping and parsing logic
- integration tests for provider behavior
- sandbox tests for signing and broadcasting
- failure-path tests for timeout and fallback logic

### Current Reality

The current codebase has only limited direct testing around these integrations, and much of the runtime behavior still depends on manual verification.

## Current Missing Integration Layers

From an architectural perspective, the most important missing integration layers are:

### 1. Web3 Session Integration

No WalletConnect or EIP-1193 surface exists yet.

### 2. Typed Data And Permissioned Signing Integration

The wallet can send transactions, but does not yet expose a general message-signing integration surface.

### 3. Unified Integration Health Model

The wallet now partially exposes a UI-facing model for:

- source health
- freshness
- fallback mode
- partial integration failure

That model is still incomplete across swap-market integrations and some history/update paths.

### 4. Separation Of Market Data And Authority Data

Swap already hints at this separation, but it is not yet generalized as an architecture-wide principle.
