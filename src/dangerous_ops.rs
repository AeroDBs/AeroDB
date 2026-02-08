//! Dangerous Operation Protection
//!
//! HARDENING: Protects against operator misuse.
//!
//! - Requires confirmation for destructive operations
//! - Provides clear warnings
//! - Logs all dangerous operations

use std::time::SystemTime;
use serde::{Deserialize, Serialize};

/// Types of dangerous operations that require confirmation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DangerousOperation {
    /// Delete all documents in a collection
    TruncateCollection,
    /// Drop a collection entirely
    DropCollection,
    /// Delete a schema
    DeleteSchema,
    /// Force failover
    ForceFailover,
    /// Reset WAL (data loss risk)
    ResetWal,
    /// Restore from backup (overwrites current data)
    RestoreBackup,
    /// Compact storage (requires downtime)
    CompactStorage,
    /// Delete all data (factory reset)
    FactoryReset,
}

impl DangerousOperation {
    /// Human-readable description of the operation
    pub fn description(&self) -> &'static str {
        match self {
            DangerousOperation::TruncateCollection => "Delete all documents in the collection",
            DangerousOperation::DropCollection => "Permanently delete the collection and all its data",
            DangerousOperation::DeleteSchema => "Delete the schema definition",
            DangerousOperation::ForceFailover => "Force a failover to a replica",
            DangerousOperation::ResetWal => "Reset the write-ahead log (may cause data loss)",
            DangerousOperation::RestoreBackup => "Restore from backup (overwrites all current data)",
            DangerousOperation::CompactStorage => "Compact storage files (requires brief downtime)",
            DangerousOperation::FactoryReset => "Delete all data and reset to factory state",
        }
    }

    /// Warning message shown before confirmation
    pub fn warning(&self) -> &'static str {
        match self {
            DangerousOperation::TruncateCollection => 
                "WARNING: This will permanently delete ALL documents in this collection. This action cannot be undone.",
            DangerousOperation::DropCollection => 
                "WARNING: This will permanently delete the collection and all its data. This action cannot be undone.",
            DangerousOperation::DeleteSchema => 
                "WARNING: Deleting the schema will also delete all collections using this schema.",
            DangerousOperation::ForceFailover => 
                "WARNING: Force failover may cause brief unavailability. Ensure replicas are healthy.",
            DangerousOperation::ResetWal => 
                "DANGER: Resetting the WAL may cause data loss for un-checkpointed operations. Only use if WAL is corrupted.",
            DangerousOperation::RestoreBackup => 
                "WARNING: Restore will overwrite ALL current data with the backup contents. Current data will be lost.",
            DangerousOperation::CompactStorage => 
                "NOTE: Compaction requires brief downtime. Schedule during maintenance window.",
            DangerousOperation::FactoryReset => 
                "DANGER: This will permanently delete ALL data, including all collections, schemas, users, and settings. This action CANNOT be undone.",
        }
    }

    /// Whether this operation requires typing a confirmation phrase
    pub fn requires_type_confirm(&self) -> bool {
        matches!(
            self,
            DangerousOperation::FactoryReset
                | DangerousOperation::ResetWal
                | DangerousOperation::DropCollection
        )
    }

    /// Confirmation phrase to type (if requires_type_confirm is true)
    pub fn confirm_phrase(&self, resource_name: &str) -> String {
        match self {
            DangerousOperation::FactoryReset => "delete everything".to_string(),
            DangerousOperation::ResetWal => "reset wal".to_string(),
            DangerousOperation::DropCollection => format!("drop {}", resource_name),
            _ => "confirm".to_string(),
        }
    }

    /// Log level for this operation
    pub fn log_level(&self) -> &'static str {
        match self {
            DangerousOperation::FactoryReset | DangerousOperation::ResetWal => "CRITICAL",
            DangerousOperation::DropCollection 
            | DangerousOperation::TruncateCollection 
            | DangerousOperation::RestoreBackup => "WARNING",
            _ => "INFO",
        }
    }
}

/// Confirmation token for dangerous operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmationToken {
    /// Operation being confirmed
    pub operation: DangerousOperation,
    /// Resource affected (e.g., collection name)
    pub resource: String,
    /// Timestamp when confirmation was created
    pub created_at: SystemTime,
    /// User ID who requested the operation
    pub user_id: String,
    /// Random token for verification
    pub token: String,
}

impl ConfirmationToken {
    /// Create a new confirmation token
    pub fn new(operation: DangerousOperation, resource: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            operation,
            resource: resource.into(),
            created_at: SystemTime::now(),
            user_id: user_id.into(),
            token: uuid::Uuid::new_v4().to_string(),
        }
    }

    /// Check if the token has expired (default: 5 minutes)
    pub fn is_expired(&self) -> bool {
        self.is_expired_after(std::time::Duration::from_secs(300))
    }

    /// Check if the token has expired after given duration
    pub fn is_expired_after(&self, duration: std::time::Duration) -> bool {
        match self.created_at.elapsed() {
            Ok(elapsed) => elapsed > duration,
            Err(_) => true, // Clock went backwards, treat as expired
        }
    }
}

/// Result of a confirmation check
#[derive(Debug)]
pub enum ConfirmationResult {
    /// Confirmation is valid, proceed
    Confirmed,
    /// Token expired
    Expired,
    /// Token doesn't match
    InvalidToken,
    /// Typed phrase doesn't match
    PhraseNotMatch { expected: String },
    /// Confirmation required but not provided
    NotProvided { warning: String },
}

/// Dangerous operation guard
pub struct DangerousOperationGuard {
    operation: DangerousOperation,
    resource: String,
    user_id: String,
    confirmed: bool,
}

impl DangerousOperationGuard {
    /// Create a new guard for a dangerous operation
    pub fn new(
        operation: DangerousOperation,
        resource: impl Into<String>,
        user_id: impl Into<String>,
    ) -> (Self, ConfirmationToken) {
        let resource = resource.into();
        let user_id = user_id.into();
        let token = ConfirmationToken::new(operation, resource.clone(), user_id.clone());
        
        (
            Self {
                operation,
                resource,
                user_id,
                confirmed: false,
            },
            token,
        )
    }

    /// Confirm with token
    pub fn confirm(&mut self, token: &ConfirmationToken) -> ConfirmationResult {
        if token.is_expired() {
            return ConfirmationResult::Expired;
        }
        
        if token.operation != self.operation || token.resource != self.resource {
            return ConfirmationResult::InvalidToken;
        }
        
        self.confirmed = true;
        ConfirmationResult::Confirmed
    }

    /// Confirm with typed phrase (for operations requiring it)
    pub fn confirm_with_phrase(&mut self, token: &ConfirmationToken, phrase: &str) -> ConfirmationResult {
        // First do basic confirmation
        let result = self.confirm(token);
        if !matches!(result, ConfirmationResult::Confirmed) {
            return result;
        }
        
        // Then check phrase if required
        if self.operation.requires_type_confirm() {
            let expected = self.operation.confirm_phrase(&self.resource);
            if phrase.trim().to_lowercase() != expected.to_lowercase() {
                self.confirmed = false;
                return ConfirmationResult::PhraseNotMatch { expected };
            }
        }
        
        ConfirmationResult::Confirmed
    }

    /// Check if operation is confirmed
    pub fn is_confirmed(&self) -> bool {
        self.confirmed
    }

    /// Get the operation
    pub fn operation(&self) -> DangerousOperation {
        self.operation
    }

    /// Get the resource
    pub fn resource(&self) -> &str {
        &self.resource
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirmation_token_creation() {
        let token = ConfirmationToken::new(
            DangerousOperation::DropCollection,
            "users",
            "admin",
        );
        
        assert_eq!(token.operation, DangerousOperation::DropCollection);
        assert_eq!(token.resource, "users");
        assert!(!token.is_expired());
    }

    #[test]
    fn test_dangerous_operation_requires_phrase() {
        assert!(DangerousOperation::FactoryReset.requires_type_confirm());
        assert!(DangerousOperation::DropCollection.requires_type_confirm());
        assert!(!DangerousOperation::TruncateCollection.requires_type_confirm());
    }

    #[test]
    fn test_guard_confirm() {
        let (mut guard, token) = DangerousOperationGuard::new(
            DangerousOperation::TruncateCollection,
            "posts",
            "admin",
        );
        
        assert!(!guard.is_confirmed());
        
        match guard.confirm(&token) {
            ConfirmationResult::Confirmed => {}
            _ => panic!("Expected Confirmed"),
        }
        
        assert!(guard.is_confirmed());
    }

    #[test]
    fn test_guard_phrase_confirm() {
        let (mut guard, token) = DangerousOperationGuard::new(
            DangerousOperation::DropCollection,
            "users",
            "admin",
        );
        
        // Wrong phrase
        match guard.confirm_with_phrase(&token, "wrong") {
            ConfirmationResult::PhraseNotMatch { .. } => {}
            _ => panic!("Expected PhraseNotMatch"),
        }
        
        // Correct phrase
        match guard.confirm_with_phrase(&token, "drop users") {
            ConfirmationResult::Confirmed => {}
            _ => panic!("Expected Confirmed"),
        }
    }
}
