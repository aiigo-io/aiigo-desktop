// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

library MarketplaceTypes {
    enum NodeStatus {
        Pending,
        Verified,
        Active,
        Inactive,
        Slashed
    }

    enum ResourceType {
        GPU,
        CPU,
        Network,
        Mobile,
        IoT
    }

    enum TaskStatus {
        Open,
        Assigned,
        InProgress,
        Completed,
        Verified,
        Disputed,
        Cancelled
    }

    enum EscrowSettlementKind {
        None,
        Refunded,
        Released,
        Split
    }

    struct Node {
        address owner;
        bytes32 nodeId;
        NodeStatus status;
        ResourceType resourceType;
        uint256 computePower;
        uint256 stakedAmount;
        uint256 reputation;
        uint256 totalTasksCompleted;
        uint256 totalEarnings;
        uint256 registeredAt;
        uint256 lastActiveAt;
        string metadataURI;
    }

    struct HardwareSpec {
        string gpuModel;
        uint256 gpuMemory;
        uint256 cpuCores;
        uint256 ramGB;
        uint256 storageGB;
        uint256 bandwidthMbps;
        string region;
    }

    struct Task {
        bytes32 taskId;
        address buyer;
        bytes32 assignedNode;
        ResourceType resourceType;
        uint256 requiredPower;
        uint256 duration;
        uint256 maxPrice;
        uint256 escrowAmount;
        uint256 createdAt;
        uint256 startedAt;
        uint256 completedAt;
        TaskStatus status;
        uint8 minTrustLevel;
        string specificationURI;
    }

    struct TaskResult {
        bytes32 taskId;
        bytes32 resultHash;
        string resultURI;
        uint256 actualDuration;
        uint256 computeUnitsUsed;
        bool verified;
    }

    struct EscrowDeposit {
        bytes32 taskId;
        address buyer;
        address provider;
        address treasuryRecipient;
        uint256 amount;
        uint256 platformFee;
        uint256 providerPayout;
        uint256 buyerRefund;
        uint256 depositedAt;
        EscrowSettlementKind settlement;
        bool exists;
    }

    struct Challenge {
        bytes32 challengeId;
        bytes32 nodeId;
        bytes32 seed;
        uint256 difficulty;
        uint256 issuedAt;
        uint256 deadline;
        bool completed;
        uint256 solutionTime;
    }

    struct VerificationResult {
        bytes32 nodeId;
        uint256 verifiedPower;
        uint256 timestamp;
        bool passed;
    }

    struct TaskLifecycle {
        uint256 challengeDeadline;
        address disputedBy;
        string disputeReason;
        bool resolved;
        address resolvedBy;
        uint256 grossProviderAmount;
    }
}

