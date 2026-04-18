# ADR 0003: Price Cache Strategy

## Status

Accepted

## Context

The current implementation fetches token prices from CoinGecko and maintains an in-process cache through `wallet/evm/price_manager.rs`. Stablecoins receive a synthetic price of `1.0`. A Bitcoin price helper (`get_bitcoin_price`) exists in Rust but is not registered in `lib.rs`, so the frontend silently substitutes a hardcoded fallback (`95000`) whenever the command throws.

This leaves the pricing subsystem in an ambiguous state:

- prices cross a process boundary without being classified by freshness
- synthetic stablecoin prices are indistinguishable from fetched prices
- fallback values reach the UI without provenance
- there is no explicit distinction between "fresh", "stale cache", "synthetic", and "unavailable"

Without an explicit policy, each consumer (dashboard, portfolio, swap) invents its own interpretation of what a price value means. Silent fallback bugs become invisible by design.

## Decision

We adopt the following price cache strategy.

### 1. Pricing Is An Owned Subsystem

Price handling belongs to the Pricing And Valuation layer defined in `target-architecture.md`. Dashboard, portfolio, and wallet-screen code must not fetch prices directly from external APIs.

### 2. Prices Are Synchronized External State

Under ADR-0002, cached prices are `synchronized external state`. Every price value returned across the backend boundary must carry:

- `price_usd`
- `price_source` (e.g. `coingecko`, `synthetic-stablecoin`)
- `price_updated_at`
- `price_status` ∈ { `fresh`, `stale`, `unavailable`, `synthetic` }

### 3. Stale Cache Is Allowed, But Must Be Labeled

When a refresh fails, the cache may serve the last known value, but the response must flag it as `stale`. Silent substitution is disallowed.

### 4. Synthetic Prices Are Not Fresh Prices

Stablecoin synthetic pegging must report `price_status = synthetic`. UI components are responsible for distinguishing synthetic vs fetched prices if the product design requires it.

### 5. Unavailable Is A First-Class State

If no price is available (no cache, no fetch, no synthetic rule), the response returns `price_status = unavailable` rather than a fallback number. Downstream consumers must handle this explicitly.

### 6. Bitcoin Price Follows The Same Model

The Bitcoin price command must be a registered backend command returning the same `PriceState` shape. Frontend-side hardcoded fallbacks are disallowed once the hardening plan completes.

### 7. Freshness Thresholds Belong To The Pricing Layer

The Pricing layer owns the thresholds that decide when `fresh` degrades to `stale`. These are not per-page decisions and must not be reinvented by each consumer.

## Consequences

### Positive

- The UI can honestly render stale, synthetic, and unavailable price states.
- Portfolio valuation becomes auditable: every value has a source and a freshness tag.
- Adding a secondary provider later fits the same model without reshaping consumers.
- Silent fallback bugs become architectural violations rather than accidents.

### Negative

- Every price consumer must handle non-fresh states rather than treating a number as always valid.
- Tauri command responses are slightly larger than a bare float.
- Stablecoin and fetched prices are no longer interchangeable primitives in UI code.

### Architectural Implications

- Pricing subsystem is the single source of price truth for the wallet.
- Wallet-state facade exposes price state as part of its UI-facing contract.
- Dashboard and portfolio aggregation consume tagged prices, not raw numbers.
- Portfolio valuation carries an inherited freshness that reflects its worst-graded price input.

## Alternatives Considered

### 1. Pure Frontend Price Fetching

Rejected.

Frontend fetching cannot guarantee consistent freshness semantics across Dashboard, Portfolio, and Swap. It also produces exactly the class of silent fallback this ADR is meant to eliminate.

### 2. No Cache, Always Live

Rejected.

Tying every render to a CoinGecko round-trip couples the wallet to external availability and degrades perceived performance. It also removes any path for offline-tolerant rendering.

### 3. Treat All Prices As Equally Fresh

Rejected.

That is the current implicit behavior and it is exactly what this ADR corrects. Conflating synthetic and fetched prices removes audit-ability from portfolio valuation.

### 4. Move Pricing Cache To An External Service

Rejected at the current stage.

ADR-0001 commits the wallet to local-first. The pricing cache belongs on the local device next to the rest of the wallet runtime state. A server-side cache may be reconsidered later if multi-device sync becomes a goal, but not as the baseline strategy.
