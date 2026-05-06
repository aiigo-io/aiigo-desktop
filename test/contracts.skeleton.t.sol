// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {EscrowManager} from "../contracts/EscrowManager.sol";
import {NodeRegistry} from "../contracts/NodeRegistry.sol";
import {TaskMarketplace} from "../contracts/TaskMarketplace.sol";
import {IEscrowManager} from "../contracts/interfaces/IEscrowManager.sol";
import {INodeRegistry} from "../contracts/interfaces/INodeRegistry.sol";
import {MarketplaceTypes} from "../contracts/types/MarketplaceTypes.sol";

interface Vm {
    function deal(address account, uint256 newBalance) external;
    function prank(address sender) external;
    function expectRevert() external;
    function expectRevert(bytes calldata revertData) external;
    function expectEmit(bool checkTopic1, bool checkTopic2, bool checkTopic3, bool checkData) external;
}

contract MockEscrowManager is IEscrowManager {
    error AmountMustBeGreaterThanZero();

    address private _taskMarketplace;
    address private _treasury;

    mapping(bytes32 => MarketplaceTypes.EscrowDeposit) private _escrows;

    constructor(address treasury_) {
        _treasury = treasury_;
    }

    function deposit(bytes32 taskId, address buyer) external payable returns (bytes32 escrowId) {
        if (msg.value == 0) {
            revert AmountMustBeGreaterThanZero();
        }

        _escrows[taskId] = MarketplaceTypes.EscrowDeposit({
            taskId: taskId,
            buyer: buyer,
            provider: address(0),
            treasuryRecipient: _treasury,
            amount: msg.value,
            platformFee: 0,
            providerPayout: 0,
            buyerRefund: 0,
            depositedAt: block.timestamp,
            settlement: MarketplaceTypes.EscrowSettlementKind.None,
            exists: true
        });

        emit EscrowDeposited(taskId, buyer, msg.value);
        return taskId;
    }

    function release(bytes32, address) external {}

    function refund(bytes32) external {}

    function splitPayment(bytes32, address, uint256) external {}

    function claimBuyerRefund() external {}

    function claimProviderPayout() external {}

    function claimTreasuryFees() external {}

    function setTaskMarketplace(address taskMarketplace_) external {
        _taskMarketplace = taskMarketplace_;
    }

    function setTreasury(address treasury_) external {
        _treasury = treasury_;
    }

    function getEscrow(bytes32 taskId) external view returns (MarketplaceTypes.EscrowDeposit memory) {
        return _escrows[taskId];
    }

    function getPendingBuyerRefund(address) external pure returns (uint256) {
        return 0;
    }

    function getPendingProviderPayout(address) external pure returns (uint256) {
        return 0;
    }

    function getPendingTreasuryFees(address) external pure returns (uint256) {
        return 0;
    }

    function getTaskMarketplace() external view returns (address) {
        return _taskMarketplace;
    }

    function getTreasury() external view returns (address) {
        return _treasury;
    }

    function totalAccountedObligations() external pure returns (uint256) {
        return 0;
    }
}

contract MinimalTaskMarketplace is TaskMarketplace {
    uint256 private _taskNonce;

    constructor(address owner_) TaskMarketplace(owner_) {}

    function estimateTaskCost(
        MarketplaceTypes.ResourceType,
        uint256,
        uint256
    ) external pure override returns (uint256) {
        return 0;
    }

    function _createTask(
        MarketplaceTypes.ResourceType resourceType,
        uint256 requiredPower,
        uint256 duration,
        uint256 maxPrice,
        uint8 minTrustLevel,
        string calldata specificationURI
    ) internal override returns (bytes32 taskId) {
        _taskNonce += 1;
        taskId = keccak256(abi.encodePacked(address(this), msg.sender, _taskNonce));

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
    }

    function _cancelTask(bytes32 taskId) internal override {
        _markTaskCancelled(taskId);
    }

    function _disputeTask(bytes32 taskId, string calldata) internal override {
        _markTaskDisputed(taskId);
    }

    function _approveResult(bytes32 taskId) internal override {
        _markTaskVerified(taskId);
    }

    function _acceptTask(bytes32 taskId, bytes32 nodeId) internal override {
        _assignTask(taskId, nodeId, block.timestamp);
    }

    function _submitResult(bytes32 taskId, bytes32 resultHash, string calldata resultURI) internal override {
        MarketplaceTypes.TaskResult memory result = MarketplaceTypes.TaskResult({
            taskId: taskId,
            resultHash: resultHash,
            resultURI: resultURI,
            actualDuration: 0,
            computeUnitsUsed: 0,
            verified: false
        });

        _recordTaskResult(taskId, result, block.timestamp);
    }
}

contract TaskEscrowFundingSkeletonBase {
    Vm private constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    address private constant BUYER = address(0xB0B);
    address private constant NOT_BUYER = address(0xCAFE);
    address private constant TREASURY = address(0xD00D);
    bytes32 private constant NODE_ID = keccak256("node-1");

    MinimalTaskMarketplace private _marketplace;
    MockEscrowManager private _mockEscrowManager;

    event AccountingBucketMoved(
        bytes32 indexed referenceId,
        address indexed recipient,
        uint8 fromBucket,
        uint8 toBucket,
        uint256 amount
    );

    function _setupMarketplace() internal {
        _marketplace = new MinimalTaskMarketplace(address(this));
        _mockEscrowManager = new MockEscrowManager(TREASURY);
        _mockEscrowManager.setTaskMarketplace(address(_marketplace));
        _marketplace.setEscrowManager(address(_mockEscrowManager));

        vm.deal(BUYER, 20 ether);
        vm.deal(NOT_BUYER, 20 ether);
    }

    function _marketplaceRef() internal view returns (MinimalTaskMarketplace) {
        return _marketplace;
    }

    function _buyer() internal pure returns (address) {
        return BUYER;
    }

    function _notBuyer() internal pure returns (address) {
        return NOT_BUYER;
    }

    function _nodeId() internal pure returns (bytes32) {
        return NODE_ID;
    }

    function _treasury() internal pure returns (address) {
        return TREASURY;
    }

    function _vm() internal pure returns (Vm) {
        return vm;
    }

    receive() external payable {}
}

contract TaskEscrowFundingConstraintsTest is TaskEscrowFundingSkeletonBase {
    function setUp() public {
        _setupMarketplace();
    }

    function testCreateTaskRejectsEthValue() public {
        MinimalTaskMarketplace marketplace = _marketplaceRef();
        bytes memory callData = abi.encodeWithSelector(
            marketplace.createTask.selector,
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "spec"
        );

        Vm vmRef = _vm();
        vmRef.prank(_buyer());
        (bool success, ) = address(_marketplaceRef()).call{value: 1 ether}(callData);
        require(!success, "createTask accepted ETH");
    }

    function testFundTaskEscrowBuyerOnlyOneShotAndWithinMax() public {
        Vm vmRef = _vm();
        MinimalTaskMarketplace marketplace = _marketplaceRef();

        vmRef.prank(_buyer());
        bytes32 taskId = marketplace.createTask(MarketplaceTypes.ResourceType.GPU, 100, 1 hours, 2 ether, 1, "spec");

        vmRef.prank(_notBuyer());
        vmRef.expectRevert(
            abi.encodeWithSelector(TaskMarketplace.TaskFundingNotAllowed.selector, taskId, _notBuyer(), _buyer())
        );
        marketplace.fundTaskEscrow{value: 1 ether}(taskId);

        vmRef.prank(_buyer());
        vmRef.expectRevert(abi.encodeWithSelector(TaskMarketplace.TaskValueOutOfRange.selector, 0, 2 ether));
        marketplace.fundTaskEscrow{value: 0}(taskId);

        vmRef.prank(_buyer());
        vmRef.expectRevert(abi.encodeWithSelector(TaskMarketplace.TaskValueOutOfRange.selector, 3 ether, 2 ether));
        marketplace.fundTaskEscrow{value: 3 ether}(taskId);

        vmRef.prank(_buyer());
        marketplace.fundTaskEscrow{value: 1 ether}(taskId);

        MarketplaceTypes.Task memory task = marketplace.getTask(taskId);
        require(task.escrowAmount == 1 ether, "escrow amount mismatch");

        vmRef.prank(_buyer());
        vmRef.expectRevert(abi.encodeWithSelector(TaskMarketplace.TaskEscrowAlreadyFunded.selector, taskId));
        marketplace.fundTaskEscrow{value: 1 ether}(taskId);
    }

    function testAcceptTaskRequiresFundedOpenTask() public {
        Vm vmRef = _vm();
        MinimalTaskMarketplace marketplace = _marketplaceRef();

        vmRef.prank(_buyer());
        bytes32 unfundedTaskId = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "unfunded"
        );

        vmRef.prank(_buyer());
        vmRef.expectRevert(abi.encodeWithSelector(TaskMarketplace.TaskEscrowNotFunded.selector, unfundedTaskId));
        marketplace.acceptTask(unfundedTaskId, _nodeId());

        vmRef.prank(_buyer());
        bytes32 fundedTaskId = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "funded"
        );

        vmRef.prank(_buyer());
        marketplace.fundTaskEscrow{value: 1 ether}(fundedTaskId);

        vmRef.prank(_buyer());
        marketplace.acceptTask(fundedTaskId, _nodeId());

        MarketplaceTypes.Task memory fundedTask = marketplace.getTask(fundedTaskId);
        require(fundedTask.status == MarketplaceTypes.TaskStatus.Assigned, "funded task not assigned");
    }
}

contract TaskEscrowOpenTasksFilterTest is TaskEscrowFundingSkeletonBase {
    function setUp() public {
        _setupMarketplace();
    }

    function testGetOpenTasksReturnsOnlyFundedOpenTasks() public {
        Vm vmRef = _vm();
        MinimalTaskMarketplace marketplace = _marketplaceRef();

        vmRef.prank(_buyer());
        bytes32 unfundedOpen = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "unfunded-open"
        );

        vmRef.prank(_buyer());
        bytes32 fundedOpen = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "funded-open"
        );
        vmRef.prank(_buyer());
        marketplace.fundTaskEscrow{value: 1 ether}(fundedOpen);

        vmRef.prank(_buyer());
        bytes32 assignedTask = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "assigned"
        );
        vmRef.prank(_buyer());
        marketplace.fundTaskEscrow{value: 1 ether}(assignedTask);
        vmRef.prank(_buyer());
        marketplace.acceptTask(assignedTask, _nodeId());

        vmRef.prank(_buyer());
        bytes32 cancelledTask = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "cancelled"
        );
        vmRef.prank(_buyer());
        marketplace.fundTaskEscrow{value: 1 ether}(cancelledTask);
        vmRef.prank(_buyer());
        marketplace.cancelTask(cancelledTask);

        vmRef.prank(_buyer());
        bytes32 verifiedTask = marketplace.createTask(
            MarketplaceTypes.ResourceType.GPU,
            100,
            1 hours,
            2 ether,
            1,
            "verified"
        );
        vmRef.prank(_buyer());
        marketplace.fundTaskEscrow{value: 1 ether}(verifiedTask);
        vmRef.prank(_buyer());
        marketplace.acceptTask(verifiedTask, _nodeId());
        vmRef.prank(_buyer());
        marketplace.submitResult(verifiedTask, keccak256("result"), "uri");
        vmRef.prank(_buyer());
        marketplace.approveResult(verifiedTask);

        bytes32[] memory openTasks = marketplace.getOpenTasks(MarketplaceTypes.ResourceType.GPU);

        require(openTasks.length == 1, "unexpected open task count");
        require(openTasks[0] == fundedOpen, "wrong open task returned");
        require(openTasks[0] != unfundedOpen, "unfunded open task leaked");
    }
}

contract NodeRegistrySlashAccountingTest {
    Vm private constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    address private constant TREASURY = address(0xD00D);

    event AccountingBucketMoved(
        bytes32 indexed referenceId,
        address indexed recipient,
        uint8 fromBucket,
        uint8 toBucket,
        uint256 amount
    );

    function testSlashStakeEmitsLockedStakeToTreasuryBucketMove() public {
        NodeRegistry nodeRegistry = new NodeRegistry(address(this), TREASURY);

        vm.deal(address(this), 20 ether);
        bytes32 nodeId = nodeRegistry.registerNode{value: 0.6 ether}(MarketplaceTypes.ResourceType.GPU, "node");

        uint256 slashAmount = 0.1 ether;

        vm.expectEmit(true, true, false, true);
        emit AccountingBucketMoved(nodeId, TREASURY, 1, 3, slashAmount);
        nodeRegistry.slashStake(nodeId, slashAmount);
    }

    receive() external payable {}
}
