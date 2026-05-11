/**
 * useComputeMarketplace — pure Tauri backend boundary.
 *
 * Zero viem / ABI / direct-RPC code in this file.
 * All on-chain operations are delegated to the Rust compute module
 * via Tauri commands. The hook exposes the same public interface
 * that Compute UI components depend on.
 */

import { useEffect, useRef, useState } from 'react';
import { invoke, isTauriUnavailableError } from '@/lib/tauri';

// ── Public display types (unchanged API surface for UI components) ───────────

export type ComputeTab = 'marketplace' | 'provider' | 'buyer' | 'governance';
export type ResourceType = 'GPU' | 'CPU' | 'Network' | 'Mobile' | 'IoT';
export type NodeStatus = 'Pending' | 'Verified' | 'Active' | 'Inactive' | 'Slashed';
export type TaskStatus =
  | 'Open'
  | 'Assigned'
  | 'InProgress'
  | 'Completed'
  | 'Verified'
  | 'Disputed'
  | 'Cancelled';

export interface ComputeWallet {
  id: string;
  label: string;
  address: string;
}

export interface ComputeNode {
  nodeId: string;
  owner: string;
  status: NodeStatus;
  resourceType: ResourceType;
  computePower: number;
  stakedAmountEth: number;
  reputation: number;
  totalTasksCompleted: number;
  totalEarningsEth: number;
  registeredAt: number;
  lastActiveAt: number;
  metadataUri: string;
  metadata: { gpuModel: string; region: string };
  trustLevel: number;
  pendingTaskCount: number;
}

export interface ComputeTask {
  taskId: string;
  buyer: string;
  assignedNodeId: string | null;
  resourceType: ResourceType;
  requiredPower: number;
  durationSeconds: number;
  maxPriceEth: number;
  escrowAmountEth: number;
  createdAt: number;
  startedAt: number | null;
  completedAt: number | null;
  status: TaskStatus;
  minTrustLevel: number;
  specificationUri: string;
  specTitle: string;
  challengeDeadline: number | null;
  disputeReason: string | null;
  disputedBy: string | null;
  resolved: boolean;
  resolvedBy: string | null;
  grossProviderAmountEth: number;
}

export interface ComputeSyncOutcome {
  status: 'fresh' | 'stale' | 'partial' | 'unavailable';
  partial: boolean;
  failedSources: string[];
  syncedToBlock: number | null;
  confirmedHeadBlock: number | null;
  updatedAt: string | null;
}

export interface ComputeContractsConfig {
  chainId: number;
  chainName: string;
  rpcUrl: string;
  taskMarketplaceAddress: string | null;
  nodeRegistryAddress: string | null;
  escrowManagerAddress: string | null;
  isConfigured: boolean;
  missing: string[];
}

export interface ComputeSnapshot {
  openTasks: ComputeTask[];
  buyerTasks: ComputeTask[];
  providerTasks: ComputeTask[];
  disputes: ComputeTask[];
  ownedNodes: ComputeNode[];
  activeNodes: ComputeNode[];
  pendingBuyerRefundEth: number;
  pendingProviderPayoutEth: number;
  totalLockedEscrowEth: number;
  lastRefreshedAt: number | null;
}

// ── Tauri backend types (snake_case enums, camelCase structs) ──────────────

// Matches ComputeConfigResponse from Rust (no rename_all = snake_case)
interface BackendConfig {
  chain_id: number | null;
  chain_name: string | null;
  rpc_url: string | null;
  task_marketplace_address: string | null;
  node_registry_address: string | null;
  escrow_manager_address: string | null;
  confirmation_depth: number | null;
  bootstrap_start_block: number | null;
  is_configured: boolean;
  missing: string[];
  warnings: string[];
}

// Matches ComputeNode from Rust with rename_all = "camelCase"
interface BackendNode {
  nodeId: string;
  owner: string;
  status: string;
  resourceType: string;
  computePower: string;
  stakedAmountWei: string;
  reputation: string;
  totalTasksCompleted: number;
  totalEarningsWei: string;
  registeredAt: number | null;
  lastActiveAt: number | null;
  metadataUri: string;
  trustLevel: number;
  pendingTaskCount: number;
}

// Matches ComputeTask from Rust with rename_all = "camelCase"
interface BackendTask {
  taskId: string;
  buyer: string;
  assignedNodeId: string | null;
  resourceType: string;
  requiredPower: string;
  durationSeconds: number;
  maxPriceWei: string;
  escrowAmountWei: string;
  status: string;
  specificationUri: string;
  minTrustLevel: number;
  createdAt: number | null;
  startedAt: number | null;
  completedAt: number | null;
  challengeDeadline: number | null;
  disputeReason: string | null;
  disputedBy: string | null;
  resolved: boolean;
  resolvedBy: string | null;
  grossProviderAmountWei: string;
}

// Matches ComputeSnapshot from Rust with rename_all = "camelCase"
interface BackendSnapshot {
  openTasks: BackendTask[];
  buyerTasks: BackendTask[];
  providerTasks: BackendTask[];
  disputes: BackendTask[];
  ownedNodes: BackendNode[];
  activeNodes: BackendNode[];
  pendingBuyerRefundWei: string;
  pendingProviderPayoutWei: string;
  totalLockedEscrowWei: string;
}

// ComputeSyncOutcome from Rust (no rename_all — snake_case keys)
interface BackendSyncOutcome {
  status: string; // 'fresh' | 'stale' | 'partial' | 'unavailable'
  coverage: string;
  partial: boolean;
  failed_sources: string[];
  synced_to_block: number | null;
  confirmed_head_block: number | null;
  updated_at: string | null;
}

interface BackendSnapshotResponse {
  snapshot: BackendSnapshot;
  sync: BackendSyncOutcome;
}

export interface ComputeMutationResponse {
  mutationId: string;
  walletId: string;
  clientRequestId: string;
  requestHash: string;
  status: string;
  action: string;
  currentStep: string | null;
  txHash: string | null;
  taskId: string | null;
  nodeId: string | null;
  error: string | null;
  createdAt: string;
  updatedAt: string;
}

interface WalletInfo {
  id: string;
  label: string;
  address: string;
}

// ── Conversion helpers ───────────────────────────────────────────────────────

const RESOURCE_TYPES: ResourceType[] = ['GPU', 'CPU', 'Network', 'Mobile', 'IoT'];
const NODE_STATUSES: NodeStatus[] = ['Pending', 'Verified', 'Active', 'Inactive', 'Slashed'];
const TASK_STATUSES: TaskStatus[] = [
  'Open',
  'Assigned',
  'InProgress',
  'Completed',
  'Verified',
  'Disputed',
  'Cancelled',
];

function weiToEth(wei: string): number {
  try {
    const weiInt = BigInt(wei);
    return Number(weiInt) / 1e18;
  } catch {
    return 0;
  }
}

function parseMetadata(metadataUri: string) {
  try {
    const parsed = JSON.parse(metadataUri) as { gpuModel?: string; region?: string };
    return { gpuModel: parsed.gpuModel ?? 'Unspecified', region: parsed.region ?? 'Unknown' };
  } catch {
    return { gpuModel: metadataUri || 'Unspecified', region: 'Unknown' };
  }
}

function parseTaskTitle(specificationUri: string) {
  try {
    const parsed = JSON.parse(specificationUri) as { title?: string };
    return parsed.title ?? specificationUri;
  } catch {
    return specificationUri;
  }
}

function toResourceType(value: string): ResourceType {
  // Backend sends enum variants like "GPU", "CPU"
  const idx = RESOURCE_TYPES.indexOf(value as ResourceType);
  return idx >= 0 ? RESOURCE_TYPES[idx] : 'GPU';
}

function toNodeStatus(value: string): NodeStatus {
  const idx = NODE_STATUSES.indexOf(value as NodeStatus);
  return idx >= 0 ? NODE_STATUSES[idx] : 'Pending';
}

function toTaskStatus(value: string): TaskStatus {
  const idx = TASK_STATUSES.indexOf(value as TaskStatus);
  return idx >= 0 ? TASK_STATUSES[idx] : 'Open';
}

function backendNodeToDisplay(node: BackendNode): ComputeNode {
  const metadata = parseMetadata(node.metadataUri);
  return {
    nodeId: node.nodeId,
    owner: node.owner,
    status: toNodeStatus(node.status),
    resourceType: toResourceType(node.resourceType),
    computePower: Number(node.computePower),
    stakedAmountEth: weiToEth(node.stakedAmountWei),
    reputation: Number(node.reputation),
    totalTasksCompleted: node.totalTasksCompleted,
    totalEarningsEth: weiToEth(node.totalEarningsWei),
    registeredAt: node.registeredAt ?? 0,
    lastActiveAt: node.lastActiveAt ?? 0,
    metadataUri: node.metadataUri,
    metadata,
    trustLevel: node.trustLevel,
    pendingTaskCount: node.pendingTaskCount,
  };
}

function backendTaskToDisplay(task: BackendTask): ComputeTask {
  return {
    taskId: task.taskId,
    buyer: task.buyer,
    assignedNodeId: task.assignedNodeId,
    resourceType: toResourceType(task.resourceType),
    requiredPower: Number(task.requiredPower),
    durationSeconds: task.durationSeconds,
    maxPriceEth: weiToEth(task.maxPriceWei),
    escrowAmountEth: weiToEth(task.escrowAmountWei),
    createdAt: task.createdAt ?? 0,
    startedAt: task.startedAt,
    completedAt: task.completedAt,
    status: toTaskStatus(task.status),
    minTrustLevel: task.minTrustLevel,
    specificationUri: task.specificationUri,
    specTitle: parseTaskTitle(task.specificationUri),
    challengeDeadline: task.challengeDeadline,
    disputeReason: task.disputeReason,
    disputedBy: task.disputedBy,
    resolved: task.resolved,
    resolvedBy: task.resolvedBy,
    grossProviderAmountEth: weiToEth(task.grossProviderAmountWei),
  };
}

function backendSnapshotToDisplay(
  backend: BackendSnapshot,
  lastRefreshedAt: number | null,
): ComputeSnapshot {
  return {
    openTasks: backend.openTasks.map(backendTaskToDisplay),
    buyerTasks: backend.buyerTasks.map(backendTaskToDisplay),
    providerTasks: backend.providerTasks.map(backendTaskToDisplay),
    disputes: backend.disputes.map(backendTaskToDisplay),
    ownedNodes: backend.ownedNodes.map(backendNodeToDisplay),
    activeNodes: backend.activeNodes.map(backendNodeToDisplay),
    pendingBuyerRefundEth: weiToEth(backend.pendingBuyerRefundWei),
    pendingProviderPayoutEth: weiToEth(backend.pendingProviderPayoutWei),
    totalLockedEscrowEth: weiToEth(backend.totalLockedEscrowWei),
    lastRefreshedAt,
  };
}

function backendConfigToDisplay(cfg: BackendConfig): ComputeContractsConfig {
  return {
    chainId: cfg.chain_id ?? 0,
    chainName: cfg.chain_name ?? 'Unknown',
    rpcUrl: cfg.rpc_url ?? '',
    taskMarketplaceAddress: cfg.task_marketplace_address,
    nodeRegistryAddress: cfg.node_registry_address,
    escrowManagerAddress: cfg.escrow_manager_address,
    isConfigured: cfg.is_configured,
    missing: cfg.missing,
  };
}

// ── Error normalization ──────────────────────────────────────────────────────

function normalizeError(error: unknown): string {
  if (isTauriUnavailableError(error)) {
    return 'Tauri runtime is unavailable. Start the desktop app to use compute marketplace actions.';
  }
  if (error instanceof Error) return error.message;
  return String(error);
}

// ── Constants ────────────────────────────────────────────────────────────────

const emptySnapshot: ComputeSnapshot = {
  openTasks: [],
  buyerTasks: [],
  providerTasks: [],
  disputes: [],
  ownedNodes: [],
  activeNodes: [],
  pendingBuyerRefundEth: 0,
  pendingProviderPayoutEth: 0,
  totalLockedEscrowEth: 0,
  lastRefreshedAt: null,
};

const unconfiguredConfig: ComputeContractsConfig = {
  chainId: 0,
  chainName: 'Unknown',
  rpcUrl: '',
  taskMarketplaceAddress: null,
  nodeRegistryAddress: null,
  escrowManagerAddress: null,
  isConfigured: false,
  missing: ['compute_backend_not_loaded'],
};

// ── Hook ─────────────────────────────────────────────────────────────────────

export function useComputeMarketplace() {
  const [activeTab, setActiveTab] = useState<ComputeTab>('marketplace');
  const [wallets, setWallets] = useState<ComputeWallet[]>([]);
  const [selectedWalletId, setSelectedWalletId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<ComputeSnapshot>(emptySnapshot);
  const [config, setConfig] = useState<ComputeContractsConfig>(unconfiguredConfig);
  const [syncOutcome, setSyncOutcome] = useState<ComputeSyncOutcome | null>(null);
  const [isLoadingWallets, setIsLoadingWallets] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [pendingActionLabel, setPendingActionLabel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Stable request IDs per action key — allows idempotent retry without creating a new mutation.
  const pendingRequestIds = useRef<Map<string, string>>(new Map());

  function getOrCreateRequestId(key: string): string {
    if (!pendingRequestIds.current.has(key)) {
      pendingRequestIds.current.set(key, crypto.randomUUID());
    }
    return pendingRequestIds.current.get(key)!;
  }

  const selectedWallet = wallets.find((w) => w.id === selectedWalletId) ?? null;

  // Load config from Tauri backend (env-driven, validated server-side)
  async function loadConfig() {
    try {
      const cfg = await invoke<BackendConfig>('compute_get_config');
      setConfig(backendConfigToDisplay(cfg));
    } catch {
      // Keep unconfigured default; don't surface config errors as user-visible errors
    }
  }

  async function loadWallets() {
    setIsLoadingWallets(true);
    setError(null);
    try {
      const result = await invoke<WalletInfo[]>('evm_get_wallets');
      const nextWallets = result.map((w) => ({ id: w.id, label: w.label, address: w.address }));
      setWallets(nextWallets);

      if (nextWallets.length === 0) {
        setSelectedWalletId(null);
      } else if (!selectedWalletId || !nextWallets.some((w) => w.id === selectedWalletId)) {
        setSelectedWalletId(nextWallets[0].id);
      }
    } catch (loadError) {
      setError(normalizeError(loadError));
    } finally {
      setIsLoadingWallets(false);
    }
  }

  // Load cached snapshot from DB (fast, no RPC)
  async function loadCachedSnapshot(walletId: string) {
    try {
      const resp = await invoke<BackendSnapshotResponse>(
        'query_compute_marketplace_snapshot',
        { walletId },
      );
      setSnapshot(backendSnapshotToDisplay(resp.snapshot, null));
      // Propagate sync outcome so the UI can distinguish never-synced vs stale cache.
      setSyncOutcome({
        status: resp.sync.status as ComputeSyncOutcome['status'],
        partial: resp.sync.partial,
        failedSources: resp.sync.failed_sources,
        syncedToBlock: resp.sync.synced_to_block,
        confirmedHeadBlock: resp.sync.confirmed_head_block,
        updatedAt: resp.sync.updated_at,
      });
    } catch {
      // Ignore — snapshot stays empty until refresh
    }
  }

  useEffect(() => {
    void Promise.all([loadConfig(), loadWallets()]);
  }, []);

  // When selected wallet changes, load cached snapshot immediately
  useEffect(() => {
    if (selectedWalletId) {
      void loadCachedSnapshot(selectedWalletId);
    }
  }, [selectedWalletId]);

  async function refreshSnapshot() {
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }
    if (!selectedWallet) {
      throw new Error('Create or import an EVM wallet before refreshing compute marketplace state.');
    }

    setIsRefreshing(true);
    setError(null);

    try {
      const resp = await invoke<BackendSnapshotResponse>(
        'refresh_compute_marketplace_snapshot',
        { walletId: selectedWallet.id },
      );
      // Only mark lastRefreshedAt if the sync actually reached the chain.
      // 'unavailable' means RPC failed entirely; 'partial' means some data may be stale.
      const syncOk = resp.sync.status === 'fresh' || resp.sync.status === 'stale';
      setSnapshot(backendSnapshotToDisplay(resp.snapshot, syncOk ? Date.now() : null));
      setSyncOutcome({
        status: resp.sync.status as ComputeSyncOutcome['status'],
        partial: resp.sync.partial,
        failedSources: resp.sync.failed_sources,
        syncedToBlock: resp.sync.synced_to_block,
        confirmedHeadBlock: resp.sync.confirmed_head_block,
        updatedAt: resp.sync.updated_at,
      });
      if (resp.sync.status === 'unavailable') {
        const sources = resp.sync.failed_sources.join(', ');
        throw new Error(`Sync unavailable — check RPC connection and config. Failed: ${sources || 'unknown'}`);
      }
    } catch (refreshError) {
      setError(normalizeError(refreshError));
      throw refreshError;
    } finally {
      setIsRefreshing(false);
    }
  }

  async function registerNode(input: {
    resourceType: ResourceType;
    computePower: string;
    gpuModel: string;
    region: string;
    stakeEth: string;
  }): Promise<ComputeMutationResponse> {
    if (!selectedWallet) throw new Error('Select an EVM wallet before registering a node.');
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Registering node');
    try {
      const metadataUri = JSON.stringify({ gpuModel: input.gpuModel, region: input.region });
      const stakeWei = ethToWeiStr(input.stakeEth);

      const result = await invoke<ComputeMutationResponse>('compute_register_node', {
        input: {
          walletId: selectedWallet.id,
          clientRequestId: getOrCreateRequestId('register_node'),
          resourceType: RESOURCE_TYPES.indexOf(input.resourceType),
          computePower: input.computePower,
          stakeAmountWei: stakeWei,
          metadataUri,
        },
      });

      // Best-effort snapshot refresh after mutation
      void refreshSnapshot().catch(() => null);
      return result;
    } finally {
      setPendingActionLabel(null);
    }
  }

  /// Activate a registered Pending node via PoW challenge.
  /// Runs issueChallenge → off-chain nonce solve → submitSolution in one backend command.
  async function verifyNode(nodeId: string): Promise<ComputeMutationResponse> {
    if (!selectedWallet) throw new Error('Select an EVM wallet before activating a node.');
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Activating node (PoW challenge)');
    try {
      const result = await invoke<ComputeMutationResponse>('compute_verify_node', {
        input: {
          walletId: selectedWallet.id,
          // Generate a fresh request id per node so a second activation attempt
          // on a different node doesn't collide with an earlier one.
          clientRequestId: getOrCreateRequestId('verify_node:' + nodeId),
          nodeId,
        },
      });

      void refreshSnapshot().catch(() => null);
      return result;
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function createAndFundTask(input: {
    resourceType: ResourceType;
    requiredPower: number;
    durationHours: number;
    maxPriceEthPerHour: string;
    minTrustLevel: number;
    specTitle: string;
  }): Promise<ComputeMutationResponse> {
    if (!selectedWallet) throw new Error('Select an EVM wallet before creating a task.');
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Creating task');
    try {
      const specificationUri = JSON.stringify({ title: input.specTitle });
      const durationSeconds = input.durationHours * 3600;
      const maxPriceWei = ethToWeiStr(input.maxPriceEthPerHour);

      const result = await invoke<ComputeMutationResponse>('compute_create_and_fund_task', {
        input: {
          walletId: selectedWallet.id,
          clientRequestId: getOrCreateRequestId('create_and_fund_task'),
          resourceType: RESOURCE_TYPES.indexOf(input.resourceType),
          requiredPower: String(input.requiredPower),
          durationSeconds,
          maxPriceWei,
          minTrustLevel: input.minTrustLevel,
          specificationUri,
        },
      });

      // Clear the request ID so the next task creation gets a fresh ID.
      // On failure/crash the ID is retained for retry of the same intent.
      pendingRequestIds.current.delete('create_and_fund_task');

      void refreshSnapshot().catch(() => null);
      return result;
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function acceptTask(
    taskId: string,
    nodeId: string,
  ): Promise<ComputeMutationResponse> {
    if (!selectedWallet) throw new Error('Select an EVM wallet before accepting a task.');

    setPendingActionLabel('Accepting task');
    try {
      const result = await invoke<ComputeMutationResponse>('compute_accept_task', {
        input: {
          walletId: selectedWallet.id,
          clientRequestId: getOrCreateRequestId('accept_task:' + taskId),
          taskId,
          nodeId,
        },
      });
      void refreshSnapshot().catch(() => null);
      return result;
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function submitResult(
    taskId: string,
    resultUri: string,
  ): Promise<ComputeMutationResponse> {
    if (!selectedWallet) throw new Error('Select an EVM wallet before submitting a result.');

    setPendingActionLabel('Submitting result');
    try {
      // Compute result_hash client-side using SubtleCrypto (no viem)
      const encoder = new TextEncoder();
      const data = encoder.encode(resultUri);
      const hashBuffer = await crypto.subtle.digest('SHA-256', data);
      const hashArray = new Uint8Array(hashBuffer);
      const resultHash = '0x' + Array.from(hashArray).map((b) => b.toString(16).padStart(2, '0')).join('');

      const result = await invoke<ComputeMutationResponse>('compute_submit_result', {
        input: {
          walletId: selectedWallet.id,
          clientRequestId: getOrCreateRequestId('submit_result:' + taskId),
          taskId,
          resultHash,
          resultUri,
        },
      });
      void refreshSnapshot().catch(() => null);
      return result;
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function approveTask(
    taskId: string,
  ): Promise<ComputeMutationResponse> {
    if (!selectedWallet) throw new Error('Select an EVM wallet before approving a task.');

    setPendingActionLabel('Approving result');
    try {
      const result = await invoke<ComputeMutationResponse>('compute_approve_task', {
        input: {
          walletId: selectedWallet.id,
          clientRequestId: getOrCreateRequestId('approve_task:' + taskId),
          taskId,
        },
      });
      void refreshSnapshot().catch(() => null);
      return result;
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function disputeTask(
    taskId: string,
    reason: string,
  ): Promise<ComputeMutationResponse> {
    if (!selectedWallet) throw new Error('Select an EVM wallet before disputing a task.');

    setPendingActionLabel('Opening dispute');
    try {
      const result = await invoke<ComputeMutationResponse>('compute_dispute_task', {
        input: {
          walletId: selectedWallet.id,
          clientRequestId: getOrCreateRequestId('dispute_task:' + taskId),
          taskId,
          reason,
        },
      });
      void refreshSnapshot().catch(() => null);
      return result;
    } finally {
      setPendingActionLabel(null);
    }
  }

  return {
    activeTab,
    setActiveTab,
    wallets,
    selectedWallet,
    selectedWalletId,
    setSelectedWalletId,
    snapshot,
    syncOutcome,
    config,
    error,
    isLoadingWallets,
    isRefreshing,
    pendingActionLabel,
    refreshSnapshot,
    registerNode,
    verifyNode,
    createAndFundTask,
    acceptTask,
    submitResult,
    approveTask,
    disputeTask,
    reloadWallets: loadWallets,
  };
}

export type ComputeMarketplaceModel = ReturnType<typeof useComputeMarketplace>;

// ── Utility: ETH string → wei decimal string ─────────────────────────────────

function ethToWeiStr(eth: string): string {
  try {
    const trimmed = eth.trim();
    const [whole = '0', frac = ''] = trimmed.split('.');
    const fracPadded = frac.padEnd(18, '0').slice(0, 18);
    const wei = BigInt(whole) * BigInt('1000000000000000000') + BigInt(fracPadded);
    return wei.toString();
  } catch {
    return '0';
  }
}
