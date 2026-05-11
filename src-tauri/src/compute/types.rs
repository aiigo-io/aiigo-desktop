use serde::{Deserialize, Serialize};

// ── Config ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub task_marketplace_address: String,
    pub node_registry_address: String,
    pub escrow_manager_address: String,
    pub task_marketplace_deploy_block: u64,
    pub node_registry_deploy_block: u64,
    pub escrow_manager_deploy_block: u64,
    pub confirmation_depth: u64,
}

impl ComputeConfig {
    pub fn bootstrap_start_block(&self) -> u64 {
        self.task_marketplace_deploy_block
            .min(self.node_registry_deploy_block)
            .min(self.escrow_manager_deploy_block)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeConfigResponse {
    pub chain_id: Option<u64>,
    pub chain_name: Option<String>,
    pub rpc_url: Option<String>,
    pub task_marketplace_address: Option<String>,
    pub node_registry_address: Option<String>,
    pub escrow_manager_address: Option<String>,
    pub task_marketplace_deploy_block: Option<u64>,
    pub node_registry_deploy_block: Option<u64>,
    pub escrow_manager_deploy_block: Option<u64>,
    pub confirmation_depth: Option<u64>,
    pub bootstrap_start_block: Option<u64>,
    pub is_configured: bool,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
    /// ProofOfWorkVerifier contract address (optional — separate from core marketplace config).
    pub pow_verifier_address: Option<String>,
    /// True when AIIGO_COMPUTE_POW_VERIFIER_ADDRESS is set.
    pub is_pow_configured: bool,
}

// ── Domain enums ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum ResourceType {
    Gpu,
    Cpu,
    Network,
    Mobile,
    Iot,
}

impl ResourceType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Gpu,
            1 => Self::Cpu,
            2 => Self::Network,
            3 => Self::Mobile,
            _ => Self::Iot,
        }
    }
    pub fn to_u8(&self) -> u8 {
        match self {
            Self::Gpu => 0,
            Self::Cpu => 1,
            Self::Network => 2,
            Self::Mobile => 3,
            Self::Iot => 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum NodeStatus {
    Pending,
    Verified,
    Active,
    Inactive,
    Slashed,
}

impl NodeStatus {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Pending,
            1 => Self::Verified,
            2 => Self::Active,
            3 => Self::Inactive,
            _ => Self::Slashed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum TaskStatus {
    Open,
    Assigned,
    InProgress,
    Completed,
    Verified,
    Disputed,
    Cancelled,
}

impl TaskStatus {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Open,
            1 => Self::Assigned,
            2 => Self::InProgress,
            3 => Self::Completed,
            4 => Self::Verified,
            5 => Self::Disputed,
            _ => Self::Cancelled,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Assigned => "Assigned",
            Self::InProgress => "InProgress",
            Self::Completed => "Completed",
            Self::Verified => "Verified",
            Self::Disputed => "Disputed",
            Self::Cancelled => "Cancelled",
        }
    }
}

// ── Snapshot domain types ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeNode {
    pub node_id: String,
    pub owner: String,
    pub status: NodeStatus,
    pub resource_type: ResourceType,
    pub compute_power: String, // u256 as decimal string
    pub staked_amount_wei: String,
    pub reputation: String,
    pub total_tasks_completed: u64,
    pub total_earnings_wei: String,
    pub registered_at: Option<u64>,
    pub last_active_at: Option<u64>,
    pub metadata_uri: String,
    pub trust_level: u8,
    pub pending_task_count: u64,
    pub last_chain_block: Option<u64>,
    pub last_chain_block_hash: Option<String>,
    pub synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeTask {
    pub task_id: String,
    pub buyer: String,
    pub assigned_node_id: Option<String>,
    pub resource_type: ResourceType,
    pub required_power: String,
    pub duration_seconds: u64,
    pub max_price_wei: String,
    pub escrow_amount_wei: String,
    pub status: TaskStatus,
    pub specification_uri: String,
    pub min_trust_level: u8,
    pub created_at: Option<u64>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub challenge_deadline: Option<u64>,
    pub dispute_reason: Option<String>,
    pub disputed_by: Option<String>,
    pub resolved: bool,
    pub resolved_by: Option<String>,
    pub gross_provider_amount_wei: String,
    pub last_chain_block: Option<u64>,
    pub last_chain_block_hash: Option<String>,
    pub synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeSnapshot {
    pub open_tasks: Vec<ComputeTask>,
    pub buyer_tasks: Vec<ComputeTask>,
    pub provider_tasks: Vec<ComputeTask>,
    pub disputes: Vec<ComputeTask>,
    pub owned_nodes: Vec<ComputeNode>,
    pub active_nodes: Vec<ComputeNode>,
    pub pending_buyer_refund_wei: String,
    pub pending_provider_payout_wei: String,
    pub total_locked_escrow_wei: String,
}

impl ComputeSnapshot {
    pub fn empty() -> Self {
        Self {
            open_tasks: vec![],
            buyer_tasks: vec![],
            provider_tasks: vec![],
            disputes: vec![],
            owned_nodes: vec![],
            active_nodes: vec![],
            pending_buyer_refund_wei: "0".to_string(),
            pending_provider_payout_wei: "0".to_string(),
            total_locked_escrow_wei: "0".to_string(),
        }
    }
}

// ── Freshness / sync outcome ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Fresh,
    Stale,
    Partial,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncReason {
    Query,
    Manual,
    AfterBroadcast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncCoverage {
    FullEventHistory,
    CachedOnly,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeSyncOutcome {
    pub reason: SyncReason,
    pub target: String,
    pub status: SyncStatus,
    pub updated_at: Option<String>,
    pub bootstrap_start_block: Option<u64>,
    pub synced_to_block: Option<u64>,
    pub synced_to_block_hash: Option<String>,
    pub confirmed_head_block: Option<u64>,
    pub confirmation_depth: Option<u64>,
    pub partial: bool,
    pub failed_sources: Vec<String>,
    pub coverage: SyncCoverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeSnapshotResponse {
    pub snapshot: ComputeSnapshot,
    pub sync: ComputeSyncOutcome,
}

// ── Mutation types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MutationStatus {
    Pending,
    Signing,
    Broadcasting,
    Confirming,
    Confirmed,
    Failed,
    PartialRequiresResume,
    IdempotencyConflict,
}

impl MutationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Signing => "signing",
            Self::Broadcasting => "broadcasting",
            Self::Confirming => "confirming",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
            Self::PartialRequiresResume => "partial_requires_resume",
            Self::IdempotencyConflict => "idempotency_conflict",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => Self::Pending,
            "signing" => Self::Signing,
            "broadcasting" => Self::Broadcasting,
            "confirming" => Self::Confirming,
            "confirmed" => Self::Confirmed,
            "failed" => Self::Failed,
            "partial_requires_resume" => Self::PartialRequiresResume,
            "idempotency_conflict" => Self::IdempotencyConflict,
            _ => Self::Failed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeMutationResponse {
    pub mutation_id: String,
    pub wallet_id: String,
    pub client_request_id: String,
    pub request_hash: String,
    pub status: String,
    pub action: String,
    pub current_step: Option<String>,
    pub tx_hash: Option<String>,
    pub task_id: Option<String>,
    pub node_id: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Mutation inputs ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterNodeInput {
    pub wallet_id: String,
    pub client_request_id: String,
    // node_id is NOT supplied by caller — it is derived on-chain by registerNode() and
    // extracted from the NodeRegistered event receipt.
    pub resource_type: u8,      // 0=GPU,1=CPU,2=Network,3=Mobile,4=IoT
    pub compute_power: String,  // u256 decimal (set post-registration via updateComputePower)
    pub stake_amount_wei: String,
    pub metadata_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyNodeInput {
    pub wallet_id: String,
    pub client_request_id: String,
    /// bytes32 hex (chain-assigned by NodeRegistry.registerNode).
    /// The node must be registered (Pending status) before issuing a PoW challenge.
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAndFundTaskInput {
    pub wallet_id: String,
    pub client_request_id: String,
    // task_id is NOT supplied by caller — it is derived on-chain by createTask() and
    // extracted from the TaskCreated event receipt.
    pub resource_type: u8,
    pub required_power: String,    // u256 decimal
    pub duration_seconds: u64,
    pub max_price_wei: String,
    pub min_trust_level: u8,
    pub specification_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptTaskInput {
    pub wallet_id: String,
    pub client_request_id: String,
    pub task_id: String,
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitResultInput {
    pub wallet_id: String,
    pub client_request_id: String,
    pub task_id: String,
    pub result_hash: String, // bytes32 hex
    pub result_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApproveTaskInput {
    pub wallet_id: String,
    pub client_request_id: String,
    pub task_id: String,
    // actual_price_wei removed: the contract's approveResult(bytes32) takes no price arg;
    // escrow settlement is handled internally by the contract.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisputeTaskInput {
    pub wallet_id: String,
    pub client_request_id: String,
    pub task_id: String,
    pub reason: String,
}

// ── Internal sync cursor ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SyncCursor {
    pub scope_key: String,
    pub chain_id: u64,
    pub bootstrap_start_block: u64,
    pub synced_to_block: Option<u64>,
    pub synced_to_block_hash: Option<String>,
    pub confirmed_head_block: Option<u64>,
    pub confirmation_depth: u64,
    pub status: SyncStatus,
    pub failed_sources: Vec<String>,
    pub updated_at: Option<String>,
}
