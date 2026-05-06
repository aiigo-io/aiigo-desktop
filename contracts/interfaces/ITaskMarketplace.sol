// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {MarketplaceTypes} from "../types/MarketplaceTypes.sol";

interface ITaskMarketplace {
    event TaskCreated(
        bytes32 indexed taskId,
        address indexed buyer,
        MarketplaceTypes.ResourceType resourceType,
        uint256 maxPrice
    );
    event TaskAssigned(bytes32 indexed taskId, bytes32 indexed nodeId, uint256 startTime);
    event TaskCompleted(bytes32 indexed taskId, bytes32 resultHash);
    event TaskVerified(bytes32 indexed taskId, uint256 payoutAmount);
    event TaskDisputed(bytes32 indexed taskId, address disputedBy, string reason);
    event TaskCancelled(bytes32 indexed taskId, uint256 refundAmount);

    function createTask(
        MarketplaceTypes.ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration,
        uint256 maxPrice,
        uint8 minTrustLevel,
        string calldata specificationURI
    ) external returns (bytes32 taskId);

    function fundTaskEscrow(bytes32 taskId) external payable;

    function cancelTask(bytes32 taskId) external;

    function disputeTask(bytes32 taskId, string calldata reason) external;

    function approveResult(bytes32 taskId) external;

    function acceptTask(bytes32 taskId, bytes32 nodeId) external;

    function submitResult(bytes32 taskId, bytes32 resultHash, string calldata resultURI) external;

    function getTask(bytes32 taskId) external view returns (MarketplaceTypes.Task memory);

    function getOpenTasks(
        MarketplaceTypes.ResourceType resourceType
    ) external view returns (bytes32[] memory);

    function getTasksByBuyer(address buyer) external view returns (bytes32[] memory);

    function getTasksByProvider(bytes32 nodeId) external view returns (bytes32[] memory);

    function estimateTaskCost(
        MarketplaceTypes.ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration
    ) external view returns (uint256);
}

