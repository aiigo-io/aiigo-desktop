// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {EscrowManager} from "../contracts/EscrowManager.sol";
import {NodeRegistry} from "../contracts/NodeRegistry.sol";
import {ProofOfWorkVerifier} from "../contracts/ProofOfWorkVerifier.sol";
import {TaskMarketplace} from "../contracts/TaskMarketplace.sol";
import {IEscrowManager} from "../contracts/interfaces/IEscrowManager.sol";
import {INodeRegistry} from "../contracts/interfaces/INodeRegistry.sol";
import {ITaskMarketplace} from "../contracts/interfaces/ITaskMarketplace.sol";
import {MarketplaceTypes} from "../contracts/types/MarketplaceTypes.sol";

interface Vm {
    function deal(address account, uint256 newBalance) external;
    function prank(address sender) external;
    function expectRevert() external;
    function expectRevert(bytes calldata revertData) external;
    function expectEmit(bool checkTopic1, bool checkTopic2, bool checkTopic3, bool checkData) external;
    function warp(uint256 newTimestamp) external;
}

contract InvalidInterfaceStub {
    fallback() external payable {
        assembly {
            mstore(0x00, 0x01)
            return(0x00, 0x20)
        }
    }
}

contract ForceFeeder {
    constructor() payable {}

    function destroy(address payable target) external {
        selfdestruct(target);
    }
}

contract ReentrantProvider {
    IEscrowManager private immutable _escrowManager;

    bool public attemptedReentry;
    bool public successfulReentry;

    constructor(IEscrowManager escrowManager_) {
        _escrowManager = escrowManager_;
    }

    function claimProviderPayout() external {
        _escrowManager.claimProviderPayout();
    }

    receive() external payable {
        if (attemptedReentry) {
            return;
        }

        attemptedReentry = true;
        (bool ok, ) = address(_escrowManager).call(
            abi.encodeWithSelector(IEscrowManager.claimProviderPayout.selector)
        );
        successfulReentry = ok;
    }
}

contract HarnessProofOfWorkVerifier is ProofOfWorkVerifier {
    uint256 private _challengeNonce;

    constructor(address owner_, address nodeRegistry_) ProofOfWorkVerifier(owner_, nodeRegistry_) {}

    function calculateComputePower(uint256 solutionTime, uint256 difficulty) external pure override returns (uint256) {
        if (solutionTime == 0) {
            return difficulty;
        }

        return difficulty / solutionTime;
    }

    function _issueChallenge(bytes32 nodeId) internal override returns (bytes32 challengeId) {
        _challengeNonce += 1;
        challengeId = keccak256(abi.encodePacked(address(this), nodeId, _challengeNonce));

        MarketplaceTypes.Challenge memory challenge = MarketplaceTypes.Challenge({
            challengeId: challengeId,
            nodeId: nodeId,
            seed: keccak256(abi.encodePacked(nodeId, _challengeNonce)),
            difficulty: BASE_DIFFICULTY,
            issuedAt: block.timestamp,
            deadline: block.timestamp + 1 hours,
            completed: false,
            solutionTime: 0
        });

        _registerIssuedChallenge(challenge);
        emit ChallengeIssued(challengeId, nodeId, challenge.difficulty, challenge.deadline);
    }

    function _submitSolution(bytes32 challengeId, uint256 nonce) internal override returns (bool passed) {
        MarketplaceTypes.Challenge memory challenge = _readChallenge(challengeId);
        uint256 solutionTime = nonce == 0 ? 1 : nonce;
        uint256 verifiedPower = BASE_DIFFICULTY / solutionTime;

        MarketplaceTypes.VerificationResult memory result = MarketplaceTypes.VerificationResult({
            nodeId: challenge.nodeId,
            verifiedPower: verifiedPower,
            timestamp: block.timestamp,
            passed: true
        });

        _resolveSolvedChallenge(challengeId, solutionTime, result);
        emit ChallengeSolved(challengeId, challenge.nodeId, solutionTime, verifiedPower);

        return true;
    }
}

contract HarnessTaskMarketplace is TaskMarketplace {
    constructor(address owner_) TaskMarketplace(owner_) {}

    function estimateTaskCost(
        MarketplaceTypes.ResourceType,
        uint256,
        uint256
    ) external pure override returns (uint256) {
        return 0;
    }

    function forceDeposit(bytes32 taskId, address buyer) external payable onlyOwner {
        _escrowManagerRef().deposit{value: msg.value}(taskId, buyer);
    }

    function forceRelease(bytes32 taskId, address provider) external onlyOwner {
        _escrowManagerRef().release(taskId, provider);
    }

    function forceRefund(bytes32 taskId) external onlyOwner {
        _escrowManagerRef().refund(taskId);
    }

    function forceSplit(bytes32 taskId, address provider, uint256 providerShare) external onlyOwner {
        _escrowManagerRef().splitPayment(taskId, provider, providerShare);
    }
}

contract SecuritySkeletonTestBase {
    Vm internal constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    address internal constant BUYER = address(0xB0B);
    address internal constant ALT_BUYER = address(0xCAFE);
    address internal constant PROVIDER = address(0xA11CE);
    address internal constant ALT_PROVIDER = address(0xBEEF);
    address internal constant TREASURY = address(0xD00D);
    address internal constant NEW_TREASURY = address(0xFEE1);

    uint256 internal constant NODE_REGISTRATION_VALUE = 0.6 ether;
    uint256 internal constant DEFAULT_ESCROW = 1 ether;

    HarnessTaskMarketplace internal _marketplace;
    EscrowManager internal _escrowManager;
    NodeRegistry internal _nodeRegistry;

    bytes32 internal _providerNodeId;
    bytes32 internal _altProviderNodeId;

    event BuyerRefundQueued(bytes32 indexed taskId, address indexed recipient, uint256 amount);
    event BuyerRefundClaimed(address indexed recipient, uint256 amount);
    event ProviderPayoutQueued(bytes32 indexed taskId, address indexed recipient, uint256 amount);
    event ProviderPayoutClaimed(address indexed recipient, uint256 amount);
    event TreasuryFeeQueued(bytes32 indexed taskId, address indexed recipient, uint256 amount);
    event TreasuryFeeClaimed(address indexed recipient, uint256 amount);
    event StakeWithdrawalQueued(bytes32 indexed nodeId, address indexed recipient, uint256 amount);
    event StakeWithdrawalClaimed(address indexed recipient, uint256 amount);

    // TaskMarketplace events (redeclared for vm.expectEmit)
    event TaskVerified(bytes32 indexed taskId, uint256 payoutAmount);
    event ChallengeWindowUpdated(uint256 previousWindow, uint256 newWindow);
    event TaskUndisputedSettled(
        bytes32 indexed taskId,
        address indexed provider,
        uint256 providerPayout,
        uint256 platformFee
    );
    event DisputeResolved(
        bytes32 indexed taskId,
        address indexed resolvedBy,
        uint256 grossProviderAmount,
        uint256 providerPayout,
        uint256 buyerRefund,
        uint256 platformFee
    );

    function _setUpSecurityHarness() internal {
        _marketplace = new HarnessTaskMarketplace(address(this));
        _escrowManager = new EscrowManager(address(this), TREASURY, 0);
        _nodeRegistry = new NodeRegistry(address(this), TREASURY, 0);

        _escrowManager.setTaskMarketplace(address(_marketplace));
        _nodeRegistry.setTaskMarketplace(address(_marketplace));
        _marketplace.setEscrowManager(address(_escrowManager));
        _marketplace.setNodeRegistry(address(_nodeRegistry));

        vm.deal(address(this), 100 ether);
        vm.deal(BUYER, 100 ether);
        vm.deal(ALT_BUYER, 100 ether);
        vm.deal(PROVIDER, 100 ether);
        vm.deal(ALT_PROVIDER, 100 ether);
        vm.deal(TREASURY, 100 ether);
        vm.deal(NEW_TREASURY, 100 ether);

        _providerNodeId = _registerNode(PROVIDER, "provider-node");
        _altProviderNodeId = _registerNode(ALT_PROVIDER, "alt-provider-node");

        _nodeRegistry.updateNodeStatus(_providerNodeId, MarketplaceTypes.NodeStatus.Verified);
        _nodeRegistry.updateNodeStatus(_providerNodeId, MarketplaceTypes.NodeStatus.Active);
        _nodeRegistry.updateComputePower(_providerNodeId, 250);

        _nodeRegistry.updateNodeStatus(_altProviderNodeId, MarketplaceTypes.NodeStatus.Verified);
        _nodeRegistry.updateNodeStatus(_altProviderNodeId, MarketplaceTypes.NodeStatus.Active);
        _nodeRegistry.updateComputePower(_altProviderNodeId, 250);
    }

    function _createTask(address buyer, uint256 maxPrice, string memory specificationURI) internal returns (bytes32 taskId) {
        vm.prank(buyer);
        taskId = _marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            maxPrice,
            1,
            specificationURI
        );
    }

    function _fundTask(bytes32 taskId, address buyer, uint256 amount) internal {
        vm.prank(buyer);
        _marketplace.fundTaskEscrow{value: amount}(taskId);
    }

    function _registerNode(address owner, string memory metadataURI) internal returns (bytes32 nodeId) {
        return _registerNodeWithResource(owner, MarketplaceTypes.ResourceType.GPU, metadataURI);
    }

    function _registerNodeWithResource(
        address owner,
        MarketplaceTypes.ResourceType resourceType,
        string memory metadataURI
    ) internal returns (bytes32 nodeId) {
        vm.prank(owner);
        nodeId = _nodeRegistry.registerNode{value: NODE_REGISTRATION_VALUE}(resourceType, metadataURI);
    }

    function _providerPayout(uint256 amount) internal pure returns (uint256) {
        return amount - _platformFee(amount);
    }

    function _platformFee(uint256 amount) internal pure returns (uint256) {
        return (amount * 800) / 10_000;
    }

    receive() external payable {}
}

contract TaskMarketplaceBoundaryAndEscrowConstraintsTest is SecuritySkeletonTestBase {
    function setUp() public {
        _setUpSecurityHarness();
    }

    function testCreateTaskRejectsEthValue() public {
        bytes memory callData = abi.encodeWithSelector(
            _marketplace.createTask.selector,
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "spec"
        );

        vm.prank(BUYER);
        (bool success, ) = address(_marketplace).call{value: 1 ether}(callData);
        require(!success, "createTask accepted ETH");
    }

    function testFundingBoundaryEnforcesBuyerConsistencyAndEscrowDoesNotIntrospectTaskBuyer() public {
        bytes32 protectedTaskId = _createTask(BUYER, 2 ether, "protected-task");

        vm.prank(ALT_BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(TaskMarketplace.TaskFundingNotAllowed.selector, protectedTaskId, ALT_BUYER, BUYER)
        );
        _marketplace.fundTaskEscrow{value: DEFAULT_ESCROW}(protectedTaskId);

        bytes32 bypassTaskId = _createTask(BUYER, 2 ether, "bypass-task");
        _marketplace.forceDeposit{value: DEFAULT_ESCROW}(bypassTaskId, ALT_BUYER);

        MarketplaceTypes.Task memory task = _marketplace.getTask(bypassTaskId);
        MarketplaceTypes.EscrowDeposit memory escrow = _escrowManager.getEscrow(bypassTaskId);

        require(task.buyer == BUYER, "task buyer changed");
        require(escrow.buyer == ALT_BUYER, "escrow should trust marketplace boundary");
    }

    function testDepositRejectsZeroValueZeroBuyerDuplicateAndNonMarketplaceCaller() public {
        bytes32 manualTaskId = keccak256("manual-task");

        vm.prank(BUYER);
        vm.expectRevert(abi.encodeWithSelector(EscrowManager.UnauthorizedTaskMarketplace.selector, BUYER));
        _escrowManager.deposit{value: DEFAULT_ESCROW}(manualTaskId, BUYER);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.AmountMustBeGreaterThanZero.selector));
        _marketplace.forceDeposit{value: 0}(manualTaskId, BUYER);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.InvalidBuyer.selector));
        _marketplace.forceDeposit{value: DEFAULT_ESCROW}(manualTaskId, address(0));

        _marketplace.forceDeposit{value: DEFAULT_ESCROW}(manualTaskId, BUYER);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.EscrowAlreadyExists.selector, manualTaskId));
        _marketplace.forceDeposit{value: DEFAULT_ESCROW}(manualTaskId, BUYER);
    }

    function testFundedOpenTasksFilterStillExcludesUnfundedAndTerminalTasks() public {
        bytes32 unfundedOpen = _createTask(BUYER, 2 ether, "unfunded-open");
        bytes32 fundedOpen = _createTask(BUYER, 2 ether, "funded-open");
        _fundTask(fundedOpen, BUYER, DEFAULT_ESCROW);

        bytes32 assignedTask = _createTask(BUYER, 2 ether, "assigned-task");
        _fundTask(assignedTask, BUYER, DEFAULT_ESCROW);
        vm.prank(PROVIDER);
        _marketplace.acceptTask(assignedTask, _providerNodeId);

        bytes32 cancelledTask = _createTask(BUYER, 2 ether, "cancelled-task");
        _fundTask(cancelledTask, BUYER, DEFAULT_ESCROW);
        vm.prank(BUYER);
        _marketplace.cancelTask(cancelledTask);

        bytes32[] memory openTasks = _marketplace.getOpenTasks(MarketplaceTypes.ResourceType.GPU);

        require(openTasks.length == 1, "unexpected open task count");
        require(openTasks[0] == fundedOpen, "wrong open task returned");
        require(openTasks[0] != unfundedOpen, "unfunded task leaked");
    }

    function testBaseAuthorizationAndAcceptanceMatrix() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "auth-matrix-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);

        vm.prank(ALT_BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.NotAssignedNodeOwner.selector,
                taskId,
                _providerNodeId,
                ALT_BUYER,
                PROVIDER
            )
        );
        _marketplace.acceptTask(taskId, _providerNodeId);

        bytes32 trustTaskId = _createTask(BUYER, 2 ether, "trust-task");
        vm.prank(BUYER);
        trustTaskId = _marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            2,
            "trust-task"
        );
        _fundTask(trustTaskId, BUYER, DEFAULT_ESCROW);

        vm.prank(PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(TaskMarketplace.NodeTrustLevelTooLow.selector, _providerNodeId, uint8(1), uint8(2))
        );
        _marketplace.acceptTask(trustTaskId, _providerNodeId);

        _nodeRegistry.updateComputePower(_altProviderNodeId, 25);
        bytes32 lowPowerTaskId = _createTask(BUYER, 2 ether, "low-power-task");
        _fundTask(lowPowerTaskId, BUYER, DEFAULT_ESCROW);

        vm.prank(ALT_PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(TaskMarketplace.NodeComputePowerTooLow.selector, _altProviderNodeId, 25, 100)
        );
        _marketplace.acceptTask(lowPowerTaskId, _altProviderNodeId);

        bytes32 cpuNodeId = _registerNodeWithResource(PROVIDER, MarketplaceTypes.ResourceType.CPU, "cpu-node");
        _nodeRegistry.updateNodeStatus(cpuNodeId, MarketplaceTypes.NodeStatus.Verified);
        _nodeRegistry.updateNodeStatus(cpuNodeId, MarketplaceTypes.NodeStatus.Active);
        _nodeRegistry.updateComputePower(cpuNodeId, 250);

        bytes32 resourceTaskId = _createTask(BUYER, 2 ether, "resource-task");
        _fundTask(resourceTaskId, BUYER, DEFAULT_ESCROW);

        vm.prank(PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.NodeResourceMismatch.selector,
                cpuNodeId,
                MarketplaceTypes.ResourceType.GPU,
                MarketplaceTypes.ResourceType.CPU
            )
        );
        _marketplace.acceptTask(resourceTaskId, cpuNodeId);

        bytes32 inactiveNodeId = _registerNode(PROVIDER, "inactive-node");
        bytes32 inactiveTaskId = _createTask(BUYER, 2 ether, "inactive-task");
        _fundTask(inactiveTaskId, BUYER, DEFAULT_ESCROW);

        vm.prank(PROVIDER);
        vm.expectRevert(abi.encodeWithSelector(TaskMarketplace.NodeInactive.selector, inactiveNodeId));
        _marketplace.acceptTask(inactiveTaskId, inactiveNodeId);

        vm.prank(PROVIDER);
        _marketplace.acceptTask(taskId, _providerNodeId);

        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "task lock missing after accept");

        vm.prank(ALT_BUYER);
        vm.expectRevert(abi.encodeWithSelector(TaskMarketplace.NotTaskBuyer.selector, taskId, ALT_BUYER, BUYER));
        _marketplace.cancelTask(taskId);

        vm.prank(ALT_BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.NotAssignedNodeOwner.selector,
                taskId,
                _providerNodeId,
                ALT_BUYER,
                PROVIDER
            )
        );
        _marketplace.disputeTask(taskId, "no standing");

        vm.prank(ALT_PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.NotAssignedNodeOwner.selector,
                taskId,
                _providerNodeId,
                ALT_PROVIDER,
                PROVIDER
            )
        );
        _marketplace.submitResult(taskId, keccak256("bad"), "bad-uri");

        vm.prank(PROVIDER);
        _marketplace.submitResult(taskId, keccak256("good"), "good-uri");

        vm.prank(PROVIDER);
        vm.expectRevert(abi.encodeWithSelector(TaskMarketplace.NotTaskBuyer.selector, taskId, PROVIDER, BUYER));
        _marketplace.approveResult(taskId);

        vm.prank(BUYER);
        _marketplace.approveResult(taskId);

        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "task lock not cleared after approval");
    }
}

contract EscrowManagerSettlementAndClaimTest is SecuritySkeletonTestBase {
    function setUp() public {
        _setUpSecurityHarness();
    }

    function testReleaseAndSplitPaymentFormulasMatchExactRoundingRules() public {
        bytes32 releaseTaskId = _createTask(BUYER, 2 ether, "release-task");
        uint256 releaseAmount = 1 ether + 1 wei;
        _fundTask(releaseTaskId, BUYER, releaseAmount);

        _marketplace.forceRelease(releaseTaskId, PROVIDER);

        uint256 expectedReleaseFee = _platformFee(releaseAmount);
        uint256 expectedReleasePayout = _providerPayout(releaseAmount);

        require(_escrowManager.getPendingProviderPayout(PROVIDER) == expectedReleasePayout, "wrong release payout");
        require(_escrowManager.getPendingTreasuryFees(TREASURY) == expectedReleaseFee, "wrong release fee");

        // splitPayment now uses gross provider amount semantics:
        // fee is deducted from grossProviderAmount; buyer receives escrowAmount - grossProviderAmount
        bytes32 splitTaskId = _createTask(BUYER, 2 ether, "split-task");
        uint256 splitAmount = 1 ether;
        uint256 grossProviderAmount = 0.61 ether;
        _fundTask(splitTaskId, BUYER, splitAmount);

        _marketplace.forceSplit(splitTaskId, ALT_PROVIDER, grossProviderAmount);

        uint256 splitFee = _platformFee(grossProviderAmount);
        uint256 expectedNetProviderPayout = grossProviderAmount - splitFee;
        uint256 expectedBuyerRefund = splitAmount - grossProviderAmount;

        require(
            _escrowManager.getPendingProviderPayout(ALT_PROVIDER) == expectedNetProviderPayout,
            "wrong split net provider payout"
        );
        require(
            _escrowManager.getPendingBuyerRefund(BUYER) == expectedBuyerRefund,
            "wrong split buyer refund"
        );
        require(
            _escrowManager.getPendingTreasuryFees(TREASURY) == expectedReleaseFee + splitFee,
            "wrong split treasury fee"
        );
    }

    function testSettlementFinalityMatrixBlocksDoubleAndCrossSettlement() public {
        bytes32 releasedTaskId = _createTask(BUYER, 2 ether, "released-task");
        _fundTask(releasedTaskId, BUYER, DEFAULT_ESCROW);
        _marketplace.forceRelease(releasedTaskId, PROVIDER);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.EscrowAlreadySettled.selector, releasedTaskId));
        _marketplace.forceRelease(releasedTaskId, PROVIDER);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.EscrowAlreadySettled.selector, releasedTaskId));
        _marketplace.forceSplit(releasedTaskId, PROVIDER, 0.5 ether);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.EscrowAlreadySettled.selector, releasedTaskId));
        _marketplace.forceRefund(releasedTaskId);

        bytes32 splitTaskId = _createTask(BUYER, 2 ether, "split-finality-task");
        _fundTask(splitTaskId, BUYER, DEFAULT_ESCROW);
        _marketplace.forceSplit(splitTaskId, PROVIDER, 0.55 ether);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.EscrowAlreadySettled.selector, splitTaskId));
        _marketplace.forceSplit(splitTaskId, PROVIDER, 0.55 ether);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.EscrowAlreadySettled.selector, splitTaskId));
        _marketplace.forceRelease(splitTaskId, PROVIDER);

        vm.expectRevert(abi.encodeWithSelector(EscrowManager.EscrowAlreadySettled.selector, splitTaskId));
        _marketplace.forceRefund(splitTaskId);
    }

    function testPauseSemanticsAllowClaimsAndRefundButBlockReleaseAndSplit() public {
        bytes32 refundTaskId = _createTask(BUYER, 2 ether, "paused-refund-task");
        _fundTask(refundTaskId, BUYER, DEFAULT_ESCROW);

        bytes32 blockedReleaseTaskId = _createTask(BUYER, 2 ether, "blocked-release-task");
        _fundTask(blockedReleaseTaskId, BUYER, DEFAULT_ESCROW);

        bytes32 blockedSplitTaskId = _createTask(BUYER, 2 ether, "blocked-split-task");
        _fundTask(blockedSplitTaskId, BUYER, DEFAULT_ESCROW);

        _escrowManager.pause();
        _marketplace.forceRefund(refundTaskId);

        uint256 buyerBalanceBefore = BUYER.balance;
        vm.prank(BUYER);
        _escrowManager.claimBuyerRefund();
        require(BUYER.balance == buyerBalanceBefore + DEFAULT_ESCROW, "buyer claim while paused failed");

        vm.expectRevert();
        _marketplace.forceRelease(blockedReleaseTaskId, PROVIDER);

        vm.expectRevert();
        _marketplace.forceSplit(blockedSplitTaskId, PROVIDER, 0.5 ether);

        _escrowManager.unpause();

        bytes32 releaseTaskId = _createTask(BUYER, 2 ether, "claim-release-task");
        _fundTask(releaseTaskId, BUYER, DEFAULT_ESCROW);
        _marketplace.forceRelease(releaseTaskId, PROVIDER);

        _escrowManager.pause();

        uint256 providerBalanceBefore = PROVIDER.balance;
        uint256 treasuryBalanceBefore = TREASURY.balance;
        uint256 expectedProviderPayout = _providerPayout(DEFAULT_ESCROW);
        uint256 expectedTreasuryFee = _platformFee(DEFAULT_ESCROW);

        vm.prank(PROVIDER);
        _escrowManager.claimProviderPayout();
        vm.prank(TREASURY);
        _escrowManager.claimTreasuryFees();

        require(PROVIDER.balance == providerBalanceBefore + expectedProviderPayout, "provider claim while paused failed");
        require(TREASURY.balance == treasuryBalanceBefore + expectedTreasuryFee, "treasury claim while paused failed");
    }

    function testRecipientSpecificLedgersAndTreasuryUpdateCannotRedirectOldFees() public {
        bytes32 providerTaskId = _createTask(BUYER, 2 ether, "provider-task");
        _fundTask(providerTaskId, BUYER, DEFAULT_ESCROW);
        _marketplace.forceRelease(providerTaskId, PROVIDER);

        bytes32 altProviderTaskId = _createTask(ALT_BUYER, 2 ether, "alt-provider-task");
        _fundTask(altProviderTaskId, ALT_BUYER, DEFAULT_ESCROW);
        _marketplace.forceRelease(altProviderTaskId, ALT_PROVIDER);

        require(_escrowManager.getPendingProviderPayout(PROVIDER) == _providerPayout(DEFAULT_ESCROW), "provider ledger mismatch");
        require(
            _escrowManager.getPendingProviderPayout(ALT_PROVIDER) == _providerPayout(DEFAULT_ESCROW),
            "alt provider ledger mismatch"
        );

        bytes32 buyerRefundTaskId = _createTask(BUYER, 2 ether, "buyer-refund-task");
        _fundTask(buyerRefundTaskId, BUYER, DEFAULT_ESCROW);
        _marketplace.forceRefund(buyerRefundTaskId);

        bytes32 altBuyerRefundTaskId = _createTask(ALT_BUYER, 2 ether, "alt-buyer-refund-task");
        _fundTask(altBuyerRefundTaskId, ALT_BUYER, DEFAULT_ESCROW);
        _marketplace.forceRefund(altBuyerRefundTaskId);

        require(_escrowManager.getPendingBuyerRefund(BUYER) == DEFAULT_ESCROW, "buyer refund ledger mismatch");
        require(
            _escrowManager.getPendingBuyerRefund(ALT_BUYER) == DEFAULT_ESCROW,
            "alt buyer refund ledger mismatch"
        );

        uint256 oldTreasuryFees = _escrowManager.getPendingTreasuryFees(TREASURY);
        _escrowManager.setTreasury(NEW_TREASURY);

        bytes32 newTreasuryTaskId = _createTask(BUYER, 2 ether, "new-treasury-task");
        _fundTask(newTreasuryTaskId, BUYER, DEFAULT_ESCROW);
        _marketplace.forceRelease(newTreasuryTaskId, PROVIDER);

        uint256 perReleaseFee = _platformFee(DEFAULT_ESCROW);

        require(oldTreasuryFees == perReleaseFee * 2, "old treasury fee baseline mismatch");
        require(_escrowManager.getPendingTreasuryFees(TREASURY) == oldTreasuryFees, "old treasury fees redirected");
        require(
            _escrowManager.getPendingTreasuryFees(NEW_TREASURY) == perReleaseFee,
            "new treasury fee missing"
        );
    }

    function testQueuedAndClaimedEventsIncludeRecipientAndAmount() public {
        bytes32 refundTaskId = _createTask(BUYER, 2 ether, "event-refund-task");
        _fundTask(refundTaskId, BUYER, DEFAULT_ESCROW);

        vm.expectEmit(true, true, false, true);
        emit BuyerRefundQueued(refundTaskId, BUYER, DEFAULT_ESCROW);
        _marketplace.forceRefund(refundTaskId);

        vm.prank(BUYER);
        vm.expectEmit(true, false, false, true);
        emit BuyerRefundClaimed(BUYER, DEFAULT_ESCROW);
        _escrowManager.claimBuyerRefund();

        bytes32 releaseTaskId = _createTask(BUYER, 2 ether, "event-release-task");
        _fundTask(releaseTaskId, BUYER, DEFAULT_ESCROW);

        uint256 expectedProviderPayout = _providerPayout(DEFAULT_ESCROW);
        uint256 expectedTreasuryFee = _platformFee(DEFAULT_ESCROW);

        vm.expectEmit(true, true, false, true);
        emit ProviderPayoutQueued(releaseTaskId, PROVIDER, expectedProviderPayout);
        vm.expectEmit(true, true, false, true);
        emit TreasuryFeeQueued(releaseTaskId, TREASURY, expectedTreasuryFee);
        _marketplace.forceRelease(releaseTaskId, PROVIDER);

        vm.prank(PROVIDER);
        vm.expectEmit(true, false, false, true);
        emit ProviderPayoutClaimed(PROVIDER, expectedProviderPayout);
        _escrowManager.claimProviderPayout();

        vm.prank(TREASURY);
        vm.expectEmit(true, false, false, true);
        emit TreasuryFeeClaimed(TREASURY, expectedTreasuryFee);
        _escrowManager.claimTreasuryFees();
    }

    function testClaimUsesCallAndReentrantReceiverCannotDrainTwice() public {
        ReentrantProvider reentrantProvider = new ReentrantProvider(IEscrowManager(address(_escrowManager)));
        bytes32 taskId = _createTask(BUYER, 2 ether, "reentrant-provider-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _marketplace.forceRelease(taskId, address(reentrantProvider));

        uint256 expectedPayout = _providerPayout(DEFAULT_ESCROW);
        reentrantProvider.claimProviderPayout();

        require(reentrantProvider.attemptedReentry(), "receiver did not execute on call");
        require(!reentrantProvider.successfulReentry(), "reentrant claim should fail");
        require(address(reentrantProvider).balance == expectedPayout, "receiver drained unexpected amount");
        require(_escrowManager.getPendingProviderPayout(address(reentrantProvider)) == 0, "pending payout not cleared");
    }

    function testDirectEthRevertsAndForceFedEthPreservesObligationSemantics() public {
        (bool escrowSendOk, ) = address(_escrowManager).call{value: 1 wei}("");
        require(!escrowSendOk, "escrow accepted direct ETH");

        bytes32 taskId = _createTask(BUYER, 2 ether, "force-fed-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);

        require(_escrowManager.totalAccountedObligations() == DEFAULT_ESCROW, "unexpected obligations before force feed");

        ForceFeeder feeder = new ForceFeeder{value: 0.4 ether}();
        feeder.destroy(payable(address(_escrowManager)));

        require(_escrowManager.totalAccountedObligations() == DEFAULT_ESCROW, "force-fed ETH changed obligations");
        require(address(_escrowManager).balance == 1.4 ether, "force-fed ETH missing");

        _marketplace.forceRefund(taskId);
        vm.prank(BUYER);
        _escrowManager.claimBuyerRefund();

        require(_escrowManager.totalAccountedObligations() == 0, "obligations not cleared after claim");
        require(address(_escrowManager).balance == 0.4 ether, "force-fed surplus should remain");
    }
}

contract NodeRegistrySecuritySkeletonTest is SecuritySkeletonTestBase {
    function setUp() public {
        _setUpSecurityHarness();
    }

    function testStakeWithdrawalBlockedWhileNodeHasPendingTaskLockAndUnlocksOnResolveDispute() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "locked-node-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);

        vm.prank(PROVIDER);
        _marketplace.acceptTask(taskId, _providerNodeId);

        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "pending task lock missing");

        vm.prank(PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(NodeRegistry.NodeHasPendingTasks.selector, _providerNodeId, uint256(1))
        );
        _nodeRegistry.withdrawStake(_providerNodeId, 0.1 ether);

        // Dispute does NOT unlock stake
        vm.prank(PROVIDER);
        _marketplace.disputeTask(taskId, "provider dispute");

        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "stake must remain locked after dispute");

        vm.prank(PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(NodeRegistry.NodeHasPendingTasks.selector, _providerNodeId, uint256(1))
        );
        _nodeRegistry.withdrawStake(_providerNodeId, 0.1 ether);

        // Owner resolves dispute (full refund) — this unlocks the stake
        _marketplace.resolveDispute(taskId, 0);

        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "pending task lock not cleared after resolveDispute");

        _nodeRegistry.updateNodeStatus(_providerNodeId, MarketplaceTypes.NodeStatus.Inactive);

        vm.prank(PROVIDER);
        _nodeRegistry.withdrawStake(_providerNodeId, 0.1 ether);
    }

    function testPauserRoleAndOperatorRenounceSemantics() public {
        bytes32 nodePauserRole = _nodeRegistry.PAUSER_ROLE();
        bytes32 escrowPauserRole = _escrowManager.PAUSER_ROLE();
        bytes32 updaterRole = _nodeRegistry.UPDATER_ROLE();
        bytes32 releaserRole = _escrowManager.RELEASER_ROLE();

        _nodeRegistry.grantRole(nodePauserRole, PROVIDER);
        _escrowManager.grantRole(escrowPauserRole, PROVIDER);

        vm.prank(PROVIDER);
        _nodeRegistry.pause();
        vm.prank(PROVIDER);
        _nodeRegistry.unpause();

        vm.prank(PROVIDER);
        _escrowManager.pause();
        vm.prank(PROVIDER);
        _escrowManager.unpause();

        _nodeRegistry.grantRole(updaterRole, PROVIDER);
        _escrowManager.grantRole(releaserRole, PROVIDER);

        vm.prank(PROVIDER);
        _nodeRegistry.renounceRole(updaterRole, PROVIDER);

        vm.prank(PROVIDER);
        _escrowManager.renounceRole(releaserRole, PROVIDER);

        require(!_nodeRegistry.hasRole(updaterRole, PROVIDER), "updater role not renounced");
        require(!_escrowManager.hasRole(releaserRole, PROVIDER), "releaser role not renounced");

        bytes32 defaultAdminRole = _nodeRegistry.DEFAULT_ADMIN_ROLE();
        vm.expectRevert(abi.encodeWithSelector(NodeRegistry.RoleRenounceDisabled.selector));
        _nodeRegistry.renounceRole(defaultAdminRole, address(this));

        bytes32 escrowDefaultAdminRole = _escrowManager.DEFAULT_ADMIN_ROLE();
        vm.expectRevert(abi.encodeWithSelector(EscrowManager.RoleRenounceDisabled.selector));
        _escrowManager.renounceRole(escrowDefaultAdminRole, address(this));
    }

    function testStakeWithdrawalAndTreasuryClaimsWorkWhilePausedAndRemainRecipientSpecific() public {
        _nodeRegistry.updateNodeStatus(_providerNodeId, MarketplaceTypes.NodeStatus.Inactive);
        _nodeRegistry.updateNodeStatus(_altProviderNodeId, MarketplaceTypes.NodeStatus.Inactive);

        vm.prank(PROVIDER);
        _nodeRegistry.withdrawStake(_providerNodeId, 0.15 ether);

        vm.prank(ALT_PROVIDER);
        _nodeRegistry.withdrawStake(_altProviderNodeId, 0.2 ether);

        require(_nodeRegistry.getPendingStakeWithdrawal(PROVIDER) == 0.15 ether, "provider stake ledger mismatch");
        require(
            _nodeRegistry.getPendingStakeWithdrawal(ALT_PROVIDER) == 0.2 ether,
            "alt provider stake ledger mismatch"
        );

        require(
            _nodeRegistry.getPendingTreasuryFees(TREASURY) == 0.2 ether,
            "registration fees should accrue to treasury"
        );

        _nodeRegistry.pause();

        uint256 providerBalanceBefore = PROVIDER.balance;
        uint256 treasuryBalanceBefore = TREASURY.balance;

        vm.prank(PROVIDER);
        _nodeRegistry.claimStakeWithdrawal();

        vm.prank(TREASURY);
        _nodeRegistry.claimTreasuryFees();

        require(PROVIDER.balance == providerBalanceBefore + 0.15 ether, "paused stake claim failed");
        require(TREASURY.balance == treasuryBalanceBefore + 0.2 ether, "paused treasury claim failed");
        require(_nodeRegistry.getPendingStakeWithdrawal(PROVIDER) == 0, "provider claim not cleared");
    }

    function testWithdrawWhilePausedDirectEthRevertsAndHistoricalTreasuryDebtStaysBound() public {
        (bool registrySendOk, ) = address(_nodeRegistry).call{value: 1 wei}("");
        require(!registrySendOk, "node registry accepted direct ETH");

        _nodeRegistry.setTreasury(NEW_TREASURY);
        _nodeRegistry.updateNodeStatus(_providerNodeId, MarketplaceTypes.NodeStatus.Inactive);
        _nodeRegistry.updateNodeStatus(_altProviderNodeId, MarketplaceTypes.NodeStatus.Inactive);

        vm.prank(PROVIDER);
        _nodeRegistry.withdrawStake(_providerNodeId, 0.1 ether);

        _nodeRegistry.pause();

        vm.prank(ALT_PROVIDER);
        _nodeRegistry.withdrawStake(_altProviderNodeId, 0.1 ether);

        require(_nodeRegistry.getPendingStakeWithdrawal(PROVIDER) == 0.1 ether, "provider withdrawal missing");
        require(_nodeRegistry.getPendingStakeWithdrawal(ALT_PROVIDER) == 0.1 ether, "alt withdrawal missing");
        require(_nodeRegistry.getPendingTreasuryFees(TREASURY) == 0.2 ether, "old treasury redirected");
        require(_nodeRegistry.getPendingTreasuryFees(NEW_TREASURY) == 0, "new treasury should not own old fees");
    }

    function testStakeWithdrawalEventsIncludeRecipientAndAmount() public {
        _nodeRegistry.updateNodeStatus(_providerNodeId, MarketplaceTypes.NodeStatus.Inactive);

        vm.expectEmit(true, true, false, true);
        emit StakeWithdrawalQueued(_providerNodeId, PROVIDER, 0.12 ether);
        vm.prank(PROVIDER);
        _nodeRegistry.withdrawStake(_providerNodeId, 0.12 ether);

        vm.prank(PROVIDER);
        vm.expectEmit(true, false, false, true);
        emit StakeWithdrawalClaimed(PROVIDER, 0.12 ether);
        _nodeRegistry.claimStakeWithdrawal();
    }
}

contract GovernanceAndProofSkeletonTest is SecuritySkeletonTestBase {
    HarnessProofOfWorkVerifier private _powVerifier;
    InvalidInterfaceStub private _invalidInterfaceStub;

    function setUp() public {
        _setUpSecurityHarness();
        _powVerifier = new HarnessProofOfWorkVerifier(address(this), address(_nodeRegistry));
        _invalidInterfaceStub = new InvalidInterfaceStub();
    }

    function testSetterSemanticsRejectSameReferenceAndWrongInterface() public {
        vm.expectRevert(
            abi.encodeWithSelector(EscrowManager.SameContractReference.selector, address(_marketplace), address(_marketplace))
        );
        _escrowManager.setTaskMarketplace(address(_marketplace));

        vm.expectRevert(
            abi.encodeWithSelector(NodeRegistry.SameContractReference.selector, address(_marketplace), address(_marketplace))
        );
        _nodeRegistry.setTaskMarketplace(address(_marketplace));

        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.SameContractReference.selector,
                address(_nodeRegistry),
                address(_nodeRegistry)
            )
        );
        _marketplace.setNodeRegistry(address(_nodeRegistry));

        vm.expectRevert(
            abi.encodeWithSelector(
                ProofOfWorkVerifier.SameContractReference.selector,
                address(_nodeRegistry),
                address(_nodeRegistry)
            )
        );
        _powVerifier.setNodeRegistry(address(_nodeRegistry));

        vm.expectRevert(
            abi.encodeWithSelector(
                EscrowManager.InvalidContractInterface.selector,
                ITaskMarketplace.getTasksByBuyer.selector,
                address(_invalidInterfaceStub)
            )
        );
        _escrowManager.setTaskMarketplace(address(_invalidInterfaceStub));

        vm.expectRevert(
            abi.encodeWithSelector(
                NodeRegistry.InvalidContractInterface.selector,
                ITaskMarketplace.getTasksByBuyer.selector,
                address(_invalidInterfaceStub)
            )
        );
        _nodeRegistry.setTaskMarketplace(address(_invalidInterfaceStub));

        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.InvalidContractInterface.selector,
                INodeRegistry.getNodesByOwner.selector,
                address(_invalidInterfaceStub)
            )
        );
        _marketplace.setNodeRegistry(address(_invalidInterfaceStub));

        vm.expectRevert(
            abi.encodeWithSelector(
                ProofOfWorkVerifier.InvalidContractInterface.selector,
                INodeRegistry.getNodesByOwner.selector,
                address(_invalidInterfaceStub)
            )
        );
        _powVerifier.setNodeRegistry(address(_invalidInterfaceStub));
    }

    function testProofOfWorkChallengeSkeletonStoresCompletionHistory() public {
        bytes32 challengeId = _powVerifier.issueChallenge(_providerNodeId);

        MarketplaceTypes.Challenge memory challenge = _powVerifier.getChallenge(challengeId);
        require(challenge.nodeId == _providerNodeId, "challenge node mismatch");
        require(!challenge.completed, "challenge should start open");

        bool passed = _powVerifier.submitSolution(challengeId, 5);
        require(passed, "solution should pass");

        MarketplaceTypes.Challenge memory completedChallenge = _powVerifier.getChallenge(challengeId);
        require(completedChallenge.completed, "challenge completion not recorded");
        require(completedChallenge.solutionTime == 5, "solution time mismatch");

        MarketplaceTypes.VerificationResult[] memory history = _powVerifier.getVerificationHistory(_providerNodeId);
        require(history.length == 1, "verification history missing");
        require(history[0].nodeId == _providerNodeId, "history node mismatch");
        require(history[0].passed, "verification result should pass");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Optimistic Settlement & Dispute Resolution Tests
// ─────────────────────────────────────────────────────────────────────────────

contract OptimisticSettlementAndDisputeResolutionTest is SecuritySkeletonTestBase {
    uint256 internal constant CHALLENGE_WINDOW = 1 hours;

    function setUp() public {
        _setUpSecurityHarness();
        _marketplace.setChallengeWindow(CHALLENGE_WINDOW);
    }

    // ── helpers ──────────────────────────────────────────────────────────────

    function _acceptAndSubmit(
        bytes32 taskId,
        address provider,
        bytes32 nodeId
    ) internal {
        vm.prank(provider);
        _marketplace.acceptTask(taskId, nodeId);
        vm.prank(provider);
        _marketplace.submitResult(taskId, keccak256("result"), "result-uri");
    }

    // ── lifecycle getter ──────────────────────────────────────────────────────

    function testTaskLifecycleGetterTracksDeadlineDisputeAndResolution() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "lifecycle-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        MarketplaceTypes.TaskLifecycle memory lc = _marketplace.getTaskLifecycle(taskId);
        require(lc.challengeDeadline == block.timestamp + CHALLENGE_WINDOW, "deadline not set after submit");
        require(lc.disputedBy == address(0), "disputedBy should be zero before dispute");
        require(!lc.resolved, "should not be resolved yet");

        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "bad work");

        lc = _marketplace.getTaskLifecycle(taskId);
        require(lc.disputedBy == BUYER, "disputedBy not recorded");
        require(
            keccak256(bytes(lc.disputeReason)) == keccak256(bytes("bad work")),
            "dispute reason mismatch"
        );
        require(!lc.resolved, "resolved too early");

        _marketplace.resolveDispute(taskId, DEFAULT_ESCROW);

        lc = _marketplace.getTaskLifecycle(taskId);
        require(lc.resolved, "not resolved after resolveDispute");
        require(lc.resolvedBy == address(this), "resolvedBy mismatch");
        require(lc.grossProviderAmount == DEFAULT_ESCROW, "grossProviderAmount mismatch");
    }

    // ── optimistic settlement lifecycle ───────────────────────────────────────

    function testSettleUndisputedTaskBeforeDeadlineReverts() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "early-settle-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        // Exactly at deadline — still inside window
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.ChallengePeriodActive.selector,
                taskId,
                block.timestamp + CHALLENGE_WINDOW,
                block.timestamp
            )
        );
        _marketplace.settleUndisputedTask(taskId);
    }

    function testSettleUndisputedTaskAfterDeadlineSucceeds() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "settle-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        uint256 deadline = _marketplace.getTaskLifecycle(taskId).challengeDeadline;
        vm.warp(deadline + 1);

        uint256 expectedPayout = _providerPayout(DEFAULT_ESCROW);
        uint256 expectedFee = _platformFee(DEFAULT_ESCROW);
        vm.expectEmit(true, false, false, true);
        emit TaskVerified(taskId, expectedPayout);
        vm.expectEmit(true, true, false, true);
        emit TaskUndisputedSettled(taskId, PROVIDER, expectedPayout, expectedFee);
        _marketplace.settleUndisputedTask(taskId);

        require(
            _escrowManager.getPendingProviderPayout(PROVIDER) == expectedPayout,
            "provider payout missing after settle"
        );
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "stake not unlocked after settle");
        require(
            _marketplace.getTask(taskId).status == MarketplaceTypes.TaskStatus.Verified,
            "task not verified after settle"
        );
    }

    function testSettleUndisputedTaskWithNoChallengeDeadlineReverts() public {
        // Task in Completed status without challengeDeadline (shouldn't occur normally; guard test)
        bytes32 taskId = _createTask(BUYER, 2 ether, "no-deadline-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);

        // Force via acceptTask + submitResult sets deadline, so use a raw Completed task bypassing submitResult is
        // not possible through the public API — verify that getTaskLifecycle returns 0 for Open task
        MarketplaceTypes.TaskLifecycle memory lc = _marketplace.getTaskLifecycle(taskId);
        require(lc.challengeDeadline == 0, "deadline should be zero for un-submitted task");

        // Attempting settle on non-Completed task reverts with status error
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.InvalidTaskStatusTransition.selector,
                MarketplaceTypes.TaskStatus.Open,
                MarketplaceTypes.TaskStatus.Verified
            )
        );
        _marketplace.settleUndisputedTask(taskId);
    }

    function testBuyerEarlyApprovalSettlesBeforeDeadline() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "early-approve-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        // Deadline not reached but buyer approves early
        uint256 expectedPayout = _providerPayout(DEFAULT_ESCROW);
        vm.prank(BUYER);
        _marketplace.approveResult(taskId);

        require(
            _escrowManager.getPendingProviderPayout(PROVIDER) == expectedPayout,
            "early approval payout mismatch"
        );
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "stake not cleared on early approval");
    }

    // ── dispute semantics ─────────────────────────────────────────────────────

    function testProviderCannotDisputeCompletedResult() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "provider-dispute-completed-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        // Provider is not the buyer, so disputing a Completed task is rejected
        vm.prank(PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(TaskMarketplace.NotTaskBuyer.selector, taskId, PROVIDER, BUYER)
        );
        _marketplace.disputeTask(taskId, "self dispute");
    }

    function testBuyerCanDisputeCompletedWithinDeadline() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "buyer-dispute-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        // Within challenge window — buyer can dispute
        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "quality issue");

        require(
            _marketplace.getTask(taskId).status == MarketplaceTypes.TaskStatus.Disputed,
            "task not disputed"
        );
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "stake must stay locked after dispute");
    }

    function testBuyerCannotDisputeAfterDeadline() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "late-dispute-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        uint256 deadline = _marketplace.getTaskLifecycle(taskId).challengeDeadline;
        vm.warp(deadline + 1);

        vm.prank(BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(TaskMarketplace.ChallengePeriodExpired.selector, taskId, deadline)
        );
        _marketplace.disputeTask(taskId, "too late");
    }

    function testProviderCanDisputeAssignedTask() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "provider-dispute-assigned-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);

        vm.prank(PROVIDER);
        _marketplace.acceptTask(taskId, _providerNodeId);
        // Task is now Assigned; provider can dispute
        vm.prank(PROVIDER);
        _marketplace.disputeTask(taskId, "resource unavailable");

        require(
            _marketplace.getTask(taskId).status == MarketplaceTypes.TaskStatus.Disputed,
            "task not disputed by provider"
        );
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "stake released too early on provider dispute");
    }

    function testDisputedTaskCannotBeSettledOrApprovedOrCancelled() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "disputed-block-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "bad result");

        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.InvalidTaskStatusTransition.selector,
                MarketplaceTypes.TaskStatus.Disputed,
                MarketplaceTypes.TaskStatus.Verified
            )
        );
        _marketplace.settleUndisputedTask(taskId);

        vm.prank(BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.InvalidTaskStatusTransition.selector,
                MarketplaceTypes.TaskStatus.Disputed,
                MarketplaceTypes.TaskStatus.Verified
            )
        );
        _marketplace.approveResult(taskId);

        vm.prank(BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.InvalidTaskStatusTransition.selector,
                MarketplaceTypes.TaskStatus.Disputed,
                MarketplaceTypes.TaskStatus.Cancelled
            )
        );
        _marketplace.cancelTask(taskId);
    }

    function testCompletedTaskCannotBeCancelled() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "completed-no-cancel-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        vm.prank(BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.InvalidTaskStatusTransition.selector,
                MarketplaceTypes.TaskStatus.Completed,
                MarketplaceTypes.TaskStatus.Cancelled
            )
        );
        _marketplace.cancelTask(taskId);
    }

    // ── dispute resolution: full refund / full release / partial split ─────────

    function testResolveDisputeFullRefundTooBuyer() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "full-refund-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "fraud");

        vm.expectEmit(true, true, false, true);
        emit DisputeResolved(taskId, address(this), 0, 0, DEFAULT_ESCROW, 0);
        _marketplace.resolveDispute(taskId, 0);

        require(
            _escrowManager.getPendingBuyerRefund(BUYER) == DEFAULT_ESCROW,
            "buyer should receive full escrow on 0 gross"
        );
        require(
            _escrowManager.getPendingProviderPayout(PROVIDER) == 0,
            "provider should receive nothing on full refund"
        );
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "stake not unlocked on full refund resolution");
        require(
            _marketplace.getTask(taskId).status == MarketplaceTypes.TaskStatus.Verified,
            "task should be Verified after resolution"
        );
        MarketplaceTypes.EscrowDeposit memory escrow = _escrowManager.getEscrow(taskId);
        require(
            escrow.settlement == MarketplaceTypes.EscrowSettlementKind.Refunded,
            "escrow settlement kind mismatch"
        );
    }

    function testResolveDisputeFullReleaseToBuyer() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "full-release-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "re-examine needed");

        uint256 expectedPayout = _providerPayout(DEFAULT_ESCROW);
        uint256 expectedFee = _platformFee(DEFAULT_ESCROW);
        vm.expectEmit(true, true, false, true);
        emit DisputeResolved(taskId, address(this), DEFAULT_ESCROW, expectedPayout, 0, expectedFee);
        _marketplace.resolveDispute(taskId, DEFAULT_ESCROW);

        require(
            _escrowManager.getPendingProviderPayout(PROVIDER) == expectedPayout,
            "provider payout mismatch on full release"
        );
        require(
            _escrowManager.getPendingTreasuryFees(TREASURY) == expectedFee,
            "treasury fee mismatch on full release"
        );
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "stake not unlocked on full release resolution");
    }

    function testResolveDisputePartialSplitWithProviderFeeAndBuyerRefund() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "split-dispute-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW); // 1 ether
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "partial quality");

        // Resolve with 0.6 ether gross for provider; 0.4 ether back to buyer
        uint256 grossProviderAmount = 0.6 ether;

        uint256 expectedFee = _platformFee(grossProviderAmount);           // fee on gross
        uint256 expectedNetProvider = grossProviderAmount - expectedFee;    // net to provider
        uint256 expectedBuyerRefund = DEFAULT_ESCROW - grossProviderAmount; // remainder to buyer

        vm.expectEmit(true, true, false, true);
        emit DisputeResolved(taskId, address(this), grossProviderAmount, expectedNetProvider, expectedBuyerRefund, expectedFee);
        _marketplace.resolveDispute(taskId, grossProviderAmount);

        require(
            _escrowManager.getPendingProviderPayout(PROVIDER) == expectedNetProvider,
            "net provider payout mismatch on partial split"
        );
        require(
            _escrowManager.getPendingBuyerRefund(BUYER) == expectedBuyerRefund,
            "buyer refund must not go to treasury on partial split"
        );
        require(
            _escrowManager.getPendingTreasuryFees(TREASURY) == expectedFee,
            "treasury fee mismatch on partial split"
        );
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "stake not unlocked on partial resolution");
        MarketplaceTypes.EscrowDeposit memory escrow = _escrowManager.getEscrow(taskId);
        require(
            escrow.settlement == MarketplaceTypes.EscrowSettlementKind.Split,
            "escrow settlement kind should be Split"
        );
        require(escrow.providerPayout == expectedNetProvider, "escrow.providerPayout not truthful");
        require(escrow.buyerRefund == expectedBuyerRefund, "escrow.buyerRefund not truthful");
        require(escrow.platformFee == expectedFee, "escrow.platformFee not truthful");
    }

    function testResolveDisputeDoubleCallReverts() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "double-resolve-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);
        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "bad");
        _marketplace.resolveDispute(taskId, 0);

        vm.expectRevert(abi.encodeWithSelector(TaskMarketplace.DisputeAlreadyResolved.selector, taskId));
        _marketplace.resolveDispute(taskId, 0);
    }

    function testResolveDisputeGrossExceedsEscrowReverts() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "gross-exceed-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);
        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "bad");

        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.GrossAmountExceedsEscrow.selector,
                taskId,
                DEFAULT_ESCROW + 1,
                DEFAULT_ESCROW
            )
        );
        _marketplace.resolveDispute(taskId, DEFAULT_ESCROW + 1);
    }

    function testNonOwnerCannotResolveDispute() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "non-owner-resolve-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);
        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "bad");

        vm.prank(BUYER);
        vm.expectRevert();
        _marketplace.resolveDispute(taskId, 0);
    }

    // ── stake invariants across all paths ────────────────────────────────────

    function testStakeUnlocksOnCancel() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "cancel-unlock-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        vm.prank(PROVIDER);
        _marketplace.acceptTask(taskId, _providerNodeId);
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "no lock on accept");

        vm.prank(BUYER);
        _marketplace.cancelTask(taskId);
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "stake not unlocked on cancel");
    }

    function testStakeUnlocksOnApprove() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "approve-unlock-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "no lock after submit");

        vm.prank(BUYER);
        _marketplace.approveResult(taskId);
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "stake not unlocked on approve");
    }

    function testStakeUnlocksOnUndisputedSettle() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "settle-unlock-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        uint256 deadline = _marketplace.getTaskLifecycle(taskId).challengeDeadline;
        vm.warp(deadline + 1);
        _marketplace.settleUndisputedTask(taskId);

        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 0, "stake not unlocked on settle");
    }

    function testStakeDoesNotUnlockOnSubmitResult() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "submit-no-unlock-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        vm.prank(PROVIDER);
        _marketplace.acceptTask(taskId, _providerNodeId);
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "no lock on accept");

        vm.prank(PROVIDER);
        _marketplace.submitResult(taskId, keccak256("r"), "uri");
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "stake must stay locked after submitResult");
    }

    function testStakeDoesNotUnlockOnDisputeOpen() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "dispute-no-unlock-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "no lock after submit");

        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "bad");
        require(_nodeRegistry.getPendingTaskCount(_providerNodeId) == 1, "stake must stay locked after dispute");
    }

    // ── node eligibility: acceptTask guards ───────────────────────────────────

    function testAcceptTaskByInactiveNodeReverts() public {
        bytes32 inactiveNodeId = _registerNode(PROVIDER, "inactive-eligibility-node");
        bytes32 taskId = _createTask(BUYER, 2 ether, "inactive-eligibility-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);

        vm.prank(PROVIDER);
        vm.expectRevert(abi.encodeWithSelector(TaskMarketplace.NodeInactive.selector, inactiveNodeId));
        _marketplace.acceptTask(taskId, inactiveNodeId);
    }

    function testAcceptTaskByWrongOwnerReverts() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "wrong-owner-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);

        vm.prank(ALT_BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.NotAssignedNodeOwner.selector,
                taskId,
                _providerNodeId,
                ALT_BUYER,
                PROVIDER
            )
        );
        _marketplace.acceptTask(taskId, _providerNodeId);
    }

    function testAcceptUnfundedTaskReverts() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "unfunded-accept-task");
        // Not funded

        vm.prank(PROVIDER);
        vm.expectRevert(abi.encodeWithSelector(TaskMarketplace.TaskEscrowNotFunded.selector, taskId));
        _marketplace.acceptTask(taskId, _providerNodeId);
    }

    function testAcceptNonOpenTaskReverts() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "non-open-accept-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);

        vm.prank(PROVIDER);
        _marketplace.acceptTask(taskId, _providerNodeId);

        // Task is now Assigned — second accept must fail
        vm.prank(ALT_PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.InvalidTaskStatusTransition.selector,
                MarketplaceTypes.TaskStatus.Assigned,
                MarketplaceTypes.TaskStatus.Open
            )
        );
        _marketplace.acceptTask(taskId, _altProviderNodeId);
    }

    // ── pause semantics with new functions ────────────────────────────────────

    function testMarketplacePauseBlocksSettleAndApproveButAllowsDisputeAndResolve() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "pause-settle-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        _marketplace.pause();

        // settleUndisputedTask blocked while marketplace paused
        uint256 deadline = _marketplace.getTaskLifecycle(taskId).challengeDeadline;
        vm.warp(deadline + 1);
        vm.expectRevert();
        _marketplace.settleUndisputedTask(taskId);

        // approveResult blocked while paused
        vm.prank(BUYER);
        vm.expectRevert();
        _marketplace.approveResult(taskId);

        // dispute is still allowed (exit path)
        _marketplace.unpause();

        // Re-submit for a clean Completed task after unpause
        bytes32 taskId2 = _createTask(BUYER, 2 ether, "pause-dispute-path-task");
        _fundTask(taskId2, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId2, PROVIDER, _providerNodeId);

        _marketplace.pause();

        vm.prank(BUYER);
        _marketplace.disputeTask(taskId2, "exit path");

        // resolveDispute is also allowed as exit path
        _marketplace.resolveDispute(taskId2, 0);

        _marketplace.unpause();
    }

    function testEscrowPauseBlocksProviderPayingResolutionButAllowsFullRefund() public {
        bytes32 taskId = _createTask(BUYER, 2 ether, "escrow-pause-resolve-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);
        vm.prank(BUYER);
        _marketplace.disputeTask(taskId, "bad");

        _escrowManager.pause();

        // Full refund (escrow.refund has no whenNotPaused) — should succeed
        _marketplace.resolveDispute(taskId, 0);
        require(
            _escrowManager.getPendingBuyerRefund(BUYER) == DEFAULT_ESCROW,
            "full refund should work when escrow paused"
        );

        _escrowManager.unpause();

        // Now test that provider-paying path is blocked when escrow paused
        bytes32 taskId2 = _createTask(BUYER, 2 ether, "escrow-pause-release-task");
        _fundTask(taskId2, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId2, PROVIDER, _providerNodeId);

        // Use altProviderNode to avoid pending task count conflict
        bytes32 taskId3 = _createTask(BUYER, 2 ether, "escrow-pause-split-task");
        _fundTask(taskId3, BUYER, DEFAULT_ESCROW);
        vm.prank(ALT_PROVIDER);
        _marketplace.acceptTask(taskId3, _altProviderNodeId);
        vm.prank(ALT_PROVIDER);
        _marketplace.submitResult(taskId3, keccak256("r3"), "uri3");
        vm.prank(BUYER);
        _marketplace.disputeTask(taskId3, "bad");

        _escrowManager.pause();

        vm.expectRevert();
        _marketplace.resolveDispute(taskId3, DEFAULT_ESCROW);

        // Verify no state was mutated by the reverted provider-paying resolution
        MarketplaceTypes.TaskLifecycle memory lc3 = _marketplace.getTaskLifecycle(taskId3);
        require(!lc3.resolved, "resolved must stay false after reverted provider-paying resolution");
        require(
            _nodeRegistry.getPendingTaskCount(_altProviderNodeId) == 1,
            "pendingTaskCount must stay at 1 after reverting resolution"
        );
        require(
            _marketplace.getTask(taskId3).status == MarketplaceTypes.TaskStatus.Disputed,
            "task status must remain Disputed after reverting resolution"
        );
        MarketplaceTypes.EscrowDeposit memory escrow3 = _escrowManager.getEscrow(taskId3);
        require(
            escrow3.settlement == MarketplaceTypes.EscrowSettlementKind.None,
            "escrow settlement must remain None after reverting resolution"
        );
        require(
            _escrowManager.getPendingProviderPayout(ALT_PROVIDER) == 0,
            "provider payout must not change after reverting resolution"
        );
        require(
            _escrowManager.getPendingTreasuryFees(TREASURY) == 0,
            "treasury fees must not change after reverting resolution"
        );

        _escrowManager.unpause();
    }

    // ── challenge window configuration ────────────────────────────────────────

    function testSetChallengeWindowRejectsZero() public {
        vm.expectRevert(abi.encodeWithSelector(TaskMarketplace.ZeroChallengeWindowNotAllowed.selector));
        _marketplace.setChallengeWindow(0);
    }

    function testSetChallengeWindowEmitsEvent() public {
        uint256 previousWindow = _marketplace.challengeWindow();
        uint256 newWindow = 2 hours;

        vm.expectEmit(false, false, false, true);
        emit ChallengeWindowUpdated(previousWindow, newWindow);
        _marketplace.setChallengeWindow(newWindow);

        require(_marketplace.challengeWindow() == newWindow, "challengeWindow not updated");
    }

    function testSubmitResultUsesConfiguredChallengeWindow() public {
        uint256 customWindow = 2 hours;
        _marketplace.setChallengeWindow(customWindow);

        bytes32 taskId = _createTask(BUYER, 2 ether, "custom-window-task");
        _fundTask(taskId, BUYER, DEFAULT_ESCROW);
        _acceptAndSubmit(taskId, PROVIDER, _providerNodeId);

        MarketplaceTypes.TaskLifecycle memory lc = _marketplace.getTaskLifecycle(taskId);
        require(
            lc.challengeDeadline == block.timestamp + customWindow,
            "deadline should reflect configured challenge window"
        );
    }
}
