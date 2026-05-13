/// Compute-specific SQLite read-model schema and CRUD operations.
///
/// All tables are additive; we use `CREATE TABLE IF NOT EXISTS` + column
/// migrations so the DB can be upgraded without dropping existing data.

use crate::compute::types::{
    ComputeNode, ComputeSnapshot, ComputeTask, NodeStatus, ResourceType,
    SyncCursor, SyncStatus, TaskStatus,
};
use rusqlite::{params, Connection, Result as SqlResult};
use serde_json;

pub fn init_compute_tables(conn: &Connection) -> SqlResult<()> {
    // ── compute_events ────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_events (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            chain_id    INTEGER NOT NULL,
            contract_address TEXT NOT NULL,
            block_number     INTEGER NOT NULL,
            block_hash       TEXT NOT NULL,
            tx_hash          TEXT NOT NULL,
            log_index        INTEGER NOT NULL,
            event_name       TEXT NOT NULL,
            entity_kind      TEXT,
            entity_id        TEXT,
            account_address  TEXT,
            payload_json     TEXT NOT NULL DEFAULT '{}',
            observed_at      TEXT NOT NULL,
            UNIQUE (chain_id, contract_address, tx_hash, log_index)
        );
        CREATE INDEX IF NOT EXISTS idx_compute_events_block
            ON compute_events (chain_id, block_number);
        CREATE INDEX IF NOT EXISTS idx_compute_events_entity
            ON compute_events (entity_kind, entity_id);
        CREATE INDEX IF NOT EXISTS idx_compute_events_account
            ON compute_events (account_address);",
    )?;

    // ── compute_tasks ─────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_tasks (
            chain_id                  INTEGER NOT NULL,
            task_id                   TEXT    NOT NULL,
            buyer                     TEXT    NOT NULL,
            assigned_node_id          TEXT,
            resource_type             INTEGER NOT NULL DEFAULT 0,
            required_power            TEXT    NOT NULL DEFAULT '0',
            duration_seconds          INTEGER NOT NULL DEFAULT 0,
            max_price_wei             TEXT    NOT NULL DEFAULT '0',
            escrow_amount_wei         TEXT    NOT NULL DEFAULT '0',
            status                    TEXT    NOT NULL DEFAULT 'Open',
            specification_uri         TEXT    NOT NULL DEFAULT '',
            min_trust_level           INTEGER NOT NULL DEFAULT 0,
            created_at_ts             INTEGER,
            started_at_ts             INTEGER,
            completed_at_ts           INTEGER,
            challenge_deadline_ts     INTEGER,
            dispute_reason            TEXT,
            disputed_by               TEXT,
            resolved                  INTEGER NOT NULL DEFAULT 0,
            resolved_by               TEXT,
            gross_provider_amount_wei TEXT    NOT NULL DEFAULT '0',
            last_chain_block          INTEGER,
            last_chain_block_hash     TEXT,
            synced_at                 TEXT,
            PRIMARY KEY (chain_id, task_id)
        );
        CREATE INDEX IF NOT EXISTS idx_compute_tasks_buyer
            ON compute_tasks (chain_id, buyer, status);
        CREATE INDEX IF NOT EXISTS idx_compute_tasks_node
            ON compute_tasks (chain_id, assigned_node_id, status);",
    )?;

    // ── compute_nodes ─────────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_nodes (
            chain_id                INTEGER NOT NULL,
            node_id                 TEXT    NOT NULL,
            owner                   TEXT    NOT NULL,
            status                  TEXT    NOT NULL DEFAULT 'Pending',
            resource_type           INTEGER NOT NULL DEFAULT 0,
            compute_power           TEXT    NOT NULL DEFAULT '0',
            staked_amount_wei       TEXT    NOT NULL DEFAULT '0',
            reputation              TEXT    NOT NULL DEFAULT '0',
            total_tasks_completed   INTEGER NOT NULL DEFAULT 0,
            total_earnings_wei      TEXT    NOT NULL DEFAULT '0',
            registered_at_ts        INTEGER,
            last_active_at_ts       INTEGER,
            metadata_uri            TEXT    NOT NULL DEFAULT '',
            trust_level             INTEGER NOT NULL DEFAULT 0,
            pending_task_count      INTEGER NOT NULL DEFAULT 0,
            last_chain_block        INTEGER,
            last_chain_block_hash   TEXT,
            synced_at               TEXT,
            PRIMARY KEY (chain_id, node_id)
        );
        CREATE INDEX IF NOT EXISTS idx_compute_nodes_owner
            ON compute_nodes (chain_id, owner);
        CREATE INDEX IF NOT EXISTS idx_compute_nodes_status
            ON compute_nodes (chain_id, status, resource_type);",
    )?;

    // ── compute_account_summaries ─────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_account_summaries (
            chain_id                    INTEGER NOT NULL,
            account_address             TEXT    NOT NULL,
            pending_buyer_refund_wei    TEXT    NOT NULL DEFAULT '0',
            pending_provider_payout_wei TEXT    NOT NULL DEFAULT '0',
            total_locked_escrow_wei     TEXT    NOT NULL DEFAULT '0',
            synced_at                   TEXT,
            PRIMARY KEY (chain_id, account_address)
        );",
    )?;

    // ── compute_sync_cursors ──────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_sync_cursors (
            scope_key              TEXT PRIMARY KEY,
            chain_id               INTEGER NOT NULL,
            bootstrap_start_block  INTEGER NOT NULL,
            synced_to_block        INTEGER,
            synced_to_block_hash   TEXT,
            confirmed_head_block   INTEGER,
            confirmation_depth     INTEGER NOT NULL DEFAULT 12,
            status                 TEXT    NOT NULL DEFAULT 'unavailable',
            failed_sources_json    TEXT    NOT NULL DEFAULT '[]',
            updated_at             TEXT
        );",
    )?;

    // ── compute_mutations ─────────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_mutations (
            mutation_id         TEXT PRIMARY KEY,
            chain_id            INTEGER NOT NULL,
            wallet_id           TEXT    NOT NULL,
            client_request_id   TEXT    NOT NULL,
            request_hash        TEXT    NOT NULL,
            action              TEXT    NOT NULL,
            status              TEXT    NOT NULL DEFAULT 'pending',
            from_address        TEXT    NOT NULL DEFAULT '',
            current_step        TEXT,
            task_id             TEXT,
            node_id             TEXT,
            final_tx_hash       TEXT,
            error               TEXT,
            created_at          TEXT    NOT NULL,
            updated_at          TEXT    NOT NULL,
            UNIQUE (chain_id, wallet_id, client_request_id)
        );",
    )?;

    // ── compute_mutation_steps ────────────────────────────────────────────
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS compute_mutation_steps (
            step_id              TEXT PRIMARY KEY,
            mutation_id          TEXT NOT NULL,
            step_name            TEXT NOT NULL,
            to_address           TEXT NOT NULL DEFAULT '',
            value_wei            TEXT NOT NULL DEFAULT '0',
            calldata_hash        TEXT NOT NULL DEFAULT '',
            nonce                TEXT,
            tx_hash              TEXT,
            raw_signed_tx_hex    TEXT,
            status               TEXT NOT NULL DEFAULT 'pending',
            receipt_status       INTEGER,
            error                TEXT,
            created_at           TEXT NOT NULL,
            updated_at           TEXT NOT NULL,
            block_number         INTEGER,
            block_hash           TEXT,
            transaction_index    INTEGER,
            gas_used             TEXT,
            effective_gas_price  TEXT,
            projected_at         TEXT,
            FOREIGN KEY (mutation_id) REFERENCES compute_mutations(mutation_id)
        );",
    )?;

    // Additive migration: add receipt/projection columns to any existing DB that
    // was created before these columns were added to the schema.
    for stmt in &[
        "ALTER TABLE compute_mutation_steps ADD COLUMN block_number INTEGER",
        "ALTER TABLE compute_mutation_steps ADD COLUMN block_hash TEXT",
        "ALTER TABLE compute_mutation_steps ADD COLUMN transaction_index INTEGER",
        "ALTER TABLE compute_mutation_steps ADD COLUMN gas_used TEXT",
        "ALTER TABLE compute_mutation_steps ADD COLUMN effective_gas_price TEXT",
        "ALTER TABLE compute_mutation_steps ADD COLUMN projected_at TEXT",
    ] {
        // Ignore errors — column already exists in this DB.
        let _ = conn.execute_batch(stmt);
    }

    Ok(())
}

// ── compute_sync_cursors CRUD ──────────────────────────────────────────────

pub fn upsert_sync_cursor(conn: &Connection, cursor: &SyncCursor) -> SqlResult<()> {
    let failed_json = serde_json::to_string(&cursor.failed_sources).unwrap_or_default();
    conn.execute(
        "INSERT INTO compute_sync_cursors
            (scope_key, chain_id, bootstrap_start_block, synced_to_block,
             synced_to_block_hash, confirmed_head_block, confirmation_depth,
             status, failed_sources_json, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(scope_key) DO UPDATE SET
             chain_id              = excluded.chain_id,
             bootstrap_start_block = excluded.bootstrap_start_block,
             synced_to_block       = excluded.synced_to_block,
             synced_to_block_hash  = excluded.synced_to_block_hash,
             confirmed_head_block  = excluded.confirmed_head_block,
             confirmation_depth    = excluded.confirmation_depth,
             status                = excluded.status,
             failed_sources_json   = excluded.failed_sources_json,
             updated_at            = excluded.updated_at",
        params![
            &cursor.scope_key,
            cursor.chain_id as i64,
            cursor.bootstrap_start_block as i64,
            cursor.synced_to_block.map(|b| b as i64),
            &cursor.synced_to_block_hash,
            cursor.confirmed_head_block.map(|b| b as i64),
            cursor.confirmation_depth as i64,
            cursor.status.as_str(),
            &failed_json,
            &cursor.updated_at,
        ],
    )?;
    Ok(())
}

pub fn load_sync_cursor(conn: &Connection, scope_key: &str) -> SqlResult<Option<SyncCursor>> {
    let result = conn.query_row(
        "SELECT scope_key, chain_id, bootstrap_start_block, synced_to_block,
                synced_to_block_hash, confirmed_head_block, confirmation_depth,
                status, failed_sources_json, updated_at
         FROM compute_sync_cursors WHERE scope_key = ?1",
        params![scope_key],
        |row| {
            let failed_json: String = row.get(8)?;
            let failed_sources: Vec<String> =
                serde_json::from_str(&failed_json).unwrap_or_default();
            let status_str: String = row.get(7)?;
            Ok(SyncCursor {
                scope_key: row.get(0)?,
                chain_id: row.get::<_, i64>(1)? as u64,
                bootstrap_start_block: row.get::<_, i64>(2)? as u64,
                synced_to_block: row.get::<_, Option<i64>>(3)?.map(|v| v as u64),
                synced_to_block_hash: row.get(4)?,
                confirmed_head_block: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                confirmation_depth: row.get::<_, i64>(6)? as u64,
                status: sync_status_from_str(&status_str),
                failed_sources,
                updated_at: row.get(9)?,
            })
        },
    );
    match result {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

fn sync_status_from_str(s: &str) -> SyncStatus {
    match s {
        "fresh" => SyncStatus::Fresh,
        "stale" => SyncStatus::Stale,
        "partial" => SyncStatus::Partial,
        _ => SyncStatus::Unavailable,
    }
}

impl SyncStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncStatus::Fresh => "fresh",
            SyncStatus::Stale => "stale",
            SyncStatus::Partial => "partial",
            SyncStatus::Unavailable => "unavailable",
        }
    }
}

// ── compute_events CRUD ────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ComputeEventRecord {
    pub chain_id: u64,
    pub contract_address: String,
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub log_index: u32,
    pub event_name: String,
    pub entity_kind: Option<String>,
    pub entity_id: Option<String>,
    pub account_address: Option<String>,
    pub payload_json: String,
    pub observed_at: String,
}

/// Idempotent upsert — unique on (chain_id, contract_address, tx_hash, log_index).
pub fn upsert_compute_event(conn: &Connection, ev: &ComputeEventRecord) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO compute_events
            (chain_id, contract_address, block_number, block_hash, tx_hash, log_index,
             event_name, entity_kind, entity_id, account_address, payload_json, observed_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
         ON CONFLICT(chain_id, contract_address, tx_hash, log_index) DO NOTHING",
        params![
            ev.chain_id as i64,
            &ev.contract_address,
            ev.block_number as i64,
            &ev.block_hash,
            &ev.tx_hash,
            ev.log_index as i64,
            &ev.event_name,
            &ev.entity_kind,
            &ev.entity_id,
            &ev.account_address,
            &ev.payload_json,
            &ev.observed_at,
        ],
    )?;
    Ok(())
}

// ── compute_tasks CRUD ─────────────────────────────────────────────────────

pub struct ComputeTaskRow {
    pub chain_id: u64,
    pub task_id: String,
    pub buyer: String,
    pub assigned_node_id: Option<String>,
    pub resource_type: u8,
    pub required_power: String,
    pub duration_seconds: u64,
    pub max_price_wei: String,
    pub escrow_amount_wei: String,
    pub status: String,
    pub specification_uri: String,
    pub min_trust_level: u8,
    pub created_at_ts: Option<u64>,
    pub started_at_ts: Option<u64>,
    pub completed_at_ts: Option<u64>,
    pub challenge_deadline_ts: Option<u64>,
    pub dispute_reason: Option<String>,
    pub disputed_by: Option<String>,
    pub resolved: bool,
    pub resolved_by: Option<String>,
    pub gross_provider_amount_wei: String,
    pub last_chain_block: Option<u64>,
    pub last_chain_block_hash: Option<String>,
    pub synced_at: String,
}

pub fn upsert_compute_task(conn: &Connection, row: &ComputeTaskRow) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO compute_tasks
            (chain_id, task_id, buyer, assigned_node_id, resource_type, required_power,
             duration_seconds, max_price_wei, escrow_amount_wei, status, specification_uri,
             min_trust_level, created_at_ts, started_at_ts, completed_at_ts,
             challenge_deadline_ts, dispute_reason, disputed_by, resolved, resolved_by,
             gross_provider_amount_wei, last_chain_block, last_chain_block_hash, synced_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24)
         ON CONFLICT(chain_id, task_id) DO UPDATE SET
             buyer                     = excluded.buyer,
             assigned_node_id          = excluded.assigned_node_id,
             resource_type             = excluded.resource_type,
             required_power            = excluded.required_power,
             duration_seconds          = excluded.duration_seconds,
             max_price_wei             = excluded.max_price_wei,
             escrow_amount_wei         = excluded.escrow_amount_wei,
             status                    = excluded.status,
             specification_uri         = excluded.specification_uri,
             min_trust_level           = excluded.min_trust_level,
             created_at_ts             = excluded.created_at_ts,
             started_at_ts             = excluded.started_at_ts,
             completed_at_ts           = excluded.completed_at_ts,
             challenge_deadline_ts     = excluded.challenge_deadline_ts,
             dispute_reason            = excluded.dispute_reason,
             disputed_by               = excluded.disputed_by,
             resolved                  = excluded.resolved,
             resolved_by               = excluded.resolved_by,
             gross_provider_amount_wei = excluded.gross_provider_amount_wei,
             last_chain_block          = excluded.last_chain_block,
             last_chain_block_hash     = excluded.last_chain_block_hash,
             synced_at                 = excluded.synced_at",
        params![
            row.chain_id as i64,
            &row.task_id,
            &row.buyer,
            &row.assigned_node_id,
            row.resource_type as i64,
            &row.required_power,
            row.duration_seconds as i64,
            &row.max_price_wei,
            &row.escrow_amount_wei,
            &row.status,
            &row.specification_uri,
            row.min_trust_level as i64,
            row.created_at_ts.map(|v| v as i64),
            row.started_at_ts.map(|v| v as i64),
            row.completed_at_ts.map(|v| v as i64),
            row.challenge_deadline_ts.map(|v| v as i64),
            &row.dispute_reason,
            &row.disputed_by,
            row.resolved as i64,
            &row.resolved_by,
            &row.gross_provider_amount_wei,
            row.last_chain_block.map(|v| v as i64),
            &row.last_chain_block_hash,
            &row.synced_at,
        ],
    )?;
    Ok(())
}

fn row_to_compute_task(row: &rusqlite::Row) -> rusqlite::Result<ComputeTask> {
    let resource_type: i64 = row.get(4)?;
    let status: String = row.get(9)?;
    Ok(ComputeTask {
        task_id: row.get(1)?,
        buyer: row.get(2)?,
        assigned_node_id: row.get(3)?,
        resource_type: ResourceType::from_u8(resource_type as u8),
        required_power: row.get(5)?,
        duration_seconds: row.get::<_, i64>(6)? as u64,
        max_price_wei: row.get(7)?,
        escrow_amount_wei: row.get(8)?,
        status: TaskStatus::from_u8(task_status_code(&status)),
        specification_uri: row.get(10)?,
        min_trust_level: row.get::<_, i64>(11)? as u8,
        created_at: row.get::<_, Option<i64>>(12)?.map(|v| v as u64),
        started_at: row.get::<_, Option<i64>>(13)?.map(|v| v as u64),
        completed_at: row.get::<_, Option<i64>>(14)?.map(|v| v as u64),
        challenge_deadline: row.get::<_, Option<i64>>(15)?.map(|v| v as u64),
        dispute_reason: row.get(16)?,
        disputed_by: row.get(17)?,
        resolved: row.get::<_, i64>(18)? != 0,
        resolved_by: row.get(19)?,
        gross_provider_amount_wei: row.get(20)?,
        last_chain_block: row.get::<_, Option<i64>>(21)?.map(|v| v as u64),
        last_chain_block_hash: row.get(22)?,
        synced_at: row.get(23)?,
    })
}

fn task_status_code(s: &str) -> u8 {
    match s {
        "Open" => 0,
        "Assigned" => 1,
        "InProgress" => 2,
        "Completed" => 3,
        "Verified" => 4,
        "Disputed" => 5,
        _ => 6,
    }
}

// ── compute_nodes CRUD ─────────────────────────────────────────────────────

pub struct ComputeNodeRow {
    pub chain_id: u64,
    pub node_id: String,
    pub owner: String,
    pub status: String,
    pub resource_type: u8,
    pub compute_power: String,
    pub staked_amount_wei: String,
    pub reputation: String,
    pub total_tasks_completed: u64,
    pub total_earnings_wei: String,
    pub registered_at_ts: Option<u64>,
    pub last_active_at_ts: Option<u64>,
    pub metadata_uri: String,
    pub trust_level: u8,
    pub pending_task_count: u64,
    pub last_chain_block: Option<u64>,
    pub last_chain_block_hash: Option<String>,
    pub synced_at: String,
}

pub fn upsert_compute_node(conn: &Connection, row: &ComputeNodeRow) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO compute_nodes
            (chain_id, node_id, owner, status, resource_type, compute_power,
             staked_amount_wei, reputation, total_tasks_completed, total_earnings_wei,
             registered_at_ts, last_active_at_ts, metadata_uri, trust_level,
             pending_task_count, last_chain_block, last_chain_block_hash, synced_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)
         ON CONFLICT(chain_id, node_id) DO UPDATE SET
             owner                 = excluded.owner,
             status                = excluded.status,
             resource_type         = excluded.resource_type,
             compute_power         = excluded.compute_power,
             staked_amount_wei     = excluded.staked_amount_wei,
             reputation            = excluded.reputation,
             total_tasks_completed = excluded.total_tasks_completed,
             total_earnings_wei    = excluded.total_earnings_wei,
             registered_at_ts      = excluded.registered_at_ts,
             last_active_at_ts     = excluded.last_active_at_ts,
             metadata_uri          = excluded.metadata_uri,
             trust_level           = excluded.trust_level,
             pending_task_count    = excluded.pending_task_count,
             last_chain_block      = excluded.last_chain_block,
             last_chain_block_hash = excluded.last_chain_block_hash,
             synced_at             = excluded.synced_at",
        params![
            row.chain_id as i64,
            &row.node_id,
            &row.owner,
            &row.status,
            row.resource_type as i64,
            &row.compute_power,
            &row.staked_amount_wei,
            &row.reputation,
            row.total_tasks_completed as i64,
            &row.total_earnings_wei,
            row.registered_at_ts.map(|v| v as i64),
            row.last_active_at_ts.map(|v| v as i64),
            &row.metadata_uri,
            row.trust_level as i64,
            row.pending_task_count as i64,
            row.last_chain_block.map(|v| v as i64),
            &row.last_chain_block_hash,
            &row.synced_at,
        ],
    )?;
    Ok(())
}

fn row_to_compute_node(row: &rusqlite::Row) -> rusqlite::Result<ComputeNode> {
    let resource_type: i64 = row.get(4)?;
    let status: String = row.get(3)?;
    Ok(ComputeNode {
        node_id: row.get(1)?,
        owner: row.get(2)?,
        status: NodeStatus::from_u8(node_status_code(&status)),
        resource_type: ResourceType::from_u8(resource_type as u8),
        compute_power: row.get(5)?,
        staked_amount_wei: row.get(6)?,
        reputation: row.get(7)?,
        total_tasks_completed: row.get::<_, i64>(8)? as u64,
        total_earnings_wei: row.get(9)?,
        registered_at: row.get::<_, Option<i64>>(10)?.map(|v| v as u64),
        last_active_at: row.get::<_, Option<i64>>(11)?.map(|v| v as u64),
        metadata_uri: row.get(12)?,
        trust_level: row.get::<_, i64>(13)? as u8,
        pending_task_count: row.get::<_, i64>(14)? as u64,
        last_chain_block: row.get::<_, Option<i64>>(15)?.map(|v| v as u64),
        last_chain_block_hash: row.get(16)?,
        synced_at: row.get(17)?,
    })
}

fn node_status_code(s: &str) -> u8 {
    match s {
        "Pending" => 0,
        "Verified" => 1,
        "Active" => 2,
        "Inactive" => 3,
        _ => 4,
    }
}

// ── compute_account_summaries CRUD ─────────────────────────────────────────

pub fn upsert_account_summary(
    conn: &Connection,
    chain_id: u64,
    account_address: &str,
    pending_buyer_refund_wei: &str,
    pending_provider_payout_wei: &str,
    total_locked_escrow_wei: &str,
    synced_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO compute_account_summaries
            (chain_id, account_address, pending_buyer_refund_wei,
             pending_provider_payout_wei, total_locked_escrow_wei, synced_at)
         VALUES (?1,?2,?3,?4,?5,?6)
         ON CONFLICT(chain_id, account_address) DO UPDATE SET
             pending_buyer_refund_wei    = excluded.pending_buyer_refund_wei,
             pending_provider_payout_wei = excluded.pending_provider_payout_wei,
             total_locked_escrow_wei     = excluded.total_locked_escrow_wei,
             synced_at                   = excluded.synced_at",
        params![
            chain_id as i64,
            account_address,
            pending_buyer_refund_wei,
            pending_provider_payout_wei,
            total_locked_escrow_wei,
            synced_at,
        ],
    )?;
    Ok(())
}

// ── compute_mutations CRUD ─────────────────────────────────────────────────

pub struct MutationRow {
    pub mutation_id: String,
    pub chain_id: u64,
    pub wallet_id: String,
    pub client_request_id: String,
    pub request_hash: String,
    pub action: String,
    pub status: String,
    pub from_address: String,
    pub current_step: Option<String>,
    pub task_id: Option<String>,
    pub node_id: Option<String>,
    pub final_tx_hash: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn insert_mutation(conn: &Connection, row: &MutationRow) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO compute_mutations
            (mutation_id, chain_id, wallet_id, client_request_id, request_hash, action,
             status, from_address, current_step, task_id, node_id, final_tx_hash, error,
             created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            &row.mutation_id,
            row.chain_id as i64,
            &row.wallet_id,
            &row.client_request_id,
            &row.request_hash,
            &row.action,
            &row.status,
            &row.from_address,
            &row.current_step,
            &row.task_id,
            &row.node_id,
            &row.final_tx_hash,
            &row.error,
            &row.created_at,
            &row.updated_at,
        ],
    )?;
    Ok(())
}

pub fn load_mutation_by_client_request_id(
    conn: &Connection,
    chain_id: u64,
    wallet_id: &str,
    client_request_id: &str,
) -> SqlResult<Option<MutationRow>> {
    let result = conn.query_row(
        "SELECT mutation_id, chain_id, wallet_id, client_request_id, request_hash,
                action, status, from_address, current_step, task_id, node_id,
                final_tx_hash, error, created_at, updated_at
         FROM compute_mutations
         WHERE chain_id = ?1 AND wallet_id = ?2 AND client_request_id = ?3",
        params![chain_id as i64, wallet_id, client_request_id],
        |row| {
            Ok(MutationRow {
                mutation_id: row.get(0)?,
                chain_id: row.get::<_, i64>(1)? as u64,
                wallet_id: row.get(2)?,
                client_request_id: row.get(3)?,
                request_hash: row.get(4)?,
                action: row.get(5)?,
                status: row.get(6)?,
                from_address: row.get(7)?,
                current_step: row.get(8)?,
                task_id: row.get(9)?,
                node_id: row.get(10)?,
                final_tx_hash: row.get(11)?,
                error: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        },
    );
    match result {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn update_mutation_status(
    conn: &Connection,
    mutation_id: &str,
    status: &str,
    current_step: Option<&str>,
    task_id: Option<&str>,
    node_id: Option<&str>,
    error: Option<&str>,
    updated_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE compute_mutations SET
             status       = ?2,
             current_step = ?3,
             task_id      = COALESCE(?4, task_id),
             node_id      = COALESCE(?5, node_id),
             error        = ?6,
             updated_at   = ?7
         WHERE mutation_id = ?1",
        params![mutation_id, status, current_step, task_id, node_id, error, updated_at],
    )?;
    Ok(())
}

pub fn update_mutation_tx(
    conn: &Connection,
    mutation_id: &str,
    final_tx_hash: &str,
    status: &str,
    updated_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE compute_mutations SET
             final_tx_hash = ?2,
             status        = ?3,
             updated_at    = ?4
         WHERE mutation_id = ?1",
        params![mutation_id, final_tx_hash, status, updated_at],
    )?;
    Ok(())
}

pub fn update_mutation_node_id(
    conn: &Connection,
    mutation_id: &str,
    node_id: &str,
    updated_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE compute_mutations \
         SET node_id = ?1, current_step = 'register_node_done', updated_at = ?2 \
         WHERE mutation_id = ?3",
        params![node_id, updated_at, mutation_id],
    )?;
    Ok(())
}

pub fn update_mutation_task_id(
    conn: &Connection,
    mutation_id: &str,
    task_id: &str,
    updated_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE compute_mutations \
         SET task_id = ?1, current_step = 'create_task_done', updated_at = ?2 \
         WHERE mutation_id = ?3",
        params![task_id, updated_at, mutation_id],
    )?;
    Ok(())
}

pub fn load_mutation_by_id(
    conn: &Connection,
    mutation_id: &str,
) -> SqlResult<Option<MutationRow>> {
    let result = conn.query_row(
        "SELECT mutation_id, chain_id, wallet_id, client_request_id, request_hash,
                action, status, from_address, current_step, task_id, node_id,
                final_tx_hash, error, created_at, updated_at
         FROM compute_mutations WHERE mutation_id = ?1",
        params![mutation_id],
        |row| {
            Ok(MutationRow {
                mutation_id: row.get(0)?,
                chain_id: row.get::<_, i64>(1)? as u64,
                wallet_id: row.get(2)?,
                client_request_id: row.get(3)?,
                request_hash: row.get(4)?,
                action: row.get(5)?,
                status: row.get(6)?,
                from_address: row.get(7)?,
                current_step: row.get(8)?,
                task_id: row.get(9)?,
                node_id: row.get(10)?,
                final_tx_hash: row.get(11)?,
                error: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        },
    );
    match result {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

// ── compute_mutation_steps CRUD ────────────────────────────────────────────

pub struct MutationStepRow {
    pub step_id: String,
    pub mutation_id: String,
    pub step_name: String,
    pub to_address: String,
    pub value_wei: String,
    pub calldata_hash: String,
    pub nonce: Option<String>,
    pub tx_hash: Option<String>,
    pub raw_signed_tx_hex: Option<String>,
    pub status: String,
    pub receipt_status: Option<i32>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // Receipt metadata (populated by update_mutation_step_receipt)
    pub block_number: Option<u64>,
    pub block_hash: Option<String>,
    pub transaction_index: Option<u64>,
    pub gas_used: Option<String>,
    pub effective_gas_price: Option<String>,
    pub projected_at: Option<String>,
}

/// Upsert on (mutation_id, step_name) — if step already exists keep
/// nonce/tx_hash/raw_signed_tx so we never re-sign the same step.
pub fn upsert_mutation_step(conn: &Connection, row: &MutationStepRow) -> SqlResult<()> {
    conn.execute(
        "INSERT INTO compute_mutation_steps
            (step_id, mutation_id, step_name, to_address, value_wei, calldata_hash,
             nonce, tx_hash, raw_signed_tx_hex, status, receipt_status, error, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
         ON CONFLICT(step_id) DO NOTHING",
        params![
            &row.step_id,
            &row.mutation_id,
            &row.step_name,
            &row.to_address,
            &row.value_wei,
            &row.calldata_hash,
            &row.nonce,
            &row.tx_hash,
            &row.raw_signed_tx_hex,
            &row.status,
            &row.receipt_status,
            &row.error,
            &row.created_at,
            &row.updated_at,
        ],
    )?;
    Ok(())
}

pub fn load_mutation_step(
    conn: &Connection,
    mutation_id: &str,
    step_name: &str,
) -> SqlResult<Option<MutationStepRow>> {
    let result = conn.query_row(
        "SELECT step_id, mutation_id, step_name, to_address, value_wei, calldata_hash,
                nonce, tx_hash, raw_signed_tx_hex, status, receipt_status, error,
                created_at, updated_at,
                block_number, block_hash, transaction_index, gas_used,
                effective_gas_price, projected_at
         FROM compute_mutation_steps
         WHERE mutation_id = ?1 AND step_name = ?2",
        params![mutation_id, step_name],
        |row| {
            Ok(MutationStepRow {
                step_id: row.get(0)?,
                mutation_id: row.get(1)?,
                step_name: row.get(2)?,
                to_address: row.get(3)?,
                value_wei: row.get(4)?,
                calldata_hash: row.get(5)?,
                nonce: row.get(6)?,
                tx_hash: row.get(7)?,
                raw_signed_tx_hex: row.get(8)?,
                status: row.get(9)?,
                receipt_status: row.get(10)?,
                error: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
                block_number: row.get::<_, Option<i64>>(14)?.map(|v| v as u64),
                block_hash: row.get(15)?,
                transaction_index: row.get::<_, Option<i64>>(16)?.map(|v| v as u64),
                gas_used: row.get(17)?,
                effective_gas_price: row.get(18)?,
                projected_at: row.get(19)?,
            })
        },
    );
    match result {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn update_mutation_step(
    conn: &Connection,
    mutation_id: &str,
    step_name: &str,
    status: &str,
    receipt_status: Option<i32>,
    error: Option<&str>,
    updated_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE compute_mutation_steps SET
             status         = ?3,
             receipt_status = COALESCE(?4, receipt_status),
             error          = ?5,
             updated_at     = ?6
         WHERE mutation_id = ?1 AND step_name = ?2",
        params![mutation_id, step_name, status, receipt_status, error, updated_at],
    )?;
    Ok(())
}

/// Persist receipt metadata for a confirmed step WITHOUT changing status, error, nonce,
/// tx_hash, or raw_signed_tx_hex.  Call this as soon as the receipt is obtained, BEFORE
/// projection.  status is updated separately by mark_step_projected.
///
/// Forbidden: must not touch nonce / tx_hash / raw_signed_tx_hex.
pub fn update_mutation_step_receipt(
    conn: &Connection,
    mutation_id: &str,
    step_name: &str,
    block_number: u64,
    block_hash: &str,
    transaction_index: u64,
    gas_used: &str,
    effective_gas_price: &str,
    updated_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE compute_mutation_steps SET
             block_number          = ?3,
             block_hash            = ?4,
             transaction_index     = ?5,
             gas_used              = ?6,
             effective_gas_price   = ?7,
             updated_at            = ?8
         WHERE mutation_id = ?1 AND step_name = ?2",
        params![
            mutation_id,
            step_name,
            block_number as i64,
            block_hash,
            transaction_index as i64,
            gas_used,
            effective_gas_price,
            updated_at,
        ],
    )?;
    Ok(())
}

/// Mark a step as fully projected into the SQLite read model.  Sets status='projected',
/// receipt_status=1, and projected_at.  Only call AFTER write_projection_batch succeeds.
///
/// Forbidden: must not be called without prior update_mutation_step_receipt.
pub fn mark_step_projected(
    conn: &Connection,
    mutation_id: &str,
    step_name: &str,
    projected_at: &str,
) -> SqlResult<()> {
    conn.execute(
        "UPDATE compute_mutation_steps SET
             status         = 'projected',
             receipt_status = 1,
             projected_at   = ?3,
             updated_at     = ?3
         WHERE mutation_id = ?1 AND step_name = ?2",
        params![mutation_id, step_name, projected_at],
    )?;
    Ok(())
}

// ── Snapshot assembly ──────────────────────────────────────────────────────

/// Sum a slice of decimal-string wei values using `u128` arithmetic.
///
/// Skips empty strings and `"0"` entries.  Returns `Err` (propagated as a
/// `rusqlite::Error`) if any non-empty, non-zero value is not a valid decimal
/// `u128`.  `u128::MAX` is approximately `3.4 × 10^38` wei, which is far
/// above any realistic total ETH supply, so overflow is treated as a hard
/// error rather than silently saturating.
fn sum_wei_decimal_strings(values: &[String]) -> SqlResult<String> {
    let mut total: u128 = 0;
    for v in values {
        let trimmed = v.trim();
        if trimmed.is_empty() || trimmed == "0" {
            continue;
        }
        let n = trimmed.parse::<u128>().map_err(|_| {
            rusqlite::Error::InvalidParameterName(format!(
                "malformed escrow_amount_wei: {:?}",
                trimmed
            ))
        })?;
        total = total.checked_add(n).ok_or_else(|| {
            rusqlite::Error::InvalidParameterName(
                "escrow_amount_wei sum overflowed u128".to_string(),
            )
        })?;
    }
    Ok(total.to_string())
}

/// Build the full ComputeSnapshot for a wallet from the local read model.
pub fn build_snapshot_from_db(
    conn: &Connection,
    chain_id: u64,
    wallet_address: &str,
) -> SqlResult<ComputeSnapshot> {
    // Buyer tasks
    let mut stmt = conn.prepare(
        "SELECT chain_id,task_id,buyer,assigned_node_id,resource_type,required_power,
                duration_seconds,max_price_wei,escrow_amount_wei,status,specification_uri,
                min_trust_level,created_at_ts,started_at_ts,completed_at_ts,
                challenge_deadline_ts,dispute_reason,disputed_by,resolved,resolved_by,
                gross_provider_amount_wei,last_chain_block,last_chain_block_hash,synced_at
         FROM compute_tasks
         WHERE chain_id = ?1 AND LOWER(buyer) = LOWER(?2)
         ORDER BY created_at_ts DESC",
    )?;
    let buyer_tasks: Vec<ComputeTask> = stmt
        .query_map(params![chain_id as i64, wallet_address], |row| {
            row_to_compute_task(row)
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Open tasks (status = Open, escrow > 0)
    let mut stmt = conn.prepare(
        "SELECT chain_id,task_id,buyer,assigned_node_id,resource_type,required_power,
                duration_seconds,max_price_wei,escrow_amount_wei,status,specification_uri,
                min_trust_level,created_at_ts,started_at_ts,completed_at_ts,
                challenge_deadline_ts,dispute_reason,disputed_by,resolved,resolved_by,
                gross_provider_amount_wei,last_chain_block,last_chain_block_hash,synced_at
         FROM compute_tasks
         WHERE chain_id = ?1 AND status = 'Open'
           AND CAST(escrow_amount_wei AS INTEGER) > 0
         ORDER BY created_at_ts DESC",
    )?;
    let open_tasks: Vec<ComputeTask> = stmt
        .query_map(params![chain_id as i64], |row| row_to_compute_task(row))?
        .filter_map(|r| r.ok())
        .collect();

    // Disputes
    let disputes: Vec<ComputeTask> = buyer_tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Disputed)
        .cloned()
        .collect();

    // Provider tasks — tasks assigned to nodes owned by wallet
    let mut stmt = conn.prepare(
        "SELECT ct.chain_id,ct.task_id,ct.buyer,ct.assigned_node_id,ct.resource_type,
                ct.required_power,ct.duration_seconds,ct.max_price_wei,ct.escrow_amount_wei,
                ct.status,ct.specification_uri,ct.min_trust_level,ct.created_at_ts,
                ct.started_at_ts,ct.completed_at_ts,ct.challenge_deadline_ts,ct.dispute_reason,
                ct.disputed_by,ct.resolved,ct.resolved_by,ct.gross_provider_amount_wei,
                ct.last_chain_block,ct.last_chain_block_hash,ct.synced_at
         FROM compute_tasks ct
         JOIN compute_nodes cn
           ON ct.chain_id = cn.chain_id
          AND ct.assigned_node_id = cn.node_id
         WHERE ct.chain_id = ?1
           AND LOWER(cn.owner) = LOWER(?2)
         ORDER BY ct.created_at_ts DESC",
    )?;
    let provider_tasks: Vec<ComputeTask> = stmt
        .query_map(params![chain_id as i64, wallet_address], |row| {
            row_to_compute_task(row)
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Owned nodes
    let mut stmt = conn.prepare(
        "SELECT chain_id,node_id,owner,status,resource_type,compute_power,staked_amount_wei,
                reputation,total_tasks_completed,total_earnings_wei,registered_at_ts,
                last_active_at_ts,metadata_uri,trust_level,pending_task_count,
                last_chain_block,last_chain_block_hash,synced_at
         FROM compute_nodes
         WHERE chain_id = ?1 AND LOWER(owner) = LOWER(?2)
         ORDER BY registered_at_ts DESC",
    )?;
    let owned_nodes: Vec<ComputeNode> = stmt
        .query_map(params![chain_id as i64, wallet_address], |row| {
            row_to_compute_node(row)
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Active nodes
    let mut stmt = conn.prepare(
        "SELECT chain_id,node_id,owner,status,resource_type,compute_power,staked_amount_wei,
                reputation,total_tasks_completed,total_earnings_wei,registered_at_ts,
                last_active_at_ts,metadata_uri,trust_level,pending_task_count,
                last_chain_block,last_chain_block_hash,synced_at
         FROM compute_nodes
         WHERE chain_id = ?1 AND status = 'Active'
         ORDER BY reputation DESC",
    )?;
    let active_nodes: Vec<ComputeNode> = stmt
        .query_map(params![chain_id as i64], |row| row_to_compute_node(row))?
        .filter_map(|r| r.ok())
        .collect();

    // Account summary — pending refund/payout from chain; locked escrow computed from DB.
    let (pending_buyer_refund_wei, pending_provider_payout_wei) = conn
        .query_row(
            "SELECT pending_buyer_refund_wei, pending_provider_payout_wei
             FROM compute_account_summaries
             WHERE chain_id = ?1 AND LOWER(account_address) = LOWER(?2)",
            params![chain_id as i64, wallet_address],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .unwrap_or_else(|_| ("0".to_string(), "0".to_string()));

    // Safe Rust-level u128 aggregation — avoids SQLite INTEGER overflow for large
    // wei values.  `SUM(CAST(... AS INTEGER))` overflows silently at i64::MAX
    // (~9.2 ETH), so we fetch individual decimal strings and sum in Rust.
    let total_locked_escrow_wei: String = {
        let mut stmt = conn.prepare(
            "SELECT escrow_amount_wei
             FROM compute_tasks
             WHERE chain_id = ?1 AND LOWER(buyer) = LOWER(?2)
               AND status NOT IN ('Cancelled')",
        )?;
        let rows: Vec<String> = stmt
            .query_map(params![chain_id as i64, wallet_address], |row| {
                row.get::<_, String>(0)
            })?
            .filter_map(|r| r.ok())
            .collect();
        sum_wei_decimal_strings(&rows)?
    };

    Ok(ComputeSnapshot {
        open_tasks,
        buyer_tasks,
        provider_tasks,
        disputes,
        owned_nodes,
        active_nodes,
        pending_buyer_refund_wei,
        pending_provider_payout_wei,
        total_locked_escrow_wei,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn open_mem_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        init_compute_tables(&conn).unwrap();
        conn
    }

    #[test]
    fn event_upsert_is_idempotent_on_unique_key() {
        let conn = open_mem_db();

        let ev = ComputeEventRecord {
            chain_id: 1,
            contract_address: "0xabc".to_string(),
            block_number: 100,
            block_hash: "0xblockhash".to_string(),
            tx_hash: "0xtxhash".to_string(),
            log_index: 0,
            event_name: "TaskCreated".to_string(),
            entity_kind: Some("task".to_string()),
            entity_id: Some("0xtaskid".to_string()),
            account_address: Some("0xbuyer".to_string()),
            payload_json: "{}".to_string(),
            observed_at: "2024-01-01T00:00:00Z".to_string(),
        };

        upsert_compute_event(&conn, &ev).unwrap();
        // Second insert with same (chain_id, contract_address, tx_hash, log_index) must not fail.
        upsert_compute_event(&conn, &ev).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM compute_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "Duplicate event upsert must be idempotent");
    }

    #[test]
    fn sync_cursor_bootstrap_start_block_is_persisted_and_loaded() {
        let conn = open_mem_db();

        let cursor = SyncCursor {
            scope_key: "compute:1".to_string(),
            chain_id: 1,
            bootstrap_start_block: 500,
            synced_to_block: None,
            synced_to_block_hash: None,
            confirmed_head_block: None,
            confirmation_depth: 12,
            status: SyncStatus::Unavailable,
            failed_sources: vec![],
            updated_at: None,
        };

        upsert_sync_cursor(&conn, &cursor).unwrap();

        let loaded = load_sync_cursor(&conn, "compute:1").unwrap().unwrap();
        assert_eq!(loaded.bootstrap_start_block, 500);
        assert!(matches!(loaded.status, SyncStatus::Unavailable));
    }

    #[test]
    fn partial_refresh_must_not_advance_cursor_beyond_last_fully_processed_block() {
        let conn = open_mem_db();

        // First, simulate a fresh sync up to block 1000.
        let cursor_v1 = SyncCursor {
            scope_key: "compute:1".to_string(),
            chain_id: 1,
            bootstrap_start_block: 500,
            synced_to_block: Some(1000),
            synced_to_block_hash: Some("0xhash_1000".to_string()),
            confirmed_head_block: Some(1000),
            confirmation_depth: 12,
            status: SyncStatus::Fresh,
            failed_sources: vec![],
            updated_at: Some("2024-01-01T00:00:00Z".to_string()),
        };
        upsert_sync_cursor(&conn, &cursor_v1).unwrap();

        // Simulate partial failure: should store partial status, NOT advance to 1200.
        let cursor_v2 = SyncCursor {
            scope_key: "compute:1".to_string(),
            chain_id: 1,
            bootstrap_start_block: 500,
            synced_to_block: Some(1050), // only partially advanced
            synced_to_block_hash: Some("0xhash_1050".to_string()),
            confirmed_head_block: Some(1200),
            confirmation_depth: 12,
            status: SyncStatus::Partial,
            failed_sources: vec!["node_registry".to_string()],
            updated_at: Some("2024-01-01T01:00:00Z".to_string()),
        };
        upsert_sync_cursor(&conn, &cursor_v2).unwrap();

        let loaded = load_sync_cursor(&conn, "compute:1").unwrap().unwrap();
        assert_eq!(
            loaded.synced_to_block,
            Some(1050),
            "partial failure must not push cursor to 1200"
        );
        assert!(matches!(loaded.status, SyncStatus::Partial));
        assert!(!loaded.failed_sources.is_empty());
    }

    #[test]
    fn total_locked_escrow_aggregates_non_cancelled_tasks() {
        let conn = open_mem_db();

        // Insert 3 tasks for wallet 0xbuyer: one Open, one Assigned, one Cancelled.
        // Expect locked escrow = Open + Assigned (Cancelled excluded).
        let insert = |task_id: &str, status: &str, escrow_wei: &str| {
            conn.execute(
                "INSERT INTO compute_tasks (chain_id, task_id, buyer, assigned_node_id,
                     resource_type, required_power, duration_seconds, max_price_wei,
                     escrow_amount_wei, status, specification_uri, min_trust_level,
                     created_at_ts, started_at_ts, completed_at_ts, challenge_deadline_ts,
                     dispute_reason, disputed_by, resolved, resolved_by,
                     gross_provider_amount_wei, last_chain_block, last_chain_block_hash, synced_at)
                 VALUES (1, ?1, '0xbuyer', NULL,
                     0, '100', 3600, '1000000000000000',
                     ?2, ?3, 'uri', 0,
                     '2024-01-01T00:00:00Z', NULL, NULL, NULL,
                     NULL, NULL, 0, NULL,
                     '0', 1, '0xblock', '2024-01-01T00:00:00Z')",
                rusqlite::params![task_id, escrow_wei, status],
            ).unwrap()
        };
        insert("0xtask_open",      "Open",      "1000000000000000000"); // 1 ETH
        insert("0xtask_assigned",  "Assigned",  "2000000000000000000"); // 2 ETH
        insert("0xtask_cancelled", "Cancelled", "3000000000000000000"); // should be excluded

        let snap = build_snapshot_from_db(&conn, 1, "0xbuyer").unwrap();

        // locked = 1 ETH + 2 ETH = 3 ETH
        let locked: u128 = snap.total_locked_escrow_wei.parse().unwrap();
        assert_eq!(
            locked,
            3_000_000_000_000_000_000u128,
            "total_locked_escrow_wei must sum Open+Assigned and exclude Cancelled"
        );
    }

    // ── safe wei aggregation tests ──────────────────────────────────────────

    fn insert_task(conn: &Connection, task_id: &str, status: &str, escrow_wei: &str) {
        conn.execute(
            "INSERT INTO compute_tasks (chain_id, task_id, buyer, assigned_node_id,
                 resource_type, required_power, duration_seconds, max_price_wei,
                 escrow_amount_wei, status, specification_uri, min_trust_level,
                 created_at_ts, started_at_ts, completed_at_ts, challenge_deadline_ts,
                 dispute_reason, disputed_by, resolved, resolved_by,
                 gross_provider_amount_wei, last_chain_block, last_chain_block_hash, synced_at)
             VALUES (1, ?1, '0xbuyer', NULL,
                 0, '100', 3600, '1000000000000000000',
                 ?2, ?3, 'uri', 0,
                 1704067200, NULL, NULL, NULL,
                 NULL, NULL, 0, NULL,
                 '0', 1, '0xblock', '2024-01-01T00:00:00Z')",
            rusqlite::params![task_id, escrow_wei, status],
        )
        .unwrap();
    }

    #[test]
    fn single_large_escrow_above_i64_max_is_preserved_exactly() {
        // 10^19 wei ≈ 10 ETH; i64::MAX ≈ 9.22 × 10^18, so this overflows i64.
        let conn = open_mem_db();
        insert_task(&conn, "0xtask1", "Open", "10000000000000000000");

        let snap = build_snapshot_from_db(&conn, 1, "0xbuyer").unwrap();
        assert_eq!(
            snap.total_locked_escrow_wei, "10000000000000000000",
            "single large escrow must be returned exactly"
        );
    }

    #[test]
    fn two_large_escrow_rows_sum_to_correct_value_not_zero() {
        // Each row: 9 × 10^18 wei; sum: 18 × 10^18.
        // With i64 aggregation both individually fit but the sum overflows i64::MAX
        // (9.22 × 10^18), producing a wrong or zero result.
        let conn = open_mem_db();
        insert_task(&conn, "0xtask1", "Open",     "9000000000000000000");
        insert_task(&conn, "0xtask2", "Assigned", "9000000000000000000");

        let snap = build_snapshot_from_db(&conn, 1, "0xbuyer").unwrap();
        assert_eq!(
            snap.total_locked_escrow_wei, "18000000000000000000",
            "two large escrow rows must not overflow to '0'"
        );
    }

    #[test]
    fn large_cancelled_escrow_is_excluded_from_locked_total() {
        let conn = open_mem_db();
        insert_task(&conn, "0xtask_open",      "Open",      "1000000000000000000");
        insert_task(&conn, "0xtask_cancelled", "Cancelled", "10000000000000000000");

        let snap = build_snapshot_from_db(&conn, 1, "0xbuyer").unwrap();
        assert_eq!(
            snap.total_locked_escrow_wei, "1000000000000000000",
            "cancelled escrow must not be included in locked total"
        );
    }

    #[test]
    fn malformed_escrow_amount_wei_returns_error() {
        let conn = open_mem_db();
        // Store a non-numeric string as escrow — should propagate an error,
        // not silently return "0" or corrupt the snapshot.
        insert_task(&conn, "0xtask1", "Open", "not_a_number");

        let result = build_snapshot_from_db(&conn, 1, "0xbuyer");
        assert!(
            result.is_err(),
            "malformed escrow_amount_wei must return Err, not a silent zero"
        );
    }

    // ── Task 1: receipt metadata tests ─────────────────────────────────────────

    fn insert_step(conn: &Connection, mutation_id: &str, step_name: &str) {
        let now = "2024-01-01T00:00:00Z";
        conn.execute(
            "INSERT INTO compute_mutations
                (mutation_id, chain_id, wallet_id, client_request_id, request_hash,
                 action, status, from_address, created_at, updated_at)
             VALUES (?1, 1, 'w1', 'req-1', 'hash1', 'test', 'pending', '0x0', ?2, ?2)",
            rusqlite::params![mutation_id, now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO compute_mutation_steps
                (step_id, mutation_id, step_name, nonce, tx_hash, raw_signed_tx_hex,
                 status, created_at, updated_at)
             VALUES (?1, ?2, ?3, '42', '0xabcdef', '0xrawraw', 'broadcast', ?4, ?4)",
            rusqlite::params![
                format!("{}::{}", mutation_id, step_name),
                mutation_id,
                step_name,
                now
            ],
        )
        .unwrap();
    }

    #[test]
    fn compute_mutation_steps_receipt_columns_migrate_additively() {
        // Running init_compute_tables on an in-memory DB that was just created
        // should create all columns includig the additive receipt columns.
        let conn = open_mem_db();
        insert_step(&conn, "mut-1", "register_node");

        // All 6 new columns should be readable (will be NULL for fresh row).
        let row = load_mutation_step(&conn, "mut-1", "register_node")
            .unwrap()
            .unwrap();
        assert!(row.block_number.is_none());
        assert!(row.block_hash.is_none());
        assert!(row.transaction_index.is_none());
        assert!(row.gas_used.is_none());
        assert!(row.effective_gas_price.is_none());
        assert!(row.projected_at.is_none());
    }

    #[test]
    fn update_step_receipt_preserves_signed_tx_fields() {
        let conn = open_mem_db();
        insert_step(&conn, "mut-2", "accept_task");

        // Update only receipt metadata.
        let now = "2024-06-01T12:00:00Z";
        update_mutation_step_receipt(
            &conn,
            "mut-2",
            "accept_task",
            100,
            "0xblockhash",
            5,
            "21000",
            "1000000000",
            now,
        )
        .unwrap();

        let row = load_mutation_step(&conn, "mut-2", "accept_task")
            .unwrap()
            .unwrap();

        // Receipt metadata must be set.
        assert_eq!(row.block_number, Some(100));
        assert_eq!(row.block_hash.as_deref(), Some("0xblockhash"));
        assert_eq!(row.transaction_index, Some(5));
        assert_eq!(row.gas_used.as_deref(), Some("21000"));
        assert_eq!(row.effective_gas_price.as_deref(), Some("1000000000"));

        // Original signing fields must be unchanged.
        assert_eq!(row.nonce.as_deref(), Some("42"),   "nonce must not be overwritten");
        assert_eq!(row.tx_hash.as_deref(), Some("0xabcdef"), "tx_hash must not be overwritten");
        assert_eq!(row.raw_signed_tx_hex.as_deref(), Some("0xrawraw"), "raw_signed_tx_hex must not be overwritten");
        // projected_at must not be set by update_mutation_step_receipt.
        assert!(row.projected_at.is_none(), "update_mutation_step_receipt must not set projected_at");
    }
}
