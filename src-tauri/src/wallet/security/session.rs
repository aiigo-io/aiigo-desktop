use super::types::{SecurityError, SignerOperation};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type NowFn = Arc<dyn Fn() -> Instant + Send + Sync>;

#[derive(Clone, Copy)]
struct OperationGrant {
    operation: SignerOperation,
    granted_at: Instant,
}

enum SessionState {
    Locked,
    Unlocked(Instant),
    Expired,
}

pub struct SessionManager {
    ttl: Duration,
    reauth_ttl: Duration,
    state: Mutex<SessionState>,
    operation_grant: Mutex<Option<OperationGrant>>,
    now: NowFn,
}

impl SessionManager {
    pub fn new(ttl: Duration, reauth_ttl: Duration) -> Self {
        Self::with_clock(ttl, reauth_ttl, Instant::now)
    }

    fn with_clock<F>(ttl: Duration, reauth_ttl: Duration, now: F) -> Self
    where
        F: Fn() -> Instant + Send + Sync + 'static,
    {
        Self {
            ttl,
            reauth_ttl,
            state: Mutex::new(SessionState::Locked),
            operation_grant: Mutex::new(None),
            now: Arc::new(now),
        }
    }

    pub fn unlock_verified(&self) -> Result<(), SecurityError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| SecurityError::OperationNotAllowed)?;
        *state = SessionState::Unlocked(self.now());
        self.clear_operation_grant()?;
        Ok(())
    }

    pub fn authorize_verified_operation(
        &self,
        operation: SignerOperation,
    ) -> Result<(), SecurityError> {
        let granted_at = self.now();

        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| SecurityError::OperationNotAllowed)?;
            *state = SessionState::Unlocked(granted_at);
        }

        let mut grant = self
            .operation_grant
            .lock()
            .map_err(|_| SecurityError::OperationNotAllowed)?;
        *grant = Some(OperationGrant {
            operation,
            granted_at,
        });

        Ok(())
    }

    pub fn is_unlocked(&self) -> bool {
        self.refresh_state().unwrap_or(false)
    }

    pub fn lock(&self) {
        if let Ok(mut state) = self.state.lock() {
            *state = SessionState::Locked;
        }

        if let Ok(mut grant) = self.operation_grant.lock() {
            *grant = None;
        }
    }

    pub fn authorize(&self, op: SignerOperation) -> Result<(), SecurityError> {
        match self.authorization_state()? {
            AuthorizationState::Locked => Err(SecurityError::Locked),
            AuthorizationState::Expired => Err(SecurityError::Expired),
            AuthorizationState::Unlocked => self.consume_operation_grant(op),
        }
    }

    fn now(&self) -> Instant {
        (self.now.as_ref())()
    }

    fn refresh_state(&self) -> Result<bool, SecurityError> {
        Ok(matches!(
            self.authorization_state()?,
            AuthorizationState::Unlocked
        ))
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
                    self.clear_operation_grant()?;
                    Ok(AuthorizationState::Expired)
                }
            }
        }
    }

    fn consume_operation_grant(&self, operation: SignerOperation) -> Result<(), SecurityError> {
        let mut grant = self
            .operation_grant
            .lock()
            .map_err(|_| SecurityError::OperationNotAllowed)?;

        match *grant {
            Some(current)
                if current.operation == operation
                    && self.now().saturating_duration_since(current.granted_at)
                        < self.reauth_ttl =>
            {
                if !operation_grant_is_reusable(operation) {
                    *grant = None;
                }
                Ok(())
            }
            _ => {
                *grant = None;
                Err(SecurityError::ReauthRequired)
            }
        }
    }

    fn clear_operation_grant(&self) -> Result<(), SecurityError> {
        let mut grant = self
            .operation_grant
            .lock()
            .map_err(|_| SecurityError::OperationNotAllowed)?;
        *grant = None;
        Ok(())
    }
}

fn operation_grant_is_reusable(operation: SignerOperation) -> bool {
    matches!(operation, SignerOperation::Send | SignerOperation::Approve)
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

    fn test_session(ttl: Duration, reauth_ttl: Duration) -> (SessionManager, Arc<Mutex<Instant>>) {
        let clock = Arc::new(Mutex::new(Instant::now()));
        let now = Arc::clone(&clock);
        let session = SessionManager::with_clock(ttl, reauth_ttl, move || *now.lock().unwrap());
        (session, clock)
    }

    #[test]
    fn unlock_marks_session_unlocked_without_high_risk_grant() {
        let (session, _) = test_session(Duration::from_secs(30), Duration::from_secs(90));

        session.unlock_verified().unwrap();

        assert!(session.is_unlocked());
        assert_eq!(
            session.authorize(SignerOperation::Send),
            Err(SecurityError::ReauthRequired)
        );
    }

    #[test]
    fn verified_send_grant_is_reusable_within_reauth_window() {
        let (session, _) = test_session(Duration::from_secs(30), Duration::from_secs(90));

        session
            .authorize_verified_operation(SignerOperation::Send)
            .unwrap();

        assert_eq!(session.authorize(SignerOperation::Send), Ok(()));
        assert_eq!(session.authorize(SignerOperation::Send), Ok(()));
    }

    #[test]
    fn verified_approve_grant_is_reusable_within_reauth_window() {
        let (session, _) = test_session(Duration::from_secs(30), Duration::from_secs(90));

        session
            .authorize_verified_operation(SignerOperation::Approve)
            .unwrap();

        assert_eq!(session.authorize(SignerOperation::Approve), Ok(()));
        assert_eq!(session.authorize(SignerOperation::Approve), Ok(()));
    }

    #[test]
    fn export_grant_remains_single_use() {
        let (session, _) = test_session(Duration::from_secs(30), Duration::from_secs(90));

        session
            .authorize_verified_operation(SignerOperation::ExportMnemonic)
            .unwrap();

        assert_eq!(session.authorize(SignerOperation::ExportMnemonic), Ok(()));
        assert_eq!(
            session.authorize(SignerOperation::ExportMnemonic),
            Err(SecurityError::ReauthRequired)
        );
    }

    #[test]
    fn wrong_operation_requires_reauth() {
        let (session, _) = test_session(Duration::from_secs(30), Duration::from_secs(90));

        session
            .authorize_verified_operation(SignerOperation::Send)
            .unwrap();

        assert_eq!(
            session.authorize(SignerOperation::ExportPrivateKey),
            Err(SecurityError::ReauthRequired)
        );
    }

    #[test]
    fn authorize_returns_expired_after_ttl_expiry() {
        let (session, clock) = test_session(Duration::from_secs(30), Duration::from_secs(90));

        session
            .authorize_verified_operation(SignerOperation::Send)
            .unwrap();
        *clock.lock().unwrap() += Duration::from_secs(31);

        assert_eq!(
            session.authorize(SignerOperation::Send),
            Err(SecurityError::Expired)
        );
    }

    #[test]
    fn expired_state_survives_is_unlocked_polling() {
        let (session, clock) = test_session(Duration::from_secs(30), Duration::from_secs(90));

        session
            .authorize_verified_operation(SignerOperation::Send)
            .unwrap();
        *clock.lock().unwrap() += Duration::from_secs(31);

        assert!(!session.is_unlocked());
        assert_eq!(
            session.authorize(SignerOperation::Send),
            Err(SecurityError::Expired)
        );
    }

    #[test]
    fn lock_after_unlock_revokes_authorization() {
        let (session, _) = test_session(Duration::from_secs(30), Duration::from_secs(90));

        session
            .authorize_verified_operation(SignerOperation::Send)
            .unwrap();
        session.lock();

        assert_eq!(
            session.authorize(SignerOperation::Send),
            Err(SecurityError::Locked)
        );
    }

    #[test]
    fn reauth_grant_expires_before_use() {
        let (session, clock) = test_session(Duration::from_secs(300), Duration::from_secs(10));

        session
            .authorize_verified_operation(SignerOperation::Send)
            .unwrap();
        *clock.lock().unwrap() += Duration::from_secs(11);

        assert_eq!(
            session.authorize(SignerOperation::Send),
            Err(SecurityError::ReauthRequired)
        );
    }
}
