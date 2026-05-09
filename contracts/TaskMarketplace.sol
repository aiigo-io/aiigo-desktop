// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {IEscrowManager} from "./interfaces/IEscrowManager.sol";
import {INodeRegistry} from "./interfaces/INodeRegistry.sol";
import {IProofOfWorkVerifier} from "./interfaces/IProofOfWorkVerifier.sol";
import {ITaskMarketplace} from "./interfaces/ITaskMarketplace.sol";
import {MarketplaceTypes} from "./types/MarketplaceTypes.sol";

contract TaskMarketplace is ITaskMarketplace, Ownable2Step, Pausable, ReentrancyGuard {
    error ContractReferenceNotSet(bytes32 referenceName);
    error SameContractReference(address currentReference, address candidate);
    error InvalidContractInterface(bytes4 selector, address candidate);
    error InvalidTaskStatusTransition(MarketplaceTypes.TaskStatus from, MarketplaceTypes.TaskStatus to);
    error NonContractAddress(address candidate);
    error NotAssignedNodeOwner(bytes32 taskId, bytes32 nodeId, address caller, address expectedOwner);
    error NotTaskBuyer(bytes32 taskId, address caller, address buyer);
    error NodeComputePowerTooLow(bytes32 nodeId, uint256 availablePower, uint256 requiredPower);
    error NodeInactive(bytes32 nodeId);
    error NodeResourceMismatch(bytes32 nodeId, MarketplaceTypes.ResourceType expected, MarketplaceTypes.ResourceType actual);
    error NodeTrustLevelTooLow(bytes32 nodeId, uint8 availableTrustLevel, uint8 requiredTrustLevel);
    error OwnerRenounceDisabled();
    error TaskAlreadyExists(bytes32 taskId);
    error TaskEscrowAlreadyFunded(bytes32 taskId);
    error TaskEscrowNotFunded(bytes32 taskId);
    error TaskFundingNotAllowed(bytes32 taskId, address caller, address buyer);
    error TaskNotFound(bytes32 taskId);
    error TaskResultAlreadyExists(bytes32 taskId);
    error TaskResultNotFound(bytes32 taskId);
    error TaskValueOutOfRange(uint256 provided, uint256 min, uint256 max);
    error UnderfundableTaskCreation(uint256 quoteMin, uint256 quoteCap);
    error ZeroBuyerAddressNotAllowed();
    error ZeroResultHashNotAllowed();
    error ZeroNodeIdNotAllowed();
    error ZeroAddressNotAllowed();
    error ZeroTaskIdNotAllowed();
    error UnexpectedEscrowId(bytes32 expectedTaskId, bytes32 actualEscrowId);
    error ChallengePeriodActive(bytes32 taskId, uint256 deadline, uint256 currentTime);
    error ChallengePeriodExpired(bytes32 taskId, uint256 deadline);
    error ChallengePeriodNotStarted(bytes32 taskId);
    error DisputeAlreadyResolved(bytes32 taskId);
    error GrossAmountExceedsEscrow(bytes32 taskId, uint256 grossAmount, uint256 escrowAmount);
    error ZeroChallengeWindowNotAllowed();

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
    mapping(bytes32 => MarketplaceTypes.TaskLifecycle) private _taskLifecycles;
    uint256 private _taskNonce;
    uint256 public challengeWindow;

    // ── Pricing config (owner-adjustable) ────────────────────────────────────
    uint256 public startFee;
    mapping(MarketplaceTypes.ResourceType => uint256) public envHourFee;
    mapping(MarketplaceTypes.ResourceType => uint256) public computePowerHourFee;

    // Quote snapshots frozen at task creation time
    mapping(bytes32 => uint256) private _taskQuoteMin;
    mapping(bytes32 => uint256) private _taskQuoteCap;

    bytes32 private constant REFERENCE_NODE_REGISTRY = keccak256("NODE_REGISTRY");
    bytes32 private constant REFERENCE_ESCROW_MANAGER = keccak256("ESCROW_MANAGER");
    bytes32 private constant REFERENCE_PROOF_OF_WORK_VERIFIER = keccak256("PROOF_OF_WORK_VERIFIER");

    event ContractReferenceUpdated(bytes32 indexed referenceName, address indexed previousReference, address indexed newReference);
    event StartFeeUpdated(uint256 previousFee, uint256 newFee);
    event PricingUpdated(
        MarketplaceTypes.ResourceType indexed resourceType,
        uint256 envHourFee,
        uint256 computePowerHourFee
    );

    constructor(address owner_) Ownable(owner_) {
        challengeWindow = 1 days;
        _initDefaultPricing();
    }

    function setChallengeWindow(uint256 window) external onlyOwner {
        if (window == 0) revert ZeroChallengeWindowNotAllowed();
        uint256 previousWindow = challengeWindow;
        challengeWindow = window;
        emit ChallengeWindowUpdated(previousWindow, window);
    }

    function setStartFee(uint256 newStartFee) external onlyOwner {
        uint256 previous = startFee;
        startFee = newStartFee;
        emit StartFeeUpdated(previous, newStartFee);
    }

    function setResourcePricing(
        MarketplaceTypes.ResourceType resourceType,
        uint256 envHourFee_,
        uint256 computePowerHourFee_
    ) external onlyOwner {
        envHourFee[resourceType] = envHourFee_;
        computePowerHourFee[resourceType] = computePowerHourFee_;
        emit PricingUpdated(resourceType, envHourFee_, computePowerHourFee_);
    }

    function setNodeRegistry(address nodeRegistry_) external virtual onlyOwner {
        if (nodeRegistry_ == address(0)) {
            revert ZeroAddressNotAllowed();
        }
        if (nodeRegistry_.code.length == 0) {
            revert NonContractAddress(nodeRegistry_);
        }

        (bool ok, bytes memory returnData) = nodeRegistry_.staticcall(
            abi.encodeWithSelector(INodeRegistry.getNodesByOwner.selector, address(this))
        );
        if (!ok || returnData.length < 64) {
            revert InvalidContractInterface(INodeRegistry.getNodesByOwner.selector, nodeRegistry_);
        }

        abi.decode(returnData, (bytes32[]));

        address previousReference = address(_nodeRegistry);
        if (nodeRegistry_ == previousReference) {
            revert SameContractReference(previousReference, nodeRegistry_);
        }

        _nodeRegistry = INodeRegistry(nodeRegistry_);

        emit ContractReferenceUpdated(REFERENCE_NODE_REGISTRY, previousReference, nodeRegistry_);
    }

    function setEscrowManager(address escrowManager_) external virtual onlyOwner {
        if (escrowManager_ == address(0)) {
            revert ZeroAddressNotAllowed();
        }
        if (escrowManager_.code.length == 0) {
            revert NonContractAddress(escrowManager_);
        }

        (bool ok, bytes memory returnData) = escrowManager_.staticcall(
            abi.encodeWithSelector(IEscrowManager.getTaskMarketplace.selector)
        );
        if (!ok || returnData.length < 32) {
            revert InvalidContractInterface(IEscrowManager.getTaskMarketplace.selector, escrowManager_);
        }

        abi.decode(returnData, (address));

        address previousReference = address(_escrowManager);
        if (escrowManager_ == previousReference) {
            revert SameContractReference(previousReference, escrowManager_);
        }

        _escrowManager = IEscrowManager(escrowManager_);

        emit ContractReferenceUpdated(REFERENCE_ESCROW_MANAGER, previousReference, escrowManager_);
    }

    function setPowVerifier(address proofOfWorkVerifier_) external virtual onlyOwner {
        if (proofOfWorkVerifier_ == address(0)) {
            revert ZeroAddressNotAllowed();
        }
        if (proofOfWorkVerifier_.code.length == 0) {
            revert NonContractAddress(proofOfWorkVerifier_);
        }

        (bool ok, bytes memory returnData) = proofOfWorkVerifier_.staticcall(
            abi.encodeWithSelector(IProofOfWorkVerifier.getVerificationHistory.selector, bytes32(0))
        );
        if (!ok || returnData.length < 64) {
            revert InvalidContractInterface(
                IProofOfWorkVerifier.getVerificationHistory.selector,
                proofOfWorkVerifier_
            );
        }

        abi.decode(returnData, (MarketplaceTypes.VerificationResult[]));

        address previousReference = address(_proofOfWorkVerifier);
        if (proofOfWorkVerifier_ == previousReference) {
            revert SameContractReference(previousReference, proofOfWorkVerifier_);
        }

        _proofOfWorkVerifier = IProofOfWorkVerifier(proofOfWorkVerifier_);

        emit ContractReferenceUpdated(
            REFERENCE_PROOF_OF_WORK_VERIFIER,
            previousReference,
            proofOfWorkVerifier_
        );
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
    ) external override whenNotPaused nonReentrant returns (bytes32 taskId) {
        taskId = _deriveTaskId(msg.sender, _taskNonce);
        _taskNonce += 1;

        // Freeze quote snapshot at creation time so fundTaskEscrow uses
        // consistent pricing regardless of future pricing updates.
        // Ceil duration to the nearest whole hour so a 1h30m task bills as 2h.
        uint256 durationHours = (duration + 1 hours - 1) / 1 hours;
        if (durationHours == 0) durationHours = 1;
        uint256 quoteMin = _estimateTaskCostInternal(resourceType, requiredPower, duration);
        uint256 quoteCap = maxPrice * durationHours;
        // Reject tasks whose price floor already exceeds the funding cap — they
        // can never be funded, so fail fast rather than leaving open-but-locked tasks.
        if (quoteMin > quoteCap) {
            revert UnderfundableTaskCreation(quoteMin, quoteCap);
        }
        _taskQuoteMin[taskId] = quoteMin;
        _taskQuoteCap[taskId] = quoteCap;

        MarketplaceTypes.Task memory task = MarketplaceTypes.Task({
            taskId: taskId,
            buyer: msg.sender,
            assignedNode: bytes32(0),
            resourceType: resourceType,
            requiredPower: requiredPower,
            duration: duration,
            maxPrice: maxPrice,
            escrowAmount: 0,
            createdAt: block.timestamp,
            startedAt: 0,
            completedAt: 0,
            status: MarketplaceTypes.TaskStatus.Open,
            minTrustLevel: minTrustLevel,
            specificationURI: specificationURI
        });

        _initializeTask(task);

        emit TaskCreated(taskId, msg.sender, resourceType, maxPrice);
    }

    function fundTaskEscrow(bytes32 taskId) external payable override whenNotPaused nonReentrant {
        _requireTaskId(taskId);
        _requireTaskFundingAllowed(taskId);

        _setTaskEscrowAmount(taskId, msg.value);
        bytes32 escrowId = _escrowManagerRef().deposit{value: msg.value}(taskId, msg.sender);
        if (escrowId != taskId) {
            revert UnexpectedEscrowId(taskId, escrowId);
        }
    }

    function cancelTask(bytes32 taskId) external override nonReentrant {
        _requireTaskId(taskId);
        MarketplaceTypes.Task memory task = _requireTaskBuyer(taskId, msg.sender);

        _markTaskCancelled(taskId);

        if (task.escrowAmount != 0) {
            _escrowManagerRef().refund(taskId);
        }
        if (task.assignedNode != bytes32(0)) {
            _nodeRegistryRef().unlockNodeStake(task.assignedNode);
        }

        emit TaskCancelled(taskId, task.escrowAmount);
    }

    function disputeTask(bytes32 taskId, string calldata reason) external override nonReentrant {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task memory task = _tasks[taskId];

        if (task.status == MarketplaceTypes.TaskStatus.Completed) {
            // Only buyer may dispute a completed result, and only within the challenge window
            if (task.buyer != msg.sender) {
                revert NotTaskBuyer(taskId, msg.sender, task.buyer);
            }
            uint256 deadline = _taskLifecycles[taskId].challengeDeadline;
            if (deadline == 0) {
                revert ChallengePeriodNotStarted(taskId);
            }
            if (block.timestamp > deadline) {
                revert ChallengePeriodExpired(taskId, deadline);
            }
        } else if (
            task.status == MarketplaceTypes.TaskStatus.Assigned ||
            task.status == MarketplaceTypes.TaskStatus.InProgress
        ) {
            // Buyer or assigned node owner may dispute an in-flight task
            if (task.buyer != msg.sender) {
                if (task.assignedNode == bytes32(0)) {
                    revert NotTaskBuyer(taskId, msg.sender, task.buyer);
                }
                MarketplaceTypes.Node memory node = _nodeRegistryRef().getNode(task.assignedNode);
                if (node.owner != msg.sender) {
                    revert NotAssignedNodeOwner(taskId, task.assignedNode, msg.sender, node.owner);
                }
            }
        } else {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Disputed);
        }

        // Stake intentionally NOT unlocked here; unlocks only on cancel, approve, settle, or resolveDispute
        _markTaskDisputed(taskId);

        _taskLifecycles[taskId].disputedBy = msg.sender;
        _taskLifecycles[taskId].disputeReason = reason;

        emit TaskDisputed(taskId, msg.sender, reason);
    }

    function approveResult(bytes32 taskId) external override whenNotPaused nonReentrant {
        _requireTaskId(taskId);
        MarketplaceTypes.Task memory task = _requireTaskBuyer(taskId, msg.sender);
        _readTaskResult(taskId);

        if (task.status != MarketplaceTypes.TaskStatus.Completed) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Verified);
        }

        address providerOwner = _assignedNodeOwner(taskId);
        uint256 providerPayout = task.escrowAmount - ((task.escrowAmount * 800) / 10_000);

        _markTaskVerified(taskId);
        _escrowManagerRef().release(taskId, providerOwner);
        _nodeRegistryRef().unlockNodeStake(task.assignedNode);

        emit TaskVerified(taskId, providerPayout);
    }

    function acceptTask(bytes32 taskId, bytes32 nodeId) external override whenNotPaused nonReentrant {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        _requireTaskOpen(taskId);
        _requireTaskFunded(taskId);
        _requireNodeId(nodeId);

        MarketplaceTypes.Task memory task = _readTask(taskId);
        MarketplaceTypes.Node memory node = _nodeRegistryRef().getNode(nodeId);

        if (node.owner != msg.sender) {
            revert NotAssignedNodeOwner(taskId, nodeId, msg.sender, node.owner);
        }
        if (node.status != MarketplaceTypes.NodeStatus.Active) {
            revert NodeInactive(nodeId);
        }
        if (node.resourceType != task.resourceType) {
            revert NodeResourceMismatch(nodeId, task.resourceType, node.resourceType);
        }

        uint8 trustLevel = _nodeRegistryRef().getNodeTrustLevel(nodeId);
        if (trustLevel < task.minTrustLevel) {
            revert NodeTrustLevelTooLow(nodeId, trustLevel, task.minTrustLevel);
        }
        if (node.computePower < task.requiredPower) {
            revert NodeComputePowerTooLow(nodeId, node.computePower, task.requiredPower);
        }

        uint256 startedAt = block.timestamp;
        _assignTask(taskId, nodeId, startedAt);
        _nodeRegistryRef().lockNodeStake(nodeId);

        emit TaskAssigned(taskId, nodeId, startedAt);
    }

    function submitResult(
        bytes32 taskId,
        bytes32 resultHash,
        string calldata resultURI
    ) external override whenNotPaused nonReentrant {
        _requireTaskId(taskId);
        MarketplaceTypes.Task memory task = _readTask(taskId);
        if (task.status != MarketplaceTypes.TaskStatus.Assigned && task.status != MarketplaceTypes.TaskStatus.InProgress) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Completed);
        }
        _requireAssignedNodeOwner(taskId, msg.sender);
        if (_taskResultExists[taskId]) {
            revert TaskResultAlreadyExists(taskId);
        }

        MarketplaceTypes.TaskResult memory result = MarketplaceTypes.TaskResult({
            taskId: taskId,
            resultHash: resultHash,
            resultURI: resultURI,
            actualDuration: 0,
            computeUnitsUsed: 0,
            verified: false
        });

        _recordTaskResult(taskId, result, block.timestamp);
        _taskLifecycles[taskId].challengeDeadline = block.timestamp + challengeWindow;

        emit TaskCompleted(taskId, resultHash);
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
        uint256 openAndFundedCount = 0;

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
        uint256 writeIndex = 0;

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

    function settleUndisputedTask(bytes32 taskId) external override whenNotPaused nonReentrant {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.Task memory task = _tasks[taskId];
        if (task.status != MarketplaceTypes.TaskStatus.Completed) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Verified);
        }

        uint256 deadline = _taskLifecycles[taskId].challengeDeadline;
        if (deadline == 0) {
            revert ChallengePeriodNotStarted(taskId);
        }
        if (block.timestamp <= deadline) {
            revert ChallengePeriodActive(taskId, deadline, block.timestamp);
        }

        address providerOwner = _assignedNodeOwner(taskId);
        uint256 platformFee = (task.escrowAmount * 800) / 10_000;
        uint256 providerPayout = task.escrowAmount - platformFee;

        _markTaskVerified(taskId);
        _escrowManagerRef().release(taskId, providerOwner);
        _nodeRegistryRef().unlockNodeStake(task.assignedNode);

        emit TaskVerified(taskId, providerPayout);
        emit TaskUndisputedSettled(taskId, providerOwner, providerPayout, platformFee);
    }

    function resolveDispute(bytes32 taskId, uint256 grossProviderAmount) external override onlyOwner nonReentrant {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);

        MarketplaceTypes.TaskLifecycle storage lifecycle = _taskLifecycles[taskId];
        if (lifecycle.resolved) {
            revert DisputeAlreadyResolved(taskId);
        }

        MarketplaceTypes.Task memory task = _tasks[taskId];
        if (task.status != MarketplaceTypes.TaskStatus.Disputed) {
            revert InvalidTaskStatusTransition(task.status, MarketplaceTypes.TaskStatus.Verified);
        }
        if (grossProviderAmount > task.escrowAmount) {
            revert GrossAmountExceedsEscrow(taskId, grossProviderAmount, task.escrowAmount);
        }

        // Step 1: Execute escrow settlement first. Provider-paying paths may revert
        //         when escrow is paused; if they do, no state is written (CEI ordering).
        uint256 providerPayout;
        uint256 buyerRefund;
        uint256 fee;

        if (grossProviderAmount == 0) {
            _escrowManagerRef().refund(taskId);
            buyerRefund = task.escrowAmount;
        } else {
            address providerOwner = _assignedNodeOwner(taskId);
            fee = (grossProviderAmount * 800) / 10_000;
            providerPayout = grossProviderAmount - fee;
            if (grossProviderAmount == task.escrowAmount) {
                _escrowManagerRef().release(taskId, providerOwner);
            } else {
                buyerRefund = task.escrowAmount - grossProviderAmount;
                _escrowManagerRef().splitPayment(taskId, providerOwner, grossProviderAmount);
            }
        }

        // Step 2: Write lifecycle state (only reached when escrow call succeeded).
        lifecycle.resolved = true;
        lifecycle.resolvedBy = msg.sender;
        lifecycle.grossProviderAmount = grossProviderAmount;

        // Step 3: Unlock stake.
        if (task.assignedNode != bytes32(0)) {
            _nodeRegistryRef().unlockNodeStake(task.assignedNode);
        }

        // Step 4: Mark terminal state and emit full accounting event.
        _markTaskVerified(taskId);

        emit DisputeResolved(taskId, msg.sender, grossProviderAmount, providerPayout, buyerRefund, fee);
    }

    function getTaskLifecycle(bytes32 taskId) external view override returns (MarketplaceTypes.TaskLifecycle memory) {
        _requireTaskId(taskId);
        _requireTaskExists(taskId);
        return _taskLifecycles[taskId];
    }

    function estimateTaskCost(
        MarketplaceTypes.ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration
    ) external view virtual override returns (uint256) {
        return _estimateTaskCostInternal(resourceType, requiredPower, duration);
    }

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
        uint256 quoteMin = _taskQuoteMin[taskId];
        uint256 quoteCap = _taskQuoteCap[taskId];
        if (msg.value < quoteMin || msg.value > quoteCap) {
            revert TaskValueOutOfRange(msg.value, quoteMin, quoteCap);
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

    function _deriveTaskId(address buyer, uint256 taskNonce) private view returns (bytes32) {
        return keccak256(abi.encode(block.chainid, address(this), buyer, taskNonce));
    }

    // ── Pricing helpers ───────────────────────────────────────────────────────

    /// @dev Private so quote snapshots always use on-chain pricing, not override.
    function _estimateTaskCostInternal(
        MarketplaceTypes.ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration
    ) private view returns (uint256) {
        uint256 durationHours = (duration + 1 hours - 1) / 1 hours;
        if (durationHours == 0) durationHours = 1;
        return startFee
            + envHourFee[resourceType] * durationHours
            + computePowerHourFee[resourceType] * requiredPower * durationHours;
    }

    function _initDefaultPricing() private {
        startFee = 0.0002 ether;

        envHourFee[MarketplaceTypes.ResourceType.GPU]     = 0.0008 ether;
        envHourFee[MarketplaceTypes.ResourceType.CPU]     = 0.0002 ether;
        envHourFee[MarketplaceTypes.ResourceType.Network] = 0.0001 ether;
        envHourFee[MarketplaceTypes.ResourceType.Mobile]  = 0.00005 ether;
        envHourFee[MarketplaceTypes.ResourceType.IoT]     = 0.00002 ether;

        computePowerHourFee[MarketplaceTypes.ResourceType.GPU]     = 0.00008 ether;
        computePowerHourFee[MarketplaceTypes.ResourceType.CPU]     = 0.00002 ether;
        computePowerHourFee[MarketplaceTypes.ResourceType.Network] = 0.00001 ether;
        computePowerHourFee[MarketplaceTypes.ResourceType.Mobile]  = 0.000005 ether;
        computePowerHourFee[MarketplaceTypes.ResourceType.IoT]     = 0.000002 ether;
    }

    function _assignedNodeOwner(bytes32 taskId) internal view returns (address owner) {
        MarketplaceTypes.Task memory task = _readTask(taskId);
        if (task.assignedNode == bytes32(0)) {
            revert ZeroNodeIdNotAllowed();
        }

        MarketplaceTypes.Node memory node = _nodeRegistryRef().getNode(task.assignedNode);
        return node.owner;
    }

    function _requireTaskBuyer(bytes32 taskId, address caller) internal view returns (MarketplaceTypes.Task memory task) {
        task = _readTask(taskId);
        if (task.buyer != caller) {
            revert NotTaskBuyer(taskId, caller, task.buyer);
        }
    }

    function _requireAssignedNodeOwner(bytes32 taskId, address caller) internal view returns (MarketplaceTypes.Task memory task) {
        task = _readTask(taskId);
        if (task.assignedNode == bytes32(0)) {
            revert ZeroNodeIdNotAllowed();
        }

        MarketplaceTypes.Node memory node = _nodeRegistryRef().getNode(task.assignedNode);
        if (node.owner != caller) {
            revert NotAssignedNodeOwner(taskId, task.assignedNode, caller, node.owner);
        }
    }

    function _requireTaskParticipant(
        bytes32 taskId,
        address caller
    ) internal view returns (MarketplaceTypes.Task memory task) {
        task = _readTask(taskId);
        if (task.buyer == caller) {
            return task;
        }

        if (task.assignedNode == bytes32(0)) {
            revert NotTaskBuyer(taskId, caller, task.buyer);
        }

        MarketplaceTypes.Node memory node = _nodeRegistryRef().getNode(task.assignedNode);
        if (node.owner != caller) {
            revert NotAssignedNodeOwner(taskId, task.assignedNode, caller, node.owner);
        }
    }

    function _requireTaskExists(bytes32 taskId) internal view {
        if (!_taskExists[taskId]) {
            revert TaskNotFound(taskId);
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
        if (from == MarketplaceTypes.TaskStatus.Disputed && to == MarketplaceTypes.TaskStatus.Verified) {
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

}
