// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// Minimal deploy script — no forge-std dependency required.
// Run with:
//   forge script script/DeployComputeMvp.s.sol:DeployComputeMvp \
//     --rpc-url $RPC_URL --broadcast --sender $DEPLOYER --private-key $PRIVATE_KEY
//
// Or with a local Anvil node:
//   forge script script/DeployComputeMvp.s.sol:DeployComputeMvp \
//     --rpc-url http://localhost:8545 --broadcast

import {NodeRegistry} from "../contracts/NodeRegistry.sol";
import {ProofOfWorkVerifier} from "../contracts/ProofOfWorkVerifier.sol";
import {EscrowManager} from "../contracts/EscrowManager.sol";
import {TaskMarketplace} from "../contracts/TaskMarketplace.sol";

/// @dev Minimal VM cheatcode interface (subset needed for scripting)
interface IVm {
    function startBroadcast() external;
    function startBroadcast(address sender) external;
    function stopBroadcast() external;
    function envAddress(string calldata name) external view returns (address value);
    function envOr(string calldata name, address defaultValue) external returns (address value);
    function addr(uint256 privateKey) external pure returns (address keyAddr);
}

/// @dev Minimal console — just log bytes32 and address pairs via events for on-chain queryability.
///      Off-chain, use `cast run <txhash> --verbose` to inspect deployment logs.
contract DeployComputeMvp {
    IVm private constant vm = IVm(address(uint160(uint256(keccak256("hevm cheat code")))));

    event Deployed(string name, address addr);
    event RoleGranted(string role, address grantee);
    event ReferenceSet(string ref, address contract_);
    event DeployComplete(
        address nodeRegistry,
        address powVerifier,
        address escrowManager,
        address taskMarketplace,
        address deployer,
        address treasury
    );
    /// @dev Emitted when Ownable2Step.transferOwnership() has been called.
    ///      The pending owner must call acceptOwnership() to complete the handoff.
    event PendingOwnerSet(string contractName, address pendingOwner);
    /// @dev Emitted when AccessControlDefaultAdminRules.beginDefaultAdminTransfer() has been called.
    ///      The pending admin must call acceptDefaultAdminTransfer() after the delay.
    event PendingAdminSet(string contractName, address pendingAdmin);

    function run() external {
        // ── Configuration ────────────────────────────────────────────────────
        // DEPLOYER and TREASURY can be set via environment variables.
        // If TREASURY is not set, deployer is used as treasury.
        // MULTISIG: optional address of a multisig / DAO that should own contracts.
        //   If set, this script will begin the 2-step ownership transfer so the
        //   multisig only needs to call acceptOwnership / acceptDefaultAdminTransfer.
        //   If not set, deployer remains the permanent owner (acceptable for local testing).
        address deployer = vm.envOr("DEPLOYER", msg.sender);
        address treasury = vm.envOr("TREASURY", deployer);
        address multisig = vm.envOr("MULTISIG", deployer);

        vm.startBroadcast(deployer);

        // ── 1. Deploy NodeRegistry ────────────────────────────────────────────
        // Uses AccessControlDefaultAdminRules; deployer becomes defaultAdmin.
        // adminDelay = 0 for MVP (no timelock); set to > 0 for production safety.
        NodeRegistry nodeRegistry = new NodeRegistry(deployer, treasury, 0);
        emit Deployed("NodeRegistry", address(nodeRegistry));

        // ── 2. Deploy ProofOfWorkVerifier ─────────────────────────────────────
        // Needs NodeRegistry address at construction; owns Ownable2Step.
        ProofOfWorkVerifier powVerifier = new ProofOfWorkVerifier(deployer, address(nodeRegistry));
        emit Deployed("ProofOfWorkVerifier", address(powVerifier));

        // ── 3. Deploy EscrowManager ───────────────────────────────────────────
        // Uses AccessControlDefaultAdminRules; deployer becomes defaultAdmin.
        EscrowManager escrowManager = new EscrowManager(deployer, treasury, 0);
        emit Deployed("EscrowManager", address(escrowManager));

        // ── 4. Deploy TaskMarketplace ─────────────────────────────────────────
        // Ownable2Step; deployer is owner. Inherits default pricing from constructor.
        TaskMarketplace marketplace = new TaskMarketplace(deployer);
        emit Deployed("TaskMarketplace", address(marketplace));

        // ── 5. Wire contract references ───────────────────────────────────────
        // NodeRegistry: grant TASK_MANAGER_ROLE to marketplace (done inside setTaskMarketplace)
        nodeRegistry.setTaskMarketplace(address(marketplace));
        emit ReferenceSet("NodeRegistry.taskMarketplace", address(marketplace));

        // EscrowManager: grant RELEASER_ROLE to marketplace (done inside setTaskMarketplace)
        escrowManager.setTaskMarketplace(address(marketplace));
        emit ReferenceSet("EscrowManager.taskMarketplace", address(marketplace));

        // TaskMarketplace: set its three contract references
        marketplace.setNodeRegistry(address(nodeRegistry));
        emit ReferenceSet("TaskMarketplace.nodeRegistry", address(nodeRegistry));

        marketplace.setEscrowManager(address(escrowManager));
        emit ReferenceSet("TaskMarketplace.escrowManager", address(escrowManager));

        marketplace.setPowVerifier(address(powVerifier));
        emit ReferenceSet("TaskMarketplace.powVerifier", address(powVerifier));

        // ── 6. Grant UPDATER_ROLE on NodeRegistry to ProofOfWorkVerifier ─────
        // Required so that submitSolution can call updateNodeStatus / updateComputePower.
        nodeRegistry.grantRole(nodeRegistry.UPDATER_ROLE(), address(powVerifier));
        emit RoleGranted("UPDATER_ROLE -> ProofOfWorkVerifier", address(powVerifier));

        // ── 7. Initiate ownership / admin transfer to multisig ────────────────
        // Ownable2Step (TaskMarketplace, ProofOfWorkVerifier):
        //   Step 1 (here):      owner calls transferOwnership(multisig)
        //   Step 2 (multisig):  pendingOwner calls acceptOwnership()
        //
        // AccessControlDefaultAdminRules (NodeRegistry, EscrowManager):
        //   Step 1 (here):      admin calls beginDefaultAdminTransfer(multisig)
        //   Step 2 (multisig):  pendingAdmin calls acceptDefaultAdminTransfer() after delay
        //
        // If MULTISIG == deployer, no events are emitted and the deployer keeps ownership
        // (safe for local test environments).
        if (multisig != deployer) {
            marketplace.transferOwnership(multisig);
            emit PendingOwnerSet("TaskMarketplace", multisig);

            powVerifier.transferOwnership(multisig);
            emit PendingOwnerSet("ProofOfWorkVerifier", multisig);

            nodeRegistry.beginDefaultAdminTransfer(multisig);
            emit PendingAdminSet("NodeRegistry", multisig);

            escrowManager.beginDefaultAdminTransfer(multisig);
            emit PendingAdminSet("EscrowManager", multisig);
        }

        emit DeployComplete(
            address(nodeRegistry),
            address(powVerifier),
            address(escrowManager),
            address(marketplace),
            deployer,
            treasury
        );

        vm.stopBroadcast();
    }
}
