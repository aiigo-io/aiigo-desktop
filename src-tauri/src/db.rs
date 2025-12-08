use rusqlite::{Connection, Result as SqliteResult, params};
use crate::wallet::types::WalletInfo;
use crate::wallet::transaction_types::{BitcoinTransaction, EvmTransaction, TransactionStatus, TransactionType};
use std::sync::Mutex;
use uuid::Uuid;
use chrono::Utc;

pub struct Database {
    conn: Mutex<Connection>,
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

        // Bitcoin transactions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS bitcoin_transactions (
                id TEXT PRIMARY KEY,
                wallet_id TEXT NOT NULL,
                tx_hash TEXT NOT NULL UNIQUE,
                tx_type TEXT NOT NULL,
                from_address TEXT NOT NULL,
                to_address TEXT NOT NULL,
                amount REAL NOT NULL,
                fee REAL NOT NULL,
                status TEXT NOT NULL,
                confirmations INTEGER NOT NULL DEFAULT 0,
                block_height INTEGER,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (wallet_id) REFERENCES bitcoin_wallets(id)
            )",
            [],
        )?;

        // EVM transactions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS evm_transactions (
                id TEXT PRIMARY KEY,
                wallet_id TEXT NOT NULL,
                tx_hash TEXT NOT NULL,
                tx_type TEXT NOT NULL,
                from_address TEXT NOT NULL,
                to_address TEXT NOT NULL,
                amount TEXT NOT NULL,
                amount_float REAL NOT NULL,
                asset_symbol TEXT NOT NULL,
                asset_name TEXT NOT NULL,
                contract_address TEXT,
                chain TEXT NOT NULL,
                chain_id INTEGER NOT NULL,
                gas_used TEXT NOT NULL,
                gas_price TEXT NOT NULL,
                fee REAL NOT NULL,
                status TEXT NOT NULL,
                block_number INTEGER,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (wallet_id) REFERENCES evm_wallets(id),
                UNIQUE(wallet_id, tx_hash, chain)
            )",
            [],
        )?;

        // Dashboard stats table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS dashboard_stats (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                total_balance_usd REAL NOT NULL DEFAULT 0,
                total_balance_btc REAL NOT NULL DEFAULT 0,
                change_24h_amount REAL NOT NULL DEFAULT 0,
                change_24h_percentage REAL NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Portfolio history table - stores daily snapshots
        conn.execute(
            "CREATE TABLE IF NOT EXISTS portfolio_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL UNIQUE,
                total_balance_usd REAL NOT NULL,
                created_at TEXT NOT NULL
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
        let rows_affected = conn.execute(
            "DELETE FROM evm_wallets WHERE id = ?1",
            params![wallet_id],
        )?;
        
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

    pub fn get_evm_asset_balances(&self, wallet_id: &str) -> SqliteResult<Vec<(String, String, u64, String, Option<String>, u8, String, f64, f64, f64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT chain, asset_symbol, chain_id, asset_name, contract_address, asset_decimals, balance, balance_float, usd_price, usd_value
             FROM evm_asset_balances 
             WHERE wallet_id = ?1 
             ORDER BY chain, asset_symbol",
        )?;
        
        let balances = stmt.query_map(params![wallet_id], |row| {
            Ok((
                row.get::<_, String>(0)?,  // chain
                row.get::<_, String>(1)?,  // asset_symbol
                row.get::<_, u64>(2)?,     // chain_id
                row.get::<_, String>(3)?,  // asset_name
                row.get::<_, Option<String>>(4)?,  // contract_address
                row.get::<_, u8>(5)?,      // asset_decimals
                row.get::<_, String>(6)?,  // balance
                row.get::<_, f64>(7)?,     // balance_float
                row.get::<_, f64>(8)?,     // usd_price
                row.get::<_, f64>(9)?,     // usd_value
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

    // Bitcoin Transaction Methods
    pub fn add_bitcoin_transaction(&self, tx: &BitcoinTransaction) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO bitcoin_transactions
             (id, wallet_id, tx_hash, tx_type, from_address, to_address, amount, fee, status, confirmations, block_height, timestamp, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                &tx.id,
                &tx.wallet_id,
                &tx.tx_hash,
                tx.tx_type.as_str(),
                &tx.from_address,
                &tx.to_address,
                tx.amount,
                tx.fee,
                tx.status.as_str(),
                tx.confirmations,
                tx.block_height,
                &tx.timestamp,
                &tx.created_at,
            ],
        )?;

        Ok(())
    }

    pub fn get_bitcoin_transactions(&self, wallet_id: &str) -> SqliteResult<Vec<BitcoinTransaction>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, wallet_id, tx_hash, tx_type, from_address, to_address, amount, fee, status, confirmations, block_height, timestamp, created_at
             FROM bitcoin_transactions
             WHERE wallet_id = ?1
             ORDER BY timestamp DESC",
        )?;

        let transactions = stmt.query_map(params![wallet_id], |row| {
            Ok(BitcoinTransaction {
                id: row.get(0)?,
                wallet_id: row.get(1)?,
                tx_hash: row.get(2)?,
                tx_type: TransactionType::from_str(&row.get::<_, String>(3)?),
                from_address: row.get(4)?,
                to_address: row.get(5)?,
                amount: row.get(6)?,
                fee: row.get(7)?,
                status: TransactionStatus::from_str(&row.get::<_, String>(8)?),
                confirmations: row.get(9)?,
                block_height: row.get(10)?,
                timestamp: row.get(11)?,
                created_at: row.get(12)?,
            })
        })?;

        let mut result = Vec::new();
        for tx in transactions {
            result.push(tx?);
        }

        Ok(result)
    }

    pub fn get_all_bitcoin_transactions(&self) -> SqliteResult<Vec<BitcoinTransaction>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, wallet_id, tx_hash, tx_type, from_address, to_address, amount, fee, status, confirmations, block_height, timestamp, created_at
             FROM bitcoin_transactions
             ORDER BY timestamp DESC",
        )?;

        let transactions = stmt.query_map([], |row| {
            Ok(BitcoinTransaction {
                id: row.get(0)?,
                wallet_id: row.get(1)?,
                tx_hash: row.get(2)?,
                tx_type: TransactionType::from_str(&row.get::<_, String>(3)?),
                from_address: row.get(4)?,
                to_address: row.get(5)?,
                amount: row.get(6)?,
                fee: row.get(7)?,
                status: TransactionStatus::from_str(&row.get::<_, String>(8)?),
                confirmations: row.get(9)?,
                block_height: row.get(10)?,
                timestamp: row.get(11)?,
                created_at: row.get(12)?,
            })
        })?;

        let mut result = Vec::new();
        for tx in transactions {
            result.push(tx?);
        }

        Ok(result)
    }

    // EVM Transaction Methods
    pub fn add_evm_transaction(&self, tx: &EvmTransaction) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO evm_transactions
             (id, wallet_id, tx_hash, tx_type, from_address, to_address, amount, amount_float, asset_symbol, asset_name, contract_address, chain, chain_id, gas_used, gas_price, fee, status, block_number, timestamp, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                &tx.id,
                &tx.wallet_id,
                &tx.tx_hash,
                tx.tx_type.as_str(),
                &tx.from_address,
                &tx.to_address,
                &tx.amount,
                tx.amount_float,
                &tx.asset_symbol,
                &tx.asset_name,
                &tx.contract_address,
                &tx.chain,
                tx.chain_id,
                &tx.gas_used,
                &tx.gas_price,
                tx.fee,
                tx.status.as_str(),
                tx.block_number,
                &tx.timestamp,
                &tx.created_at,
            ],
        )?;

        Ok(())
    }

    pub fn get_evm_transactions(&self, wallet_id: &str) -> SqliteResult<Vec<EvmTransaction>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, wallet_id, tx_hash, tx_type, from_address, to_address, amount, amount_float, asset_symbol, asset_name, contract_address, chain, chain_id, gas_used, gas_price, fee, status, block_number, timestamp, created_at
             FROM evm_transactions
             WHERE wallet_id = ?1
             ORDER BY timestamp DESC",
        )?;

        let transactions = stmt.query_map(params![wallet_id], |row| {
            Ok(EvmTransaction {
                id: row.get(0)?,
                wallet_id: row.get(1)?,
                tx_hash: row.get(2)?,
                tx_type: TransactionType::from_str(&row.get::<_, String>(3)?),
                from_address: row.get(4)?,
                to_address: row.get(5)?,
                amount: row.get(6)?,
                amount_float: row.get(7)?,
                asset_symbol: row.get(8)?,
                asset_name: row.get(9)?,
                contract_address: row.get(10)?,
                chain: row.get(11)?,
                chain_id: row.get(12)?,
                gas_used: row.get(13)?,
                gas_price: row.get(14)?,
                fee: row.get(15)?,
                status: TransactionStatus::from_str(&row.get::<_, String>(16)?),
                block_number: row.get(17)?,
                timestamp: row.get(18)?,
                created_at: row.get(19)?,
            })
        })?;

        let mut result = Vec::new();
        for tx in transactions {
            result.push(tx?);
        }

        Ok(result)
    }

    pub fn get_all_evm_transactions(&self) -> SqliteResult<Vec<EvmTransaction>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, wallet_id, tx_hash, tx_type, from_address, to_address, amount, amount_float, asset_symbol, asset_name, contract_address, chain, chain_id, gas_used, gas_price, fee, status, block_number, timestamp, created_at
             FROM evm_transactions
             ORDER BY timestamp DESC",
        )?;

        let transactions = stmt.query_map([], |row| {
            Ok(EvmTransaction {
                id: row.get(0)?,
                wallet_id: row.get(1)?,
                tx_hash: row.get(2)?,
                tx_type: TransactionType::from_str(&row.get::<_, String>(3)?),
                from_address: row.get(4)?,
                to_address: row.get(5)?,
                amount: row.get(6)?,
                amount_float: row.get(7)?,
                asset_symbol: row.get(8)?,
                asset_name: row.get(9)?,
                contract_address: row.get(10)?,
                chain: row.get(11)?,
                chain_id: row.get(12)?,
                gas_used: row.get(13)?,
                gas_price: row.get(14)?,
                fee: row.get(15)?,
                status: TransactionStatus::from_str(&row.get::<_, String>(16)?),
                block_number: row.get(17)?,
                timestamp: row.get(18)?,
                created_at: row.get(19)?,
            })
        })?;

        let mut result = Vec::new();
        for tx in transactions {
            result.push(tx?);
        }

        Ok(result)
    }

    // Dashboard Methods
    pub fn get_dashboard_stats(&self) -> SqliteResult<Option<(f64, f64, f64, f64, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT total_balance_usd, total_balance_btc, change_24h_amount, change_24h_percentage, updated_at 
             FROM dashboard_stats WHERE id = 1",
        )?;
        
        let result = stmt.query_row([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        });
        
        match result {
            Ok(stats) => Ok(Some(stats)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn update_dashboard_stats(
        &self, 
        total_usd: f64, 
        total_btc: f64, 
        change_amount: f64, 
        change_pct: f64
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        
        conn.execute(
            "INSERT OR REPLACE INTO dashboard_stats (id, total_balance_usd, total_balance_btc, change_24h_amount, change_24h_percentage, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![total_usd, total_btc, change_amount, change_pct, &now],
        )?;
        
        Ok(())
    }

    // Portfolio History Methods
    pub fn save_portfolio_snapshot(&self, total_usd: f64) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let date = now.format("%Y-%m-%d").to_string();
        let created_at = now.to_rfc3339();
        
        conn.execute(
            "INSERT OR REPLACE INTO portfolio_history (date, total_balance_usd, created_at)
             VALUES (?1, ?2, ?3)",
            params![&date, total_usd, &created_at],
        )?;
        
        Ok(())
    }

    pub fn get_portfolio_history(&self, days: i64) -> SqliteResult<Vec<(String, f64)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT date, total_balance_usd 
             FROM portfolio_history 
             ORDER BY date DESC 
             LIMIT ?1",
        )?;
        
        let history = stmt.query_map(params![days], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
            ))
        })?;
        
        let mut result = Vec::new();
        for item in history {
            result.push(item?);
        }
        
        // Reverse to get chronological order (oldest first)
        result.reverse();
        Ok(result)
    }
}
