# ADR 0004: Web3 Session Boundary

## Status

Accepted

## Context

The target architecture explicitly reserves a `Web3 Interaction Layer`, but the current runtime does not yet implement:

- EIP-1193 provider support
- WalletConnect
- typed-data signing
- generalized dApp session state

This creates a dangerous temptation: when Web3 wallet features are eventually added, implementers may try to wire dApp requests directly into:

- transaction modules
- raw secret access
- ad hoc UI prompts

That would produce a wallet that can technically connect to dApps, but without a clean session boundary, permission model, or signing review model.

The architecture needs an explicit decision before those features arrive, otherwise future implementation will be shaped by convenience rather than trust boundaries.

## Decision

We introduce an explicit **Web3 session boundary** as a separate architectural surface, even before the full feature set is implemented.

This means:

1. dApp-originated requests must be treated as session-scoped inputs, not as ordinary wallet actions.
2. Web3 session handling must live in a dedicated interaction layer rather than being spread across wallet transaction modules.
3. No dApp-facing path may directly access raw secrets.
4. All future Web3 signing requests must pass through:
   - session policy
   - account exposure policy
   - chain exposure policy
   - signing authority checks
5. The interaction layer is allowed to translate between external protocols and internal wallet actions, but it must not bypass the security boundary or wallet-state facade.

### Immediate Implication

Even while WalletConnect and EIP-1193 remain out of scope for the current runtime, the architecture reserves a strict boundary for them so they do not distort the core wallet design later.

### Current Runtime Anchor

The current runtime already has a local security boundary for product-owned actions:

- send, approve, and export flows pass through `wallet/security/*`
- unlock state is tracked by `SessionManager`
- secret reads are routed through `SqliteKeystore`

That boundary is not a Web3 session layer, but it is the authority future dApp-facing session code must call into rather than bypass.

## Consequences

### Positive

- Future Web3 wallet features have a dedicated home in the architecture.
- dApp session logic stays separate from asset-wallet business logic.
- Permission review and signing review can evolve independently from chain adapters and transaction lifecycle logic.
- Secret access remains behind the security boundary even when external sessions are introduced.

### Negative

- The architecture intentionally reserves a subsystem that is not yet fully implemented.
- Some future feature work will feel slower because it must conform to the reserved boundary instead of taking shortcuts.

### Architectural Implications

- `Web3 Interaction Layer` sits above transaction lifecycle and beside UI prompt flows
- it talks to the security boundary, not directly to secret backing
- it talks to the wallet-state facade for exposed accounts and active-chain state
- it does not own chain truth or transaction persistence

## Alternatives Considered

### 1. Add WalletConnect / EIP-1193 Later Without A Prior Boundary Decision

Rejected.

This would almost certainly cause dApp behavior to leak directly into transaction and signing code, making later cleanup expensive.

### 2. Let Transaction Modules Handle dApp Requests Directly

Rejected.

Transaction modules should build and track transactions. They should not become the product's session-permission engine.

### 3. Let The Security Layer Also Own Web3 Sessions

Rejected.

The security boundary should guard secret use and signing authority. Session policy and account exposure are related concerns, but not the same concern. Combining them would blur responsibilities again.
