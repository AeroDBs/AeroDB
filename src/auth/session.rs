//! # Session Management
//!
//! Session and refresh token management.
//! Sessions are stored in the `_sessions` collection.
//!
//! ## Invariants
//! - AUTH-SS1: Refresh tokens are single-use
//! - AUTH-SS2: Sessions expire at stated time
//! - AUTH-SS3: Logout invalidates immediately

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::crypto::{constant_time_str_eq, generate_token, hash_token};
use super::errors::{AuthError, AuthResult};

/// Session model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: Uuid,

    /// User this session belongs to
    pub user_id: Uuid,

    /// Hashed refresh token (raw token given to client)
    #[serde(skip_serializing)]
    pub refresh_token_hash: String,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// When the session expires
    pub expires_at: DateTime<Utc>,

    /// Whether the session has been revoked
    pub revoked: bool,

    /// User agent from the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    /// IP address from the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
}

/// Token pair returned to user on login/refresh
#[derive(Debug, Clone, Serialize)]
pub struct TokenPair {
    /// JWT access token (short-lived)
    pub access_token: String,

    /// Refresh token (long-lived, single-use)
    pub refresh_token: String,

    /// Access token expiration timestamp
    pub expires_at: DateTime<Utc>,
}

/// Session manager configuration
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Refresh token lifetime
    pub refresh_token_ttl: Duration,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            refresh_token_ttl: Duration::days(30),
        }
    }
}

/// Session manager handles session creation and validation
pub struct SessionManager<R: SessionRepository> {
    config: SessionConfig,
    repository: R,
}

impl<R: SessionRepository> SessionManager<R> {
    pub fn new(config: SessionConfig, repository: R) -> Self {
        Self { config, repository }
    }

    /// Create a new session for a user
    ///
    /// Returns the raw refresh token (not hashed) to give to the client.
    pub fn create_session(
        &self,
        user_id: Uuid,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> AuthResult<(Session, String)> {
        let refresh_token = generate_token();
        let refresh_token_hash = hash_token(&refresh_token);

        let now = Utc::now();
        let session = Session {
            id: Uuid::new_v4(),
            user_id,
            refresh_token_hash,
            created_at: now,
            expires_at: now + self.config.refresh_token_ttl,
            revoked: false,
            user_agent,
            ip_address,
        };

        self.repository.create(&session)?;

        Ok((session, refresh_token))
    }

    /// Refresh a session using the refresh token
    ///
    /// # Invariant
    /// AUTH-SS1: Refresh tokens are single-use (old session revoked)
    pub fn refresh_session(&self, refresh_token: &str) -> AuthResult<(Session, String)> {
        let token_hash = hash_token(refresh_token);

        // Find session by refresh token hash
        let old_session = self
            .repository
            .find_by_refresh_token_hash(&token_hash)?
            .ok_or(AuthError::InvalidRefreshToken)?;

        // Check if revoked
        if old_session.revoked {
            return Err(AuthError::SessionRevoked);
        }

        // Check if expired
        if old_session.expires_at < Utc::now() {
            return Err(AuthError::SessionInvalid);
        }

        // Revoke old session (single-use token)
        self.repository.revoke(old_session.id)?;

        // Create new session
        self.create_session(
            old_session.user_id,
            old_session.user_agent,
            old_session.ip_address,
        )
    }

    /// Revoke a session (logout)
    ///
    /// # Invariant
    /// AUTH-SS3: Logout invalidates immediately
    pub fn revoke_session(&self, session_id: Uuid) -> AuthResult<()> {
        self.repository.revoke(session_id)
    }

    /// Revoke all sessions for a user
    pub fn revoke_all_user_sessions(&self, user_id: Uuid) -> AuthResult<()> {
        self.repository.revoke_all_for_user(user_id)
    }

    /// Validate a refresh token and return the associated session
    pub fn validate_refresh_token(&self, refresh_token: &str) -> AuthResult<Session> {
        let token_hash = hash_token(refresh_token);

        let session = self
            .repository
            .find_by_refresh_token_hash(&token_hash)?
            .ok_or(AuthError::InvalidRefreshToken)?;

        if session.revoked {
            return Err(AuthError::SessionRevoked);
        }

        if session.expires_at < Utc::now() {
            return Err(AuthError::SessionInvalid);
        }

        Ok(session)
    }

    /// Get all active sessions for a user
    pub fn get_user_sessions(&self, user_id: Uuid) -> AuthResult<Vec<Session>> {
        self.repository.find_all_for_user(user_id)
    }
}

/// Session repository trait
pub trait SessionRepository: Send + Sync {
    /// Create a new session
    fn create(&self, session: &Session) -> AuthResult<()>;

    /// Find session by ID
    fn find_by_id(&self, id: Uuid) -> AuthResult<Option<Session>>;

    /// Find session by refresh token hash
    fn find_by_refresh_token_hash(&self, hash: &str) -> AuthResult<Option<Session>>;

    /// Find all sessions for a user
    fn find_all_for_user(&self, user_id: Uuid) -> AuthResult<Vec<Session>>;

    /// Revoke a session
    fn revoke(&self, id: Uuid) -> AuthResult<()>;

    /// Revoke all sessions for a user
    fn revoke_all_for_user(&self, user_id: Uuid) -> AuthResult<()>;

    /// Delete expired sessions (cleanup)
    fn delete_expired(&self) -> AuthResult<usize>;
}

/// In-memory session repository for testing
#[derive(Debug, Default)]
pub struct InMemorySessionRepository {
    sessions: std::sync::RwLock<Vec<Session>>,
}

impl InMemorySessionRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SessionRepository for InMemorySessionRepository {
    fn create(&self, session: &Session) -> AuthResult<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;
        sessions.push(session.clone());
        Ok(())
    }

    fn find_by_id(&self, id: Uuid) -> AuthResult<Option<Session>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;
        Ok(sessions.iter().find(|s| s.id == id).cloned())
    }

    fn find_by_refresh_token_hash(&self, hash: &str) -> AuthResult<Option<Session>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;
        Ok(sessions
            .iter()
            .find(|s| constant_time_str_eq(&s.refresh_token_hash, hash))
            .cloned())
    }

    fn find_all_for_user(&self, user_id: Uuid) -> AuthResult<Vec<Session>> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;
        Ok(sessions
            .iter()
            .filter(|s| s.user_id == user_id && !s.revoked)
            .cloned()
            .collect())
    }

    fn revoke(&self, id: Uuid) -> AuthResult<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;

        if let Some(session) = sessions.iter_mut().find(|s| s.id == id) {
            session.revoked = true;
            Ok(())
        } else {
            Err(AuthError::SessionInvalid)
        }
    }

    fn revoke_all_for_user(&self, user_id: Uuid) -> AuthResult<()> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;

        for session in sessions.iter_mut().filter(|s| s.user_id == user_id) {
            session.revoked = true;
        }

        Ok(())
    }

    fn delete_expired(&self) -> AuthResult<usize> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;

        let now = Utc::now();
        let len_before = sessions.len();
        sessions.retain(|s| s.expires_at > now);
        Ok(len_before - sessions.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_manager() -> SessionManager<InMemorySessionRepository> {
        SessionManager::new(SessionConfig::default(), InMemorySessionRepository::new())
    }

    #[test]
    fn test_session_creation() {
        let manager = create_manager();
        let user_id = Uuid::new_v4();

        let (session, refresh_token) = manager
            .create_session(
                user_id,
                Some("Test Agent".to_string()),
                Some("127.0.0.1".to_string()),
            )
            .unwrap();

        assert_eq!(session.user_id, user_id);
        assert!(!session.revoked);
        assert!(!refresh_token.is_empty());
    }

    #[test]
    fn test_refresh_token_validation() {
        let manager = create_manager();
        let user_id = Uuid::new_v4();

        let (_, refresh_token) = manager.create_session(user_id, None, None).unwrap();

        // Valid token should work
        let session = manager.validate_refresh_token(&refresh_token).unwrap();
        assert_eq!(session.user_id, user_id);

        // Invalid token should fail
        let result = manager.validate_refresh_token("invalid_token");
        assert!(matches!(result, Err(AuthError::InvalidRefreshToken)));
    }

    #[test]
    fn test_session_refresh_single_use() {
        let manager = create_manager();
        let user_id = Uuid::new_v4();

        let (_, refresh_token) = manager.create_session(user_id, None, None).unwrap();

        // First refresh should work
        let (new_session, new_token) = manager.refresh_session(&refresh_token).unwrap();
        assert_eq!(new_session.user_id, user_id);

        // Using old token again should fail (single-use)
        let result = manager.refresh_session(&refresh_token);
        assert!(matches!(result, Err(AuthError::SessionRevoked)));

        // New token should work
        let _ = manager.refresh_session(&new_token).unwrap();
    }

    #[test]
    fn test_session_revocation() {
        let manager = create_manager();
        let user_id = Uuid::new_v4();

        let (session, refresh_token) = manager.create_session(user_id, None, None).unwrap();

        // Revoke session
        manager.revoke_session(session.id).unwrap();

        // Token should no longer work
        let result = manager.validate_refresh_token(&refresh_token);
        assert!(matches!(result, Err(AuthError::SessionRevoked)));
    }

    #[test]
    fn test_revoke_all_user_sessions() {
        let manager = create_manager();
        let user_id = Uuid::new_v4();

        // Create multiple sessions
        let (_, token1) = manager.create_session(user_id, None, None).unwrap();
        let (_, token2) = manager.create_session(user_id, None, None).unwrap();

        // Both should be valid
        assert!(manager.validate_refresh_token(&token1).is_ok());
        assert!(manager.validate_refresh_token(&token2).is_ok());

        // Revoke all
        manager.revoke_all_user_sessions(user_id).unwrap();

        // Both should be invalid
        assert!(matches!(
            manager.validate_refresh_token(&token1),
            Err(AuthError::SessionRevoked)
        ));
        assert!(matches!(
            manager.validate_refresh_token(&token2),
            Err(AuthError::SessionRevoked)
        ));
    }
}
