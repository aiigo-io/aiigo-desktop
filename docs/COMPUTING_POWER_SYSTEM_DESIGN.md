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
│                           BLOCKCHAIN LAYER                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐            │
│  │  AIIGO     │  │   Node     │  │   Task     │  │  Staking   │            │
│  │  Token     │  │  Registry  │  │ Marketplace│  │   Pool     │            │
│  └────────────┘  └────────────┘  └────────────┘  └────────────┘            │
│                                                                             │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐                            │
│  │   Escrow   │  │   PoW      │  │ Governor   │                            │
│  │  Contract  │  │ Verifier   │  │   (DAO)    │                            │
│  └────────────┘  └────────────┘  └────────────┘                            │
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

## Token Economics

### AIIGO Token Utility

```
┌─────────────────────────────────────────────────────────┐
│                    AIIGO TOKEN FLOW                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────┐                                        │
│  │  PROVIDERS  │                                        │
│  │             │                                        │
│  │  • Stake to │──────────┐                             │
│  │    register │          │                             │
│  │             │          ▼                             │
│  │  • Earn     │    ┌───────────┐                       │
│  │    rewards  │◀───│  STAKING  │                       │
│  └─────────────┘    │   POOL    │                       │
│                     └─────┬─────┘                       │
│                           │                             │
│  ┌─────────────┐          │                             │
│  │   BUYERS    │          │                             │
│  │             │          ▼                             │
│  │  • Pay with │    ┌───────────┐     ┌───────────┐    │
│  │    ETH/USDC │───▶│  PLATFORM │────▶│ TREASURY  │    │
│  │             │    │   FEES    │     │           │    │
│  │  • Discount │    └───────────┘     └─────┬─────┘    │
│  │    with     │                            │          │
│  │    AIIGO    │                            ▼          │
│  └─────────────┘                      ┌───────────┐    │
│                                       │ GOVERNANCE│    │
│  ┌─────────────┐                      │   (DAO)   │    │
│  │  STAKERS    │                      └───────────┘    │
│  │             │                                        │
│  │  • Lock     │                                        │
│  │    tokens   │                                        │
│  │             │                                        │
│  │  • Earn     │                                        │
│  │    yield    │                                        │
│  │             │                                        │
│  │  • Vote on  │                                        │
│  │    proposals│                                        │
│  └─────────────┘                                        │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Token Distribution

| Allocation | Percentage | Vesting |
|------------|------------|---------|
| Community Rewards | 40% | Released over 4 years |
| Team & Advisors | 15% | 1 year cliff, 3 year vest |
| Platform Treasury | 20% | DAO controlled |
| Private Sale | 10% | 6 month cliff, 2 year vest |
| Public Sale | 10% | Immediate |
| Ecosystem Fund | 5% | DAO controlled |

### Fee Structure

| Action | Fee | Recipient |
|--------|-----|-----------|
| Task Completion | 8% | Platform Treasury |
| Early Unstake | 5% | Burn |
| Dispute Resolution | 2% | Arbitrators |
| Node Registration | 0.1 ETH | Treasury |

---

## Provider Earnings Model

### Revenue Calculation

```
Provider Earnings = (Task Revenue × 92%) + Staking Rewards + Bonuses

Where:
- Task Revenue = Hours × Hourly Rate × Utilization
- Staking Rewards = Staked Amount × APY × Time
- Bonuses = Referral + Uptime + Performance
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

### Provider Verification Levels

```
┌─────────────────────────────────────────────────────────┐
│                 TRUST LEVELS                            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  LEVEL 1: Basic                                         │
│  ├── Wallet connected                                   │
│  ├── Minimum stake (100 AIIGO)                          │
│  └── Access: Small tasks only                           │
│                                                         │
│  LEVEL 2: Verified                                      │
│  ├── 10+ successful PoW challenges                      │
│  ├── Medium stake (1,000 AIIGO)                         │
│  └── Access: Standard marketplace                       │
│                                                         │
│  LEVEL 3: Trusted                                       │
│  ├── 100+ tasks completed                               │
│  ├── 95%+ success rate                                  │
│  ├── High stake (10,000 AIIGO)                          │
│  └── Access: Enterprise + Priority matching             │
│                                                         │
│  LEVEL 4: Partner                                       │
│  ├── Business verification (optional)                   │
│  ├── SLA commitment                                     │
│  ├── Insurance bond                                     │
│  └── Access: Direct enterprise contracts                │
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

### Smart Contract Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SMART CONTRACT ARCHITECTURE                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                         CORE CONTRACTS                              │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                     │   │
│  │  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐       │   │
│  │  │  AIIGOToken  │     │ NodeRegistry │     │   Staking    │       │   │
│  │  │   (ERC20)    │────▶│              │◀───▶│    Pool      │       │   │
│  │  └──────────────┘     └──────┬───────┘     └──────────────┘       │   │
│  │                              │                                     │   │
│  │                              ▼                                     │   │
│  │  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐       │   │
│  │  │    Escrow    │◀───▶│    Task      │◀───▶│   PoW        │       │   │
│  │  │   Manager    │     │  Marketplace │     │  Verifier    │       │   │
│  │  └──────────────┘     └──────────────┘     └──────────────┘       │   │
│  │                              │                                     │   │
│  │                              ▼                                     │   │
│  │  ┌──────────────┐     ┌──────────────┐                            │   │
│  │  │   Reward     │     │  Governor    │                            │   │
│  │  │ Distributor  │     │    (DAO)     │                            │   │
│  │  └──────────────┘     └──────────────┘                            │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                       UTILITY CONTRACTS                             │   │
│  ├─────────────────────────────────────────────────────────────────────┤   │
│  │                                                                     │   │
│  │  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐       │   │
│  │  │AccessControl │     │  Pausable    │     │ReentrancyGua │       │   │
│  │  │(OpenZeppelin)│     │(OpenZeppelin)│     │(OpenZeppelin)│       │   │
│  │  └──────────────┘     └──────────────┘     └──────────────┘       │   │
│  │                                                                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Contract Specifications

#### 1. AIIGOToken.sol
**Purpose:** ERC20 governance token with vote delegation support

```solidity
// SPDX-License-Identifier: MIT
// Key interfaces and data structures

interface IAIIGOToken {
    // ERC20 + ERC20Votes standard functions
    function mint(address to, uint256 amount) external;
    function burn(uint256 amount) external;

    // Governance
    function delegate(address delegatee) external;
    function getVotes(address account) external view returns (uint256);
    function getPastVotes(address account, uint256 blockNumber) external view returns (uint256);
}

// Events
event TokensMinted(address indexed to, uint256 amount);
event TokensBurned(address indexed from, uint256 amount);
event DelegateChanged(address indexed delegator, address indexed fromDelegate, address indexed toDelegate);
```

**Key Features:**
- ERC20Votes extension for governance
- Minting controlled by RewardDistributor
- Burning mechanism for penalties
- Total supply: 1,000,000,000 AIIGO

---

#### 2. NodeRegistry.sol
**Purpose:** Manages provider registration, status, and hardware inventory

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
    uint256 stakedAmount;     // AIIGO tokens staked
    uint256 reputation;       // 0-10000 (basis points)
    uint256 totalTasksCompleted;
    uint256 totalEarnings;
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
    function registerNode(
        ResourceType resourceType,
        string calldata metadataURI
    ) external payable returns (bytes32 nodeId);

    function stakeForNode(bytes32 nodeId, uint256 amount) external;
    function unstake(bytes32 nodeId, uint256 amount) external;
    function updateNodeStatus(bytes32 nodeId, NodeStatus status) external;
    function updateReputation(bytes32 nodeId, int256 delta) external;
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

**Trust Level Calculation:**
```solidity
function getNodeTrustLevel(bytes32 nodeId) public view returns (uint8) {
    Node memory node = nodes[nodeId];

    // Level 4: Partner (10,000+ AIIGO, 95%+ success, SLA commitment)
    if (node.stakedAmount >= 10000e18 &&
        node.reputation >= 9500 &&
        hasPartnerSLA[nodeId]) {
        return 4;
    }

    // Level 3: Trusted (10,000+ AIIGO, 95%+ success rate, 100+ tasks)
    if (node.stakedAmount >= 10000e18 &&
        node.reputation >= 9500 &&
        node.totalTasksCompleted >= 100) {
        return 3;
    }

    // Level 2: Verified (1,000+ AIIGO, 10+ PoW challenges)
    if (node.stakedAmount >= 1000e18 && powChallengesPassed[nodeId] >= 10) {
        return 2;
    }

    // Level 1: Basic (100+ AIIGO, wallet connected)
    if (node.stakedAmount >= 100e18) {
        return 1;
    }

    return 0; // Not registered
}
```

---

#### 3. StakingPool.sol
**Purpose:** Manages AIIGO token staking for providers and yield distribution

```solidity
struct StakeInfo {
    uint256 amount;
    uint256 startTime;
    uint256 lockPeriod;       // Minimum lock duration
    uint256 rewardDebt;       // For reward calculation
    uint256 pendingRewards;
}

// Key functions
interface IStakingPool {
    function stake(uint256 amount, uint256 lockPeriod) external;
    function unstake(uint256 amount) external;
    function claimRewards() external;
    function getStakeInfo(address staker) external view returns (StakeInfo memory);
    function getPendingRewards(address staker) external view returns (uint256);
    function getAPY() external view returns (uint256);
    function getTotalStaked() external view returns (uint256);
}

// Events
event Staked(address indexed staker, uint256 amount, uint256 lockPeriod);
event Unstaked(address indexed staker, uint256 amount, uint256 penalty);
event RewardsClaimed(address indexed staker, uint256 amount);
event APYUpdated(uint256 oldAPY, uint256 newAPY);
```

**Early Unstake Penalty:**
```solidity
function calculateUnstakePenalty(address staker) public view returns (uint256) {
    StakeInfo memory info = stakes[staker];

    if (block.timestamp >= info.startTime + info.lockPeriod) {
        return 0; // No penalty after lock period
    }

    // 5% penalty for early unstake
    return (info.amount * EARLY_UNSTAKE_PENALTY) / 10000;
}
```

---

#### 4. TaskMarketplace.sol
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

#### 7. RewardDistributor.sol
**Purpose:** Manages reward distribution to providers and stakers

```solidity
struct RewardPool {
    uint256 totalRewards;
    uint256 distributedRewards;
    uint256 rewardRate;       // Rewards per second
    uint256 lastUpdateTime;
    uint256 rewardPerTokenStored;
}

// Key functions
interface IRewardDistributor {
    function distributeTaskReward(bytes32 nodeId, uint256 amount) external;
    function distributeStakingReward(address staker) external;
    function addRewardsToPool(uint256 amount) external;
    function claimRewards(bytes32 nodeId) external;
    function getPendingRewards(bytes32 nodeId) external view returns (uint256);
    function calculateBonus(bytes32 nodeId) external view returns (uint256);
}

// Events
event TaskRewardDistributed(bytes32 indexed nodeId, uint256 amount);
event StakingRewardDistributed(address indexed staker, uint256 amount);
event BonusAwarded(bytes32 indexed nodeId, string bonusType, uint256 amount);
event RewardsPoolReplenished(uint256 amount);
```

**Bonus Calculation:**
```solidity
function calculateBonus(bytes32 nodeId) public view returns (uint256) {
    INodeRegistry.Node memory node = nodeRegistry.getNode(nodeId);
    uint256 bonus = 0;

    // Uptime bonus: 5% extra for 99%+ uptime
    if (getNodeUptime(nodeId) >= 9900) {
        bonus += (node.totalEarnings * 500) / 10000; // 5%
    }

    // Performance bonus: 3% for top 10% performers
    if (isTopPerformer(nodeId)) {
        bonus += (node.totalEarnings * 300) / 10000; // 3%
    }

    // Referral bonus: 2% for each referred node
    bonus += referralCount[node.owner] * REFERRAL_BONUS;

    return bonus;
}
```

---

#### 8. AIIGOGovernor.sol
**Purpose:** Token-weighted DAO governance for protocol decisions

```solidity
// Key functions (extends OpenZeppelin Governor)
interface IAIIGOGovernor {
    function propose(
        address[] memory targets,
        uint256[] memory values,
        bytes[] memory calldatas,
        string memory description
    ) external returns (uint256 proposalId);

    function castVote(uint256 proposalId, uint8 support) external returns (uint256 weight);
    function execute(uint256 proposalId) external;

    // Custom governance functions
    function updatePlatformFee(uint256 newFee) external;
    function updateMinStake(uint256 newMinStake) external;
    function addTrustedNode(bytes32 nodeId) external;
    function slashNode(bytes32 nodeId, uint256 amount, string calldata reason) external;
}

// Governance parameters
uint256 public constant VOTING_DELAY = 1 days;      // Time before voting starts
uint256 public constant VOTING_PERIOD = 7 days;     // Duration of voting
uint256 public constant PROPOSAL_THRESHOLD = 100000e18;  // 100,000 AIIGO to propose
uint256 public constant QUORUM = 4;                 // 4% of total supply
```

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
│    │                         │ EscrowManager              │ (92% payment)   │
│    │                         │                            │                 │
│    │                         │  8. distributeReward()     │                 │
│    │                         │ RewardDistributor          │                 │
│    │                         ├───────────────────────────▶│                 │
│    │                         │                            │ (bonus tokens)  │
│    │                         │                            │                 │
│    │                         │  9. updateReputation()     │                 │
│    │                         │ NodeRegistry               │                 │
│    │                         │                            │                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### Complete Workflow Illustrations

#### Workflow 1: Provider Registration & Onboarding

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                     PROVIDER REGISTRATION WORKFLOW                                   │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  PROVIDER (User)              SMART CONTRACTS                      BLOCKCHAIN       │
│       │                            │                                    │           │
│       │  1. Connect Wallet         │                                    │           │
│       ├───────────────────────────▶│                                    │           │
│       │                            │                                    │           │
│       │  2. Approve AIIGO tokens   │                                    │           │
│       ├───────────────────────────▶│ AIIGOToken.approve()               │           │
│       │                            ├───────────────────────────────────▶│           │
│       │                            │                                    │ ✓ Approved│
│       │                            │◀───────────────────────────────────┤           │
│       │                            │                                    │           │
│       │  3. Register Node          │                                    │           │
│       │     + Hardware Metadata    │                                    │           │
│       │     + 0.1 ETH Fee          │                                    │           │
│       ├───────────────────────────▶│ NodeRegistry.registerNode()        │           │
│       │                            │  ├─ Validate metadata URI          │           │
│       │                            │  ├─ Generate nodeId                │           │
│       │                            │  ├─ Set status = Pending           │           │
│       │                            │  └─ Emit NodeRegistered            │           │
│       │                            ├───────────────────────────────────▶│           │
│       │                            │                                    │ ✓ Stored  │
│       │                            │◀───────────────────────────────────┤           │
│       │◀───────────────────────────┤ Return nodeId                      │           │
│       │                            │                                    │           │
│       │  4. Stake AIIGO Tokens     │                                    │           │
│       │     (min 100 AIIGO)        │                                    │           │
│       ├───────────────────────────▶│ NodeRegistry.stakeForNode()        │           │
│       │                            │  ├─ Transfer tokens to contract    │           │
│       │                            │  ├─ Update stakedAmount            │           │
│       │                            │  └─ Emit StakeDeposited            │           │
│       │                            ├───────────────────────────────────▶│           │
│       │                            │◀───────────────────────────────────┤           │
│       │◀───────────────────────────┤ Stake confirmed                    │           │
│       │                            │                                    │           │
│       │  5. Complete PoW Challenge │                                    │           │
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
│       │  6. Submit Solution        │                                    │           │
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
│       │                            │                                    │           │
└───────┴────────────────────────────┴────────────────────────────────────┴───────────┘

Trust Level Progression:
┌────────────────────────────────────────────────────────────────────────────────────┐
│  Level 1 → Level 2:  Complete 10+ PoW challenges + Stake 1,000 AIIGO              │
│  Level 2 → Level 3:  Complete 100+ tasks + 95% success + Stake 10,000 AIIGO       │
│  Level 3 → Level 4:  Sign SLA commitment + Insurance bond (enterprise)            │
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
│    │                      │ RewardDistributor          │                      │     │
│    │                      │  .distributeTaskReward()   │                      │     │
│    │                      │  ├─ Calculate bonus tokens │                      │     │
│    │                      │  └─ Mint AIIGO to provider │─────────────────────▶│     │
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
│    │  ✅ TASK COMPLETE    │                            │  ✅ PAYMENT RECEIVED │     │
│    │                      │                            │     + BONUS TOKENS   │     │
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
│    │                │    (DAO members with  │                    │                  │
│    │                │    stake > 10k AIIGO) │                    │                  │
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

#### Workflow 4: Staking & Rewards

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                           STAKING & REWARDS WORKFLOW                                 │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  USER                     CONTRACTS                           REWARD POOL           │
│    │                          │                                    │                │
│    │  ════════════════════════════════════════════════════════════════════════     │
│    │  STAKING FLOW                                                                  │
│    │  ════════════════════════════════════════════════════════════════════════     │
│    │                          │                                    │                │
│    │  1. Approve Tokens       │                                    │                │
│    ├─────────────────────────▶│ AIIGOToken.approve()               │                │
│    │                          │                                    │                │
│    │  2. Stake Tokens         │                                    │                │
│    │     + Lock Period        │                                    │                │
│    ├─────────────────────────▶│ StakingPool.stake()                │                │
│    │                          │  ├─ Transfer tokens to pool        │                │
│    │                          │  ├─ Record stake info              │                │
│    │                          │  │   ├─ amount                     │                │
│    │                          │  │   ├─ startTime                  │                │
│    │                          │  │   ├─ lockPeriod                 │                │
│    │                          │  │   └─ rewardDebt                 │                │
│    │                          │  └─ Update total staked            │                │
│    │                          │                                    │                │
│    │                          │ [Event: Staked]                    │                │
│    │◀─────────────────────────┤                                    │                │
│    │                          │                                    │                │
│    │  ════════════════════════════════════════════════════════════════════════     │
│    │  REWARD ACCUMULATION (Continuous)                                              │
│    │  ════════════════════════════════════════════════════════════════════════     │
│    │                          │                                    │                │
│    │                          │ [Every block]                      │                │
│    │                          │ StakingPool                        │                │
│    │                          │  ._updateReward()                  │                │
│    │                          │  ├─ rewardPerToken +=              │◀───────────────│
│    │                          │  │   (rewardRate * timeDelta)      │  Platform fees │
│    │                          │  │   / totalStaked                 │  Task rewards  │
│    │                          │  └─ Update lastUpdateTime          │                │
│    │                          │                                    │                │
│    │                          │ userReward =                       │                │
│    │                          │  stakeAmount *                     │                │
│    │                          │  (rewardPerToken - rewardDebt)     │                │
│    │                          │                                    │                │
│    │  ════════════════════════════════════════════════════════════════════════     │
│    │  CLAIM REWARDS                                                                 │
│    │  ════════════════════════════════════════════════════════════════════════     │
│    │                          │                                    │                │
│    │  3. Claim Rewards        │                                    │                │
│    ├─────────────────────────▶│ StakingPool.claimRewards()         │                │
│    │                          │  ├─ Calculate pending rewards      │                │
│    │                          │  ├─ Reset reward debt              │                │
│    │                          │  └─ Transfer AIIGO to user         │                │
│    │◀─────────────────────────┤                                    │                │
│    │  Receive AIIGO           │ [Event: RewardsClaimed]            │                │
│    │                          │                                    │                │
│    │  ════════════════════════════════════════════════════════════════════════     │
│    │  UNSTAKE (After Lock Period)                                                   │
│    │  ════════════════════════════════════════════════════════════════════════     │
│    │                          │                                    │                │
│    │  4. Unstake              │                                    │                │
│    ├─────────────────────────▶│ StakingPool.unstake()              │                │
│    │                          │  ├─ Check lock period expired      │                │
│    │                          │  ├─ Calculate penalty (if early)   │                │
│    │                          │  │   └─ 5% penalty → burn          │                │
│    │                          │  ├─ Return tokens - penalty        │                │
│    │                          │  └─ Update total staked            │                │
│    │◀─────────────────────────┤                                    │                │
│    │  Receive tokens          │ [Event: Unstaked]                  │                │
│    │  (minus any penalty)     │                                    │                │
│    │                          │                                    │                │
└────┴──────────────────────────┴────────────────────────────────────┴────────────────┘

Reward Sources:
┌────────────────────────────────────────────────────────────────────────────────────┐
│  Platform Fees (8%)  ──────▶  Treasury  ──────▶  StakingPool.addRewardsToPool()   │
│  Task Completion     ──────▶  RewardDistributor  ──────▶  Provider + Stakers      │
│  PoW Challenges      ──────▶  Bonus AIIGO minted ──────▶  Provider                │
└────────────────────────────────────────────────────────────────────────────────────┘
```

---

#### Workflow 5: Governance Proposal & Voting

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                         GOVERNANCE WORKFLOW (DAO)                                    │
├─────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                      │
│  PROPOSER              CONTRACTS              VOTERS             EXECUTION          │
│  (100k+ AIIGO)             │                    │                    │              │
│       │                    │                    │                    │              │
│       │  1. Create         │                    │                    │              │
│       │     Proposal       │                    │                    │              │
│       ├───────────────────▶│ AIIGOGovernor     │                    │              │
│       │                    │  .propose()        │                    │              │
│       │                    │  ├─ Verify 100k    │                    │              │
│       │                    │  │  AIIGO balance  │                    │              │
│       │                    │  ├─ Store proposal │                    │              │
│       │                    │  │   ├─ targets[]  │                    │              │
│       │                    │  │   ├─ values[]   │                    │              │
│       │                    │  │   ├─ calldatas[]│                    │              │
│       │                    │  │   └─ description│                    │              │
│       │                    │  └─ Set snapshot   │                    │              │
│       │                    │       block        │                    │              │
│       │◀───────────────────┤ Return proposalId  │                    │              │
│       │                    │                    │                    │              │
│       │                    │ [Event: ProposalCreated]               │              │
│       │                    ├───────────────────▶│                    │              │
│       │                    │                    │                    │              │
│       │                    │ ═══════════════════════════════════════              │
│       │                    │ VOTING DELAY (1 day)                                  │
│       │                    │ ═══════════════════════════════════════              │
│       │                    │                    │                    │              │
│       │                    │                    │ 2. Cast Votes      │              │
│       │                    │◀───────────────────┤    (For/Against/   │              │
│       │                    │ AIIGOGovernor     │     Abstain)       │              │
│       │                    │  .castVote()       │                    │              │
│       │                    │  ├─ Get voting     │                    │              │
│       │                    │  │  weight at      │                    │              │
│       │                    │  │  snapshot       │                    │              │
│       │                    │  ├─ Record vote    │                    │              │
│       │                    │  └─ Update totals  │                    │              │
│       │                    │                    │                    │              │
│       │                    │ [Event: VoteCast]  │                    │              │
│       │                    │                    │                    │              │
│       │                    │ ═══════════════════════════════════════              │
│       │                    │ VOTING PERIOD (7 days)                                │
│       │                    │ ═══════════════════════════════════════              │
│       │                    │                    │                    │              │
│       │                    │ 3. Check Quorum    │                    │              │
│       │                    │    (4% of supply)  │                    │              │
│       │                    │                    │                    │              │
│       │                    │ ┌────────────────────────────────────┐ │              │
│       │                    │ │ Proposal States:                   │ │              │
│       │                    │ │ ├─ Pending   (before voting delay) │ │              │
│       │                    │ │ ├─ Active    (during voting)       │ │              │
│       │                    │ │ ├─ Succeeded (passed + quorum)     │ │              │
│       │                    │ │ ├─ Defeated  (failed or no quorum) │ │              │
│       │                    │ │ ├─ Queued    (in timelock)         │ │              │
│       │                    │ │ └─ Executed  (completed)           │ │              │
│       │                    │ └────────────────────────────────────┘ │              │
│       │                    │                    │                    │              │
│       │                    │ ═══════════════════════════════════════              │
│       │                    │ TIMELOCK (2 days) - if passed                         │
│       │                    │ ═══════════════════════════════════════              │
│       │                    │                    │                    │              │
│       │ 4. Execute         │                    │                    │              │
│       ├───────────────────▶│ AIIGOGovernor     │                    │              │
│       │                    │  .execute()        │                    │              │
│       │                    │  ├─ Verify state   │                    │              │
│       │                    │  │  = Succeeded    │                    │              │
│       │                    │  ├─ Execute calls  │                    │              │
│       │                    │  │   ├─ targets[0] │───────────────────▶│              │
│       │                    │  │   ├─ targets[1] │───────────────────▶│              │
│       │                    │  │   └─ ...        │                    │ Apply        │
│       │                    │  └─ Mark executed  │                    │ Changes      │
│       │                    │                    │                    │              │
│       │                    │ [Event: ProposalExecuted]              │              │
│       │◀───────────────────┤                    │                    │              │
│       │                    │                    │                    │              │
└───────┴────────────────────┴────────────────────┴────────────────────┴──────────────┘

Example Governance Actions:
┌────────────────────────────────────────────────────────────────────────────────────┐
│  • updatePlatformFee(700)        // Change fee from 8% to 7%                       │
│  • updateMinStake(200e18)        // Increase minimum stake to 200 AIIGO            │
│  • slashNode(nodeId, 1000e18)    // Slash malicious provider                       │
│  • addTrustedNode(nodeId)        // Whitelist enterprise partner                   │
│  • setRewardRate(newRate)        // Adjust staking APY                             │
│  • pause() / unpause()           // Emergency circuit breaker                      │
└────────────────────────────────────────────────────────────────────────────────────┘
```

---

#### Workflow 6: Complete System Overview

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                    AIIGO COMPUTING POWER - COMPLETE SYSTEM FLOW                      │
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
│  │ Register│ │  Stake  │         │   PoW   │           │  Stake  │ │ Deposit │    │
│  │  Node   │ │  AIIGO  │         │ Verify  │           │ (opt.)  │ │   ETH   │    │
│  └────┬────┘ └────┬────┘         └────┬────┘           └────┬────┘ └────┬────┘    │
│       │           │                   │                     │           │          │
│       ▼           ▼                   ▼                     ▼           ▼          │
│  ┌─────────────────────────────────────────────────────────────────────────────┐  │
│  │                                                                               │  │
│  │   NodeRegistry    StakingPool    PoWVerifier                  EscrowManager  │  │
│  │        │               │              │                            │          │  │
│  │        └───────────────┴──────────────┴────────────────────────────┘          │  │
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
│  │  │        │               │               │     Arbitration  Release       │ │  │
│  │  │        │               │               │                  Payment       │ │  │
│  │  │        │               │               │                                │ │  │
│  │  └────────┴───────────────┴───────────────┴────────────────────────────────┘ │  │
│  │                                   │                                           │  │
│  │                                   ▼                                           │  │
│  │  ┌─────────────────────────────────────────────────────────────────────────┐ │  │
│  │  │                        PAYMENT & REWARDS                                 │ │  │
│  │  │                                                                          │ │  │
│  │  │      ┌────────────────────────────────────────────────────────┐         │ │  │
│  │  │      │                     ESCROW RELEASE                     │         │ │  │
│  │  │      │                                                        │         │ │  │
│  │  │      │   Total Payment                                        │         │ │  │
│  │  │      │        │                                               │         │ │  │
│  │  │      │        ├──────────▶  92% ─────▶ Provider Wallet        │         │ │  │
│  │  │      │        │                                               │         │ │  │
│  │  │      │        └──────────▶   8% ─────▶ Platform Treasury      │         │ │  │
│  │  │      │                            │                           │         │ │  │
│  │  │      │                            ▼                           │         │ │  │
│  │  │      │                    ┌──────────────┐                    │         │ │  │
│  │  │      │                    │RewardDistrib.│                    │         │ │  │
│  │  │      │                    │  + Bonuses   │                    │         │ │  │
│  │  │      │                    │  + Staking   │                    │         │ │  │
│  │  │      │                    └──────────────┘                    │         │ │  │
│  │  │      │                                                        │         │ │  │
│  │  │      └────────────────────────────────────────────────────────┘         │ │  │
│  │  │                                                                          │ │  │
│  │  └──────────────────────────────────────────────────────────────────────────┘ │  │
│  │                                   │                                           │  │
│  │                                   ▼                                           │  │
│  │  ┌─────────────────────────────────────────────────────────────────────────┐ │  │
│  │  │                           GOVERNANCE (DAO)                               │ │  │
│  │  │                                                                          │ │  │
│  │  │         AIIGOToken Holders                                               │ │  │
│  │  │                │                                                         │ │  │
│  │  │                ▼                                                         │ │  │
│  │  │   ┌────────────────────────────────────────────────────────────┐        │ │  │
│  │  │   │  Propose  │──▶│  Vote  │──▶│  Queue  │──▶│  Execute  │     │        │ │  │
│  │  │   └────────────────────────────────────────────────────────────┘        │ │  │
│  │  │                │                                                         │ │  │
│  │  │                ▼                                                         │ │  │
│  │  │   ┌────────────────────────────────────────────────────────────┐        │ │  │
│  │  │   │  • Platform fee changes     • Minimum stake updates        │        │ │  │
│  │  │   │  • Slashing proposals       • Protocol upgrades            │        │ │  │
│  │  │   │  • Treasury allocation      • Emergency actions            │        │ │  │
│  │  │   └────────────────────────────────────────────────────────────┘        │ │  │
│  │  │                                                                          │ │  │
│  │  └──────────────────────────────────────────────────────────────────────────┘ │  │
│  │                                                                               │  │
│  └───────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                      │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

---

#### Contract API Call Summary

| Flow | Step | Contract | Method | Parameters |
|------|------|----------|--------|------------|
| **Registration** | 1 | AIIGOToken | `approve()` | spender, amount |
| | 2 | NodeRegistry | `registerNode()` | resourceType, metadataURI |
| | 3 | NodeRegistry | `stakeForNode()` | nodeId, amount |
| | 4 | PoWVerifier | `issueChallenge()` | nodeId |
| | 5 | PoWVerifier | `submitSolution()` | challengeId, nonce |
| **Task** | 1 | TaskMarketplace | `createTask()` | type, power, duration, price, trustLevel, specURI |
| | 2 | TaskMarketplace | `acceptTask()` | taskId, nodeId |
| | 3 | TaskMarketplace | `submitResult()` | taskId, resultHash, resultURI |
| | 4 | TaskMarketplace | `approveResult()` | taskId |
| **Dispute** | 1 | TaskMarketplace | `disputeTask()` | taskId, reason |
| | 2 | AIIGOGovernor | `castVote()` | disputeId, support |
| | 3 | EscrowManager | `refund()` / `release()` | taskId |
| **Staking** | 1 | AIIGOToken | `approve()` | spender, amount |
| | 2 | StakingPool | `stake()` | amount, lockPeriod |
| | 3 | StakingPool | `claimRewards()` | - |
| | 4 | StakingPool | `unstake()` | amount |
| **Governance** | 1 | AIIGOGovernor | `propose()` | targets, values, calldatas, description |
| | 2 | AIIGOGovernor | `castVote()` | proposalId, support |
| | 3 | AIIGOGovernor | `execute()` | proposalId |

---

### Deployment Order

```
1. AIIGOToken.sol           ← Deploy first (no dependencies)
2. NodeRegistry.sol         ← Depends on AIIGOToken
3. StakingPool.sol          ← Depends on AIIGOToken, NodeRegistry
4. ProofOfWorkVerifier.sol  ← Depends on NodeRegistry
5. EscrowManager.sol        ← Standalone escrow logic
6. RewardDistributor.sol    ← Depends on AIIGOToken, NodeRegistry
7. TaskMarketplace.sol      ← Depends on all above
8. AIIGOGovernor.sol        ← Depends on AIIGOToken, final deployment
```

**Post-Deployment Configuration:**
```solidity
// Grant roles
aiigoToken.grantRole(MINTER_ROLE, rewardDistributor);
nodeRegistry.grantRole(UPDATER_ROLE, powVerifier);
nodeRegistry.grantRole(UPDATER_ROLE, taskMarketplace);
escrowManager.grantRole(RELEASER_ROLE, taskMarketplace);

// Set contract addresses
taskMarketplace.setNodeRegistry(nodeRegistry);
taskMarketplace.setEscrowManager(escrowManager);
taskMarketplace.setPowVerifier(powVerifier);
rewardDistributor.setNodeRegistry(nodeRegistry);
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

| Layer | Technology |
|-------|------------|
| **Smart Contracts** | Solidity 0.8.20, OpenZeppelin, Foundry |
| **Blockchain** | Ethereum (Sepolia → Mainnet) |
| **Provider Agent** | Rust (Tauri), GPU libraries |
| **Backend Services** | Go/Rust microservices |
| **API Gateway** | Kong / AWS API Gateway |
| **Database** | PostgreSQL + Redis |
| **Message Queue** | RabbitMQ / Kafka |
| **Monitoring** | Prometheus + Grafana |
| **Desktop Client** | Tauri + React + TypeScript |
| **Mobile Client** | React Native |

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

AIIGO Computing Power Marketplace creates a three-sided market:

1. **Providers** earn passive income from idle devices
2. **Buyers** access affordable distributed computing
3. **Platform** ensures quality, trust, and liquidity

The combination of community-driven supply with platform-owned infrastructure ensures both flexibility and reliability, making it viable for both casual users and enterprise clients.
