// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {AccessControlDefaultAdminRules} from "@openzeppelin/contracts/access/extensions/AccessControlDefaultAdminRules.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {IEscrowManager} from "./interfaces/IEscrowManager.sol";
import {ITaskMarketplace} from "./interfaces/ITaskMarketplace.sol";
import {MarketplaceTypes} from "./types/MarketplaceTypes.sol";

contract EscrowManager is IEscrowManager, AccessControlDefaultAdminRules, Pausable, ReentrancyGuard {
    error AmountMustBeGreaterThanZero();
    error ClaimTransferFailed();
    error DirectETHNotAccepted();
    error EscrowAlreadyExists(bytes32 taskId);
    error EscrowAlreadySettled(bytes32 taskId);
    error EscrowNotFound(bytes32 taskId);
    error InvalidBuyer();
    error InvalidProvider();
    error MissingClaimableBalance();
    error RoleRenounceDisabled();
    error SolvencyInvariantViolated(uint256 balance, uint256 obligations);
    error ContractReferenceSmokeCheckFailed(bytes4 selector, address candidate);
    error TaskMarketplaceNotConfigured();
    error UnauthorizedTaskMarketplace(address caller);
    error InvalidContractReference(address candidate);
    error ZeroAddressNotAllowed();
    error ZeroTaskIdNotAllowed();

    uint48 public constant DEFAULT_ADMIN_DELAY = 1 days;
    bytes32 public constant RELEASER_ROLE = keccak256("RELEASER_ROLE");
    uint256 public constant BASIS_POINTS = 10_000;
    uint256 public constant PLATFORM_FEE_BPS = 800;

    uint8 private constant BUCKET_UNTRACKED = 0;
    uint8 private constant BUCKET_LOCKED = 1;
    uint8 private constant BUCKET_BUYER_REFUND = 2;
    uint8 private constant BUCKET_PROVIDER_PAYOUT = 3;
    uint8 private constant BUCKET_TREASURY_FEE = 4;

    address private _taskMarketplace;
    address private _treasury;

    mapping(bytes32 => MarketplaceTypes.EscrowDeposit) private _escrows;
    mapping(address => uint256) private _pendingBuyerRefunds;
    mapping(address => uint256) private _pendingProviderPayouts;
    mapping(address => uint256) private _pendingTreasuryFees;

    uint256 private _totalLockedEscrow;
    uint256 private _totalPendingBuyerRefunds;
    uint256 private _totalPendingProviderPayouts;
    uint256 private _totalPendingTreasuryFees;
    uint256 private _totalAccountedObligations;

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
        _grantRole(RELEASER_ROLE, initialDefaultAdmin);
    }

    modifier onlyTaskMarketplace() {
        if (_taskMarketplace == address(0)) {
            revert TaskMarketplaceNotConfigured();
        }
        if (msg.sender != _taskMarketplace) {
            revert UnauthorizedTaskMarketplace(msg.sender);
        }
        _;
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

    function setTaskMarketplace(address taskMarketplace_) external onlyRole(DEFAULT_ADMIN_ROLE) {
        _requireContractReference(taskMarketplace_);

        (bool ok, bytes memory returnData) = taskMarketplace_.staticcall(
            abi.encodeWithSelector(ITaskMarketplace.getTasksByBuyer.selector, address(this))
        );
        if (!ok || returnData.length < 64) {
            revert ContractReferenceSmokeCheckFailed(ITaskMarketplace.getTasksByBuyer.selector, taskMarketplace_);
        }

        abi.decode(returnData, (bytes32[]));

        address previousMarketplace = _taskMarketplace;
        if (previousMarketplace != address(0) && hasRole(RELEASER_ROLE, previousMarketplace)) {
            _revokeRole(RELEASER_ROLE, previousMarketplace);
        }

        _taskMarketplace = taskMarketplace_;

        if (!hasRole(RELEASER_ROLE, taskMarketplace_)) {
            _grantRole(RELEASER_ROLE, taskMarketplace_);
        }
    }

    function setTreasury(address treasury_) external onlyRole(DEFAULT_ADMIN_ROLE) {
        if (treasury_ == address(0)) {
            revert ZeroAddressNotAllowed();
        }

        _treasury = treasury_;
    }

    function deposit(
        bytes32 taskId,
        address buyer
    ) external payable onlyTaskMarketplace whenNotPaused returns (bytes32 escrowId) {
        _requireTaskId(taskId);
        if (msg.value == 0) {
            revert AmountMustBeGreaterThanZero();
        }
        if (buyer == address(0)) {
            revert InvalidBuyer();
        }
        if (_escrows[taskId].exists) {
            revert EscrowAlreadyExists(taskId);
        }

        _escrows[taskId] = MarketplaceTypes.EscrowDeposit({
            taskId: taskId,
            buyer: buyer,
            provider: address(0),
            treasuryRecipient: address(0),
            amount: msg.value,
            platformFee: 0,
            providerPayout: 0,
            buyerRefund: 0,
            depositedAt: block.timestamp,
            settlement: MarketplaceTypes.EscrowSettlementKind.None,
            exists: true
        });

        _totalLockedEscrow += msg.value;
        _totalAccountedObligations += msg.value;

        emit EscrowDeposited(taskId, buyer, msg.value);
        emit AccountingBucketMoved(taskId, buyer, BUCKET_UNTRACKED, BUCKET_LOCKED, msg.value);

        _assertSolvent();
        return taskId;
    }

    function release(
        bytes32 taskId,
        address provider
    ) external onlyTaskMarketplace onlyRole(RELEASER_ROLE) whenNotPaused nonReentrant {
        _requireTaskId(taskId);
        if (provider == address(0)) {
            revert InvalidProvider();
        }

        MarketplaceTypes.EscrowDeposit storage escrow = _getOpenEscrow(taskId);
        uint256 platformFee = (escrow.amount * PLATFORM_FEE_BPS) / BASIS_POINTS;
        uint256 providerPayout = escrow.amount - platformFee;

        escrow.provider = provider;
        escrow.treasuryRecipient = _treasury;
        escrow.platformFee = platformFee;
        escrow.providerPayout = providerPayout;
        escrow.buyerRefund = 0;
        escrow.settlement = MarketplaceTypes.EscrowSettlementKind.Released;

        _totalLockedEscrow -= escrow.amount;
        emit AccountingBucketMoved(taskId, escrow.buyer, BUCKET_LOCKED, BUCKET_UNTRACKED, escrow.amount);

        _pendingProviderPayouts[provider] += providerPayout;
        _totalPendingProviderPayouts += providerPayout;
        emit ProviderPayoutQueued(taskId, provider, providerPayout);
        emit AccountingBucketMoved(taskId, provider, BUCKET_UNTRACKED, BUCKET_PROVIDER_PAYOUT, providerPayout);

        if (platformFee > 0) {
            _pendingTreasuryFees[escrow.treasuryRecipient] += platformFee;
            _totalPendingTreasuryFees += platformFee;
            emit TreasuryFeeQueued(taskId, escrow.treasuryRecipient, platformFee);
            emit AccountingBucketMoved(taskId, escrow.treasuryRecipient, BUCKET_UNTRACKED, BUCKET_TREASURY_FEE, platformFee);
        }

        emit EscrowReleased(taskId, provider, providerPayout, platformFee);
        _assertSolvent();
    }

    function refund(bytes32 taskId) external onlyTaskMarketplace onlyRole(RELEASER_ROLE) nonReentrant {
        _requireTaskId(taskId);
        MarketplaceTypes.EscrowDeposit storage escrow = _getOpenEscrow(taskId);

        escrow.provider = address(0);
        escrow.treasuryRecipient = address(0);
        escrow.platformFee = 0;
        escrow.providerPayout = 0;
        escrow.buyerRefund = escrow.amount;
        escrow.settlement = MarketplaceTypes.EscrowSettlementKind.Refunded;

        _totalLockedEscrow -= escrow.amount;
        _pendingBuyerRefunds[escrow.buyer] += escrow.amount;
        _totalPendingBuyerRefunds += escrow.amount;

        emit AccountingBucketMoved(taskId, escrow.buyer, BUCKET_LOCKED, BUCKET_BUYER_REFUND, escrow.amount);
        emit BuyerRefundQueued(taskId, escrow.buyer, escrow.amount);
        emit EscrowRefunded(taskId, escrow.buyer, escrow.amount);

        _assertSolvent();
    }

    function splitPayment(
        bytes32 taskId,
        address provider,
        uint256 providerShare
    ) external onlyTaskMarketplace onlyRole(RELEASER_ROLE) whenNotPaused nonReentrant {
        _requireTaskId(taskId);
        if (provider == address(0)) {
            revert InvalidProvider();
        }
        if (providerShare == 0) {
            revert AmountMustBeGreaterThanZero();
        }

        MarketplaceTypes.EscrowDeposit storage escrow = _getOpenEscrow(taskId);
        if (providerShare > escrow.amount) {
            revert AmountMustBeGreaterThanZero();
        }

        uint256 platformFee = escrow.amount - providerShare;

        escrow.provider = provider;
        escrow.treasuryRecipient = _treasury;
        escrow.platformFee = platformFee;
        escrow.providerPayout = providerShare;
        escrow.buyerRefund = 0;
        escrow.settlement = MarketplaceTypes.EscrowSettlementKind.Split;

        _totalLockedEscrow -= escrow.amount;
        emit AccountingBucketMoved(taskId, escrow.buyer, BUCKET_LOCKED, BUCKET_UNTRACKED, escrow.amount);

        _pendingProviderPayouts[provider] += providerShare;
        _totalPendingProviderPayouts += providerShare;
        emit ProviderPayoutQueued(taskId, provider, providerShare);
        emit AccountingBucketMoved(taskId, provider, BUCKET_UNTRACKED, BUCKET_PROVIDER_PAYOUT, providerShare);

        if (platformFee > 0) {
            _pendingTreasuryFees[escrow.treasuryRecipient] += platformFee;
            _totalPendingTreasuryFees += platformFee;
            emit TreasuryFeeQueued(taskId, escrow.treasuryRecipient, platformFee);
            emit AccountingBucketMoved(taskId, escrow.treasuryRecipient, BUCKET_UNTRACKED, BUCKET_TREASURY_FEE, platformFee);
        }

        emit EscrowReleased(taskId, provider, providerShare, platformFee);
        _assertSolvent();
    }

    function claimBuyerRefund() external nonReentrant {
        uint256 amount = _pendingBuyerRefunds[msg.sender];
        if (amount == 0) {
            revert MissingClaimableBalance();
        }

        _pendingBuyerRefunds[msg.sender] = 0;
        _totalPendingBuyerRefunds -= amount;
        _totalAccountedObligations -= amount;

        (bool sent, ) = payable(msg.sender).call{value: amount}("");
        if (!sent) {
            revert ClaimTransferFailed();
        }

        emit BuyerRefundClaimed(msg.sender, amount);
        _assertSolvent();
    }

    function claimProviderPayout() external nonReentrant {
        uint256 amount = _pendingProviderPayouts[msg.sender];
        if (amount == 0) {
            revert MissingClaimableBalance();
        }

        _pendingProviderPayouts[msg.sender] = 0;
        _totalPendingProviderPayouts -= amount;
        _totalAccountedObligations -= amount;

        (bool sent, ) = payable(msg.sender).call{value: amount}("");
        if (!sent) {
            revert ClaimTransferFailed();
        }

        emit ProviderPayoutClaimed(msg.sender, amount);
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

    function getEscrow(bytes32 taskId) external view returns (MarketplaceTypes.EscrowDeposit memory) {
        _requireTaskId(taskId);
        return _escrows[taskId];
    }

    function getPendingBuyerRefund(address buyer) external view returns (uint256) {
        return _pendingBuyerRefunds[buyer];
    }

    function getPendingProviderPayout(address provider) external view returns (uint256) {
        return _pendingProviderPayouts[provider];
    }

    function getPendingTreasuryFees(address recipient) external view returns (uint256) {
        return _pendingTreasuryFees[recipient];
    }

    function getTaskMarketplace() external view returns (address) {
        return _taskMarketplace;
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

    function _requireTaskId(bytes32 taskId) private pure {
        if (taskId == bytes32(0)) {
            revert ZeroTaskIdNotAllowed();
        }
    }

    function _getOpenEscrow(
        bytes32 taskId
    ) private view returns (MarketplaceTypes.EscrowDeposit storage escrow) {
        escrow = _escrows[taskId];
        if (!escrow.exists) {
            revert EscrowNotFound(taskId);
        }
        if (escrow.settlement != MarketplaceTypes.EscrowSettlementKind.None) {
            revert EscrowAlreadySettled(taskId);
        }
    }

    function _assertSolvent() private view {
        if (address(this).balance < _totalAccountedObligations) {
            revert SolvencyInvariantViolated(address(this).balance, _totalAccountedObligations);
        }
    }

    function _requireContractReference(address candidate) private view {
        if (candidate == address(0)) {
            revert ZeroAddressNotAllowed();
        }
        if (candidate.code.length == 0) {
            revert InvalidContractReference(candidate);
        }
    }
}