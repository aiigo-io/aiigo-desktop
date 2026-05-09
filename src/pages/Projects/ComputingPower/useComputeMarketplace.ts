import { useEffect, useState } from 'react';
import { decodeEventLog, encodeFunctionData, formatEther, http, isAddress, keccak256, parseAbi, parseEther, stringToHex, type Address, type Hex } from 'viem';
import { createPublicClient } from 'viem';

import { invoke, isTauriUnavailableError } from '@/lib/tauri';

export type ComputeTab = 'marketplace' | 'provider' | 'buyer' | 'governance';
export type ResourceType = 'GPU' | 'CPU' | 'Network' | 'Mobile' | 'IoT';
export type NodeStatus = 'Pending' | 'Verified' | 'Active' | 'Inactive' | 'Slashed';
export type TaskStatus = 'Open' | 'Assigned' | 'InProgress' | 'Completed' | 'Verified' | 'Disputed' | 'Cancelled';

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
  metadata: {
    gpuModel: string;
    region: string;
  };
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

export interface ComputeContractsConfig {
  chainId: number;
  chainName: string;
  rpcUrl: string;
  taskMarketplaceAddress: Address | null;
  nodeRegistryAddress: Address | null;
  escrowManagerAddress: Address | null;
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

interface WalletInfo {
  id: string;
  label: string;
  address: string;
}

interface ContractNodeResult {
  owner: Address;
  nodeId: Hex;
  status: number;
  resourceType: number;
  computePower: bigint;
  stakedAmount: bigint;
  reputation: bigint;
  totalTasksCompleted: bigint;
  totalEarnings: bigint;
  registeredAt: bigint;
  lastActiveAt: bigint;
  metadataURI: string;
}

interface ContractTaskResult {
  taskId: Hex;
  buyer: Address;
  assignedNode: Hex;
  resourceType: number;
  requiredPower: bigint;
  duration: bigint;
  maxPrice: bigint;
  escrowAmount: bigint;
  createdAt: bigint;
  startedAt: bigint;
  completedAt: bigint;
  status: number;
  minTrustLevel: number;
  specificationURI: string;
}

interface ContractTaskLifecycleResult {
  challengeDeadline: bigint;
  disputedBy: Address;
  disputeReason: string;
  resolved: boolean;
  resolvedBy: Address;
  grossProviderAmount: bigint;
}

interface ContractEscrowResult {
  amount: bigint;
  settlement: number;
}

interface BroadcastTransactionResult {
  txHash: Hex;
  receipt: {
    logs: Array<{
      data: Hex;
      topics: Hex[];
    }>;
  };
}

interface RegisterNodeInput {
  resourceType: ResourceType;
  gpuModel: string;
  region: string;
  stakeEth: string;
}

interface CreateTaskInput {
  resourceType: ResourceType;
  requiredPower: number;
  durationHours: number;
  maxPriceEthPerHour: string;
  minTrustLevel: number;
  specTitle: string;
}

const resourceTypes: ResourceType[] = ['GPU', 'CPU', 'Network', 'Mobile', 'IoT'];
const nodeStatuses: NodeStatus[] = ['Pending', 'Verified', 'Active', 'Inactive', 'Slashed'];
const taskStatuses: TaskStatus[] = ['Open', 'Assigned', 'InProgress', 'Completed', 'Verified', 'Disputed', 'Cancelled'];
const zeroBytes32 = `0x${'0'.repeat(64)}` as Hex;
const zeroAddress = `0x${'0'.repeat(40)}` as Address;

const nodeRegistryAbi = parseAbi([
  'function registerNode(uint8 resourceType, string metadataURI) payable returns (bytes32 nodeId)',
  'function getNodesByOwner(address owner) view returns (bytes32[])',
  'function getNode(bytes32 nodeId) view returns ((address owner, bytes32 nodeId, uint8 status, uint8 resourceType, uint256 computePower, uint256 stakedAmount, uint256 reputation, uint256 totalTasksCompleted, uint256 totalEarnings, uint256 registeredAt, uint256 lastActiveAt, string metadataURI))',
  'function getNodeTrustLevel(bytes32 nodeId) view returns (uint8)',
  'function getPendingTaskCount(bytes32 nodeId) view returns (uint256)',
  'function getActiveNodes(uint8 resourceType) view returns (bytes32[])',
]);

const taskMarketplaceAbi = parseAbi([
  'event TaskCreated(bytes32 indexed taskId, address indexed buyer, uint8 resourceType, uint256 maxPrice)',
  'function createTask(uint8 resourceType, uint256 requiredPower, uint256 duration, uint256 maxPrice, uint8 minTrustLevel, string specificationURI) returns (bytes32 taskId)',
  'function fundTaskEscrow(bytes32 taskId) payable',
  'function acceptTask(bytes32 taskId, bytes32 nodeId)',
  'function submitResult(bytes32 taskId, bytes32 resultHash, string resultURI)',
  'function approveResult(bytes32 taskId)',
  'function disputeTask(bytes32 taskId, string reason)',
  'function getTask(bytes32 taskId) view returns ((bytes32 taskId, address buyer, bytes32 assignedNode, uint8 resourceType, uint256 requiredPower, uint256 duration, uint256 maxPrice, uint256 escrowAmount, uint256 createdAt, uint256 startedAt, uint256 completedAt, uint8 status, uint8 minTrustLevel, string specificationURI))',
  'function getTaskLifecycle(bytes32 taskId) view returns ((uint256 challengeDeadline, address disputedBy, string disputeReason, bool resolved, address resolvedBy, uint256 grossProviderAmount))',
  'function getOpenTasks(uint8 resourceType) view returns (bytes32[])',
  'function getTasksByBuyer(address buyer) view returns (bytes32[])',
  'function getTasksByProvider(bytes32 nodeId) view returns (bytes32[])',
]);

const escrowManagerAbi = parseAbi([
  'function getEscrow(bytes32 taskId) view returns ((bytes32 taskId, address buyer, address provider, address treasuryRecipient, uint256 amount, uint256 platformFee, uint256 providerPayout, uint256 buyerRefund, uint256 depositedAt, uint8 settlement, bool exists))',
  'function getPendingBuyerRefund(address buyer) view returns (uint256)',
  'function getPendingProviderPayout(address provider) view returns (uint256)',
]);

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

function safeAddress(value: string | undefined): Address | null {
  if (!value || !isAddress(value)) {
    return null;
  }

  return value;
}

function getComputeContractsConfig(): ComputeContractsConfig {
  const chainId = Number(import.meta.env.VITE_AIIGO_COMPUTE_CHAIN_ID ?? '11155111');
  const taskMarketplaceAddress = safeAddress(import.meta.env.VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS);
  const nodeRegistryAddress = safeAddress(import.meta.env.VITE_AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS);
  const escrowManagerAddress = safeAddress(import.meta.env.VITE_AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS);
  const rpcUrl = import.meta.env.VITE_AIIGO_COMPUTE_RPC_URL ?? 'https://ethereum-sepolia-rpc.publicnode.com';
  const missing: string[] = [];

  if (!taskMarketplaceAddress) {
    missing.push('VITE_AIIGO_COMPUTE_TASK_MARKETPLACE_ADDRESS');
  }
  if (!nodeRegistryAddress) {
    missing.push('VITE_AIIGO_COMPUTE_NODE_REGISTRY_ADDRESS');
  }
  if (!escrowManagerAddress) {
    missing.push('VITE_AIIGO_COMPUTE_ESCROW_MANAGER_ADDRESS');
  }

  return {
    chainId,
    chainName: chainId === 11155111 ? 'Ethereum Sepolia' : `Chain ${chainId}`,
    rpcUrl,
    taskMarketplaceAddress,
    nodeRegistryAddress,
    escrowManagerAddress,
    isConfigured: missing.length === 0,
    missing,
  };
}

function getPublicClient(config: ComputeContractsConfig) {
  return createPublicClient({
    transport: http(config.rpcUrl),
  });
}

function formatBytes32(value: Hex) {
  return value === zeroBytes32 ? '' : value;
}

function parseMetadata(metadataUri: string) {
  try {
    const parsed = JSON.parse(metadataUri) as { gpuModel?: string; region?: string };
    return {
      gpuModel: parsed.gpuModel ?? 'Unspecified',
      region: parsed.region ?? 'Unknown',
    };
  } catch {
    return {
      gpuModel: metadataUri || 'Unspecified',
      region: 'Unknown',
    };
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

function toEthNumber(value: bigint) {
  return Number(formatEther(value));
}

function decimalToWei(amount: string) {
  return parseEther(amount.trim());
}

function multiplyEth(pricePerHour: string, durationHours: number) {
  const scaledPrice = Math.round(Number(pricePerHour) * 1_000_000_000);
  const scaledAmount = BigInt(Math.round(durationHours * scaledPrice));

  return scaledAmount * 1_000_000_000n;
}

function parseNode(node: ContractNodeResult, trustLevel: number, pendingTaskCount: number): ComputeNode {
  const metadata = parseMetadata(node.metadataURI);

  return {
    nodeId: node.nodeId,
    owner: node.owner,
    status: nodeStatuses[node.status] ?? 'Pending',
    resourceType: resourceTypes[node.resourceType] ?? 'GPU',
    computePower: Number(node.computePower),
    stakedAmountEth: toEthNumber(node.stakedAmount),
    reputation: Number(node.reputation),
    totalTasksCompleted: Number(node.totalTasksCompleted),
    totalEarningsEth: toEthNumber(node.totalEarnings),
    registeredAt: Number(node.registeredAt),
    lastActiveAt: Number(node.lastActiveAt),
    metadataUri: node.metadataURI,
    metadata,
    trustLevel,
    pendingTaskCount,
  };
}

function parseTask(task: ContractTaskResult, lifecycle: ContractTaskLifecycleResult): ComputeTask {
  return {
    taskId: task.taskId,
    buyer: task.buyer,
    assignedNodeId: formatBytes32(task.assignedNode) || null,
    resourceType: resourceTypes[task.resourceType] ?? 'GPU',
    requiredPower: Number(task.requiredPower),
    durationSeconds: Number(task.duration),
    maxPriceEth: toEthNumber(task.maxPrice),
    escrowAmountEth: toEthNumber(task.escrowAmount),
    createdAt: Number(task.createdAt),
    startedAt: task.startedAt === 0n ? null : Number(task.startedAt),
    completedAt: task.completedAt === 0n ? null : Number(task.completedAt),
    status: taskStatuses[task.status] ?? 'Open',
    minTrustLevel: task.minTrustLevel,
    specificationUri: task.specificationURI,
    specTitle: parseTaskTitle(task.specificationURI),
    challengeDeadline: lifecycle.challengeDeadline === 0n ? null : Number(lifecycle.challengeDeadline),
    disputeReason: lifecycle.disputeReason || null,
    disputedBy: lifecycle.disputedBy === zeroAddress ? null : lifecycle.disputedBy,
    resolved: lifecycle.resolved,
    resolvedBy: lifecycle.resolvedBy === zeroAddress ? null : lifecycle.resolvedBy,
    grossProviderAmountEth: toEthNumber(lifecycle.grossProviderAmount),
  };
}

async function readTask(publicClient: ReturnType<typeof getPublicClient>, config: ComputeContractsConfig, taskId: Hex) {
  const task = await publicClient.readContract({
    address: config.taskMarketplaceAddress!,
    abi: taskMarketplaceAbi,
    functionName: 'getTask',
    args: [taskId],
  }) as ContractTaskResult;

  const lifecycle = await publicClient.readContract({
    address: config.taskMarketplaceAddress!,
    abi: taskMarketplaceAbi,
    functionName: 'getTaskLifecycle',
    args: [taskId],
  }) as ContractTaskLifecycleResult;

  return parseTask(task, lifecycle);
}

async function broadcastContractTransaction(
  config: ComputeContractsConfig,
  wallet: ComputeWallet,
  address: Address,
  data: Hex,
  value: bigint,
): Promise<BroadcastTransactionResult> {
  const publicClient = getPublicClient(config);
  const gasPrice = await publicClient.getGasPrice();
  const gasLimit = await publicClient.estimateGas({
    account: wallet.address as Address,
    to: address,
    data,
    value,
  });

  const txHash = await invoke<string>('evm_send_transaction', {
    walletId: wallet.id,
    chainId: config.chainId,
    transaction: {
      to: address,
      data,
      value: value.toString(),
      gasLimit: gasLimit.toString(),
      gasPrice: gasPrice.toString(),
    },
  }) as Hex;

  const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
  return { txHash, receipt };
}

function toComputeWallet(wallet: WalletInfo): ComputeWallet {
  return {
    id: wallet.id,
    label: wallet.label,
    address: wallet.address,
  };
}

function normalizeError(error: unknown) {
  if (isTauriUnavailableError(error)) {
    return 'Tauri runtime is unavailable. Start the desktop app to use compute marketplace actions.';
  }

  if (error instanceof Error) {
    return error.message;
  }

  return String(error);
}

export function useComputeMarketplace() {
  const [activeTab, setActiveTab] = useState<ComputeTab>('marketplace');
  const [wallets, setWallets] = useState<ComputeWallet[]>([]);
  const [selectedWalletId, setSelectedWalletId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<ComputeSnapshot>(emptySnapshot);
  const [isLoadingWallets, setIsLoadingWallets] = useState(true);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [pendingActionLabel, setPendingActionLabel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const config = getComputeContractsConfig();
  const selectedWallet = wallets.find((wallet) => wallet.id === selectedWalletId) ?? null;

  async function loadWallets() {
    setIsLoadingWallets(true);
    setError(null);

    try {
      const result = await invoke<WalletInfo[]>('evm_get_wallets');
      const nextWallets = result.map(toComputeWallet);
      setWallets(nextWallets);

      if (nextWallets.length === 0) {
        setSelectedWalletId(null);
      } else if (!selectedWalletId || !nextWallets.some((wallet) => wallet.id === selectedWalletId)) {
        setSelectedWalletId(nextWallets[0].id);
      }
    } catch (loadError) {
      setError(normalizeError(loadError));
    } finally {
      setIsLoadingWallets(false);
    }
  }

  useEffect(() => {
    void loadWallets();
  }, []);

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
      const publicClient = getPublicClient(config);
      const owner = selectedWallet.address as Address;

      const ownedNodeIds = await publicClient.readContract({
        address: config.nodeRegistryAddress!,
        abi: nodeRegistryAbi,
        functionName: 'getNodesByOwner',
        args: [owner],
      }) as Hex[];

      const ownedNodes = await Promise.all(ownedNodeIds.map(async (nodeId) => {
        const node = await publicClient.readContract({
          address: config.nodeRegistryAddress!,
          abi: nodeRegistryAbi,
          functionName: 'getNode',
          args: [nodeId],
        }) as ContractNodeResult;
        const trustLevel = await publicClient.readContract({
          address: config.nodeRegistryAddress!,
          abi: nodeRegistryAbi,
          functionName: 'getNodeTrustLevel',
          args: [nodeId],
        }) as number;
        const pendingTaskCount = await publicClient.readContract({
          address: config.nodeRegistryAddress!,
          abi: nodeRegistryAbi,
          functionName: 'getPendingTaskCount',
          args: [nodeId],
        }) as bigint;

        return parseNode(node, trustLevel, Number(pendingTaskCount));
      }));

      const openTaskIdsByType = await Promise.all(resourceTypes.map(async (_, resourceIndex) => {
        const taskIds = await publicClient.readContract({
          address: config.taskMarketplaceAddress!,
          abi: taskMarketplaceAbi,
          functionName: 'getOpenTasks',
          args: [resourceIndex],
        }) as Hex[];

        return taskIds;
      }));

      const buyerTaskIds = await publicClient.readContract({
        address: config.taskMarketplaceAddress!,
        abi: taskMarketplaceAbi,
        functionName: 'getTasksByBuyer',
        args: [owner],
      }) as Hex[];

      const providerTaskIdsByNode = await Promise.all(ownedNodeIds.map((nodeId) => publicClient.readContract({
        address: config.taskMarketplaceAddress!,
        abi: taskMarketplaceAbi,
        functionName: 'getTasksByProvider',
        args: [nodeId],
      }) as Promise<Hex[]>));

      const uniqueTaskIds = new Set<Hex>();
      for (const taskIds of openTaskIdsByType) {
        for (const taskId of taskIds) {
          uniqueTaskIds.add(taskId);
        }
      }
      for (const taskId of buyerTaskIds) {
        uniqueTaskIds.add(taskId);
      }
      for (const taskIds of providerTaskIdsByNode) {
        for (const taskId of taskIds) {
          uniqueTaskIds.add(taskId);
        }
      }

      const allTasks = await Promise.all(Array.from(uniqueTaskIds).map((taskId) => readTask(publicClient, config, taskId)));

      const openTasks = allTasks
        .filter((task) => task.status === 'Open' && task.escrowAmountEth > 0)
        .sort((left, right) => right.createdAt - left.createdAt);
      const buyerTasks = allTasks
        .filter((task) => task.buyer.toLowerCase() === selectedWallet.address.toLowerCase())
        .sort((left, right) => right.createdAt - left.createdAt);
      const providerTaskIds = new Set(providerTaskIdsByNode.flat());
      const providerTasks = allTasks
        .filter((task) => providerTaskIds.has(task.taskId as Hex))
        .sort((left, right) => right.createdAt - left.createdAt);
      const disputes = allTasks.filter((task) => task.status === 'Disputed');

      const activeNodeIdsByType = await Promise.all(resourceTypes.map(async (_, resourceIndex) => {
        const nodeIds = await publicClient.readContract({
          address: config.nodeRegistryAddress!,
          abi: nodeRegistryAbi,
          functionName: 'getActiveNodes',
          args: [resourceIndex],
        }) as Hex[];
        return nodeIds;
      }));

      const uniqueActiveNodeIds = new Set<Hex>();
      for (const nodeIds of activeNodeIdsByType) {
        for (const nodeId of nodeIds) {
          uniqueActiveNodeIds.add(nodeId);
        }
      }

      const activeNodes = await Promise.all(Array.from(uniqueActiveNodeIds).map(async (nodeId) => {
        const node = await publicClient.readContract({
          address: config.nodeRegistryAddress!,
          abi: nodeRegistryAbi,
          functionName: 'getNode',
          args: [nodeId],
        }) as ContractNodeResult;
        const trustLevel = await publicClient.readContract({
          address: config.nodeRegistryAddress!,
          abi: nodeRegistryAbi,
          functionName: 'getNodeTrustLevel',
          args: [nodeId],
        }) as number;
        const pendingTaskCount = await publicClient.readContract({
          address: config.nodeRegistryAddress!,
          abi: nodeRegistryAbi,
          functionName: 'getPendingTaskCount',
          args: [nodeId],
        }) as bigint;

        return parseNode(node, trustLevel, Number(pendingTaskCount));
      }));

      const pendingBuyerRefund = await publicClient.readContract({
        address: config.escrowManagerAddress!,
        abi: escrowManagerAbi,
        functionName: 'getPendingBuyerRefund',
        args: [owner],
      }) as bigint;
      const pendingProviderPayout = await publicClient.readContract({
        address: config.escrowManagerAddress!,
        abi: escrowManagerAbi,
        functionName: 'getPendingProviderPayout',
        args: [owner],
      }) as bigint;

      let totalLockedEscrowEth = 0;
      for (const task of allTasks) {
        try {
          const escrow = await publicClient.readContract({
            address: config.escrowManagerAddress!,
            abi: escrowManagerAbi,
            functionName: 'getEscrow',
            args: [task.taskId as Hex],
          }) as ContractEscrowResult;
          if (escrow.settlement === 0) {
            totalLockedEscrowEth += toEthNumber(escrow.amount);
          }
        } catch {
          // Ignore tasks without escrow rows.
        }
      }

      setSnapshot({
        openTasks,
        buyerTasks,
        providerTasks,
        disputes,
        ownedNodes,
        activeNodes,
        pendingBuyerRefundEth: toEthNumber(pendingBuyerRefund),
        pendingProviderPayoutEth: toEthNumber(pendingProviderPayout),
        totalLockedEscrowEth,
        lastRefreshedAt: Date.now(),
      });
    } catch (refreshError) {
      setError(normalizeError(refreshError));
      throw refreshError;
    } finally {
      setIsRefreshing(false);
    }
  }

  async function registerNode(input: RegisterNodeInput) {
    if (!selectedWallet) {
      throw new Error('Select an EVM wallet before registering a node.');
    }
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Registering node');
    try {
      const metadataUri = JSON.stringify({ gpuModel: input.gpuModel, region: input.region });
      const stakeValue = decimalToWei(input.stakeEth);
      const registrationFee = parseEther('0.1');
      const data = encodeFunctionData({
        abi: nodeRegistryAbi,
        functionName: 'registerNode',
        args: [resourceTypes.indexOf(input.resourceType), metadataUri],
      });

      await broadcastContractTransaction(
        config,
        selectedWallet,
        config.nodeRegistryAddress!,
        data,
        stakeValue + registrationFee,
      );

      await refreshSnapshot();
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function createAndFundTask(input: CreateTaskInput) {
    if (!selectedWallet) {
      throw new Error('Select an EVM wallet before creating a task.');
    }
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Creating task');
    try {
      const specificationUri = JSON.stringify({ title: input.specTitle });
      const maxPrice = decimalToWei(input.maxPriceEthPerHour);
      const createData = encodeFunctionData({
        abi: taskMarketplaceAbi,
        functionName: 'createTask',
        args: [
          resourceTypes.indexOf(input.resourceType),
          BigInt(input.requiredPower),
          BigInt(input.durationHours * 3600),
          maxPrice,
          input.minTrustLevel,
          specificationUri,
        ],
      });

      const createResult = await broadcastContractTransaction(
        config,
        selectedWallet,
        config.taskMarketplaceAddress!,
        createData,
        0n,
      );

      const createdLog = createResult.receipt.logs
        .map((log) => {
          try {
            return decodeEventLog({
              abi: taskMarketplaceAbi,
              data: log.data,
              topics: log.topics as [] | [Hex, ...Hex[]],
            });
          } catch {
            return null;
          }
        })
        .find((entry) => entry?.eventName === 'TaskCreated');

      if (!createdLog || !('taskId' in createdLog.args)) {
        throw new Error('Task was created, but the emitted task id could not be decoded.');
      }

      setPendingActionLabel('Funding escrow');
      const escrowValue = multiplyEth(input.maxPriceEthPerHour, input.durationHours);
      const fundData = encodeFunctionData({
        abi: taskMarketplaceAbi,
        functionName: 'fundTaskEscrow',
        args: [createdLog.args.taskId as Hex],
      });

      await broadcastContractTransaction(
        config,
        selectedWallet,
        config.taskMarketplaceAddress!,
        fundData,
        escrowValue,
      );

      await refreshSnapshot();
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function acceptTask(taskId: string, nodeId: string) {
    if (!selectedWallet) {
      throw new Error('Select an EVM wallet before accepting a task.');
    }
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Accepting task');
    try {
      const publicClient = getPublicClient(config);
      const task = await readTask(publicClient, config, taskId as Hex);
      if (task.status !== 'Open') {
        throw new Error('Task is no longer open on-chain. Refresh and try again.');
      }
      if (task.escrowAmountEth <= 0) {
        throw new Error('Task escrow is not funded on-chain.');
      }

      const data = encodeFunctionData({
        abi: taskMarketplaceAbi,
        functionName: 'acceptTask',
        args: [taskId as Hex, nodeId as Hex],
      });

      await broadcastContractTransaction(
        config,
        selectedWallet,
        config.taskMarketplaceAddress!,
        data,
        0n,
      );

      await refreshSnapshot();
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function submitResult(taskId: string, resultUri: string) {
    if (!selectedWallet) {
      throw new Error('Select an EVM wallet before submitting a result.');
    }
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Submitting result');
    try {
      const publicClient = getPublicClient(config);
      const task = await readTask(publicClient, config, taskId as Hex);
      if (task.status !== 'Assigned' && task.status !== 'InProgress') {
        throw new Error('Task is not in an assignable submission state on-chain.');
      }

      const resultHash = keccak256(stringToHex(resultUri));
      const data = encodeFunctionData({
        abi: taskMarketplaceAbi,
        functionName: 'submitResult',
        args: [taskId as Hex, resultHash, resultUri],
      });

      await broadcastContractTransaction(
        config,
        selectedWallet,
        config.taskMarketplaceAddress!,
        data,
        0n,
      );

      await refreshSnapshot();
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function approveTask(taskId: string) {
    if (!selectedWallet) {
      throw new Error('Select an EVM wallet before approving a task.');
    }
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Approving result');
    try {
      const publicClient = getPublicClient(config);
      const task = await readTask(publicClient, config, taskId as Hex);
      if (task.status !== 'Completed') {
        throw new Error('Task is not completed on-chain. Refresh and try again.');
      }

      const data = encodeFunctionData({
        abi: taskMarketplaceAbi,
        functionName: 'approveResult',
        args: [taskId as Hex],
      });

      await broadcastContractTransaction(
        config,
        selectedWallet,
        config.taskMarketplaceAddress!,
        data,
        0n,
      );

      await refreshSnapshot();
    } finally {
      setPendingActionLabel(null);
    }
  }

  async function disputeTask(taskId: string, reason: string) {
    if (!selectedWallet) {
      throw new Error('Select an EVM wallet before disputing a task.');
    }
    if (!config.isConfigured) {
      throw new Error(`Compute contracts are not configured: ${config.missing.join(', ')}`);
    }

    setPendingActionLabel('Opening dispute');
    try {
      const data = encodeFunctionData({
        abi: taskMarketplaceAbi,
        functionName: 'disputeTask',
        args: [taskId as Hex, reason],
      });

      await broadcastContractTransaction(
        config,
        selectedWallet,
        config.taskMarketplaceAddress!,
        data,
        0n,
      );

      await refreshSnapshot();
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
    config,
    error,
    isLoadingWallets,
    isRefreshing,
    pendingActionLabel,
    refreshSnapshot,
    registerNode,
    createAndFundTask,
    acceptTask,
    submitResult,
    approveTask,
    disputeTask,
    reloadWallets: loadWallets,
  };
}

export type ComputeMarketplaceModel = ReturnType<typeof useComputeMarketplace>;