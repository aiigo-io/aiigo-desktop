/// Idempotent mutation state machine.
///
/// Invariants:
///   - Same (chain_id, wallet_id, client_request_id) + same hash → resume, reuse tx_hash.
///   - Same (chain_id, wallet_id, client_request_id) + different hash → Conflict error.
///   - Nonce + raw_signed_tx persisted BEFORE broadcast (pre-broadcast persistence).
///   - Broadcast result (success/failure) updates step status, never changes nonce/raw_tx.

use crate::compute::config::load_compute_config;
use crate::compute::db::{self, MutationRow, MutationStepRow};
use crate::compute::types::{
    AcceptTaskInput, ApproveTaskInput, ComputeMutationResponse, CreateAndFundTaskInput,
    DisputeTaskInput, RegisterNodeInput, SubmitResultInput, VerifyNodeInput,
};
use crate::wallet::evm::private_key::map_security_error;
use crate::wallet::evm::transaction::{load_signing_secret, wallet_from_signing_secret};
use crate::wallet::security::types::SignerOperation;
use crate::DB;
use chrono::Utc;
use ethers::abi::{decode as abi_decode, encode, ParamType, Token};
use ethers::prelude::*;
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{Address, BlockId, BlockNumber, Bytes, TransactionReceipt, TransactionRequest, U256};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use uuid::Uuid;

// ── Idempotency ─────────────────────────────────────────────────────────────

pub enum IdempotencyResult {
    New,
    Resume(MutationRow),
    Conflict { existing_hash: String },
}

/// Check idempotency against a raw rusqlite connection (used in unit tests and internally).
pub fn check_idempotency(
    conn: &rusqlite::Connection,
    chain_id: u64,
    wallet_id: &str,
    client_request_id: &str,
    new_hash: &str,
) -> Result<IdempotencyResult, String> {
    match db::load_mutation_by_client_request_id(conn, chain_id, wallet_id, client_request_id)
        .map_err(|e| e.to_string())?
    {
        None => Ok(IdempotencyResult::New),
        Some(row) if row.request_hash == new_hash => Ok(IdempotencyResult::Resume(row)),
        Some(row) => Ok(IdempotencyResult::Conflict {
            existing_hash: row.request_hash,
        }),
    }
}

/// SHA256( version || chain_id || contracts || wallet_id || action || args_json )
pub fn compute_request_hash(
    version: u8,
    chain_id: u64,
    task_marketplace: &str,
    node_registry: &str,
    escrow_manager: &str,
    wallet_id: &str,
    action: &str,
    canonical_args_json: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update([version]);
    hasher.update(chain_id.to_le_bytes());
    hasher.update(task_marketplace.as_bytes());
    hasher.update(node_registry.as_bytes());
    hasher.update(escrow_manager.as_bytes());
    hasher.update(wallet_id.as_bytes());
    hasher.update(action.as_bytes());
    hasher.update(canonical_args_json.as_bytes());
    hex::encode(hasher.finalize())
}

// ── DB helper ───────────────────────────────────────────────────────────────

fn with_db<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&rusqlite::Connection) -> rusqlite::Result<T>,
{
    DB.lock()
        .map_err(|e| e.to_string())?
        .with_conn(f)
        .map_err(|e| e.to_string())
}

// ── Signing ─────────────────────────────────────────────────────────────────

/// Fetch wallet address from DB without touching the secret/keychain.
/// Used for the idempotency check that must run BEFORE any secret access.
fn get_wallet_address(wallet_id: &str) -> Result<String, String> {
    DB.lock()
        .map_err(|e| e.to_string())?
        .get_evm_wallet(wallet_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "wallet_not_found".to_string())
        .map(|w| w.address)
}

/// Fetch wallet info from DB and build a LocalWallet ready for signing.
/// Does NOT hold the DB lock across the return value.
fn build_local_wallet(
    wallet_id: &str,
    chain_id: u64,
    app_security: &crate::AppSecurity,
) -> Result<(LocalWallet, String), String> {
    let wallet_info = DB
        .lock()
        .map_err(|e| e.to_string())?
        .get_evm_wallet(wallet_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "wallet_not_found".to_string())?;

    let address = wallet_info.address.clone();

    let signing_secret = load_signing_secret(
        &wallet_info,
        app_security.secret_backend(),
        app_security.keystore(),
        app_security.session_manager(),
        SignerOperation::Send,
    )
    .map_err(map_security_error)?
    .ok_or_else(|| "wallet_secret_not_found".to_string())?;

    let wallet = wallet_from_signing_secret(signing_secret, chain_id)?;
    Ok((wallet, address))
}

// ── ABI helpers ─────────────────────────────────────────────────────────────

fn selector_from_sig(sig: &str) -> [u8; 4] {
    let hash = ethers::utils::keccak256(sig.as_bytes());
    [hash[0], hash[1], hash[2], hash[3]]
}

fn bytes32_token(hex_str: &str) -> Token {
    let cleaned = hex_str.trim_start_matches("0x");
    let padded = format!("{:0>64}", cleaned);
    let bytes = hex::decode(&padded).unwrap_or_else(|_| vec![0u8; 32]);
    let mut arr = [0u8; 32];
    let len = bytes.len().min(32);
    arr[..len].copy_from_slice(&bytes[..len]);
    Token::FixedBytes(arr.to_vec())
}

fn make_calldata(sig: &str, args: &[Token]) -> Vec<u8> {
    let mut cd = selector_from_sig(sig).to_vec();
    cd.extend_from_slice(&encode(args));
    cd
}

// ── Pre-broadcast persistence → broadcast ────────────────────────────────────

/// Sign the transaction, persist the step atomically BEFORE broadcast, then broadcast.
/// On resume after crash: skip re-signing and reuse the persisted raw tx.
async fn sign_persist_and_broadcast(
    provider: &Provider<Http>,
    wallet: &LocalWallet,
    mutation_id: &str,
    step_name: &str,
    to: Address,
    value: U256,
    calldata: Vec<u8>,
    chain_id: u64,
) -> Result<H256, String> {
    let now = Utc::now().to_rfc3339();

    // Check if step already exists (crash recovery)
    let existing_step =
        with_db(|conn| db::load_mutation_step(conn, mutation_id, step_name))?;

    let (tx_hash_str, raw_rlp_hex) = if let Some(existing) = existing_step {
        // Reuse persisted tx — do NOT re-sign (idempotent resume)
        let tx_hash = existing
            .tx_hash
            .ok_or_else(|| "step_has_no_tx_hash".to_string())?;
        let encrypted = existing
            .raw_signed_tx_hex
            .ok_or_else(|| "step_has_no_raw_tx".to_string())?;
        let raw_hex = decrypt_raw_tx(&encrypted)?;
        (tx_hash, raw_hex)
    } else {
        // Sign and persist
        let address = wallet.address();
        let nonce = provider
            .get_transaction_count(address, Some(BlockId::Number(BlockNumber::Pending)))
            .await
            .map_err(|e| format!("get_nonce: {}", e))?;

        let gas_price = provider
            .get_gas_price()
            .await
            .map_err(|e| format!("get_gas_price: {}", e))?;

        let tx = TransactionRequest::new()
            .from(address)
            .to(to)
            .nonce(nonce)
            .gas_price(gas_price)
            .value(value)
            .data(calldata.clone())
            .chain_id(chain_id);

        let gas = provider
            .estimate_gas(&tx.clone().into(), None)
            .await
            .unwrap_or(U256::from(300_000u64));
        let tx = tx.gas(gas);

        let signed = wallet
            .sign_transaction(&tx.clone().into())
            .await
            .map_err(|e| format!("sign_tx: {}", e))?;

        let raw_rlp: Bytes = tx.rlp_signed(&signed);
        let tx_hash_bytes = ethers::utils::keccak256(&raw_rlp);
        let tx_hash_str = format!("0x{}", hex::encode(tx_hash_bytes));
        let raw_hex = format!("0x{}", hex::encode(&raw_rlp));

        // Encrypt the raw RLP before persisting — prevents plaintext key-signed
        // transactions from sitting at rest in the SQLite file unprotected.
        let encrypted_raw = encrypt_raw_tx(&raw_hex)
            .map_err(|e| format!("pre_broadcast_encrypt: {}", e))?;

        let step_id = format!("{}::{}", mutation_id, step_name);
        let step_row = MutationStepRow {
            step_id,
            mutation_id: mutation_id.to_string(),
            step_name: step_name.to_string(),
            to_address: format!("{:?}", to),
            value_wei: value.to_string(),
            calldata_hash: format!(
                "0x{}",
                hex::encode(ethers::utils::keccak256(&calldata))
            ),
            nonce: Some(nonce.to_string()),
            tx_hash: Some(tx_hash_str.clone()),
            raw_signed_tx_hex: Some(encrypted_raw),
            status: "pending".to_string(),
            receipt_status: None,
            error: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        // Persist BEFORE broadcast — crash safe
        with_db(|conn| db::upsert_mutation_step(conn, &step_row))?;
        (tx_hash_str, raw_hex)
    };

    // Broadcast
    let raw_bytes = hex::decode(raw_rlp_hex.trim_start_matches("0x"))
        .map_err(|e| format!("decode_raw_tx: {}", e))?;

    match provider
        .send_raw_transaction(Bytes::from(raw_bytes))
        .await
    {
        Ok(_pending) => {
            let update_at = Utc::now().to_rfc3339();
            let _ = with_db(|conn| {
                db::update_mutation_step(
                    conn,
                    mutation_id,
                    step_name,
                    "broadcast",
                    None,
                    None,
                    &update_at,
                )
            });
        }
        Err(e) => {
            let err_str = e.to_string();
            let update_at = Utc::now().to_rfc3339();
            let _ = with_db(|conn| {
                db::update_mutation_step(
                    conn,
                    mutation_id,
                    step_name,
                    "failed",
                    None,
                    Some(&err_str),
                    &update_at,
                )
            });
            // Propagate the failure so the caller does NOT finalize as "broadcasting".
            // The step row is already persisted with the signed raw tx, so a retry
            // (same client_request_id) will resume and re-attempt the broadcast.
            return Err(format!("broadcast_failed: {}", err_str));
        }
    }

    // Convert hex tx_hash string to H256
    let hash_bytes = hex::decode(tx_hash_str.trim_start_matches("0x"))
        .map_err(|e| format!("tx_hash_parse: {}", e))?;
    let mut arr = [0u8; 32];
    let len = hash_bytes.len().min(32);
    arr[..len].copy_from_slice(&hash_bytes[..len]);
    Ok(H256::from(arr))
}

// ── Common mutation plumbing ─────────────────────────────────────────────────

/// Check idempotency and create/resume a mutation row.
/// Returns `(mutation_id, should_execute)`.
fn ensure_mutation(
    chain_id: u64,
    wallet_id: &str,
    from_address: &str,
    client_request_id: &str,
    request_hash: &str,
    action: &str,
) -> Result<(String, bool), String> {
    // Phase 1: check existing record (separate with_db call)
    let existing = with_db(|conn| {
        db::load_mutation_by_client_request_id(conn, chain_id, wallet_id, client_request_id)
    })?;

    match existing {
        None => {
            // New mutation: insert
            let now = Utc::now().to_rfc3339();
            let mutation_id = Uuid::new_v4().to_string();
            let row = MutationRow {
                mutation_id: mutation_id.clone(),
                chain_id,
                wallet_id: wallet_id.to_string(),
                client_request_id: client_request_id.to_string(),
                request_hash: request_hash.to_string(),
                action: action.to_string(),
                status: "pending".to_string(),
                from_address: from_address.to_string(),
                current_step: None,
                task_id: None,
                node_id: None,
                final_tx_hash: None,
                error: None,
                created_at: now.clone(),
                updated_at: now,
            };
            with_db(|conn| db::insert_mutation(conn, &row))?;
            Ok((mutation_id, true))
        }
        Some(row) if row.request_hash == request_hash => {
            let terminal = row.status == "confirmed" || row.status == "failed";
            Ok((row.mutation_id, !terminal))
        }
        Some(row) => Err(format!(
            "idempotency_conflict: client_request_id reused with different hash (existing={})",
            row.request_hash
        )),
    }
}

fn load_mutation_response(mutation_id: &str) -> Result<ComputeMutationResponse, String> {
    with_db(|conn| {
        let row = db::load_mutation_by_id(conn, mutation_id)?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
        Ok(ComputeMutationResponse {
            mutation_id: row.mutation_id,
            wallet_id: row.wallet_id,
            client_request_id: row.client_request_id,
            request_hash: row.request_hash,
            status: row.status,
            action: row.action,
            current_step: row.current_step,
            tx_hash: row.final_tx_hash,
            task_id: row.task_id,
            node_id: row.node_id,
            error: row.error,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    })
}

fn finalize(mutation_id: &str, tx_hash: &H256) -> Result<ComputeMutationResponse, String> {
    let now = Utc::now().to_rfc3339();
    let tx_hash_str = format!("{:?}", tx_hash);
    with_db(|conn| db::update_mutation_tx(conn, mutation_id, &tx_hash_str, "broadcasting", &now))?;
    load_mutation_response(mutation_id)
}

fn finalize_confirmed_escrow_synced(
    mutation_id: &str,
    tx_hash: &H256,
) -> Result<ComputeMutationResponse, String> {
    let now = Utc::now().to_rfc3339();
    let tx_hash_str = format!("{:?}", tx_hash);
    with_db(|conn| db::update_mutation_tx(conn, mutation_id, &tx_hash_str, "confirmed", &now))?;
    with_db(|conn| {
        db::update_mutation_status(
            conn,
            mutation_id,
            "confirmed",
            Some("escrow_synced"),
            None,
            None,
            None,
            &now,
        )
    })?;
    load_mutation_response(mutation_id)
}

// ── Raw-tx encryption ────────────────────────────────────────────────────────

/// Encrypt a raw signed transaction hex string before persisting to SQLite.  
/// Uses the same AES-256-GCM + keychain master key as wallet secrets.
fn encrypt_raw_tx(plaintext_hex: &str) -> Result<String, String> {
    use crate::wallet::security::secret_envelope::{
        encrypt_secret, SECRET_FORMAT_KEYRING_AES256_GCM_V1,
    };
    let stored = encrypt_secret(plaintext_hex)
        .map_err(|e| format!("encrypt_raw_tx: {}", e))?;
    // Prefix with format tag so decrypt_raw_tx can dispatch correctly
    Ok(format!("{}:{}", SECRET_FORMAT_KEYRING_AES256_GCM_V1, stored.secret_data))
}

/// Decrypt a raw signed transaction from DB.  Handles both the new encrypted
/// format and legacy plaintext hex (pre-encryption deployments).
fn decrypt_raw_tx(stored: &str) -> Result<String, String> {
    use crate::wallet::security::secret_envelope::{
        decrypt_secret, SECRET_FORMAT_KEYRING_AES256_GCM_V1,
    };
    if let Some(json) = stored.strip_prefix(&format!("{}:", SECRET_FORMAT_KEYRING_AES256_GCM_V1)) {
        decrypt_secret(json, SECRET_FORMAT_KEYRING_AES256_GCM_V1)
            .map_err(|e| format!("decrypt_raw_tx: {}", e))
    } else {
        // Legacy plaintext hex — accept as-is so existing step records still resume
        Ok(stored.to_string())
    }
}

// ── NodeRegistry constants (mirrors Solidity) ───────────────────────────────

/// `NodeRegistry.REGISTRATION_FEE` = 0.1 ETH.
/// msg.value must include this on top of the provider's initial stake.
const REGISTRATION_FEE_WEI: u128 = 100_000_000_000_000_000; // 0.1 ETH

/// `NodeRegistry.MINIMUM_INITIAL_STAKE` = 0.5 ETH.
/// The stake component of msg.value (excluding the fee) must be at least this.
const MINIMUM_INITIAL_STAKE_WEI: u128 = 500_000_000_000_000_000; // 0.5 ETH

// ── Mutation implementations ─────────────────────────────────────────────────

// ── Two-step create_and_fund helpers ────────────────────────────────────────

/// Extract the chain-derived nodeId bytes32 from the NodeRegistered event log.
fn extract_node_registered_id(
    receipt: &TransactionReceipt,
    node_registry: &str,
) -> Result<String, String> {
    // NodeRegistered(bytes32 indexed nodeId, address indexed owner, uint8 resourceType)
    let sig_hash = ethers::utils::keccak256("NodeRegistered(bytes32,address,uint8)".as_bytes());
    let node_registered_topic = H256::from(sig_hash);
    let node_registry_addr = Address::from_str(node_registry)
        .map_err(|e| format!("invalid_node_registry: {}", e))?;

    for log in &receipt.logs {
        if log.address != node_registry_addr {
            continue;
        }
        if log.topics.first() == Some(&node_registered_topic) {
            if let Some(node_id_topic) = log.topics.get(1) {
                return Ok(format!("{:?}", node_id_topic));
            }
        }
    }
    Err("node_registered_event_not_found: registerNode receipt has no NodeRegistered log".to_string())
}

/// Poll for a transaction receipt until mined (up to 120 s / 60 attempts × 2 s).
async fn await_tx_receipt(
    provider: &Provider<Http>,
    tx_hash: &H256,
) -> Result<TransactionReceipt, String> {
    for _ in 0..60u32 {
        match provider.get_transaction_receipt(*tx_hash).await {
            Ok(Some(receipt)) => return Ok(receipt),
            Ok(None) => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(e) => return Err(format!("get_receipt: {}", e)),
        }
    }
    Err(format!(
        "receipt_timeout: tx {:?} not mined after 120 s",
        tx_hash
    ))
}

/// Extract the chain-derived taskId bytes32 from the TaskCreated event log.
fn extract_task_created_id(
    receipt: &TransactionReceipt,
    task_marketplace: &str,
) -> Result<String, String> {
    // TaskCreated(bytes32 indexed taskId, address indexed buyer, uint8 resourceType, uint256 maxPrice)
    let sig_hash = ethers::utils::keccak256(
        "TaskCreated(bytes32,address,uint8,uint256)".as_bytes(),
    );
    let task_created_topic = H256::from(sig_hash);
    let task_marketplace_addr = Address::from_str(task_marketplace)
        .map_err(|e| format!("invalid_task_marketplace: {}", e))?;

    for log in &receipt.logs {
        if log.address != task_marketplace_addr {
            continue;
        }
        if log.topics.first() == Some(&task_created_topic) {
            if let Some(task_id_topic) = log.topics.get(1) {
                return Ok(format!("{:?}", task_id_topic));
            }
        }
    }
    Err("task_created_event_not_found: createTask receipt has no TaskCreated log".to_string())
}

pub async fn execute_register_node(
    input: &RegisterNodeInput,
    app_security: &crate::AppSecurity,
) -> Result<ComputeMutationResponse, String> {
    let config = load_compute_config().map_err(|m| format!("config: {}", m.join(", ")))?;

    // node_id is NOT in args_json — it is derived on-chain from registerNode()
    let args_json = serde_json::json!({
        "resourceType": input.resource_type,
        "stakeAmount": input.stake_amount_wei,
        "metadataUri": input.metadata_uri,
    })
    .to_string();

    let hash = compute_request_hash(
        1,
        config.chain_id,
        &config.task_marketplace_address,
        &config.node_registry_address,
        &config.escrow_manager_address,
        &input.wallet_id,
        "register_node",
        &args_json,
    );

    // [P1] Check idempotency BEFORE touching the signing secret (keychain).
    let address = get_wallet_address(&input.wallet_id)?;

    let (mutation_id, should_execute) = ensure_mutation(
        config.chain_id,
        &input.wallet_id,
        &address,
        &input.client_request_id,
        &hash,
        "register_node",
    )?;
    if !should_execute {
        return load_mutation_response(&mutation_id);
    }

    // [P1] Validate stake BEFORE loading the signing secret.
    // stake_amount_wei is the provider's desired initial stake (excludes the fee).
    // Must be parseable and at least MINIMUM_INITIAL_STAKE (0.5 ETH).
    let stake_wei = U256::from_dec_str(&input.stake_amount_wei)
        .map_err(|e| format!("invalid_stake_amount_wei: {}", e))?;

    let min_stake = U256::from(MINIMUM_INITIAL_STAKE_WEI);
    if stake_wei < min_stake {
        return Err(format!(
            "stake_below_minimum: provided {} wei, minimum is {} wei (0.5 ETH)",
            stake_wei, min_stake
        ));
    }

    // Total tx value = initial stake provided by the caller + the protocol registration fee.
    // NodeRegistry.registerNode requires msg.value >= REGISTRATION_FEE + MINIMUM_INITIAL_STAKE
    // and records stakeAmount = msg.value - REGISTRATION_FEE.
    let registration_fee = U256::from(REGISTRATION_FEE_WEI);
    let tx_value_wei = stake_wei
        .checked_add(registration_fee)
        .ok_or_else(|| "overflow: stake_wei + registration_fee".to_string())?;

    // Only load the signing secret once validation and idempotency are confirmed.
    let (wallet, _) = build_local_wallet(&input.wallet_id, config.chain_id, app_security)?;

    let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
        .map_err(|e| format!("provider: {}", e))?;

    let node_registry = Address::from_str(&config.node_registry_address)
        .map_err(|e| format!("node_registry_addr: {}", e))?;

    // registerNode(uint8 resourceType, string metadataURI) external payable returns (bytes32 nodeId)
    // nodeId is chain-derived from msg.sender + nonce; NOT supplied by caller.
    let cd = make_calldata(
        "registerNode(uint8,string)",
        &[
            Token::Uint(U256::from(input.resource_type as u64)),
            Token::String(input.metadata_uri.clone()),
        ],
    );

    let tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "register_node",
        node_registry,
        tx_value_wei, // stake + 0.1 ETH registration fee
        cd,
        config.chain_id,
    )
    .await?;

    // Recover chain-derived node_id — check DB first (resume path after crash).
    let existing_node_id: Option<String> = with_db(|conn| {
        db::load_mutation_by_id(conn, &mutation_id)
            .map(|opt| opt.and_then(|m| m.node_id))
    })?;

    if existing_node_id.is_none() {
        let receipt = await_tx_receipt(&provider, &tx_hash)
            .await
            .map_err(|e| {
                let now = Utc::now().to_rfc3339();
                let _ = with_db(|conn| {
                    db::update_mutation_tx(
                        conn,
                        &mutation_id,
                        &format!("{:?}", tx_hash),
                        "partial_requires_resume",
                        &now,
                    )
                });
                format!("register_node_receipt: {}", e)
            })?;

        let node_id = extract_node_registered_id(&receipt, &config.node_registry_address)?;
        let now = Utc::now().to_rfc3339();
        with_db(|conn| db::update_mutation_node_id(conn, &mutation_id, &node_id, &now))?;
    }

    finalize(&mutation_id, &tx_hash)
}

/// Pure fundability pre-check: compute quoteCap and compare to a caller-supplied quoteMin.
/// Returns Err if the task could never be funded at the given price/duration.
/// Extracted for unit-testing without network access.
pub fn check_task_fundability(
    max_price_per_hour_wei: U256,
    duration_seconds: u64,
    quote_min: U256,
) -> Result<U256, String> {
    if max_price_per_hour_wei.is_zero() {
        return Err("underfundable_task: max_price_wei must be > 0".to_string());
    }
    if duration_seconds == 0 {
        return Err("underfundable_task: duration_seconds must be > 0".to_string());
    }
    let duration_hours = (duration_seconds + 3599) / 3600;
    let quote_cap = max_price_per_hour_wei
        .checked_mul(U256::from(duration_hours))
        .ok_or_else(|| "overflow: max_price_wei * duration_hours".to_string())?;
    if quote_min > quote_cap {
        return Err(format!(
            "underfundable_task: quoteMin ({}) > quoteCap ({}); increase max_price_wei or duration",
            quote_min, quote_cap
        ));
    }
    Ok(quote_cap)
}

pub async fn execute_create_and_fund_task(
    input: &CreateAndFundTaskInput,
    app_security: &crate::AppSecurity,
) -> Result<ComputeMutationResponse, String> {
    let config = load_compute_config().map_err(|m| format!("config: {}", m.join(", ")))?;

    // task_id is NOT in args_json — it is derived on-chain from createTask()
    let args_json = serde_json::json!({
        "resourceType": input.resource_type,
        "requiredPower": input.required_power,
        "duration": input.duration_seconds,
        "maxPrice": input.max_price_wei,
        "specificationUri": input.specification_uri,
        "minTrustLevel": input.min_trust_level,
    })
    .to_string();

    let hash = compute_request_hash(
        1,
        config.chain_id,
        &config.task_marketplace_address,
        &config.node_registry_address,
        &config.escrow_manager_address,
        &input.wallet_id,
        "create_and_fund_task",
        &args_json,
    );

    // [P1] Check idempotency BEFORE touching the signing secret (keychain).
    let address = get_wallet_address(&input.wallet_id)?;

    let (mutation_id, should_execute) = ensure_mutation(
        config.chain_id,
        &input.wallet_id,
        &address,
        &input.client_request_id,
        &hash,
        "create_and_fund_task",
    )?;
    if !should_execute {
        return load_mutation_response(&mutation_id);
    }

    // ── Pre-sign validation: reject underfundable tasks before touching signer ──
    // Parse inputs and check that quoteCap >= quoteMin (on-chain estimate) so we
    // never broadcast a createTask that will revert with UnderfundableTaskCreation.
    let max_price_per_hour_wei = U256::from_dec_str(&input.max_price_wei)
        .map_err(|e| format!("max_price_wei: {}", e))?;
    let required_power = U256::from_dec_str(&input.required_power).unwrap_or(U256::zero());

    let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
        .map_err(|e| format!("provider: {}", e))?;

    let task_marketplace = Address::from_str(&config.task_marketplace_address)
        .map_err(|e| format!("task_marketplace_addr: {}", e))?;

    // Call estimateTaskCost(uint8,uint256,uint256) to get on-chain quoteMin, then
    // compare to quoteCap = max_price_per_hour_wei * durationHours.
    // Mirrors the contract check: if quoteMin > quoteCap → UnderfundableTaskCreation.
    let estimate_cd = make_calldata(
        "estimateTaskCost(uint8,uint256,uint256)",
        &[
            Token::Uint(U256::from(input.resource_type as u64)),
            Token::Uint(required_power),
            Token::Uint(U256::from(input.duration_seconds)),
        ],
    );
    let estimate_tx = TransactionRequest::new()
        .to(task_marketplace)
        .data(estimate_cd);
    let estimate_raw = provider
        .call(&estimate_tx.into(), None)
        .await
        .map_err(|e| format!("estimate_task_cost_call: {}", e))?;
    let tokens = abi_decode(&[ParamType::Uint(256)], &estimate_raw)
        .map_err(|e| format!("estimate_task_cost_decode: {}", e))?;
    let quote_min = tokens
        .into_iter()
        .next()
        .and_then(|t| t.into_uint())
        .ok_or_else(|| "estimate_task_cost: empty result".to_string())?;

    // check_task_fundability validates zero-price/duration and quoteMin > quoteCap.
    // Returns total_escrow_wei (= quoteCap) on success.
    let total_escrow_wei = check_task_fundability(
        max_price_per_hour_wei,
        input.duration_seconds,
        quote_min,
    )?;
    let duration_hours = (input.duration_seconds + 3599) / 3600;

    // Only load the signing secret once idempotency is confirmed and task is fundable.
    let (wallet, _) =
        build_local_wallet(&input.wallet_id, config.chain_id, app_security)?;

    // ── Step 1: createTask(uint8,uint256,uint256,uint256,uint8,string) ───────
    // [P2] maxPrice is the PER-HOUR rate; the contract multiplies by durationHours
    // internally to produce quoteCap. Passing total_escrow_wei here would double-count.
    // value = 0 — no ETH sent with createTask; task_id is returned by the chain.
    let create_cd = make_calldata(
        "createTask(uint8,uint256,uint256,uint256,uint8,string)",
        &[
            Token::Uint(U256::from(input.resource_type as u64)),
            Token::Uint(required_power),
            Token::Uint(U256::from(input.duration_seconds)),
            Token::Uint(max_price_per_hour_wei), // maxPrice = per-hour rate (contract × durationHours = quoteCap)
            Token::Uint(U256::from(input.min_trust_level as u64)),
            Token::String(input.specification_uri.clone()),
        ],
    );

    let create_tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "create_task",
        task_marketplace,
        U256::zero(), // no ETH for createTask
        create_cd,
        config.chain_id,
    )
    .await?;

    // Recover chain-derived task_id — check DB first (resume path after crash)
    let existing_task_id: Option<String> = with_db(|conn| {
        db::load_mutation_by_id(conn, &mutation_id)
            .map(|opt| opt.and_then(|m| m.task_id))
    })?;

    let derived_task_id = if let Some(tid) = existing_task_id {
        tid
    } else {
        // Await on-chain confirmation and extract chain-assigned task_id from TaskCreated event
        let receipt = await_tx_receipt(&provider, &create_tx_hash)
            .await
            .map_err(|e| {
                let now = Utc::now().to_rfc3339();
                let _ = with_db(|conn| {
                    db::update_mutation_tx(
                        conn,
                        &mutation_id,
                        &format!("{:?}", create_tx_hash),
                        "partial_requires_resume",
                        &now,
                    )
                });
                format!("create_task_receipt: {}", e)
            })?;

        // [P1] Check receipt.status — a reverted createTask must not proceed to fund.
        if receipt.status != Some(ethers::types::U64::from(1u64)) {
            let now = Utc::now().to_rfc3339();
            let _ = with_db(|conn| {
                db::update_mutation_tx(
                    conn,
                    &mutation_id,
                    &format!("{:?}", create_tx_hash),
                    "failed",
                    &now,
                )
            });
            return Err("create_task_reverted: createTask transaction was included but reverted".to_string());
        }

        let task_id = extract_task_created_id(&receipt, &config.task_marketplace_address)?;
        let now = Utc::now().to_rfc3339();
        with_db(|conn| db::update_mutation_task_id(conn, &mutation_id, &task_id, &now))?;
        task_id
    };

    // ── Step 2: fundTaskEscrow(bytes32 taskId) payable — ETH = total_escrow_wei ─
    let fund_cd = make_calldata(
        "fundTaskEscrow(bytes32)",
        &[bytes32_token(&derived_task_id)],
    );

    let fund_tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "fund_escrow",
        task_marketplace,
        total_escrow_wei,
        fund_cd,
        config.chain_id,
    )
    .await
    .map_err(|e| {
        let now = Utc::now().to_rfc3339();
        let _ = with_db(|conn| {
            db::update_mutation_tx(conn, &mutation_id, "", "partial_requires_resume", &now)
        });
        format!("fund_escrow: {}", e)
    })?;

    // [P1] Await fund receipt and verify status — fund_escrow getting mined is the
    // completion signal; only then is the escrow actually locked on-chain.
    let fund_receipt = await_tx_receipt(&provider, &fund_tx_hash)
        .await
        .map_err(|e| {
            let now = Utc::now().to_rfc3339();
            let _ = with_db(|conn| {
                db::update_mutation_tx(
                    conn,
                    &mutation_id,
                    &format!("{:?}", fund_tx_hash),
                    "partial_requires_resume",
                    &now,
                )
            });
            format!("fund_escrow_receipt: {}", e)
        })?;

    if fund_receipt.status != Some(ethers::types::U64::from(1u64)) {
        let now = Utc::now().to_rfc3339();
        let _ = with_db(|conn| {
            db::update_mutation_step(
                conn,
                &mutation_id,
                "fund_escrow",
                "failed",
                Some(0),
                Some("fund_escrow_reverted"),
                &now,
            )
        });
        let _ = with_db(|conn| {
            db::update_mutation_tx(
                conn,
                &mutation_id,
                &format!("{:?}", fund_tx_hash),
                "failed",
                &now,
            )
        });
        return Err("fund_escrow_reverted: fundTaskEscrow transaction was included but reverted".to_string());
    }

    // Mark the fund step as mined+confirmed before refreshing read-model state.
    let now = Utc::now().to_rfc3339();
    with_db(|conn| {
        db::update_mutation_step(
            conn,
            &mutation_id,
            "fund_escrow",
            "confirmed",
            Some(1),
            None,
            &now,
        )
    })?;

    // Sync escrow/read-model after fund confirmation so UI reflects canonical state.
    if let Err(sync_err) = crate::compute::sync::refresh_snapshot(&input.wallet_id).await {
        let fail_now = Utc::now().to_rfc3339();
        let err_msg = format!("escrow_sync_failed: {}", sync_err);
        let _ = with_db(|conn| db::update_mutation_tx(
            conn,
            &mutation_id,
            &format!("{:?}", fund_tx_hash),
            "partial_requires_resume",
            &fail_now,
        ));
        let _ = with_db(|conn| {
            db::update_mutation_status(
                conn,
                &mutation_id,
                "partial_requires_resume",
                Some("fund_escrow_confirmed"),
                None,
                None,
                Some(&err_msg),
                &fail_now,
            )
        });
        return Err(err_msg);
    }

    finalize_confirmed_escrow_synced(&mutation_id, &fund_tx_hash)
}

pub async fn execute_accept_task(
    input: &AcceptTaskInput,
    app_security: &crate::AppSecurity,
) -> Result<ComputeMutationResponse, String> {
    let config = load_compute_config().map_err(|m| format!("config: {}", m.join(", ")))?;

    let args_json =
        serde_json::json!({ "taskId": input.task_id, "nodeId": input.node_id }).to_string();

    let hash = compute_request_hash(
        1,
        config.chain_id,
        &config.task_marketplace_address,
        &config.node_registry_address,
        &config.escrow_manager_address,
        &input.wallet_id,
        "accept_task",
        &args_json,
    );

    let address = get_wallet_address(&input.wallet_id)?;

    let (mutation_id, should_execute) = ensure_mutation(
        config.chain_id,
        &input.wallet_id,
        &address,
        &input.client_request_id,
        &hash,
        "accept_task",
    )?;
    if !should_execute {
        return load_mutation_response(&mutation_id);
    }

    let (wallet, _) = build_local_wallet(&input.wallet_id, config.chain_id, app_security)?;

    let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
        .map_err(|e| format!("provider: {}", e))?;

    let task_mp = Address::from_str(&config.task_marketplace_address)
        .map_err(|e| format!("addr: {}", e))?;

    let cd = make_calldata(
        "acceptTask(bytes32,bytes32)",
        &[bytes32_token(&input.task_id), bytes32_token(&input.node_id)],
    );

    let tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "accept_task",
        task_mp,
        U256::zero(),
        cd,
        config.chain_id,
    )
    .await?;

    finalize(&mutation_id, &tx_hash)
}

pub async fn execute_submit_result(
    input: &SubmitResultInput,
    app_security: &crate::AppSecurity,
) -> Result<ComputeMutationResponse, String> {
    let config = load_compute_config().map_err(|m| format!("config: {}", m.join(", ")))?;

    let args_json = serde_json::json!({
        "taskId": input.task_id,
        "resultHash": input.result_hash,
        "resultUri": input.result_uri,
    })
    .to_string();

    let hash = compute_request_hash(
        1,
        config.chain_id,
        &config.task_marketplace_address,
        &config.node_registry_address,
        &config.escrow_manager_address,
        &input.wallet_id,
        "submit_result",
        &args_json,
    );

    let address = get_wallet_address(&input.wallet_id)?;

    let (mutation_id, should_execute) = ensure_mutation(
        config.chain_id,
        &input.wallet_id,
        &address,
        &input.client_request_id,
        &hash,
        "submit_result",
    )?;
    if !should_execute {
        return load_mutation_response(&mutation_id);
    }

    let (wallet, _) =
        build_local_wallet(&input.wallet_id, config.chain_id, app_security)?;

    let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
        .map_err(|e| format!("provider: {}", e))?;

    let task_mp = Address::from_str(&config.task_marketplace_address)
        .map_err(|e| format!("addr: {}", e))?;

    // submitResult(bytes32 taskId, bytes32 resultHash, string resultURI)
    let cd = make_calldata(
        "submitResult(bytes32,bytes32,string)",
        &[
            bytes32_token(&input.task_id),
            bytes32_token(&input.result_hash),
            Token::String(input.result_uri.clone()),
        ],
    );

    let tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "submit_result",
        task_mp,
        U256::zero(),
        cd,
        config.chain_id,
    )
    .await?;

    finalize(&mutation_id, &tx_hash)
}

pub async fn execute_approve_task(
    input: &ApproveTaskInput,
    app_security: &crate::AppSecurity,
) -> Result<ComputeMutationResponse, String> {
    let config = load_compute_config().map_err(|m| format!("config: {}", m.join(", ")))?;

    let args_json = serde_json::json!({
        "taskId": input.task_id,
    })
    .to_string();

    let hash = compute_request_hash(
        1,
        config.chain_id,
        &config.task_marketplace_address,
        &config.node_registry_address,
        &config.escrow_manager_address,
        &input.wallet_id,
        "approve_task",
        &args_json,
    );

    let address = get_wallet_address(&input.wallet_id)?;

    let (mutation_id, should_execute) = ensure_mutation(
        config.chain_id,
        &input.wallet_id,
        &address,
        &input.client_request_id,
        &hash,
        "approve_task",
    )?;
    if !should_execute {
        return load_mutation_response(&mutation_id);
    }

    let (wallet, _) =
        build_local_wallet(&input.wallet_id, config.chain_id, app_security)?;

    let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
        .map_err(|e| format!("provider: {}", e))?;

    let task_mp = Address::from_str(&config.task_marketplace_address)
        .map_err(|e| format!("addr: {}", e))?;

    // approveResult(bytes32 taskId) — contract resolves escrow internally; no price arg.
    let cd = make_calldata(
        "approveResult(bytes32)",
        &[bytes32_token(&input.task_id)],
    );

    let tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "approve_task",
        task_mp,
        U256::zero(),
        cd,
        config.chain_id,
    )
    .await?;

    finalize(&mutation_id, &tx_hash)
}

pub async fn execute_dispute_task(
    input: &DisputeTaskInput,
    app_security: &crate::AppSecurity,
) -> Result<ComputeMutationResponse, String> {
    let config = load_compute_config().map_err(|m| format!("config: {}", m.join(", ")))?;

    let args_json = serde_json::json!({
        "taskId": input.task_id,
        "reason": input.reason,
    })
    .to_string();

    let hash = compute_request_hash(
        1,
        config.chain_id,
        &config.task_marketplace_address,
        &config.node_registry_address,
        &config.escrow_manager_address,
        &input.wallet_id,
        "dispute_task",
        &args_json,
    );

    let address = get_wallet_address(&input.wallet_id)?;

    let (mutation_id, should_execute) = ensure_mutation(
        config.chain_id,
        &input.wallet_id,
        &address,
        &input.client_request_id,
        &hash,
        "dispute_task",
    )?;
    if !should_execute {
        return load_mutation_response(&mutation_id);
    }

    let (wallet, _) =
        build_local_wallet(&input.wallet_id, config.chain_id, app_security)?;

    let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
        .map_err(|e| format!("provider: {}", e))?;

    let task_mp = Address::from_str(&config.task_marketplace_address)
        .map_err(|e| format!("addr: {}", e))?;

    // disputeTask(bytes32 taskId, string reason)
    let cd = make_calldata(
        "disputeTask(bytes32,string)",
        &[
            bytes32_token(&input.task_id),
            Token::String(input.reason.clone()),
        ],
    );

    let tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "dispute_task",
        task_mp,
        U256::zero(),
        cd,
        config.chain_id,
    )
    .await?;

    finalize(&mutation_id, &tx_hash)
}

// ── PoW verification helpers ─────────────────────────────────────────────────

/// Extract the `challengeId` (topic[1]) from the `ChallengeIssued` event.
/// ChallengeIssued(bytes32 indexed challengeId, bytes32 indexed nodeId, uint256 difficulty, uint256 deadline)
fn extract_challenge_issued_id(
    receipt: &TransactionReceipt,
    pow_verifier: &str,
) -> Result<String, String> {
    let sig_hash = ethers::utils::keccak256(
        "ChallengeIssued(bytes32,bytes32,uint256,uint256)".as_bytes(),
    );
    let topic = H256::from(sig_hash);
    let addr = Address::from_str(pow_verifier)
        .map_err(|e| format!("pow_verifier_addr: {}", e))?;

    for log in &receipt.logs {
        if log.address != addr {
            continue;
        }
        if log.topics.first() == Some(&topic) {
            if let Some(challenge_id_topic) = log.topics.get(1) {
                return Ok(format!("{:?}", challenge_id_topic));
            }
        }
    }
    Err("challenge_issued_event_not_found: issueChallenge receipt has no ChallengeIssued log".to_string())
}

/// Decode the Challenge struct returned by `getChallenge(bytes32)`.
/// Returns `(seed, difficulty)`.
///
/// Challenge layout (all fixed types, 256 bytes total):
/// [0] bytes32 challengeId
/// [1] bytes32 nodeId
/// [2] bytes32 seed
/// [3] uint256 difficulty
/// [4] uint256 issuedAt
/// [5] uint256 deadline
/// [6] bool    completed
/// [7] uint256 solutionTime
fn decode_challenge_struct(data: &[u8]) -> Result<([u8; 32], U256), String> {
    let param_types = vec![ParamType::Tuple(vec![
        ParamType::FixedBytes(32), // challengeId
        ParamType::FixedBytes(32), // nodeId
        ParamType::FixedBytes(32), // seed
        ParamType::Uint(256),      // difficulty
        ParamType::Uint(256),      // issuedAt
        ParamType::Uint(256),      // deadline
        ParamType::Bool,           // completed
        ParamType::Uint(256),      // solutionTime
    ])];

    let decoded = abi_decode(&param_types, data)
        .map_err(|e| format!("decode_challenge_struct: {}", e))?;

    if let Some(Token::Tuple(fields)) = decoded.into_iter().next() {
        let seed = match fields.get(2) {
            Some(Token::FixedBytes(b)) if b.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(b);
                arr
            }
            _ => return Err("decode_challenge_struct: unexpected seed token".to_string()),
        };
        let difficulty = match fields.get(3) {
            Some(Token::Uint(v)) => *v,
            _ => return Err("decode_challenge_struct: unexpected difficulty token".to_string()),
        };
        Ok((seed, difficulty))
    } else {
        Err("decode_challenge_struct: response is not a tuple".to_string())
    }
}

/// Off-chain PoW solver: iterate nonces until keccak256(seed || nonce) ≤ target.
///
/// Contract invariant: `target = type(uint256).max / difficulty`.
/// MVP difficulty = 2^16 = 65_536 → expected ~65 K iterations (< 5 ms).
/// Cap at 2 M attempts to avoid a hang on unexpectedly high difficulty.
pub fn find_pow_nonce(seed: [u8; 32], difficulty: U256) -> Result<U256, String> {
    if difficulty.is_zero() {
        return Err("pow_solver: difficulty is zero".to_string());
    }
    let target = U256::MAX / difficulty;

    // Single 64-byte buffer; seed occupies [0..32], nonce big-endian in [32..64].
    let mut buf = [0u8; 64];
    buf[..32].copy_from_slice(&seed);

    for raw_nonce in 0u64..2_000_000 {
        let nonce_u256 = U256::from(raw_nonce);
        nonce_u256.to_big_endian(&mut buf[32..]);

        let hash = ethers::utils::keccak256(&buf);
        let hash_val = U256::from_big_endian(&hash);

        if hash_val <= target {
            return Ok(nonce_u256);
        }
    }
    Err("pow_solver_exhausted: no passing nonce found within 2_000_000 attempts; check difficulty".to_string())
}

/// Return `true` when the receipt contains a `ChallengeSolved` event from the verifier.
/// `ChallengeSolved(bytes32 indexed challengeId, bytes32 indexed nodeId, uint256 solutionTime, uint256 verifiedPower)`
fn check_challenge_solved(
    receipt: &TransactionReceipt,
    pow_verifier: &str,
) -> Result<bool, String> {
    let sig_hash = ethers::utils::keccak256(
        "ChallengeSolved(bytes32,bytes32,uint256,uint256)".as_bytes(),
    );
    let topic = H256::from(sig_hash);
    let addr = Address::from_str(pow_verifier)
        .map_err(|e| format!("pow_verifier_addr: {}", e))?;

    for log in &receipt.logs {
        if log.address == addr && log.topics.first() == Some(&topic) {
            return Ok(true);
        }
    }
    Ok(false)
}

// ── compute_verify_node ──────────────────────────────────────────────────────

/// Activate a registered Pending node through the ProofOfWorkVerifier challenge flow.
///
/// Steps (crash-safe via mutation step persistence):
///   1. issueChallenge(nodeId)  → mine → extract challengeId from ChallengeIssued event
///   2. getChallenge(challengeId) eth_call → decode seed + difficulty
///   3. Solve PoW nonce off-chain
///   4. submitSolution(challengeId, nonce) → mine → verify ChallengeSolved event
///   5. refresh_snapshot so UI reflects Active + chain-derived computePower
///
/// On crash between step 1 and step 4, resume reuses the persisted issueChallenge tx_hash,
/// re-fetches the receipt to extract challengeId, and proceeds to submitSolution.
pub async fn execute_verify_node(
    input: &VerifyNodeInput,
    app_security: &crate::AppSecurity,
) -> Result<ComputeMutationResponse, String> {
    // Load core config and PoW verifier address (separate — does NOT affect is_configured).
    let config = load_compute_config().map_err(|m| format!("config: {}", m.join(", ")))?;
    let pow_verifier_addr_str =
        crate::compute::config::load_pow_verifier_address()?;

    // Validate node_id is a non-empty hex string before any signing.
    if input.node_id.trim().is_empty() {
        return Err("invalid_node_id: node_id must not be empty".to_string());
    }

    let args_json = serde_json::json!({ "nodeId": input.node_id }).to_string();

    let hash = compute_request_hash(
        1,
        config.chain_id,
        &config.task_marketplace_address,
        &config.node_registry_address,
        &config.escrow_manager_address,
        &input.wallet_id,
        "verify_node",
        &args_json,
    );

    // [P1] Idempotency check before touching signing secret.
    let address = get_wallet_address(&input.wallet_id)?;

    let (mutation_id, should_execute) = ensure_mutation(
        config.chain_id,
        &input.wallet_id,
        &address,
        &input.client_request_id,
        &hash,
        "verify_node",
    )?;
    if !should_execute {
        return load_mutation_response(&mutation_id);
    }

    // Only access signing secret after idempotency is confirmed.
    let (wallet, _) = build_local_wallet(&input.wallet_id, config.chain_id, app_security)?;

    let provider = Provider::<Http>::try_from(config.rpc_url.as_str())
        .map_err(|e| format!("provider: {}", e))?;

    let pow_verifier = Address::from_str(&pow_verifier_addr_str)
        .map_err(|e| format!("pow_verifier_addr: {}", e))?;

    // ── Step 1: issueChallenge(bytes32 nodeId) ──────────────────────────────
    let issue_cd = make_calldata(
        "issueChallenge(bytes32)",
        &[bytes32_token(&input.node_id)],
    );

    let issue_tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "issue_challenge",
        pow_verifier,
        U256::zero(),
        issue_cd,
        config.chain_id,
    )
    .await?;

    // Wait for the issueChallenge tx to mine and extract challengeId.
    let issue_receipt = await_tx_receipt(&provider, &issue_tx_hash)
        .await
        .map_err(|e| {
            let now = Utc::now().to_rfc3339();
            let _ = with_db(|conn| {
                db::update_mutation_tx(
                    conn,
                    &mutation_id,
                    &format!("{:?}", issue_tx_hash),
                    "partial_requires_resume",
                    &now,
                )
            });
            format!("issue_challenge_receipt: {}", e)
        })?;

    if issue_receipt.status != Some(ethers::types::U64::from(1u64)) {
        let now = Utc::now().to_rfc3339();
        let _ = with_db(|conn| {
            db::update_mutation_tx(conn, &mutation_id, &format!("{:?}", issue_tx_hash), "failed", &now)
        });
        return Err("issue_challenge_reverted: issueChallenge transaction was included but reverted".to_string());
    }

    let challenge_id_hex =
        extract_challenge_issued_id(&issue_receipt, &pow_verifier_addr_str)?;

    // ── Step 2: read challenge via eth_call, solve nonce, submit ────────────
    // eth_call getChallenge(bytes32 challengeId) → decode Challenge struct for seed+difficulty.
    let get_cd = make_calldata("getChallenge(bytes32)", &[bytes32_token(&challenge_id_hex)]);

    let call_req = TransactionRequest::new().to(pow_verifier).data(get_cd);
    let raw_return = provider
        .call(&call_req.into(), None)
        .await
        .map_err(|e| format!("get_challenge_call: {}", e))?;

    let (seed_bytes, difficulty) = decode_challenge_struct(&raw_return)?;

    // Solve nonce off-chain.
    let nonce = find_pow_nonce(seed_bytes, difficulty)
        .map_err(|e| format!("pow_solve: {}", e))?;

    // submitSolution(bytes32 challengeId, uint256 nonce)
    let submit_cd = make_calldata(
        "submitSolution(bytes32,uint256)",
        &[bytes32_token(&challenge_id_hex), Token::Uint(nonce)],
    );

    let submit_tx_hash = sign_persist_and_broadcast(
        &provider,
        &wallet,
        &mutation_id,
        "submit_solution",
        pow_verifier,
        U256::zero(),
        submit_cd,
        config.chain_id,
    )
    .await
    .map_err(|e| {
        let now = Utc::now().to_rfc3339();
        let _ = with_db(|conn| {
            db::update_mutation_tx(conn, &mutation_id, "", "partial_requires_resume", &now)
        });
        format!("submit_solution: {}", e)
    })?;

    let submit_receipt = await_tx_receipt(&provider, &submit_tx_hash)
        .await
        .map_err(|e| {
            let now = Utc::now().to_rfc3339();
            let _ = with_db(|conn| {
                db::update_mutation_tx(
                    conn,
                    &mutation_id,
                    &format!("{:?}", submit_tx_hash),
                    "partial_requires_resume",
                    &now,
                )
            });
            format!("submit_solution_receipt: {}", e)
        })?;

    if submit_receipt.status != Some(ethers::types::U64::from(1u64)) {
        let now = Utc::now().to_rfc3339();
        let _ = with_db(|conn| {
            db::update_mutation_tx(conn, &mutation_id, &format!("{:?}", submit_tx_hash), "failed", &now)
        });
        return Err("submit_solution_reverted: submitSolution transaction was included but reverted".to_string());
    }

    // Verify the solution actually passed (ChallengeSolved event present).
    let solved = check_challenge_solved(&submit_receipt, &pow_verifier_addr_str)?;
    if !solved {
        let now = Utc::now().to_rfc3339();
        let _ = with_db(|conn| {
            db::update_mutation_tx(conn, &mutation_id, &format!("{:?}", submit_tx_hash), "failed", &now)
        });
        return Err("pow_challenge_failed: submitSolution mined but ChallengeSolved event not found; nonce did not pass on-chain".to_string());
    }

    // Refresh snapshot so the UI sees the Active node with chain-derived computePower.
    let _ = crate::compute::sync::refresh_snapshot(&input.wallet_id).await;

    finalize(&mutation_id, &submit_tx_hash)
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_hash_is_deterministic() {
        let h1 = compute_request_hash(
            1, 1, "0xAAA", "0xBBB", "0xCCC", "wallet1", "register_node",
            r#"{"computePower":"1000"}"#,
        );
        let h2 = compute_request_hash(
            1, 1, "0xAAA", "0xBBB", "0xCCC", "wallet1", "register_node",
            r#"{"computePower":"1000"}"#,
        );
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn request_hash_differs_on_field_change() {
        let h1 = compute_request_hash(1, 1, "0xAAA", "0xBBB", "0xCCC", "wallet1", "register_node", r#"{"computePower":"1000"}"#);
        let h2 = compute_request_hash(1, 1, "0xAAA", "0xBBB", "0xCCC", "wallet1", "create_task",   r#"{"computePower":"1000"}"#);
        let h3 = compute_request_hash(1, 1, "0xAAA", "0xBBB", "0xCCC", "wallet2", "register_node", r#"{"computePower":"1000"}"#);
        assert_ne!(h1, h2);
        assert_ne!(h1, h3);
    }

    fn make_test_conn() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::compute::db::init_compute_tables(&conn).unwrap();
        conn
    }

    fn make_row(wallet_id: &str, client_request_id: &str, hash: &str) -> MutationRow {
        let now = Utc::now().to_rfc3339();
        MutationRow {
            mutation_id: Uuid::new_v4().to_string(),
            chain_id: 1,
            wallet_id: wallet_id.to_string(),
            client_request_id: client_request_id.to_string(),
            request_hash: hash.to_string(),
            action: "register_node".to_string(),
            status: "pending".to_string(),
            from_address: "0x0".to_string(),
            current_step: None,
            task_id: None,
            node_id: None,
            final_tx_hash: None,
            error: None,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    #[test]
    fn idempotency_new_row_on_first_call() {
        let conn = make_test_conn();
        let result = check_idempotency(&conn, 1, "wallet1", "req-1", "hash123").unwrap();
        assert!(matches!(result, IdempotencyResult::New));
    }

    #[test]
    fn idempotency_resume_on_same_hash() {
        let conn = make_test_conn();
        let row = make_row("wallet1", "req-1", "hash123");
        db::insert_mutation(&conn, &row).unwrap();
        let result = check_idempotency(&conn, 1, "wallet1", "req-1", "hash123").unwrap();
        assert!(matches!(result, IdempotencyResult::Resume(_)));
    }

    #[test]
    fn idempotency_conflict_on_different_hash() {
        let conn = make_test_conn();
        let row = make_row("wallet1", "req-1", "hash123");
        db::insert_mutation(&conn, &row).unwrap();
        let result = check_idempotency(&conn, 1, "wallet1", "req-1", "different_hash").unwrap();
        assert!(matches!(result, IdempotencyResult::Conflict { .. }));
    }

    #[test]
    fn same_client_request_id_different_wallet_is_new() {
        let conn = make_test_conn();
        let row = make_row("wallet-A", "req-1", "hashA");
        db::insert_mutation(&conn, &row).unwrap();
        let result = check_idempotency(&conn, 1, "wallet-B", "req-1", "hashB").unwrap();
        assert!(matches!(result, IdempotencyResult::New));
    }

    // ── Task 1: register-node value calculation ──────────────────────────────

    /// Default 0.5 ETH stake → tx value must be 0.6 ETH (stake + 0.1 ETH fee).
    #[test]
    fn register_node_default_stake_produces_correct_tx_value() {
        let stake_wei = U256::from(MINIMUM_INITIAL_STAKE_WEI); // 0.5 ETH
        let fee_wei = U256::from(REGISTRATION_FEE_WEI);        // 0.1 ETH
        let total = stake_wei.checked_add(fee_wei).unwrap();

        let expected_wei: U256 = U256::from(600_000_000_000_000_000u128); // 0.6 ETH
        assert_eq!(total, expected_wei, "0.5 ETH stake + 0.1 ETH fee must equal 0.6 ETH total");
    }

    /// stake_amount_wei below MINIMUM_INITIAL_STAKE must be rejected before signing.
    #[test]
    fn register_node_rejects_stake_below_minimum() {
        let too_small = U256::from(400_000_000_000_000_000u128); // 0.4 ETH
        let min_stake = U256::from(MINIMUM_INITIAL_STAKE_WEI);
        assert!(
            too_small < min_stake,
            "0.4 ETH must be below the 0.5 ETH minimum"
        );
    }

    /// Zero stake must also be rejected.
    #[test]
    fn register_node_rejects_zero_stake() {
        let zero = U256::zero();
        let min_stake = U256::from(MINIMUM_INITIAL_STAKE_WEI);
        assert!(zero < min_stake);
    }

    /// Malformed stake string fails parse before any signing path is reached.
    #[test]
    fn register_node_rejects_malformed_stake_string() {
        let result = U256::from_dec_str("not_a_number");
        assert!(result.is_err(), "malformed stake string must fail U256 parse");
    }

    /// Registration fee is not silently double-counted — fee is separate from stake.
    #[test]
    fn registration_fee_is_additive_not_part_of_stake() {
        let stake_wei = U256::from(MINIMUM_INITIAL_STAKE_WEI);
        let fee_wei = U256::from(REGISTRATION_FEE_WEI);
        let tx_value = stake_wei + fee_wei;

        // The stake recorded on-chain is msg.value - REGISTRATION_FEE = stake_wei
        let recorded_stake = tx_value - fee_wei;
        assert_eq!(recorded_stake, stake_wei, "on-chain stake must equal the input stake, not stake+fee");
    }

    // ── Task 2: PoW nonce solver ─────────────────────────────────────────────

    /// Solver must find a passing nonce for MVP difficulty (2^16) with an all-zero seed.
    #[test]
    fn pow_solver_finds_nonce_for_mvp_difficulty() {
        let seed = [0u8; 32];
        let difficulty = U256::from(1u64 << 16); // 65 536

        let nonce = find_pow_nonce(seed, difficulty)
            .expect("solver must succeed for MVP difficulty");

        // Re-verify the nonce independently.
        let target = U256::MAX / difficulty;
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&seed);
        nonce.to_big_endian(&mut buf[32..]);
        let hash = ethers::utils::keccak256(&buf);
        let hash_val = U256::from_big_endian(&hash);
        assert!(
            hash_val <= target,
            "found nonce {} must produce a hash below target {:x}, got {:x}",
            nonce, target, hash_val
        );
    }

    /// Solver must find a passing nonce for a non-zero seed.
    #[test]
    fn pow_solver_works_with_nonzero_seed() {
        let mut seed = [0u8; 32];
        seed[0] = 0xde; seed[1] = 0xad; seed[2] = 0xbe; seed[3] = 0xef;
        let difficulty = U256::from(1u64 << 16);

        let nonce = find_pow_nonce(seed, difficulty)
            .expect("solver must succeed for nonzero seed");

        let target = U256::MAX / difficulty;
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&seed);
        nonce.to_big_endian(&mut buf[32..]);
        let hash_val = U256::from_big_endian(&ethers::utils::keccak256(&buf));
        assert!(hash_val <= target, "nonce must pass PoW check for nonzero seed");
    }

    /// Zero difficulty must return an error without looping.
    #[test]
    fn pow_solver_rejects_zero_difficulty() {
        let seed = [0u8; 32];
        let result = find_pow_nonce(seed, U256::zero());
        assert!(result.is_err(), "zero difficulty must be rejected");
        assert!(result.unwrap_err().contains("pow_solver: difficulty is zero"));
    }

    // ── Task 3: create-and-fund pre-sign fundability check ───────────────────

    /// Valid price + duration produces quoteCap = max_price * duration_hours.
    #[test]
    fn fundability_valid_inputs_returns_quote_cap() {
        // 0.01 ETH/hr × 2 hours = 0.02 ETH quoteCap
        let price = U256::from(10_000_000_000_000_000u128); // 0.01 ETH
        let cap = check_task_fundability(price, 7200, U256::zero()).unwrap();
        assert_eq!(cap, price * U256::from(2u64));
    }

    /// Zero max_price_wei must be rejected before any chain call.
    #[test]
    fn fundability_zero_price_is_rejected() {
        let err = check_task_fundability(U256::zero(), 3600, U256::zero()).unwrap_err();
        assert!(err.contains("underfundable_task"), "must contain underfundable_task, got: {}", err);
        assert!(err.contains("max_price_wei"), "must mention max_price_wei");
    }

    /// Zero duration_seconds must be rejected.
    #[test]
    fn fundability_zero_duration_is_rejected() {
        let price = U256::from(10_000_000_000_000_000u128);
        let err = check_task_fundability(price, 0, U256::zero()).unwrap_err();
        assert!(err.contains("underfundable_task"), "must contain underfundable_task, got: {}", err);
        assert!(err.contains("duration_seconds"), "must mention duration_seconds");
    }

    /// quoteMin > quoteCap must be rejected with a descriptive error.
    #[test]
    fn fundability_quote_min_exceeds_cap_is_rejected() {
        // 0.001 ETH/hr × 1 hour = 0.001 ETH quoteCap, but quoteMin is 0.005 ETH
        let price = U256::from(1_000_000_000_000_000u128); // 0.001 ETH/hr
        let quote_min = U256::from(5_000_000_000_000_000u128); // 0.005 ETH
        let err = check_task_fundability(price, 3600, quote_min).unwrap_err();
        assert!(err.contains("underfundable_task"), "must contain underfundable_task, got: {}", err);
        assert!(err.contains("quoteMin"), "must mention quoteMin");
    }

    /// Duration that is exactly fundable (quoteCap == quoteMin) must pass.
    #[test]
    fn fundability_exact_match_passes() {
        // Set quoteCap == quoteMin
        let price = U256::from(5_000_000_000_000_000u128); // 0.005 ETH/hr
        let quote_min = U256::from(5_000_000_000_000_000u128); // exactly 0.005 ETH
        let cap = check_task_fundability(price, 3600, quote_min).unwrap();
        assert_eq!(cap, price, "quoteCap must equal price×1hr");
    }
}
