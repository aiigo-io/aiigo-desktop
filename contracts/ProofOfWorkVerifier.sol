// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {INodeRegistry} from "./interfaces/INodeRegistry.sol";
import {IProofOfWorkVerifier} from "./interfaces/IProofOfWorkVerifier.sol";
import {MarketplaceTypes} from "./types/MarketplaceTypes.sol";

contract ProofOfWorkVerifier is IProofOfWorkVerifier, Ownable2Step, Pausable {
    error ChallengeAlreadyExists(bytes32 challengeId);
    error ChallengeNotFound(bytes32 challengeId);
    error ChallengeAlreadyCompleted(bytes32 challengeId);
    error ChallengeExpired(bytes32 challengeId);
    error NotNodeOwner(bytes32 nodeId, address caller);
    error ContractReferenceNotSet(bytes32 referenceName);
    error SameContractReference(address currentReference, address candidate);
    error InvalidContractInterface(bytes4 selector, address candidate);
    error NonContractAddress(address candidate);
    error OwnerRenounceDisabled();
    error InvalidChallengeRecord();
    error InvalidVerificationResultNodeId();
    error ZeroChallengeIdNotAllowed();
    error ZeroNodeIdNotAllowed();
    error ZeroAddressNotAllowed();

    uint256 public constant BASE_DIFFICULTY = 1e18;
    bytes32 private constant REFERENCE_NODE_REGISTRY = keccak256("NODE_REGISTRY");
    event ContractReferenceUpdated(bytes32 indexed referenceName, address indexed previousReference, address indexed newReference);

    INodeRegistry private _nodeRegistry;

    mapping(bytes32 => MarketplaceTypes.Challenge) private _challenges;
    mapping(bytes32 => bool) private _challengeExists;
    mapping(bytes32 => MarketplaceTypes.VerificationResult[]) private _verificationHistory;

    uint256 private _challengeNonce;

    constructor(address owner_, address nodeRegistry_) Ownable(owner_) {
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

        _nodeRegistry = INodeRegistry(nodeRegistry_);
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

    function pause() external onlyOwner {
        _pause();
    }

    function unpause() external onlyOwner {
        _unpause();
    }

    function issueChallenge(bytes32 nodeId) external override whenNotPaused returns (bytes32 challengeId) {
        _requireNodeId(nodeId);
        challengeId = _issueChallenge(nodeId);
        _requireChallengeId(challengeId);
    }

    function submitSolution(
        bytes32 challengeId,
        uint256 nonce
    ) external override whenNotPaused returns (bool passed) {
        // _readChallenge validates ID and existence
        MarketplaceTypes.Challenge memory ch = _readChallenge(challengeId);
        if (block.timestamp > ch.deadline) {
            revert ChallengeExpired(challengeId);
        }
        return _submitSolution(challengeId, nonce);
    }

    function renounceOwnership() public view override onlyOwner {
        revert OwnerRenounceDisabled();
    }

    function getChallenge(bytes32 challengeId) external view override returns (MarketplaceTypes.Challenge memory) {
        _requireChallengeId(challengeId);
        _requireChallengeExists(challengeId);
        return _challenges[challengeId];
    }

    function getVerificationHistory(
        bytes32 nodeId
    ) external view override returns (MarketplaceTypes.VerificationResult[] memory) {
        return _verificationHistory[nodeId];
    }

    function calculateComputePower(
        uint256 solutionTime,
        uint256 difficulty
    ) external pure virtual override returns (uint256) {
        uint256 t = solutionTime > 0 ? solutionTime : 1;
        uint256 d = difficulty > 0 ? difficulty : 1;
        return (BASE_DIFFICULTY * d) / t;
    }

    function _nodeRegistryRef() internal view returns (INodeRegistry) {
        if (address(_nodeRegistry) == address(0)) {
            revert ContractReferenceNotSet(REFERENCE_NODE_REGISTRY);
        }
        return _nodeRegistry;
    }

    function _readChallenge(bytes32 challengeId) internal view returns (MarketplaceTypes.Challenge memory) {
        _requireChallengeId(challengeId);
        _requireChallengeExists(challengeId);
        return _challenges[challengeId];
    }

    function _readVerificationHistory(
        bytes32 nodeId
    ) internal view returns (MarketplaceTypes.VerificationResult[] memory) {
        _requireNodeId(nodeId);
        return _verificationHistory[nodeId];
    }

    function _registerIssuedChallenge(MarketplaceTypes.Challenge memory challenge) internal {
        _storeIssuedChallenge(challenge);
    }

    function _resolveSolvedChallenge(
        bytes32 challengeId,
        uint256 solutionTime,
        MarketplaceTypes.VerificationResult memory result
    ) internal {
        _markChallengeCompleted(challengeId, solutionTime);
        _recordVerificationResult(challengeId, result);
    }

    function _storeIssuedChallenge(MarketplaceTypes.Challenge memory challenge) private {
        _requireChallengeId(challenge.challengeId);
        if (_challengeExists[challenge.challengeId]) {
            revert ChallengeAlreadyExists(challenge.challengeId);
        }
        _requireNodeId(challenge.nodeId);
        if (challenge.issuedAt == 0 || challenge.deadline < challenge.issuedAt) {
            revert InvalidChallengeRecord();
        }
        if (challenge.completed) {
            revert InvalidChallengeRecord();
        }
        _challengeExists[challenge.challengeId] = true;
        _challenges[challenge.challengeId] = challenge;
    }

    function _markChallengeCompleted(
        bytes32 challengeId,
        uint256 solutionTime
    ) private {
        _requireChallengeId(challengeId);
        _requireChallengeExists(challengeId);

        MarketplaceTypes.Challenge storage challenge = _challenges[challengeId];
        _requireNodeId(challenge.nodeId);
        if (challenge.completed) {
            revert ChallengeAlreadyCompleted(challengeId);
        }

        challenge.completed = true;
        challenge.solutionTime = solutionTime;
    }

    function _recordVerificationResult(bytes32 challengeId, MarketplaceTypes.VerificationResult memory result) private {
        _requireChallengeId(challengeId);
        _requireChallengeExists(challengeId);

        MarketplaceTypes.Challenge storage challenge = _challenges[challengeId];
        _requireNodeId(challenge.nodeId);

        if (result.nodeId != challenge.nodeId) {
            revert InvalidVerificationResultNodeId();
        }
        _verificationHistory[challenge.nodeId].push(result);
    }

    function _requireChallengeExists(bytes32 challengeId) internal view {
        if (!_challengeExists[challengeId]) {
            revert ChallengeNotFound(challengeId);
        }
    }

    function _requireChallengeId(bytes32 challengeId) internal pure {
        if (challengeId == bytes32(0)) {
            revert ZeroChallengeIdNotAllowed();
        }
    }

    function _requireNodeId(bytes32 nodeId) internal pure {
        if (nodeId == bytes32(0)) {
            revert ZeroNodeIdNotAllowed();
        }
    }

    function _issueChallenge(bytes32 nodeId) internal virtual returns (bytes32 challengeId) {
        // Only the node owner may trigger a PoW challenge.
        MarketplaceTypes.Node memory node = _nodeRegistryRef().getNode(nodeId);
        if (node.owner != msg.sender) {
            revert NotNodeOwner(nodeId, msg.sender);
        }

        _challengeNonce += 1;
        challengeId = keccak256(abi.encode(block.chainid, address(this), nodeId, _challengeNonce));

        // Seed mixes storage nonce + block hash so it is not predictable before the tx lands.
        bytes32 seed = keccak256(abi.encodePacked(nodeId, blockhash(block.number - 1), _challengeNonce));

        // MVP difficulty = 2^16 = 65536.
        // Target = type(uint256).max / 65536  ≈ 2^240.
        // Roughly 1-in-65536 nonces passes, so off-chain search terminates in < 1 s
        // while a trivially bad nonce (e.g. 0 if hash is unlucky) can still fail,
        // giving the ChallengeFailed branch reachability in tests.
        uint256 mvpDifficulty = 1 << 16;

        MarketplaceTypes.Challenge memory challenge = MarketplaceTypes.Challenge({
            challengeId: challengeId,
            nodeId: nodeId,
            seed: seed,
            difficulty: mvpDifficulty,
            issuedAt: block.timestamp,
            deadline: block.timestamp + 1 hours,
            completed: false,
            solutionTime: 0
        });

        _registerIssuedChallenge(challenge);
        emit ChallengeIssued(challengeId, nodeId, challenge.difficulty, challenge.deadline);
    }

    function _submitSolution(bytes32 challengeId, uint256 nonce) internal virtual returns (bool passed) {
        MarketplaceTypes.Challenge memory challenge = _readChallenge(challengeId);

        // PoW check: hash(seed || nonce) must fit within the difficulty target.
        // target = type(uint256).max / difficulty
        // A bad nonce whose hash exceeds the target returns false + emits ChallengeFailed.
        bytes32 hashResult = keccak256(abi.encodePacked(challenge.seed, nonce));
        uint256 target = type(uint256).max / challenge.difficulty;

        if (uint256(hashResult) > target) {
            emit ChallengeFailed(challengeId, challenge.nodeId, "invalid nonce");
            return false;
        }

        uint256 solutionTime = block.timestamp > challenge.issuedAt
            ? block.timestamp - challenge.issuedAt
            : 1;

        uint256 t = solutionTime > 0 ? solutionTime : 1;
        uint256 verifiedPower = (BASE_DIFFICULTY * challenge.difficulty) / t;

        MarketplaceTypes.VerificationResult memory result = MarketplaceTypes.VerificationResult({
            nodeId: challenge.nodeId,
            verifiedPower: verifiedPower,
            timestamp: block.timestamp,
            passed: true
        });

        _resolveSolvedChallenge(challengeId, solutionTime, result);

        // Always update compute power and reputation.
        INodeRegistry registry = _nodeRegistryRef();
        registry.updateComputePower(challenge.nodeId, verifiedPower);
        registry.updateReputation(challenge.nodeId, int256(100));

        // State-machine guard: only walk Pending → Verified → Active for new nodes.
        // An already-Active node keeps running; we only refresh its metrics above.
        MarketplaceTypes.Node memory node = registry.getNode(challenge.nodeId);
        if (node.status == MarketplaceTypes.NodeStatus.Pending) {
            registry.updateNodeStatus(challenge.nodeId, MarketplaceTypes.NodeStatus.Verified);
            registry.updateNodeStatus(challenge.nodeId, MarketplaceTypes.NodeStatus.Active);
        } else if (node.status == MarketplaceTypes.NodeStatus.Verified) {
            // Already verified but not yet active — only need the final step.
            registry.updateNodeStatus(challenge.nodeId, MarketplaceTypes.NodeStatus.Active);
        }
        // NodeStatus.Active, Inactive, Slashed: no status transition on re-verify.

        emit ChallengeSolved(challengeId, challenge.nodeId, solutionTime, verifiedPower);
        return true;
    }
}
