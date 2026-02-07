//! Replica Read Semantics
//!
//! Per REPLICATION_READ_SEMANTICS.md §2:
//! > A Replica may serve a read if and only if it can prove that the result
//! > is identical to what the Primary would return for the same read view.
//!
//! Per §4: Eligibility requires:
//! - Replica is in ReplicaActive state
//! - Applied WAL ends at C_replica
//! - Requested read view R satisfies: R.read_upper_bound ≤ C_replica
//! - All MVCC metadata exists locally
//! - No WAL gaps or replication errors
//!
//! Per §11: "A Replica may lag. It may never lie."

use super::errors::{ReplicationError, ReplicationResult};
use super::role::ReplicationState;
use super::wal_receiver::WalReceiver;
use crate::mvcc::CommitId;

/// Replica read eligibility result per §4
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadEligibility {
    /// Read is eligible to be served
    Eligible,

    /// Not in ReplicaActive state
    NotReplicaActive,

    /// Read boundary exceeds replica's applied WAL
    BoundaryExceedsApplied {
        requested: CommitId,
        applied: CommitId,
    },

    /// Replication is halted
    ReplicationHalted,

    /// WAL gap detected
    WalGapDetected,

    /// Snapshot installation incomplete
    SnapshotIncomplete,

    /// Replica is mid-recovery
    MidRecovery,
}

impl ReadEligibility {
    /// Check if read is eligible.
    pub fn is_eligible(&self) -> bool {
        matches!(self, Self::Eligible)
    }

    /// Convert to result.
    pub fn to_result(&self) -> ReplicationResult<()> {
        match self {
            Self::Eligible => Ok(()),
            Self::NotReplicaActive => Err(ReplicationError::read_rejected(
                "cannot serve reads when not in ReplicaActive state",
            )),
            Self::BoundaryExceedsApplied { requested, applied } => {
                Err(ReplicationError::read_rejected(format!(
                    "read boundary {} exceeds applied WAL boundary {}",
                    requested.value(),
                    applied.value()
                )))
            }
            Self::ReplicationHalted => Err(ReplicationError::read_rejected(
                "cannot serve reads when replication is halted",
            )),
            Self::WalGapDetected => Err(ReplicationError::read_rejected(
                "cannot serve reads with WAL gap detected",
            )),
            Self::SnapshotIncomplete => Err(ReplicationError::read_rejected(
                "cannot serve reads during snapshot installation",
            )),
            Self::MidRecovery => Err(ReplicationError::read_rejected(
                "cannot serve reads during recovery",
            )),
        }
    }
}

/// Replica read admission state
#[derive(Debug)]
pub struct ReplicaReadAdmission {
    /// Current applied commit boundary
    applied_commit_boundary: CommitId,

    /// Whether WAL receiver is healthy
    wal_receiver_healthy: bool,

    /// Whether snapshot is complete
    snapshot_complete: bool,

    /// Whether recovery is complete
    recovery_complete: bool,
}

impl ReplicaReadAdmission {
    /// Create new admission state.
    pub fn new(applied_commit_boundary: CommitId) -> Self {
        Self {
            applied_commit_boundary,
            wal_receiver_healthy: true,
            snapshot_complete: true,
            recovery_complete: true,
        }
    }

    /// Update applied commit boundary.
    pub fn update_boundary(&mut self, boundary: CommitId) {
        self.applied_commit_boundary = boundary;
    }

    /// Mark WAL receiver as unhealthy (gap detected).
    pub fn mark_wal_gap(&mut self) {
        self.wal_receiver_healthy = false;
    }

    /// Mark snapshot as incomplete.
    pub fn mark_snapshot_incomplete(&mut self) {
        self.snapshot_complete = false;
    }

    /// Mark snapshot as complete.
    pub fn mark_snapshot_complete(&mut self) {
        self.snapshot_complete = true;
    }

    /// Mark recovery as in progress.
    pub fn mark_recovery_in_progress(&mut self) {
        self.recovery_complete = false;
    }

    /// Mark recovery as complete.
    pub fn mark_recovery_complete(&mut self) {
        self.recovery_complete = true;
    }

    /// Get applied commit boundary.
    pub fn applied_commit_boundary(&self) -> CommitId {
        self.applied_commit_boundary
    }

    /// Check read eligibility.
    ///
    /// Per §4: Read is eligible iff:
    /// - State is ReplicaActive
    /// - R.read_upper_bound ≤ C_replica
    /// - All MVCC metadata exists
    /// - No WAL gaps
    pub fn check_eligibility(
        &self,
        state: &ReplicationState,
        requested_boundary: CommitId,
    ) -> ReadEligibility {
        // Per §8: Must not be halted (check first since halted isn't a valid "replica" state)
        if state.is_halted() {
            return ReadEligibility::ReplicationHalted;
        }

        // Per §4.1: Must be in ReplicaActive
        if !state.is_replica() {
            return ReadEligibility::NotReplicaActive;
        }

        // Per §8: No WAL gaps
        if !self.wal_receiver_healthy {
            return ReadEligibility::WalGapDetected;
        }

        // Per §8: Snapshot must be complete
        if !self.snapshot_complete {
            return ReadEligibility::SnapshotIncomplete;
        }

        // Per §9: Not mid-recovery
        if !self.recovery_complete {
            return ReadEligibility::MidRecovery;
        }

        // Per §4.3: R.read_upper_bound ≤ C_replica
        if requested_boundary > self.applied_commit_boundary {
            return ReadEligibility::BoundaryExceedsApplied {
                requested: requested_boundary,
                applied: self.applied_commit_boundary,
            };
        }

        ReadEligibility::Eligible
    }

    /// Create a safe read view boundary.
    ///
    /// Per §5.1: Replica read views use read_upper_bound = C_replica
    pub fn safe_read_boundary(&self) -> CommitId {
        self.applied_commit_boundary
    }
}

impl Default for ReplicaReadAdmission {
    fn default() -> Self {
        Self::new(CommitId::new(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eligible_when_boundary_within_applied() {
        // Per §4: R.read_upper_bound ≤ C_replica → eligible
        let admission = ReplicaReadAdmission::new(CommitId::new(100));
        let replica_id = uuid::Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };

        let result = admission.check_eligibility(&state, CommitId::new(50));
        assert!(result.is_eligible());
    }

    #[test]
    fn test_eligible_at_exact_boundary() {
        // Per §4: R.read_upper_bound ≤ C_replica → eligible (equality case)
        let admission = ReplicaReadAdmission::new(CommitId::new(100));
        let replica_id = uuid::Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };

        let result = admission.check_eligibility(&state, CommitId::new(100));
        assert!(result.is_eligible());
    }

    #[test]
    fn test_ineligible_beyond_boundary() {
        // Per §4: R.read_upper_bound > C_replica → ineligible
        let admission = ReplicaReadAdmission::new(CommitId::new(100));
        let replica_id = uuid::Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };

        let result = admission.check_eligibility(&state, CommitId::new(150));
        assert!(!result.is_eligible());

        match result {
            ReadEligibility::BoundaryExceedsApplied { requested, applied } => {
                assert_eq!(requested, CommitId::new(150));
                assert_eq!(applied, CommitId::new(100));
            }
            _ => panic!("expected BoundaryExceedsApplied"),
        }
    }

    #[test]
    fn test_ineligible_when_not_replica_active() {
        // Per §4.1: Must be ReplicaActive
        let admission = ReplicaReadAdmission::new(CommitId::new(100));

        // Test with Primary
        let primary = ReplicationState::PrimaryActive;
        assert!(!admission
            .check_eligibility(&primary, CommitId::new(50))
            .is_eligible());

        // Test with Uninitialized
        let uninit = ReplicationState::Uninitialized;
        assert!(!admission
            .check_eligibility(&uninit, CommitId::new(50))
            .is_eligible());
    }

    #[test]
    fn test_ineligible_when_halted() {
        // Per §8: Replication halted → refuse reads
        use super::super::role::HaltReason;

        let admission = ReplicaReadAdmission::new(CommitId::new(100));
        let halted = ReplicationState::ReplicationHalted {
            reason: HaltReason::WalGapDetected,
        };

        let result = admission.check_eligibility(&halted, CommitId::new(50));
        assert_eq!(result, ReadEligibility::ReplicationHalted);
    }

    #[test]
    fn test_ineligible_with_wal_gap() {
        // Per §8: WAL gaps → refuse reads
        let mut admission = ReplicaReadAdmission::new(CommitId::new(100));
        admission.mark_wal_gap();

        let replica_id = uuid::Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };
        let result = admission.check_eligibility(&state, CommitId::new(50));
        assert_eq!(result, ReadEligibility::WalGapDetected);
    }

    #[test]
    fn test_ineligible_during_snapshot() {
        // Per §8: Snapshot incomplete → refuse reads
        let mut admission = ReplicaReadAdmission::new(CommitId::new(100));
        admission.mark_snapshot_incomplete();

        let replica_id = uuid::Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };
        let result = admission.check_eligibility(&state, CommitId::new(50));
        assert_eq!(result, ReadEligibility::SnapshotIncomplete);
    }

    #[test]
    fn test_ineligible_during_recovery() {
        // Per §9: Mid-recovery → refuse reads
        let mut admission = ReplicaReadAdmission::new(CommitId::new(100));
        admission.mark_recovery_in_progress();

        let replica_id = uuid::Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };
        let result = admission.check_eligibility(&state, CommitId::new(50));
        assert_eq!(result, ReadEligibility::MidRecovery);
    }

    #[test]
    fn test_safe_read_boundary() {
        // Per §5.1: read_upper_bound = C_replica
        let admission = ReplicaReadAdmission::new(CommitId::new(100));
        assert_eq!(admission.safe_read_boundary(), CommitId::new(100));
    }

    #[test]
    fn test_update_boundary_extends_eligibility() {
        let mut admission = ReplicaReadAdmission::new(CommitId::new(100));
        let replica_id = uuid::Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };

        // Initially, 150 is beyond boundary
        assert!(!admission
            .check_eligibility(&state, CommitId::new(150))
            .is_eligible());

        // Update boundary
        admission.update_boundary(CommitId::new(200));

        // Now 150 is within boundary
        assert!(admission
            .check_eligibility(&state, CommitId::new(150))
            .is_eligible());
    }
}
