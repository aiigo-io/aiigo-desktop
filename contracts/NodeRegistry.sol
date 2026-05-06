// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {AccessControlDefaultAdminRules} from "@openzeppelin/contracts/access/extensions/AccessControlDefaultAdminRules.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {INodeRegistry} from "./interfaces/INodeRegistry.sol";
import {MarketplaceTypes} from "./types/MarketplaceTypes.sol";

contract NodeRegistry is INodeRegistry, AccessControlDefaultAdminRules, Pausable, ReentrancyGuard {
    error AmountMustBeGreaterThanZero();
    error ClaimTransferFailed();
    error DirectETHNotAccepted();
    error InsufficientRegistrationValue(uint256 provided, uint256 requiredAmount);
    error InvalidNode(bytes32 nodeId);
    error InvalidActionWhilePaused();
    error InvalidNodeStatusTransition(MarketplaceTypes.NodeStatus from, MarketplaceTypes.NodeStatus to);
    error InvalidStatusForWithdrawal(bytes32 nodeId);
    error MissingClaimableBalance();
    error NotNodeOwner(bytes32 nodeId, address caller);
    error RoleRenounceDisabled();
    error SolvencyInvariantViolated(uint256 balance, uint256 obligations);
    error StakeExceedsBalance(bytes32 nodeId, uint256 requested, uint256 available);
    error ZeroNodeIdNotAllowed();
    error ZeroAddressNotAllowed();

    bytes32 public constant UPDATER_ROLE = keccak256("UPDATER_ROLE");
    uint48 public constant DEFAULT_ADMIN_DELAY = 1 days;
    uint256 public constant REGISTRATION_FEE = 0.1 ether;
    uint256 public constant MINIMUM_INITIAL_STAKE = 0.5 ether;

    uint8 private constant BUCKET_UNTRACKED = 0;
    uint8 private constant BUCKET_LOCKED_STAKE = 1;
    uint8 private constant BUCKET_PENDING_WITHDRAWAL = 2;
    uint8 private constant BUCKET_TREASURY_FEE = 3;

    address private _treasury;
    uint256 private _registrationNonce;
    uint256 private _totalLockedStake;
    uint256 private _totalPendingStakeWithdrawals;
    uint256 private _totalPendingTreasuryFees;
    uint256 private _totalAccountedObligations;

    mapping(bytes32 => MarketplaceTypes.Node) private _nodes;
    mapping(bytes32 => bool) private _nodeExists;
    mapping(address => bytes32[]) private _nodesByOwner;
    mapping(MarketplaceTypes.ResourceType => bytes32[]) private _nodesByResourceType;
    mapping(address => uint256) private _pendingStakeWithdrawals;
    mapping(address => uint256) private _pendingTreasuryFees;

    constructor(
        address initialDefaultAdmin,
        address treasury_
    ) AccessControlDefaultAdminRules(DEFAULT_ADMIN_DELAY, initialDefaultAdmin) {
        if (initialDefaultAdmin == address(0)) {
            revert ZeroAddressNotAllowed();
        }
        if (treasury_ == address(0)) {
            revert ZeroAddressNotAllowed();
        }

        _treasury = treasury_;
        _grantRole(UPDATER_ROLE, initialDefaultAdmin);
    }

    function pause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(DEFAULT_ADMIN_ROLE) {
        _unpause();
    }

    function renounceRole(bytes32, address) public pure override {
        revert RoleRenounceDisabled();
    }

    function setTreasury(address treasury_) external onlyRole(DEFAULT_ADMIN_ROLE) {
        if (treasury_ == address(0)) {
            revert ZeroAddressNotAllowed();
        }

        _treasury = treasury_;
    }

    function registerNode(
        MarketplaceTypes.ResourceType resourceType,
        string calldata metadataURI
    ) external payable whenNotPaused returns (bytes32 nodeId) {
        uint256 requiredAmount = REGISTRATION_FEE + MINIMUM_INITIAL_STAKE;
        if (msg.value < requiredAmount) {
            revert InsufficientRegistrationValue(msg.value, requiredAmount);
        }

        nodeId = _deriveNodeId(msg.sender, _registrationNonce);
        _requireNodeId(nodeId);
        _registrationNonce += 1;

        uint256 stakeAmount = msg.value - REGISTRATION_FEE;
        _nodes[nodeId] = MarketplaceTypes.Node({
            owner: msg.sender,
            nodeId: nodeId,
            status: MarketplaceTypes.NodeStatus.Pending,
            resourceType: resourceType,
            computePower: 0,
            stakedAmount: stakeAmount,
            reputation: 0,
            totalTasksCompleted: 0,
            totalEarnings: 0,
            registeredAt: block.timestamp,
            lastActiveAt: block.timestamp,
            metadataURI: metadataURI
        });
        _nodeExists[nodeId] = true;
        _nodesByOwner[msg.sender].push(nodeId);
        _nodesByResourceType[resourceType].push(nodeId);

        _totalLockedStake += stakeAmount;
        _totalAccountedObligations += msg.value;

        emit NodeRegistered(nodeId, msg.sender, resourceType);
        emit StakeDeposited(nodeId, stakeAmount, stakeAmount);
        emit AccountingBucketMoved(nodeId, msg.sender, BUCKET_UNTRACKED, BUCKET_LOCKED_STAKE, stakeAmount);

        _pendingTreasuryFees[_treasury] += REGISTRATION_FEE;
        _totalPendingTreasuryFees += REGISTRATION_FEE;
        emit TreasuryFeeQueued(nodeId, _treasury, REGISTRATION_FEE);
        emit AccountingBucketMoved(nodeId, _treasury, BUCKET_UNTRACKED, BUCKET_TREASURY_FEE, REGISTRATION_FEE);

        _assertSolvent();
    }

    function addStake(bytes32 nodeId) external payable whenNotPaused {
        _requireNodeId(nodeId);
        MarketplaceTypes.Node storage node = _getNodeOwnedByCaller(nodeId);
        if (msg.value == 0) {
            revert AmountMustBeGreaterThanZero();
        }

        node.stakedAmount += msg.value;
        node.lastActiveAt = block.timestamp;

        _totalLockedStake += msg.value;
        _totalAccountedObligations += msg.value;

        emit StakeDeposited(nodeId, msg.value, node.stakedAmount);
        emit AccountingBucketMoved(nodeId, msg.sender, BUCKET_UNTRACKED, BUCKET_LOCKED_STAKE, msg.value);

        _assertSolvent();
    }

    function withdrawStake(bytes32 nodeId, uint256 amount) external nonReentrant {
        _requireNodeId(nodeId);
        MarketplaceTypes.Node storage node = _getNodeOwnedByCaller(nodeId);
        if (amount == 0) {
            revert AmountMustBeGreaterThanZero();
        }
        if (node.status == MarketplaceTypes.NodeStatus.Active) {
            revert InvalidStatusForWithdrawal(nodeId);
        }
        if (amount > node.stakedAmount) {
            revert StakeExceedsBalance(nodeId, amount, node.stakedAmount);
        }

        node.stakedAmount -= amount;
        node.lastActiveAt = block.timestamp;

        _totalLockedStake -= amount;
        _pendingStakeWithdrawals[msg.sender] += amount;
        _totalPendingStakeWithdrawals += amount;

        emit StakeWithdrawn(nodeId, amount, node.stakedAmount);
        emit StakeWithdrawalQueued(nodeId, msg.sender, amount);
        emit AccountingBucketMoved(nodeId, msg.sender, BUCKET_LOCKED_STAKE, BUCKET_PENDING_WITHDRAWAL, amount);

        _assertSolvent();
    }

    function claimStakeWithdrawal() external nonReentrant {
        uint256 amount = _pendingStakeWithdrawals[msg.sender];
        if (amount == 0) {
            revert MissingClaimableBalance();
        }

        _pendingStakeWithdrawals[msg.sender] = 0;
        _totalPendingStakeWithdrawals -= amount;
        _totalAccountedObligations -= amount;

        (bool sent, ) = payable(msg.sender).call{value: amount}("");
        if (!sent) {
            revert ClaimTransferFailed();
        }

        emit StakeWithdrawalClaimed(msg.sender, amount);
        _assertSolvent();
    }

    function claimTreasuryFees() external nonReentrant {
        uint256 amount = _pendingTreasuryFees[msg.sender];
        if (amount == 0) {
            revert MissingClaimableBalance();
        }

        _pendingTreasuryFees[msg.sender] = 0;
        _totalPendingTreasuryFees -= amount;
        _totalAccountedObligations -= amount;

        (bool sent, ) = payable(msg.sender).call{value: amount}("");
        if (!sent) {
            revert ClaimTransferFailed();
        }

        emit TreasuryFeeClaimed(msg.sender, amount);
        _assertSolvent();
    }

    function updateNodeStatus(bytes32 nodeId, MarketplaceTypes.NodeStatus status) external onlyRole(UPDATER_ROLE) {
        _requireNodeId(nodeId);

        MarketplaceTypes.Node storage node = _getExistingNode(nodeId);
        MarketplaceTypes.NodeStatus oldStatus = node.status;

        if (paused() && !_isRiskReducingTransition(oldStatus, status)) {
            revert InvalidActionWhilePaused();
        }

        if (!_isValidStatusTransition(oldStatus, status)) {
            revert InvalidNodeStatusTransition(oldStatus, status);
        }

        node.status = status;
        node.lastActiveAt = block.timestamp;

        emit NodeStatusChanged(nodeId, oldStatus, status);
    }

    function updateReputation(bytes32 nodeId, int256 delta) external onlyRole(UPDATER_ROLE) {
        _requireNodeId(nodeId);
        if (paused() && delta >= 0) {
            revert InvalidActionWhilePaused();
        }

        MarketplaceTypes.Node storage node = _getExistingNode(nodeId);
        uint256 oldReputation = node.reputation;

        if (delta < 0) {
            uint256 decrement = uint256(-delta);
            node.reputation = decrement >= node.reputation ? 0 : node.reputation - decrement;
        } else {
            uint256 increment = uint256(delta);
            uint256 nextValue = node.reputation + increment;
            node.reputation = nextValue > 10_000 ? 10_000 : nextValue;
        }

        emit ReputationUpdated(nodeId, oldReputation, node.reputation);
    }

    function slashStake(bytes32 nodeId, uint256 amount) external onlyRole(UPDATER_ROLE) {
        _requireNodeId(nodeId);
        MarketplaceTypes.Node storage node = _getExistingNode(nodeId);
        if (amount == 0) {
            revert AmountMustBeGreaterThanZero();
        }
        if (amount > node.stakedAmount) {
            revert StakeExceedsBalance(nodeId, amount, node.stakedAmount);
        }

        node.stakedAmount -= amount;
        node.status = MarketplaceTypes.NodeStatus.Slashed;
        node.lastActiveAt = block.timestamp;

        _totalLockedStake -= amount;
        _pendingTreasuryFees[_treasury] += amount;
        _totalPendingTreasuryFees += amount;

        emit NodeSlashed(nodeId, amount, "stake slashed");
        emit TreasuryFeeQueued(nodeId, _treasury, amount);
        emit AccountingBucketMoved(nodeId, _treasury, BUCKET_LOCKED_STAKE, BUCKET_TREASURY_FEE, amount);

        _assertSolvent();
    }

    function updateComputePower(bytes32 nodeId, uint256 computePower) external onlyRole(UPDATER_ROLE) {
        _requireNodeId(nodeId);
        MarketplaceTypes.Node storage node = _getExistingNode(nodeId);

        if (paused() && computePower >= node.computePower) {
            revert InvalidActionWhilePaused();
        }

        uint256 oldComputePower = node.computePower;

        node.computePower = computePower;
        node.lastActiveAt = block.timestamp;

        emit ComputePowerUpdated(nodeId, oldComputePower, computePower);
    }

    function getNode(bytes32 nodeId) external view returns (MarketplaceTypes.Node memory) {
        _requireNodeId(nodeId);
        return _getExistingNode(nodeId);
    }

    function getNodesByOwner(address owner) external view returns (bytes32[] memory) {
        return _nodesByOwner[owner];
    }

    function getActiveNodes(
        MarketplaceTypes.ResourceType resourceType
    ) external view returns (bytes32[] memory) {
        bytes32[] storage nodeIds = _nodesByResourceType[resourceType];
        uint256 activeCount;

        for (uint256 i = 0; i < nodeIds.length; ++i) {
            if (_nodes[nodeIds[i]].status == MarketplaceTypes.NodeStatus.Active) {
                activeCount += 1;
            }
        }

        bytes32[] memory activeNodes = new bytes32[](activeCount);
        uint256 cursor;

        for (uint256 i = 0; i < nodeIds.length; ++i) {
            if (_nodes[nodeIds[i]].status == MarketplaceTypes.NodeStatus.Active) {
                activeNodes[cursor] = nodeIds[i];
                cursor += 1;
            }
        }

        return activeNodes;
    }

    function isNodeActive(bytes32 nodeId) external view returns (bool) {
        _requireNodeId(nodeId);
        return _nodeExists[nodeId] && _nodes[nodeId].status == MarketplaceTypes.NodeStatus.Active;
    }

    function getNodeTrustLevel(bytes32 nodeId) external view returns (uint8) {
        _requireNodeId(nodeId);
        MarketplaceTypes.Node storage node = _getExistingNode(nodeId);

        if (node.stakedAmount >= 5 ether && node.reputation >= 9_500) {
            return 4;
        }
        if (node.stakedAmount >= 3 ether && node.reputation >= 9_000) {
            return 3;
        }
        if (node.stakedAmount >= 1 ether && node.status != MarketplaceTypes.NodeStatus.Pending) {
            return 2;
        }
        if (node.stakedAmount >= MINIMUM_INITIAL_STAKE) {
            return 1;
        }

        return 0;
    }

    function getPendingStakeWithdrawal(address owner) external view returns (uint256) {
        return _pendingStakeWithdrawals[owner];
    }

    function getPendingTreasuryFees(address recipient) external view returns (uint256) {
        return _pendingTreasuryFees[recipient];
    }

    function getTreasury() external view returns (address) {
        return _treasury;
    }

    function totalAccountedObligations() external view returns (uint256) {
        return _totalAccountedObligations;
    }

    receive() external payable {
        revert DirectETHNotAccepted();
    }

    fallback() external payable {
        revert DirectETHNotAccepted();
    }

    function _deriveNodeId(address registrant, uint256 registrationNonce) private view returns (bytes32) {
        return keccak256(abi.encode(block.chainid, address(this), registrant, registrationNonce));
    }

    function _getExistingNode(bytes32 nodeId) private view returns (MarketplaceTypes.Node storage node) {
        if (!_nodeExists[nodeId]) {
            revert InvalidNode(nodeId);
        }

        return _nodes[nodeId];
    }

    function _getNodeOwnedByCaller(
        bytes32 nodeId
    ) private view returns (MarketplaceTypes.Node storage node) {
        node = _getExistingNode(nodeId);
        if (node.owner != msg.sender) {
            revert NotNodeOwner(nodeId, msg.sender);
        }
    }

    function _requireNodeId(bytes32 nodeId) private pure {
        if (nodeId == bytes32(0)) {
            revert ZeroNodeIdNotAllowed();
        }
    }

    function _isValidStatusTransition(
        MarketplaceTypes.NodeStatus from,
        MarketplaceTypes.NodeStatus to
    ) private pure returns (bool) {
        if (from == to) {
            return false;
        }
        if (from == MarketplaceTypes.NodeStatus.Pending) {
            return
                to == MarketplaceTypes.NodeStatus.Verified ||
                to == MarketplaceTypes.NodeStatus.Inactive ||
                to == MarketplaceTypes.NodeStatus.Slashed;
        }
        if (from == MarketplaceTypes.NodeStatus.Verified) {
            return
                to == MarketplaceTypes.NodeStatus.Active ||
                to == MarketplaceTypes.NodeStatus.Inactive ||
                to == MarketplaceTypes.NodeStatus.Slashed;
        }
        if (from == MarketplaceTypes.NodeStatus.Active) {
            return to == MarketplaceTypes.NodeStatus.Inactive || to == MarketplaceTypes.NodeStatus.Slashed;
        }
        if (from == MarketplaceTypes.NodeStatus.Inactive) {
            return to == MarketplaceTypes.NodeStatus.Active || to == MarketplaceTypes.NodeStatus.Slashed;
        }

        return false;
    }

    function _isRiskReducingTransition(
        MarketplaceTypes.NodeStatus from,
        MarketplaceTypes.NodeStatus to
    ) private pure returns (bool) {
        if (to == MarketplaceTypes.NodeStatus.Slashed) {
            return true;
        }

        return
            to == MarketplaceTypes.NodeStatus.Inactive &&
            (from == MarketplaceTypes.NodeStatus.Pending ||
                from == MarketplaceTypes.NodeStatus.Verified ||
                from == MarketplaceTypes.NodeStatus.Active);
    }

    function _assertSolvent() private view {
        if (address(this).balance < _totalAccountedObligations) {
            revert SolvencyInvariantViolated(address(this).balance, _totalAccountedObligations);
        }
    }
}