// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {IEscrowManager} from "./interfaces/IEscrowManager.sol";
import {INodeRegistry} from "./interfaces/INodeRegistry.sol";
import {IProofOfWorkVerifier} from "./interfaces/IProofOfWorkVerifier.sol";
import {ITaskMarketplace} from "./interfaces/ITaskMarketplace.sol";
import {MarketplaceTypes} from "./types/MarketplaceTypes.sol";

abstract contract TaskMarketplace is ITaskMarketplace, Ownable2Step, Pausable {
    error ContractReferenceNotSet(bytes32 referenceName);
    error ContractReferenceSmokeCheckFailed(bytes4 selector, address candidate);
    error InvalidTaskStatusTransition(MarketplaceTypes.TaskStatus from, MarketplaceTypes.TaskStatus to);
    error InvalidContractReference(address candidate);
    error OwnerRenounceDisabled();
    error TaskAlreadyExists(bytes32 taskId);
    error TaskEscrowAlreadyFunded(bytes32 taskId);
    error TaskEscrowNotFunded(bytes32 taskId);
    error TaskFundingNotAllowed(bytes32 taskId, address caller, address buyer);
    error TaskNotFound(bytes32 taskId);
    error TaskResultNotFound(bytes32 taskId);
    error TaskValueOutOfRange(uint256 provided, uint256 maxPrice);
    error ZeroBuyerAddressNotAllowed();
    error ZeroResultHashNotAllowed();
    error ZeroNodeIdNotAllowed();
    error ZeroAddressNotAllowed();
    error ZeroTaskIdNotAllowed();

    INodeRegistry private _nodeRegistry;
    IEscrowManager private _escrowManager;
    IProofOfWorkVerifier private _proofOfWorkVerifier;

    mapping(bytes32 => MarketplaceTypes.Task) private _tasks;
    mapping(bytes32 => MarketplaceTypes.TaskResult) private _taskResults;
    mapping(bytes32 => bool) private _taskExists;
    mapping(bytes32 => bool) private _taskResultExists;
    mapping(address => bytes32[]) private _tasksByBuyer;
    mapping(bytes32 => bytes32[]) private _tasksByProvider;
    mapping(MarketplaceTypes.ResourceType => bytes32[]) private _openTasksByType;

    bytes32 private constant REFERENCE_NODE_REGISTRY = keccak256("NODE_REGISTRY");
    bytes32 private constant REFERENCE_ESCROW_MANAGER = keccak256("ESCROW_MANAGER");
    bytes32 private constant REFERENCE_PROOF_OF_WORK_VERIFIER = keccak256("PROOF_OF_WORK_VERIFIER");

    constructor(address owner_) Ownable(owner_) {}

    function setNodeRegistry(address nodeRegistry_) external virtual onlyOwner {
        _requireContractReference(nodeRegistry_);

        (bool ok, bytes memory returnData) = nodeRegistry_.staticcall(
            abi.encodeWithSelector(INodeRegistry.getNodesByOwner.selector, address(this))
        );
        if (!ok || returnData.length < 64) {
            revert ContractReferenceSmokeCheckFailed(INodeRegistry.getNodesByOwner.selector, nodeRegistry_);
        }

        abi.decode(returnData, (bytes32[]));

        _nodeRegistry = INodeRegistry(nodeRegistry_);
    }

    function setEscrowManager(address escrowManager_) external virtual onlyOwner {
        _requireContractReference(escrowManager_);

        (bool ok, bytes memory returnData) = escrowManager_.staticcall(
            abi.encodeWithSelector(IEscrowManager.getTaskMarketplace.selector)
        );
        if (!ok || returnData.length < 32) {
            revert ContractReferenceSmokeCheckFailed(IEscrowManager.getTaskMarketplace.selector, escrowManager_);
        }

        abi.decode(returnData, (address));

        _escrowManager = IEscrowManager(escrowManager_);
    }

    function setPowVerifier(address proofOfWorkVerifier_) external virtual onlyOwner {
        _requireContractReference(proofOfWorkVerifier_);

        (bool ok, bytes memory returnData) = proofOfWorkVerifier_.staticcall(
            abi.encodeWithSelector(IProofOfWorkVerifier.getVerificationHistory.selector, bytes32(0))
        );
        if (!ok || returnData.length < 64) {
            revert ContractReferenceSmokeCheckFailed(
                IProofOfWorkVerifier.getVerificationHistory.selector,
                proofOfWorkVerifier_
            );
        }

        abi.decode(returnData, (MarketplaceTypes.VerificationResult[]));

        _proofOfWorkVerifier = IProofOfWorkVerifier(proofOfWorkVerifier_);
    }

    function pause() external onlyOwner {
        _pause();
    }

    function unpause() external onlyOwner {
        _unpause();
    }

    function createTask(
        MarketplaceTypes.ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration,
        uint256 maxPrice,
        uint8 minTrustLevel,
        string calldata specificationURI
    ) external override whenNotPaused returns (bytes32 taskId) {
        taskId = _createTask(
            resourceType,
            requiredPower,
            duration,
            maxPrice,
            minTrustLevel,
            specificationURI
        );

        _requireTaskId(taskId);
    }

    function fundTaskEscrow(bytes32 taskId) external payable override whenNotPaused {
        _requireTaskId(taskId);
        _requireTaskFundingAllowed(taskId);

        _escrowManagerRef().deposit{value: msg.value}(taskId, msg.sender);
        _setTaskEscrowAmount(taskId, msg.value);
    }

    function cancelTask(bytes32 taskId) external override whenNotPaused {
        _requireTaskId(taskId);
        _cancelTask(taskId);
    }

    function disputeTask(bytes32 taskId, string calldata reason) external override whenNotPaused {
        _requireTaskId(taskId);
        _disputeTask(taskId, reason);
    }

    function approveResult(bytes32 taskId) external override whenNotPaused {
        _requireTaskId(taskId);
        _approveResult(taskId);
    }

    function acceptTask(bytes32 taskId, bytes32 nodeId) external override whenNotPaused {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        _requireTaskOpen(taskId);
        _requireTaskFunded(taskId);
        _requireNodeId(nodeId);
        _acceptTask(taskId, nodeId);
    }

    function submitResult(
        bytes32 taskId,
        bytes32 resultHash,
        string calldata resultURI
    ) external override whenNotPaused {
        _requireTaskId(taskId);
        _submitResult(taskId, resultHash, resultURI);
    }

    function renounceOwnership() public view override onlyOwner {
        revert OwnerRenounceDisabled();
    }

    function getTask(bytes32 taskId) external view override returns (MarketplaceTypes.Task memory) {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        return _tasks[taskId];
    }

    function getOpenTasks(
        MarketplaceTypes.ResourceType resourceType
    ) external view override returns (bytes32[] memory) {
        bytes32[] storage candidates = _openTasksByType[resourceType];
        uint256 openAndFundedCount;

        for (uint256 i = 0; i < candidates.length; ++i) {
            bytes32 taskId = candidates[i];
            if (_taskExists[taskId]) {
                MarketplaceTypes.Task storage task = _tasks[taskId];
                if (task.status == MarketplaceTypes.TaskStatus.Open && task.escrowAmount > 0) {
                    openAndFundedCount += 1;
                }
            }
        }

        bytes32[] memory openAndFundedTaskIds = new bytes32[](openAndFundedCount);
        uint256 writeIndex;

        for (uint256 i = 0; i < candidates.length; ++i) {
            bytes32 taskId = candidates[i];
            if (_taskExists[taskId]) {
                MarketplaceTypes.Task storage task = _tasks[taskId];
                if (task.status == MarketplaceTypes.TaskStatus.Open && task.escrowAmount > 0) {
                    openAndFundedTaskIds[writeIndex] = taskId;
                    writeIndex += 1;
                }
            }
        }

        return openAndFundedTaskIds;
    }

    function getTasksByBuyer(address buyer) external view override returns (bytes32[] memory) {
        return _tasksByBuyer[buyer];
    }

    function getTasksByProvider(bytes32 nodeId) external view override returns (bytes32[] memory) {
        return _tasksByProvider[nodeId];
    }

    function estimateTaskCost(
        MarketplaceTypes.ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration
    ) external view virtual override returns (uint256);

    function _nodeRegistryRef() internal view returns (INodeRegistry) {
        if (address(_nodeRegistry) == address(0)) {
            revert ContractReferenceNotSet(REFERENCE_NODE_REGISTRY);
        }
        return _nodeRegistry;
    }

    function _escrowManagerRef() internal view returns (IEscrowManager) {
        if (address(_escrowManager) == address(0)) {
            revert ContractReferenceNotSet(REFERENCE_ESCROW_MANAGER);
        }
        return _escrowManager;
    }

    function _proofOfWorkVerifierRef() internal view returns (IProofOfWorkVerifier) {
        if (address(_proofOfWorkVerifier) == address(0)) {
            revert ContractReferenceNotSet(REFERENCE_PROOF_OF_WORK_VERIFIER);
        }
        return _proofOfWorkVerifier;
    }

    function _readTask(bytes32 taskId) internal view returns (MarketplaceTypes.Task memory) {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        return _tasks[taskId];
    }

    function _readTaskResult(bytes32 taskId) internal view returns (MarketplaceTypes.TaskResult memory) {
        _requireTaskId(taskId);
        if (!_taskResultExists[taskId]) {
            revert TaskResultNotFound(taskId);
        }
        return _taskResults[taskId];
    }

    function _initializeTask(MarketplaceTypes.Task memory task) internal {
        if (task.status != MarketplaceTypes.TaskStatus.Open) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Open);
        }
        _storeCreatedTask(task);
        _indexTaskForBuyer(task.taskId);
        _indexTaskAsOpen(task.taskId);
    }

    function _assignTask(
        bytes32 taskId,
        bytes32 nodeId,
        uint256 startedAt
    ) internal {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        _requireTaskOpen(taskId);
        _requireTaskFunded(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        _enforceStatusTransition(task.status, MarketplaceTypes.TaskStatus.Assigned);

        _setTaskAssignment(taskId, nodeId);
        _setTaskTiming(taskId, startedAt, 0);
        _setTaskStatus(taskId, MarketplaceTypes.TaskStatus.Assigned);
        _indexTaskForProvider(taskId);
    }

    function _markTaskInProgress(bytes32 taskId) internal {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        _enforceStatusTransition(task.status, MarketplaceTypes.TaskStatus.InProgress);
        _setTaskStatus(taskId, MarketplaceTypes.TaskStatus.InProgress);
    }

    function _recordTaskResult(
        bytes32 taskId,
        MarketplaceTypes.TaskResult memory taskResult,
        uint256 completedAt
    ) internal {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        if (
            task.status != MarketplaceTypes.TaskStatus.Assigned && task.status != MarketplaceTypes.TaskStatus.InProgress
        ) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Completed);
        }

        _storeTaskResult(taskId, taskResult);
        _setTaskTiming(taskId, task.startedAt, completedAt);
        _setTaskStatus(taskId, MarketplaceTypes.TaskStatus.Completed);
    }

    function _markTaskVerified(bytes32 taskId) internal {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        _enforceStatusTransition(task.status, MarketplaceTypes.TaskStatus.Verified);
        _setTaskStatus(taskId, MarketplaceTypes.TaskStatus.Verified);
    }

    function _markTaskDisputed(bytes32 taskId) internal {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        if (
            task.status != MarketplaceTypes.TaskStatus.Assigned &&
            task.status != MarketplaceTypes.TaskStatus.InProgress &&
            task.status != MarketplaceTypes.TaskStatus.Completed
        ) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Disputed);
        }

        _setTaskStatus(taskId, MarketplaceTypes.TaskStatus.Disputed);
    }

    function _markTaskCancelled(bytes32 taskId) internal {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        _enforceStatusTransition(task.status, MarketplaceTypes.TaskStatus.Cancelled);
        _setTaskStatus(taskId, MarketplaceTypes.TaskStatus.Cancelled);
    }

    function _storeCreatedTask(MarketplaceTypes.Task memory task) private {
        _requireTaskId(task.taskId);
        if (_taskExists[task.taskId]) {
            revert TaskAlreadyExists(task.taskId);
        }
        if (task.buyer == address(0)) {
            revert ZeroBuyerAddressNotAllowed();
        }
        _taskExists[task.taskId] = true;
        _tasks[task.taskId] = task;
    }

    function _storeTaskResult(bytes32 taskId, MarketplaceTypes.TaskResult memory taskResult) private {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        if (taskResult.taskId != taskId) {
            revert ZeroTaskIdNotAllowed();
        }
        if (taskResult.resultHash == bytes32(0)) {
            revert ZeroResultHashNotAllowed();
        }
        _taskResultExists[taskId] = true;
        _taskResults[taskId] = taskResult;
    }

    function _setTaskStatus(bytes32 taskId, MarketplaceTypes.TaskStatus status) private {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        _tasks[taskId].status = status;
    }

    function _setTaskAssignment(bytes32 taskId, bytes32 nodeId) private {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        _requireNodeId(nodeId);
        _tasks[taskId].assignedNode = nodeId;
    }

    function _setTaskTiming(
        bytes32 taskId,
        uint256 startedAt,
        uint256 completedAt
    ) private {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        _tasks[taskId].startedAt = startedAt;
        _tasks[taskId].completedAt = completedAt;
    }

    function _indexTaskForBuyer(bytes32 taskId) private {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        _tasksByBuyer[task.buyer].push(taskId);
    }

    function _indexTaskForProvider(bytes32 taskId) private {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        bytes32 nodeId = _tasks[taskId].assignedNode;
        _requireNodeId(nodeId);
        _tasksByProvider[nodeId].push(taskId);
    }

    function _indexTaskAsOpen(bytes32 taskId) private {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        _openTasksByType[task.resourceType].push(taskId);
    }

    function _requireTaskFundingAllowed(bytes32 taskId) internal view {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        if (task.status != MarketplaceTypes.TaskStatus.Open) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Open);
        }
        if (msg.sender != task.buyer) {
            revert TaskFundingNotAllowed(taskId, msg.sender, task.buyer);
        }
        if (task.escrowAmount != 0) {
            revert TaskEscrowAlreadyFunded(taskId);
        }
        if (msg.value == 0 || msg.value > task.maxPrice) {
            revert TaskValueOutOfRange(msg.value, task.maxPrice);
        }
    }

    function _requireTaskOpen(bytes32 taskId) internal view {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task storage task = _tasks[taskId];
        if (task.status != MarketplaceTypes.TaskStatus.Open) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Open);
        }
    }

    function _requireTaskFunded(bytes32 taskId) internal view {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        if (_tasks[taskId].escrowAmount == 0) {
            revert TaskEscrowNotFunded(taskId);
        }
    }

    function _setTaskEscrowAmount(bytes32 taskId, uint256 amount) internal {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        _tasks[taskId].escrowAmount = amount;
    }

    function _requireTaskExists(bytes32 taskId) internal view {
        if (!_taskExists[taskId]) {
            revert TaskNotFound(taskId);
        }
    }

    function _requireContractReference(address candidate) internal view {
        if (candidate == address(0)) {
            revert ZeroAddressNotAllowed();
        }
        if (candidate.code.length == 0) {
            revert InvalidContractReference(candidate);
        }
    }

    function _requireTaskId(bytes32 taskId) internal pure {
        if (taskId == bytes32(0)) {
            revert ZeroTaskIdNotAllowed();
        }
    }

    function _requireNodeId(bytes32 nodeId) internal pure {
        if (nodeId == bytes32(0)) {
            revert ZeroNodeIdNotAllowed();
        }
    }

    function _enforceStatusTransition(
        MarketplaceTypes.TaskStatus from,
        MarketplaceTypes.TaskStatus to
    ) private pure {
        if (from == MarketplaceTypes.TaskStatus.Open && to == MarketplaceTypes.TaskStatus.Assigned) {
            return;
        }
        if (from == MarketplaceTypes.TaskStatus.Assigned && to == MarketplaceTypes.TaskStatus.InProgress) {
            return;
        }
        if (from == MarketplaceTypes.TaskStatus.Completed && to == MarketplaceTypes.TaskStatus.Verified) {
            return;
        }
        if (
            (from == MarketplaceTypes.TaskStatus.Open ||
                from == MarketplaceTypes.TaskStatus.Assigned ||
                from == MarketplaceTypes.TaskStatus.InProgress) &&
            to == MarketplaceTypes.TaskStatus.Cancelled
        ) {
            return;
        }

        revert InvalidTaskStatusTransition(from, to);
    }

    function _createTask(
        MarketplaceTypes.ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration,
        uint256 maxPrice,
        uint8 minTrustLevel,
        string calldata specificationURI
    ) internal virtual returns (bytes32 taskId);

    function _cancelTask(bytes32 taskId) internal virtual;

    function _disputeTask(bytes32 taskId, string calldata reason) internal virtual;

    function _approveResult(bytes32 taskId) internal virtual;

    function _acceptTask(bytes32 taskId, bytes32 nodeId) internal virtual;

    function _submitResult(bytes32 taskId, bytes32 resultHash, string calldata resultURI) internal virtual;
}
