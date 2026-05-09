// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// ⚠ DEPRECATED — superseded by FinalizeComputeMvpEoa.s.sol (EOA) or
//   `npm run finalize:compute:safe-calldata` (Gnosis Safe).
//   This file is retained for backwards compatibility only.
//
// Finalize script — called by the MULTISIG (or pending admin) to accept ownership
// of all four contracts after DeployComputeMvp.s.sol has been run.
//
// Ownable2Step contracts (TaskMarketplace, ProofOfWorkVerifier):
//   The pending owner calls acceptOwnership().
//
// AccessControlDefaultAdminRules contracts (NodeRegistry, EscrowManager):
//   The pending admin calls acceptDefaultAdminTransfer() once the adminDelay has elapsed.
//
// Run with:
//   forge script script/FinalizeComputeMvp.s.sol:FinalizeComputeMvp \
//     --rpc-url $RPC_URL --broadcast \
//     --sender $MULTISIG --private-key $MULTISIG_PK

/// @dev Subset of Ownable2Step interface required here.
interface IOwnable2Step {
    function pendingOwner() external view returns (address);
    function acceptOwnership() external;
    function owner() external view returns (address);
}

/// @dev Subset of AccessControlDefaultAdminRules interface required here.
interface IDefaultAdminRules {
    function pendingDefaultAdmin()
        external
        view
        returns (address newAdmin, uint48 schedule);
    function acceptDefaultAdminTransfer() external;
    function defaultAdmin() external view returns (address);
}

/// @dev Minimal VM cheatcode interface (subset needed for scripting)
interface IVm {
    function startBroadcast(address sender) external;
    function stopBroadcast() external;
    function envOr(string calldata name, address defaultValue) external returns (address value);
}

contract FinalizeComputeMvp {
    IVm private constant vm = IVm(address(uint160(uint256(keccak256("hevm cheat code")))));

    event OwnershipAccepted(string contractName, address newOwner);
    event DefaultAdminAccepted(string contractName, address newAdmin);
    event FinalizeComplete(
        address nodeRegistry,
        address powVerifier,
        address escrowManager,
        address taskMarketplace,
        address newOwner
    );

    function run() external {
        // ── Configuration ────────────────────────────────────────────────────
        // Caller must be the pending owner / pending admin set during Deploy.
        address multisig = vm.envOr("MULTISIG", msg.sender);

        // Contract addresses — set via env vars or hard-code after deploy.
        address nodeRegistryAddr   = vm.envOr("NODE_REGISTRY",    address(0));
        address powVerifierAddr    = vm.envOr("POW_VERIFIER",     address(0));
        address escrowManagerAddr  = vm.envOr("ESCROW_MANAGER",   address(0));
        address marketplaceAddr    = vm.envOr("TASK_MARKETPLACE", address(0));

        require(nodeRegistryAddr  != address(0), "NODE_REGISTRY not set");
        require(powVerifierAddr   != address(0), "POW_VERIFIER not set");
        require(escrowManagerAddr != address(0), "ESCROW_MANAGER not set");
        require(marketplaceAddr   != address(0), "TASK_MARKETPLACE not set");

        vm.startBroadcast(multisig);

        // ── 1. Accept Ownable2Step ownership ─────────────────────────────────
        IOwnable2Step marketplace = IOwnable2Step(marketplaceAddr);
        require(marketplace.pendingOwner() == multisig, "TaskMarketplace: not pendingOwner");
        marketplace.acceptOwnership();
        emit OwnershipAccepted("TaskMarketplace", multisig);

        IOwnable2Step powVerifier = IOwnable2Step(powVerifierAddr);
        require(powVerifier.pendingOwner() == multisig, "ProofOfWorkVerifier: not pendingOwner");
        powVerifier.acceptOwnership();
        emit OwnershipAccepted("ProofOfWorkVerifier", multisig);

        // ── 2. Accept AccessControlDefaultAdminRules admin transfer ───────────
        IDefaultAdminRules nodeRegistry = IDefaultAdminRules(nodeRegistryAddr);
        (address pendingNR,) = nodeRegistry.pendingDefaultAdmin();
        require(pendingNR == multisig, "NodeRegistry: not pendingDefaultAdmin");
        nodeRegistry.acceptDefaultAdminTransfer();
        emit DefaultAdminAccepted("NodeRegistry", multisig);

        IDefaultAdminRules escrowManager = IDefaultAdminRules(escrowManagerAddr);
        (address pendingEM,) = escrowManager.pendingDefaultAdmin();
        require(pendingEM == multisig, "EscrowManager: not pendingDefaultAdmin");
        escrowManager.acceptDefaultAdminTransfer();
        emit DefaultAdminAccepted("EscrowManager", multisig);

        emit FinalizeComplete(
            nodeRegistryAddr,
            powVerifierAddr,
            escrowManagerAddr,
            marketplaceAddr,
            multisig
        );

        vm.stopBroadcast();
    }
}
