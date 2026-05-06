// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {MarketplaceTypes} from "../types/MarketplaceTypes.sol";

interface IEscrowManager {
    event AccountingBucketMoved(
        bytes32 indexed taskId,
        address indexed recipient,
        uint8 fromBucket,
        uint8 toBucket,
        uint256 amount
    );
    event BuyerRefundQueued(bytes32 indexed taskId, address indexed recipient, uint256 amount);
    event BuyerRefundClaimed(address indexed recipient, uint256 amount);
    event ProviderPayoutQueued(bytes32 indexed taskId, address indexed recipient, uint256 amount);
    event ProviderPayoutClaimed(address indexed recipient, uint256 amount);
    event TreasuryFeeQueued(bytes32 indexed taskId, address indexed recipient, uint256 amount);
    event TreasuryFeeClaimed(address indexed recipient, uint256 amount);
    event EscrowDeposited(bytes32 indexed taskId, address indexed buyer, uint256 amount);
    event EscrowReleased(
        bytes32 indexed taskId,
        address indexed provider,
        uint256 providerPayout,
        uint256 platformFee
    );
    event EscrowRefunded(bytes32 indexed taskId, address indexed buyer, uint256 amount);
    event DisputeResolved(bytes32 indexed taskId, address indexed winner, uint256 amount);

    function deposit(bytes32 taskId, address buyer) external payable returns (bytes32 escrowId);

    function release(bytes32 taskId, address provider) external;

    function refund(bytes32 taskId) external;

    function splitPayment(bytes32 taskId, address provider, uint256 providerShare) external;

    function claimBuyerRefund() external;

    function claimProviderPayout() external;

    function claimTreasuryFees() external;

    function setTaskMarketplace(address taskMarketplace_) external;

    function setTreasury(address treasury_) external;

    function getEscrow(bytes32 taskId) external view returns (MarketplaceTypes.EscrowDeposit memory);

    function getPendingBuyerRefund(address buyer) external view returns (uint256);

    function getPendingProviderPayout(address provider) external view returns (uint256);

    function getPendingTreasuryFees(address recipient) external view returns (uint256);

    function getTaskMarketplace() external view returns (address);

    function getTreasury() external view returns (address);

    function totalAccountedObligations() external view returns (uint256);
}

