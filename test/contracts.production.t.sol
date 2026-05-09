// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// ─────────────────────────────────────────────────────────────────────────────
// Production Smoke Tests — contracts.production.t.sol
//
// Covers the full MVP lifecycle with PRODUCTION contracts (no harnesses):
//   registerNode -> PoW activate -> createTask -> fundEscrow -> acceptTask
//   -> submitResult -> approve/settle -> release/refund/claim
//
// Also verifies pricing boundaries and PoW access control.
// ─────────────────────────────────────────────────────────────────────────────

import {EscrowManager} from "../contracts/EscrowManager.sol";
import {NodeRegistry} from "../contracts/NodeRegistry.sol";
import {ProofOfWorkVerifier} from "../contracts/ProofOfWorkVerifier.sol";
import {TaskMarketplace} from "../contracts/TaskMarketplace.sol";
import {IEscrowManager} from "../contracts/interfaces/IEscrowManager.sol";
import {INodeRegistry} from "../contracts/interfaces/INodeRegistry.sol";
import {MarketplaceTypes} from "../contracts/types/MarketplaceTypes.sol";

// ─── Cheatcode interface (same pattern as skeleton tests) ────────────────────
interface Vm {
    function deal(address account, uint256 newBalance) external;
    function prank(address sender) external;
    function expectRevert() external;
    function expectRevert(bytes calldata revertData) external;
    function expectEmit(bool checkTopic1, bool checkTopic2, bool checkTopic3, bool checkData) external;
    function warp(uint256 newTimestamp) external;
}

// ─── Shared base ─────────────────────────────────────────────────────────────
contract ProductionTestBase {
    Vm internal constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    address internal constant DEPLOYER  = address(0xDE10);
    address internal constant TREASURY  = address(0xFEE1);
    address internal constant BUYER     = address(0xB0B0);
    address internal constant PROVIDER  = address(0xA11C);
    address internal constant BYSTANDER = address(0xBAAD);

    // Registration fee (0.1 ETH) + minimum stake (0.5 ETH) = 0.6 ETH
    uint256 internal constant REG_VALUE = 0.6 ether;

    // GPU pricing at creation-time defaults:
    //   startFee=0.0002, envHour=0.0008, computePowerHour=0.00008
    //   power=100, duration=1h  =>  0.0002 + 0.0008 + 0.00008*100 = 0.009 ETH
    uint256 internal constant GPU_QUOTE_MIN_1H_P100 = 0.009 ether;

    NodeRegistry       internal nodeRegistry;
    ProofOfWorkVerifier internal powVerifier;
    EscrowManager      internal escrowManager;
    TaskMarketplace    internal marketplace;

    bytes32 internal providerNodeId;

    // Events redeclared for vm.expectEmit
    event NodeStatusChanged(
        bytes32 indexed nodeId,
        MarketplaceTypes.NodeStatus oldStatus,
        MarketplaceTypes.NodeStatus newStatus
    );
    event ChallengeIssued(bytes32 indexed challengeId, bytes32 indexed nodeId, uint256 difficulty, uint256 deadline);
    event ChallengeSolved(bytes32 indexed challengeId, bytes32 indexed nodeId, uint256 solutionTime, uint256 verifiedPower);
    event ChallengeFailed(bytes32 indexed challengeId, bytes32 indexed nodeId, string reason);
    event TaskCreated(bytes32 indexed taskId, address indexed buyer, MarketplaceTypes.ResourceType resourceType, uint256 maxPrice);
    event TaskAssigned(bytes32 indexed taskId, bytes32 indexed nodeId, uint256 startTime);
    event TaskCompleted(bytes32 indexed taskId, bytes32 resultHash);
    event TaskVerified(bytes32 indexed taskId, uint256 payoutAmount);
    event TaskUndisputedSettled(bytes32 indexed taskId, address indexed provider, uint256 providerPayout, uint256 platformFee);
    event TaskCancelled(bytes32 indexed taskId, uint256 refundAmount);
    event PricingUpdated(MarketplaceTypes.ResourceType indexed resourceType, uint256 envHourFee, uint256 computePowerHourFee);
    event StartFeeUpdated(uint256 previousFee, uint256 newFee);

    function _deployAll() internal {
        vm.deal(DEPLOYER,  100 ether);
        vm.deal(BUYER,     100 ether);
        vm.deal(PROVIDER,  100 ether);
        vm.deal(BYSTANDER, 10 ether);
        vm.deal(TREASURY,  1 ether);

        // Deploy contracts
        vm.prank(DEPLOYER);
        nodeRegistry = new NodeRegistry(DEPLOYER, TREASURY, 0);

        vm.prank(DEPLOYER);
        powVerifier = new ProofOfWorkVerifier(DEPLOYER, address(nodeRegistry));

        vm.prank(DEPLOYER);
        escrowManager = new EscrowManager(DEPLOYER, TREASURY, 0);

        vm.prank(DEPLOYER);
        marketplace = new TaskMarketplace(DEPLOYER);

        // Wire references
        vm.prank(DEPLOYER);
        nodeRegistry.setTaskMarketplace(address(marketplace));

        vm.prank(DEPLOYER);
        escrowManager.setTaskMarketplace(address(marketplace));

        vm.prank(DEPLOYER);
        marketplace.setNodeRegistry(address(nodeRegistry));

        vm.prank(DEPLOYER);
        marketplace.setEscrowManager(address(escrowManager));

        vm.prank(DEPLOYER);
        marketplace.setPowVerifier(address(powVerifier));

        // Grant UPDATER_ROLE to powVerifier so it can update node state on PoW success
        // Read the role constant before pranking to avoid consuming the prank.
        bytes32 updaterRole = nodeRegistry.UPDATER_ROLE();
        vm.prank(DEPLOYER);
        nodeRegistry.grantRole(updaterRole, address(powVerifier));
    }

    function _activateNode(address nodeOwner, bytes32 nodeId) internal {
        // Issue challenge as node owner
        vm.prank(nodeOwner);
        bytes32 challengeId = powVerifier.issueChallenge(nodeId);

        // Brute-force a nonce that satisfies the PoW target.
        // difficulty = 2^16; target = type(uint256).max / 2^16 ≈ 2^240.
        // Foundry's in-process EVM is fast — this loop typically exits in < 65536 iterations.
        MarketplaceTypes.Challenge memory ch = powVerifier.getChallenge(challengeId);
        uint256 nonce = 0;
        uint256 target = type(uint256).max / ch.difficulty;
        while (uint256(keccak256(abi.encodePacked(ch.seed, nonce))) > target) {
            nonce++;
        }

        vm.prank(nodeOwner);
        bool passed = powVerifier.submitSolution(challengeId, nonce);
        require(passed, "PoW: brute-forced nonce should pass");
    }

    function _createAndFundTask(
        address buyer,
        uint256 maxPrice,
        uint256 fundAmount
    ) internal returns (bytes32 taskId) {
        vm.prank(buyer);
        taskId = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,       // requiredPower
            1 hours,   // duration
            maxPrice,
            1,         // minTrustLevel
            "ipfs://spec"
        );
        vm.prank(buyer);
        marketplace.fundTaskEscrow{value: fundAmount}(taskId);
    }

    function _platformFee(uint256 amount) internal pure returns (uint256) {
        return (amount * 800) / 10_000;
    }

    function _providerPayout(uint256 amount) internal pure returns (uint256) {
        return amount - _platformFee(amount);
    }

    receive() external payable {}
}

// ─────────────────────────────────────────────────────────────────────────────
// Deploy smoke test
// ─────────────────────────────────────────────────────────────────────────────
contract ProductionDeploySmokeTest is ProductionTestBase {
    function setUp() public {
        _deployAll();
    }

    function testDeploymentWiresReferencesCorrectly() public {
        // NodeRegistry has TASK_MANAGER_ROLE granted to marketplace
        require(
            nodeRegistry.hasRole(nodeRegistry.TASK_MANAGER_ROLE(), address(marketplace)),
            "marketplace missing TASK_MANAGER_ROLE on nodeRegistry"
        );

        // EscrowManager has RELEASER_ROLE granted to marketplace
        require(
            escrowManager.hasRole(escrowManager.RELEASER_ROLE(), address(marketplace)),
            "marketplace missing RELEASER_ROLE on escrowManager"
        );

        // NodeRegistry has UPDATER_ROLE granted to powVerifier
        require(
            nodeRegistry.hasRole(nodeRegistry.UPDATER_ROLE(), address(powVerifier)),
            "powVerifier missing UPDATER_ROLE on nodeRegistry"
        );

        // Owners/admins are DEPLOYER
        require(marketplace.owner() == DEPLOYER, "marketplace owner mismatch");
        require(powVerifier.owner() == DEPLOYER, "powVerifier owner mismatch");
    }

    function testDefaultPricingMatchesSpec() public {
        // GPU defaults from compute-pricing.md:
        //   startFee = 0.0002 ETH
        //   envHourFee(GPU) = 0.0008 ETH
        //   computePowerHourFee(GPU) = 0.00008 ETH
        require(marketplace.startFee() == 0.0002 ether, "startFee mismatch");
        require(
            marketplace.envHourFee(MarketplaceTypes.ResourceType.GPU) == 0.0008 ether,
            "GPU envHourFee mismatch"
        );
        require(
            marketplace.computePowerHourFee(MarketplaceTypes.ResourceType.GPU) == 0.00008 ether,
            "GPU computePowerHourFee mismatch"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PoW activation tests
// ─────────────────────────────────────────────────────────────────────────────
contract PoWActivationTest is ProductionTestBase {
    function setUp() public {
        _deployAll();

        vm.prank(PROVIDER);
        providerNodeId = nodeRegistry.registerNode{value: REG_VALUE}(
            MarketplaceTypes.ResourceType.GPU,
            "gpu-node-v1"
        );
    }

    function testRegisteredNodeStartsInPendingState() public {
        MarketplaceTypes.Node memory node = nodeRegistry.getNode(providerNodeId);
        require(node.status == MarketplaceTypes.NodeStatus.Pending, "new node should be Pending");
        require(node.computePower == 0, "new node computePower should be 0");
    }

    function testIssueChallengeOnlyNodeOwner() public {
        // Bystander cannot issue a challenge for PROVIDER's node
        vm.prank(BYSTANDER);
        vm.expectRevert(
            abi.encodeWithSelector(ProofOfWorkVerifier.NotNodeOwner.selector, providerNodeId, BYSTANDER)
        );
        powVerifier.issueChallenge(providerNodeId);

        // Node owner can issue
        vm.prank(PROVIDER);
        bytes32 challengeId = powVerifier.issueChallenge(providerNodeId);
        require(challengeId != bytes32(0), "challengeId should not be zero");
    }

    function testSuccessfulPoWActivatesNode() public {
        vm.prank(PROVIDER);
        bytes32 challengeId = powVerifier.issueChallenge(providerNodeId);

        MarketplaceTypes.Challenge memory ch = powVerifier.getChallenge(challengeId);
        require(ch.nodeId == providerNodeId, "challenge nodeId mismatch");
        require(!ch.completed, "challenge should start open");

        // Brute-force a valid nonce
        uint256 nonce = 0;
        uint256 target = type(uint256).max / ch.difficulty;
        while (uint256(keccak256(abi.encodePacked(ch.seed, nonce))) > target) {
            nonce++;
        }

        vm.prank(PROVIDER);
        bool passed = powVerifier.submitSolution(challengeId, nonce);
        require(passed, "solution should pass");

        // Challenge is now completed
        MarketplaceTypes.Challenge memory completed = powVerifier.getChallenge(challengeId);
        require(completed.completed, "challenge should be marked completed");

        // Node is now Active
        MarketplaceTypes.Node memory node = nodeRegistry.getNode(providerNodeId);
        require(node.status == MarketplaceTypes.NodeStatus.Active, "node should be Active after PoW");
        require(node.computePower > 0, "computePower should be set after PoW");
        require(node.reputation >= 100, "reputation should increase after PoW");

        // Verification history is recorded
        MarketplaceTypes.VerificationResult[] memory history = powVerifier.getVerificationHistory(providerNodeId);
        require(history.length == 1, "verification history length should be 1");
        require(history[0].passed, "history result should be passed");
    }

    function testExpiredChallengeRevertsOnSubmit() public {
        vm.prank(PROVIDER);
        bytes32 challengeId = powVerifier.issueChallenge(providerNodeId);

        MarketplaceTypes.Challenge memory ch = powVerifier.getChallenge(challengeId);

        // Warp past the deadline
        vm.warp(ch.deadline + 1);

        vm.expectRevert(
            abi.encodeWithSelector(ProofOfWorkVerifier.ChallengeExpired.selector, challengeId)
        );
        vm.prank(PROVIDER);
        powVerifier.submitSolution(challengeId, 0);
    }

    function testBadNonceReturnsFalseAndEmitsChallengeFailed() public {
        vm.prank(PROVIDER);
        bytes32 challengeId = powVerifier.issueChallenge(providerNodeId);

        MarketplaceTypes.Challenge memory ch = powVerifier.getChallenge(challengeId);
        uint256 target = type(uint256).max / ch.difficulty;

        // Find a nonce that FAILS (hash > target).
        uint256 badNonce = 0;
        while (uint256(keccak256(abi.encodePacked(ch.seed, badNonce))) <= target) {
            badNonce++;
        }

        vm.expectEmit(true, true, false, false);
        emit ChallengeFailed(challengeId, providerNodeId, "");

        vm.prank(PROVIDER);
        bool passed = powVerifier.submitSolution(challengeId, badNonce);
        require(!passed, "bad nonce should return false");

        // Node stays Pending after a failed attempt
        MarketplaceTypes.Node memory node = nodeRegistry.getNode(providerNodeId);
        require(node.status == MarketplaceTypes.NodeStatus.Pending, "node should still be Pending after failed challenge");
    }

    function testReVerifyActiveNodeOnlyUpdatesMetricsNoStateChange() public {
        // First: activate the node
        _activateNode(PROVIDER, providerNodeId);

        MarketplaceTypes.Node memory afterFirst = nodeRegistry.getNode(providerNodeId);
        require(afterFirst.status == MarketplaceTypes.NodeStatus.Active, "should be Active after first PoW");

        // Second PoW on an already-Active node
        vm.prank(PROVIDER);
        bytes32 challengeId2 = powVerifier.issueChallenge(providerNodeId);
        MarketplaceTypes.Challenge memory ch2 = powVerifier.getChallenge(challengeId2);
        uint256 nonce = 0;
        uint256 target = type(uint256).max / ch2.difficulty;
        while (uint256(keccak256(abi.encodePacked(ch2.seed, nonce))) > target) {
            nonce++;
        }
        vm.prank(PROVIDER);
        bool passed2 = powVerifier.submitSolution(challengeId2, nonce);
        require(passed2, "second PoW solution should pass");

        // Status must remain Active — no illegal Active→Verified transition
        MarketplaceTypes.Node memory afterSecond = nodeRegistry.getNode(providerNodeId);
        require(afterSecond.status == MarketplaceTypes.NodeStatus.Active, "node must remain Active after re-verify");

        // Verification history now has two entries
        MarketplaceTypes.VerificationResult[] memory history = powVerifier.getVerificationHistory(providerNodeId);
        require(history.length == 2, "should have 2 verification entries");
        require(history[1].passed, "second verification should be passed");

        // computePower was updated (may differ from first due to different solutionTime)
        // Just assert it is still positive
        require(afterSecond.computePower > 0, "computePower should remain positive after re-verify");
    }

    function testCannotAcceptTaskWithPendingNode() public {
        // Create and fund a task
        bytes32 taskId = _createAndFundTask(BUYER, 5 ether, 1 ether);

        // Node is still Pending — acceptTask should revert
        vm.prank(PROVIDER);
        vm.expectRevert(
            abi.encodeWithSelector(TaskMarketplace.NodeInactive.selector, providerNodeId)
        );
        marketplace.acceptTask(taskId, providerNodeId);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pricing boundary tests
// ─────────────────────────────────────────────────────────────────────────────
contract PricingBoundaryTest is ProductionTestBase {
    function setUp() public {
        _deployAll();
    }

    function testEstimateTaskCostMatchesPricingFormula() public {
        // GPU, power=100, duration=1h
        // = 0.0002 + 0.0008*1 + 0.00008*100*1 = 0.009 ETH
        uint256 estimate = marketplace.estimateTaskCost(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours
        );
        require(estimate == GPU_QUOTE_MIN_1H_P100, "estimate mismatch for GPU/p100/1h");
    }

    function testEstimateDurationNormalizesSubHour() public {
        // Duration < 1 hour → ceiled to 1 hour (same result as exact 1 hour)
        uint256 est30m = marketplace.estimateTaskCost(MarketplaceTypes.ResourceType.GPU, 100, 30 minutes);
        uint256 est1h  = marketplace.estimateTaskCost(MarketplaceTypes.ResourceType.GPU, 100, 1 hours);
        require(est30m == est1h, "sub-hour duration should be ceiled to 1 hour");
    }

    function testEstimateDurationCeilsToNextHour() public {
        // 1h30m should be ceiled to 2h, not floored to 1h
        uint256 est1h30m = marketplace.estimateTaskCost(MarketplaceTypes.ResourceType.GPU, 100, 90 minutes);
        uint256 est2h    = marketplace.estimateTaskCost(MarketplaceTypes.ResourceType.GPU, 100, 2 hours);
        uint256 est1h    = marketplace.estimateTaskCost(MarketplaceTypes.ResourceType.GPU, 100, 1 hours);
        require(est1h30m == est2h,  "1h30m should be priced as 2h (ceil)");
        require(est1h30m  > est1h,  "1h30m must cost more than 1h");
    }

    function testEstimateTwoHours() public {
        // GPU, power=100, duration=2h
        // = 0.0002 + 0.0008*2 + 0.00008*100*2 = 0.0002 + 0.0016 + 0.016 = 0.0178 ETH
        uint256 expected = 0.0002 ether + 0.0008 ether * 2 + 0.00008 ether * 100 * 2;
        uint256 estimate = marketplace.estimateTaskCost(
            MarketplaceTypes.ResourceType.GPU,
            100,
            2 hours
        );
        require(estimate == expected, "2-hour estimate mismatch");
    }

    function testFundingBelowQuoteMinReverts() public {
        // Create GPU task: power=100, duration=1h, maxPrice=5 ETH
        // quoteMin = 0.009 ETH, quoteCap = 5*1 = 5 ETH
        vm.prank(BUYER);
        bytes32 taskId = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            5 ether,
            1,
            "spec"
        );

        uint256 quoteMin = GPU_QUOTE_MIN_1H_P100;
        uint256 quoteCap = 5 ether;

        // Fund with 1 wei below quoteMin
        vm.prank(BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.TaskValueOutOfRange.selector,
                quoteMin - 1,
                quoteMin,
                quoteCap
            )
        );
        marketplace.fundTaskEscrow{value: quoteMin - 1}(taskId);
    }

    function testFundingAboveQuoteCapReverts() public {
        // maxPrice=0.01 ETH, duration=1h → quoteCap = 0.01 ETH
        // quoteMin = 0.009 ETH
        vm.prank(BUYER);
        bytes32 taskId = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            0.01 ether,
            1,
            "spec"
        );

        uint256 quoteMin = GPU_QUOTE_MIN_1H_P100;   // 0.009 ETH
        uint256 quoteCap = 0.01 ether;              // maxPrice * 1h = 0.01 ETH

        vm.prank(BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.TaskValueOutOfRange.selector,
                quoteCap + 1,
                quoteMin,
                quoteCap
            )
        );
        marketplace.fundTaskEscrow{value: quoteCap + 1}(taskId);
    }

    function testFundingAtExactQuoteMinSucceeds() public {
        vm.prank(BUYER);
        bytes32 taskId = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            5 ether,
            1,
            "spec"
        );

        uint256 quoteMin = GPU_QUOTE_MIN_1H_P100;
        vm.prank(BUYER);
        marketplace.fundTaskEscrow{value: quoteMin}(taskId);

        MarketplaceTypes.Task memory task = marketplace.getTask(taskId);
        require(task.escrowAmount == quoteMin, "escrow amount should equal quoteMin");
    }

    function testFundingQuoteSnapshotIsImmutableAcrossPricingUpdate() public {
        // Create task with old pricing
        vm.prank(BUYER);
        bytes32 taskId = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            5 ether,
            1,
            "spec"
        );
        uint256 oldQuoteMin = GPU_QUOTE_MIN_1H_P100;

        // Owner increases pricing dramatically
        vm.prank(DEPLOYER);
        marketplace.setResourcePricing(
            MarketplaceTypes.ResourceType.GPU,
            10 ether,    // new envHourFee — far higher
            1 ether      // new computePowerHourFee
        );

        // Task should still be fundable using the snapshot from creation time
        vm.prank(BUYER);
        marketplace.fundTaskEscrow{value: oldQuoteMin}(taskId);

        MarketplaceTypes.Task memory task = marketplace.getTask(taskId);
        require(task.escrowAmount == oldQuoteMin, "snapshot should be frozen at creation");
    }

    function testOwnerCanUpdateStartFeeWithEvent() public {
        uint256 oldFee = marketplace.startFee();

        vm.expectEmit(false, false, false, true);
        emit StartFeeUpdated(oldFee, 0.001 ether);

        vm.prank(DEPLOYER);
        marketplace.setStartFee(0.001 ether);

        require(marketplace.startFee() == 0.001 ether, "startFee not updated");
    }

    function testOwnerCanUpdateResourcePricingWithEvent() public {
        vm.expectEmit(true, false, false, true);
        emit PricingUpdated(MarketplaceTypes.ResourceType.CPU, 0.001 ether, 0.0001 ether);

        vm.prank(DEPLOYER);
        marketplace.setResourcePricing(MarketplaceTypes.ResourceType.CPU, 0.001 ether, 0.0001 ether);

        require(
            marketplace.envHourFee(MarketplaceTypes.ResourceType.CPU) == 0.001 ether,
            "CPU envHourFee not updated"
        );
    }

    function testNonOwnerCannotUpdatePricing() public {
        vm.prank(BYSTANDER);
        vm.expectRevert();
        marketplace.setStartFee(999 ether);
    }

    function testUnderfundableTaskCreationReverts() public {
        // Compute quoteMin for GPU/p100/1h = 0.009 ETH.
        // If maxPrice = 0.001 ETH then quoteCap = 0.001 * 1 = 0.001 ETH < quoteMin → revert.
        uint256 maxPrice = 0.001 ether;
        uint256 expectedQuoteMin = GPU_QUOTE_MIN_1H_P100; // 0.009 ETH
        uint256 expectedQuoteCap = maxPrice * 1;           // 0.001 ETH (1 duration-hour)

        vm.prank(BUYER);
        vm.expectRevert(
            abi.encodeWithSelector(
                TaskMarketplace.UnderfundableTaskCreation.selector,
                expectedQuoteMin,
                expectedQuoteCap
            )
        );
        marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            maxPrice,
            1,
            "underfundable-spec"
        );
    }}

// ─────────────────────────────────────────────────────────────────────────────
// Full lifecycle end-to-end test
// ─────────────────────────────────────────────────────────────────────────────
contract FullLifecycleTest is ProductionTestBase {
    function setUp() public {
        _deployAll();

        // Register and PoW-activate the provider node
        vm.prank(PROVIDER);
        providerNodeId = nodeRegistry.registerNode{value: REG_VALUE}(
            MarketplaceTypes.ResourceType.GPU,
            "gpu-node-prod"
        );
        _activateNode(PROVIDER, providerNodeId);
    }

    // ── Helper ─────────────────────────────────────────────────────────────

    function _fundedTaskId() internal returns (bytes32) {
        // maxPrice = 5 ETH (per hour), fund with 1 ETH (between quoteMin and quoteCap)
        return _createAndFundTask(BUYER, 5 ether, 1 ether);
    }

    // ── Tests ──────────────────────────────────────────────────────────────

    function testRegisterAndPoWActivateNodeEndToEnd() public {
        // Already done in setUp — just verify final state
        MarketplaceTypes.Node memory node = nodeRegistry.getNode(providerNodeId);
        require(node.status == MarketplaceTypes.NodeStatus.Active, "node should be Active");
        require(node.computePower > 0, "computePower should be positive");
    }

    function testCreateFundAndAcceptTask() public {
        bytes32 taskId = _fundedTaskId();

        // acceptTask as provider
        vm.prank(PROVIDER);
        marketplace.acceptTask(taskId, providerNodeId);

        MarketplaceTypes.Task memory task = marketplace.getTask(taskId);
        require(task.status == MarketplaceTypes.TaskStatus.Assigned, "task should be Assigned");
        require(task.assignedNode == providerNodeId, "assignedNode mismatch");
        require(nodeRegistry.getPendingTaskCount(providerNodeId) == 1, "stake lock missing");
    }

    function testSubmitAndApproveResult() public {
        bytes32 taskId = _fundedTaskId();

        vm.prank(PROVIDER);
        marketplace.acceptTask(taskId, providerNodeId);

        bytes32 resultHash = keccak256("result-data");
        vm.prank(PROVIDER);
        marketplace.submitResult(taskId, resultHash, "ipfs://result");

        MarketplaceTypes.Task memory task = marketplace.getTask(taskId);
        require(task.status == MarketplaceTypes.TaskStatus.Completed, "task should be Completed");

        uint256 escrowAmount = task.escrowAmount;
        uint256 expectedPayout = _providerPayout(escrowAmount);

        vm.prank(BUYER);
        marketplace.approveResult(taskId);

        task = marketplace.getTask(taskId);
        require(task.status == MarketplaceTypes.TaskStatus.Verified, "task should be Verified");
        require(nodeRegistry.getPendingTaskCount(providerNodeId) == 0, "stake not unlocked");

        uint256 pending = escrowManager.getPendingProviderPayout(PROVIDER);
        require(pending == expectedPayout, "provider payout mismatch on approve");
    }

    function testSubmitAndSettleAfterChallengeWindow() public {
        bytes32 taskId = _fundedTaskId();

        vm.prank(PROVIDER);
        marketplace.acceptTask(taskId, providerNodeId);
        vm.prank(PROVIDER);
        marketplace.submitResult(taskId, keccak256("result"), "ipfs://result");

        uint256 deadline = marketplace.getTaskLifecycle(taskId).challengeDeadline;
        vm.warp(deadline + 1);

        MarketplaceTypes.Task memory task = marketplace.getTask(taskId);
        uint256 expectedPayout = _providerPayout(task.escrowAmount);
        uint256 expectedFee    = _platformFee(task.escrowAmount);

        vm.expectEmit(true, false, false, true);
        emit TaskVerified(taskId, expectedPayout);
        vm.expectEmit(true, true, false, true);
        emit TaskUndisputedSettled(taskId, PROVIDER, expectedPayout, expectedFee);
        marketplace.settleUndisputedTask(taskId);

        require(
            escrowManager.getPendingProviderPayout(PROVIDER) == expectedPayout,
            "provider payout mismatch on settle"
        );
        require(
            escrowManager.getPendingTreasuryFees(TREASURY) == expectedFee,
            "treasury fee mismatch on settle"
        );
    }

    function testSubmitAndSettleBeforeChallengeWindowReverts() public {
        bytes32 taskId = _fundedTaskId();

        vm.prank(PROVIDER);
        marketplace.acceptTask(taskId, providerNodeId);
        vm.prank(PROVIDER);
        marketplace.submitResult(taskId, keccak256("result"), "ipfs://r");

        // Exactly at submission time — inside window
        vm.expectRevert();
        marketplace.settleUndisputedTask(taskId);
    }

    function testCancelOpenTaskRefundsEscrow() public {
        bytes32 taskId = _fundedTaskId();
        uint256 escrowAmount = marketplace.getTask(taskId).escrowAmount;

        uint256 buyerBefore = BUYER.balance;

        vm.prank(BUYER);
        marketplace.cancelTask(taskId);

        require(
            marketplace.getTask(taskId).status == MarketplaceTypes.TaskStatus.Cancelled,
            "task should be Cancelled"
        );

        // After claim
        vm.prank(BUYER);
        escrowManager.claimBuyerRefund();
        require(BUYER.balance == buyerBefore + escrowAmount, "buyer refund mismatch");
    }

    function testProviderPayoutAndTreasuryClaimFlow() public {
        bytes32 taskId = _fundedTaskId();
        uint256 escrowAmount = marketplace.getTask(taskId).escrowAmount;

        vm.prank(PROVIDER);
        marketplace.acceptTask(taskId, providerNodeId);
        vm.prank(PROVIDER);
        marketplace.submitResult(taskId, keccak256("r"), "uri");

        vm.prank(BUYER);
        marketplace.approveResult(taskId);

        uint256 expectedPayout = _providerPayout(escrowAmount);
        uint256 expectedFee    = _platformFee(escrowAmount);

        uint256 providerBefore  = PROVIDER.balance;
        uint256 treasuryBefore  = TREASURY.balance;

        vm.prank(PROVIDER);
        escrowManager.claimProviderPayout();
        vm.prank(TREASURY);
        escrowManager.claimTreasuryFees();

        require(PROVIDER.balance  == providerBefore + expectedPayout, "provider balance mismatch");
        require(TREASURY.balance  == treasuryBefore + expectedFee,    "treasury balance mismatch");
    }

    function testDisputeAndResolveWithFullProviderPayout() public {
        bytes32 taskId = _fundedTaskId();
        uint256 escrowAmount = marketplace.getTask(taskId).escrowAmount;

        vm.prank(PROVIDER);
        marketplace.acceptTask(taskId, providerNodeId);
        vm.prank(PROVIDER);
        marketplace.submitResult(taskId, keccak256("r"), "uri");

        vm.prank(BUYER);
        marketplace.disputeTask(taskId, "not good enough");

        // Admin resolves in provider's favour (full escrow to provider)
        vm.prank(DEPLOYER);
        marketplace.resolveDispute(taskId, escrowAmount);

        uint256 expectedPayout = _providerPayout(escrowAmount);
        require(
            escrowManager.getPendingProviderPayout(PROVIDER) == expectedPayout,
            "provider payout after dispute resolution mismatch"
        );
    }

    function testDisputeAndResolveWithFullRefundToBuyer() public {
        bytes32 taskId = _fundedTaskId();
        uint256 escrowAmount = marketplace.getTask(taskId).escrowAmount;

        vm.prank(PROVIDER);
        marketplace.acceptTask(taskId, providerNodeId);
        vm.prank(PROVIDER);
        marketplace.submitResult(taskId, keccak256("r"), "uri");

        vm.prank(BUYER);
        marketplace.disputeTask(taskId, "wrong output");

        // Admin resolves in buyer's favour (0 to provider)
        vm.prank(DEPLOYER);
        marketplace.resolveDispute(taskId, 0);

        require(
            escrowManager.getPendingBuyerRefund(BUYER) == escrowAmount,
            "buyer refund after full dispute resolution mismatch"
        );
    }

    function testOpenTasksIndexFilter() public {
        // Create two tasks; fund only one
        vm.prank(BUYER);
        bytes32 unfunded = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU, 100, 1 hours, 5 ether, 1, "unfunded"
        );

        bytes32 funded = _fundedTaskId();

        bytes32[] memory open = marketplace.getOpenTasks(MarketplaceTypes.ResourceType.GPU);
        require(open.length == 1, "only funded open tasks should be listed");
        require(open[0] == funded, "wrong task returned");
        require(unfunded != funded, "task ids should differ");
    }
}
