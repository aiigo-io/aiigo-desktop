'use strict';

/**
 * compute-finalize-safe-calldata.cjs
 *
 * Reads on-chain state and generates a Gnosis Safe Transaction Builder JSON
 * with 4 transactions for accepting pending ownership / admin transfers that
 * were initiated by DeployComputeMvp.s.sol.
 *
 * This script does NOT broadcast, sign, or interact with the Safe API.
 * JSON is written to stdout only — no files are created.
 *
 * Usage:
 *   node scripts/compute-finalize-safe-calldata.cjs
 *   npm run finalize:compute:safe-calldata
 *   npm run finalize:compute:safe-calldata > safe-finalize-compute.json
 *
 * After generating, import the JSON into Safe Transaction Builder:
 *   https://app.safe.global → Apps → Transaction Builder → Load JSON
 *
 * Required env vars:
 *   RPC_URL          e.g. https://mainnet.infura.io/v3/YOUR_KEY
 *   CHAIN_ID         e.g. 1 (mainnet), 11155111 (sepolia), 31337 (anvil)
 *   MULTISIG         Gnosis Safe address that should receive ownership
 *   NODE_REGISTRY    NodeRegistry contract address
 *   POW_VERIFIER     ProofOfWorkVerifier contract address
 *   ESCROW_MANAGER   EscrowManager contract address
 *   TASK_MARKETPLACE TaskMarketplace contract address
 */

// ── Required env var list ────────────────────────────────────────────────────

const REQUIRED_ENV = [
  'RPC_URL',
  'CHAIN_ID',
  'MULTISIG',
  'NODE_REGISTRY',
  'POW_VERIFIER',
  'ESCROW_MANAGER',
  'TASK_MARKETPLACE',
];

// ── ABI fragments ────────────────────────────────────────────────────────────

const OWNABLE2STEP_ABI = [
  {
    name: 'pendingOwner',
    type: 'function',
    stateMutability: 'view',
    inputs: [],
    outputs: [{ type: 'address' }],
  },
  {
    name: 'acceptOwnership',
    type: 'function',
    stateMutability: 'nonpayable',
    inputs: [],
    outputs: [],
  },
];

const DEFAULT_ADMIN_ABI = [
  {
    name: 'pendingDefaultAdmin',
    type: 'function',
    stateMutability: 'view',
    inputs: [],
    outputs: [
      { name: 'newAdmin', type: 'address' },
      { name: 'schedule', type: 'uint48' },
    ],
  },
  {
    name: 'acceptDefaultAdminTransfer',
    type: 'function',
    stateMutability: 'nonpayable',
    inputs: [],
    outputs: [],
  },
];

// ── Pure helpers (exported for testing) ─────────────────────────────────────

/**
 * Validates that all required env vars are present and non-empty.
 * @param {Record<string, string|undefined>} env
 * @returns {{ ok: boolean, missing: string[] }}
 */
function validateEnvVars(env) {
  const missing = REQUIRED_ENV.filter((k) => !env[k] || env[k].trim() === '');
  return { ok: missing.length === 0, missing };
}

/**
 * Builds the 4 Safe transaction objects (calldata only, no broadcast).
 * @param {{ marketplace: string, powVerifier: string, nodeRegistry: string, escrowManager: string }} addresses
 * @param {Function} encodeFn - viem's encodeFunctionData (or a test stub)
 * @returns {object[]}
 */
function buildTransactions(addresses, encodeFn) {
  const { marketplace, powVerifier, nodeRegistry, escrowManager } = addresses;
  return [
    {
      to: marketplace,
      value: '0',
      data: encodeFn({ abi: OWNABLE2STEP_ABI, functionName: 'acceptOwnership' }),
      contractMethod: { inputs: [], name: 'acceptOwnership', payable: false },
      contractInputsValues: null,
    },
    {
      to: powVerifier,
      value: '0',
      data: encodeFn({ abi: OWNABLE2STEP_ABI, functionName: 'acceptOwnership' }),
      contractMethod: { inputs: [], name: 'acceptOwnership', payable: false },
      contractInputsValues: null,
    },
    {
      to: nodeRegistry,
      value: '0',
      data: encodeFn({ abi: DEFAULT_ADMIN_ABI, functionName: 'acceptDefaultAdminTransfer' }),
      contractMethod: { inputs: [], name: 'acceptDefaultAdminTransfer', payable: false },
      contractInputsValues: null,
    },
    {
      to: escrowManager,
      value: '0',
      data: encodeFn({ abi: DEFAULT_ADMIN_ABI, functionName: 'acceptDefaultAdminTransfer' }),
      contractMethod: { inputs: [], name: 'acceptDefaultAdminTransfer', payable: false },
      contractInputsValues: null,
    },
  ];
}

// ── Main (chain-connected path) ──────────────────────────────────────────────

async function main() {
  // 1. Validate env vars
  const envCheck = validateEnvVars(process.env);
  if (!envCheck.ok) {
    process.stderr.write('Missing required environment variables:\n');
    envCheck.missing.forEach((k) => process.stderr.write(`  - ${k}\n`));
    process.exit(1);
  }

  const { RPC_URL, CHAIN_ID, MULTISIG, NODE_REGISTRY, POW_VERIFIER, ESCROW_MANAGER, TASK_MARKETPLACE } =
    process.env;

  // 2. Lazy-import viem (ESM package, must use dynamic import in CJS)
  const { createPublicClient, http, encodeFunctionData, getAddress } = await import('viem');

  // Normalize to checksummed addresses; fail early on bad input
  let multisig, nodeRegistry, powVerifier, escrowManager, marketplace;
  try {
    multisig      = getAddress(MULTISIG);
    nodeRegistry  = getAddress(NODE_REGISTRY);
    powVerifier   = getAddress(POW_VERIFIER);
    escrowManager = getAddress(ESCROW_MANAGER);
    marketplace   = getAddress(TASK_MARKETPLACE);
  } catch (e) {
    process.stderr.write(`Invalid address in env vars: ${e.message}\n`);
    process.exit(1);
  }

  const chainIdNum = parseInt(CHAIN_ID, 10);
  if (isNaN(chainIdNum) || chainIdNum <= 0) {
    process.stderr.write(`CHAIN_ID must be a positive integer, got: ${CHAIN_ID}\n`);
    process.exit(1);
  }

  // 3. Create read-only viem client (no private key, no signing)
  const client = createPublicClient({
    transport: http(RPC_URL),
    chain: {
      id: chainIdNum,
      name: `chain-${chainIdNum}`,
      nativeCurrency: { name: 'ETH', symbol: 'ETH', decimals: 18 },
      rpcUrls: { default: { http: [RPC_URL] } },
    },
  });

  // 4. Read and validate pending states from chain
  process.stderr.write('Verifying chain state...\n');
  const errors = [];

  // TaskMarketplace: Ownable2Step
  const pendingTM = await client
    .readContract({ address: marketplace, abi: OWNABLE2STEP_ABI, functionName: 'pendingOwner' })
    .catch((e) => {
      errors.push(`TaskMarketplace.pendingOwner() call failed: ${e.message}`);
      return null;
    });

  if (pendingTM !== null && pendingTM.toLowerCase() !== multisig.toLowerCase()) {
    errors.push(`TaskMarketplace.pendingOwner() = ${pendingTM}, expected ${multisig}`);
  }

  // ProofOfWorkVerifier: Ownable2Step
  const pendingPOW = await client
    .readContract({ address: powVerifier, abi: OWNABLE2STEP_ABI, functionName: 'pendingOwner' })
    .catch((e) => {
      errors.push(`ProofOfWorkVerifier.pendingOwner() call failed: ${e.message}`);
      return null;
    });

  if (pendingPOW !== null && pendingPOW.toLowerCase() !== multisig.toLowerCase()) {
    errors.push(`ProofOfWorkVerifier.pendingOwner() = ${pendingPOW}, expected ${multisig}`);
  }

  // Fetch latest block timestamp for schedule checks
  const latestBlock = await client
    .getBlock({ blockTag: 'latest' })
    .catch((e) => {
      errors.push(`getBlock(latest) failed: ${e.message}`);
      return null;
    });
  const nowTs = latestBlock ? Number(latestBlock.timestamp) : null;

  // NodeRegistry: AccessControlDefaultAdminRules
  const pendingNR = await client
    .readContract({ address: nodeRegistry, abi: DEFAULT_ADMIN_ABI, functionName: 'pendingDefaultAdmin' })
    .catch((e) => {
      errors.push(`NodeRegistry.pendingDefaultAdmin() call failed: ${e.message}`);
      return null;
    });

  if (pendingNR !== null) {
    const [newAdminNR, scheduleNR] = pendingNR;
    if (newAdminNR.toLowerCase() !== multisig.toLowerCase()) {
      errors.push(`NodeRegistry.pendingDefaultAdmin().newAdmin = ${newAdminNR}, expected ${multisig}`);
    }
    if (nowTs !== null && Number(scheduleNR) > nowTs) {
      errors.push(
        `NodeRegistry: transfer schedule not yet reached (schedule=${scheduleNR}, now=${nowTs})`
      );
    }
  }

  // EscrowManager: AccessControlDefaultAdminRules
  const pendingEM = await client
    .readContract({ address: escrowManager, abi: DEFAULT_ADMIN_ABI, functionName: 'pendingDefaultAdmin' })
    .catch((e) => {
      errors.push(`EscrowManager.pendingDefaultAdmin() call failed: ${e.message}`);
      return null;
    });

  if (pendingEM !== null) {
    const [newAdminEM, scheduleEM] = pendingEM;
    if (newAdminEM.toLowerCase() !== multisig.toLowerCase()) {
      errors.push(`EscrowManager.pendingDefaultAdmin().newAdmin = ${newAdminEM}, expected ${multisig}`);
    }
    if (nowTs !== null && Number(scheduleEM) > nowTs) {
      errors.push(
        `EscrowManager: transfer schedule not yet reached (schedule=${scheduleEM}, now=${nowTs})`
      );
    }
  }

  if (errors.length > 0) {
    process.stderr.write('\nChain state validation FAILED:\n');
    errors.forEach((e) => process.stderr.write(`  ✗ ${e}\n`));
    process.exit(1);
  }

  process.stderr.write('Chain state OK — all pending transfers verified.\n\n');

  // 5. Build transactions using real viem encodeFunctionData
  const txs = buildTransactions(
    { marketplace, powVerifier, nodeRegistry, escrowManager },
    encodeFunctionData
  );

  // Print human-readable summary to stderr (so stdout stays clean JSON)
  const labels = [
    'TaskMarketplace.acceptOwnership()',
    'ProofOfWorkVerifier.acceptOwnership()',
    'NodeRegistry.acceptDefaultAdminTransfer()',
    'EscrowManager.acceptDefaultAdminTransfer()',
  ];
  process.stderr.write('Transactions:\n');
  txs.forEach((tx, i) => {
    process.stderr.write(`  [${i + 1}] ${labels[i]}\n`);
    process.stderr.write(`       to:   ${tx.to}\n`);
    process.stderr.write(`       data: ${tx.data}\n`);
  });
  process.stderr.write('\n');

  // 6. Output Safe Transaction Builder JSON to stdout ONLY (caller redirects to file)
  const safeJson = {
    version: '1.0',
    chainId: CHAIN_ID,
    createdAt: Date.now(),
    meta: {
      name: 'Finalize Compute MVP — Accept Admin/Ownership Transfers',
      description:
        'Accepts the pending ownership (Ownable2Step) and default admin ' +
        '(AccessControlDefaultAdminRules) transfers initiated by DeployComputeMvp.s.sol. ' +
        'Import this JSON into Safe Transaction Builder to execute.',
      txBuilderVersion: '1.16.5',
      createdFromSafeAddress: multisig,
      createdFromOwnerAddress: '',
    },
    transactions: txs,
  };

  process.stdout.write(JSON.stringify(safeJson, null, 2) + '\n');
  process.stderr.write('Done. Redirect stdout to a file and import into Safe Transaction Builder.\n');
  process.stderr.write(
    '  npm run finalize:compute:safe-calldata > safe-finalize-compute.json\n'
  );
}

// ── Exports (for testing) ────────────────────────────────────────────────────

module.exports = {
  validateEnvVars,
  buildTransactions,
  OWNABLE2STEP_ABI,
  DEFAULT_ADMIN_ABI,
  REQUIRED_ENV,
};

// ── Entry point ──────────────────────────────────────────────────────────────

if (require.main === module) {
  main().catch((e) => {
    process.stderr.write(`Fatal: ${e.message}\n`);
    process.exit(1);
  });
}
