# Wallet Foundation Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the current local-first asset wallet into a reliable wallet foundation by hardening secret handling, introducing a unified wallet state model, centralizing sync behavior, and normalizing transaction lifecycle semantics.

**Architecture:** Keep the current Tauri + React + SQLite shape, but add the missing wallet system layers instead of rewriting the app. The end state is still local-first, but with an explicit security boundary, explicit freshness/state semantics, a shared sync engine, and a consistent transaction lifecycle across BTC and EVM.

**Tech Stack:** Tauri v2, Rust, SQLite via rusqlite, React 19, TypeScript, existing BTC explorer integrations, existing EVM RPC/WSS provider layer, CoinGecko price cache, OpenOcean swap API.

---

## Why This Plan Exists

This plan is not an architectural polish pass. Every phase maps to a currently reproducible path that causes user loss — secret disclosure, wrong portfolio value, or ambiguous transaction outcome. The table below pins each phase to the concrete harm it eliminates.

| Phase | Reproducible harm today | Evidence in code | What this phase closes |
|---|---|---|---|
| **1 Security Boundary** | A single frontend XSS, malicious browser extension, or stray invoke call reaches `bitcoin_export_mnemonic` / `evm_export_mnemonic` directly — SQLite plaintext is only the last line of defense. Logs may also capture raw secret strings. | `wallet/bitcoin/wallet.rs:51` plaintext TODO; `wallet/bitcoin/private_key.rs:68` same; `lib.rs` registers export commands at the same trust level as read commands | Attack surface collapses from "any page can call" to "must hold a valid unlock session + matching `SignerOperation`"; secrets can no longer appear in logs |
| **2 Wallet State Model** | Users make real sell/hold decisions against a fabricated portfolio value. Bitcoin price is silently hardcoded to `95000` whenever the backend call fails. This is not a display bug; it is decision-time deception. | `src/pages/Portfolio/components/BitcoinAssets.tsx:111-122` sets `setBtcPrice(95000)`; `get_bitcoin_price` exists in Rust but is not registered in `invoke_handler!` so the frontend call always throws | UI can distinguish `fresh` / `stale` / `synthetic` / `unavailable`; silent fallback becomes an architectural violation rather than a coding shortcut |
| **3 Sync Engine + Chain Adapter + Lifecycle** | EVM sends are inserted as `confirmed` at broadcast time while BTC sends are inserted as `pending`. Users see "confirmed" while the transaction is still in mempool, then resend → double-spend or wasted gas. Each new chain family duplicates this ambiguity. | `wallet/evm/transaction.rs:555` inserts `TransactionStatus::Confirmed`; `wallet/bitcoin/transaction.rs:515` inserts `TransactionStatus::Pending`; same send, two meanings | BTC and EVM share one 6-state lifecycle with explicit finality rules; adding a chain no longer rewrites "what does sent mean" |
| **4 UI Consumption** | Dashboard quietly recomputes total USD from allocation ratios when backend aggregates are inconsistent — the user sees a plausible number manufactured by the frontend, not an honest error. | `src/pages/Dashboard/hooks/useDashboardData.ts` reconciles allocation to produce total | UI stops repairing backend data on the user's behalf; inconsistencies become visible instead of hidden |
| **5 Documentation Alignment** | Without this phase, the next implementer reads stale docs and either duplicates past work or introduces decisions that contradict code. | — | docs become a live mirror of code vocabulary, not a historical snapshot |

A phase is justified only if removing its delivery leaves one of the harms above in place. No phase in this plan exists for abstraction reasons alone.

---

## Acceptance Gates (Per-Phase Zero-Drift Contract)

Every phase must clear six gates before its PR can merge. The gates exist because reviewing prose intent is not a reliable way to catch "code drifted from plan" — the gates make drift mechanically detectable.

### Gate 1: Module Directory Diff

- Ground truth: the `New Rust Modules` list in this plan.
- Check: `ls src-tauri/src/wallet/{security,state,sync,chain}` output must match the plan list exactly. Missing or extra modules fail the gate.

### Gate 2: Type Field Parity

- Ground truth: the Rust struct/enum skeletons embedded in Phase 2's Acceptance Criteria (`FreshnessStatus`, `FreshnessMetadata`, `PriceStatus`, `PriceState`, `BalanceState`, `PortfolioState`) and Phase 3's 6-state transaction lifecycle.
- Check: field names and enum variants in the implementation must be byte-equivalent to the plan code blocks. Recommended mechanism: extract the plan's Rust code blocks as a compile-time fixture and fail build if the real types diverge.

### Gate 3: ADR-Bound Enum Sets

- ADR-0003 pins `PriceStatus` to exactly four variants: `Fresh`, `Stale`, `Unavailable`, `Synthetic`.
- Phase 3 pins the transaction lifecycle to exactly six states: `broadcasted`, `pending`, `confirmed`, `failed`, `replaced`, `dropped`.
- Check: `rg "PriceStatus::" src-tauri/src/` and equivalent for transaction status must produce exactly that set, deduped. Extra or missing variants fail the gate.

### Gate 4: Tauri Command Registration Diff

- Ground truth: each phase's `Acceptance Criteria` lists commands that must be registered, removed, or renamed.
- Check: a Rust unit test iterates the `invoke_handler!` macro in `lib.rs` and diffs against the expected set. Example concrete case this gate would have caught: `get_bitcoin_price` existing in Rust but not being wired into `invoke_handler!`.

### Gate 5: Milestone Meaning Walkthrough

- Each `Delivery Milestone` has a `Meaning:` block and, for Milestone A, an `Explicitly not meaning:` block.
- Check: every line in `Meaning:` must correspond to a reproducible demonstration in the PR. Examples:
  - Milestone A "unlock is enforced" → live demo: call `bitcoin_export_mnemonic` with no unlock session, expect a structured denial and verify no secret byte is read from DB.
  - Milestone D "`get_bitcoin_price` registered" → live demo: `invoke('get_bitcoin_price')` returns a `PriceState`, not a "command not found" error.
- Approximate demonstrations are not acceptable; each line is a binary pass/fail.

### Gate 6: Docs ↔ Code Drift Scan

A script (to be landed as part of Phase 5, but runnable earlier) enforces:

```bash
# 6a. every wallet module path mentioned in docs/ must exist on disk
grep -roE 'wallet/(security|state|sync|chain)/[a-z_]+' docs/architecture/ | \
  check_each_against_filesystem

# 6b. every type name promised in docs must be defined in Rust
for type in FreshnessStatus FreshnessMetadata PriceStatus PriceState \
            BalanceState PortfolioState; do
  rg "pub (struct|enum) $type" src-tauri/src/ >/dev/null || fail "$type missing"
done

# 6c. commands registered in invoke_handler! must be referenced in current-architecture.md
compare_invoke_handler_to_docs
```

Run on every PR. Any red line blocks merge.

### Per-Phase PR Checklist

Every phase's pull request description must include this checklist with all boxes checked before merge:

- [ ] Gate 1: module directory matches `New Rust Modules` list
- [ ] Gate 2: new type fields and variants are byte-equivalent to the plan code blocks
- [ ] Gate 3: ADR-bound enum sets match exactly
- [ ] Gate 4: `invoke_handler!` registration diff is empty
- [ ] Gate 5: each `Meaning:` line has a reproducible demonstration linked in the PR
- [ ] Gate 6: docs ↔ code drift scan passes
- [ ] Testing-strategy must-test list for this phase's subsystem is satisfied

A phase is not "done" when code compiles. A phase is done when all six gates pass. Calling a phase complete without the checklist misrepresents the delivered risk posture.

---

## Final Target State

When this plan is complete, the wallet should be in the following state:

### 1. Secret Handling Is Behind A Dedicated Security Boundary

- Wallet creation, import, export, and signing no longer read and write secrets directly from domain modules.
- A dedicated `wallet/security/*` subsystem owns secret access, unlock state, export policy, and signing permission checks.
- Export and signing are treated as distinct operations with different risk levels.

### 2. Wallet State Has Explicit Freshness And Failure Semantics

- Balances, prices, portfolio totals, and transaction states carry explicit metadata.
- The backend can distinguish:
  - fresh
  - cached
  - stale
  - unavailable
  - partial
- The frontend consumes this modeled state instead of reconstructing consistency heuristically.

### 3. Refresh Behavior Is Centralized

- A `wallet/sync/*` subsystem owns refresh entry points for:
  - Bitcoin balance sync
  - EVM balance sync
  - dashboard recompute
  - transaction status refresh
- Page-level refresh logic becomes a consumer of the sync engine rather than a source of sync truth.

### 4. Transaction Lifecycle Is Consistent Across BTC And EVM

- BTC and EVM use one shared lifecycle vocabulary.
- A send action is no longer interpreted differently depending on chain type.
- The wallet can expose richer transaction state to the UI.

### 5. The UI Stops Pretending Consistency

- No silent BTC price fallback that looks like a fresh price.
- No dashboard-side recomputation to patch backend inconsistency.
- No ambiguous transaction state presentation after broadcast.

---

## What This Plan Does Not Do

This plan is intentionally limited.

It does **not** implement:

- WalletConnect
- EIP-1193 provider support
- typed-data signing
- approval simulation
- hardware wallet support
- full encrypted keystore rollout

Those are follow-on plans. This plan only lays the foundation they would need.

---

## Migration And Compatibility Strategy

This codebase is local-first and may already have live SQLite data on disk. Any schema change introduced by this plan must preserve existing wallets and transaction history.

### Migration Rules

- Use additive schema changes only in this plan.
- Do not drop or rename existing tables in place.
- Prefer:
  - new nullable columns with safe defaults
  - new metadata tables
  - lazy backfill over destructive migration

### Backfill Policy

- Existing wallet rows must remain valid without requiring re-import.
- Existing transaction rows must remain readable even if new lifecycle states are introduced later.
- Freshness metadata may initialize to:
  - `cached` for pre-existing rows with unknown provenance
  - null timestamps where exact historical sync time is unknowable
- Derived dashboard rows may be recomputed lazily on first refresh rather than migrated eagerly.

### Acceptance Constraint

At the end of every migration-bearing phase:

- an existing local database must still open successfully
- wallet lists must still load
- transaction history must still deserialize
- dashboard and balance queries must not fail on missing backfilled values

---

## Files And Modules Affected

### New Rust Modules

- `src-tauri/src/wallet/security/mod.rs`
- `src-tauri/src/wallet/security/types.rs`
- `src-tauri/src/wallet/security/keystore.rs`
- `src-tauri/src/wallet/security/session.rs`
- `src-tauri/src/wallet/security/log_sanitize.rs`
- `src-tauri/src/wallet/security/commands.rs`
- `src-tauri/src/wallet/state/mod.rs`
- `src-tauri/src/wallet/state/types.rs`
- `src-tauri/src/wallet/state/commands.rs`
- `src-tauri/src/wallet/sync/mod.rs`
- `src-tauri/src/wallet/sync/types.rs`
- `src-tauri/src/wallet/sync/engine.rs`
- `src-tauri/src/wallet/chain/mod.rs`
- `src-tauri/src/wallet/chain/traits.rs`

### Existing Rust Files To Modify

- `src-tauri/src/lib.rs`
- `src-tauri/src/wallet/mod.rs`
- `src-tauri/src/db.rs`
- `src-tauri/src/dashboard/commands.rs`
- `src-tauri/src/wallet/types.rs`
- `src-tauri/src/wallet/bitcoin/mnemonic.rs`
- `src-tauri/src/wallet/bitcoin/wallet.rs`
- `src-tauri/src/wallet/bitcoin/private_key.rs`
- `src-tauri/src/wallet/bitcoin/commands.rs`
- `src-tauri/src/wallet/bitcoin/transaction.rs`
- `src-tauri/src/wallet/evm/mnemonic.rs`
- `src-tauri/src/wallet/evm/wallet.rs`
- `src-tauri/src/wallet/evm/private_key.rs`
- `src-tauri/src/wallet/evm/commands.rs`
- `src-tauri/src/wallet/evm/transaction.rs`
- `src-tauri/src/wallet/evm/price.rs`
- `src-tauri/src/wallet/evm/price_manager.rs`
- `src-tauri/src/wallet/transaction_types.rs`
- `src-tauri/src/wallet/transaction_commands.rs`

### Existing Frontend Files To Modify

- `src/pages/Dashboard/hooks/useDashboardData.ts`
- `src/pages/Portfolio/components/BitcoinAssets.tsx`
- `src/pages/Portfolio/components/EvmAssets.tsx`
- `src/pages/Transactions/index.tsx`
- `src/pages/Swap/hooks/useSwap.ts`

### Documentation To Update

- `docs/architecture/security-model.md`
- `docs/architecture/wallet-state-model.md`
- `docs/architecture/integration-surfaces.md`
- `docs/architecture/appendices/current-architecture.md`
- `docs/architecture/adr/0002-sqlite-as-read-model.md`
- `docs/architecture/adr/0004-web3-session-boundary.md`

---

## Phase Overview

```text
Phase 1 -> Build security boundary
Phase 2 -> Define wallet state model and sync metadata
Phase 3 -> Centralize sync and normalize transaction lifecycle
Phase 4 -> Make the UI consume explicit wallet state
Phase 5 -> Align docs and close the loop
```

---

## Phase 1: Security Boundary

### Target State After This Phase

- Secret reads and writes are routed through a `wallet/security/*` subsystem.
- The app has a first-class unlock model, even if still minimal.
- Export and signing checks stop being scattered through domain modules.
- Mnemonic-based creation and import paths are included in the same security boundary.
- Existing BTC/EVM wallet behavior still works, but security control has a single entry point.

### Acceptance Criteria

- New `wallet/security/*` modules exist and compile.
- `lib.rs` registers security-related commands.
- BTC and EVM wallet code no longer directly perform secret read/write as their primary model; direct DB secret reads outside `wallet/security/*` are removed.
- Mnemonic creation/import flows no longer bypass the security boundary when persisting secret material.
- Signing and export operations **must enforce** unlock state and operation type. Invocations of `bitcoin_export_*`, `evm_export_*`, send, and approve paths without a valid unlock session return a structured error and perform no secret read. "Can check" is not sufficient.
- A log-sanitization helper exists (`wallet/security/log_sanitize.rs`) and is used at every logging site in `wallet/security/*`, `wallet/bitcoin/*`, and `wallet/evm/*`. No log line or error surface contains raw mnemonic, private-key, or unlock-token material. A repo-wide check (grep gate or dedicated test) enforces this.
- `cargo check --manifest-path src-tauri/Cargo.toml` passes.
- New Rust unit tests for the security subsystem pass (see `docs/architecture/testing-strategy.md` → Security Boundary must-test list).

### Key Implementation Logic

- Introduce a `keystore` abstraction first, without trying to solve all encryption concerns immediately. At-rest encryption is **deferred** to a follow-on plan; Phase 1 only relocates secret access behind an interface.
- Introduce a `session` abstraction that models lock/unlock state and TTL.
- Introduce `SignerOperation` or equivalent so the system can distinguish:
  - send
  - approve
  - export mnemonic
  - export private key
- Move domain modules to call the security subsystem instead of raw DB secret helpers.
- Include both mnemonic-based and private-key-based entry points so the security boundary closes across all wallet creation/import flows.
- Introduce a log-sanitization helper in `wallet/security/log_sanitize.rs`. Replace existing `println!`, `eprintln!`, and logging macro sites inside `wallet/security/*`, `wallet/bitcoin/*`, and `wallet/evm/*` with sanitized calls so secret-bearing strings are never emitted raw.
- Ship the minimal frontend unlock step alongside enforcement. An enforcement boundary without a UI path to obtain an unlock session would break existing export and send flows; the unlock UI is part of Phase 1's scope.

### Files In Scope

- Create: `src-tauri/src/wallet/security/*`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/wallet/mod.rs`
- Modify: `src-tauri/src/wallet/types.rs`
- Modify: `src-tauri/src/wallet/bitcoin/mnemonic.rs`
- Modify: `src-tauri/src/wallet/bitcoin/wallet.rs`
- Modify: `src-tauri/src/wallet/bitcoin/private_key.rs`
- Modify: `src-tauri/src/wallet/bitcoin/transaction.rs`
- Modify: `src-tauri/src/wallet/evm/mnemonic.rs`
- Modify: `src-tauri/src/wallet/evm/wallet.rs`
- Modify: `src-tauri/src/wallet/evm/private_key.rs`
- Modify: `src-tauri/src/wallet/evm/transaction.rs`

### Verification

- `cargo check --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml`
- `cargo test --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml security:: -- --nocapture`

### Risks

- Over-designing the keystore before the abstraction exists.
- Accidentally breaking current import/export flows while introducing unlock checks.
- Enforcement landing before the unlock UI is wired, making previously-working export/send commands fail with no user recovery path.
- Treating "Security Boundary Extracted" as equivalent to "encrypted at rest"; it is not, and downstream communication must be explicit about this.

---

## Phase 2: Wallet State Model And Sync Metadata

### Target State After This Phase

- The backend has first-class types for freshness, partial failure, and wallet-state metadata.
- SQLite can persist sync metadata or equivalent timestamps.
- The backend can start telling the frontend whether balance and price data are fresh, cached, stale, unavailable, or partial.

### Acceptance Criteria

- New `wallet/state/*` modules exist and compile.
- Freshness and partial-failure types are defined and tested.
- Persistence exists for sync-related timestamps or metadata.
- A small Tauri command can expose state metadata to the frontend.
- Price freshness and fallback state are represented in backend state metadata, not only in frontend local state.
- Backend wallet-state command responses have an explicit typed shape for freshness, partial failure, and price state so frontend integration is deterministic in later phases.
- The following types are defined in `wallet/state/types.rs` (or re-exported from it) and are treated as stable contracts for Phase 4 consumption. Field names and variants must not change in Phase 4:

```rust
pub enum FreshnessStatus {
    Fresh,
    Cached,
    Stale,
    Unavailable,
    Partial,
}

pub struct FreshnessMetadata {
    pub status: FreshnessStatus,
    pub updated_at: Option<i64>,       // unix seconds; None if unknown
    pub failed_sources: Vec<String>,   // e.g. ["ethereum", "coingecko"]
}

pub enum PriceStatus {
    Fresh,
    Stale,
    Unavailable,
    Synthetic,                         // see ADR-0003
}

pub struct PriceState {
    pub price_usd: Option<f64>,
    pub price_source: Option<String>,
    pub price_updated_at: Option<i64>,
    pub status: PriceStatus,
}

pub struct BalanceState {
    pub raw_amount: String,
    pub display_amount: f64,
    pub chain_id: Option<String>,
    pub freshness: FreshnessMetadata,
}

pub struct PortfolioState {
    pub value_usd: Option<f64>,
    pub value_btc: Option<f64>,
    pub freshness: FreshnessMetadata,
}
```

- Testing-strategy minimums apply: see `docs/architecture/testing-strategy.md` → Wallet State Model must-test list (shape tests, partial-failure tests, legacy-row compatibility tests).

### Key Implementation Logic

- Define types before wiring behavior.
- Keep the first state model small:
  - freshness status
  - updated_at
  - failed_sources
  - per-surface metadata for balances/prices/portfolio
- Extend persistence only enough to track current state age and recent failures.
- Do not try to solve full dApp session state in this phase.
- Use additive SQLite changes only.
- Treat unknown historical freshness as `cached` rather than inventing false precision.
- Include pricing metadata in the first state model so later UI work can replace silent BTC price fallback with explicit price state.
- Define the backend response shape now, even if the frontend does not consume it until Phase 4.

### Files In Scope

- Create: `src-tauri/src/wallet/state/*`
- Modify: `src-tauri/src/wallet/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/wallet/evm/price.rs`
- Modify: `src-tauri/src/wallet/evm/price_manager.rs`

### Verification

- `cargo test --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml wallet::state -- --nocapture`
- `cargo check --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml`

### Risks

- Making the model too broad before its first consumers exist.
- Confusing raw persisted data with derived portfolio state.
- Introducing schema changes without a safe compatibility path for existing local installs.

---

## Phase 3: Sync Engine And Transaction Lifecycle

### Target State After This Phase

- A shared `wallet/sync/*` subsystem owns the main refresh entry points for:
  - balances
  - dashboard recompute
  - transaction status / receipt refresh
  - history refresh coordination
  - approval state refresh where applicable
- A `ChainAdapter` trait (in `wallet/chain/*`) defines the normalized chain-facing surface required by sync and transaction lifecycle code. BTC and EVM wallet modules implement it. This honors the Chain Adapters subsystem listed in `docs/architecture/appendices/target-architecture.md`.
- BTC and EVM transactions follow one lifecycle vocabulary.
- Send flows no longer diverge semantically by chain family.

### Acceptance Criteria

- `wallet/sync/*` modules exist and are used by existing refresh commands.
- `wallet/chain/*` defines a `ChainAdapter` trait. BTC (`wallet/bitcoin/*`) and EVM (`wallet/evm/*`) each provide an implementation. The sync engine depends on the trait rather than on concrete chain modules.
- `bitcoin_get_wallet_with_balance`, `evm_get_wallet_with_balances`, and `refresh_dashboard_stats` use shared sync logic and the ChainAdapter trait.
- transaction and receipt refresh paths are routed through sync entry points rather than scattered direct refresh logic
- any intentionally deferred approval-refresh behavior is explicitly documented in code comments and docs
- Transaction status enum is expanded beyond `pending/confirmed/failed` to the 6-state vocabulary below.
- BTC and EVM send flows use the same lifecycle entry semantics after broadcast.
- Per-chain finality rules are documented in `wallet/sync/types.rs` or alongside the trait: BTC uses a minimum-confirmation threshold (constant default, configurable) to advance from `pending` to `confirmed`; EVM uses `receipt.status == 1` with a minimum block-depth threshold to advance from `pending` to `confirmed`.
- Per-chain partial failure is representable: `evm_get_wallet_with_balances` returns a typed response where individual chains may be marked `Unavailable` or `Stale` without failing the whole call.
- `cargo check` and targeted lifecycle tests pass (see `docs/architecture/testing-strategy.md` → Transaction Lifecycle and Sync Engine must-test lists; at minimum one failure-path test per external dependency class).

### Key Implementation Logic

- Create a sync engine that centralizes:
  - sync reason
  - sync target
  - sync outcome
- Centralize logic first; a background scheduler may still be deferred, but refresh entry points must stop being scattered across unrelated modules.
- Define a canonical lifecycle vocabulary up front:
  - `broadcasted`
  - `pending`
  - `confirmed`
  - `failed`
  - `replaced`
  - `dropped`
- Define minimum transition rules up front:
  - send success after local broadcast -> `broadcasted`
  - chain observation without sufficient finality -> `pending`
  - receipt or chain state indicating success/finality -> `confirmed`
  - explicit chain failure or rejected execution -> `failed`
  - transaction superseded by another tx from the same wallet / same nonce path -> `replaced`
  - transaction disappears from active tracking without confirmation and is no longer expected to land -> `dropped`
- A send must not be written directly as `confirmed` at insert time just because it was broadcast successfully.
- Add enough lifecycle richness to support the UI, without overcommitting to every edge case.
- For this plan, deeper approval semantics may remain deferred, but approval refresh must have a sync-engine entry point or be explicitly marked deferred.

### Files In Scope

- Create: `src-tauri/src/wallet/sync/*`
- Create: `src-tauri/src/wallet/chain/*`
- Modify: `src-tauri/src/wallet/mod.rs` (register `chain` module)
- Modify: `src-tauri/src/wallet/bitcoin/commands.rs`
- Modify: `src-tauri/src/wallet/bitcoin/balance.rs` (implement ChainAdapter for BTC)
- Modify: `src-tauri/src/wallet/evm/commands.rs`
- Modify: `src-tauri/src/wallet/evm/balance.rs` (implement ChainAdapter for EVM)
- Modify: `src-tauri/src/dashboard/commands.rs`
- Modify: `src-tauri/src/wallet/transaction_types.rs`
- Modify: `src-tauri/src/wallet/bitcoin/transaction.rs`
- Modify: `src-tauri/src/wallet/evm/transaction.rs`
- Modify: `src-tauri/src/wallet/transaction_commands.rs`
- Modify: `src-tauri/src/db.rs`

### Verification

- `cargo test --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml wallet::sync -- --nocapture`
- `cargo test --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml transaction_status -- --nocapture`
- `cargo check --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml`

### Risks

- Accidentally baking chain-specific semantics into the shared lifecycle model.
- Allowing dashboard refresh to remain a separate logic island.
- Expanding lifecycle semantics without preserving compatibility for existing persisted transaction rows.
- Defining `ChainAdapter` around the current (2-chain-family) shape in a way that an additional chain family would force reshape; keep the trait minimal.
- Representing partial failure as a log message instead of a typed value in command responses.

---

## Phase 4: UI Consumption Of Explicit Wallet State

### Target State After This Phase

- Dashboard, Portfolio, Transactions, and relevant Swap flows consume explicit freshness and lifecycle state from the backend.
- The frontend no longer silently repairs or disguises backend inconsistency.
- Users can see whether data is fresh, cached, stale, partial, or unavailable.
- BTC price display can distinguish a real fetched value from fallback or unavailable price state.

### Acceptance Criteria

- `useDashboardData.ts` no longer recalculates consistency heuristics from allocation ratios.
- BTC price fallback is no longer silent.
- The `get_bitcoin_price` Tauri command is registered in `lib.rs` (it currently exists in Rust but is not wired into `invoke_handler!`, so the frontend call always throws and the hardcoded `95000` fallback always applies). After registration, the command returns a `PriceState` (per ADR-0003) rather than a bare number.
- The hardcoded BTC price fallback (`setBtcPrice(95000)`) in `src/pages/Portfolio/components/BitcoinAssets.tsx` is removed. BTC price display consumes `PriceState` with explicit `fresh` / `stale` / `unavailable` / `synthetic` distinctions.
- BTC/EVM wallet UIs expose freshness and last-refresh semantics.
- Transactions UI can show richer lifecycle states.
- Backend-facing commands provide enough price metadata for the UI to render fallback and stale states honestly.
- `npm run build` passes. UI consumption matches `docs/architecture/testing-strategy.md` → UI Consumption Layer must-test list.

### Key Implementation Logic

- Remove UI-side “make it look consistent” logic first.
- Then wire explicit state from backend commands.
- Keep visual changes small; the point is semantic clarity, not redesign.
- Reuse state-model types across the frontend where possible.
- Update dashboard-facing types and presentation components, not just the data hook, so freshness and fallback semantics are actually visible to users.

### Files In Scope

- Modify: `src/pages/Dashboard/hooks/useDashboardData.ts`
- Modify: `src/pages/Dashboard/types.ts`
- Modify: `src/pages/Dashboard/components/StatsCards.tsx`
- Modify: `src/pages/Dashboard/components/PortfolioChart.tsx`
- Modify: `src/pages/Portfolio/components/BitcoinAssets.tsx`
- Modify: `src/pages/Portfolio/components/EvmAssets.tsx`
- Modify: `src/pages/Transactions/index.tsx`
- Modify: `src/pages/Swap/hooks/useSwap.ts`
- Modify: `src-tauri/src/lib.rs` (register `get_bitcoin_price` in `invoke_handler!`)
- Modify: `src-tauri/src/dashboard/commands.rs`
- Modify: `src-tauri/src/wallet/state/commands.rs`
- Modify: `src-tauri/src/wallet/evm/price.rs` (return `PriceState` shape)
- Modify: `src-tauri/src/wallet/evm/price_manager.rs`

### Verification

- `cd /Users/hhx/work/aiigo/aiigo-desktop && npm run build`

### Risks

- Shipping a state model in Rust that the UI does not fully consume.
- Keeping old fallback behavior alive behind the new metadata.

---

## Phase 5: Documentation And Decision Alignment

### Target State After This Phase

- The architecture documents accurately describe the implemented runtime boundaries.
- ADRs reflect the wallet foundation decisions introduced by earlier phases.

### Acceptance Criteria

- Architecture docs match implemented module names and state terminology.
- ADRs explain why SQLite/read-model and Web3 session boundaries are being shaped this way.
- Full project verification passes.

### Key Implementation Logic

- Update docs only after code semantics stabilize.
- Align naming between code and docs before writing explanatory text.
- Record the final vocabulary for:
  - freshness
  - partial failure
  - transaction lifecycle
  - security boundary

### Files In Scope

- Modify: `docs/architecture/security-model.md`
- Modify: `docs/architecture/wallet-state-model.md`
- Modify: `docs/architecture/integration-surfaces.md`
- Modify: `docs/architecture/appendices/current-architecture.md`
- Modify: `docs/architecture/adr/0002-sqlite-as-read-model.md`
- Modify: `docs/architecture/adr/0004-web3-session-boundary.md`

### Verification

- `cargo check --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml`
- `cargo test --manifest-path /Users/hhx/work/aiigo/aiigo-desktop/src-tauri/Cargo.toml`
- `cd /Users/hhx/work/aiigo/aiigo-desktop && npm run build`

### Risks

- Letting docs drift from the final code vocabulary.
- Writing docs before the lifecycle and freshness model are stable.

---

## Delivery Milestones

### Milestone A: Security Boundary Extracted (Not Yet At-Rest Encrypted)

Achieved when Phase 1 completes.

Meaning:
- secret handling is centralized behind one abstraction
- unlock exists as a first-class concept and is enforced (not merely checkable) on all export and signing paths
- logs and error surfaces no longer leak secret-bearing material
- future keystore improvements, including real at-rest encryption, can land without reworking every wallet flow

Explicitly not meaning:
- secrets are encrypted at rest (this is deferred to a follow-on plan)
- the wallet is immune to local disclosure via DB file access
Communication about Milestone A must preserve this distinction; calling Phase 1 "safe" without the qualifier misrepresents the delivered risk posture.

### Milestone B: State Is Explicit

Achieved when Phase 2 completes.

Meaning:
- freshness and partial failure are part of the backend contract
- the wallet stops pretending all values are equal kinds of truth

### Milestone C: Runtime Semantics Are Coherent

Achieved when Phase 3 completes.

Meaning:
- refreshes are not scattered ad hoc
- BTC and EVM transaction states mean the same thing

### Milestone D: User Feedback Is Honest

Achieved when Phase 4 completes.

Meaning:
- the UI no longer masks stale or partial state
- transaction progress is clearer

### Milestone E: Docs Match Reality

Achieved when Phase 5 completes.

Meaning:
- architecture docs and ADRs are aligned with code

---

## Acceptance Matrix

| Phase | Resulting State | Primary Acceptance |
|---|---|---|
| 1 | Security boundary extracted (not yet at-rest encrypted) | secret access centralized, unlock enforced, logs sanitized |
| 2 | State model exists | freshness, partial failure, and price state modeled with stable typed contracts |
| 3 | Sync, lifecycle, and chain adapter are coherent | shared refresh paths, ChainAdapter trait, normalized tx lifecycle with per-chain finality rules |
| 4 | UI reflects explicit truth | no hidden consistency repair, no silent fallback, `get_bitcoin_price` registered |
| 5 | Docs match implementation | architecture docs and ADRs updated |

---

## Implementation Notes

- Do not start with WalletConnect or EIP-1193 in this plan.
- Do not attempt full encrypted keystore rollout in the same phase as boundary extraction.
- Do not communicate Milestone A as "secrets are now safe". The milestone extracts the boundary; it does not encrypt secrets at rest. Any release note, internal update, or doc referencing Milestone A must carry this qualifier.
- Do not ship enforcement (Phase 1) without a usable unlock UI, or existing flows will break silently.
- Do not leave BTC and EVM transaction lifecycle semantics divergent.
- Do not let the sync engine absorb chain-specific RPC or explorer code; it must depend on `ChainAdapter`.
- Do not keep UI-side consistency heuristics after explicit backend state exists.
- Do not ship new Tauri commands without registering them in `lib.rs`; the current `get_bitcoin_price` omission is the canonical example of this class of bug and its fix is mandatory in Phase 4.

## Follow-On Plans

These are intentionally out of scope for this plan:

- WalletConnect integration
- EIP-1193 provider support
- typed-data signing
- approval simulation and risk review
- stronger encrypted keystore implementation
