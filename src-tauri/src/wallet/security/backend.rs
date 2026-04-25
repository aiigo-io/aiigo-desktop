use super::secret_envelope::{
    decrypt_secret, encrypt_secret, probe_secret_backend, SecretEnvelopeError, StoredSecret,
};
use super::types::{
    SecretBackendStatus, SecretBackendUnavailableKind, SecretBackendUnavailableReason,
    SecurityError,
};
use std::sync::{Arc, Mutex};

pub trait SecretBackendAdapter {
    fn probe(&self) -> Result<(), SecretEnvelopeError>;
    fn encrypt(&self, plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError>;
    fn decrypt(
        &self,
        secret_data: &str,
        secret_format: &str,
    ) -> Result<String, SecretEnvelopeError>;
}

struct DefaultSecretBackendAdapter;

impl SecretBackendAdapter for DefaultSecretBackendAdapter {
    fn probe(&self) -> Result<(), SecretEnvelopeError> {
        probe_secret_backend()
    }

    fn encrypt(&self, plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
        encrypt_secret(plaintext)
    }

    fn decrypt(
        &self,
        secret_data: &str,
        secret_format: &str,
    ) -> Result<String, SecretEnvelopeError> {
        decrypt_secret(secret_data, secret_format)
    }
}

pub struct SecretBackend {
    status: Mutex<SecretBackendStatus>,
    adapter: Arc<dyn SecretBackendAdapter + Send + Sync>,
}

impl SecretBackend {
    pub fn new() -> Self {
        Self {
            status: Mutex::new(SecretBackendStatus::Unknown),
            adapter: Arc::new(DefaultSecretBackendAdapter),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_adapter(adapter: Arc<dyn SecretBackendAdapter + Send + Sync>) -> Self {
        Self {
            status: Mutex::new(SecretBackendStatus::Unknown),
            adapter,
        }
    }

    pub fn current_status(&self) -> SecretBackendStatus {
        self.status.lock().map(|status| status.clone()).unwrap_or(
            SecretBackendStatus::Unavailable {
                reason: SecretBackendUnavailableReason {
                    kind: SecretBackendUnavailableKind::UnknownBackendError,
                    message: "secret backend status lock poisoned".to_string(),
                },
            },
        )
    }

    pub fn refresh_status(&self) -> SecretBackendStatus {
        let next = match self.adapter.probe() {
            Ok(()) => SecretBackendStatus::Ready,
            Err(error) => SecretBackendStatus::Unavailable {
                reason: reason_from_error(&error),
            },
        };

        if let Ok(mut status) = self.status.lock() {
            *status = next.clone();
        }

        next
    }

    pub fn ensure_ready_for_command(&self) -> Result<(), SecurityError> {
        if matches!(self.current_status(), SecretBackendStatus::Ready) {
            return Ok(());
        }

        if matches!(self.refresh_status(), SecretBackendStatus::Ready) {
            Ok(())
        } else {
            Err(SecurityError::SecretBackendUnavailable)
        }
    }

    pub fn prepare_encrypted_secret(&self, plaintext: &str) -> Result<StoredSecret, SecurityError> {
        match self.adapter.encrypt(plaintext) {
            Ok(secret) => {
                self.set_status(SecretBackendStatus::Ready);
                Ok(secret)
            }
            Err(error) => {
                self.set_status(SecretBackendStatus::Unavailable {
                    reason: reason_from_error(&error),
                });
                Err(SecurityError::SecretBackendUnavailable)
            }
        }
    }

    pub fn decrypt_for_command(
        &self,
        secret_data: &str,
        secret_format: &str,
    ) -> Result<String, SecurityError> {
        self.ensure_ready_for_command()?;

        match self.adapter.decrypt(secret_data, secret_format) {
            Ok(secret) => {
                self.set_status(SecretBackendStatus::Ready);
                Ok(secret)
            }
            Err(error) => {
                self.set_status(SecretBackendStatus::Unavailable {
                    reason: reason_from_error(&error),
                });
                Err(SecurityError::SecretBackendUnavailable)
            }
        }
    }

    fn set_status(&self, next: SecretBackendStatus) {
        if let Ok(mut status) = self.status.lock() {
            *status = next;
        }
    }
}

fn reason_from_error(error: &SecretEnvelopeError) -> SecretBackendUnavailableReason {
    let (kind, message) = match error {
        SecretEnvelopeError::Keyring(message) => {
            let lower = message.to_ascii_lowercase();
            if lower.contains("secret service") {
                (
                    SecretBackendUnavailableKind::SecretServiceUnreachable,
                    message.clone(),
                )
            } else if lower.contains("denied") || lower.contains("permission") {
                (SecretBackendUnavailableKind::AccessDenied, message.clone())
            } else {
                (
                    SecretBackendUnavailableKind::KeyringUnavailable,
                    message.clone(),
                )
            }
        }
        SecretEnvelopeError::InvalidMasterKeyLength(length) => (
            SecretBackendUnavailableKind::KeyDecodeFailed,
            format!("invalid master key length {length}"),
        ),
        SecretEnvelopeError::Base64(error) => (
            SecretBackendUnavailableKind::KeyDecodeFailed,
            error.to_string(),
        ),
        other => (
            SecretBackendUnavailableKind::UnknownBackendError,
            other.to_string(),
        ),
    };

    SecretBackendUnavailableReason { kind, message }
}

#[cfg(test)]
mod tests {
    use super::{SecretBackend, SecretBackendAdapter};
    use crate::wallet::security::secret_envelope::{SecretEnvelopeError, StoredSecret};
    use crate::wallet::security::types::{
        SecretBackendStatus, SecretBackendUnavailableKind, SecurityError,
    };
    use std::sync::{Arc, Mutex};

    struct StubAdapter {
        probe_results: Mutex<Vec<Result<(), SecretEnvelopeError>>>,
        encrypt_should_fail: bool,
        decrypt_should_fail: bool,
    }

    impl SecretBackendAdapter for StubAdapter {
        fn probe(&self) -> Result<(), SecretEnvelopeError> {
            self.probe_results.lock().unwrap().remove(0)
        }

        fn encrypt(&self, _plaintext: &str) -> Result<StoredSecret, SecretEnvelopeError> {
            if self.encrypt_should_fail {
                Err(SecretEnvelopeError::Keyring("down".to_string()))
            } else {
                Ok(stored_secret())
            }
        }

        fn decrypt(
            &self,
            _secret_data: &str,
            _secret_format: &str,
        ) -> Result<String, SecretEnvelopeError> {
            if self.decrypt_should_fail {
                Err(SecretEnvelopeError::Decrypt)
            } else {
                Ok("seed words".to_string())
            }
        }
    }

    fn stored_secret() -> StoredSecret {
        StoredSecret {
            secret_data: "cipher".to_string(),
            secret_format: "keyring_aes256_gcm_v1".to_string(),
        }
    }

    #[test]
    fn backend_status_starts_unknown() {
        let backend = SecretBackend::new();

        assert_eq!(backend.current_status(), SecretBackendStatus::Unknown);
    }

    #[test]
    fn backend_status_refreshes_after_recovery() {
        let adapter = Arc::new(StubAdapter {
            probe_results: Mutex::new(vec![
                Err(SecretEnvelopeError::Keyring("offline".to_string())),
                Ok(()),
            ]),
            encrypt_should_fail: false,
            decrypt_should_fail: false,
        });
        let backend = SecretBackend::with_adapter(adapter);

        let first = backend.refresh_status();
        assert!(matches!(
            first,
            SecretBackendStatus::Unavailable {
                reason
            } if reason.kind == SecretBackendUnavailableKind::KeyringUnavailable
        ));

        let second = backend.refresh_status();
        assert_eq!(second, SecretBackendStatus::Ready);
    }

    #[test]
    fn decrypt_for_command_uses_latest_backend_status() {
        let adapter = Arc::new(StubAdapter {
            probe_results: Mutex::new(vec![Ok(())]),
            encrypt_should_fail: false,
            decrypt_should_fail: false,
        });
        let backend = SecretBackend::with_adapter(adapter);

        assert_eq!(
            backend.decrypt_for_command("cipher", "keyring_aes256_gcm_v1"),
            Ok("seed words".to_string())
        );
    }

    #[test]
    fn prepare_encrypted_secret_returns_backend_unavailable() {
        let adapter = Arc::new(StubAdapter {
            probe_results: Mutex::new(vec![Ok(())]),
            encrypt_should_fail: true,
            decrypt_should_fail: false,
        });
        let backend = SecretBackend::with_adapter(adapter);

        assert!(matches!(
            backend.prepare_encrypted_secret("seed words"),
            Err(SecurityError::SecretBackendUnavailable)
        ));
    }
}
