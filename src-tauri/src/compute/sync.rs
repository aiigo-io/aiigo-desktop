/// Event-log scanning and canonical entity re-read from chain.
///
/// Forbidden:
///   - Starting from latest/head when DB has no cursor and no deploy block.
///   - Returning fresh empty when bootstrap config is missing.

use crate::compute::config::load_compute_config;
use crate::compute::db::{
    self, ComputeEventRecord, ComputeNodeRow, ComputeTaskRow,
};
use crate::compute::types::{
    ComputeConfig, ComputeSnapshot, ComputeSnapshotResponse, ComputeSyncOutcome, SyncCoverage,
    SyncCursor, SyncReason, SyncStatus,
};
use crate::DB;
use chrono::Utc;
use ethers::abi::{encode, Token};
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{
    Address, Bytes, Filter, Log, H256, U256,
};
use sha2::{Digest, Sha256};
use std::str::FromStr;

const COMPUTE_SCOPE_KEY_PREFIX: &str = "compute";
const MAX_BLOCKS_PER_BATCH: u64 = 2000;

fn scope_key(chain_id: u64, config: &ComputeConfig) -> String {
    // Include a hash of the contract addresses so that a contract redeployment
    // on the same chain_id doesn't silently reuse a stale cursor.
    let mut h = Sha256::new();
    h.update(config.task_marketplace_address.to_lowercase().as_bytes());
    h.update(config.node_registry_address.to_lowercase().as_bytes());
    h.update(config.escrow_manager_address.to_lowercase().as_bytes());
    let digest = h.finalize();
    format!("{}:{}:{}", COMPUTE_SCOPE_KEY_PREFIX, chain_id, &hex::encode(digest)[..8])
}

// ── Public entry points ─────────────────────────────────────────────────────

/// Read-only: build snapshot from cached DB data; never contact RPC.
pub fn query_snapshot_from_db(
    wallet_id: &str,
) -> Result<ComputeSnapshotResponse, String> {
    let config = match load_compute_config() {
        Ok(c) => c,
        Err(missing) => {
            return Ok(ComputeSnapshotResponse {
                snapshot: ComputeSnapshot::empty(),
                sync: unavailable_outcome(
                    SyncReason::Query,
                    &format!("configuration_missing: {}", missing.join(", ")),
                ),
            });
        }
    };

    let wallet_info = {
        let db = DB.lock().map_err(|e| e.to_string())?;
        db.get_evm_wallet(wallet_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "wallet_not_found".to_string())?
    };

    let cursor = DB.lock().map_err(|e| e.to_string())?
        .with_conn(|conn| db::load_sync_cursor(conn, &scope_key(config.chain_id, &config)))
        .map_err(|e| e.to_string())?;

    let snapshot = DB.lock().map_err(|e| e.to_string())?
        .with_conn(|conn| db::build_snapshot_from_db(conn, config.chain_id, &wallet_info.address))
        .map_err(|e| e.to_string())?;

    let sync = compute_query_outcome(&config, cursor.as_ref());
    Ok(ComputeSnapshotResponse { snapshot, sync })
}

/// Full RPC refresh: scan events, re-read entities, update read model and cursor.
pub async fn refresh_snapshot(
    wallet_id: &str,
) -> Result<ComputeSnapshotResponse, String> {
    let config = load_compute_config().map_err(|missing| {
        format!("configuration_missing: {}", missing.join(", "))
    })?;

    let wallet_info = {
        let db = DB.lock().map_err(|e| e.to_string())?;
        db.get_evm_wallet(wallet_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "wallet_not_found".to_string())?
    };

    let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
        .map_err(|e| format!("provider_init: {}", e))?;

    // Determine confirmed head
    let latest_block_num = provider
        .get_block_number()
        .await
        .map_err(|e| format!("rpc_get_block_number: {}", e))?
        .as_u64();

    if latest_block_num < config.confirmation_depth {
        return Ok(ComputeSnapshotResponse {
            snapshot: ComputeSnapshot::empty(),
            sync: unavailable_outcome(
                SyncReason::Manual,
                "confirmed_head_below_zero",
            ),
        });
    }
    let confirmed_head = latest_block_num - config.confirmation_depth;

    // Read cursor
    let key = scope_key(config.chain_id, &config);
    let existing_cursor = DB.lock().map_err(|e| e.to_string())?
        .with_conn(|conn| db::load_sync_cursor(conn, &key))
        .map_err(|e| e.to_string())?;

    // Compute start block: MUST be at least bootstrap_start_block.
    let bootstrap_start = config.bootstrap_start_block();
    let start_block = match &existing_cursor {
        Some(c) if c.synced_to_block.is_some() => {
            // Resume from last synced block + 1, but never before bootstrap.
            (c.synced_to_block.unwrap() + 1).max(bootstrap_start)
        }
        _ => bootstrap_start, // First sync always starts from deploy block.
    };

    if start_block > confirmed_head {
        // Already up to date — return current snapshot with stale/fresh from cursor.
        return build_no_new_blocks_response(&config, &wallet_info.address, existing_cursor.as_ref(), confirmed_head);
    }

    // ── Phase 1 (async): Scan events — nothing written to DB yet ───────────
    let scan_result = scan_events_batched(
        &provider,
        &config,
        start_block,
        confirmed_head,
    )
    .await;

    let (last_processed_block, last_processed_hash, collected_events, failed_sources) =
        match scan_result {
            Ok((block, hash, events)) => (Some(block), Some(hash), events, vec![]),
            Err(e) => {
                // Partial failure: keep cursor at last good block; nothing written.
                let last_good = existing_cursor.as_ref().and_then(|c| c.synced_to_block);
                let last_hash = existing_cursor.as_ref().and_then(|c| c.synced_to_block_hash.clone());
                (last_good, last_hash, vec![], vec![format!("event_scan:{}", e)])
            }
        };

    let partial = !failed_sources.is_empty();
    let now = Utc::now().to_rfc3339();

    // ── Phase 2 (async, only on success): Re-read entities from chain ───────
    // Nothing is written until Phase 3.
    if !partial {
        let entity_result =
            collect_entity_updates(&provider, &config, &wallet_info.address, &collected_events)
                .await;

        match entity_result {
            Ok((task_rows, node_rows, refund_wei, payout_wei)) => {
                // ── Phase 3: Single DB transaction — events + entities + cursor ──
                let cursor = SyncCursor {
                    scope_key: key,
                    chain_id: config.chain_id,
                    bootstrap_start_block: bootstrap_start,
                    synced_to_block: last_processed_block.or(Some(confirmed_head)),
                    synced_to_block_hash: last_processed_hash,
                    confirmed_head_block: Some(confirmed_head),
                    confirmation_depth: config.confirmation_depth,
                    status: SyncStatus::Fresh,
                    failed_sources: vec![],
                    updated_at: Some(now.clone()),
                };
                write_sync_batch(
                    &collected_events,
                    &task_rows,
                    &node_rows,
                    &refund_wei,
                    &payout_wei,
                    config.chain_id,
                    &wallet_info.address,
                    &now,
                    &cursor,
                )?;
                return build_snapshot_response(SyncReason::Manual, &config, &wallet_info.address, &cursor);
            }
            Err(e) => {
                // Entity re-read failed → partial; revert cursor to last-known-good block
                // so the next sync retries this range rather than skipping it.
                // [P2] Do NOT update updated_at — partial must not masquerade as a fresh refresh.
                let safe_synced_to = existing_cursor.as_ref().and_then(|c| c.synced_to_block);
                let safe_hash = existing_cursor.as_ref().and_then(|c| c.synced_to_block_hash.clone());
                let safe_updated_at = existing_cursor.as_ref().and_then(|c| c.updated_at.clone());
                let cursor = SyncCursor {
                    scope_key: key.clone(),
                    chain_id: config.chain_id,
                    bootstrap_start_block: bootstrap_start,
                    synced_to_block: safe_synced_to,
                    synced_to_block_hash: safe_hash,
                    confirmed_head_block: Some(confirmed_head),
                    confirmation_depth: config.confirmation_depth,
                    status: SyncStatus::Partial,
                    failed_sources: vec![format!("entity_sync:{}", e)],
                    updated_at: safe_updated_at,
                };
                persist_cursor(&cursor)?;
                return build_snapshot_response(SyncReason::Manual, &config, &wallet_info.address, &cursor);
            }
        }
    }

    // Partial path (event scan failed): cursor reverts to last-known-good block.
    // [P2] updated_at preserved from existing cursor — not masqueraded as a fresh refresh.
    let safe_updated_at = existing_cursor.as_ref().and_then(|c| c.updated_at.clone());
    let cursor = SyncCursor {
        scope_key: key,
        chain_id: config.chain_id,
        bootstrap_start_block: bootstrap_start,
        synced_to_block: last_processed_block,
        synced_to_block_hash: last_processed_hash,
        confirmed_head_block: Some(confirmed_head),
        confirmation_depth: config.confirmation_depth,
        status: SyncStatus::Partial,
        failed_sources,
        updated_at: safe_updated_at,
    };

    persist_cursor(&cursor)?;
    build_snapshot_response(SyncReason::Manual, &config, &wallet_info.address, &cursor)
}

// ── Event scanning ──────────────────────────────────────────────────────────

/// Scan all logs for the three contracts from `start_block..=end_block` in batches.
/// Returns the highest fully-processed block number, its hash, and all collected events.
/// Does NOT write to the database — the caller commits everything in one transaction.
async fn scan_events_batched(
    provider: &Provider<Http>,
    config: &ComputeConfig,
    start_block: u64,
    end_block: u64,
) -> Result<(u64, String, Vec<ComputeEventRecord>), String> {
    let task_addr = Address::from_str(&config.task_marketplace_address)
        .map_err(|e| format!("invalid_task_marketplace_address: {}", e))?;
    let node_addr = Address::from_str(&config.node_registry_address)
        .map_err(|e| format!("invalid_node_registry_address: {}", e))?;
    let escrow_addr = Address::from_str(&config.escrow_manager_address)
        .map_err(|e| format!("invalid_escrow_manager_address: {}", e))?;

    let mut current = start_block;
    let mut last_block = start_block;
    let mut last_hash = String::new();
    let mut all_events: Vec<ComputeEventRecord> = Vec::new();

    while current <= end_block {
        let batch_end = (current + MAX_BLOCKS_PER_BATCH - 1).min(end_block);

        let filter = Filter::new()
            .from_block(current)
            .to_block(batch_end)
            .address(vec![task_addr, node_addr, escrow_addr]);

        let logs = provider
            .get_logs(&filter)
            .await
            .map_err(|e| format!("get_logs [{}-{}]: {}", current, batch_end, e))?;

        let observed_at = Utc::now().to_rfc3339();
        for log in &logs {
            let event = log_to_event_record(log, config.chain_id, &observed_at)
                .map_err(|e| format!("log_malformed [{}-{}]: {}", current, batch_end, e))?;
            all_events.push(event);
        }

        // Get the block hash for the last block in this batch
        if let Ok(Some(block)) = provider.get_block(batch_end).await {
            last_hash = block
                .hash
                .map(|h| format!("{:?}", h))
                .unwrap_or_default();
        }

        last_block = batch_end;
        current = batch_end + 1;
    }

    Ok((last_block, last_hash, all_events))
}

pub(crate) fn log_to_event_record(log: &Log, chain_id: u64, observed_at: &str) -> Result<ComputeEventRecord, String> {
    let event_name = decode_event_name(log.topics.first());
    let (entity_kind, entity_id) = extract_entity_from_log(log, &event_name);
    let account_address = extract_account_from_log(log);

    // Required identity fields — a log without these cannot be stored as a
    // canonical event record (storing 0 / "" would corrupt event deduplication).
    let block_number = log
        .block_number
        .ok_or_else(|| "log_missing_block_number".to_string())?
        .as_u64();
    let block_hash = log
        .block_hash
        .map(|h| format!("{:?}", h))
        .ok_or_else(|| "log_missing_block_hash".to_string())?;
    let tx_hash = log
        .transaction_hash
        .map(|h| format!("{:?}", h))
        .ok_or_else(|| "log_missing_tx_hash".to_string())?;
    let log_index = log
        .log_index
        .ok_or_else(|| "log_missing_log_index".to_string())?
        .as_u32();

    Ok(ComputeEventRecord {
        chain_id,
        contract_address: format!("{:?}", log.address),
        block_number,
        block_hash,
        tx_hash,
        log_index,
        event_name,
        entity_kind,
        entity_id,
        account_address,
        payload_json: serde_json::json!({
            "topics": log.topics.iter().map(|t| format!("{:?}", t)).collect::<Vec<_>>(),
            "data": format!("0x{}", hex::encode(&log.data))
        })
        .to_string(),
        observed_at: observed_at.to_string(),
    })
}

/// Map the topic0 (event signature hash) to a human-readable name.
/// Uses runtime keccak256 so signatures stay correct even if contracts evolve.
fn decode_event_name(topic0: Option<&H256>) -> String {
    let Some(t0) = topic0 else {
        return "Unknown".to_string();
    };

    // Compute keccak256("EventSig(types...)") and compare at runtime.
    let k = |sig: &str| -> H256 {
        H256::from(ethers::utils::keccak256(sig.as_bytes()))
    };

    // Canonical ABI signatures (enums serialised as their underlying uint8).
    match *t0 {
        // TaskMarketplace events
        h if h == k("TaskCreated(bytes32,address,uint8,uint256)") => "TaskCreated",
        h if h == k("TaskAssigned(bytes32,bytes32,uint256)") => "TaskAssigned",
        h if h == k("TaskCompleted(bytes32,bytes32)") => "TaskCompleted",
        h if h == k("TaskVerified(bytes32,uint256)") => "TaskVerified",
        h if h == k("TaskDisputed(bytes32,address,string)") => "TaskDisputed",
        h if h == k("TaskCancelled(bytes32,uint256)") => "TaskCancelled",
        h if h == k("DisputeResolved(bytes32,address,uint256,uint256,uint256,uint256)") => "DisputeResolved",
        h if h == k("TaskUndisputedSettled(bytes32,address,uint256,uint256)") => "TaskUndisputedSettled",
        // NodeRegistry events
        h if h == k("NodeRegistered(bytes32,address,uint8)") => "NodeRegistered",
        h if h == k("NodeStatusChanged(bytes32,uint8,uint8)") => "NodeStatusChanged",
        h if h == k("NodeSlashed(bytes32,uint256,string)") => "NodeSlashed",
        h if h == k("ReputationUpdated(bytes32,uint256,uint256)") => "ReputationUpdated",
        h if h == k("ComputePowerUpdated(bytes32,uint256,uint256)") => "ComputePowerUpdated",
        h if h == k("StakeDeposited(bytes32,uint256,uint256)") => "StakeDeposited",
        h if h == k("StakeWithdrawn(bytes32,uint256,uint256)") => "StakeWithdrawn",
        h if h == k("StakeWithdrawalQueued(bytes32,address,uint256)") => "StakeWithdrawalQueued",
        h if h == k("StakeWithdrawalClaimed(address,uint256)") => "StakeWithdrawalClaimed",
        // ProofOfWorkVerifier events
        h if h == k("ChallengeIssued(bytes32,bytes32,uint256,uint256)") => "ChallengeIssued",
        h if h == k("ChallengeSolved(bytes32,bytes32,uint256,uint256)") => "ChallengeSolved",
        // EscrowManager events
        h if h == k("EscrowDeposited(bytes32,address,uint256)") => "EscrowDeposited",
        h if h == k("EscrowReleased(bytes32,address,uint256,uint256)") => "EscrowReleased",
        h if h == k("EscrowRefunded(bytes32,address,uint256)") => "EscrowRefunded",
        h if h == k("DisputeResolved(bytes32,address,uint256)") => "DisputeResolved",
        h if h == k("AccountingBucketMoved(bytes32,address,uint8,uint8,uint256)") => "AccountingBucketMoved",
        h if h == k("BuyerRefundQueued(bytes32,address,uint256)") => "BuyerRefundQueued",
        h if h == k("BuyerRefundClaimed(address,uint256)") => "BuyerRefundClaimed",
        h if h == k("ProviderPayoutQueued(bytes32,address,uint256)") => "ProviderPayoutQueued",
        h if h == k("ProviderPayoutClaimed(address,uint256)") => "ProviderPayoutClaimed",
        _ => "Unknown",
    }
    .to_string()
}

fn extract_entity_from_log(log: &Log, event_name: &str) -> (Option<String>, Option<String>) {
    // For events with indexed task_id or node_id as first indexed param (topic[1])
    match event_name {
        "TaskCreated" | "TaskAssigned" | "TaskCompleted" | "TaskVerified"
        | "TaskDisputed" | "TaskCancelled" | "DisputeResolved" | "TaskUndisputedSettled" => {
            let entity_id = log.topics.get(1).map(|t| format!("{:?}", t));
            (Some("task".to_string()), entity_id)
        }
        "NodeRegistered" | "NodeStatusChanged" | "NodeSlashed"
        | "ReputationUpdated" | "ComputePowerUpdated"
        | "StakeDeposited" | "StakeWithdrawn" | "StakeWithdrawalQueued" => {
            let entity_id = log.topics.get(1).map(|t| format!("{:?}", t));
            (Some("node".to_string()), entity_id)
        }
        // ChallengeSolved(bytes32 indexed challengeId, bytes32 indexed nodeId, ...)
        // nodeId is topic[2]; topic[1] is challengeId.
        "ChallengeSolved" => {
            let entity_id = log.topics.get(2).map(|t| format!("{:?}", t));
            (Some("node".to_string()), entity_id)
        }
        "EscrowDeposited" | "EscrowReleased" | "EscrowRefunded"
        | "AccountingBucketMoved" | "BuyerRefundQueued" | "ProviderPayoutQueued" => {
            let entity_id = log.topics.get(1).map(|t| format!("{:?}", t));
            (Some("escrow".to_string()), entity_id)
        }
        // Account-level events (no task/node id, identify by address)
        "StakeWithdrawalClaimed" | "BuyerRefundClaimed" | "ProviderPayoutClaimed" => {
            (Some("account".to_string()), None)
        }
        _ => (None, None),
    }
}

fn extract_account_from_log(log: &Log) -> Option<String> {
    // buyer/owner is typically topic[2] for task/node events
    log.topics.get(2).map(|t| {
        // Address is padded to 32 bytes — extract last 20 bytes
        let t_bytes = t.as_bytes();
        format!("0x{}", hex::encode(&t_bytes[12..]))
    })
}

// ── Canonical entity re-read from chain ────────────────────────────────────

/// After scanning events, re-read canonical state for all affected entities.
/// Returns collected rows WITHOUT writing to DB; the caller commits everything
/// in a single transaction via write_sync_batch or write_projection_batch.
pub(crate) async fn collect_entity_updates(
    provider: &Provider<Http>,
    config: &ComputeConfig,
    wallet_address: &str,
    events: &[ComputeEventRecord],
) -> Result<(Vec<ComputeTaskRow>, Vec<ComputeNodeRow>, String, String), String> {
    // Derive affected entity IDs directly from the collected events (no DB query).
    let affected_task_ids: Vec<String> = {
        let mut ids: Vec<String> = events
            .iter()
            .filter(|e| e.entity_kind.as_deref() == Some("task"))
            .filter_map(|e| e.entity_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    };
    let affected_node_ids: Vec<String> = {
        let mut ids: Vec<String> = events
            .iter()
            .filter(|e| e.entity_kind.as_deref() == Some("node"))
            .filter_map(|e| e.entity_id.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    };

    let task_marketplace = Address::from_str(&config.task_marketplace_address)
        .map_err(|e| format!("invalid_task_marketplace_address: {}", e))?;
    let node_registry = Address::from_str(&config.node_registry_address)
        .map_err(|e| format!("invalid_node_registry_address: {}", e))?;
    let escrow_manager = Address::from_str(&config.escrow_manager_address)
        .map_err(|e| format!("invalid_escrow_manager_address: {}", e))?;

    let now = Utc::now().to_rfc3339();

    let mut task_rows: Vec<ComputeTaskRow> = Vec::with_capacity(affected_task_ids.len());
    for task_id_hex in &affected_task_ids {
        let task_row = read_task_from_chain(
            provider,
            task_marketplace,
            task_id_hex,
            config.chain_id,
            &now,
        )
        .await
        .map_err(|e| format!("task_reread:{task_id_hex}:{e}"))?;
        task_rows.push(task_row);
    }

    let mut node_rows: Vec<ComputeNodeRow> = Vec::with_capacity(affected_node_ids.len());
    for node_id_hex in &affected_node_ids {
        let node_row = read_node_from_chain(
            provider,
            node_registry,
            node_id_hex,
            config.chain_id,
            &now,
        )
        .await
        .map_err(|e| format!("node_reread:{node_id_hex}:{e}"))?;
        node_rows.push(node_row);
    }

    // Read account summary values (eth_call only, no DB write here)
    let (refund_wei, payout_wei) = read_account_summary_values(
        provider,
        escrow_manager,
        wallet_address,
    ).await?;

    Ok((task_rows, node_rows, refund_wei, payout_wei))
}

/// Write all sync results atomically in a SINGLE DB transaction:
/// events + entity read-model rows + account summary + cursor.
/// Crash safety: either everything is committed or nothing is.
fn write_sync_batch(
    events: &[ComputeEventRecord],
    task_rows: &[ComputeTaskRow],
    node_rows: &[ComputeNodeRow],
    refund_wei: &str,
    payout_wei: &str,
    chain_id: u64,
    wallet_address: &str,
    now: &str,
    cursor: &SyncCursor,
) -> Result<(), String> {
    DB.lock()
        .map_err(|e| e.to_string())?
        .with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            for ev in events {
                db::upsert_compute_event(&tx, ev)?;
            }
            for task in task_rows {
                db::upsert_compute_task(&tx, task)?;
            }
            for node in node_rows {
                db::upsert_compute_node(&tx, node)?;
            }
            db::upsert_account_summary(
                &tx, chain_id, wallet_address, refund_wei, payout_wei, "0", now,
            )?;
            db::upsert_sync_cursor(&tx, cursor)?;
            tx.commit()
        })
        .map_err(|e| format!("write_sync_batch: {}", e))
}

/// Write projection batch atomically: events + entity read-model rows + account summary.
/// Does NOT write compute_sync_cursors — targeted projection must not advance the sync cursor.
/// Crash safety: either everything is committed or nothing is.
///
/// Forbidden: must not write compute_sync_cursors.
pub(crate) fn write_projection_batch(
    events: &[ComputeEventRecord],
    task_rows: &[ComputeTaskRow],
    node_rows: &[ComputeNodeRow],
    refund_wei: &str,
    payout_wei: &str,
    chain_id: u64,
    wallet_address: &str,
    now: &str,
) -> Result<(), String> {
    DB.lock()
        .map_err(|e| e.to_string())?
        .with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            for ev in events {
                db::upsert_compute_event(&tx, ev)?;
            }
            for task in task_rows {
                db::upsert_compute_task(&tx, task)?;
            }
            for node in node_rows {
                db::upsert_compute_node(&tx, node)?;
            }
            db::upsert_account_summary(
                &tx, chain_id, wallet_address, refund_wei, payout_wei, "0", now,
            )?;
            tx.commit()
        })
        .map_err(|e| format!("write_projection_batch: {}", e))
}

async fn read_account_summary_values(
    provider: &Provider<Http>,
    escrow_manager: Address,
    wallet_address: &str,
) -> Result<(String, String), String> {
    let addr_token = address_as_token(wallet_address);
    let args = encode(&[addr_token.clone()]);

    let sel_refund = selector("getPendingBuyerRefund(address)");
    let mut cd_refund = sel_refund.to_vec();
    cd_refund.extend_from_slice(&args);
    let refund_bytes = eth_call_bytes(provider, escrow_manager, cd_refund)
        .await
        .map_err(|e| format!("getPendingBuyerRefund RPC failed: {e}"))?;
    let refund_wei = decode_single_u256(&refund_bytes)
        .map(|v| v.to_string())
        .ok_or_else(|| "failed to decode refund U256".to_string())?;

    let sel_payout = selector("getPendingProviderPayout(address)");
    let mut cd_payout = sel_payout.to_vec();
    cd_payout.extend_from_slice(&encode(&[addr_token]));
    let payout_bytes = eth_call_bytes(provider, escrow_manager, cd_payout)
        .await
        .map_err(|e| format!("getPendingProviderPayout RPC failed: {e}"))?;
    let payout_wei = decode_single_u256(&payout_bytes)
        .map(|v| v.to_string())
        .ok_or_else(|| "failed to decode payout U256".to_string())?;

    Ok((refund_wei, payout_wei))
}

// ── eth_call helpers ────────────────────────────────────────────────────────

async fn eth_call_bytes(
    provider: &Provider<Http>,
    to: Address,
    calldata: Vec<u8>,
) -> Result<Bytes, String> {
    let tx = ethers::types::transaction::eip2718::TypedTransaction::Legacy(
        ethers::types::TransactionRequest::new()
            .to(to)
            .data(calldata),
    );
    provider
        .call(&tx, None)
        .await
        .map_err(|e| format!("eth_call: {}", e))
}

/// Build 4-byte selector from function signature string.
fn selector(sig: &str) -> [u8; 4] {
    let hash = ethers::utils::keccak256(sig.as_bytes());
    [hash[0], hash[1], hash[2], hash[3]]
}

fn bytes32_as_token(hex_str: &str) -> Token {
    let cleaned = hex_str.trim_start_matches("0x");
    let padded = format!("{:0>64}", cleaned);
    let bytes = hex::decode(&padded).unwrap_or_else(|_| vec![0u8; 32]);
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes[..32]);
    Token::FixedBytes(arr.to_vec())
}

fn address_as_token(hex_str: &str) -> Token {
    let addr = Address::from_str(hex_str).unwrap_or(Address::zero());
    Token::Address(addr)
}

pub(crate) async fn read_task_from_chain(
    provider: &Provider<Http>,
    contract: Address,
    task_id_hex: &str,
    chain_id: u64,
    now: &str,
) -> Result<ComputeTaskRow, String> {
    // getTask(bytes32)
    let sel = selector("getTask(bytes32)");
    let args = encode(&[bytes32_as_token(task_id_hex)]);
    let mut calldata = sel.to_vec();
    calldata.extend_from_slice(&args);

    let result = eth_call_bytes(provider, contract, calldata).await?;
    let task = decode_task_struct(&result, task_id_hex, chain_id, now)?;

    // getTaskLifecycle(bytes32)
    let sel2 = selector("getTaskLifecycle(bytes32)");
    let args2 = encode(&[bytes32_as_token(task_id_hex)]);
    let mut calldata2 = sel2.to_vec();
    calldata2.extend_from_slice(&args2);

    let lifecycle_bytes = eth_call_bytes(provider, contract, calldata2).await?;
    let task = merge_task_lifecycle(task, &lifecycle_bytes);

    Ok(task)
}

fn decode_task_struct(
    data: &Bytes,
    task_id_hex: &str,
    chain_id: u64,
    now: &str,
) -> Result<ComputeTaskRow, String> {
    use ethers::abi::{decode as abi_decode, ParamType};

    // Task struct layout (from solidity):
    // bytes32 taskId, address buyer, bytes32 assignedNode, uint8 resourceType,
    // uint256 requiredPower, uint256 duration, uint256 maxPrice, uint256 escrowAmount,
    // uint256 createdAt, uint256 startedAt, uint256 completedAt, uint8 status,
    // uint8 minTrustLevel, string specificationURI
    let types = vec![
        ParamType::Tuple(vec![
            ParamType::FixedBytes(32), // taskId
            ParamType::Address,        // buyer
            ParamType::FixedBytes(32), // assignedNode
            ParamType::Uint(8),        // resourceType
            ParamType::Uint(256),      // requiredPower
            ParamType::Uint(256),      // duration
            ParamType::Uint(256),      // maxPrice
            ParamType::Uint(256),      // escrowAmount
            ParamType::Uint(256),      // createdAt
            ParamType::Uint(256),      // startedAt
            ParamType::Uint(256),      // completedAt
            ParamType::Uint(8),        // status
            ParamType::Uint(8),        // minTrustLevel
            ParamType::String,         // specificationURI
        ]),
    ];

    let decoded = abi_decode(&types, data).map_err(|e| format!("decode_task: {}", e))?;
    let tuple = match decoded.into_iter().next() {
        Some(Token::Tuple(t)) => t,
        _ => return Err("decode_task: not a tuple".to_string()),
    };

    let buyer = match &tuple[1] {
        Token::Address(a) => format!("{:?}", a),
        _ => return Err("decode_task: buyer not address".to_string()),
    };
    let assigned_raw = match &tuple[2] {
        Token::FixedBytes(b) => hex::encode(b),
        _ => "0".repeat(64),
    };
    let zero32 = "0".repeat(64);
    let assigned_node_id = if assigned_raw == zero32 {
        None
    } else {
        Some(format!("0x{}", assigned_raw))
    };
    let resource_type = match &tuple[3] {
        Token::Uint(v) => v.as_u64() as u8,
        _ => 0,
    };
    let required_power = match &tuple[4] {
        Token::Uint(v) => v.to_string(),
        _ => "0".to_string(),
    };
    let duration = match &tuple[5] {
        Token::Uint(v) => v.as_u64(),
        _ => 0,
    };
    let max_price_wei = match &tuple[6] {
        Token::Uint(v) => v.to_string(),
        _ => "0".to_string(),
    };
    let escrow_amount_wei = match &tuple[7] {
        Token::Uint(v) => v.to_string(),
        _ => "0".to_string(),
    };
    let created_at = match &tuple[8] {
        Token::Uint(v) if !v.is_zero() => Some(v.as_u64()),
        _ => None,
    };
    let started_at = match &tuple[9] {
        Token::Uint(v) if !v.is_zero() => Some(v.as_u64()),
        _ => None,
    };
    let completed_at = match &tuple[10] {
        Token::Uint(v) if !v.is_zero() => Some(v.as_u64()),
        _ => None,
    };
    let status_code = match &tuple[11] {
        Token::Uint(v) => v.as_u64() as u8,
        _ => 0,
    };
    let min_trust_level = match &tuple[12] {
        Token::Uint(v) => v.as_u64() as u8,
        _ => 0,
    };
    let specification_uri = match &tuple[13] {
        Token::String(s) => s.clone(),
        _ => "".to_string(),
    };

    let status_str = crate::compute::types::TaskStatus::from_u8(status_code)
        .as_str()
        .to_string();

    Ok(ComputeTaskRow {
        chain_id,
        task_id: task_id_hex.to_string(),
        buyer,
        assigned_node_id,
        resource_type,
        required_power,
        duration_seconds: duration,
        max_price_wei,
        escrow_amount_wei,
        status: status_str,
        specification_uri,
        min_trust_level,
        created_at_ts: created_at,
        started_at_ts: started_at,
        completed_at_ts: completed_at,
        challenge_deadline_ts: None, // filled by lifecycle
        dispute_reason: None,
        disputed_by: None,
        resolved: false,
        resolved_by: None,
        gross_provider_amount_wei: "0".to_string(),
        last_chain_block: None,
        last_chain_block_hash: None,
        synced_at: now.to_string(),
    })
}

fn merge_task_lifecycle(mut task: ComputeTaskRow, data: &Bytes) -> ComputeTaskRow {
    use ethers::abi::{decode as abi_decode, ParamType};

    // TaskLifecycle: uint256 challengeDeadline, address disputedBy, string disputeReason,
    //               bool resolved, address resolvedBy, uint256 grossProviderAmount
    let types = vec![ParamType::Tuple(vec![
        ParamType::Uint(256), // challengeDeadline
        ParamType::Address,   // disputedBy
        ParamType::String,    // disputeReason
        ParamType::Bool,      // resolved
        ParamType::Address,   // resolvedBy
        ParamType::Uint(256), // grossProviderAmount
    ])];

    let Ok(decoded) = abi_decode(&types, data) else {
        return task;
    };
    let Token::Tuple(t) = decoded.into_iter().next().unwrap_or(Token::Bool(false)) else {
        return task;
    };

    if let Some(Token::Uint(v)) = t.first() {
        if !v.is_zero() {
            task.challenge_deadline_ts = Some(v.as_u64());
        }
    }
    let zero_addr = format!("{:?}", Address::zero());
    if let Some(Token::Address(a)) = t.get(1) {
        let s = format!("{:?}", a);
        if s != zero_addr {
            task.disputed_by = Some(s);
        }
    }
    if let Some(Token::String(s)) = t.get(2) {
        if !s.is_empty() {
            task.dispute_reason = Some(s.clone());
        }
    }
    if let Some(Token::Bool(r)) = t.get(3) {
        task.resolved = *r;
    }
    if let Some(Token::Address(a)) = t.get(4) {
        let s = format!("{:?}", a);
        if s != zero_addr {
            task.resolved_by = Some(s);
        }
    }
    if let Some(Token::Uint(v)) = t.get(5) {
        task.gross_provider_amount_wei = v.to_string();
    }

    task
}

async fn read_node_from_chain(
    provider: &Provider<Http>,
    contract: Address,
    node_id_hex: &str,
    chain_id: u64,
    now: &str,
) -> Result<ComputeNodeRow, String> {
    // getNode(bytes32)
    let sel = selector("getNode(bytes32)");
    let args = encode(&[bytes32_as_token(node_id_hex)]);
    let mut calldata = sel.to_vec();
    calldata.extend_from_slice(&args);
    let data = eth_call_bytes(provider, contract, calldata).await?;
    let mut node_row = decode_node_struct(&data, node_id_hex, chain_id, now)?;

    // getNodeTrustLevel(bytes32)
    let sel2 = selector("getNodeTrustLevel(bytes32)");
    let args2 = encode(&[bytes32_as_token(node_id_hex)]);
    let mut calldata2 = sel2.to_vec();
    calldata2.extend_from_slice(&args2);
    if let Ok(tl_bytes) = eth_call_bytes(provider, contract, calldata2).await {
        if let Some(v) = decode_single_u256(&tl_bytes) {
            node_row.trust_level = v.as_u64() as u8;
        }
    }

    Ok(node_row)
}

fn decode_node_struct(
    data: &Bytes,
    node_id_hex: &str,
    chain_id: u64,
    now: &str,
) -> Result<ComputeNodeRow, String> {
    use ethers::abi::{decode as abi_decode, ParamType};

    // Node: address owner, bytes32 nodeId, NodeStatus status, ResourceType resourceType,
    //       uint256 computePower, uint256 stakedAmount, uint256 reputation,
    //       uint256 totalTasksCompleted, uint256 totalEarnings,
    //       uint256 registeredAt, uint256 lastActiveAt, string metadataURI
    let types = vec![ParamType::Tuple(vec![
        ParamType::Address,    // owner
        ParamType::FixedBytes(32), // nodeId
        ParamType::Uint(8),    // status
        ParamType::Uint(8),    // resourceType
        ParamType::Uint(256),  // computePower
        ParamType::Uint(256),  // stakedAmount
        ParamType::Uint(256),  // reputation
        ParamType::Uint(256),  // totalTasksCompleted
        ParamType::Uint(256),  // totalEarnings
        ParamType::Uint(256),  // registeredAt
        ParamType::Uint(256),  // lastActiveAt
        ParamType::String,     // metadataURI
    ])];

    let decoded = abi_decode(&types, data).map_err(|e| format!("decode_node: {}", e))?;
    let Token::Tuple(t) = decoded.into_iter().next().ok_or("decode_node: empty")? else {
        return Err("decode_node: not tuple".to_string());
    };

    let owner = match &t[0] {
        Token::Address(a) => format!("{:?}", a),
        _ => return Err("decode_node: owner".to_string()),
    };
    let status_code = match &t[2] {
        Token::Uint(v) => v.as_u64() as u8,
        _ => 0,
    };
    let resource_type = match &t[3] {
        Token::Uint(v) => v.as_u64() as u8,
        _ => 0,
    };

    let status_str = match status_code {
        0 => "Pending",
        1 => "Verified",
        2 => "Active",
        3 => "Inactive",
        _ => "Slashed",
    };

    Ok(ComputeNodeRow {
        chain_id,
        node_id: node_id_hex.to_string(),
        owner,
        status: status_str.to_string(),
        resource_type,
        compute_power: token_u256_str(&t[4]),
        staked_amount_wei: token_u256_str(&t[5]),
        reputation: token_u256_str(&t[6]),
        total_tasks_completed: token_u256_u64(&t[7]),
        total_earnings_wei: token_u256_str(&t[8]),
        registered_at_ts: token_u256_opt_ts(&t[9]),
        last_active_at_ts: token_u256_opt_ts(&t[10]),
        metadata_uri: match &t[11] {
            Token::String(s) => s.clone(),
            _ => "".to_string(),
        },
        trust_level: 0, // filled after getNodeTrustLevel call
        pending_task_count: 0,
        last_chain_block: None,
        last_chain_block_hash: None,
        synced_at: now.to_string(),
    })
}

async fn sync_account_summary(
    provider: &Provider<Http>,
    escrow_manager: Address,
    wallet_address: &str,
    chain_id: u64,
    now: &str,
) -> Result<(), String> {
    let (refund_wei, payout_wei) = read_account_summary_values(
        provider, escrow_manager, wallet_address
    ).await?;

    DB.lock().map_err(|e| e.to_string())?
        .with_conn(|conn| {
            db::upsert_account_summary(
                conn, chain_id, wallet_address, &refund_wei, &payout_wei, "0", now,
            )
        })
        .map_err(|e| e.to_string())
}

// ── ABI decoding helpers ────────────────────────────────────────────────────

fn decode_single_u256(data: &Bytes) -> Option<U256> {
    use ethers::abi::{decode as abi_decode, ParamType};
    abi_decode(&[ParamType::Uint(256)], data)
        .ok()
        .and_then(|mut v| v.pop())
        .and_then(|t| match t {
            Token::Uint(u) => Some(u),
            _ => None,
        })
}

fn token_u256_str(t: &Token) -> String {
    match t {
        Token::Uint(v) => v.to_string(),
        _ => "0".to_string(),
    }
}

fn token_u256_u64(t: &Token) -> u64 {
    match t {
        Token::Uint(v) => v.as_u64(),
        _ => 0,
    }
}

fn token_u256_opt_ts(t: &Token) -> Option<u64> {
    match t {
        Token::Uint(v) if !v.is_zero() => Some(v.as_u64()),
        _ => None,
    }
}

// ── Outcome helpers ─────────────────────────────────────────────────────────

fn unavailable_outcome(reason: SyncReason, detail: &str) -> ComputeSyncOutcome {
    ComputeSyncOutcome {
        reason,
        target: "compute_marketplace".to_string(),
        status: SyncStatus::Unavailable,
        updated_at: None,
        bootstrap_start_block: None,
        synced_to_block: None,
        synced_to_block_hash: None,
        confirmed_head_block: None,
        confirmation_depth: None,
        partial: false,
        failed_sources: vec![detail.to_string()],
        coverage: SyncCoverage::Unavailable,
    }
}

fn compute_query_outcome(
    config: &ComputeConfig,
    cursor: Option<&SyncCursor>,
) -> ComputeSyncOutcome {
    let Some(c) = cursor else {
        return ComputeSyncOutcome {
            reason: SyncReason::Query,
            target: "compute_marketplace".to_string(),
            status: SyncStatus::Unavailable,
            updated_at: None,
            bootstrap_start_block: Some(config.bootstrap_start_block()),
            synced_to_block: None,
            synced_to_block_hash: None,
            confirmed_head_block: None,
            confirmation_depth: Some(config.confirmation_depth),
            partial: false,
            failed_sources: vec!["never_synced".to_string()],
            coverage: SyncCoverage::Unavailable,
        };
    };

    let coverage = if c.synced_to_block.is_some() {
        SyncCoverage::FullEventHistory
    } else {
        SyncCoverage::Unavailable
    };

    ComputeSyncOutcome {
        reason: SyncReason::Query,
        target: "compute_marketplace".to_string(),
        status: c.status.clone(),
        updated_at: c.updated_at.clone(),
        bootstrap_start_block: Some(c.bootstrap_start_block),
        synced_to_block: c.synced_to_block,
        synced_to_block_hash: c.synced_to_block_hash.clone(),
        confirmed_head_block: c.confirmed_head_block,
        confirmation_depth: Some(c.confirmation_depth),
        partial: !c.failed_sources.is_empty(),
        failed_sources: c.failed_sources.clone(),
        coverage,
    }
}

fn build_no_new_blocks_response(
    config: &ComputeConfig,
    wallet_address: &str,
    cursor: Option<&SyncCursor>,
    confirmed_head: u64,
) -> Result<ComputeSnapshotResponse, String> {
    let snapshot = DB.lock().map_err(|e| e.to_string())?
        .with_conn(|conn| db::build_snapshot_from_db(conn, config.chain_id, wallet_address))
        .map_err(|e| e.to_string())?;

    let mut outcome = compute_query_outcome(config, cursor);
    outcome.reason = SyncReason::Manual;
    outcome.confirmed_head_block = Some(confirmed_head);
    Ok(ComputeSnapshotResponse { snapshot, sync: outcome })
}

fn persist_cursor(cursor: &SyncCursor) -> Result<(), String> {
    DB.lock().map_err(|e| e.to_string())?
        .with_conn(|conn| db::upsert_sync_cursor(conn, cursor))
        .map_err(|e| e.to_string())
}

fn build_snapshot_response(
    reason: SyncReason,
    config: &ComputeConfig,
    wallet_address: &str,
    cursor: &SyncCursor,
) -> Result<ComputeSnapshotResponse, String> {
    let snapshot = DB.lock().map_err(|e| e.to_string())?
        .with_conn(|conn| db::build_snapshot_from_db(conn, config.chain_id, wallet_address))
        .map_err(|e| e.to_string())?;

    let partial = !cursor.failed_sources.is_empty();
    let coverage = if cursor.synced_to_block.is_some() && !partial {
        SyncCoverage::FullEventHistory
    } else if cursor.synced_to_block.is_some() {
        SyncCoverage::CachedOnly
    } else {
        SyncCoverage::Unavailable
    };

    let sync = ComputeSyncOutcome {
        reason,
        target: "compute_marketplace".to_string(),
        status: cursor.status.clone(),
        updated_at: cursor.updated_at.clone(),
        bootstrap_start_block: Some(cursor.bootstrap_start_block),
        synced_to_block: cursor.synced_to_block,
        synced_to_block_hash: cursor.synced_to_block_hash.clone(),
        confirmed_head_block: cursor.confirmed_head_block,
        confirmation_depth: Some(cursor.confirmation_depth),
        partial,
        failed_sources: cursor.failed_sources.clone(),
        coverage,
    };

    Ok(ComputeSnapshotResponse { snapshot, sync })
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::db::init_compute_tables;
    use crate::compute::types::SyncStatus;

    fn open_mem_db_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        init_compute_tables(&conn).unwrap();
        conn
    }

    fn make_event(tx_hash: &str, log_index: u32) -> ComputeEventRecord {
        ComputeEventRecord {
            chain_id: 11155111,
            contract_address: "0xtaskmp".to_string(),
            block_number: 100,
            block_hash: "0xbh".to_string(),
            tx_hash: tx_hash.to_string(),
            log_index,
            event_name: "TaskCreated".to_string(),
            entity_kind: Some("task".to_string()),
            entity_id: Some("0xtask1".to_string()),
            account_address: Some("0xbuyer".to_string()),
            payload_json: "{}".to_string(),
            observed_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    // ── Task 2 Tests ──────────────────────────────────────────────────────────

    #[test]
    fn receipt_projection_upserts_events_idempotently() {
        // write_projection_batch called twice with same event must not double-insert.
        let conn = open_mem_db_conn();
        let events = vec![make_event("0xtx1", 0), make_event("0xtx1", 1)];

        for _ in 0..2 {
            let tx = conn.unchecked_transaction().unwrap();
            for ev in &events {
                db::upsert_compute_event(&tx, ev).unwrap();
            }
            tx.commit().unwrap();
        }

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM compute_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2, "Duplicate event upsert must be idempotent: expected 2 rows");
    }

    #[test]
    fn receipt_projection_does_not_advance_sync_cursor() {
        // write_projection_batch primitives must not touch compute_sync_cursors.
        let conn = open_mem_db_conn();

        let initial_cursor = SyncCursor {
            scope_key: "compute:11155111:abc12345".to_string(),
            chain_id: 11155111,
            bootstrap_start_block: 10,
            synced_to_block: Some(50),
            synced_to_block_hash: Some("0xsync50".to_string()),
            confirmed_head_block: Some(62),
            confirmation_depth: 12,
            status: SyncStatus::Fresh,
            failed_sources: vec![],
            updated_at: Some("2024-05-01T00:00:00Z".to_string()),
        };
        db::upsert_sync_cursor(&conn, &initial_cursor).unwrap();

        // Simulate write_projection_batch: events + account summary (no cursor write).
        let events = vec![make_event("0xtx_projection", 0)];
        let tx = conn.unchecked_transaction().unwrap();
        for ev in &events {
            db::upsert_compute_event(&tx, ev).unwrap();
        }
        db::upsert_account_summary(&tx, 11155111, "0xbuyer", "0", "0", "0", "2024-05-01T01:00:00Z").unwrap();
        tx.commit().unwrap();

        // Cursor must be unchanged.
        let loaded = db::load_sync_cursor(&conn, "compute:11155111:abc12345")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.synced_to_block, Some(50), "cursor must not advance after projection");
        assert_eq!(
            loaded.synced_to_block_hash.as_deref(),
            Some("0xsync50"),
            "cursor hash must not change"
        );
    }

    // ── Task 2 negative tests: fallible log_to_event_record ─────────────────

    /// A log with all required identity fields populated returns Ok.
    #[test]
    fn log_to_event_record_succeeds_with_all_fields() {
        let mut log = Log::default();
        log.block_number = Some(ethers::types::U64::from(42u64));
        log.block_hash = Some(H256::zero());
        log.transaction_hash = Some(H256::zero());
        log.log_index = Some(ethers::types::U256::zero());

        let result = log_to_event_record(&log, 1, "2024-01-01T00:00:00Z");
        assert!(result.is_ok(), "fully-populated log must return Ok, got: {:?}", result.err());
    }

    /// A log missing block_number must return Err — zero must never be stored.
    #[test]
    fn log_to_event_record_rejects_missing_block_number() {
        let mut log = Log::default();
        log.block_number = None; // deliberately absent
        log.block_hash = Some(H256::zero());
        log.transaction_hash = Some(H256::zero());
        log.log_index = Some(ethers::types::U256::zero());

        let err = log_to_event_record(&log, 1, "2024-01-01T00:00:00Z")
            .expect_err("missing block_number must return Err");
        assert!(err.contains("log_missing_block_number"), "error must name the missing field, got: {err}");
    }

    /// A log missing block_hash must return Err — empty string must never be stored.
    #[test]
    fn log_to_event_record_rejects_missing_block_hash() {
        let mut log = Log::default();
        log.block_number = Some(ethers::types::U64::from(42u64));
        log.block_hash = None; // deliberately absent
        log.transaction_hash = Some(H256::zero());
        log.log_index = Some(ethers::types::U256::zero());

        let err = log_to_event_record(&log, 1, "2024-01-01T00:00:00Z")
            .expect_err("missing block_hash must return Err");
        assert!(err.contains("log_missing_block_hash"), "error must name the missing field, got: {err}");
    }

    /// A log missing transaction_hash must return Err — empty string must never be stored.
    #[test]
    fn log_to_event_record_rejects_missing_tx_hash() {
        let mut log = Log::default();
        log.block_number = Some(ethers::types::U64::from(42u64));
        log.block_hash = Some(H256::zero());
        log.transaction_hash = None; // deliberately absent
        log.log_index = Some(ethers::types::U256::zero());

        let err = log_to_event_record(&log, 1, "2024-01-01T00:00:00Z")
            .expect_err("missing tx_hash must return Err");
        assert!(err.contains("log_missing_tx_hash"), "error must name the missing field, got: {err}");
    }

    /// A log missing log_index must return Err — zero must never be stored.
    #[test]
    fn log_to_event_record_rejects_missing_log_index() {
        let mut log = Log::default();
        log.block_number = Some(ethers::types::U64::from(42u64));
        log.block_hash = Some(H256::zero());
        log.transaction_hash = Some(H256::zero());
        log.log_index = None; // deliberately absent

        let err = log_to_event_record(&log, 1, "2024-01-01T00:00:00Z")
            .expect_err("missing log_index must return Err");
        assert!(err.contains("log_missing_log_index"), "error must name the missing field, got: {err}");
    }
}
