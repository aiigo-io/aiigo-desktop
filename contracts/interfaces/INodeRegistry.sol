// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {MarketplaceTypes} from "../types/MarketplaceTypes.sol";

interface INodeRegistry {
    event ContractReferenceUpdated(bytes32 indexed referenceName, address indexed previousReference, address indexed newReference);
    event AccountingBucketMoved(
        bytes32 indexed referenceId,
        address indexed recipient,
        uint8 fromBucket,
        uint8 toBucket,
        uint256 amount
    );
    event NodeRegistered(
        bytes32 indexed nodeId,
        address indexed owner,
        MarketplaceTypes.ResourceType resourceType
    );
    event NodeStatusChanged(
        bytes32 indexed nodeId,
        MarketplaceTypes.NodeStatus oldStatus,
        MarketplaceTypes.NodeStatus newStatus
    );
    event StakeDeposited(bytes32 indexed nodeId, uint256 amount, uint256 totalStake);
    event StakeWithdrawn(bytes32 indexed nodeId, uint256 amount, uint256 totalStake);
    event StakeWithdrawalQueued(bytes32 indexed nodeId, address indexed recipient, uint256 amount);
    event StakeWithdrawalClaimed(address indexed recipient, uint256 amount);
    event ReputationUpdated(bytes32 indexed nodeId, uint256 oldReputation, uint256 newReputation);
    event NodeSlashed(bytes32 indexed nodeId, uint256 slashedAmount, string reason);
    event ComputePowerUpdated(bytes32 indexed nodeId, uint256 oldComputePower, uint256 newComputePower);
    event TreasuryFeeQueued(bytes32 indexed referenceId, address indexed recipient, uint256 amount);
    event TreasuryFeeClaimed(address indexed recipient, uint256 amount);
    event TreasuryUpdated(address indexed previousTreasury, address indexed newTreasury);

    function registerNode(
        MarketplaceTypes.ResourceType resourceType,
        string calldata metadataURI
    ) external payable returns (bytes32 nodeId);

    function addStake(bytes32 nodeId) external payable;

    function withdrawStake(bytes32 nodeId, uint256 amount) external;

    function claimStakeWithdrawal() external;

    function claimTreasuryFees() external;

    function updateNodeStatus(bytes32 nodeId, MarketplaceTypes.NodeStatus status) external;

    function updateReputation(bytes32 nodeId, int256 delta) external;

    function slashStake(bytes32 nodeId, uint256 amount) external;

    function updateComputePower(bytes32 nodeId, uint256 computePower) external;

    function setTreasury(address treasury_) external;

    function setTaskMarketplace(address taskMarketplace_) external;

    function lockNodeStake(bytes32 nodeId) external;

    function unlockNodeStake(bytes32 nodeId) external;

    function getNode(bytes32 nodeId) external view returns (MarketplaceTypes.Node memory);

    function getNodesByOwner(address owner) external view returns (bytes32[] memory);

    function getActiveNodes(
        MarketplaceTypes.ResourceType resourceType
    ) external view returns (bytes32[] memory);

    function isNodeActive(bytes32 nodeId) external view returns (bool);

    function getNodeTrustLevel(bytes32 nodeId) external view returns (uint8);

    function getPendingStakeWithdrawal(address owner) external view returns (uint256);

    function getPendingTreasuryFees(address recipient) external view returns (uint256);

    function getTreasury() external view returns (address);

    function getPendingTaskCount(bytes32 nodeId) external view returns (uint256);

    function totalAccountedObligations() external view returns (uint256);
}

