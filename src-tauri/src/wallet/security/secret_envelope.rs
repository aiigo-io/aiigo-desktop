use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SECRET_FORMAT_PLAINTEXT_V0: &str = "plaintext_v0";
pub const SECRET_FORMAT_KEYRING_AES256_GCM_V1: &str = "keyring_aes256_gcm_v1";

const KEYRING_SERVICE: &str = "aiigo-desktop";
const KEYRING_ACCOUNT: &str = "wallet-secret-master-key";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretEnvelope {
    version: u8,
    nonce: String,
    ciphertext: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSecret {
    pub secret_data: String,
    pub secret_format: String,
}

#[derive(Debug, Error)]
pub enum SecretEnvelopeError {
    #[error("keyring error: {0}")]
    Keyring(String),
    #[error("invalid master key length: {0}")]
    InvalidMasterKeyLength(usize),
    #[error("invalid envelope: {0}")]
    InvalidEnvelope(String),
    #[error("unsupported secret format: {0}")]
    UnsupportedFormat(String),
    #[error("base64 decode failed: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("json encode failed: {0}")]
    JsonEncode(#[from] serde_json::Error),
    #[error("secret encryption failed")]
    Encrypt,
    #[error("secret decryption failed")]
    Decrypt,
}

pub(crate) fn encrypt_secret(plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
    let master_key = load_or_create_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&master_key).map_err(|_| SecretEnvelopeError::Encrypt)?;
    let nonce_bytes: [u8; 12] = rand::random();
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
        .map_err(|_| SecretEnvelopeError::Encrypt)?;

    let envelope = SecretEnvelope {
        version: 1,
        nonce: STANDARD.encode(nonce_bytes),
        ciphertext: STANDARD.encode(ciphertext),
    };

    Ok(StoredSecret {
        secret_data: serde_json::to_string(&envelope)?,
        secret_format: SECRET_FORMAT_KEYRING_AES256_GCM_V1.to_string(),
    })
}

pub(crate) fn decrypt_secret(secret_data: &str, secret_format: &str) -> Result<String, SecretEnvelopeError> {
    match normalize_secret_format(secret_format) {
        SECRET_FORMAT_PLAINTEXT_V0 => Ok(secret_data.to_string()),
        SECRET_FORMAT_KEYRING_AES256_GCM_V1 => decrypt_secret_envelope(secret_data),
        other => Err(SecretEnvelopeError::UnsupportedFormat(other.to_string())),
    }
}

fn decrypt_secret_envelope(secret_data: &str) -> Result<String, SecretEnvelopeError> {
    let envelope: SecretEnvelope = serde_json::from_str(secret_data)?;
    if envelope.version != 1 {
        return Err(SecretEnvelopeError::InvalidEnvelope(format!(
            "unsupported envelope version {}",
            envelope.version
        )));
    }

    let nonce = STANDARD.decode(envelope.nonce)?;
    if nonce.len() != 12 {
        return Err(SecretEnvelopeError::InvalidEnvelope(format!(
            "invalid nonce length {}",
            nonce.len()
        )));
    }

    let ciphertext = STANDARD.decode(envelope.ciphertext)?;
    let master_key = load_or_create_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&master_key).map_err(|_| SecretEnvelopeError::Decrypt)?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| SecretEnvelopeError::Decrypt)?;

    String::from_utf8(plaintext)
        .map_err(|error| SecretEnvelopeError::InvalidEnvelope(error.to_string()))
}

fn normalize_secret_format(secret_format: &str) -> &str {
    if secret_format.trim().is_empty() {
        SECRET_FORMAT_PLAINTEXT_V0
    } else {
        secret_format
    }
}

pub(crate) fn probe_secret_backend() -> Result<(), SecretEnvelopeError> {
    let _ = load_or_create_master_key()?;
    Ok(())
}

#[cfg(not(test))]
fn load_or_create_master_key() -> Result<[u8; 32], SecretEnvelopeError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|error| SecretEnvelopeError::Keyring(error.to_string()))?;

    match entry.get_password() {
        Ok(encoded_key) => decode_master_key(&encoded_key),
        Err(keyring::Error::NoEntry) => {
            let master_key: [u8; 32] = rand::random();
            let encoded_key = STANDARD.encode(master_key);
            entry
                .set_password(&encoded_key)
                .map_err(|error| SecretEnvelopeError::Keyring(error.to_string()))?;
            Ok(master_key)
        }
        Err(error) => Err(SecretEnvelopeError::Keyring(error.to_string())),
    }
}

#[cfg(test)]
fn load_or_create_master_key() -> Result<[u8; 32], SecretEnvelopeError> {
    Ok([7_u8; 32])
}

fn decode_master_key(encoded_key: &str) -> Result<[u8; 32], SecretEnvelopeError> {
    let decoded = STANDARD.decode(encoded_key)?;
    if decoded.len() != 32 {
        return Err(SecretEnvelopeError::InvalidMasterKeyLength(decoded.len()));
    }

    let mut key = [0_u8; 32];
    key.copy_from_slice(&decoded);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::{decrypt_secret, encrypt_secret, SECRET_FORMAT_PLAINTEXT_V0};

    #[test]
    fn encrypted_secret_round_trips() {
        let stored = encrypt_secret("seed words").unwrap();

        assert_ne!(stored.secret_data, "seed words");
        assert_eq!(
            decrypt_secret(&stored.secret_data, &stored.secret_format).unwrap(),
            "seed words"
        );
    }

    #[test]
    fn plaintext_format_remains_backward_compatible() {
        assert_eq!(
            decrypt_secret("legacy secret", SECRET_FORMAT_PLAINTEXT_V0).unwrap(),
            "legacy secret"
        );
    }
}