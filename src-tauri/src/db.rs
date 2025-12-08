use crate::wallet::types::WalletInfo;
use chrono::Utc;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::sync::Mutex;
use uuid::Uuid;

pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct AssetBalanceData {
    pub wallet_id: String,
    pub chain: String,
    pub chain_id: u64,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub contract_address: Option<String>,
    pub balance: String,
    pub balance_float: f64,
    pub usd_price: f64,
    pub usd_value: f64,
}

impl Database {
    pub fn new(db_path: &str) -> SqliteResult<Self> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )?;

        let db = Database {
            conn: Mutex::new(conn),
        };

        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        // Bitcoin wallets table - stores only address, not private key
        conn.execute(
            "CREATE TABLE IF NOT EXISTS bitcoin_wallets (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                wallet_type TEXT NOT NULL,
                address TEXT NOT NULL UNIQUE,
                balance REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Bitcoin wallet secrets table - stores encrypted mnemonic/private key
        conn.execute(
            "CREATE TABLE IF NOT EXISTS bitcoin_wallet_secrets (
                wallet_id TEXT PRIMARY KEY,
                secret_data TEXT NOT NULL,
                secret_type TEXT NOT NULL,
                FOREIGN KEY (wallet_id) REFERENCES bitcoin_wallets(id)
            )",
            [],
        )?;

        // EVM wallets table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS evm_wallets (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                wallet_type TEXT NOT NULL,
                address TEXT NOT NULL UNIQUE,
                balance REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // EVM wallet secrets table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS evm_wallet_secrets (
                wallet_id TEXT PRIMARY KEY,
                secret_data TEXT NOT NULL,
                secret_type TEXT NOT NULL,
                FOREIGN KEY (wallet_id) REFERENCES evm_wallets(id)
            )",
            [],
        )?;

        // EVM asset balances table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS evm_asset_balances (
                id TEXT PRIMARY KEY,
                wallet_id TEXT NOT NULL,
                chain TEXT NOT NULL,
                chain_id INTEGER NOT NULL,
                asset_symbol TEXT NOT NULL,
                asset_name TEXT NOT NULL,
                asset_decimals INTEGER NOT NULL,
                contract_address TEXT,
                balance TEXT NOT NULL,
                balance_float REAL NOT NULL,
                usd_price REAL DEFAULT 0,
                usd_value REAL DEFAULT 0,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (wallet_id) REFERENCES evm_wallets(id),
                UNIQUE(wallet_id, chain, asset_symbol)
            )",
            [],
        )?;

        Ok(())
    }

    pub fn add_bitcoin_wallet(
        &self,
        label: String,
        wallet_type: String,
        address: String,
    ) -> SqliteResult<WalletInfo> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO bitcoin_wallets (id, label, wallet_type, address, balance, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![&id, &label, &wallet_type, &address, 0.0, &now, &now],
        )?;

        Ok(WalletInfo {
            id,
            label,
            wallet_type,
            address,
            balance: 0.0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn add_wallet_secret(
        &self,
        wallet_id: String,
        secret_data: String,
        secret_type: String,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO bitcoin_wallet_secrets (wallet_id, secret_data, secret_type)
             VALUES (?1, ?2, ?3)",
            params![&wallet_id, &secret_data, &secret_type],
        )?;

        Ok(())
    }

    pub fn get_wallet_secret(&self, wallet_id: &str) -> SqliteResult<Option<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT secret_data, secret_type FROM bitcoin_wallet_secrets WHERE wallet_id = ?1",
        )?;

        let result = stmt.query_row(params![wallet_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });

        match result {
            Ok(secret) => Ok(Some(secret)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn delete_bitcoin_wallet(&self, wallet_id: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();

        // Delete wallet secret first
        conn.execute(
            "DELETE FROM bitcoin_wallet_secrets WHERE wallet_id = ?1",
            params![wallet_id],
        )?;

        // Delete wallet
        let rows_affected = conn.execute(
            "DELETE FROM bitcoin_wallets WHERE id = ?1",
            params![wallet_id],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn get_bitcoin_wallets(&self) -> SqliteResult<Vec<WalletInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, label, wallet_type, address, balance, created_at, updated_at
             FROM bitcoin_wallets ORDER BY created_at DESC",
        )?;

        let wallets = stmt.query_map([], |row| {
            Ok(WalletInfo {
                id: row.get(0)?,
                label: row.get(1)?,
                wallet_type: row.get(2)?,
                address: row.get(3)?,
                balance: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for wallet in wallets {
            result.push(wallet?);
        }

        Ok(result)
    }

    pub fn get_bitcoin_wallet(&self, wallet_id: &str) -> SqliteResult<Option<WalletInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, label, wallet_type, address, balance, created_at, updated_at
             FROM bitcoin_wallets WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![wallet_id], |row| {
            Ok(WalletInfo {
                id: row.get(0)?,
                label: row.get(1)?,
                wallet_type: row.get(2)?,
                address: row.get(3)?,
                balance: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        });

        match result {
            Ok(wallet) => Ok(Some(wallet)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn update_bitcoin_wallet_balance(&self, wallet_id: &str, balance: f64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "UPDATE bitcoin_wallets SET balance = ?1, updated_at = ?2 WHERE id = ?3",
            params![balance, &now, wallet_id],
        )?;

        Ok(())
    }

    // EVM Wallet Methods
    pub fn add_evm_wallet(
        &self,
        label: String,
        wallet_type: String,
        address: String,
    ) -> SqliteResult<WalletInfo> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO evm_wallets (id, label, wallet_type, address, balance, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![&id, &label, &wallet_type, &address, 0.0, &now, &now],
        )?;

        Ok(WalletInfo {
            id,
            label,
            wallet_type,
            address,
            balance: 0.0,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn add_evm_wallet_secret(
        &self,
        wallet_id: String,
        secret_data: String,
        secret_type: String,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO evm_wallet_secrets (wallet_id, secret_data, secret_type)
             VALUES (?1, ?2, ?3)",
            params![&wallet_id, &secret_data, &secret_type],
        )?;

        Ok(())
    }

    pub fn get_evm_wallet_secret(&self, wallet_id: &str) -> SqliteResult<Option<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT secret_data, secret_type FROM evm_wallet_secrets WHERE wallet_id = ?1",
        )?;

        let result = stmt.query_row(params![wallet_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        });

        match result {
            Ok(secret) => Ok(Some(secret)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn delete_evm_wallet(&self, wallet_id: &str) -> SqliteResult<bool> {
        let conn = self.conn.lock().unwrap();

        // Delete asset balances first (to satisfy foreign key constraints)
        conn.execute(
            "DELETE FROM evm_asset_balances WHERE wallet_id = ?1",
            params![wallet_id],
        )?;

        // Delete wallet secret
        conn.execute(
            "DELETE FROM evm_wallet_secrets WHERE wallet_id = ?1",
            params![wallet_id],
        )?;

        // Delete wallet
        let rows_affected =
            conn.execute("DELETE FROM evm_wallets WHERE id = ?1", params![wallet_id])?;

        Ok(rows_affected > 0)
    }

    pub fn get_evm_wallets(&self) -> SqliteResult<Vec<WalletInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, label, wallet_type, address, balance, created_at, updated_at
             FROM evm_wallets ORDER BY created_at DESC",
        )?;

        let wallets = stmt.query_map([], |row| {
            Ok(WalletInfo {
                id: row.get(0)?,
                label: row.get(1)?,
                wallet_type: row.get(2)?,
                address: row.get(3)?,
                balance: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for wallet in wallets {
            result.push(wallet?);
        }

        Ok(result)
    }

    pub fn get_evm_wallet(&self, wallet_id: &str) -> SqliteResult<Option<WalletInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, label, wallet_type, address, balance, created_at, updated_at
             FROM evm_wallets WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![wallet_id], |row| {
            Ok(WalletInfo {
                id: row.get(0)?,
                label: row.get(1)?,
                wallet_type: row.get(2)?,
                address: row.get(3)?,
                balance: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        });

        match result {
            Ok(wallet) => Ok(Some(wallet)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // EVM Asset Balance Methods
    pub fn save_evm_asset_balance(
        &self,
        wallet_id: String,
        chain: String,
        chain_id: u64,
        asset_symbol: String,
        asset_name: String,
        asset_decimals: u8,
        contract_address: Option<String>,
        balance: String,
        balance_float: f64,
        usd_price: f64,
        usd_value: f64,
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT OR REPLACE INTO evm_asset_balances 
             (id, wallet_id, chain, chain_id, asset_symbol, asset_name, asset_decimals, contract_address, balance, balance_float, usd_price, usd_value, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                &id, &wallet_id, &chain, chain_id, &asset_symbol, &asset_name,
                asset_decimals, &contract_address, &balance, balance_float, usd_price, usd_value, &now
            ],
        )?;

        Ok(())
    }

    pub fn batch_save_evm_asset_balances(&self, assets: &[AssetBalanceData]) -> SqliteResult<()> {
        if assets.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        let now = Utc::now().to_rfc3339();

        for asset in assets {
            let id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT OR REPLACE INTO evm_asset_balances (
                    id, wallet_id, chain, chain_id, asset_symbol, asset_name,
                    asset_decimals, contract_address, balance, balance_float,
                    usd_price, usd_value, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    &id,
                    &asset.wallet_id,
                    &asset.chain,
                    asset.chain_id,
                    &asset.symbol,
                    &asset.name,
                    asset.decimals,
                    &asset.contract_address,
                    &asset.balance,
                    asset.balance_float,
                    asset.usd_price,
                    asset.usd_value,
                    &now,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_evm_asset_balances(
        &self,
        wallet_id: &str,
    ) -> SqliteResult<
        Vec<(
            String,
            String,
            u64,
            String,
            Option<String>,
            u8,
            String,
            f64,
            f64,
            f64,
        )>,
    > {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT chain, asset_symbol, chain_id, asset_name, contract_address, asset_decimals, balance, balance_float, usd_price, usd_value
             FROM evm_asset_balances 
             WHERE wallet_id = ?1 
             ORDER BY chain, asset_symbol",
        )?;

        let balances = stmt.query_map(params![wallet_id], |row| {
            Ok((
                row.get::<_, String>(0)?,         // chain
                row.get::<_, String>(1)?,         // asset_symbol
                row.get::<_, u64>(2)?,            // chain_id
                row.get::<_, String>(3)?,         // asset_name
                row.get::<_, Option<String>>(4)?, // contract_address
                row.get::<_, u8>(5)?,             // asset_decimals
                row.get::<_, String>(6)?,         // balance
                row.get::<_, f64>(7)?,            // balance_float
                row.get::<_, f64>(8)?,            // usd_price
                row.get::<_, f64>(9)?,            // usd_value
            ))
        })?;

        let mut result = Vec::new();
        for balance in balances {
            result.push(balance?);
        }

        Ok(result)
    }

    pub fn clear_evm_asset_balances(&self, wallet_id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "DELETE FROM evm_asset_balances WHERE wallet_id = ?1",
            params![wallet_id],
        )?;

        Ok(())
    }
}
