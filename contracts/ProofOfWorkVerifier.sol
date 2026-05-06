// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {Ownable2Step} from "@openzeppelin/contracts/access/Ownable2Step.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {INodeRegistry} from "./interfaces/INodeRegistry.sol";
import {IProofOfWorkVerifier} from "./interfaces/IProofOfWorkVerifier.sol";
import {MarketplaceTypes} from "./types/MarketplaceTypes.sol";

abstract contract ProofOfWorkVerifier is IProofOfWorkVerifier, Ownable2Step, Pausable {
    error ChallengeAlreadyExists(bytes32 challengeId);
    error ChallengeNotFound(bytes32 challengeId);
    error ChallengeAlreadyCompleted(bytes32 challengeId);
    error ContractReferenceNotSet(bytes32 referenceName);
    error ContractReferenceSmokeCheckFailed(bytes4 selector, address candidate);
    error InvalidContractReference(address candidate);
    error OwnerRenounceDisabled();
    error InvalidChallengeRecord();
    error InvalidVerificationResultNodeId();
    error ZeroChallengeIdNotAllowed();
    error ZeroNodeIdNotAllowed();
    error ZeroAddressNotAllowed();

    uint256 public constant BASE_DIFFICULTY = 1e18;
    bytes32 private constant REFERENCE_NODE_REGISTRY = keccak256("NODE_REGISTRY");

    INodeRegistry private _nodeRegistry;

    mapping(bytes32 => MarketplaceTypes.Challenge) private _challenges;
    mapping(bytes32 => bool) private _challengeExists;
    mapping(bytes32 => MarketplaceTypes.VerificationResult[]) private _verificationHistory;

    constructor(address owner_, address nodeRegistry_) Ownable(owner_) {
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
        _requireChallengeId(challengeId);
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
    ) external pure virtual override returns (uint256);

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

    function _requireContractReference(address candidate) internal view {
        if (candidate == address(0)) {
            revert ZeroAddressNotAllowed();
        }
        if (candidate.code.length == 0) {
            revert InvalidContractReference(candidate);
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

    function _issueChallenge(bytes32 nodeId) internal virtual returns (bytes32 challengeId);

    function _submitSolution(bytes32 challengeId, uint256 nonce) internal virtual returns (bool passed);
}
