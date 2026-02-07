//! Replication Failure Matrix
//!
//! Per REPLICATION_FAILURE_MATRIX.md:
//! - There must be no ambiguous outcomes under failure
//! - Every failure maps to exactly one correct outcome
//! - If replication cannot be proven correct, it must stop
//!
//! This module defines all replication failure points and their required outcomes.

use super::errors::{ReplicationError, ReplicationResult};
use super::role::HaltReason;

/// Replication crash point enumeration per FAILURE_MATRIX.md §2-7
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicationCrashPoint {
    // === Primary-Side Failures (§3) ===
    /// §3.1: Primary crash before WAL commit
    /// Required outcome: Commit does not exist
    PrimaryBeforeWalCommit,

    /// §3.2: Primary crash after WAL commit, before replication
    /// Required outcome: Commit is durable, replicas receive after recovery
    PrimaryAfterWalCommitBeforeReplication,

    /// §3.3: Primary crash during WAL shipping
    /// Required outcome: Replica detects incomplete, halts, resumes explicitly
    PrimaryDuringWalShipping,

    // === Replica-Side Failures (§4) ===
    /// §4.1: Replica crash before WAL append
    /// Required outcome: Record lost, retransmission required
    ReplicaBeforeWalAppend,

    /// §4.2: Replica crash after WAL append
    /// Required outcome: Recovery replays WAL deterministically
    ReplicaAfterWalAppend,

    /// §4.3: Replica crash during snapshot installation
    /// Required outcome: Snapshot discarded, no partial state
    ReplicaDuringSnapshotInstall,

    // === Network Failures (§5) ===
    /// §5.1: Network partition between Primary and Replica
    /// Required outcome: Replica stops, reads may continue if safe
    NetworkPartition,

    /// §5.2: Message duplication or reordering
    /// Required outcome: Replica rejects, replication halts
    NetworkMessageCorruption,

    // === WAL Integrity Failures (§6) ===
    /// §6.1: Corrupted WAL record in transit (checksum mismatch)
    /// Required outcome: Record rejected, replication halts
    WalRecordCorrupted,

    /// §6.2: Missing WAL record (gap)
    /// Required outcome: Replica halts, enters ReplicationHalted
    WalRecordGap,

    // === Snapshot Failures (§7) ===
    /// §7.1: Invalid snapshot transfer
    /// Required outcome: Snapshot rejected, replica unchanged
    SnapshotIntegrityFailure,

    /// §7.2: Snapshot boundary mismatch
    /// Required outcome: Fatal error, replica must not continue
    SnapshotBoundaryMismatch,

    // === Divergence (§8) ===
    /// §8.1: Replica history diverges from Primary
    /// Required outcome: Fatal, manual intervention required
    HistoryDivergence,
}

/// Required outcome for a failure
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureOutcome {
    /// Commit/operation did not happen (as if it never started)
    OperationNeverHappened,

    /// Operation is durable, will be delivered after recovery
    OperationDurable,

    /// Halt replication, explicit resume required
    HaltReplication(HaltReason),

    /// Retransmission required
    RetransmissionRequired,

    /// Deterministic recovery via WAL replay
    DeterministicRecovery,

    /// Snapshot discarded, no side effects
    SnapshotDiscarded,

    /// Fatal error, manual intervention required
    FatalError,
}

impl ReplicationCrashPoint {
    /// Get the required outcome for this crash point.
    ///
    /// Per FAILURE_MATRIX.md: Every failure maps to exactly one correct outcome.
    pub fn required_outcome(&self) -> FailureOutcome {
        match self {
            // Primary-side
            Self::PrimaryBeforeWalCommit => FailureOutcome::OperationNeverHappened,
            Self::PrimaryAfterWalCommitBeforeReplication => FailureOutcome::OperationDurable,
            Self::PrimaryDuringWalShipping => {
                FailureOutcome::HaltReplication(HaltReason::WalGapDetected)
            }

            // Replica-side
            Self::ReplicaBeforeWalAppend => FailureOutcome::RetransmissionRequired,
            Self::ReplicaAfterWalAppend => FailureOutcome::DeterministicRecovery,
            Self::ReplicaDuringSnapshotInstall => FailureOutcome::SnapshotDiscarded,

            // Network
            Self::NetworkPartition => FailureOutcome::HaltReplication(HaltReason::WalGapDetected),
            Self::NetworkMessageCorruption => {
                FailureOutcome::HaltReplication(HaltReason::WalCorruption)
            }

            // WAL integrity
            Self::WalRecordCorrupted => FailureOutcome::HaltReplication(HaltReason::WalCorruption),
            Self::WalRecordGap => FailureOutcome::HaltReplication(HaltReason::WalGapDetected),

            // Snapshot
            Self::SnapshotIntegrityFailure => FailureOutcome::SnapshotDiscarded,
            Self::SnapshotBoundaryMismatch => FailureOutcome::FatalError,

            // Divergence
            Self::HistoryDivergence => FailureOutcome::FatalError,
        }
    }

    /// Check if this crash point is fatal (requires manual intervention).
    pub fn is_fatal(&self) -> bool {
        matches!(self.required_outcome(), FailureOutcome::FatalError)
    }

    /// Check if this crash point halts replication.
    pub fn halts_replication(&self) -> bool {
        matches!(
            self.required_outcome(),
            FailureOutcome::HaltReplication(_) | FailureOutcome::FatalError
        )
    }
}

impl FailureOutcome {
    /// Check if this outcome requires halt.
    pub fn requires_halt(&self) -> bool {
        matches!(self, Self::HaltReplication(_) | Self::FatalError)
    }

    /// Get halt reason if applicable.
    pub fn halt_reason(&self) -> Option<HaltReason> {
        match self {
            Self::HaltReplication(reason) => Some(*reason),
            Self::FatalError => Some(HaltReason::ConfigurationError),
            _ => None,
        }
    }
}

/// Current failure state tracking
#[derive(Debug, Default)]
pub struct FailureState {
    /// Active crash point (if simulating)
    active_crash_point: Option<ReplicationCrashPoint>,

    /// Detected failures
    detected_failures: Vec<ReplicationCrashPoint>,
}

impl FailureState {
    /// Create new failure state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Trigger a crash point (for testing).
    pub fn trigger_crash_point(&mut self, point: ReplicationCrashPoint) {
        self.active_crash_point = Some(point);
    }

    /// Record detected failure.
    pub fn record_failure(&mut self, point: ReplicationCrashPoint) {
        self.detected_failures.push(point);
    }

    /// Check if any fatal failure detected.
    pub fn has_fatal_failure(&self) -> bool {
        self.detected_failures.iter().any(|f| f.is_fatal())
    }

    /// Check if any failure requires halt.
    pub fn requires_halt(&self) -> bool {
        self.detected_failures.iter().any(|f| f.halts_replication())
    }

    /// Get halt reason if any failure requires halt.
    pub fn halt_reason(&self) -> Option<HaltReason> {
        self.detected_failures
            .iter()
            .find(|f| f.halts_replication())
            .and_then(|f| f.required_outcome().halt_reason())
    }

    /// Clear detected failures (after recovery).
    pub fn clear_failures(&mut self) {
        self.detected_failures.clear();
        self.active_crash_point = None;
    }

    /// Get count of detected failures.
    pub fn failure_count(&self) -> usize {
        self.detected_failures.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primary_before_commit_never_happened() {
        // §3.1: Commit does not exist
        let point = ReplicationCrashPoint::PrimaryBeforeWalCommit;
        assert_eq!(
            point.required_outcome(),
            FailureOutcome::OperationNeverHappened
        );
        assert!(!point.is_fatal());
        assert!(!point.halts_replication());
    }

    #[test]
    fn test_primary_after_commit_durable() {
        // §3.2: Commit is durable
        let point = ReplicationCrashPoint::PrimaryAfterWalCommitBeforeReplication;
        assert_eq!(point.required_outcome(), FailureOutcome::OperationDurable);
    }

    #[test]
    fn test_wal_gap_halts_replication() {
        // §6.2: Replica halts, enters ReplicationHalted
        let point = ReplicationCrashPoint::WalRecordGap;
        assert!(point.halts_replication());
        assert!(!point.is_fatal());

        let outcome = point.required_outcome();
        assert!(outcome.requires_halt());
        assert_eq!(outcome.halt_reason(), Some(HaltReason::WalGapDetected));
    }

    #[test]
    fn test_divergence_is_fatal() {
        // §8.1: Fatal, manual intervention required
        let point = ReplicationCrashPoint::HistoryDivergence;
        assert!(point.is_fatal());
        assert!(point.halts_replication());
    }

    #[test]
    fn test_snapshot_boundary_mismatch_fatal() {
        // §7.2: Fatal error, replica must not continue
        let point = ReplicationCrashPoint::SnapshotBoundaryMismatch;
        assert!(point.is_fatal());
        assert_eq!(point.required_outcome(), FailureOutcome::FatalError);
    }

    #[test]
    fn test_replica_after_wal_append_recovers() {
        // §4.2: Recovery replays WAL deterministically
        let point = ReplicationCrashPoint::ReplicaAfterWalAppend;
        assert_eq!(
            point.required_outcome(),
            FailureOutcome::DeterministicRecovery
        );
        assert!(!point.halts_replication());
    }

    #[test]
    fn test_snapshot_crash_discards() {
        // §4.3: Snapshot discarded, no partial state
        let point = ReplicationCrashPoint::ReplicaDuringSnapshotInstall;
        assert_eq!(point.required_outcome(), FailureOutcome::SnapshotDiscarded);
    }

    #[test]
    fn test_failure_state_tracking() {
        let mut state = FailureState::new();
        assert!(!state.has_fatal_failure());
        assert!(!state.requires_halt());

        state.record_failure(ReplicationCrashPoint::WalRecordGap);
        assert!(state.requires_halt());
        assert!(!state.has_fatal_failure());

        state.record_failure(ReplicationCrashPoint::HistoryDivergence);
        assert!(state.has_fatal_failure());

        assert_eq!(state.failure_count(), 2);
    }

    #[test]
    fn test_failure_state_clear() {
        let mut state = FailureState::new();
        state.record_failure(ReplicationCrashPoint::WalRecordGap);
        assert!(state.requires_halt());

        state.clear_failures();
        assert!(!state.requires_halt());
        assert_eq!(state.failure_count(), 0);
    }
}
