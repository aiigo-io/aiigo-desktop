# Security Model

## Purpose

This document describes the trust boundary around secrets, signing authority, unlock sessions, and export behavior.

## Threat Model

The current application is a local-first desktop wallet. Its primary security concerns are not only network-originated attacks, but also misuse of local trust and signing authority.

### Main Threat Categories

- `Local secret disclosure`
  Mnemonic or private-key material is exposed through storage, logs, crash paths, or export flows.
- `Over-broad signing authority`
  A runtime path can sign or export without an explicit unlock or permission boundary.
- `Unsafe export behavior`
  Users can reveal mnemonic or private-key material without a strong enough trust boundary or risk acknowledgement.
- `Protocol-side signing abuse`
  Future Web3 session surfaces may cause the wallet to sign messages or approve permissions that users do not understand.
- `State ambiguity leading to unsafe actions`
  Users may resend, overpay, or mis-handle assets because transaction and freshness state are unclear.

### Threats That Already Matter Today

- plaintext storage of secret material
- immediate export of mnemonic or private key through Tauri commands
- transaction signing paths that read secret material directly from local storage

## Sensitive Data Classification

The wallet should classify sensitive data into at least four tiers.

### Tier 1: Secret Material

- mnemonic phrases
- private keys
- any future seed- or keystore-derived secret

These must be treated as the highest-risk data in the system.

### Tier 2: Signing Authority State

- unlock session tokens or flags
- any in-memory representation that enables signing
- future dApp session permissions

### Tier 3: Privacy-Sensitive Wallet Data

- addresses
- wallet labels
- transaction history
- balances and portfolio values

This data is lower risk than key material, but still privacy sensitive.

### Tier 4: Public Or Low-Sensitivity Metadata

- static asset metadata
- chain configuration
- non-sensitive UI preferences

## Key Material Lifecycle

### Current Lifecycle

Today, key material enters the system through:

- mnemonic generation
- mnemonic import
- private-key import

After import or generation, secret material is written into SQLite-backed secret tables and later read again when:

- exporting mnemonic or private key
- signing a Bitcoin transaction
- signing an EVM transaction
- approving an ERC20 token

### Current Architectural Problem

The application already treats secret material as usable runtime input, but does not yet isolate it behind a hardened key-security layer.

### Target Lifecycle

The intended lifecycle should be:

1. import or generate secret material
2. encrypt and persist through a keystore abstraction
3. unlock into memory only for bounded signing sessions
4. sign through an explicit authority boundary
5. clear or expire in-memory access
6. require a dedicated export path with additional user friction

## Storage Strategy

### Current Strategy

Secret material is currently persisted in SQLite-backed secret tables:

- `bitcoin_wallet_secrets`
- `evm_wallet_secrets`

The current codebase also contains explicit comments indicating that at least some paths still store values in plaintext "for now" with a future encryption TODO.

### Current Risks

- database disclosure exposes secret material directly
- storage and signing are tightly coupled
- storage semantics are not separated from export semantics

### Target Strategy

The target storage strategy should introduce a dedicated key-security abstraction with:

- encrypted-at-rest secret storage
- separation between wallet metadata and secret material
- explicit unlock flow
- no direct "read secret from DB and sign" path outside the key-security layer

## Unlock Session Model

### Current Reality

The current architecture does not yet model unlock state as a first-class concept.

This means the wallet currently behaves closer to:

- "secret is locally available and can be read when needed"

than to:

- "secret access is mediated by an explicit unlock session"

### Target Model

The wallet should eventually support:

- locked state
- unlocked state with expiration
- per-operation re-authentication for high-risk actions
- separate policies for:
  - send transaction
  - export mnemonic
  - export private key
  - future typed data signing

## Signing Authority Boundary

### Current Boundary

Today, signing paths are effectively implemented by:

1. look up wallet metadata
2. read secret material from SQLite
3. derive or reconstruct signing key
4. sign and broadcast

This boundary is too thin for a mature wallet.

### Target Boundary

The target architecture should require all signing operations to pass through one explicit signing authority layer that:

- checks unlock state
- checks operation type
- enforces policy
- emits auditable result metadata
- avoids exposing raw secrets to general business logic

## Export Policy

### Current Reality

The wallet exposes direct Tauri commands for:

- mnemonic export
- private-key export

These commands are part of the registered command surface today.

### Security Concern

Export is not the same thing as normal signing. Export materially increases the blast radius of any compromise and should not share the same trust level as a normal send flow.

### Target Export Policy

Exports should eventually require:

- explicit unlock state
- stronger user intent confirmation
- clear risk messaging
- optional platform-level restrictions or feature gating

## Recovery And Backup Model

### Current Reality

The current implementation supports mnemonic-based creation and import, and private-key import and export.

### Missing Model

What is still missing is a documented and enforced recovery model that clearly distinguishes:

- mnemonic-backed wallets
- private-key-backed wallets
- which operations are possible for each
- which recovery promises the product makes

### Target Direction

The wallet should explicitly document and enforce:

- backup responsibilities
- recovery guarantees
- wallet-type limitations
- migration rules between wallet types if supported in the future

## Logging And Error Hygiene

### Current Risk

The current codebase includes direct console and logging usage in transaction and refresh paths. Even without obvious secret logging today, the architecture does not yet formally constrain what may or may not be logged around sensitive operations.

### Required Policy

The security model should guarantee that logs and surfaced errors do not contain:

- mnemonic phrases
- private keys
- raw secret payloads
- sensitive unlock/session tokens

## IPC Security Considerations

### Current Model

The frontend talks to the Rust backend through Tauri command invocation. Sensitive commands and normal commands currently share the same transport mechanism.

### Security Implication

Security depends on the command surface itself being tightly designed:

- low-risk queries should be distinct from high-risk secret or signing operations
- export flows should be clearly separated from normal wallet reads
- future dApp-facing interactions must not be allowed to directly map onto raw secret reads

## Security Non-Goals

The current architecture does not attempt to provide:

- remote custody
- MPC or distributed signing
- hardware wallet support
- account-abstraction policy engines
- full phishing-resistant dApp signing UX

These are future concerns, not current guarantees.

## Current Implementation Risks

The most important currently visible risks are:

### 1. Plaintext-Oriented Secret Handling

The codebase explicitly contains plaintext-storage TODO comments for secret persistence paths. This is the clearest sign that the key-security layer is not yet complete.

### 2. No First-Class Unlock Model

The runtime does not yet model secret access through a lock/unlock session abstraction.

### 3. Export And Signing Share The Same Fundamental Secret Access Pattern

Both exporting and signing rely on direct secret retrieval from persistence, rather than a hardened signing boundary.

### 4. Security Boundary Is Not Yet Separated From Business Logic

Wallet creation, export, transaction signing, and approval signing still cross through business logic modules rather than a dedicated security subsystem.

## Security Hardening Roadmap

### Near-Term

- introduce a dedicated key-security abstraction
- stop direct plaintext-oriented storage paths
- separate export policy from transaction-signing policy
- define a minimal unlock session model

### Mid-Term

- route all signing through one signing authority boundary
- classify and sanitize all sensitive logging and error paths
- add stronger user-intent confirmation for secret export

### Longer-Term

- integrate future Web3 signing request review into the same authority boundary
- add support for richer session policy and permission-scoped signing
