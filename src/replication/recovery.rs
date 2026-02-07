//! Replication Recovery & Restart Semantics
//!
//! Per REPLICATION_RECOVERY.md:
//! - After crash or restart, node must prove safety
//! - If safety cannot be proven, node must refuse to operate
//! - Recovery prioritizes correctness over availability
//!
//! Per §11: "A node that cannot prove safety must refuse to run."

use super::errors::{ReplicationError, ReplicationResult};
use super::role::{HaltReason, ReplicationRole, ReplicationState};
use crate::mvcc::CommitId;

/// Recovery validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryValidation {
    /// Recovery valid, replication may proceed
    Valid,

    /// WAL integrity check failed
    WalIntegrityFailure,

    /// WAL has gaps
    WalGapDetected,

    /// MVCC state inconsistent
    MvccInconsistency,

    /// Snapshot integrity failure
    SnapshotIntegrityFailure,

    /// Snapshot boundary mismatch with WAL
    SnapshotBoundaryMismatch,

    /// Authority ambiguity detected
    AuthorityAmbiguity,

    /// History divergence detected
    HistoryDivergence,
}

impl RecoveryValidation {
    /// Check if validation passed.
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }

    /// Convert to halt reason if validation failed.
    pub fn to_halt_reason(&self) -> Option<HaltReason> {
        match self {
            Self::Valid => None,
            Self::WalIntegrityFailure => Some(HaltReason::WalCorruption),
            Self::WalGapDetected => Some(HaltReason::WalGapDetected),
            Self::MvccInconsistency => Some(HaltReason::ConfigurationError),
            Self::SnapshotIntegrityFailure => Some(HaltReason::SnapshotIntegrityFailure),
            Self::SnapshotBoundaryMismatch => Some(HaltReason::SnapshotIntegrityFailure),
            Self::AuthorityAmbiguity => Some(HaltReason::AuthorityAmbiguity),
            Self::HistoryDivergence => Some(HaltReason::HistoryDivergence),
        }
    }
}

/// Primary recovery state per §3
#[derive(Debug)]
pub struct PrimaryRecovery {
    /// Last durable CommitId from WAL
    last_durable_commit: Option<CommitId>,

    /// Whether WAL integrity verified
    wal_integrity_verified: bool,

    /// Whether MVCC state verified
    mvcc_state_verified: bool,

    /// Whether recovery completed
    recovery_complete: bool,
}

impl PrimaryRecovery {
    /// Create new Primary recovery state.
    pub fn new() -> Self {
        Self {
            last_durable_commit: None,
            wal_integrity_verified: false,
            mvcc_state_verified: false,
            recovery_complete: false,
        }
    }

    /// Set last durable commit found in WAL.
    pub fn set_last_durable_commit(&mut self, commit_id: CommitId) {
        self.last_durable_commit = Some(commit_id);
    }

    /// Mark WAL integrity as verified.
    pub fn mark_wal_verified(&mut self) {
        self.wal_integrity_verified = true;
    }

    /// Mark MVCC state as verified.
    pub fn mark_mvcc_verified(&mut self) {
        self.mvcc_state_verified = true;
    }

    /// Validate Primary recovery.
    ///
    /// Per §3.1: Primary must verify WAL integrity, completeness, MVCC consistency.
    /// Failure of any check → startup aborts.
    pub fn validate(&self) -> RecoveryValidation {
        if !self.wal_integrity_verified {
            return RecoveryValidation::WalIntegrityFailure;
        }

        if !self.mvcc_state_verified {
            return RecoveryValidation::MvccInconsistency;
        }

        RecoveryValidation::Valid
    }

    /// Complete recovery.
    ///
    /// Per §3.2: Primary reasserts commit authority.
    /// CommitId assignment resumes strictly after last durable CommitId.
    pub fn complete(&mut self) -> ReplicationResult<CommitId> {
        let validation = self.validate();
        if !validation.is_valid() {
            return Err(ReplicationError::halted(format!(
                "Primary recovery failed: {:?}",
                validation
            )));
        }

        self.recovery_complete = true;

        // Return next CommitId (strictly after last durable)
        Ok(self
            .last_durable_commit
            .map(|c| CommitId::new(c.value() + 1))
            .unwrap_or(CommitId::new(1)))
    }

    /// Check if recovery is complete.
    pub fn is_complete(&self) -> bool {
        self.recovery_complete
    }

    /// Get last durable commit.
    pub fn last_durable_commit(&self) -> Option<CommitId> {
        self.last_durable_commit
    }
}

impl Default for PrimaryRecovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Replica recovery state per §4
#[derive(Debug)]
pub struct ReplicaRecovery {
    /// Last applied CommitId
    last_applied_commit: Option<CommitId>,

    /// Expected next WAL sequence
    expected_wal_sequence: u64,

    /// Whether WAL integrity verified
    wal_integrity_verified: bool,

    /// Whether WAL prefix verified
    wal_prefix_verified: bool,

    /// Whether snapshot verified (if present)
    snapshot_verified: Option<bool>,

    /// Snapshot boundary (if present)
    snapshot_boundary: Option<CommitId>,

    /// Whether recovery completed
    recovery_complete: bool,
}

impl ReplicaRecovery {
    /// Create new Replica recovery state.
    pub fn new() -> Self {
        Self {
            last_applied_commit: None,
            expected_wal_sequence: 0,
            wal_integrity_verified: false,
            wal_prefix_verified: false,
            snapshot_verified: None,
            snapshot_boundary: None,
            recovery_complete: false,
        }
    }

    /// Set last applied commit.
    pub fn set_last_applied_commit(&mut self, commit_id: CommitId, wal_sequence: u64) {
        self.last_applied_commit = Some(commit_id);
        self.expected_wal_sequence = wal_sequence + 1;
    }

    /// Mark WAL integrity as verified.
    pub fn mark_wal_verified(&mut self) {
        self.wal_integrity_verified = true;
    }

    /// Mark WAL prefix as verified.
    pub fn mark_prefix_verified(&mut self) {
        self.wal_prefix_verified = true;
    }

    /// Set snapshot recovery state.
    pub fn set_snapshot(&mut self, boundary: CommitId, verified: bool) {
        self.snapshot_boundary = Some(boundary);
        self.snapshot_verified = Some(verified);
    }

    /// Validate Replica recovery.
    ///
    /// Per §4.1: Replica must verify WAL integrity, prefix validity,
    /// snapshot boundary correctness.
    /// Failure → ReplicationHalted.
    pub fn validate(&self) -> RecoveryValidation {
        if !self.wal_integrity_verified {
            return RecoveryValidation::WalIntegrityFailure;
        }

        if !self.wal_prefix_verified {
            return RecoveryValidation::WalGapDetected;
        }

        // Check snapshot if present (§5.1)
        if let Some(verified) = self.snapshot_verified {
            if !verified {
                return RecoveryValidation::SnapshotIntegrityFailure;
            }
        }

        RecoveryValidation::Valid
    }

    /// Complete recovery.
    ///
    /// Per §4.3: Replica may resume WAL replay only if continuity is provable.
    pub fn complete(&mut self) -> ReplicationResult<ReplicaResumeState> {
        let validation = self.validate();
        if !validation.is_valid() {
            return Err(ReplicationError::halted(format!(
                "Replica recovery failed: {:?}",
                validation
            )));
        }

        self.recovery_complete = true;

        Ok(ReplicaResumeState {
            last_applied_commit: self.last_applied_commit,
            expected_wal_sequence: self.expected_wal_sequence,
            snapshot_boundary: self.snapshot_boundary,
        })
    }

    /// Check if recovery is complete.
    pub fn is_complete(&self) -> bool {
        self.recovery_complete
    }

    /// Get expected WAL sequence.
    pub fn expected_wal_sequence(&self) -> u64 {
        self.expected_wal_sequence
    }
}

impl Default for ReplicaRecovery {
    fn default() -> Self {
        Self::new()
    }
}

/// State for resuming replication after Replica recovery
#[derive(Debug, Clone)]
pub struct ReplicaResumeState {
    /// Last applied commit
    pub last_applied_commit: Option<CommitId>,

    /// Expected next WAL sequence
    pub expected_wal_sequence: u64,

    /// Snapshot boundary (if recovery from snapshot)
    pub snapshot_boundary: Option<CommitId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_recovery_valid() {
        let mut recovery = PrimaryRecovery::new();
        recovery.set_last_durable_commit(CommitId::new(100));
        recovery.mark_wal_verified();
        recovery.mark_mvcc_verified();

        assert!(recovery.validate().is_valid());
    }

    #[test]
    fn test_primary_recovery_fails_without_wal_verify() {
        let mut recovery = PrimaryRecovery::new();
        recovery.mark_mvcc_verified();

        let result = recovery.validate();
        assert!(!result.is_valid());
        assert_eq!(result, RecoveryValidation::WalIntegrityFailure);
    }

    #[test]
    fn test_primary_recovery_fails_without_mvcc_verify() {
        let mut recovery = PrimaryRecovery::new();
        recovery.mark_wal_verified();

        let result = recovery.validate();
        assert!(!result.is_valid());
        assert_eq!(result, RecoveryValidation::MvccInconsistency);
    }

    #[test]
    fn test_primary_complete_returns_next_commit_id() {
        // Per §3.2: CommitId assignment resumes strictly after last durable
        let mut recovery = PrimaryRecovery::new();
        recovery.set_last_durable_commit(CommitId::new(50));
        recovery.mark_wal_verified();
        recovery.mark_mvcc_verified();

        let next = recovery.complete().unwrap();
        assert_eq!(next, CommitId::new(51));
        assert!(recovery.is_complete());
    }

    #[test]
    fn test_replica_recovery_valid() {
        let mut recovery = ReplicaRecovery::new();
        recovery.set_last_applied_commit(CommitId::new(100), 99);
        recovery.mark_wal_verified();
        recovery.mark_prefix_verified();

        assert!(recovery.validate().is_valid());
    }

    #[test]
    fn test_replica_recovery_fails_without_prefix_verify() {
        // Per §4.1: WAL prefix validity required
        let mut recovery = ReplicaRecovery::new();
        recovery.mark_wal_verified();
        // No prefix verification

        let result = recovery.validate();
        assert!(!result.is_valid());
        assert_eq!(result, RecoveryValidation::WalGapDetected);
    }

    #[test]
    fn test_replica_recovery_with_snapshot() {
        let mut recovery = ReplicaRecovery::new();
        recovery.set_snapshot(CommitId::new(50), true);
        recovery.mark_wal_verified();
        recovery.mark_prefix_verified();

        assert!(recovery.validate().is_valid());
    }

    #[test]
    fn test_replica_recovery_fails_with_invalid_snapshot() {
        // Per §5.2: Snapshot integrity cannot be proven → discard
        let mut recovery = ReplicaRecovery::new();
        recovery.set_snapshot(CommitId::new(50), false); // Snapshot failed verification
        recovery.mark_wal_verified();
        recovery.mark_prefix_verified();

        let result = recovery.validate();
        assert!(!result.is_valid());
        assert_eq!(result, RecoveryValidation::SnapshotIntegrityFailure);
    }

    #[test]
    fn test_replica_complete_returns_resume_state() {
        let mut recovery = ReplicaRecovery::new();
        recovery.set_last_applied_commit(CommitId::new(100), 99);
        recovery.mark_wal_verified();
        recovery.mark_prefix_verified();

        let resume = recovery.complete().unwrap();
        assert_eq!(resume.last_applied_commit, Some(CommitId::new(100)));
        assert_eq!(resume.expected_wal_sequence, 100);
    }

    #[test]
    fn test_validation_to_halt_reason() {
        assert!(RecoveryValidation::Valid.to_halt_reason().is_none());
        assert_eq!(
            RecoveryValidation::WalGapDetected.to_halt_reason(),
            Some(HaltReason::WalGapDetected)
        );
        assert_eq!(
            RecoveryValidation::HistoryDivergence.to_halt_reason(),
            Some(HaltReason::HistoryDivergence)
        );
    }
}
