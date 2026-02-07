//! Authority Transition Manager
//!
//! Per PHASE6_ARCHITECTURE.md §3.3:
//! - Applies an approved authority transition
//! - Ensures atomicity of authority change
//! - Integrates with existing replication state
//!
//! Constraints:
//! - Must not overlap authority
//! - Must be crash-safe
//! - Must leave recovery unambiguous
//!
//! Per PHASE6_INVARIANTS.md §P6-A2:
//! Promotion MUST be atomic with respect to authority.
//! All-or-nothing. No intermediate state where two nodes are writable.
//!
//! Per PHASE6_ARCHITECTURE.md §4.2 (amended):
//! Uses fsynced marker file for durable authority transition.

use super::errors::{PromotionError, PromotionErrorKind, PromotionResult};
use super::marker::{AuthorityMarker, DurableMarker};
use crate::replication::ReplicationState;
use std::path::Path;
use uuid::Uuid;

/// Result of an authority transition.
///
/// Per PHASE6_STATE_MACHINE.md §8:
/// - AuthorityTransitioning → Atomic outcome enforced
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionResult {
    /// Transition completed successfully.
    /// Authority has been atomically transferred.
    Completed { new_primary_id: Uuid },

    /// Transition failed and was rolled back.
    /// No authority change occurred.
    Failed { reason: TransitionFailureReason },
}

impl TransitionResult {
    /// Check if transition completed successfully.
    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }
}

/// Reason for transition failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionFailureReason {
    /// Replica is not in correct state for transition.
    InvalidReplicaState,

    /// Authority rebinding failed.
    RebindFailed,

    /// Transition was interrupted (crash recovery).
    Interrupted,

    /// Durable marker write failed.
    MarkerWriteFailed,
}

/// Authority Transition Manager
///
/// Per PHASE6_ARCHITECTURE.md §3.3:
/// - Applies an approved authority transition
/// - Ensures atomicity of authority change
/// - Integrates with existing replication state
///
/// Per PHASE6_INVARIANTS.md §P6-F2:
/// Crash-safe via durable marker file.
///
/// This component performs state transition ONLY after validation succeeds.
pub struct AuthorityTransitionManager {
    /// Whether a transition is currently in progress
    transition_in_progress: bool,

    /// ID of replica being promoted (if transition in progress)
    promoting_replica_id: Option<Uuid>,

    /// Durable marker for crash-safe atomicity
    durable_marker: DurableMarker,
}

impl AuthorityTransitionManager {
    /// Create a new authority transition manager.
    ///
    /// # Arguments
    /// * `data_dir` - Base data directory for marker file storage
    pub fn new(data_dir: &Path) -> Self {
        Self {
            transition_in_progress: false,
            promoting_replica_id: None,
            durable_marker: DurableMarker::new(data_dir),
        }
    }

    /// Create for testing with a temp directory.
    #[cfg(test)]
    pub fn new_for_testing(data_dir: &Path) -> Self {
        Self::new(data_dir)
    }

    /// Check if a transition is in progress.
    pub fn is_transition_in_progress(&self) -> bool {
        self.transition_in_progress
    }

    /// Begin authority transition.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-A2:
    /// Authority transfer is atomic. All-or-nothing.
    ///
    /// Per PHASE6_FAILURE_MODEL.md §3.4:
    /// Crash during authority transition → Atomic outcome enforced.
    /// Recovery observes either old or new authority, never mixed.
    pub fn begin_transition(
        &mut self,
        replica_id: Uuid,
        current_state: &ReplicationState,
    ) -> PromotionResult<()> {
        // Validate replica state
        match current_state {
            ReplicationState::ReplicaActive { replica_id: rid } => {
                if *rid != replica_id {
                    return Err(PromotionError::new(
                        PromotionErrorKind::InvalidReplica,
                        "replica ID mismatch",
                    ));
                }
            }
            _ => {
                return Err(PromotionError::new(
                    PromotionErrorKind::AuthorityTransitionFailed,
                    "replica not in ReplicaActive state",
                ));
            }
        }

        if self.transition_in_progress {
            return Err(PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                "transition already in progress",
            ));
        }

        // Mark transition as in progress
        self.transition_in_progress = true;
        self.promoting_replica_id = Some(replica_id);

        Ok(())
    }

    /// Apply the authority transition atomically.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-A2:
    /// No intermediate state where two nodes are writable.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-F2:
    /// After promotion completes → new authority is authoritative.
    ///
    /// Per PHASE6_ARCHITECTURE.md §4.2:
    /// Uses durable marker file for crash-safe atomicity.
    ///
    /// Returns the new ReplicationState for the promoted replica.
    pub fn apply_transition(&mut self) -> PromotionResult<ReplicationState> {
        if !self.transition_in_progress {
            return Err(PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                "no transition in progress",
            ));
        }

        let replica_id = self.promoting_replica_id.ok_or_else(|| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                "no replica ID for transition",
            )
        })?;

        // Write durable marker - CRITICAL per P6-F2
        // This is the point of no return
        let marker = AuthorityMarker::new(replica_id, "ReplicaActive");
        self.durable_marker.write_atomic(&marker)?;

        // Authority rebinding complete
        // The replica is now PrimaryActive
        let new_state = ReplicationState::PrimaryActive;

        // Clear in-memory transition state
        self.transition_in_progress = false;

        Ok(new_state)
    }

    /// Complete the transition and return the new primary ID.
    pub fn complete_transition(&mut self) -> PromotionResult<Uuid> {
        let replica_id = self.promoting_replica_id.take().ok_or_else(|| {
            PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                "no replica ID for transition",
            )
        })?;

        // Remove marker after successful completion
        // The node is now operating as Primary
        self.durable_marker.remove()?;

        Ok(replica_id)
    }

    /// Abort a transition that hasn't been applied atomically yet.
    ///
    /// Per PHASE6_FAILURE_MODEL.md §3.3:
    /// Failure before authority transition → Promotion is NOT applied.
    pub fn abort_transition(&mut self) -> PromotionResult<()> {
        // Check if marker exists - if so, cannot abort
        if self.durable_marker.exists() {
            // Cannot abort after durable marker is written
            return Err(PromotionError::new(
                PromotionErrorKind::AuthorityTransitionFailed,
                "cannot abort after durable marker is committed",
            ));
        }

        self.transition_in_progress = false;
        self.promoting_replica_id = None;

        Ok(())
    }

    /// Recover authority state after crash.
    ///
    /// Per PHASE6_FAILURE_MODEL.md §3.4:
    /// Recovery MUST observe either old or new authority state.
    /// Never a mixed or ambiguous state.
    ///
    /// Per PHASE6_STATE_MACHINE.md §8:
    /// AuthorityTransitioning → Atomic outcome enforced.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-D2:
    /// Recovery reads ONLY durable state (marker file).
    pub fn recover_after_crash(&self) -> PromotionResult<(bool, Option<Uuid>)> {
        // Read marker from disk - the SOLE source of truth
        match self.durable_marker.read()? {
            Some(marker) => {
                // Marker exists → transition was committed
                // New authority is authoritative
                Ok((true, marker.get_primary_id()))
            }
            None => {
                // Marker absent → transition was not committed
                // Old authority remains
                Ok((false, None))
            }
        }
    }

    /// Check if durable marker exists (for recovery logic).
    pub fn has_durable_marker(&self) -> bool {
        self.durable_marker.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    fn make_manager() -> (TempDir, AuthorityTransitionManager) {
        let tmp = TempDir::new().unwrap();
        let manager = AuthorityTransitionManager::new(tmp.path());
        (tmp, manager)
    }

    #[test]
    fn test_manager_starts_idle() {
        let (_tmp, manager) = make_manager();
        assert!(!manager.is_transition_in_progress());
    }

    #[test]
    fn test_begin_transition_valid_replica() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        let result = manager.begin_transition(replica_id, &state);
        assert!(result.is_ok());
        assert!(manager.is_transition_in_progress());
    }

    #[test]
    fn test_begin_transition_wrong_replica_id() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let other_id = test_uuid();
        let state = ReplicationState::ReplicaActive {
            replica_id: other_id,
        };

        let result = manager.begin_transition(replica_id, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_begin_transition_invalid_state() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::Disabled;

        let result = manager.begin_transition(replica_id, &state);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_transition_returns_primary_active() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        manager.begin_transition(replica_id, &state).unwrap();

        let new_state = manager.apply_transition().unwrap();
        assert_eq!(new_state, ReplicationState::PrimaryActive);

        // Marker should exist after apply
        assert!(manager.has_durable_marker());
    }

    #[test]
    fn test_complete_transition_returns_replica_id() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        manager.begin_transition(replica_id, &state).unwrap();
        manager.apply_transition().unwrap();

        let new_primary_id = manager.complete_transition().unwrap();
        assert_eq!(new_primary_id, replica_id);

        // Marker should be removed after complete
        assert!(!manager.has_durable_marker());
    }

    #[test]
    fn test_abort_transition_before_apply() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        manager.begin_transition(replica_id, &state).unwrap();

        let result = manager.abort_transition();
        assert!(result.is_ok());
        assert!(!manager.is_transition_in_progress());
    }

    #[test]
    fn test_cannot_abort_after_apply() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        manager.begin_transition(replica_id, &state).unwrap();
        manager.apply_transition().unwrap();

        let result = manager.abort_transition();
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_before_marker() {
        let (_tmp, manager) = make_manager();

        let (was_committed, id) = manager.recover_after_crash().unwrap();
        assert!(!was_committed);
        assert!(id.is_none());
    }

    #[test]
    fn test_recovery_after_marker() {
        let (tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        manager.begin_transition(replica_id, &state).unwrap();
        manager.apply_transition().unwrap();

        // Simulate crash and restart with new manager
        let new_manager = AuthorityTransitionManager::new(tmp.path());

        let (was_committed, id) = new_manager.recover_after_crash().unwrap();
        assert!(was_committed);
        assert_eq!(id, Some(replica_id));
    }

    #[test]
    fn test_full_transition_lifecycle() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        // Begin
        manager.begin_transition(replica_id, &state).unwrap();
        assert!(manager.is_transition_in_progress());

        // Apply (atomic)
        let new_state = manager.apply_transition().unwrap();
        assert_eq!(new_state, ReplicationState::PrimaryActive);
        assert!(manager.has_durable_marker());

        // Complete
        let new_primary_id = manager.complete_transition().unwrap();
        assert_eq!(new_primary_id, replica_id);

        // Manager is now idle, marker removed
        assert!(!manager.is_transition_in_progress());
        assert!(!manager.has_durable_marker());
    }

    #[test]
    fn test_recovery_determinism() {
        // Per P6-D2: Same disk state → same recovery outcome
        let (tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        manager.begin_transition(replica_id, &state).unwrap();
        manager.apply_transition().unwrap();

        // Multiple recoveries should return identical results
        let mgr1 = AuthorityTransitionManager::new(tmp.path());
        let mgr2 = AuthorityTransitionManager::new(tmp.path());

        let result1 = mgr1.recover_after_crash().unwrap();
        let result2 = mgr2.recover_after_crash().unwrap();

        assert_eq!(result1, result2);
        assert!(result1.0); // Both should see committed
        assert_eq!(result1.1, Some(replica_id));
    }
}
