use super::backend::SecretBackend;
#[cfg(test)]
use super::backend::SecretBackendAdapter;
#[cfg(test)]
use super::secret_envelope::{decrypt_secret, SecretEnvelopeError, StoredSecret};
use super::types::SecurityError;
use crate::db::Database;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

pub trait Keystore {
    fn load_mnemonic(&self, address: &str) -> Result<Option<String>, SecurityError>;
    fn load_private_key(&self, address: &str) -> Result<Option<String>, SecurityError>;
}

pub struct SqliteKeystore {
    backend: Backend,
    secret_backend: Arc<SecretBackend>,
}

enum Backend {
    ExistingDb(&'static Mutex<Database>),
    Connection(Mutex<Connection>),
}

impl SqliteKeystore {
    pub fn new(db: &'static Mutex<Database>, secret_backend: Arc<SecretBackend>) -> Self {
        Self {
            backend: Backend::ExistingDb(db),
            secret_backend,
        }
    }

    #[cfg(test)]
    fn from_connection(conn: Connection) -> Self {
        Self {
            backend: Backend::Connection(Mutex::new(conn)),
            secret_backend: Arc::new(SecretBackend::with_adapter(Arc::new(
                TestSecretBackendAdapter,
            ))),
        }
    }

    fn load_secret(
        &self,
        address: &str,
    ) -> Result<Option<(String, String, String)>, SecurityError> {
        match &self.backend {
            Backend::ExistingDb(db) => {
                let db = db.lock().map_err(|_| SecurityError::OperationNotAllowed)?;
                load_secret_from_database(&db, address)
            }
            Backend::Connection(conn) => {
                let conn = conn
                    .lock()
                    .map_err(|_| SecurityError::OperationNotAllowed)?;
                load_secret_from_connection(&conn, address)
            }
        }
    }
}

impl Keystore for SqliteKeystore {
    fn load_mnemonic(&self, address: &str) -> Result<Option<String>, SecurityError> {
        let secret = self.load_secret(address)?;
        Ok(secret
            .and_then(|(secret_data, secret_type, secret_format)| {
                (secret_type == "mnemonic").then_some((secret_data, secret_format))
            })
            .map(|(secret_data, secret_format)| {
                self.secret_backend
                    .decrypt_for_command(&secret_data, &secret_format)
            })
            .transpose()?)
    }

    fn load_private_key(&self, address: &str) -> Result<Option<String>, SecurityError> {
        let secret = self.load_secret(address)?;
        Ok(secret
            .and_then(|(secret_data, secret_type, secret_format)| {
                (secret_type == "private-key").then_some((secret_data, secret_format))
            })
            .map(|(secret_data, secret_format)| {
                self.secret_backend
                    .decrypt_for_command(&secret_data, &secret_format)
            })
            .transpose()?)
    }
}

fn load_secret_from_database(
    db: &Database,
    address: &str,
) -> Result<Option<(String, String, String)>, SecurityError> {
    if let Some(wallet_id) = find_wallet_id(
        db.get_bitcoin_wallets()
            .map_err(|_| SecurityError::OperationNotAllowed)?,
        address,
    ) {
        return db
            .load_bitcoin_wallet_secret(&wallet_id)
            .map_err(|_| SecurityError::OperationNotAllowed);
    }

    if let Some(wallet_id) = find_wallet_id(
        db.get_evm_wallets()
            .map_err(|_| SecurityError::OperationNotAllowed)?,
        address,
    ) {
        return db
            .load_evm_wallet_secret(&wallet_id)
            .map_err(|_| SecurityError::OperationNotAllowed);
    }

    Err(SecurityError::UnknownWallet)
}

fn find_wallet_id(wallets: Vec<crate::wallet::types::WalletInfo>, address: &str) -> Option<String> {
    wallets
        .into_iter()
        .find(|wallet| wallet.address == address)
        .map(|wallet| wallet.id)
}

fn load_secret_from_connection(
    conn: &Connection,
    address: &str,
) -> Result<Option<(String, String, String)>, SecurityError> {
    if let Some(secret) = query_secret(conn, "bitcoin_wallets", "bitcoin_wallet_secrets", address)
        .map_err(|_| SecurityError::OperationNotAllowed)?
    {
        return Ok(secret);
    }

    if let Some(secret) = query_secret(conn, "evm_wallets", "evm_wallet_secrets", address)
        .map_err(|_| SecurityError::OperationNotAllowed)?
    {
        return Ok(secret);
    }

    Err(SecurityError::UnknownWallet)
}

fn query_secret(
    conn: &Connection,
    wallet_table: &str,
    secret_table: &str,
    address: &str,
) -> rusqlite::Result<Option<Option<(String, String, String)>>> {
    let sql = format!(
        "SELECT s.secret_data, s.secret_type, COALESCE(s.secret_format, 'plaintext_v0')
         FROM {wallet_table} w
         LEFT JOIN {secret_table} s ON s.wallet_id = w.id
         WHERE w.address = ?1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let row = stmt.query_row(params![address], |row| {
        let secret_data: Option<String> = row.get(0)?;
        let secret_type: Option<String> = row.get(1)?;
        let secret_format: Option<String> = row.get(2)?;
        Ok(match (secret_data, secret_type, secret_format) {
            (Some(secret_data), Some(secret_type), Some(secret_format)) => {
                Some((secret_data, secret_type, secret_format))
            }
            _ => None,
        })
    });

    match row {
        Ok(secret) => Ok(Some(secret)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
struct TestSecretBackendAdapter;

#[cfg(test)]
impl SecretBackendAdapter for TestSecretBackendAdapter {
    fn probe(&self) -> Result<(), SecretEnvelopeError> {
        Ok(())
    }

    fn initialize_empty_store(&self) -> Result<(), SecretEnvelopeError> {
        Ok(())
    }

    fn encrypt(&self, plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
        Ok(StoredSecret {
            secret_data: plaintext.to_string(),
            secret_format: "plaintext_v0".to_string(),
        })
    }

    fn decrypt(
        &self,
        secret_data: &str,
        secret_format: &str,
    ) -> Result<String, SecretEnvelopeError> {
        decrypt_secret(secret_data, secret_format)
    }
}

#[cfg(test)]
mod tests {
    use super::{Keystore, SqliteKeystore};
    use crate::wallet::security::types::SecurityError;
    use rusqlite::{params, Connection};

    fn setup_wallet_schema(conn: &Connection) {
        conn.execute_batch(
            "CREATE TABLE bitcoin_wallets (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                wallet_type TEXT NOT NULL,
                address TEXT NOT NULL UNIQUE,
                balance REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE bitcoin_wallet_secrets (
                wallet_id TEXT PRIMARY KEY,
                secret_data TEXT NOT NULL,
                secret_type TEXT NOT NULL,
                secret_format TEXT NOT NULL DEFAULT 'plaintext_v0'
            );
            CREATE TABLE evm_wallets (
                id TEXT PRIMARY KEY,
                label TEXT NOT NULL,
                wallet_type TEXT NOT NULL,
                address TEXT NOT NULL UNIQUE,
                balance REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE evm_wallet_secrets (
                wallet_id TEXT PRIMARY KEY,
                secret_data TEXT NOT NULL,
                secret_type TEXT NOT NULL,
                secret_format TEXT NOT NULL DEFAULT 'plaintext_v0'
            );",
        )
        .unwrap();
    }

    #[test]
    fn load_mnemonic_returns_unknown_wallet_for_missing_address() {
        let conn = Connection::open_in_memory().unwrap();
        setup_wallet_schema(&conn);

        let keystore = SqliteKeystore::from_connection(conn);

        assert_eq!(
            keystore.load_mnemonic("missing-address"),
            Err(SecurityError::UnknownWallet)
        );
    }

    #[test]
    fn load_mnemonic_returns_secret_for_matching_wallet_address() {
        let conn = Connection::open_in_memory().unwrap();
        setup_wallet_schema(&conn);
        conn.execute(
            "INSERT INTO bitcoin_wallets (id, label, wallet_type, address, balance, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "btc-wallet-1",
                "Bitcoin Wallet",
                "mnemonic",
                "bc1testaddress",
                0.0_f64,
                "2026-04-18T00:00:00Z",
                "2026-04-18T00:00:00Z"
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO bitcoin_wallet_secrets (wallet_id, secret_data, secret_type, secret_format)
             VALUES (?1, ?2, ?3, ?4)",
            params!["btc-wallet-1", "seed words", "mnemonic", "plaintext_v0"],
        )
        .unwrap();

        let keystore = SqliteKeystore::from_connection(conn);

        assert_eq!(
            keystore.load_mnemonic("bc1testaddress"),
            Ok(Some("seed words".to_string()))
        );
    }

    #[test]
    fn load_private_key_returns_secret_for_matching_wallet_address() {
        let conn = Connection::open_in_memory().unwrap();
        setup_wallet_schema(&conn);
        conn.execute(
            "INSERT INTO evm_wallets (id, label, wallet_type, address, balance, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                "evm-wallet-1",
                "EVM Wallet",
                "private-key",
                "0x1234",
                0.0_f64,
                "2026-04-18T00:00:00Z",
                "2026-04-18T00:00:00Z"
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO evm_wallet_secrets (wallet_id, secret_data, secret_type, secret_format)
             VALUES (?1, ?2, ?3, ?4)",
            params!["evm-wallet-1", "0xdeadbeef", "private-key", "plaintext_v0"],
        )
        .unwrap();

        let keystore = SqliteKeystore::from_connection(conn);

        assert_eq!(
            keystore.load_private_key("0x1234"),
            Ok(Some("0xdeadbeef".to_string()))
        );
    }
}
