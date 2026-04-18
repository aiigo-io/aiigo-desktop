use super::types::{SecurityError, SignerOperation};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type NowFn = Arc<dyn Fn() -> Instant + Send + Sync>;

pub struct SessionManager {
    ttl: Duration,
    unlocked_at: Mutex<Option<Instant>>,
    now: NowFn,
}

impl SessionManager {
    pub fn new(ttl: Duration) -> Self {
        Self::with_clock(ttl, Instant::now)
    }

    fn with_clock<F>(ttl: Duration, now: F) -> Self
    where
        F: Fn() -> Instant + Send + Sync + 'static,
    {
        Self {
            ttl,
            unlocked_at: Mutex::new(None),
            now: Arc::new(now),
        }
    }

    pub fn unlock(&self, token_material: &str) -> Result<(), SecurityError> {
        if token_material.is_empty() {
            return Err(SecurityError::PolicyDenied);
        }

        let mut unlocked_at = self
            .unlocked_at
            .lock()
            .map_err(|_| SecurityError::OperationNotAllowed)?;
        *unlocked_at = Some(self.now());
        Ok(())
    }

    pub fn is_unlocked(&self) -> bool {
        self.refresh_state().unwrap_or(false)
    }

    pub fn lock(&self) {
        if let Ok(mut unlocked_at) = self.unlocked_at.lock() {
            *unlocked_at = None;
        }
    }

    pub fn authorize(&self, op: SignerOperation) -> Result<(), SecurityError> {
        // TODO(phase1): per-op policy branch wired in Task 6 when command
        // sites supply real SignerOperation values. Parameter is accepted
        // now so the signature stays stable across phase.
        let _ = op;

        if self.refresh_state().unwrap_or(false) {
            Ok(())
        } else {
            Err(SecurityError::Locked)
        }
    }

    fn now(&self) -> Instant {
        (self.now.as_ref())()
    }

    fn refresh_state(&self) -> Result<bool, SecurityError> {
        let mut unlocked_at = self
            .unlocked_at
            .lock()
            .map_err(|_| SecurityError::OperationNotAllowed)?;

        let Some(last_unlocked_at) = *unlocked_at else {
            return Ok(false);
        };

        if self.now().saturating_duration_since(last_unlocked_at) < self.ttl {
            return Ok(true);
        }

        *unlocked_at = None;
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::SessionManager;
    use crate::wallet::security::types::{SecurityError, SignerOperation};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn test_session(ttl: Duration) -> (SessionManager, Arc<Mutex<Instant>>) {
        let clock = Arc::new(Mutex::new(Instant::now()));
        let now = Arc::clone(&clock);
        let session = SessionManager::with_clock(ttl, move || *now.lock().unwrap());
        (session, clock)
    }

    #[test]
    fn unlock_with_empty_token_returns_policy_denied() {
        let (session, _) = test_session(Duration::from_secs(30));

        assert_eq!(session.unlock(""), Err(SecurityError::PolicyDenied));
    }

    #[test]
    fn unlock_marks_session_unlocked_and_authorizes_send() {
        let (session, _) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();

        assert!(session.is_unlocked());
        assert_eq!(session.authorize(SignerOperation::Send), Ok(()));
    }

    #[test]
    fn unlock_marks_session_unlocked_and_authorizes_approve() {
        let (session, _) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();

        assert_eq!(session.authorize(SignerOperation::Approve), Ok(()));
    }

    #[test]
    fn unlock_marks_session_unlocked_and_authorizes_export_mnemonic() {
        let (session, _) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();

        assert_eq!(session.authorize(SignerOperation::ExportMnemonic), Ok(()));
    }

    #[test]
    fn unlock_marks_session_unlocked_and_authorizes_export_private_key() {
        let (session, _) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();

        assert_eq!(session.authorize(SignerOperation::ExportPrivateKey), Ok(()));
    }

    #[test]
    fn authorize_returns_locked_after_ttl_expiry() {
        let (session, clock) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();
        *clock.lock().unwrap() += Duration::from_secs(31);

        assert_eq!(session.authorize(SignerOperation::Send), Err(SecurityError::Locked));
    }

    #[test]
    fn lock_after_unlock_revokes_authorization() {
        let (session, _) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();
        session.lock();

        assert_eq!(session.authorize(SignerOperation::Send), Err(SecurityError::Locked));
    }
}
