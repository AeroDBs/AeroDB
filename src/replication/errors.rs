//! Replication Error Types
//!
//! Per PHASE2_REPLICATION_INVARIANTS.md:
//! - All invariant violations are fatal
//! - No automatic healing
//! - Explicit failure on uncertainty

use std::fmt;

/// Replication error type
#[derive(Debug, Clone)]
pub struct ReplicationError {
    /// Error kind
    pub kind: ReplicationErrorKind,
    /// Error message
    pub message: String,
}

/// Replication error kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicationErrorKind {
    /// Illegal state transition attempted
    IllegalTransition,

    /// System is halted, requires operator intervention
    Halted,

    /// Write rejected (not Primary or halted)
    WriteRejected,

    /// Read rejected (halted state)
    ReadRejected,

    /// Authority ambiguity detected
    AuthorityAmbiguity,

    /// Commit authority violation (Replica tried to commit)
    CommitAuthorityViolation,

    /// WAL gap detected
    WalGap,

    /// WAL integrity check failed
    WalIntegrity,

    /// History divergence detected
    HistoryDivergence,

    /// Configuration error
    ConfigurationError,
}

impl ReplicationError {
    /// Create a new replication error.
    pub fn new(kind: ReplicationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Create an illegal transition error.
    pub fn illegal_transition(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::IllegalTransition, message)
    }

    /// Create a halted error.
    pub fn halted(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::Halted, message)
    }

    /// Create a write rejected error.
    pub fn write_rejected(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::WriteRejected, message)
    }

    /// Create a read rejected error.
    pub fn read_rejected(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::ReadRejected, message)
    }

    /// Create an authority ambiguity error.
    pub fn authority_ambiguity(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::AuthorityAmbiguity, message)
    }

    /// Create a commit authority violation error.
    pub fn commit_authority_violation(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::CommitAuthorityViolation, message)
    }

    /// Create a WAL gap error.
    pub fn wal_gap(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::WalGap, message)
    }

    /// Create a WAL integrity error.
    pub fn wal_integrity_failed(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::WalIntegrity, message)
    }

    /// Create a history divergence error.
    pub fn history_divergence(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::HistoryDivergence, message)
    }

    /// Create a configuration error.
    pub fn configuration_error(message: impl Into<String>) -> Self {
        Self::new(ReplicationErrorKind::ConfigurationError, message)
    }

    /// Check if this error is fatal (requires operator intervention).
    pub fn is_fatal(&self) -> bool {
        matches!(
            self.kind,
            ReplicationErrorKind::Halted
                | ReplicationErrorKind::AuthorityAmbiguity
                | ReplicationErrorKind::HistoryDivergence
                | ReplicationErrorKind::WalGap
        )
    }
}

impl fmt::Display for ReplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReplicationError({:?}): {}", self.kind, self.message)
    }
}

impl std::error::Error for ReplicationError {}

/// Result type for replication operations
pub type ReplicationResult<T> = Result<T, ReplicationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fatal_errors() {
        assert!(ReplicationError::halted("test").is_fatal());
        assert!(ReplicationError::authority_ambiguity("test").is_fatal());
        assert!(ReplicationError::history_divergence("test").is_fatal());
        assert!(ReplicationError::wal_gap("test").is_fatal());
    }

    #[test]
    fn test_non_fatal_errors() {
        assert!(!ReplicationError::write_rejected("test").is_fatal());
        assert!(!ReplicationError::illegal_transition("test").is_fatal());
    }
}
