//! # Cryptographic Utilities
//!
//! Password hashing and secure token generation.
//!
//! ## Invariants
//! - AUTH-S2: Passwords only stored as Argon2id hashes
//! - AUTH-S3: Constant-time comparison for all secrets

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::errors::{AuthError, AuthResult};

/// Password requirements configuration
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_number: bool,
    pub require_special: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            require_uppercase: false,
            require_lowercase: false,
            require_number: false,
            require_special: false,
        }
    }
}

impl PasswordPolicy {
    /// Validate a password against this policy
    pub fn validate(&self, password: &str) -> AuthResult<()> {
        validate_password(password, self)
    }
}

/// Validate password against policy
pub fn validate_password(password: &str, policy: &PasswordPolicy) -> AuthResult<()> {
    if password.len() < policy.min_length {
        return Err(AuthError::WeakPassword(format!(
            "Password must be at least {} characters",
            policy.min_length
        )));
    }

    if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        return Err(AuthError::WeakPassword(
            "Password must contain at least one uppercase letter".to_string(),
        ));
    }

    if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
        return Err(AuthError::WeakPassword(
            "Password must contain at least one lowercase letter".to_string(),
        ));
    }

    if policy.require_number && !password.chars().any(|c| c.is_numeric()) {
        return Err(AuthError::WeakPassword(
            "Password must contain at least one number".to_string(),
        ));
    }

    if policy.require_special && !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err(AuthError::WeakPassword(
            "Password must contain at least one special character".to_string(),
        ));
    }

    Ok(())
}

/// Hash a password using Argon2id
///
/// # Invariant
/// AUTH-S2: Passwords only stored as Argon2id hashes
pub fn hash_password(password: &str) -> AuthResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::HashingFailed)
}

/// Verify a password against its hash
///
/// Uses constant-time comparison internally (via argon2 crate).
pub fn verify_password(password: &str, hash: &str) -> AuthResult<bool> {
    let parsed_hash = PasswordHash::new(hash).map_err(|_| AuthError::InvalidCredentials)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

/// Generate a cryptographically secure random token
///
/// Returns a 256-bit (32-byte) random value as base64.
pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// Hash a token for storage using SHA-256
///
/// Tokens are stored hashed; the raw token is only given to the user.
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let result = hasher.finalize();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, result)
}

/// Constant-time comparison of two byte slices
///
/// # Invariant
/// AUTH-S3: Constant-time comparison for all secrets
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

/// Constant-time comparison of two strings
pub fn constant_time_str_eq(a: &str, b: &str) -> bool {
    constant_time_eq(a.as_bytes(), b.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let password = "secure_password_123";
        let hash = hash_password(password).unwrap();

        // Hash should be different from password
        assert_ne!(hash, password);

        // Verification should succeed
        assert!(verify_password(password, &hash).unwrap());

        // Wrong password should fail
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_password_hash_produces_unique_hashes() {
        let password = "same_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Same password should produce different hashes (due to salt)
        assert_ne!(hash1, hash2);

        // But both should verify
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_password_validation() {
        let policy = PasswordPolicy {
            min_length: 8,
            require_uppercase: true,
            require_number: true,
            ..Default::default()
        };

        // Too short
        assert!(validate_password("Ab1", &policy).is_err());

        // Missing uppercase
        assert!(validate_password("abcdefgh1", &policy).is_err());

        // Missing number
        assert!(validate_password("Abcdefgh", &policy).is_err());

        // Valid
        assert!(validate_password("Abcdefgh1", &policy).is_ok());
    }

    #[test]
    fn test_token_generation() {
        let token1 = generate_token();
        let token2 = generate_token();

        // Tokens should be unique
        assert_ne!(token1, token2);

        // Tokens should be reasonable length (base64 of 32 bytes)
        assert!(token1.len() >= 32);
    }

    #[test]
    fn test_token_hashing() {
        let token = generate_token();
        let hash = hash_token(&token);

        // Hash should be different from token
        assert_ne!(token, hash);

        // Same token should produce same hash
        assert_eq!(hash, hash_token(&token));
    }

    #[test]
    fn test_constant_time_comparison() {
        assert!(constant_time_str_eq("hello", "hello"));
        assert!(!constant_time_str_eq("hello", "world"));
        assert!(!constant_time_str_eq("hello", "hello!"));
    }
}
