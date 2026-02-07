//! # JWT Token Management
//!
//! JSON Web Token generation and validation.
//!
//! ## Invariants
//! - AUTH-JWT1: Stateless validation (no DB lookup)
//! - AUTH-JWT2: Short expiration (15 minutes)
//! - AUTH-JWT3: No secrets in token

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::{AuthError, AuthResult};
use super::user::User;

/// JWT claims for access tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,

    /// User's email
    pub email: String,

    /// Issued at timestamp (Unix epoch seconds)
    pub iat: i64,

    /// Expiration timestamp (Unix epoch seconds)
    pub exp: i64,

    /// Audience (project or application ID)
    pub aud: String,

    /// Issuer
    pub iss: String,

    /// Whether email is verified
    pub email_verified: bool,
}

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing (256-bit minimum recommended)
    pub secret: String,

    /// Access token lifetime
    pub access_token_ttl: Duration,

    /// Issuer identifier
    pub issuer: String,

    /// Audience identifier
    pub audience: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "CHANGE_THIS_SECRET_IN_PRODUCTION".to_string(),
            access_token_ttl: Duration::minutes(15),
            issuer: "aerodb".to_string(),
            audience: "aerodb".to_string(),
        }
    }
}

/// JWT manager for token generation and validation
#[derive(Clone)]
pub struct JwtManager {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    /// Create a new JWT manager with the given configuration
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// Generate an access token for a user
    ///
    /// # Invariants
    /// - AUTH-JWT2: Token expires in 15 minutes
    /// - AUTH-JWT3: No secrets in token (only user ID, email, verification status)
    pub fn generate_access_token(&self, user: &User) -> AuthResult<String> {
        let now = Utc::now();
        let exp = now + self.config.access_token_ttl;

        let claims = JwtClaims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            iat: now.timestamp(),
            exp: exp.timestamp(),
            aud: self.config.audience.clone(),
            iss: self.config.issuer.clone(),
            email_verified: user.email_verified,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|_| AuthError::TokenGenerationFailed)
    }

    /// Validate an access token and extract claims
    ///
    /// # Invariant
    /// AUTH-JWT1: Validation is stateless (no DB lookup required)
    pub fn validate_token(&self, token: &str) -> AuthResult<JwtClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[&self.config.audience]);
        validation.set_issuer(&[&self.config.issuer]);

        let token_data =
            decode::<JwtClaims>(token, &self.decoding_key, &validation).map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                        AuthError::InvalidSignature
                    }
                    _ => AuthError::MalformedToken,
                }
            })?;

        Ok(token_data.claims)
    }

    /// Extract user ID from validated claims
    pub fn get_user_id(claims: &JwtClaims) -> AuthResult<Uuid> {
        Uuid::parse_str(&claims.sub).map_err(|_| AuthError::MalformedToken)
    }

    /// Get the expiration time for a new token
    pub fn get_expiration(&self) -> chrono::DateTime<Utc> {
        Utc::now() + self.config.access_token_ttl
    }
}

/// Token response returned to client
#[derive(Debug, Clone, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub expires_at: i64,
    pub refresh_token: String,
}

impl TokenResponse {
    pub fn new(
        access_token: String,
        refresh_token: String,
        expires_at: chrono::DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        let expires_in = (expires_at - now).num_seconds();

        Self {
            access_token,
            token_type: "bearer".to_string(),
            expires_in,
            expires_at: expires_at.timestamp(),
            refresh_token,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::crypto::PasswordPolicy;

    fn create_test_manager() -> JwtManager {
        JwtManager::new(JwtConfig {
            secret: "test_secret_key_for_testing_only".to_string(),
            access_token_ttl: Duration::minutes(15),
            issuer: "test".to_string(),
            audience: "test".to_string(),
        })
    }

    fn create_test_user() -> User {
        User::new(
            "test@example.com".to_string(),
            "password123",
            &PasswordPolicy::default(),
        )
        .unwrap()
    }

    #[test]
    fn test_token_generation() {
        let manager = create_test_manager();
        let user = create_test_user();

        let token = manager.generate_access_token(&user).unwrap();

        // Token should be non-empty
        assert!(!token.is_empty());

        // Token should have three parts (header.payload.signature)
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn test_token_validation() {
        let manager = create_test_manager();
        let user = create_test_user();

        let token = manager.generate_access_token(&user).unwrap();
        let claims = manager.validate_token(&token).unwrap();

        assert_eq!(claims.sub, user.id.to_string());
        assert_eq!(claims.email, user.email);
        assert_eq!(claims.email_verified, user.email_verified);
    }

    #[test]
    fn test_invalid_token_rejected() {
        let manager = create_test_manager();

        let result = manager.validate_token("invalid.token.here");
        assert!(matches!(
            result,
            Err(AuthError::MalformedToken) | Err(AuthError::InvalidSignature)
        ));
    }

    #[test]
    fn test_wrong_secret_rejected() {
        let manager1 = JwtManager::new(JwtConfig {
            secret: "secret_one".to_string(),
            ..JwtConfig::default()
        });

        let manager2 = JwtManager::new(JwtConfig {
            secret: "secret_two".to_string(),
            ..JwtConfig::default()
        });

        let user = create_test_user();
        let token = manager1.generate_access_token(&user).unwrap();

        // Token from manager1 should not validate with manager2
        let result = manager2.validate_token(&token);
        assert!(matches!(result, Err(AuthError::InvalidSignature)));
    }

    #[test]
    fn test_expired_token_rejected() {
        // Create a token with an expiration time in the past by encoding manually
        let secret = "test_secret";
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());

        let now = Utc::now();
        let claims = JwtClaims {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            iat: (now - Duration::hours(2)).timestamp(),
            exp: (now - Duration::hours(1)).timestamp(), // Expired 1 hour ago
            aud: "test".to_string(),
            iss: "test".to_string(),
            email_verified: false,
        };

        let token = encode(&Header::default(), &claims, &encoding_key).unwrap();

        let manager = JwtManager::new(JwtConfig {
            secret: secret.to_string(),
            access_token_ttl: Duration::minutes(15),
            issuer: "test".to_string(),
            audience: "test".to_string(),
        });

        let result = manager.validate_token(&token);
        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[test]
    fn test_user_id_extraction() {
        let manager = create_test_manager();
        let user = create_test_user();

        let token = manager.generate_access_token(&user).unwrap();
        let claims = manager.validate_token(&token).unwrap();
        let user_id = JwtManager::get_user_id(&claims).unwrap();

        assert_eq!(user_id, user.id);
    }

    #[test]
    fn test_token_does_not_contain_secrets() {
        let manager = create_test_manager();
        let user = create_test_user();

        let token = manager.generate_access_token(&user).unwrap();

        // Token should not contain password hash or other secrets
        // (The password hash shouldn't even be accessible to generate_access_token,
        // but we verify the token string itself doesn't contain it)
        assert!(!token.contains("password"));
        assert!(!token.contains(&user.password_hash));
    }
}
