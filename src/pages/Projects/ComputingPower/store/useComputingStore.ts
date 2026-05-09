import { create } from 'zustand';
import { persist } from 'zustand/middleware';

// --- Types based on Solidity Specs ---

export type ResourceType = 'GPU' | 'CPU' | 'Network' | 'Mobile' | 'IoT';
export type NodeStatus = 'Pending' | 'Verified' | 'Active' | 'Inactive' | 'Slashed';
export type TaskStatus = 'Open' | 'Assigned' | 'InProgress' | 'Completed' | 'Verified' | 'Disputed' | 'Cancelled';

export interface Node {
    owner: string;
    nodeId: string;
    status: NodeStatus;
    resourceType: ResourceType;
    computePower: number; // TFLOPS
    stakedAmount: number; // ETH
    reputation: number; // 0-10000
    totalTasksCompleted: number;
    totalEarnings: number; // ETH
    registeredAt: number;
    metadata: {
        gpuModel?: string;
        region: string;
    };
    trustLevel: 1 | 2 | 3 | 4;
}

export interface Task {
    taskId: string;
    buyer: string;
    assignedNode?: string;
    resourceType: ResourceType;
    requiredPower: number;
    duration: number; // seconds
    maxPrice: number; // ETH/hour
    escrowAmount: number; // ETH
    createdAt: number;
    startedAt?: number;
    completedAt?: number;
    status: TaskStatus;
    minTrustLevel: number;
    specTitle: string;
}

interface UserState {
    address: string;
    balance: number; // ETH
    isProvider: boolean;
    providerNodeId?: string;
}

interface ComputingStore {
    // State
    user: UserState;
    nodes: Record<string, Node>;
    tasks: Record<string, Task>;
    activeTab: 'marketplace' | 'provider' | 'buyer' | 'governance';

    // Actions
    setActiveTab: (tab: 'marketplace' | 'provider' | 'buyer' | 'governance') => void;
    registerNode: (type: ResourceType, specs: any, stake: number) => Promise<string>;
    submitPoW: (nodeId: string) => Promise<void>;
    createTask: (task: Omit<Task, 'taskId' | 'status' | 'createdAt' | 'buyer' | 'escrowAmount'>) => Promise<string>;
    acceptTask: (taskId: string, nodeId: string) => Promise<void>;
    submitResult: (taskId: string) => Promise<void>;
    verifyResult: (taskId: string) => Promise<void>;
    disputeTask: (taskId: string) => Promise<void>;

    // Admin/Debug
    reset: () => void;
}

// --- Initial Mock Data ---

const INITIAL_NODES: Record<string, Node> = {
    'node-1': {
        owner: '0x123...abc',
        nodeId: 'node-1',
        status: 'Active',
        resourceType: 'GPU',
        computePower: 82,
        stakedAmount: 3.5,
        reputation: 9800,
        totalTasksCompleted: 142,
        totalEarnings: 12.4,
        registeredAt: Date.now() - 10000000,
        metadata: { gpuModel: 'RTX 4090', region: 'US-East' },
        trustLevel: 3,
    },
    'node-2': {
        owner: '0x456...def',
        nodeId: 'node-2',
        status: 'Active',
        resourceType: 'CPU',
        computePower: 12,
        stakedAmount: 1.0,
        reputation: 9200,
        totalTasksCompleted: 45,
        totalEarnings: 2.1,
        registeredAt: Date.now() - 5000000,
        metadata: { gpuModel: 'EPYC 7763', region: 'EU-Central' },
        trustLevel: 2,
    }
};

const INITIAL_TASKS: Record<string, Task> = {
    'task-1': {
        taskId: 'task-1',
        buyer: '0x789...ghi',
        resourceType: 'GPU',
        requiredPower: 80,
        duration: 3600 * 4,
        maxPrice: 0.005,
        escrowAmount: 0.02,
        createdAt: Date.now() - 3600,
        status: 'Open',
        minTrustLevel: 2,
        specTitle: 'LLM Fine-tuning (Llama-3-70b)',
    },
    'task-2': {
        taskId: 'task-2',
        buyer: '0xabc...jkl',
        assignedNode: 'node-1',
        resourceType: 'GPU',
        requiredPower: 82,
        duration: 3600 * 2,
        maxPrice: 0.008,
        escrowAmount: 0.016,
        createdAt: Date.now() - 7200,
        startedAt: Date.now() - 1000,
        status: 'InProgress',
        minTrustLevel: 3,
        specTitle: '3D Rendering Batch',
    }
};

export const useComputingStore = create<ComputingStore>()(
    persist(
        (set, get) => ({
            user: {
                address: '0xUser...Wallet',
                balance: 100.0, // Mock ETH
                isProvider: false,
            },
            nodes: INITIAL_NODES,
            tasks: INITIAL_TASKS,
            activeTab: 'marketplace',

            setActiveTab: (tab) => set({ activeTab: tab }),

            registerNode: async (type, specs, stake) => {
                const nodeId = `node-${Date.now()}`;
                const newNode: Node = {
                    owner: get().user.address,
                    nodeId,
                    status: 'Pending',
                    resourceType: type,
                    computePower: 0, // Verified later
                    stakedAmount: stake,
                    reputation: 5000, // Starting rep
                    totalTasksCompleted: 0,
                    totalEarnings: 0,
                    registeredAt: Date.now(),
                    metadata: {
                        gpuModel: specs.gpuModel || 'Generic',
                        region: specs.region || 'Global',
                    },
                    trustLevel: stake >= 5 ? 4 : stake >= 3 ? 3 : stake >= 1 ? 2 : 1, // Simplified logic
                };

                set((state) => ({
                    user: { ...state.user, balance: state.user.balance - stake - 0.1, isProvider: true, providerNodeId: nodeId }, // Stake + Fee
                    nodes: { ...state.nodes, [nodeId]: newNode }
                }));
                return nodeId;
            },

            submitPoW: async (nodeId) => {
                // Mock waiting for PoW
                await new Promise(r => setTimeout(r, 2000));

                set((state) => {
                    const node = state.nodes[nodeId];
                    if (!node) return state;
                    return {
                        nodes: {
                            ...state.nodes,
                            [nodeId]: {
                                ...node,
                                status: 'Active',
                                computePower: Math.floor(Math.random() * 100) + 50 // Random verified power
                            }
                        }
                    };
                });
            },

            createTask: async (taskParams) => {
                const taskId = `task-${Date.now()}`;
                const escrowAmount = taskParams.maxPrice * (taskParams.duration / 3600);

                const newTask: Task = {
                    ...taskParams,
                    taskId,
                    buyer: get().user.address,
                    createdAt: Date.now(),
                    status: 'Open',
                    escrowAmount,
                };

                set((state) => ({
                    user: { ...state.user, balance: state.user.balance - escrowAmount },
                    tasks: { ...state.tasks, [taskId]: newTask }
                }));
                return taskId;
            },

            acceptTask: async (taskId, nodeId) => {
                set((state) => ({
                    tasks: {
                        ...state.tasks,
                        [taskId]: {
                            ...state.tasks[taskId],
                            status: 'InProgress',
                            assignedNode: nodeId,
                            startedAt: Date.now()
                        }
                    }
                }));
            },

            submitResult: async (taskId) => {
                await new Promise(r => setTimeout(r, 1000));
                set((state) => ({
                    tasks: {
                        ...state.tasks,
                        [taskId]: {
                            ...state.tasks[taskId],
                            status: 'Completed',
                            completedAt: Date.now()
                        }
                    }
                }));
            },

            verifyResult: async (taskId) => {
                await new Promise(r => setTimeout(r, 1000));
                const task = get().tasks[taskId];
                if (!task || !task.assignedNode) return;

                const payout = task.escrowAmount * 0.92;

                set((state) => {
                    const providerNode = state.nodes[task.assignedNode!];
                    // In real flow, this goes to wallet, here we add to totalEarnings
                    return {
                        tasks: {
                            ...state.tasks,
                            [taskId]: { ...task, status: 'Verified' }
                        },
                        nodes: {
                            ...state.nodes,
                            [task.assignedNode!]: {
                                ...providerNode,
                                totalTasksCompleted: providerNode.totalTasksCompleted + 1,
                                totalEarnings: providerNode.totalEarnings + payout,
                                reputation: Math.min(10000, providerNode.reputation + 100)
                            }
                        }
                    };
                });
            },

            disputeTask: async (taskId) => {
                set((state) => ({
                    tasks: {
                        ...state.tasks,
                        [taskId]: { ...state.tasks[taskId], status: 'Disputed' }
                    }
                }));
            },

            reset: () => set({ nodes: INITIAL_NODES, tasks: INITIAL_TASKS, user: { address: '0xUser...Wallet', balance: 100.0, isProvider: false } })
        }),
        {
            name: 'aiigo-computing-store',
        }
    )
);
