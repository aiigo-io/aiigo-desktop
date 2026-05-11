/// Tauri command handlers for the compute marketplace.

use crate::compute::config::load_compute_config_response;
use crate::compute::mutations::{
    execute_accept_task, execute_approve_task, execute_create_and_fund_task,
    execute_dispute_task, execute_register_node, execute_submit_result, execute_verify_node,
};
use crate::compute::sync::{query_snapshot_from_db, refresh_snapshot};
use crate::compute::types::{
    AcceptTaskInput, ApproveTaskInput, ComputeConfigResponse, ComputeMutationResponse,
    ComputeSnapshotResponse, CreateAndFundTaskInput, DisputeTaskInput, RegisterNodeInput,
    SubmitResultInput, VerifyNodeInput,
};
use crate::AppSecurity;
use tauri::State;

/// Return the current compute configuration parsed from environment variables.
/// Never errors — returns `configured: false` with `missing` list instead.
#[tauri::command]
pub async fn compute_get_config() -> Result<ComputeConfigResponse, String> {
    Ok(load_compute_config_response())
}

/// Return a snapshot built from the local SQLite read model (no RPC).
/// Fast path for UI hydration; may return stale or empty data.
#[tauri::command]
pub async fn query_compute_marketplace_snapshot(
    wallet_id: String,
) -> Result<ComputeSnapshotResponse, String> {
    query_snapshot_from_db(&wallet_id)
}

/// Scan on-chain events, re-read canonical entity state, update local DB, return fresh snapshot.
#[tauri::command]
pub async fn refresh_compute_marketplace_snapshot(
    wallet_id: String,
) -> Result<ComputeSnapshotResponse, String> {
    refresh_snapshot(&wallet_id).await
}

/// Register a compute node on NodeRegistry.
#[tauri::command]
pub async fn compute_register_node(
    input: RegisterNodeInput,
    state: State<'_, AppSecurity>,
) -> Result<ComputeMutationResponse, String> {
    execute_register_node(&input, &state).await
}

/// Create a new task and fund the escrow in a single transaction.
#[tauri::command]
pub async fn compute_create_and_fund_task(
    input: CreateAndFundTaskInput,
    state: State<'_, AppSecurity>,
) -> Result<ComputeMutationResponse, String> {
    execute_create_and_fund_task(&input, &state).await
}

/// Accept a posted task as the assigned compute node.
#[tauri::command]
pub async fn compute_accept_task(
    input: AcceptTaskInput,
    state: State<'_, AppSecurity>,
) -> Result<ComputeMutationResponse, String> {
    execute_accept_task(&input, &state).await
}

/// Submit the task result URI as the compute node.
#[tauri::command]
pub async fn compute_submit_result(
    input: SubmitResultInput,
    state: State<'_, AppSecurity>,
) -> Result<ComputeMutationResponse, String> {
    execute_submit_result(&input, &state).await
}

/// Approve the completed task result as the buyer.
#[tauri::command]
pub async fn compute_approve_task(
    input: ApproveTaskInput,
    state: State<'_, AppSecurity>,
) -> Result<ComputeMutationResponse, String> {
    execute_approve_task(&input, &state).await
}

/// Raise a dispute for a completed task as the buyer within the challenge window.
#[tauri::command]
pub async fn compute_dispute_task(
    input: DisputeTaskInput,
    state: State<'_, AppSecurity>,
) -> Result<ComputeMutationResponse, String> {
    execute_dispute_task(&input, &state).await
}

/// Activate a registered Pending node via the ProofOfWorkVerifier challenge flow.
/// Runs issueChallenge → off-chain nonce solve → submitSolution in one command.
/// On success the node status transitions to Active with chain-derived computePower.
#[tauri::command]
pub async fn compute_verify_node(
    input: VerifyNodeInput,
    state: State<'_, AppSecurity>,
) -> Result<ComputeMutationResponse, String> {
    execute_verify_node(&input, &state).await
}
