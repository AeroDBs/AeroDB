//! # Magic Links (Passwordless Authentication)
//!
//! Email-based passwordless authentication via one-time tokens.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::RwLock;
use chrono::{DateTime, Duration, Utc};

use super::errors::{AuthError, AuthResult};
use super::user::{User, UserRepository};
use super::email::{EmailSender, EmailTemplate};
use super::crypto::generate_token;

// ==================
// Magic Link Configuration
// ==================

/// Configuration for magic links
#[derive(Debug, Clone)]
pub struct MagicLinkConfig {
    /// Token expiration time
    pub expiration_minutes: i64,
    /// Base URL for the magic link
    pub base_url: String,
    /// Email subject
    pub email_subject: String,
    /// Maximum attempts per email per hour
    pub rate_limit: u32,
}

impl Default for MagicLinkConfig {
    fn default() -> Self {
        Self {
            expiration_minutes: 15,
            base_url: "http://localhost:3000".to_string(),
            email_subject: "Your login link".to_string(),
            rate_limit: 5,
        }
    }
}

// ==================
// Magic Link Token
// ==================

/// A magic link token entry
#[derive(Debug, Clone)]
struct MagicLinkToken {
    /// Token hash (we store hash, not raw token)
    token_hash: String,
    /// User ID (None if user doesn't exist yet)
    user_id: Option<Uuid>,
    /// Email address
    email: String,
    /// Redirect URL after login
    redirect_to: Option<String>,
    /// Expiration time
    expires_at: DateTime<Utc>,
    /// Whether this is for signup (new user)
    is_signup: bool,
}

// ==================
// Rate Limiting
// ==================

/// Rate limit entry
#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: DateTime<Utc>,
}

// ==================
// Magic Link Service
// ==================

/// Magic link authentication service
pub struct MagicLinkService<U: UserRepository> {
    config: MagicLinkConfig,
    user_repo: std::sync::Arc<U>,
    email_sender: Option<std::sync::Arc<dyn EmailSender>>,
    tokens: RwLock<HashMap<String, MagicLinkToken>>,
    rate_limits: RwLock<HashMap<String, RateLimitEntry>>,
}

impl<U: UserRepository> MagicLinkService<U> {
    pub fn new(
        config: MagicLinkConfig,
        user_repo: std::sync::Arc<U>,
        email_sender: Option<std::sync::Arc<dyn EmailSender>>,
    ) -> Self {
        Self {
            config,
            user_repo,
            email_sender,
            tokens: RwLock::new(HashMap::new()),
            rate_limits: RwLock::new(HashMap::new()),
        }
    }

    /// Request a magic link for login/signup
    pub fn request_magic_link(
        &self,
        email: &str,
        redirect_to: Option<String>,
    ) -> AuthResult<()> {
        // Check rate limit
        self.check_rate_limit(email)?;

        // Validate email format
        if !is_valid_email(email) {
            return Err(AuthError::ValidationError("Invalid email format".to_string()));
        }

        // Check if user exists
        let existing_user = self.user_repo.find_by_email(email)?;

        // Generate token
        let raw_token = generate_token();
        let token_hash = hash_token(&raw_token);

        let token_entry = MagicLinkToken {
            token_hash: token_hash.clone(),
            user_id: existing_user.as_ref().map(|u| u.id),
            email: email.to_string(),
            redirect_to,
            expires_at: Utc::now() + Duration::minutes(self.config.expiration_minutes),
            is_signup: existing_user.is_none(),
        };

        // Store token
        {
            let mut tokens = self.tokens.write().unwrap();
            // Remove any existing token for this email
            tokens.retain(|_, t| t.email != email);
            tokens.insert(token_hash, token_entry);
        }

        // Update rate limit
        self.update_rate_limit(email);

        // Build magic link URL
        let magic_link = format!(
            "{}/auth/verify?token={}",
            self.config.base_url,
            urlencoding::encode(&raw_token)
        );

        // Send email
        if let Some(sender) = &self.email_sender {
            sender.send(EmailTemplate::MagicLink {
                link: magic_link,
                user_email: email.to_string(),
                expires_minutes: self.config.expiration_minutes,
            })?;
        }

        Ok(())
    }

    /// Verify a magic link token
    pub fn verify_magic_link(&self, raw_token: &str) -> AuthResult<(User, bool)> {
        let token_hash = hash_token(raw_token);

        // Find and remove token
        let token_entry = {
            let mut tokens = self.tokens.write().unwrap();
            tokens.remove(&token_hash)
        };

        let entry = token_entry.ok_or_else(|| {
            AuthError::TokenInvalid("Invalid or expired magic link".to_string())
        })?;

        // Check expiration
        if entry.expires_at < Utc::now() {
            return Err(AuthError::TokenExpired);
        }

        // Get or create user
        let (user, is_new) = if let Some(user_id) = entry.user_id {
            // Existing user
            let user = self.user_repo.find_by_id(user_id)?
                .ok_or(AuthError::UserNotFound)?;
            (user, false)
        } else {
            // Create new user
            let new_user = User {
                id: Uuid::new_v4(),
                email: entry.email.clone(),
                email_verified: true, // Magic link verifies email
                password_hash: String::new(), // No password for magic link users
                metadata: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };
            self.user_repo.create(&new_user)?;
            (new_user, true)
        };

        // If user existed but email wasn't verified, verify it now
        if !is_new && !user.email_verified {
            // Note: In production, update the user's email_verified flag
        }

        Ok((user, is_new))
    }

    /// Check rate limit for an email
    fn check_rate_limit(&self, email: &str) -> AuthResult<()> {
        let rate_limits = self.rate_limits.read().unwrap();

        if let Some(entry) = rate_limits.get(&email.to_lowercase()) {
            let hour_ago = Utc::now() - Duration::hours(1);

            if entry.window_start > hour_ago && entry.count >= self.config.rate_limit {
                return Err(AuthError::RateLimitExceeded(
                    "Too many login attempts. Please try again later.".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Update rate limit for an email
    fn update_rate_limit(&self, email: &str) {
        let mut rate_limits = self.rate_limits.write().unwrap();
        let email_lower = email.to_lowercase();
        let now = Utc::now();
        let hour_ago = now - Duration::hours(1);

        let entry = rate_limits.entry(email_lower).or_insert(RateLimitEntry {
            count: 0,
            window_start: now,
        });

        if entry.window_start < hour_ago {
            // Reset window
            entry.count = 1;
            entry.window_start = now;
        } else {
            entry.count += 1;
        }
    }

    /// Clean up expired tokens
    pub fn cleanup_expired(&self) {
        let mut tokens = self.tokens.write().unwrap();
        let now = Utc::now();
        tokens.retain(|_, t| t.expires_at > now);
    }

    /// Get the redirect URL for a token (for internal use)
    pub fn get_redirect_url(&self, raw_token: &str) -> Option<String> {
        let token_hash = hash_token(raw_token);
        let tokens = self.tokens.read().unwrap();
        tokens.get(&token_hash).and_then(|t| t.redirect_to.clone())
    }
}

// ==================
// Helper Functions
// ==================

/// Hash a token for storage
fn hash_token(token: &str) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Basic email validation
fn is_valid_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local = parts[0];
    let domain = parts[1];

    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
}

// ==================
// Auth Hooks
// ==================

/// Auth event types for hooks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthEvent {
    UserSignedUp,
    UserSignedIn,
    UserSignedOut,
    PasswordChanged,
    PasswordReset,
    EmailVerified,
    MfaEnrolled,
    MfaVerified,
    OAuthLinked,
    OAuthUnlinked,
}

/// Auth hook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthHookPayload {
    pub event: AuthEvent,
    pub user_id: Uuid,
    pub email: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

impl AuthHookPayload {
    pub fn new(event: AuthEvent, user: &User) -> Self {
        Self {
            event,
            user_id: user.id,
            email: user.email.clone(),
            timestamp: Utc::now(),
            metadata: serde_json::json!({}),
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Auth hook handler trait
pub trait AuthHookHandler: Send + Sync {
    fn handle(&self, payload: &AuthHookPayload) -> AuthResult<()>;
}

/// Auth hooks registry
pub struct AuthHooks {
    handlers: RwLock<Vec<(AuthEvent, Box<dyn AuthHookHandler>)>>,
}

impl AuthHooks {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
        }
    }

    /// Register a hook handler for an event
    pub fn on(&self, event: AuthEvent, handler: Box<dyn AuthHookHandler>) {
        let mut handlers = self.handlers.write().unwrap();
        handlers.push((event, handler));
    }

    /// Trigger hooks for an event
    pub fn trigger(&self, payload: &AuthHookPayload) {
        let handlers = self.handlers.read().unwrap();
        for (event, handler) in handlers.iter() {
            if *event == payload.event {
                // Ignore errors in hooks (don't block auth flow)
                let _ = handler.handle(payload);
            }
        }
    }
}

impl Default for AuthHooks {
    fn default() -> Self {
        Self::new()
    }
}

// ==================
// Tests
// ==================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::user::InMemoryUserRepository;

    fn create_test_service() -> MagicLinkService<InMemoryUserRepository> {
        let user_repo = std::sync::Arc::new(InMemoryUserRepository::new());
        MagicLinkService::new(MagicLinkConfig::default(), user_repo, None)
    }

    #[test]
    fn test_email_validation() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("user.name@example.co.uk"));
        assert!(!is_valid_email("invalid"));
        assert!(!is_valid_email("@example.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@.com"));
    }

    #[test]
    fn test_token_hashing() {
        let token = "test_token_123";
        let hash1 = hash_token(token);
        let hash2 = hash_token(token);

        assert_eq!(hash1, hash2); // Deterministic
        assert_ne!(hash1, token); // Not plaintext
        assert_eq!(hash1.len(), 64); // SHA256 hex length
    }

    #[test]
    fn test_request_magic_link() {
        let service = create_test_service();

        // Should succeed
        assert!(service.request_magic_link("user@example.com", None).is_ok());

        // Token should be stored
        let tokens = service.tokens.read().unwrap();
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn test_invalid_email_rejected() {
        let service = create_test_service();

        let result = service.request_magic_link("invalid-email", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limiting() {
        let mut config = MagicLinkConfig::default();
        config.rate_limit = 2;

        let user_repo = std::sync::Arc::new(InMemoryUserRepository::new());
        let service = MagicLinkService::new(config, user_repo, None);

        // First two should succeed
        assert!(service.request_magic_link("user@example.com", None).is_ok());
        assert!(service.request_magic_link("user@example.com", None).is_ok());

        // Third should fail
        let result = service.request_magic_link("user@example.com", None);
        assert!(matches!(result, Err(AuthError::RateLimitExceeded(_))));
    }

    #[test]
    fn test_cleanup_expired() {
        let service = create_test_service();

        service.request_magic_link("user@example.com", None).unwrap();
        assert_eq!(service.tokens.read().unwrap().len(), 1);

        // Manually expire the token
        {
            let mut tokens = service.tokens.write().unwrap();
            for (_, token) in tokens.iter_mut() {
                token.expires_at = Utc::now() - Duration::hours(1);
            }
        }

        service.cleanup_expired();
        assert_eq!(service.tokens.read().unwrap().len(), 0);
    }

    #[test]
    fn test_auth_hook_payload() {
        let user = User {
            id: Uuid::new_v4(),
            email: "user@example.com".to_string(),
            email_verified: true,
            password_hash: String::new(),
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let payload = AuthHookPayload::new(AuthEvent::UserSignedIn, &user)
            .with_metadata(serde_json::json!({"ip": "127.0.0.1"}));

        assert_eq!(payload.event, AuthEvent::UserSignedIn);
        assert_eq!(payload.user_id, user.id);
        assert_eq!(payload.metadata["ip"], "127.0.0.1");
    }

    #[test]
    fn test_auth_hooks_registry() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        struct CountingHandler(Arc<AtomicUsize>);

        impl AuthHookHandler for CountingHandler {
            fn handle(&self, _payload: &AuthHookPayload) -> AuthResult<()> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let hooks = AuthHooks::new();

        hooks.on(AuthEvent::UserSignedIn, Box::new(CountingHandler(counter.clone())));

        let user = User {
            id: Uuid::new_v4(),
            email: "user@example.com".to_string(),
            email_verified: true,
            password_hash: String::new(),
            metadata: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let payload = AuthHookPayload::new(AuthEvent::UserSignedIn, &user);
        hooks.trigger(&payload);

        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Different event shouldn't trigger
        let payload2 = AuthHookPayload::new(AuthEvent::UserSignedUp, &user);
        hooks.trigger(&payload2);

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
