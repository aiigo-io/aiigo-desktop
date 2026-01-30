# AIIGO Computing Power Marketplace - System Design

## Executive Summary

A decentralized marketplace connecting **idle computing resource providers** with **AI/compute buyers**, enabling passive income for device owners while providing affordable distributed computing power.

---

## Business Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           AIIGO Marketplace                                 │
│                                                                             │
│   ┌───────────────┐                              ┌───────────────┐          │
│   │   PROVIDERS   │                              │    BUYERS     │          │
│   │               │         ┌─────────┐          │               │          │
│   │ • GPU Owners  │────────▶│ MATCHING│◀─────────│ • AI Startups │          │
│   │ • CPU Owners  │         │ ENGINE  │          │ • Researchers │          │
│   │ • Network     │         └────┬────┘          │ • Enterprises │          │
│   │ • Mobile      │              │               │ • Developers  │          │
│   │ • IoT/Edge    │              ▼               │               │          │
│   └───────────────┘         ┌─────────┐          └───────────────┘          │
│          │                  │ PAYMENT │                  │                  │
│          │                  │ ESCROW  │                  │                  │
│          │                  └────┬────┘                  │                  │
│          │                       │                       │                  │
│          ▼                       ▼                       ▼                  │
│   ┌─────────────────────────────────────────────────────────────────┐      │
│   │                      REWARD DISTRIBUTION                        │      │
│   │  Provider Revenue (92%) ◀────────────▶ Platform Fee (8%)        │      │
│   └─────────────────────────────────────────────────────────────────┘      │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────┐      │
│   │                    PLATFORM DEDICATED NODES                     │      │
│   │        (AIIGO-owned infrastructure for guaranteed capacity)     │      │
│   └─────────────────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## User Roles

### 1. Resource Providers
Users who contribute idle computing power to earn passive income.

| Provider Type | Resource | Typical Use Case | Earnings Model |
|--------------|----------|------------------|----------------|
| **GPU Provider** | Gaming PCs, Mining Rigs | AI Training, Rendering | Per GPU-hour |
| **CPU Provider** | Servers, Desktops | Data Processing, Compilation | Per CPU-hour |
| **Network Provider** | Bandwidth, CDN | Data Transfer, Streaming | Per GB transferred |
| **Mobile Provider** | Smartphones, Tablets | Edge Computing, Testing | Per task completed |
| **IoT/Edge Provider** | E-bikes, Smart Devices | Sensor Data, Location Services | Per data point |

### 2. Resource Buyers
Organizations or individuals who need computing power.

| Buyer Type | Typical Needs | Payment Model |
|-----------|---------------|---------------|
| **AI Startups** | Model training, inference | Subscription / Pay-as-you-go |
| **Researchers** | Scientific computing, simulations | Grant-funded / Hourly |
| **Enterprises** | Batch processing, rendering | Volume contracts |
| **Developers** | CI/CD, testing environments | Per-minute billing |

### 3. Platform (AIIGO)
- Operates matching engine
- Maintains dedicated infrastructure for guaranteed SLA
- Collects platform fees (8%)
- Handles dispute resolution
- Manages token economics

---

## System Architecture

### High-Level Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLIENT LAYER                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                 │
│   │ Desktop App  │    │  Mobile App  │    │   Web Portal │                 │
│   │ (Tauri)      │    │ (React Native)│   │   (React)    │                 │
│   └──────┬───────┘    └──────┬───────┘    └──────┬───────┘                 │
│          │                   │                   │                          │
└──────────┼───────────────────┼───────────────────┼──────────────────────────┘
           │                   │                   │
           ▼                   ▼                   ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              API GATEWAY                                    │
│                    (Authentication, Rate Limiting, Routing)                 │
└──────────────────────────────────┬──────────────────────────────────────────┘
                                   │
           ┌───────────────────────┼───────────────────────┐
           │                       │                       │
           ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   NODE SERVICE  │    │ MATCHING ENGINE │    │  TASK SERVICE   │
│                 │    │                 │    │                 │
│ • Registration  │    │ • Demand/Supply │    │ • Job Queue     │
│ • Health Check  │    │ • Price Oracle  │    │ • Scheduling    │
│ • Verification  │    │ • SLA Matching  │    │ • Monitoring    │
└────────┬────────┘    └────────┬────────┘    └────────┬────────┘
         │                      │                      │
         └──────────────────────┼──────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SIMPLIFIED BLOCKCHAIN LAYER                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │   Node     │  │   Task     │  │   Escrow   │  │    PoW     │            │
│  │  Registry  │  │ Marketplace│  │  Manager   │  │  Verifier  │            │
│  │            │  │            │  │            │  │            │            │
│  │ ETH-based  │  │ ETH/USDC   │  │  Payment   │  │  Compute   │            │
│  │  Staking   │  │  Trading   │  │  Custody   │  │   Proof    │            │
│  └────────────┘  └────────────┘  └────────────┘  └────────────┘            │
│                                                                             │
│  Removed: AIIGO Token, StakingPool, RewardDistributor, Governor (DAO)      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Node Registry
Manages provider registration and resource inventory.

**Functions:**
- Provider onboarding (wallet-based)
- Device registration (hardware specs, location, availability)
- Stake collateral for reputation
- Track uptime and performance metrics

**Node States:**
```
[Pending] ──▶ [Verified] ──▶ [Active] ──▶ [Earning]
                  │              │
                  ▼              ▼
              [Rejected]    [Inactive] ──▶ [Slashed]
```

### 2. Matching Engine
Connects buyers with optimal providers.

**Matching Criteria:**
```
┌─────────────────────────────────────────────────────────┐
│                    MATCHING ALGORITHM                   │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Buyer Request:                                         │
│  ┌─────────────────────────────────────────────┐       │
│  │ • Resource Type (GPU/CPU/Network)           │       │
│  │ • Computing Power Required (TFLOPS/Cores)   │       │
│  │ • Duration (hours/days)                     │       │
│  │ • Geographic Preference (region/latency)    │       │
│  │ • Budget (max price per hour)               │       │
│  │ • SLA Requirements (uptime %)               │       │
│  └─────────────────────────────────────────────┘       │
│                         │                               │
│                         ▼                               │
│  ┌─────────────────────────────────────────────┐       │
│  │           MATCHING SCORE CALCULATION        │       │
│  │                                             │       │
│  │  Score = w1×Price + w2×Reputation +         │       │
│  │          w3×Latency + w4×Uptime +           │       │
│  │          w5×VerifiedPower                   │       │
│  └─────────────────────────────────────────────┘       │
│                         │                               │
│                         ▼                               │
│              [Ranked Provider List]                     │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 3. Task Execution System
Manages job lifecycle from submission to completion.

**Task Flow:**
```
[Submit] → [Queue] → [Match] → [Assign] → [Execute] → [Verify] → [Pay]
              │                    │           │          │
              ▼                    ▼           ▼          ▼
          [Timeout]           [Reassign]   [Failed]  [Dispute]
```

### 4. Payment & Escrow System
Handles secure fund transfers.

**Payment Flow:**
```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    BUYER     │     │    ESCROW    │     │   PROVIDER   │
│              │     │              │     │              │
│  Deposits    │────▶│   Holds      │     │              │
│  ETH/USDC    │     │   Funds      │     │              │
│              │     │              │     │              │
│              │     │   Task       │     │   Executes   │
│              │     │   Completed  │────▶│   Task       │
│              │     │              │     │              │
│              │     │   Releases   │────▶│   Receives   │
│              │     │   Payment    │     │   92%        │
│              │     │              │     │              │
│              │     │   Platform   │     │              │
│              │     │   Fee: 8%    │     │              │
└──────────────┘     └──────────────┘     └──────────────┘
```

### 5. Proof-of-Work Verification
Verifies real computing power through challenges.

**Verification Process:**
```
┌─────────────────────────────────────────────────────────┐
│              COMPUTING POWER VERIFICATION               │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  1. Platform issues cryptographic challenge             │
│     └── Random seed + difficulty target                 │
│                                                         │
│  2. Provider's device computes solution                 │
│     └── Find nonce where hash < difficulty              │
│                                                         │
│  3. Solution submitted on-chain                         │
│     └── Verified in smart contract                      │
│                                                         │
│  4. Performance score calculated                        │
│     └── Faster solve = higher verified power            │
│                                                         │
│  5. Reputation updated                                  │
│     └── Success: +reputation, Fail: -reputation         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### 6. Platform Dedicated Nodes
AIIGO-owned infrastructure for guaranteed capacity.

**Purpose:**
- Fulfill SLA guarantees when community nodes unavailable
- Handle overflow demand
- Provide baseline capacity for enterprise clients
- Act as benchmark for pricing

---

## Simplified Economics (ETH-Only)

### Payment Flow

```
┌─────────────────────────────────────────────────────────┐
│                ETH PAYMENT FLOW (SIMPLIFIED)            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐                                        │
│  │  PROVIDERS  │                                        │
│  │             │                                        │
│  │  • Deposit  │──────────┐                             │
│  │    ETH      │          │                             │
│  │    stake    │          ▼                             │
│  │             │    ┌───────────┐                       │
│  │  • Earn ETH │◀───│   NODE    │                       │
│  │    directly │    │ REGISTRY  │                       │
│  └─────────────┘    └───────────┘                       │
│                                                         │
│  ┌─────────────┐                                        │
│  │   BUYERS    │                                        │
│  │             │                                        │
│  │  • Pay ETH  │    ┌───────────┐                       │
│  │  • Pay USDC │───▶│  ESCROW   │                       │
│  │             │    │  MANAGER  │                       │
│  └─────────────┘    └─────┬─────┘                       │
│                           │                             │
│                           ▼                             │
│                    ┌───────────┐                        │
│                    │ TASK DONE │                        │
│                    └─────┬─────┘                        │
│                          │                              │
│        ┌─────────────────┼─────────────────┐            │
│        │                 │                 │            │
│        ▼                 ▼                 ▼            │
│   92% to            8% Platform      Treasury           │
│   Provider              Fee          Accumulates        │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Fee Structure (ETH-Based)

| Action | Fee | Recipient |
|--------|-----|-----------|
| Node Registration | 0.1 ETH | Platform Treasury (one-time) |
| Task Completion | 8% of task value | Platform Treasury |
| Dispute Resolution | 2% of escrowed amount | Arbitrator pool |
| Early Stake Withdrawal | No penalty (cooldown period only) |
| Slashing (violations) | Up to 50% of stake | Platform Treasury |

---

## Provider Earnings Model (Simplified)

### Revenue Calculation

```
Provider Earnings = Task Revenue × 92%

Where:
- Task Revenue = Hours × Hourly Rate × Utilization
- No staking rewards (just collateral)
- No token bonuses (simplified marketplace)
```

**Example Calculation:**
```
Task Value: 1 ETH for 10 hours of GPU compute
Provider receives: 1 ETH × 92% = 0.92 ETH
Platform keeps: 1 ETH × 8% = 0.08 ETH
```

### Example Earnings (Monthly)

| Resource Type | Hourly Rate | Utilization | Monthly Earnings |
|--------------|-------------|-------------|------------------|
| High-end GPU (RTX 4090) | $0.80 | 60% | ~$350 |
| Mid-range GPU (RTX 3080) | $0.40 | 60% | ~$175 |
| Server CPU (32 cores) | $0.15 | 70% | ~$75 |
| Network (1Gbps) | $0.02/GB | 500GB/day | ~$300 |
| Mobile Device | $0.01/task | 100 tasks/day | ~$30 |

---

## Platform Dedicated Infrastructure

### Deployment Strategy

```
┌─────────────────────────────────────────────────────────┐
│              PLATFORM INFRASTRUCTURE                    │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  TIER 1: Enterprise Grade (Owned)                       │
│  ┌─────────────────────────────────────────────┐       │
│  │ • Data centers in 3 regions (US, EU, Asia)  │       │
│  │ • 99.99% SLA guarantee                      │       │
│  │ • Reserved for enterprise contracts         │       │
│  │ • Serves as pricing benchmark               │       │
│  └─────────────────────────────────────────────┘       │
│                                                         │
│  TIER 2: Cloud Partnerships (Leased)                    │
│  ┌─────────────────────────────────────────────┐       │
│  │ • AWS/GCP/Azure reserved instances          │       │
│  │ • Elastic scaling for demand spikes         │       │
│  │ • Geographic redundancy                     │       │
│  └─────────────────────────────────────────────┘       │
│                                                         │
│  TIER 3: Community Nodes (Marketplace)                  │
│  ┌─────────────────────────────────────────────┐       │
│  │ • Distributed providers worldwide           │       │
│  │ • Variable availability                     │       │
│  │ • Price competition                         │       │
│  └─────────────────────────────────────────────┘       │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Capacity Planning

| Tier | Capacity Target | Purpose |
|------|-----------------|---------|
| Tier 1 (Owned) | 20% of demand | Baseline + Enterprise SLA |
| Tier 2 (Cloud) | 30% of demand | Overflow + Burst capacity |
| Tier 3 (Community) | 50% of demand | Cost efficiency + Growth |

---

## Security & Trust

### Provider Verification Levels (ETH-Based)

```
┌─────────────────────────────────────────────────────────┐
│              ETH-BASED TRUST LEVELS                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  LEVEL 1: Basic                                         │
│  ├── ETH stake: 0.5 ETH minimum                         │
│  ├── Registration fee paid: 0.1 ETH                     │
│  └── Access: Small tasks only (<$100 value)             │
│                                                         │
│  LEVEL 2: Verified                                      │
│  ├── ETH stake: 1+ ETH                                  │
│  ├── 10+ successful PoW challenges                      │
│  ├── Basic reputation (50%+ success)                    │
│  └── Access: Standard marketplace (all public tasks)    │
│                                                         │
│  LEVEL 3: Trusted                                       │
│  ├── ETH stake: 3+ ETH                                  │
│  ├── 100+ tasks completed                               │
│  ├── 95%+ success rate                                  │
│  └── Access: Enterprise + Priority matching             │
│                                                         │
│  LEVEL 4: Partner                                       │
│  ├── ETH stake: 5+ ETH                                  │
│  ├── SLA commitment signed                              │
│  ├── Insurance bond (additional)                        │
│  ├── Optional business verification                     │
│  └── Access: Direct enterprise contracts + Whitelabel   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Slashing Conditions

| Violation | Penalty |
|-----------|---------|
| Failed PoW challenge | -2% reputation |
| Task timeout | -5% stake |
| Data breach/Misuse | -50% stake + Ban |
| Repeated failures (>20%) | Review + Potential ban |

---

## Solidity Implementation Design

### Simplified Architecture (Marketplace Only)

This design focuses purely on the computing power marketplace without token governance complexity.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                   SIMPLIFIED SMART CONTRACT ARCHITECTURE                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         CORE CONTRACTS                              │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                     │   │
│  │                      ┌──────────────┐                              │   │
│  │                      │ NodeRegistry │                              │   │
│  │                      │              │                              │   │
│  │                      │ • ETH Stake  │                              │   │
│  │                      │ • Reputation │                              │   │
│  │                      └──────┬───────┘                              │   │
│  │                             │                                      │   │
│  │                             ▼                                      │   │
│  │  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐       │   │
│  │  │   Escrow     │◀───▶│    Task      │◀───▶│    PoW       │       │   │
│  │  │   Manager    │     │ Marketplace  │     │  Verifier    │       │   │
│  │  │              │     │              │     │              │       │   │
│  │  │ ETH/USDC     │     │ ETH/USDC     │     │ Verify Power │       │   │
│  │  │ Custody      │     │ Payments     │     │              │       │   │
│  │  └──────────────┘     └──────────────┘     └──────────────┘       │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       UTILITY CONTRACTS                             │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                     │   │
│  │  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐       │   │
│  │  │   Ownable    │     │  Pausable    │     │ReentrancyGua │       │   │
│  │  │(OpenZeppelin)│     │(OpenZeppelin)│     │(OpenZeppelin)│       │   │
│  │  └──────────────┘     └──────────────┘     └──────────────┘       │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  Key Changes:                                                               │
│  • Removed: AIIGOToken (no governance token)                                │
│  • Removed: StakingPool (no token staking)                                  │
│  • Removed: RewardDistributor (direct ETH payments)                         │
│  • Removed: Governor (no DAO - owner-controlled)                            │
│  • Simplified: ETH-based deposits and payments only                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Payment Flow (Simplified):**
```
Buyer deposits ETH/USDC → Escrow → Task completed → Provider receives 92% → Platform 8%
No tokens, no staking rewards, no governance - just pure marketplace economics
```

### Simplified Contract Specifications

#### 1. NodeRegistry.sol
**Purpose:** Manages provider registration, status, and hardware inventory with ETH-based deposits

```solidity
// Key data structures
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

// Key functions
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

// Events
event NodeRegistered(bytes32 indexed nodeId, address indexed owner, ResourceType resourceType);
event NodeStatusChanged(bytes32 indexed nodeId, NodeStatus oldStatus, NodeStatus newStatus);
event StakeDeposited(bytes32 indexed nodeId, uint256 amount, uint256 totalStake);
event StakeWithdrawn(bytes32 indexed nodeId, uint256 amount, uint256 totalStake);
event ReputationUpdated(bytes32 indexed nodeId, uint256 oldReputation, uint256 newReputation);
event NodeSlashed(bytes32 indexed nodeId, uint256 slashedAmount, string reason);
```

**Trust Level Calculation (ETH-based):**
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

**Stake Requirements:**
- Registration Fee: 0.1 ETH (goes to platform)
- Minimum Stake: 0.5 ETH (refundable, held as collateral)
- Level 2: 1 ETH stake
- Level 3: 3 ETH stake
- Level 4: 5 ETH stake

---

#### 2. TaskMarketplace.sol
**Purpose:** Core marketplace for task creation, matching, and lifecycle management

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

// Key functions
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

// Events
event TaskCreated(bytes32 indexed taskId, address indexed buyer, ResourceType resourceType, uint256 maxPrice);
event TaskAssigned(bytes32 indexed taskId, bytes32 indexed nodeId, uint256 startTime);
event TaskCompleted(bytes32 indexed taskId, bytes32 resultHash);
event TaskVerified(bytes32 indexed taskId, uint256 payoutAmount);
event TaskDisputed(bytes32 indexed taskId, address disputedBy, string reason);
event TaskCancelled(bytes32 indexed taskId, uint256 refundAmount);
```

**Task Matching Logic:**
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

---

#### 5. EscrowManager.sol
**Purpose:** Secure fund custody during task execution

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

// Key functions
interface IEscrowManager {
    function deposit(bytes32 taskId, address buyer) external payable returns (bytes32 escrowId);
    function release(bytes32 taskId, address provider) external;
    function refund(bytes32 taskId) external;
    function splitPayment(bytes32 taskId, address provider, uint256 providerShare) external;
    function getEscrow(bytes32 taskId) external view returns (EscrowDeposit memory);
}

// Events
event EscrowDeposited(bytes32 indexed taskId, address indexed buyer, uint256 amount);
event EscrowReleased(bytes32 indexed taskId, address indexed provider, uint256 providerPayout, uint256 platformFee);
event EscrowRefunded(bytes32 indexed taskId, address indexed buyer, uint256 amount);
event DisputeResolved(bytes32 indexed taskId, address indexed winner, uint256 amount);
```

**Payment Distribution:**
```solidity
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

---

#### 6. ProofOfWorkVerifier.sol
**Purpose:** Verifies provider computing power through cryptographic challenges

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

// Key functions
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

// Events
event ChallengeIssued(bytes32 indexed challengeId, bytes32 indexed nodeId, uint256 difficulty, uint256 deadline);
event ChallengeSolved(bytes32 indexed challengeId, bytes32 indexed nodeId, uint256 solutionTime, uint256 verifiedPower);
event ChallengeFailed(bytes32 indexed challengeId, bytes32 indexed nodeId, string reason);
```

**Challenge Verification:**
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

---

**Note:** Removed token-related contracts (StakingPool, RewardDistributor, AIIGOGovernor) for marketplace simplicity. Platform governance is owner-controlled with multi-sig wallet recommended.

---

### Contract Interaction Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        TASK LIFECYCLE FLOW                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  BUYER                    CONTRACTS                    PROVIDER             │
│    │                         │                            │                 │
│    │  1. createTask()        │                            │                 │
│    ├────────────────────────▶│ TaskMarketplace            │                 │
│    │                         ├────────────────────────────│                 │
│    │  2. deposit ETH         │                            │                 │
│    ├────────────────────────▶│ EscrowManager              │                 │
│    │                         │                            │                 │
│    │                         │                            │ 3. acceptTask() │
│    │                         │◀───────────────────────────┤                 │
│    │                         │ TaskMarketplace            │                 │
│    │                         │                            │                 │
│    │                         │  4. Verify PoW             │                 │
│    │                         │  (if required)             │                 │
│    │                         │◀──────────────────────────▶│                 │
│    │                         │ PoWVerifier                │                 │
│    │                         │                            │                 │
│    │                         │                            │ 5. submitResult │
│    │                         │◀───────────────────────────┤                 │
│    │                         │ TaskMarketplace            │                 │
│    │                         │                            │                 │
│    │  6. approveResult()     │                            │                 │
│    ├────────────────────────▶│                            │                 │
│    │                         │                            │                 │
│    │                         │  7. release()              │                 │
│    │                         ├───────────────────────────▶│                 │
│    │                         │ EscrowManager              │ (92% ETH)       │
│    │                         │                            │                 │
│    │                         │  8. updateReputation()     │                 │
│    │                         │ NodeRegistry               │                 │
│    │                         │                            │                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### Complete Workflow Illustrations

#### Workflow 1: Provider Registration & Onboarding (ETH-Based)

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                  SIMPLIFIED PROVIDER REGISTRATION WORKFLOW                           │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  PROVIDER (User)              SMART CONTRACTS                      BLOCKCHAIN       │
│       │                            │                                    │           │
│       │  1. Connect Wallet         │                                    │           │
│       ├───────────────────────────▶│                                    │           │
│       │                            │                                    │           │
│       │  2. Register Node          │                                    │           │
│       │     + Hardware Metadata    │                                    │           │
│       │     + Send 0.6 ETH         │                                    │           │
│       │     (0.1 fee + 0.5 stake)  │                                    │           │
│       ├───────────────────────────▶│ NodeRegistry.registerNode()        │           │
│       │                            │  ├─ Receive 0.6 ETH                │           │
│       │                            │  ├─ Validate metadata URI          │           │
│       │                            │  ├─ Generate nodeId                │           │
│       │                            │  ├─ Store 0.5 ETH as stake         │           │
│       │                            │  ├─ Send 0.1 ETH to treasury       │           │
│       │                            │  ├─ Set status = Pending           │           │
│       │                            │  └─ Emit NodeRegistered            │           │
│       │                            ├───────────────────────────────────▶│           │
│       │                            │                                    │ ✓ Stored  │
│       │                            │◀───────────────────────────────────┤           │
│       │◀───────────────────────────┤ Return nodeId                      │           │
│       │                            │                                    │           │
│       │  3. Complete PoW Challenge │                                    │           │
│       ├───────────────────────────▶│ PoWVerifier.issueChallenge()       │           │
│       │                            │  ├─ Generate random seed           │           │
│       │                            │  ├─ Set difficulty target          │           │
│       │                            │  └─ Emit ChallengeIssued           │           │
│       │                            ├───────────────────────────────────▶│           │
│       │◀───────────────────────────┤ Return challengeId + seed          │           │
│       │                            │                                    │           │
│       │  [Provider computes        │                                    │           │
│       │   nonce offline]           │                                    │           │
│       │                            │                                    │           │
│       │  4. Submit Solution        │                                    │           │
│       ├───────────────────────────▶│ PoWVerifier.submitSolution()       │           │
│       │                            │  ├─ Verify hash(seed,nonce)        │           │
│       │                            │  ├─ Calculate compute power        │           │
│       │                            │  ├─ Update node reputation         │           │
│       │                            │  └─ Emit ChallengeSolved           │           │
│       │                            ├───────────────────────────────────▶│           │
│       │                            │         │                          │           │
│       │                            │         ▼                          │           │
│       │                            │ NodeRegistry.updateReputation()    │           │
│       │                            │ NodeRegistry.updateComputePower()  │           │
│       │                            │         │                          │           │
│       │                            │◀────────┘                          │           │
│       │◀───────────────────────────┤ Verification passed                │           │
│       │                            │                                    │           │
│       │                            │ [Auto-update status]               │           │
│       │                            │ NodeRegistry.updateNodeStatus()    │           │
│       │                            │  └─ status = Active                │           │
│       │                            ├───────────────────────────────────▶│           │
│       │                            │                                    │           │
│       │  ✅ NODE ACTIVE            │                                    │           │
│       │  Trust Level 1 (Basic)     │                                    │           │
│       │  0.5 ETH staked            │                                    │           │
│       │                            │                                    │           │
└───────┴────────────────────────────┴────────────────────────────────────┴───────────┘

Trust Level Progression (ETH-Based):
┌────────────────────────────────────────────────────────────────────────────────────┐
│  Level 1 → Level 2:  Complete 10+ PoW challenges + Stake 1 ETH total              │
│  Level 2 → Level 3:  Complete 100+ tasks + 95% success + Stake 3 ETH total        │
│  Level 3 → Level 4:  Sign SLA commitment + Stake 5 ETH total + Insurance bond     │
└────────────────────────────────────────────────────────────────────────────────────┘
```

---

#### Workflow 2: Task Creation & Execution (Happy Path)

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                      TASK EXECUTION WORKFLOW (HAPPY PATH)                            │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  BUYER                 CONTRACTS                    PROVIDER               RESULT   │
│    │                      │                            │                      │     │
│    │  1. Create Task      │                            │                      │     │
│    │     + Send ETH       │                            │                      │     │
│    ├─────────────────────▶│ TaskMarketplace            │                      │     │
│    │                      │  .createTask()             │                      │     │
│    │                      │     │                      │                      │     │
│    │                      │     ▼                      │                      │     │
│    │                      │ EscrowManager              │                      │     │
│    │                      │  .deposit()                │                      │     │
│    │                      │  └─ Lock ETH in escrow     │                      │     │
│    │                      │     │                      │                      │     │
│    │                      │◀────┘                      │                      │     │
│    │◀─────────────────────┤ Return taskId              │                      │     │
│    │                      │                            │                      │     │
│    │                      │ [Event: TaskCreated]       │                      │     │
│    │                      │     │                      │                      │     │
│    │                      │     ▼                      │                      │     │
│    │                      │ [Match eligible nodes]     │                      │     │
│    │                      │ NodeRegistry               │                      │     │
│    │                      │  .getActiveNodes()         │                      │     │
│    │                      │  .getNodeTrustLevel()      │                      │     │
│    │                      │                            │                      │     │
│    │                      │                            │ 2. Accept Task       │     │
│    │                      │◀───────────────────────────┤    (sees open task)  │     │
│    │                      │ TaskMarketplace            │                      │     │
│    │                      │  .acceptTask()             │                      │     │
│    │                      │  ├─ Verify node eligible   │                      │     │
│    │                      │  ├─ Check trust level      │                      │     │
│    │                      │  ├─ Assign task to node    │                      │     │
│    │                      │  └─ status = InProgress    │                      │     │
│    │                      │                            │                      │     │
│    │                      │ [Event: TaskAssigned]      │                      │     │
│    │                      ├───────────────────────────▶│                      │     │
│    │                      │                            │                      │     │
│    │                      │                            │ 3. Execute Task      │     │
│    │                      │                            │    [OFF-CHAIN]       │     │
│    │                      │                            │    ├─ Download spec  │     │
│    │                      │                            │    ├─ Run compute    │     │
│    │                      │                            │    ├─ Generate result│     │
│    │                      │                            │    └─ Upload to IPFS │     │
│    │                      │                            │         │            │     │
│    │                      │                            │         ▼            │     │
│    │                      │                            │ 4. Submit Result     │     │
│    │                      │◀───────────────────────────┤    + resultHash      │     │
│    │                      │ TaskMarketplace            │    + resultURI       │     │
│    │                      │  .submitResult()           │                      │     │
│    │                      │  ├─ Verify sender is       │                      │     │
│    │                      │  │  assigned provider      │                      │     │
│    │                      │  ├─ Store result hash      │                      │     │
│    │                      │  └─ status = Completed     │                      │     │
│    │                      │                            │                      │     │
│    │                      │ [Event: TaskCompleted]     │                      │     │
│    │◀─────────────────────┤ Notify buyer               │                      │     │
│    │                      │                            │                      │     │
│    │ 5. Verify Result     │                            │                      │     │
│    │    [OFF-CHAIN]       │                            │                      │     │
│    │    ├─ Download result│                            │                      │     │
│    │    ├─ Validate output│                            │                      │     │
│    │    └─ Confirm quality│                            │                      │     │
│    │         │            │                            │                      │     │
│    │         ▼            │                            │                      │     │
│    │ 6. Approve Result    │                            │                      │     │
│    ├─────────────────────▶│ TaskMarketplace            │                      │     │
│    │                      │  .approveResult()          │                      │     │
│    │                      │  └─ status = Verified      │                      │     │
│    │                      │         │                  │                      │     │
│    │                      │         ▼                  │                      │     │
│    │                      │ EscrowManager.release()    │                      │     │
│    │                      │  ├─ Calculate 92%/8% split │                      │     │
│    │                      │  ├─ Transfer 92% → Provider│─────────────────────▶│     │
│    │                      │  └─ Transfer 8% → Treasury │                      │     │
│    │                      │         │                  │                      │     │
│    │                      │         ▼                  │                      │     │
│    │                      │ NodeRegistry               │                      │     │
│    │                      │  .updateReputation(+)      │                      │     │
│    │                      │  .incrementTaskCount()     │                      │     │
│    │                      │                            │                      │     │
│    │                      │ [Event: TaskVerified]      │                      │     │
│    │◀─────────────────────┤                            │                      │     │
│    │                      │                            │◀─────────────────────┤     │
│    │                      │                            │                      │     │
│    │  ✅ TASK COMPLETE    │                            │  ✅ ETH RECEIVED     │     │
│    │                      │                            │     + REPUTATION UP  │     │
│    │                      │                            │                      │     │
└────┴──────────────────────┴────────────────────────────┴──────────────────────┴─────┘
```

---

#### Workflow 3: Task Dispute Resolution

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                         DISPUTE RESOLUTION WORKFLOW                                  │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  BUYER           CONTRACTS              ARBITRATORS           PROVIDER              │
│    │                │                       │                    │                  │
│    │ 1. Reject      │                       │                    │                  │
│    │    Result      │                       │                    │                  │
│    ├───────────────▶│ TaskMarketplace       │                    │                  │
│    │                │  .disputeTask()       │                    │                  │
│    │                │  ├─ status=Disputed   │                    │                  │
│    │                │  ├─ Lock escrow       │                    │                  │
│    │                │  └─ Start dispute     │                    │                  │
│    │                │       timer           │                    │                  │
│    │                │         │             │                    │                  │
│    │                │ [Event: TaskDisputed] │                    │                  │
│    │                ├────────────────────────────────────────────▶                  │
│    │                │                       │                    │ Notified        │
│    │                │                       │                    │                  │
│    │                │ 2. Arbitrator         │                    │                  │
│    │                │    Selection          │                    │                  │
│    │                │    (Platform owner/   │                    │                  │
│    │                │     multi-sig)        │                    │                  │
│    │                ├──────────────────────▶│                    │                  │
│    │                │                       │ 3. Review          │                  │
│    │                │                       │    Evidence        │                  │
│    │                │                       │    ├─ Task spec    │                  │
│    │                │                       │    ├─ Result data  │                  │
│    │                │                       │    └─ Dispute      │                  │
│    │                │                       │        reason      │                  │
│    │                │                       │         │          │                  │
│    │                │                       │         ▼          │                  │
│    │                │                       │ 4. Vote on         │                  │
│    │                │                       │    Resolution      │                  │
│    │                │◀──────────────────────┤    ├─ Buyer wins   │                  │
│    │                │                       │    ├─ Provider wins│                  │
│    │                │                       │    └─ Split        │                  │
│    │                │                       │                    │                  │
│    │                │ 5. Execute Resolution │                    │                  │
│    │                │                       │                    │                  │
│    │                │ ┌─────────────────────┴────────────────────┴─────────────┐   │
│    │                │ │                                                         │   │
│    │                │ │  CASE A: Buyer Wins (Provider Failed)                   │   │
│    │                │ │  ─────────────────────────────────────                  │   │
│    │                │ │  EscrowManager.refund()                                 │   │
│    │                │ │   └─ Return 98% to buyer (2% arbitration fee)           │   │
│    │                │ │  NodeRegistry.updateReputation(provider, -500)          │   │
│    │                │ │  NodeRegistry.slashStake(provider, 5%)                  │   │
│    │                │ │                                                         │   │
│    │                │ │  CASE B: Provider Wins (Buyer Wrong)                    │   │
│    │                │ │  ────────────────────────────────────                   │   │
│    │                │ │  EscrowManager.release()                                │   │
│    │                │ │   └─ Pay provider (90% task + 2% penalty from buyer)    │   │
│    │                │ │  NodeRegistry.updateReputation(provider, +100)          │   │
│    │                │ │                                                         │   │
│    │                │ │  CASE C: Split Decision (Partial Fault)                 │   │
│    │                │ │  ──────────────────────────────────────                 │   │
│    │                │ │  EscrowManager.splitPayment()                           │   │
│    │                │ │   ├─ X% to buyer                                        │   │
│    │                │ │   ├─ Y% to provider                                     │   │
│    │                │ │   └─ 2% to arbitrators                                  │   │
│    │                │ │                                                         │   │
│    │                │ └─────────────────────────────────────────────────────────┘   │
│    │                │                       │                    │                  │
│    │                │ [Event: DisputeResolved]                   │                  │
│    │◀───────────────┤                       │                    │◀─────────────────│
│    │                │                       │                    │                  │
└────┴────────────────┴───────────────────────┴────────────────────┴──────────────────┘
```

---

#### Workflow 4: ETH Stake Management

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                        ETH STAKE MANAGEMENT WORKFLOW                                 │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  PROVIDER                 CONTRACTS                              RESULT             │
│    │                          │                                    │                │
│    │  ═══════════════════════════════════════════════════════════════              │
│    │  ADD STAKE (Increase Trust Level)                                             │
│    │  ═══════════════════════════════════════════════════════════════              │
│    │                          │                                    │                │
│    │  1. Add Stake            │                                    │                │
│    │     (Send more ETH)      │                                    │                │
│    ├─────────────────────────▶│ NodeRegistry.addStake()            │                │
│    │                          │  ├─ Receive ETH                    │                │
│    │                          │  ├─ Update stakedAmount            │                │
│    │                          │  ├─ Recalculate trust level        │                │
│    │                          │  └─ Emit StakeDeposited            │                │
│    │◀─────────────────────────┤                                    │                │
│    │                          │                                    │                │
│    │  ═══════════════════════════════════════════════════════════════              │
│    │  WITHDRAW STAKE (After node deactivation)                                     │
│    │  ═══════════════════════════════════════════════════════════════              │
│    │                          │                                    │                │
│    │  2. Request Withdrawal   │                                    │                │
│    ├─────────────────────────▶│ NodeRegistry.withdrawStake()       │                │
│    │                          │  ├─ Check node status = Inactive   │                │
│    │                          │  ├─ Check no pending tasks         │                │
│    │                          │  ├─ Apply cooldown period          │                │
│    │                          │  ├─ Transfer ETH back to provider  │────────────────▶
│    │                          │  └─ Emit StakeWithdrawn            │  Receive ETH   │
│    │◀─────────────────────────┤                                    │                │
│    │                          │                                    │                │
└────┴──────────────────────────┴────────────────────────────────────┴────────────────┘

Stake Requirements:
┌────────────────────────────────────────────────────────────────────────────────────┐
│  Trust Level 1 (Basic):      0.5 ETH   → Small tasks (<$100)                      │
│  Trust Level 2 (Verified):   1.0 ETH   → Standard marketplace                     │
│  Trust Level 3 (Trusted):    3.0 ETH   → Enterprise + Priority                    │
│  Trust Level 4 (Partner):    5.0 ETH   → Direct contracts + SLA                   │
└────────────────────────────────────────────────────────────────────────────────────┘
```

---

#### Workflow 5: Platform Governance (Multi-Sig)

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                   PLATFORM GOVERNANCE WORKFLOW (MULTI-SIG)                           │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  OWNER 1         OWNER 2         MULTI-SIG          CONTRACTS        EXECUTION      │
│    │               │                 │                  │                │          │
│    │  1. Propose   │                 │                  │                │          │
│    │     Change    │                 │                  │                │          │
│    ├──────────────────────────────────▶                 │                │          │
│    │               │      Gnosis Safe │                  │                │          │
│    │               │      Dashboard   │                  │                │          │
│    │               │                 │                  │                │          │
│    │               │  2. Review       │                  │                │          │
│    │               ├────────────────▶│                  │                │          │
│    │               │                 │                  │                │          │
│    │               │  3. Approve      │                  │                │          │
│    │               ├────────────────▶│                  │                │          │
│    │               │                 │                  │                │          │
│    │               │                 │ 4. Execute (2/3  │                │          │
│    │               │                 │    signatures)   │                │          │
│    │               │                 ├─────────────────▶│                │          │
│    │               │                 │                  │                │          │
│    │               │                 │  Contract Calls: │                │          │
│    │               │                 │  ├─ updateFee()  │───────────────▶│          │
│    │               │                 │  ├─ slashNode()  │───────────────▶│          │
│    │               │                 │  ├─ pause()      │───────────────▶│          │
│    │               │                 │  └─ ...          │                │ Apply    │
│    │               │                 │                  │                │ Changes  │
│    │               │                 │                  │                │          │
└────┴───────────────┴─────────────────┴──────────────────┴────────────────┴──────────┘

Common Administrative Actions:
┌────────────────────────────────────────────────────────────────────────────────────┐
│  • updatePlatformFee(uint256 newFee)         // Change fee (e.g., 8% → 7%)        │
│  • updateMinStake(uint256 newMinStake)       // Adjust minimum stake requirement  │
│  • slashNode(bytes32 nodeId, uint256 amount) // Penalize malicious provider       │
│  • addTrustedNode(bytes32 nodeId)            // Whitelist enterprise partner       │
│  • pause() / unpause()                       // Emergency circuit breaker          │
│  • setArbitrator(address newArbitrator)      // Update dispute resolver            │
└────────────────────────────────────────────────────────────────────────────────────┘
```

---

#### Workflow 6: Simplified System Overview (ETH-Only)

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                  AIIGO COMPUTING POWER - SIMPLIFIED SYSTEM FLOW                      │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│   PROVIDERS                         PLATFORM                         BUYERS         │
│       │                                │                                │           │
│       │  ┌──────────────────────────────────────────────────────────┐  │           │
│       │  │                    ONBOARDING LAYER                      │  │           │
│       │  └──────────────────────────────────────────────────────────┘  │           │
│       │         │                      │                      │        │           │
│       ▼         ▼                      ▼                      ▼        ▼           │
│  ┌─────────┐ ┌─────────┐         ┌─────────┐           ┌─────────┐ ┌─────────┐    │
│  │ Register│ │  Stake  │         │   PoW   │           │ Connect │ │ Deposit │    │
│  │  Node   │ │  ETH    │         │ Verify  │           │ Wallet  │ │ ETH/USDC│    │
│  └────┬────┘ └────┬────┘         └────┬────┘           └────┬────┘ └────┬────┘    │
│       │           │                   │                     │           │          │
│       ▼           ▼                   ▼                     ▼           ▼          │
│  ┌─────────────────────────────────────────────────────────────────────────────┐  │
│  │                        SMART CONTRACT LAYER                                 │  │
│  │                                                                               │  │
│  │   NodeRegistry       PoWVerifier                         EscrowManager       │  │
│  │   (ETH Stake)        (Compute Proof)                     (ETH/USDC)          │  │
│  │        │                   │                                  │              │  │
│  │        └───────────────────┴──────────────────────────────────┘              │  │
│  │                                   │                                           │  │
│  │                                   ▼                                           │  │
│  │  ┌─────────────────────────────────────────────────────────────────────────┐ │  │
│  │  │                         TASK MARKETPLACE                                 │ │  │
│  │  │                                                                          │ │  │
│  │  │   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐         │ │  │
│  │  │   │  Create  │───▶│  Match   │───▶│ Execute  │───▶│  Verify  │         │ │  │
│  │  │   │  Task    │    │  Nodes   │    │  Task    │    │  Result  │         │ │  │
│  │  │   └──────────┘    └──────────┘    └──────────┘    └──────────┘         │ │  │
│  │  │        │               │               │               │                │ │  │
│  │  │        │               │               │               ▼                │ │  │
│  │  │        │               │               │         ┌──────────┐           │ │  │
│  │  │        │               │               │         │ Dispute? │           │ │  │
│  │  │        │               │               │         └─────┬────┘           │ │  │
│  │  │        │               │               │          YES  │  NO            │ │  │
│  │  │        │               │               │          ┌────┴────┐           │ │  │
│  │  │        │               │               │          ▼         ▼           │ │  │
│  │  │        │               │               │    Multi-Sig   Release         │ │  │
│  │  │        │               │               │   Arbitration  Payment         │ │  │
│  │  │        │               │               │                                │ │  │
│  │  └────────┴───────────────┴───────────────┴────────────────────────────────┘ │  │
│  │                                   │                                           │  │
│  │                                   ▼                                           │  │
│  │  ┌─────────────────────────────────────────────────────────────────────────┐ │  │
│  │  │                      PAYMENT DISTRIBUTION                                │ │  │
│  │  │                                                                          │ │  │
│  │  │      ┌────────────────────────────────────────────────────────┐         │ │  │
│  │  │      │                  ESCROW RELEASE (ETH)                 │         │ │  │
│  │  │      │                                                        │         │ │  │
│  │  │      │   Total Payment (ETH/USDC)                            │         │ │  │
│  │  │      │        │                                               │         │ │  │
│  │  │      │        ├──────────▶  92% ─────▶ Provider Wallet        │         │ │  │
│  │  │      │        │                                               │         │ │  │
│  │  │      │        └──────────▶   8% ─────▶ Platform Treasury      │         │ │  │
│  │  │      │                                                        │         │ │  │
│  │  │      │   No tokens, no bonuses - pure ETH distribution        │         │ │  │
│  │  │      │                                                        │         │ │  │
│  │  │      └────────────────────────────────────────────────────────┘         │ │  │
│  │  │                                                                          │ │  │
│  │  └──────────────────────────────────────────────────────────────────────────┘ │  │
│  │                                   │                                           │  │
│  │                                   ▼                                           │  │
│  │  ┌─────────────────────────────────────────────────────────────────────────┐ │  │
│  │  │                    GOVERNANCE (MULTI-SIG)                                │ │  │
│  │  │                                                                          │ │  │
│  │  │         Platform Owners (Gnosis Safe)                                    │ │  │
│  │  │                │                                                         │ │  │
│  │  │                ▼                                                         │ │  │
│  │  │   ┌────────────────────────────────────────────────────────────┐        │ │  │
│  │  │   │  Propose  │──▶│ Review │──▶│ Sign (2/3) │──▶│ Execute │    │        │ │  │
│  │  │   └────────────────────────────────────────────────────────────┘        │ │  │
│  │  │                │                                                         │ │  │
│  │  │                ▼                                                         │ │  │
│  │  │   ┌────────────────────────────────────────────────────────────┐        │ │  │
│  │  │   │  • Platform fee updates     • Minimum stake changes        │        │ │  │
│  │  │   │  • Slashing malicious nodes • Emergency pause/unpause      │        │ │  │
│  │  │   │  • Arbitrator updates       • Contract upgrades            │        │ │  │
│  │  │   └────────────────────────────────────────────────────────────┘        │ │  │
│  │  │                                                                          │ │  │
│  │  └──────────────────────────────────────────────────────────────────────────┘ │  │
│  │                                                                               │  │
│  └───────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                      │
│  Key Simplifications:                                                                │
│  ✓ No AIIGO token (removed)                                                         │
│  ✓ No StakingPool (ETH stake in NodeRegistry)                                       │
│  ✓ No RewardDistributor (direct ETH payment)                                        │
│  ✓ No DAO (multi-sig governance)                                                    │
│                                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

---

#### Simplified Contract API Call Summary

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

### Simplified Deployment Order

```
1. NodeRegistry.sol         ← Deploy first (standalone, owner-controlled)
2. ProofOfWorkVerifier.sol  ← Depends on NodeRegistry
3. EscrowManager.sol        ← Standalone escrow logic
4. TaskMarketplace.sol      ← Depends on all above (orchestrator contract)
```

**Post-Deployment Configuration:**
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

### Gas Optimization Strategies

| Optimization | Description | Estimated Savings |
|-------------|-------------|-------------------|
| **Packed Structs** | Order struct fields by size | 15-20% storage |
| **Events over Storage** | Use events for historical data | 80% write cost |
| **Batch Operations** | Group multiple updates | 30-40% per batch |
| **Merkle Proofs** | For bulk reward claims | 60% verification |
| **Proxy Pattern** | Upgradeable contracts | Reduced redeploy cost |

---

### Security Considerations

| Risk | Mitigation |
|------|------------|
| **Reentrancy** | ReentrancyGuard on all payment functions |
| **Front-running** | Commit-reveal for task assignment |
| **Oracle Manipulation** | Multiple price feeds + time-weighted average |
| **Stake Griefing** | Minimum stake requirements + cooldown periods |
| **Flash Loan Attacks** | Snapshot voting weights + timelock |

---

## Technical Stack Summary

| Layer | Technology | Notes |
|-------|------------|-------|
| **Smart Contracts** | Solidity 0.8.20, OpenZeppelin, Foundry | 4 core contracts (simplified) |
| **Blockchain** | Ethereum (Sepolia → Mainnet) | ETH + USDC payments |
| **Provider Agent** | Rust (Tauri), GPU libraries | Desktop mining-like agent |
| **Backend Services** | Go/Rust microservices | Off-chain matching + monitoring |
| **API Gateway** | Kong / AWS API Gateway | Rate limiting + auth |
| **Database** | PostgreSQL + Redis | Task queue + cache |
| **Message Queue** | RabbitMQ / Kafka | Task distribution |
| **Monitoring** | Prometheus + Grafana | Performance tracking |
| **Desktop Client** | Tauri + React + TypeScript | Provider + Buyer UI |
| **Mobile Client** | React Native (optional) | Future expansion |
| **Governance** | Multi-sig wallet (Gnosis Safe) | Owner-controlled, no DAO |

---

## Key Metrics & KPIs

| Metric | Target (Year 1) |
|--------|-----------------|
| Active Providers | 10,000+ |
| Active Buyers | 500+ |
| Total Computing Power | 100 PH/s |
| Monthly Transaction Volume | $1M+ |
| Provider Retention | >80% |
| Task Success Rate | >95% |
| Average Provider Earnings | $200/month |

---

## Summary

### Simplified Architecture Benefits

The AIIGO Computing Power Marketplace has been simplified to focus on core marketplace functionality:

**Removed Complexity:**
- ❌ No governance token (AIIGO)
- ❌ No token staking rewards
- ❌ No DAO governance
- ❌ No token economics

**Core Focus:**
- ✅ Pure marketplace for computing power trading
- ✅ ETH-based deposits and payments (+ USDC support)
- ✅ Simple stake-based reputation system
- ✅ Direct provider earnings (92% of task value)
- ✅ Owner/multi-sig controlled platform

**Three-Sided Market:**
1. **Providers** earn ETH from idle devices (92% revenue share)
2. **Buyers** pay ETH/USDC for computing power
3. **Platform** takes 8% fee + collects registration fees

**Key Advantages:**
- Lower complexity = faster development + fewer attack vectors
- No token speculation = focus on real utility
- ETH-based = better liquidity + no token launch required
- Simpler governance = faster decision making
- Easier audit = 4 contracts instead of 8

The combination of community-driven supply with platform-owned infrastructure ensures both flexibility and reliability, making it viable for both casual users and enterprise clients.
