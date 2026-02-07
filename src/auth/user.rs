//! # User Management
//!
//! User model and repository for authentication.
//! Users are stored as documents in the `_users` collection.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::crypto::{hash_password, validate_password, verify_password, PasswordPolicy};
use super::errors::{AuthError, AuthResult};

/// User model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user identifier
    pub id: Uuid,

    /// User's email address (unique)
    pub email: String,

    /// Whether email has been verified
    pub email_verified: bool,

    /// Argon2id password hash (never plaintext)
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// When the user was created
    pub created_at: DateTime<Utc>,

    /// When the user was last updated
    pub updated_at: DateTime<Utc>,

    /// Optional user metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl User {
    /// Create a new user with the given email and password
    pub fn new(email: String, password: &str, policy: &PasswordPolicy) -> AuthResult<Self> {
        // Validate password
        validate_password(password, policy)?;

        // Hash password
        let password_hash = hash_password(password)?;

        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4(),
            email,
            email_verified: false,
            password_hash,
            created_at: now,
            updated_at: now,
            metadata: None,
        })
    }

    /// Verify a password against this user's stored hash
    pub fn verify_password(&self, password: &str) -> AuthResult<bool> {
        verify_password(password, &self.password_hash)
    }

    /// Update the user's password
    pub fn update_password(
        &mut self,
        new_password: &str,
        policy: &PasswordPolicy,
    ) -> AuthResult<()> {
        validate_password(new_password, policy)?;
        self.password_hash = hash_password(new_password)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark email as verified
    pub fn verify_email(&mut self) {
        self.email_verified = true;
        self.updated_at = Utc::now();
    }

    /// Set a new password (bypasses policy validation - use when policy already validated)
    pub fn set_password(&mut self, new_password: &str) -> AuthResult<()> {
        self.password_hash = hash_password(new_password)?;
        self.updated_at = Utc::now();
        Ok(())
    }
}

/// User creation request
#[derive(Debug, Clone, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// User login request
#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// User profile update request
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// User repository trait
///
/// Abstracts storage operations for users.
pub trait UserRepository: Send + Sync {
    /// Find a user by their ID
    fn find_by_id(&self, id: Uuid) -> AuthResult<Option<User>>;

    /// Find a user by their email
    fn find_by_email(&self, email: &str) -> AuthResult<Option<User>>;

    /// Check if an email is already registered
    fn email_exists(&self, email: &str) -> AuthResult<bool>;

    /// Create a new user
    fn create(&self, user: &User) -> AuthResult<()>;

    /// Update an existing user
    fn update(&self, user: &User) -> AuthResult<()>;

    /// Delete a user
    fn delete(&self, id: Uuid) -> AuthResult<()>;
}

/// In-memory user repository for testing
#[derive(Debug, Default)]
pub struct InMemoryUserRepository {
    users: std::sync::RwLock<Vec<User>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl UserRepository for InMemoryUserRepository {
    fn find_by_id(&self, id: Uuid) -> AuthResult<Option<User>> {
        let users = self
            .users
            .read()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    fn find_by_email(&self, email: &str) -> AuthResult<Option<User>> {
        let users = self
            .users
            .read()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;
        Ok(users.iter().find(|u| u.email == email).cloned())
    }

    fn email_exists(&self, email: &str) -> AuthResult<bool> {
        let users = self
            .users
            .read()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;
        Ok(users.iter().any(|u| u.email == email))
    }

    fn create(&self, user: &User) -> AuthResult<()> {
        let mut users = self
            .users
            .write()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;

        if users.iter().any(|u| u.email == user.email) {
            return Err(AuthError::EmailAlreadyExists);
        }

        users.push(user.clone());
        Ok(())
    }

    fn update(&self, user: &User) -> AuthResult<()> {
        let mut users = self
            .users
            .write()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;

        if let Some(existing) = users.iter_mut().find(|u| u.id == user.id) {
            *existing = user.clone();
            Ok(())
        } else {
            Err(AuthError::StorageError("User not found".to_string()))
        }
    }

    fn delete(&self, id: Uuid) -> AuthResult<()> {
        let mut users = self
            .users
            .write()
            .map_err(|_| AuthError::StorageError("Lock poisoned".to_string()))?;

        let len_before = users.len();
        users.retain(|u| u.id != id);

        if users.len() == len_before {
            Err(AuthError::StorageError("User not found".to_string()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_policy() -> PasswordPolicy {
        PasswordPolicy::default()
    }

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "test@example.com".to_string(),
            "password123",
            &default_policy(),
        )
        .unwrap();

        assert_eq!(user.email, "test@example.com");
        assert!(!user.email_verified);
        assert!(!user.password_hash.is_empty());
        assert_ne!(user.password_hash, "password123"); // Not plaintext!
    }

    #[test]
    fn test_password_verification() {
        let user = User::new(
            "test@example.com".to_string(),
            "password123",
            &default_policy(),
        )
        .unwrap();

        assert!(user.verify_password("password123").unwrap());
        assert!(!user.verify_password("wrong_password").unwrap());
    }

    #[test]
    fn test_weak_password_rejected() {
        let policy = PasswordPolicy {
            min_length: 10,
            ..Default::default()
        };

        let result = User::new("test@example.com".to_string(), "short", &policy);
        assert!(matches!(result, Err(AuthError::WeakPassword(_))));
    }

    #[test]
    fn test_email_verification() {
        let mut user = User::new(
            "test@example.com".to_string(),
            "password123",
            &default_policy(),
        )
        .unwrap();

        assert!(!user.email_verified);
        user.verify_email();
        assert!(user.email_verified);
    }

    #[test]
    fn test_in_memory_repository() {
        let repo = InMemoryUserRepository::new();

        // Create user
        let user = User::new(
            "test@example.com".to_string(),
            "password123",
            &default_policy(),
        )
        .unwrap();
        let user_id = user.id;

        repo.create(&user).unwrap();

        // Find by ID
        let found = repo.find_by_id(user_id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().email, "test@example.com");

        // Find by email
        let found = repo.find_by_email("test@example.com").unwrap();
        assert!(found.is_some());

        // Email exists check
        assert!(repo.email_exists("test@example.com").unwrap());
        assert!(!repo.email_exists("other@example.com").unwrap());

        // Duplicate email rejected
        let user2 = User::new(
            "test@example.com".to_string(),
            "password456",
            &default_policy(),
        )
        .unwrap();
        assert!(matches!(
            repo.create(&user2),
            Err(AuthError::EmailAlreadyExists)
        ));

        // Delete
        repo.delete(user_id).unwrap();
        assert!(repo.find_by_id(user_id).unwrap().is_none());
    }

    #[test]
    fn test_user_serialization_omits_password() {
        let user = User::new(
            "test@example.com".to_string(),
            "password123",
            &default_policy(),
        )
        .unwrap();

        let json = serde_json::to_string(&user).unwrap();

        // Password hash should NOT appear in serialized output
        assert!(!json.contains("password_hash"));
        assert!(!json.contains(&user.password_hash));
    }
}
