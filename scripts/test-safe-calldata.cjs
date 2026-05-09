'use strict';

/**
 * test-safe-calldata.cjs — Smoke tests for compute-finalize-safe-calldata.cjs
 *
 * Tests the pure, chain-free functions:
 *   - validateEnvVars: env var detection
 *   - buildTransactions: calldata targets and selectors
 *
 * Run:  node scripts/test-safe-calldata.cjs
 * Exit: 0 on all pass, 1 on any failure
 */

const {
  validateEnvVars,
  buildTransactions,
  REQUIRED_ENV,
} = require('./compute-finalize-safe-calldata.cjs');

// ── Well-known 4-byte selectors ──────────────────────────────────────────────
// keccak256("acceptOwnership()")          → 0x79ba5097
// keccak256("acceptDefaultAdminTransfer()") → 0x634e93da
const ACCEPT_OWNERSHIP_SELECTOR       = '0x79ba5097';
const ACCEPT_DEFAULT_ADMIN_SELECTOR   = '0x634e93da';

// ── Minimal test harness ─────────────────────────────────────────────────────

let pass = 0;
let fail = 0;

function assert(condition, label) {
  if (condition) {
    process.stdout.write(`  \u2713 ${label}\n`);
    pass++;
  } else {
    process.stderr.write(`  \u2717 FAIL: ${label}\n`);
    fail++;
  }
}

function section(title) {
  process.stdout.write(`\n${title}\n`);
}

// ── Test 1: All env vars missing ─────────────────────────────────────────────

section('Test 1: All env vars missing');
{
  const result = validateEnvVars({});
  assert(result.ok === false, 'ok=false when all vars absent');
  assert(
    result.missing.length === REQUIRED_ENV.length,
    `reports all ${REQUIRED_ENV.length} missing vars (got ${result.missing.length})`
  );
  for (const key of REQUIRED_ENV) {
    assert(result.missing.includes(key), `includes ${key}`);
  }
}

// ── Test 2: Partial env vars ─────────────────────────────────────────────────

section('Test 2: Partial env vars (RPC_URL + CHAIN_ID present)');
{
  const env = { RPC_URL: 'http://localhost:8545', CHAIN_ID: '31337' };
  const result = validateEnvVars(env);
  assert(result.ok === false, 'ok=false when some vars missing');
  assert(!result.missing.includes('RPC_URL'), 'RPC_URL not in missing');
  assert(!result.missing.includes('CHAIN_ID'), 'CHAIN_ID not in missing');
  assert(result.missing.includes('MULTISIG'), 'MULTISIG in missing');
  assert(result.missing.includes('NODE_REGISTRY'), 'NODE_REGISTRY in missing');
  assert(result.missing.includes('TASK_MARKETPLACE'), 'TASK_MARKETPLACE in missing');
}

// ── Test 3: Empty-string values treated as missing ───────────────────────────

section('Test 3: Empty-string values are treated as missing');
{
  const env = {
    RPC_URL: '   ',  // only whitespace
    CHAIN_ID: '',
    MULTISIG: '0xabc',
    NODE_REGISTRY: '0xdef',
    POW_VERIFIER: '0x111',
    ESCROW_MANAGER: '0x222',
    TASK_MARKETPLACE: '0x333',
  };
  const result = validateEnvVars(env);
  assert(result.ok === false, 'ok=false when whitespace-only value present');
  assert(result.missing.includes('RPC_URL'), 'whitespace-only RPC_URL is missing');
  assert(result.missing.includes('CHAIN_ID'), 'empty CHAIN_ID is missing');
  assert(!result.missing.includes('MULTISIG'), 'non-empty MULTISIG is not missing');
}

// ── Test 4: All env vars present ─────────────────────────────────────────────

section('Test 4: All env vars present');
{
  const fullEnv = {
    RPC_URL: 'http://localhost:8545',
    CHAIN_ID: '31337',
    MULTISIG:          '0x1000000000000000000000000000000000000001',
    NODE_REGISTRY:     '0x1000000000000000000000000000000000000002',
    POW_VERIFIER:      '0x1000000000000000000000000000000000000003',
    ESCROW_MANAGER:    '0x1000000000000000000000000000000000000004',
    TASK_MARKETPLACE:  '0x1000000000000000000000000000000000000005',
  };
  const result = validateEnvVars(fullEnv);
  assert(result.ok === true, 'ok=true when all vars present');
  assert(result.missing.length === 0, 'missing array is empty');
}

// ── Test 5: buildTransactions — correct count and targets ────────────────────

section('Test 5: buildTransactions — count, targets, value');
{
  // Stub encodeFunctionData to return selectors based on functionName
  function encodeFnStub({ functionName }) {
    if (functionName === 'acceptOwnership')          return ACCEPT_OWNERSHIP_SELECTOR;
    if (functionName === 'acceptDefaultAdminTransfer') return ACCEPT_DEFAULT_ADMIN_SELECTOR;
    throw new Error(`Unexpected functionName: ${functionName}`);
  }

  const addrs = {
    marketplace:   '0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
    powVerifier:   '0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
    nodeRegistry:  '0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
    escrowManager: '0xDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD',
  };

  const txs = buildTransactions(addrs, encodeFnStub);

  assert(txs.length === 4, 'generates exactly 4 transactions');
  assert(txs[0].to === addrs.marketplace,   'tx[0] targets TaskMarketplace');
  assert(txs[1].to === addrs.powVerifier,   'tx[1] targets ProofOfWorkVerifier');
  assert(txs[2].to === addrs.nodeRegistry,  'tx[2] targets NodeRegistry');
  assert(txs[3].to === addrs.escrowManager, 'tx[3] targets EscrowManager');
  assert(txs.every((tx) => tx.value === '0'), 'all tx have value="0"');
  assert(txs.every((tx) => tx.contractInputsValues === null), 'contractInputsValues=null for all');
}

// ── Test 6: buildTransactions — correct selectors ────────────────────────────

section('Test 6: buildTransactions — correct 4-byte selectors');
{
  function encodeFnStub({ functionName }) {
    if (functionName === 'acceptOwnership')          return ACCEPT_OWNERSHIP_SELECTOR;
    if (functionName === 'acceptDefaultAdminTransfer') return ACCEPT_DEFAULT_ADMIN_SELECTOR;
    throw new Error(`Unexpected functionName: ${functionName}`);
  }

  const addrs = {
    marketplace:   '0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
    powVerifier:   '0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
    nodeRegistry:  '0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
    escrowManager: '0xDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD',
  };

  const txs = buildTransactions(addrs, encodeFnStub);

  assert(txs[0].data === ACCEPT_OWNERSHIP_SELECTOR,     'tx[0] data = acceptOwnership selector');
  assert(txs[1].data === ACCEPT_OWNERSHIP_SELECTOR,     'tx[1] data = acceptOwnership selector');
  assert(txs[2].data === ACCEPT_DEFAULT_ADMIN_SELECTOR, 'tx[2] data = acceptDefaultAdminTransfer selector');
  assert(txs[3].data === ACCEPT_DEFAULT_ADMIN_SELECTOR, 'tx[3] data = acceptDefaultAdminTransfer selector');
  assert(txs[0].contractMethod.name === 'acceptOwnership',          'tx[0] contractMethod.name');
  assert(txs[1].contractMethod.name === 'acceptOwnership',          'tx[1] contractMethod.name');
  assert(txs[2].contractMethod.name === 'acceptDefaultAdminTransfer', 'tx[2] contractMethod.name');
  assert(txs[3].contractMethod.name === 'acceptDefaultAdminTransfer', 'tx[3] contractMethod.name');
}

// ── Test 7: buildTransactions — contractMethod shape ────────────────────────

section('Test 7: buildTransactions — contractMethod shape');
{
  function encodeFnStub({ functionName }) {
    if (functionName === 'acceptOwnership')            return ACCEPT_OWNERSHIP_SELECTOR;
    if (functionName === 'acceptDefaultAdminTransfer') return ACCEPT_DEFAULT_ADMIN_SELECTOR;
    throw new Error(`Unexpected functionName: ${functionName}`);
  }
  const addrs = {
    marketplace:   '0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
    powVerifier:   '0xBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB',
    nodeRegistry:  '0xCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC',
    escrowManager: '0xDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD',
  };
  const txs = buildTransactions(addrs, encodeFnStub);
  assert(
    txs.every((tx) => Array.isArray(tx.contractMethod.inputs) && tx.contractMethod.inputs.length === 0),
    'all contractMethod.inputs are empty arrays'
  );
  assert(
    txs.every((tx) => tx.contractMethod.payable === false),
    'all contractMethod.payable=false'
  );
}

// ── Summary ──────────────────────────────────────────────────────────────────

process.stdout.write(`\nResults: ${pass} passed, ${fail} failed\n`);
if (fail > 0) {
  process.exit(1);
}
