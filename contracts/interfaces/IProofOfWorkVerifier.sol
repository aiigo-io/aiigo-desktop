// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {MarketplaceTypes} from "../types/MarketplaceTypes.sol";

interface IProofOfWorkVerifier {
    event ChallengeIssued(
        bytes32 indexed challengeId,
        bytes32 indexed nodeId,
        uint256 difficulty,
        uint256 deadline
    );
    event ChallengeSolved(
        bytes32 indexed challengeId,
        bytes32 indexed nodeId,
        uint256 solutionTime,
        uint256 verifiedPower
    );
    event ChallengeFailed(bytes32 indexed challengeId, bytes32 indexed nodeId, string reason);

    function issueChallenge(bytes32 nodeId) external returns (bytes32 challengeId);

    function submitSolution(bytes32 challengeId, uint256 nonce) external returns (bool passed);

    function getChallenge(bytes32 challengeId) external view returns (MarketplaceTypes.Challenge memory);

    function getVerificationHistory(
        bytes32 nodeId
    ) external view returns (MarketplaceTypes.VerificationResult[] memory);

    function calculateComputePower(
        uint256 solutionTime,
        uint256 difficulty
    ) external pure returns (uint256);
}

