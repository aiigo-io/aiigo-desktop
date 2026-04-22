use super::types::{SecurityError, SignerOperation};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type NowFn = Arc<dyn Fn() -> Instant + Send + Sync>;

enum SessionState {
    Locked,
    Unlocked(Instant),
    Expired,
}

pub struct SessionManager {
    ttl: Duration,
    state: Mutex<SessionState>,
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
            state: Mutex::new(SessionState::Locked),
            now: Arc::new(now),
        }
    }

    pub fn unlock(&self, token_material: &str) -> Result<(), SecurityError> {
        if token_material.is_empty() {
            return Err(SecurityError::PolicyDenied);
        }

        let mut state = self
            .state
            .lock()
            .map_err(|_| SecurityError::OperationNotAllowed)?;
        *state = SessionState::Unlocked(self.now());
        Ok(())
    }

    pub fn is_unlocked(&self) -> bool {
        self.refresh_state().unwrap_or(false)
    }

    pub fn lock(&self) {
        if let Ok(mut state) = self.state.lock() {
            *state = SessionState::Locked;
        }
    }

    pub fn authorize(&self, op: SignerOperation) -> Result<(), SecurityError> {
        match self.authorization_state()? {
            AuthorizationState::Locked => Err(SecurityError::Locked),
            AuthorizationState::Expired => Err(SecurityError::Expired),
            AuthorizationState::Unlocked => match op {
                SignerOperation::Send | SignerOperation::Approve => Ok(()),
                SignerOperation::ExportMnemonic | SignerOperation::ExportPrivateKey => {
                    Err(SecurityError::PolicyDenied)
                }
            },
        }
    }

    fn now(&self) -> Instant {
        (self.now.as_ref())()
    }

    fn refresh_state(&self) -> Result<bool, SecurityError> {
        Ok(matches!(self.authorization_state()?, AuthorizationState::Unlocked))
    }

    fn authorization_state(&self) -> Result<AuthorizationState, SecurityError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SecurityError::OperationNotAllowed)?;

        match &*state {
            SessionState::Locked => Ok(AuthorizationState::Locked),
            SessionState::Expired => Ok(AuthorizationState::Expired),
            SessionState::Unlocked(last_unlocked_at) => {
                if self.now().saturating_duration_since(*last_unlocked_at) < self.ttl {
                    Ok(AuthorizationState::Unlocked)
                } else {
                    *state = SessionState::Expired;
                    Ok(AuthorizationState::Expired)
                }
            }
        }
    }
}

enum AuthorizationState {
    Locked,
    Expired,
    Unlocked,
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

        assert_eq!(
            session.authorize(SignerOperation::ExportMnemonic),
            Err(SecurityError::PolicyDenied)
        );
    }

    #[test]
    fn unlock_marks_session_unlocked_and_authorizes_export_private_key() {
        let (session, _) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();

        assert_eq!(
            session.authorize(SignerOperation::ExportPrivateKey),
            Err(SecurityError::PolicyDenied)
        );
    }

    #[test]
    fn authorize_returns_expired_after_ttl_expiry() {
        let (session, clock) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();
        *clock.lock().unwrap() += Duration::from_secs(31);

        assert_eq!(session.authorize(SignerOperation::Send), Err(SecurityError::Expired));
    }

    #[test]
    fn expired_state_survives_is_unlocked_polling() {
        let (session, clock) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();
        *clock.lock().unwrap() += Duration::from_secs(31);

        assert!(!session.is_unlocked());
        assert_eq!(session.authorize(SignerOperation::Send), Err(SecurityError::Expired));
    }

    #[test]
    fn lock_after_unlock_revokes_authorization() {
        let (session, _) = test_session(Duration::from_secs(30));

        session.unlock("token").unwrap();
        session.lock();

        assert_eq!(session.authorize(SignerOperation::Send), Err(SecurityError::Locked));
    }
}
