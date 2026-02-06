# AIIGO Computing Power Marketplace - Solidity Contracts Specification

> Extracted from `COMPUTING_POWER_SYSTEM_DESIGN.md` for implementation reference.
> Architecture: Simplified ETH-only marketplace (no governance token, no DAO).

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                   SMART CONTRACT ARCHITECTURE                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  CORE CONTRACTS (4 total)                                        │
│                                                                  │
│                    ┌──────────────┐                               │
│                    │ NodeRegistry │                               │
│                    │  • ETH Stake │                               │
│                    │  • Reputation│                               │
│                    └──────┬───────┘                               │
│                           │                                      │
│  ┌──────────────┐   ┌────┴─────────┐   ┌──────────────────┐    │
│  │   Escrow     │◀─▶│    Task      │◀─▶│ ProofOfWork      │    │
│  │   Manager    │   │ Marketplace  │   │ Verifier         │    │
│  │  ETH/USDC   │   │  ETH/USDC   │   │ Compute Proof    │    │
│  └──────────────┘   └──────────────┘   └──────────────────┘    │
│                                                                  │
│  UTILITY CONTRACTS (OpenZeppelin)                                │
│  ┌────────────┐  ┌────────────┐  ┌──────────────────┐          │
│  │  Ownable   │  │  Pausable  │  │ ReentrancyGuard  │          │
│  └────────────┘  └────────────┘  └──────────────────┘          │
│                                                                  │
│  Removed:                                                        │
│  • AIIGOToken (no governance token)                              │
│  • StakingPool (no token staking)                                │
│  • RewardDistributor (direct ETH payments)                       │
│  • Governor (no DAO - owner-controlled)                          │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Payment Flow:**
```
Buyer deposits ETH/USDC → Escrow → Task completed → Provider receives 92% → Platform 8%
```

---

## Deployment Order

```
1. NodeRegistry.sol          ← Deploy first (standalone, owner-controlled)
2. ProofOfWorkVerifier.sol   ← Depends on NodeRegistry
3. EscrowManager.sol         ← Standalone escrow logic
4. TaskMarketplace.sol       ← Depends on all above (orchestrator contract)
```

### Post-Deployment Configuration

```solidity
// Set contract references
taskMarketplace.setNodeRegistry(address(nodeRegistry));
taskMarketplace.setEscrowManager(address(escrowManager));
taskMarketplace.setPowVerifier(address(powVerifier));

// Grant permissions
nodeRegistry.grantRole(UPDATER_ROLE, address(powVerifier));
nodeRegistry.grantRole(UPDATER_ROLE, address(taskMarketplace));
escrowManager.grantRole(RELEASER_ROLE, address(taskMarketplace));

// Transfer ownership to multi-sig (recommended for production)
nodeRegistry.transferOwnership(MULTISIG_ADDRESS);
taskMarketplace.transferOwnership(MULTISIG_ADDRESS);
escrowManager.transferOwnership(MULTISIG_ADDRESS);
powVerifier.transferOwnership(MULTISIG_ADDRESS);
```

---

## Contract 1: NodeRegistry.sol

**Purpose:** Manages provider registration, status, hardware inventory, and ETH-based staking.

### Data Structures

```solidity
enum NodeStatus { Pending, Verified, Active, Inactive, Slashed }
enum ResourceType { GPU, CPU, Network, Mobile, IoT }

struct Node {
    address owner;
    bytes32 nodeId;           // Unique identifier
    NodeStatus status;
    ResourceType resourceType;
    uint256 computePower;     // Verified computing power (TFLOPS)
    uint256 stakedAmount;     // ETH staked (in wei)
    uint256 reputation;       // 0-10000 (basis points)
    uint256 totalTasksCompleted;
    uint256 totalEarnings;    // Total ETH earned (in wei)
    uint256 registeredAt;
    uint256 lastActiveAt;
    string metadataURI;       // IPFS URI for hardware specs
}

struct HardwareSpec {
    string gpuModel;          // e.g., "RTX 4090"
    uint256 gpuMemory;        // in GB
    uint256 cpuCores;
    uint256 ramGB;
    uint256 storageGB;
    uint256 bandwidthMbps;
    string region;            // Geographic region
}
```

### Node State Machine

```
[Pending] ──▶ [Verified] ──▶ [Active] ──▶ [Earning]
                  │              │
                  ▼              ▼
              [Rejected]    [Inactive] ──▶ [Slashed]
```

### Interface

```solidity
interface INodeRegistry {
    // Registration with ETH deposit (min 0.1 ETH registration + 0.5 ETH stake)
    function registerNode(
        ResourceType resourceType,
        string calldata metadataURI
    ) external payable returns (bytes32 nodeId); // Requires msg.value >= 0.6 ETH

    // Stake additional ETH
    function addStake(bytes32 nodeId) external payable;

    // Withdraw stake (only when node inactive and no pending tasks)
    function withdrawStake(bytes32 nodeId, uint256 amount) external;

    // Admin functions
    function updateNodeStatus(bytes32 nodeId, NodeStatus status) external;
    function updateReputation(bytes32 nodeId, int256 delta) external;
    function slashStake(bytes32 nodeId, uint256 amount) external;

    // View functions
    function getNode(bytes32 nodeId) external view returns (Node memory);
    function getNodesByOwner(address owner) external view returns (bytes32[] memory);
    function getActiveNodes(ResourceType resourceType) external view returns (bytes32[] memory);
    function isNodeActive(bytes32 nodeId) external view returns (bool);
    function getNodeTrustLevel(bytes32 nodeId) external view returns (uint8);
}
```

### Events

```solidity
event NodeRegistered(bytes32 indexed nodeId, address indexed owner, ResourceType resourceType);
event NodeStatusChanged(bytes32 indexed nodeId, NodeStatus oldStatus, NodeStatus newStatus);
event StakeDeposited(bytes32 indexed nodeId, uint256 amount, uint256 totalStake);
event StakeWithdrawn(bytes32 indexed nodeId, uint256 amount, uint256 totalStake);
event ReputationUpdated(bytes32 indexed nodeId, uint256 oldReputation, uint256 newReputation);
event NodeSlashed(bytes32 indexed nodeId, uint256 slashedAmount, string reason);
```

### Trust Level Calculation

```solidity
function getNodeTrustLevel(bytes32 nodeId) public view returns (uint8) {
    Node memory node = nodes[nodeId];

    // Level 4: Partner (5+ ETH stake, 95%+ success, SLA commitment)
    if (node.stakedAmount >= 5 ether &&
        node.reputation >= 9500 &&
        hasPartnerSLA[nodeId]) {
        return 4;
    }

    // Level 3: Trusted (3+ ETH stake, 95%+ success rate, 100+ tasks)
    if (node.stakedAmount >= 3 ether &&
        node.reputation >= 9500 &&
        node.totalTasksCompleted >= 100) {
        return 3;
    }

    // Level 2: Verified (1+ ETH stake, 10+ PoW challenges)
    if (node.stakedAmount >= 1 ether && powChallengesPassed[nodeId] >= 10) {
        return 2;
    }

    // Level 1: Basic (0.5+ ETH stake, registered)
    if (node.stakedAmount >= 0.5 ether) {
        return 1;
    }

    return 0; // Not registered
}
```

### Stake Requirements

| Trust Level | ETH Stake | Access |
|-------------|-----------|--------|
| Level 1 (Basic) | 0.5 ETH | Small tasks (<$100) |
| Level 2 (Verified) | 1.0 ETH + 10 PoW challenges | Standard marketplace |
| Level 3 (Trusted) | 3.0 ETH + 100 tasks + 95% success | Enterprise + Priority |
| Level 4 (Partner) | 5.0 ETH + SLA commitment | Direct contracts + Whitelabel |

### Fee Schedule

| Action | Fee | Recipient |
|--------|-----|-----------|
| Node Registration | 0.1 ETH (one-time) | Platform Treasury |
| Minimum Stake | 0.5 ETH (refundable) | Held as collateral |

---

## Contract 2: TaskMarketplace.sol

**Purpose:** Core marketplace for task creation, matching, and lifecycle management.

### Data Structures

```solidity
enum TaskStatus { Open, Assigned, InProgress, Completed, Verified, Disputed, Cancelled }

struct Task {
    bytes32 taskId;
    address buyer;
    bytes32 assignedNode;     // Assigned provider node
    ResourceType resourceType;
    uint256 requiredPower;    // Minimum TFLOPS required
    uint256 duration;         // Expected duration in seconds
    uint256 maxPrice;         // Maximum price per hour (in wei)
    uint256 escrowAmount;     // Total escrowed payment
    uint256 createdAt;
    uint256 startedAt;
    uint256 completedAt;
    TaskStatus status;
    uint8 minTrustLevel;      // Minimum provider trust level
    string specificationURI;  // IPFS URI for task details
}

struct TaskResult {
    bytes32 taskId;
    bytes32 resultHash;       // Hash of computation result
    string resultURI;         // IPFS URI for result data
    uint256 actualDuration;
    uint256 computeUnitsUsed;
    bool verified;
}
```

### Task Lifecycle

```
[Submit] → [Queue] → [Match] → [Assign] → [Execute] → [Verify] → [Pay]
              │                    │           │          │
              ▼                    ▼           ▼          ▼
          [Timeout]           [Reassign]   [Failed]  [Dispute]
```

### Interface

```solidity
interface ITaskMarketplace {
    // Buyer functions
    function createTask(
        ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration,
        uint256 maxPrice,
        uint8 minTrustLevel,
        string calldata specificationURI
    ) external payable returns (bytes32 taskId);

    function cancelTask(bytes32 taskId) external;
    function disputeTask(bytes32 taskId, string calldata reason) external;
    function approveResult(bytes32 taskId) external;

    // Provider functions
    function acceptTask(bytes32 taskId, bytes32 nodeId) external;
    function submitResult(
        bytes32 taskId,
        bytes32 resultHash,
        string calldata resultURI
    ) external;

    // View functions
    function getTask(bytes32 taskId) external view returns (Task memory);
    function getOpenTasks(ResourceType resourceType) external view returns (bytes32[] memory);
    function getTasksByBuyer(address buyer) external view returns (bytes32[] memory);
    function getTasksByProvider(bytes32 nodeId) external view returns (bytes32[] memory);
    function estimateTaskCost(
        ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration
    ) external view returns (uint256);
}
```

### Events

```solidity
event TaskCreated(bytes32 indexed taskId, address indexed buyer, ResourceType resourceType, uint256 maxPrice);
event TaskAssigned(bytes32 indexed taskId, bytes32 indexed nodeId, uint256 startTime);
event TaskCompleted(bytes32 indexed taskId, bytes32 resultHash);
event TaskVerified(bytes32 indexed taskId, uint256 payoutAmount);
event TaskDisputed(bytes32 indexed taskId, address disputedBy, string reason);
event TaskCancelled(bytes32 indexed taskId, uint256 refundAmount);
```

### Task Matching Logic

```solidity
function matchTask(bytes32 taskId) internal view returns (bytes32[] memory eligibleNodes) {
    Task memory task = tasks[taskId];
    bytes32[] memory activeNodes = nodeRegistry.getActiveNodes(task.resourceType);

    uint256 count = 0;
    bytes32[] memory candidates = new bytes32[](activeNodes.length);

    for (uint256 i = 0; i < activeNodes.length; i++) {
        INodeRegistry.Node memory node = nodeRegistry.getNode(activeNodes[i]);

        // Check eligibility
        if (node.computePower >= task.requiredPower &&
            nodeRegistry.getNodeTrustLevel(activeNodes[i]) >= task.minTrustLevel &&
            node.status == INodeRegistry.NodeStatus.Active) {
            candidates[count] = activeNodes[i];
            count++;
        }
    }

    // Resize array
    eligibleNodes = new bytes32[](count);
    for (uint256 i = 0; i < count; i++) {
        eligibleNodes[i] = candidates[i];
    }
}
```

### Matching Score Formula

```
Score = w1×Price + w2×Reputation + w3×Latency + w4×Uptime + w5×VerifiedPower
```

---

## Contract 3: EscrowManager.sol

**Purpose:** Secure fund custody during task execution with 92/8 split.

### Data Structures

```solidity
struct EscrowDeposit {
    bytes32 taskId;
    address buyer;
    address provider;
    uint256 amount;
    uint256 platformFee;      // 8% of amount
    uint256 providerPayout;   // 92% of amount
    uint256 depositedAt;
    bool released;
    bool refunded;
}
```

### Interface

```solidity
interface IEscrowManager {
    function deposit(bytes32 taskId, address buyer) external payable returns (bytes32 escrowId);
    function release(bytes32 taskId, address provider) external;
    function refund(bytes32 taskId) external;
    function splitPayment(bytes32 taskId, address provider, uint256 providerShare) external;
    function getEscrow(bytes32 taskId) external view returns (EscrowDeposit memory);
}
```

### Events

```solidity
event EscrowDeposited(bytes32 indexed taskId, address indexed buyer, uint256 amount);
event EscrowReleased(bytes32 indexed taskId, address indexed provider, uint256 providerPayout, uint256 platformFee);
event EscrowRefunded(bytes32 indexed taskId, address indexed buyer, uint256 amount);
event DisputeResolved(bytes32 indexed taskId, address indexed winner, uint256 amount);
```

### Payment Distribution Implementation

```solidity
uint256 constant PLATFORM_FEE_BPS = 800; // 8%

function release(bytes32 taskId, address provider) external onlyTaskMarketplace {
    EscrowDeposit storage escrow = escrows[taskId];
    require(!escrow.released && !escrow.refunded, "Already processed");

    // Calculate splits
    uint256 platformFee = (escrow.amount * PLATFORM_FEE_BPS) / 10000; // 8%
    uint256 providerPayout = escrow.amount - platformFee; // 92%

    escrow.platformFee = platformFee;
    escrow.providerPayout = providerPayout;
    escrow.released = true;
    escrow.provider = provider;

    // Transfer funds
    payable(provider).transfer(providerPayout);
    payable(treasury).transfer(platformFee);

    emit EscrowReleased(taskId, provider, providerPayout, platformFee);
}
```

### Fee Structure

| Action | Fee | Recipient |
|--------|-----|-----------|
| Task Completion | 8% of task value | Platform Treasury |
| Dispute Resolution | 2% of escrowed amount | Arbitrator pool |
| Slashing (violations) | Up to 50% of stake | Platform Treasury |

---

## Contract 4: ProofOfWorkVerifier.sol

**Purpose:** Verifies provider computing power through cryptographic challenges.

### Data Structures

```solidity
struct Challenge {
    bytes32 challengeId;
    bytes32 nodeId;
    bytes32 seed;             // Random seed
    uint256 difficulty;       // Target difficulty
    uint256 issuedAt;
    uint256 deadline;
    bool completed;
    uint256 solutionTime;     // Time taken to solve
}

struct VerificationResult {
    bytes32 nodeId;
    uint256 verifiedPower;    // TFLOPS equivalent
    uint256 timestamp;
    bool passed;
}
```

### Interface

```solidity
interface IProofOfWorkVerifier {
    function issueChallenge(bytes32 nodeId) external returns (bytes32 challengeId);
    function submitSolution(
        bytes32 challengeId,
        uint256 nonce
    ) external returns (bool passed);
    function getChallenge(bytes32 challengeId) external view returns (Challenge memory);
    function getVerificationHistory(bytes32 nodeId) external view returns (VerificationResult[] memory);
    function calculateComputePower(uint256 solutionTime, uint256 difficulty) external pure returns (uint256);
}
```

### Events

```solidity
event ChallengeIssued(bytes32 indexed challengeId, bytes32 indexed nodeId, uint256 difficulty, uint256 deadline);
event ChallengeSolved(bytes32 indexed challengeId, bytes32 indexed nodeId, uint256 solutionTime, uint256 verifiedPower);
event ChallengeFailed(bytes32 indexed challengeId, bytes32 indexed nodeId, string reason);
```

### Challenge Verification Implementation

```solidity
function submitSolution(bytes32 challengeId, uint256 nonce) external returns (bool) {
    Challenge storage challenge = challenges[challengeId];
    require(!challenge.completed, "Already completed");
    require(block.timestamp <= challenge.deadline, "Challenge expired");

    // Verify node ownership
    INodeRegistry.Node memory node = nodeRegistry.getNode(challenge.nodeId);
    require(node.owner == msg.sender, "Not node owner");

    // Verify solution: hash(seed || nonce) < difficulty
    bytes32 solution = keccak256(abi.encodePacked(challenge.seed, nonce));
    require(uint256(solution) < challenge.difficulty, "Invalid solution");

    // Calculate solve time and verified power
    uint256 solutionTime = block.timestamp - challenge.issuedAt;
    uint256 verifiedPower = calculateComputePower(solutionTime, challenge.difficulty);

    challenge.completed = true;
    challenge.solutionTime = solutionTime;

    // Update node reputation and verified power
    nodeRegistry.updateReputation(challenge.nodeId, REPUTATION_BOOST);
    nodeRegistry.updateComputePower(challenge.nodeId, verifiedPower);

    emit ChallengeSolved(challengeId, challenge.nodeId, solutionTime, verifiedPower);
    return true;
}

function calculateComputePower(uint256 solutionTime, uint256 difficulty) public pure returns (uint256) {
    // Power = difficulty / time (normalized to TFLOPS)
    // Faster solutions at higher difficulty = more compute power
    return (difficulty * 1e18) / (solutionTime * BASE_DIFFICULTY);
}
```

### Verification Process

```
1. Platform issues cryptographic challenge (random seed + difficulty target)
2. Provider's device computes solution (find nonce where hash < difficulty)
3. Solution submitted on-chain (verified in smart contract)
4. Performance score calculated (faster solve = higher verified power)
5. Reputation updated (success: +reputation, fail: -reputation)
```

---

## Slashing Conditions

| Violation | Penalty |
|-----------|---------|
| Failed PoW challenge | -2% reputation |
| Task timeout | -5% stake |
| Data breach/Misuse | -50% stake + Ban |
| Repeated failures (>20%) | Review + Potential ban |

---

## Task Lifecycle Flow (Contract Interactions)

```
BUYER                    CONTRACTS                    PROVIDER
  │                         │                            │
  │  1. createTask()        │                            │
  ├────────────────────────▶│ TaskMarketplace            │
  │  2. deposit ETH         │                            │
  ├────────────────────────▶│ EscrowManager              │
  │                         │                            │ 3. acceptTask()
  │                         │◀───────────────────────────┤
  │                         │  4. Verify PoW (optional)  │
  │                         │◀──────────────────────────▶│ PoWVerifier
  │                         │                            │ 5. submitResult()
  │                         │◀───────────────────────────┤
  │  6. approveResult()     │                            │
  ├────────────────────────▶│                            │
  │                         │  7. release() → 92% ETH   ─▶│
  │                         │  8. updateReputation()     │
```

---

## Contract API Call Summary

| Flow | Step | Contract | Method | Parameters | Value (ETH) |
|------|------|----------|--------|------------|-------------|
| **Registration** | 1 | NodeRegistry | `registerNode()` | resourceType, metadataURI | 0.6 ETH (0.1 fee + 0.5 stake) |
| | 2 | PoWVerifier | `issueChallenge()` | nodeId | - |
| | 3 | PoWVerifier | `submitSolution()` | challengeId, nonce | - |
| | 4 | NodeRegistry | `addStake()` | nodeId | Additional ETH for higher trust |
| **Task** | 1 | TaskMarketplace | `createTask()` | type, power, duration, price, trustLevel, specURI | Task payment in ETH |
| | 2 | TaskMarketplace | `acceptTask()` | taskId, nodeId | - |
| | 3 | TaskMarketplace | `submitResult()` | taskId, resultHash, resultURI | - |
| | 4 | TaskMarketplace | `approveResult()` | taskId | - |
| **Dispute** | 1 | TaskMarketplace | `disputeTask()` | taskId, reason | - |
| | 2 | Admin/Owner | `resolveDispute()` | taskId, winner, split | - |
| | 3 | EscrowManager | `refund()` / `release()` | taskId | - |
| **Withdrawal** | 1 | NodeRegistry | `withdrawStake()` | nodeId, amount | - (receives ETH back) |

---

## Dispute Resolution Cases

| Case | Escrow Action | Provider Impact |
|------|---------------|-----------------|
| **Buyer Wins** (provider failed) | Refund 98% to buyer (2% arbitration fee) | -500 reputation, -5% stake slashed |
| **Provider Wins** (buyer wrong) | Release 90% task + 2% penalty from buyer | +100 reputation |
| **Split Decision** (partial fault) | X% to buyer, Y% to provider, 2% to arbitrators | Varies |

---

## Gas Optimization Strategies

| Optimization | Description | Estimated Savings |
|-------------|-------------|-------------------|
| Packed Structs | Order struct fields by size | 15-20% storage |
| Events over Storage | Use events for historical data | 80% write cost |
| Batch Operations | Group multiple updates | 30-40% per batch |
| Merkle Proofs | For bulk reward claims | 60% verification |
| Proxy Pattern | Upgradeable contracts | Reduced redeploy cost |

---

## Security Considerations

| Risk | Mitigation |
|------|------------|
| Reentrancy | ReentrancyGuard on all payment functions |
| Front-running | Commit-reveal for task assignment |
| Oracle Manipulation | Multiple price feeds + time-weighted average |
| Stake Griefing | Minimum stake requirements + cooldown periods |
| Flash Loan Attacks | Snapshot voting weights + timelock |

---

## Target Blockchain

- **Network:** Ethereum (Sepolia testnet → Mainnet)
- **Solidity Version:** 0.8.20
- **Framework:** Foundry
- **Dependencies:** OpenZeppelin Contracts
- **Payment Tokens:** ETH + USDC
- **Governance:** Multi-sig wallet (Gnosis Safe), owner-controlled