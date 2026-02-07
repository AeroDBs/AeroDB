//! MVCC Version Storage - Commit-bound version persistence
//!
//! Per MVCC_WAL_INTERACTION.md and PHASE2_INVARIANTS.md:
//! - A version exists if and only if its commit identity exists durably
//! - Partial version persistence is detectable and fatal
//! - Recovery cross-validates WAL commits and storage versions
//!
//! This module provides:
//! - MvccVersionStore - Version persistence with atomicity enforcement
//! - Recovery validation - Cross-validation of commits and versions

use std::collections::{HashMap, HashSet};
use std::io;

use crate::mvcc::CommitId;

/// Error types for version storage operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionStorageError {
    /// Commit identity exists but its versions are missing
    /// Per atomicity Rule A: This is corruption
    MissingVersion { commit_id: u64, key: String },
    /// Version exists without a valid commit identity
    /// This should never happen - indicates WAL/storage desync
    OrphanVersion { version_commit_id: u64, key: String },
    /// Version references a commit ID that doesn't match expected
    CommitMismatch {
        key: String,
        expected_commit: u64,
        found_commit: u64,
    },
    /// Partial write detected during recovery
    PartialWrite { key: String, description: String },
    /// I/O error
    IoError(String),
}

impl std::fmt::Display for VersionStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionStorageError::MissingVersion { commit_id, key } => {
                write!(
                    f,
                    "FATAL: Missing version for commit {} key '{}'",
                    commit_id, key
                )
            }
            VersionStorageError::OrphanVersion {
                version_commit_id,
                key,
            } => {
                write!(
                    f,
                    "FATAL: Orphan version with commit {} key '{}' - no matching commit record",
                    version_commit_id, key
                )
            }
            VersionStorageError::CommitMismatch {
                key,
                expected_commit,
                found_commit,
            } => {
                write!(
                    f,
                    "FATAL: Commit mismatch for key '{}': expected {}, found {}",
                    key, expected_commit, found_commit
                )
            }
            VersionStorageError::PartialWrite { key, description } => {
                write!(
                    f,
                    "FATAL: Partial write detected for key '{}': {}",
                    key, description
                )
            }
            VersionStorageError::IoError(msg) => {
                write!(f, "I/O error: {}", msg)
            }
        }
    }
}

impl std::error::Error for VersionStorageError {}

impl From<io::Error> for VersionStorageError {
    fn from(e: io::Error) -> Self {
        VersionStorageError::IoError(e.to_string())
    }
}

/// Result type for version storage operations
pub type VersionStorageResult<T> = Result<T, VersionStorageError>;

/// A persisted version record with commit binding
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedVersion {
    /// The commit identity this version belongs to
    pub commit_id: CommitId,
    /// Document key
    pub key: String,
    /// Whether this is a tombstone
    pub is_tombstone: bool,
    /// Document payload (empty for tombstones)
    pub payload: Vec<u8>,
}

impl PersistedVersion {
    /// Create a new persisted version
    pub fn new(commit_id: CommitId, key: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            commit_id,
            key: key.into(),
            is_tombstone: false,
            payload,
        }
    }

    /// Create a tombstone version
    pub fn tombstone(commit_id: CommitId, key: impl Into<String>) -> Self {
        Self {
            commit_id,
            key: key.into(),
            is_tombstone: true,
            payload: Vec::new(),
        }
    }
}

/// Tracks versions expected from WAL for cross-validation
#[derive(Debug, Default)]
pub struct VersionExpectations {
    /// Map of commit_id -> set of expected version keys
    expected_versions: HashMap<u64, HashSet<String>>,
    /// Set of observed commits from WAL
    observed_commits: HashSet<u64>,
}

impl VersionExpectations {
    /// Create new expectations tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a commit identity from WAL
    pub fn observe_commit(&mut self, commit_id: u64) {
        self.observed_commits.insert(commit_id);
    }

    /// Record an expected version from WAL
    pub fn expect_version(&mut self, commit_id: u64, key: String) {
        self.expected_versions
            .entry(commit_id)
            .or_default()
            .insert(key);
    }

    /// Check if a commit was observed
    pub fn has_commit(&self, commit_id: u64) -> bool {
        self.observed_commits.contains(&commit_id)
    }

    /// Get expected versions for a commit
    pub fn expected_for_commit(&self, commit_id: u64) -> Option<&HashSet<String>> {
        self.expected_versions.get(&commit_id)
    }

    /// Get all observed commits
    pub fn all_commits(&self) -> &HashSet<u64> {
        &self.observed_commits
    }
}

/// Cross-validates WAL expectations against observed versions
///
/// Per MVCC_WAL_INTERACTION.md and MVCC_FAILURE_MATRIX.md:
/// - Every committed version must exist in storage
/// - No version may exist without commit identity
/// - Recovery must halt on any violation
pub struct VersionValidator {
    /// Expected versions from WAL
    expectations: VersionExpectations,
    /// Actually observed versions in storage
    observed_versions: HashMap<u64, HashSet<String>>,
}

impl VersionValidator {
    /// Create validator from WAL expectations
    pub fn new(expectations: VersionExpectations) -> Self {
        Self {
            expectations,
            observed_versions: HashMap::new(),
        }
    }

    /// Record a version observed in storage
    pub fn observe_stored_version(&mut self, commit_id: u64, key: String) {
        self.observed_versions
            .entry(commit_id)
            .or_default()
            .insert(key);
    }

    /// Validate consistency between WAL and storage
    ///
    /// Returns list of all violations (should be empty for valid state)
    pub fn validate(&self) -> Vec<VersionStorageError> {
        let mut errors = Vec::new();

        // Check for missing versions (commit exists but version doesn't)
        for commit_id in self.expectations.all_commits() {
            if let Some(expected_keys) = self.expectations.expected_for_commit(*commit_id) {
                let observed_keys = self.observed_versions.get(commit_id);

                for key in expected_keys {
                    let has_version = observed_keys
                        .map(|keys| keys.contains(key))
                        .unwrap_or(false);

                    if !has_version {
                        errors.push(VersionStorageError::MissingVersion {
                            commit_id: *commit_id,
                            key: key.clone(),
                        });
                    }
                }
            }
        }

        // Check for orphan versions (version exists but commit doesn't)
        for (commit_id, keys) in &self.observed_versions {
            if !self.expectations.has_commit(*commit_id) {
                for key in keys {
                    errors.push(VersionStorageError::OrphanVersion {
                        version_commit_id: *commit_id,
                        key: key.clone(),
                    });
                }
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persisted_version_creation() {
        let version = PersistedVersion::new(CommitId::new(1), "users:user_1", b"data".to_vec());
        assert_eq!(version.commit_id, CommitId::new(1));
        assert_eq!(version.key, "users:user_1");
        assert!(!version.is_tombstone);
    }

    #[test]
    fn test_persisted_tombstone_creation() {
        let version = PersistedVersion::tombstone(CommitId::new(1), "users:user_1");
        assert!(version.is_tombstone);
        assert!(version.payload.is_empty());
    }

    #[test]
    fn test_expectations_tracking() {
        let mut exp = VersionExpectations::new();
        exp.observe_commit(1);
        exp.observe_commit(2);
        exp.expect_version(1, "key_a".to_string());
        exp.expect_version(1, "key_b".to_string());
        exp.expect_version(2, "key_c".to_string());

        assert!(exp.has_commit(1));
        assert!(exp.has_commit(2));
        assert!(!exp.has_commit(3));

        let v1 = exp.expected_for_commit(1).unwrap();
        assert!(v1.contains("key_a"));
        assert!(v1.contains("key_b"));
    }

    #[test]
    fn test_validator_detects_missing_version() {
        let mut exp = VersionExpectations::new();
        exp.observe_commit(1);
        exp.expect_version(1, "key_a".to_string());

        let validator = VersionValidator::new(exp);
        // No versions observed

        let errors = validator.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            VersionStorageError::MissingVersion { commit_id: 1, key } if key == "key_a"
        ));
    }

    #[test]
    fn test_validator_detects_orphan_version() {
        let exp = VersionExpectations::new();
        // No commits observed

        let mut validator = VersionValidator::new(exp);
        validator.observe_stored_version(99, "orphan_key".to_string());

        let errors = validator.validate();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            VersionStorageError::OrphanVersion { version_commit_id: 99, key } if key == "orphan_key"
        ));
    }

    #[test]
    fn test_validator_passes_consistent_state() {
        let mut exp = VersionExpectations::new();
        exp.observe_commit(1);
        exp.observe_commit(2);
        exp.expect_version(1, "key_a".to_string());
        exp.expect_version(2, "key_b".to_string());

        let mut validator = VersionValidator::new(exp);
        validator.observe_stored_version(1, "key_a".to_string());
        validator.observe_stored_version(2, "key_b".to_string());

        let errors = validator.validate();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_error_display() {
        let err = VersionStorageError::MissingVersion {
            commit_id: 42,
            key: "test_key".to_string(),
        };
        assert!(err.to_string().contains("FATAL"));
        assert!(err.to_string().contains("42"));
        assert!(err.to_string().contains("test_key"));
    }
}
