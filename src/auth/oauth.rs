//! # OAuth Provider Support
//!
//! OAuth 2.0 authentication with popular identity providers.
//! Supports Google, GitHub, and Discord out of the box.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::errors::{AuthError, AuthResult};
use super::jwt::TokenResponse;
use super::user::{User, UserRepository};
use super::session::{SessionConfig, SessionRepository};

// ==================
// OAuth Provider Configuration
// ==================

/// Supported OAuth providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    Google,
    GitHub,
    Discord,
    Custom,
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthProvider::Google => write!(f, "google"),
            OAuthProvider::GitHub => write!(f, "github"),
            OAuthProvider::Discord => write!(f, "discord"),
            OAuthProvider::Custom => write!(f, "custom"),
        }
    }
}

/// Configuration for an OAuth provider
#[derive(Debug, Clone)]
pub struct OAuthProviderConfig {
    pub provider: OAuthProvider,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    /// Custom authorization URL (for custom providers)
    pub auth_url: Option<String>,
    /// Custom token URL (for custom providers)
    pub token_url: Option<String>,
    /// Custom user info URL (for custom providers)
    pub userinfo_url: Option<String>,
}

impl OAuthProviderConfig {
    /// Create Google OAuth config
    pub fn google(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            provider: OAuthProvider::Google,
            client_id,
            client_secret,
            redirect_uri,
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            auth_url: None,
            token_url: None,
            userinfo_url: None,
        }
    }

    /// Create GitHub OAuth config
    pub fn github(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            provider: OAuthProvider::GitHub,
            client_id,
            client_secret,
            redirect_uri,
            scopes: vec!["user:email".to_string(), "read:user".to_string()],
            auth_url: None,
            token_url: None,
            userinfo_url: None,
        }
    }

    /// Create Discord OAuth config
    pub fn discord(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            provider: OAuthProvider::Discord,
            client_id,
            client_secret,
            redirect_uri,
            scopes: vec!["identify".to_string(), "email".to_string()],
            auth_url: None,
            token_url: None,
            userinfo_url: None,
        }
    }

    /// Get authorization URL for the provider
    pub fn auth_url(&self) -> &str {
        self.auth_url.as_deref().unwrap_or(match self.provider {
            OAuthProvider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            OAuthProvider::GitHub => "https://github.com/login/oauth/authorize",
            OAuthProvider::Discord => "https://discord.com/api/oauth2/authorize",
            OAuthProvider::Custom => "",
        })
    }

    /// Get token URL for the provider
    pub fn token_url(&self) -> &str {
        self.token_url.as_deref().unwrap_or(match self.provider {
            OAuthProvider::Google => "https://oauth2.googleapis.com/token",
            OAuthProvider::GitHub => "https://github.com/login/oauth/access_token",
            OAuthProvider::Discord => "https://discord.com/api/oauth2/token",
            OAuthProvider::Custom => "",
        })
    }

    /// Get user info URL for the provider
    pub fn userinfo_url(&self) -> &str {
        self.userinfo_url.as_deref().unwrap_or(match self.provider {
            OAuthProvider::Google => "https://www.googleapis.com/oauth2/v3/userinfo",
            OAuthProvider::GitHub => "https://api.github.com/user",
            OAuthProvider::Discord => "https://discord.com/api/users/@me",
            OAuthProvider::Custom => "",
        })
    }
}

// ==================
// OAuth State Management
// ==================

/// OAuth state for CSRF protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthState {
    pub state: String,
    pub provider: OAuthProvider,
    pub redirect_to: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl OAuthState {
    pub fn new(provider: OAuthProvider, redirect_to: Option<String>) -> Self {
        Self {
            state: Uuid::new_v4().to_string(),
            provider,
            redirect_to,
            created_at: chrono::Utc::now(),
        }
    }

    pub fn is_expired(&self, max_age_seconds: i64) -> bool {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.created_at);
        age.num_seconds() > max_age_seconds
    }
}

// ==================
// OAuth User Info
// ==================

/// Normalized user info from OAuth providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    pub provider: OAuthProvider,
    pub provider_id: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub raw_data: serde_json::Value,
}

/// Google user info response
#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
}

/// GitHub user info response
#[derive(Debug, Deserialize)]
struct GitHubUserInfo {
    id: i64,
    email: Option<String>,
    name: Option<String>,
    avatar_url: Option<String>,
}

/// Discord user info response
#[derive(Debug, Deserialize)]
struct DiscordUserInfo {
    id: String,
    email: Option<String>,
    verified: Option<bool>,
    username: String,
    avatar: Option<String>,
}

impl OAuthUserInfo {
    /// Parse Google user info
    pub fn from_google(data: serde_json::Value) -> AuthResult<Self> {
        let info: GoogleUserInfo = serde_json::from_value(data.clone())
            .map_err(|e| AuthError::OAuthError(format!("Failed to parse Google user info: {}", e)))?;

        Ok(Self {
            provider: OAuthProvider::Google,
            provider_id: info.sub,
            email: info.email,
            email_verified: info.email_verified.unwrap_or(false),
            name: info.name,
            avatar_url: info.picture,
            raw_data: data,
        })
    }

    /// Parse GitHub user info
    pub fn from_github(data: serde_json::Value) -> AuthResult<Self> {
        let info: GitHubUserInfo = serde_json::from_value(data.clone())
            .map_err(|e| AuthError::OAuthError(format!("Failed to parse GitHub user info: {}", e)))?;

        Ok(Self {
            provider: OAuthProvider::GitHub,
            provider_id: info.id.to_string(),
            email: info.email,
            email_verified: true, // GitHub emails are verified
            name: info.name,
            avatar_url: info.avatar_url,
            raw_data: data,
        })
    }

    /// Parse Discord user info
    pub fn from_discord(data: serde_json::Value) -> AuthResult<Self> {
        let info: DiscordUserInfo = serde_json::from_value(data.clone())
            .map_err(|e| AuthError::OAuthError(format!("Failed to parse Discord user info: {}", e)))?;

        let avatar_url = info.avatar.map(|hash| {
            format!("https://cdn.discordapp.com/avatars/{}/{}.png", info.id, hash)
        });

        Ok(Self {
            provider: OAuthProvider::Discord,
            provider_id: info.id,
            email: info.email,
            email_verified: info.verified.unwrap_or(false),
            name: Some(info.username),
            avatar_url,
            raw_data: data,
        })
    }
}

// ==================
// OAuth Token Response
// ==================

/// Token response from OAuth provider
#[derive(Debug, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub id_token: Option<String>,
}

// ==================
// OAuth Identity (for linking)
// ==================

/// Linked OAuth identity for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthIdentity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: OAuthProvider,
    pub provider_id: String,
    pub provider_email: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl OAuthIdentity {
    pub fn new(user_id: Uuid, info: &OAuthUserInfo) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_id,
            provider: info.provider,
            provider_id: info.provider_id.clone(),
            provider_email: info.email.clone(),
            access_token: None,
            refresh_token: None,
            created_at: now,
            updated_at: now,
        }
    }
}

// ==================
// OAuth Repository Trait
// ==================

/// Repository for OAuth identities
pub trait OAuthRepository: Send + Sync {
    /// Find identity by provider and provider ID
    fn find_by_provider_id(
        &self,
        provider: OAuthProvider,
        provider_id: &str,
    ) -> AuthResult<Option<OAuthIdentity>>;

    /// Find all identities for a user
    fn find_by_user_id(&self, user_id: Uuid) -> AuthResult<Vec<OAuthIdentity>>;

    /// Create a new identity
    fn create(&self, identity: OAuthIdentity) -> AuthResult<OAuthIdentity>;

    /// Update identity tokens
    fn update_tokens(
        &self,
        identity_id: Uuid,
        access_token: Option<String>,
        refresh_token: Option<String>,
    ) -> AuthResult<()>;

    /// Delete an identity (unlink provider)
    fn delete(&self, identity_id: Uuid) -> AuthResult<()>;
}

// ==================
// In-Memory OAuth Repository
// ==================

/// Simple in-memory OAuth repository for testing
pub struct InMemoryOAuthRepository {
    identities: std::sync::RwLock<Vec<OAuthIdentity>>,
}

impl InMemoryOAuthRepository {
    pub fn new() -> Self {
        Self {
            identities: std::sync::RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryOAuthRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl OAuthRepository for InMemoryOAuthRepository {
    fn find_by_provider_id(
        &self,
        provider: OAuthProvider,
        provider_id: &str,
    ) -> AuthResult<Option<OAuthIdentity>> {
        let identities = self.identities.read().unwrap();
        Ok(identities
            .iter()
            .find(|i| i.provider == provider && i.provider_id == provider_id)
            .cloned())
    }

    fn find_by_user_id(&self, user_id: Uuid) -> AuthResult<Vec<OAuthIdentity>> {
        let identities = self.identities.read().unwrap();
        Ok(identities
            .iter()
            .filter(|i| i.user_id == user_id)
            .cloned()
            .collect())
    }

    fn create(&self, identity: OAuthIdentity) -> AuthResult<OAuthIdentity> {
        let mut identities = self.identities.write().unwrap();
        identities.push(identity.clone());
        Ok(identity)
    }

    fn update_tokens(
        &self,
        identity_id: Uuid,
        access_token: Option<String>,
        refresh_token: Option<String>,
    ) -> AuthResult<()> {
        let mut identities = self.identities.write().unwrap();
        if let Some(identity) = identities.iter_mut().find(|i| i.id == identity_id) {
            identity.access_token = access_token;
            identity.refresh_token = refresh_token;
            identity.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    fn delete(&self, identity_id: Uuid) -> AuthResult<()> {
        let mut identities = self.identities.write().unwrap();
        identities.retain(|i| i.id != identity_id);
        Ok(())
    }
}

// ==================
// OAuth Service
// ==================

/// OAuth authentication service
pub struct OAuthService<U: UserRepository, O: OAuthRepository> {
    providers: HashMap<OAuthProvider, OAuthProviderConfig>,
    user_repo: Arc<U>,
    oauth_repo: Arc<O>,
    state_store: std::sync::RwLock<HashMap<String, OAuthState>>,
    state_max_age_seconds: i64,
}

impl<U: UserRepository, O: OAuthRepository> OAuthService<U, O> {
    pub fn new(user_repo: Arc<U>, oauth_repo: Arc<O>) -> Self {
        Self {
            providers: HashMap::new(),
            user_repo,
            oauth_repo,
            state_store: std::sync::RwLock::new(HashMap::new()),
            state_max_age_seconds: 600, // 10 minutes
        }
    }

    /// Register an OAuth provider
    pub fn register_provider(&mut self, config: OAuthProviderConfig) {
        self.providers.insert(config.provider, config);
    }

    /// Get authorization URL for a provider
    pub fn get_authorization_url(
        &self,
        provider: OAuthProvider,
        redirect_to: Option<String>,
    ) -> AuthResult<(String, String)> {
        let config = self.providers.get(&provider).ok_or_else(|| {
            AuthError::OAuthError(format!("Provider {} not configured", provider))
        })?;

        let state = OAuthState::new(provider, redirect_to);
        let state_value = state.state.clone();

        // Store state for validation
        {
            let mut states = self.state_store.write().unwrap();
            states.insert(state_value.clone(), state);
        }

        // Build authorization URL
        let params = [
            ("client_id", config.client_id.as_str()),
            ("redirect_uri", config.redirect_uri.as_str()),
            ("response_type", "code"),
            ("state", &state_value),
            ("scope", &config.scopes.join(" ")),
        ];

        let url = format!(
            "{}?{}",
            config.auth_url(),
            params
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect::<Vec<_>>()
                .join("&")
        );

        Ok((url, state_value))
    }

    /// Validate OAuth state
    pub fn validate_state(&self, state: &str) -> AuthResult<OAuthState> {
        let mut states = self.state_store.write().unwrap();

        let oauth_state = states
            .remove(state)
            .ok_or_else(|| AuthError::OAuthError("Invalid or expired state".to_string()))?;

        if oauth_state.is_expired(self.state_max_age_seconds) {
            return Err(AuthError::OAuthError("State expired".to_string()));
        }

        Ok(oauth_state)
    }

    /// Get provider config
    pub fn get_provider_config(&self, provider: OAuthProvider) -> AuthResult<&OAuthProviderConfig> {
        self.providers.get(&provider).ok_or_else(|| {
            AuthError::OAuthError(format!("Provider {} not configured", provider))
        })
    }

    /// Exchange authorization code for tokens (HTTP client needed externally)
    pub fn build_token_request(
        &self,
        provider: OAuthProvider,
        code: &str,
    ) -> AuthResult<(String, HashMap<String, String>)> {
        let config = self.get_provider_config(provider)?;

        let mut params = HashMap::new();
        params.insert("client_id".to_string(), config.client_id.clone());
        params.insert("client_secret".to_string(), config.client_secret.clone());
        params.insert("code".to_string(), code.to_string());
        params.insert("redirect_uri".to_string(), config.redirect_uri.clone());
        params.insert("grant_type".to_string(), "authorization_code".to_string());

        Ok((config.token_url().to_string(), params))
    }

    /// Get user info URL for a provider
    pub fn get_userinfo_url(&self, provider: OAuthProvider) -> AuthResult<String> {
        let config = self.get_provider_config(provider)?;
        Ok(config.userinfo_url().to_string())
    }

    /// Parse user info based on provider
    pub fn parse_user_info(
        &self,
        provider: OAuthProvider,
        data: serde_json::Value,
    ) -> AuthResult<OAuthUserInfo> {
        match provider {
            OAuthProvider::Google => OAuthUserInfo::from_google(data),
            OAuthProvider::GitHub => OAuthUserInfo::from_github(data),
            OAuthProvider::Discord => OAuthUserInfo::from_discord(data),
            OAuthProvider::Custom => Err(AuthError::OAuthError(
                "Custom provider requires manual parsing".to_string(),
            )),
        }
    }

    /// Handle OAuth callback - find or create user
    pub fn handle_oauth_user(&self, info: OAuthUserInfo) -> AuthResult<(User, bool)> {
        // Check if identity already exists
        if let Some(identity) = self.oauth_repo.find_by_provider_id(info.provider, &info.provider_id)? {
            // User exists, return them
            let user = self.user_repo.find_by_id(identity.user_id)?
                .ok_or_else(|| AuthError::UserNotFound)?;
            return Ok((user, false));
        }

        // Check if email exists
        let email = info.email.as_ref().ok_or_else(|| {
            AuthError::OAuthError("Email is required from OAuth provider".to_string())
        })?;

        let (user, is_new) = if let Some(existing_user) = self.user_repo.find_by_email(email)? {
            // Link to existing user
            (existing_user, false)
        } else {
            // Create new user
            let new_user = User {
                id: Uuid::new_v4(),
                email: email.clone(),
                email_verified: info.email_verified,
                password_hash: String::new(), // OAuth users don't have passwords initially
                metadata: Some(serde_json::json!({
                    "name": info.name,
                    "avatar_url": info.avatar_url,
                })),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            self.user_repo.create(&new_user)?;
            (new_user, true)
        };

        // Create OAuth identity link
        let identity = OAuthIdentity::new(user.id, &info);
        self.oauth_repo.create(identity)?;

        Ok((user, is_new))
    }

    /// Link OAuth provider to existing user
    pub fn link_provider(&self, user_id: Uuid, info: OAuthUserInfo) -> AuthResult<OAuthIdentity> {
        // Verify user exists
        self.user_repo.find_by_id(user_id)?
            .ok_or(AuthError::UserNotFound)?;

        // Check if already linked to another user
        if let Some(existing) = self.oauth_repo.find_by_provider_id(info.provider, &info.provider_id)? {
            if existing.user_id != user_id {
                return Err(AuthError::OAuthError(
                    "This provider account is already linked to another user".to_string(),
                ));
            }
            return Ok(existing);
        }

        let identity = OAuthIdentity::new(user_id, &info);
        self.oauth_repo.create(identity)
    }

    /// Unlink OAuth provider from user
    pub fn unlink_provider(&self, user_id: Uuid, provider: OAuthProvider) -> AuthResult<()> {
        let identities = self.oauth_repo.find_by_user_id(user_id)?;

        let identity = identities
            .iter()
            .find(|i| i.provider == provider)
            .ok_or_else(|| AuthError::OAuthError("Provider not linked".to_string()))?;

        // Ensure user has at least one auth method remaining
        let user = self.user_repo.find_by_id(user_id)?
            .ok_or(AuthError::UserNotFound)?;

        if identities.len() == 1 && user.password_hash.is_empty() {
            return Err(AuthError::OAuthError(
                "Cannot unlink the only authentication method".to_string(),
            ));
        }

        self.oauth_repo.delete(identity.id)
    }

    /// Get linked providers for a user
    pub fn get_linked_providers(&self, user_id: Uuid) -> AuthResult<Vec<OAuthIdentity>> {
        self.oauth_repo.find_by_user_id(user_id)
    }
}

// ==================
// Tests
// ==================

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::user::InMemoryUserRepository;

    fn create_test_service() -> OAuthService<InMemoryUserRepository, InMemoryOAuthRepository> {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let oauth_repo = Arc::new(InMemoryOAuthRepository::new());
        let mut service = OAuthService::new(user_repo, oauth_repo);

        service.register_provider(OAuthProviderConfig::google(
            "google-client-id".to_string(),
            "google-secret".to_string(),
            "http://localhost/callback".to_string(),
        ));
        service.register_provider(OAuthProviderConfig::github(
            "github-client-id".to_string(),
            "github-secret".to_string(),
            "http://localhost/callback".to_string(),
        ));

        service
    }

    #[test]
    fn test_google_config() {
        let config = OAuthProviderConfig::google(
            "client-id".to_string(),
            "secret".to_string(),
            "http://localhost/callback".to_string(),
        );

        assert_eq!(config.provider, OAuthProvider::Google);
        assert!(config.scopes.contains(&"email".to_string()));
        assert_eq!(config.auth_url(), "https://accounts.google.com/o/oauth2/v2/auth");
    }

    #[test]
    fn test_get_authorization_url() {
        let service = create_test_service();

        let (url, state) = service
            .get_authorization_url(OAuthProvider::Google, None)
            .unwrap();

        assert!(url.contains("accounts.google.com"));
        assert!(url.contains("client_id=google-client-id"));
        assert!(url.contains(&format!("state={}", state)));
    }

    #[test]
    fn test_state_validation() {
        let service = create_test_service();

        let (_, state) = service
            .get_authorization_url(OAuthProvider::GitHub, Some("/dashboard".to_string()))
            .unwrap();

        let validated = service.validate_state(&state).unwrap();
        assert_eq!(validated.provider, OAuthProvider::GitHub);
        assert_eq!(validated.redirect_to, Some("/dashboard".to_string()));

        // State should be consumed
        assert!(service.validate_state(&state).is_err());
    }

    #[test]
    fn test_parse_google_user_info() {
        let data = serde_json::json!({
            "sub": "123456789",
            "email": "user@gmail.com",
            "email_verified": true,
            "name": "Test User",
            "picture": "https://example.com/photo.jpg"
        });

        let info = OAuthUserInfo::from_google(data).unwrap();
        assert_eq!(info.provider, OAuthProvider::Google);
        assert_eq!(info.provider_id, "123456789");
        assert_eq!(info.email, Some("user@gmail.com".to_string()));
        assert!(info.email_verified);
    }

    #[test]
    fn test_parse_github_user_info() {
        let data = serde_json::json!({
            "id": 12345,
            "email": "user@github.com",
            "name": "GitHub User",
            "avatar_url": "https://avatars.githubusercontent.com/u/12345"
        });

        let info = OAuthUserInfo::from_github(data).unwrap();
        assert_eq!(info.provider, OAuthProvider::GitHub);
        assert_eq!(info.provider_id, "12345");
        assert!(info.email_verified); // GitHub emails are always verified
    }

    #[test]
    fn test_parse_discord_user_info() {
        let data = serde_json::json!({
            "id": "987654321",
            "email": "user@discord.com",
            "verified": true,
            "username": "DiscordUser",
            "avatar": "abc123"
        });

        let info = OAuthUserInfo::from_discord(data).unwrap();
        assert_eq!(info.provider, OAuthProvider::Discord);
        assert_eq!(info.provider_id, "987654321");
        assert!(info.avatar_url.unwrap().contains("cdn.discordapp.com"));
    }

    #[test]
    fn test_provider_not_configured() {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let oauth_repo = Arc::new(InMemoryOAuthRepository::new());
        let service = OAuthService::new(user_repo, oauth_repo);

        let result = service.get_authorization_url(OAuthProvider::Google, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_oauth_state_expiry() {
        let mut state = OAuthState::new(OAuthProvider::Google, None);

        assert!(!state.is_expired(600));

        // Simulate expired state
        state.created_at = chrono::Utc::now() - chrono::Duration::seconds(700);
        assert!(state.is_expired(600));
    }
}
