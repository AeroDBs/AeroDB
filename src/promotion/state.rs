//! Promotion State Machine
//!
//! Per PHASE6_STATE_MACHINE.md:
//! - States are explicit and enumerable
//! - Transitions are event-driven, never inferred
//! - All transitions are deterministic
//! - No background or time-based transitions
//! - All authority changes are atomic
//! - All failures are explicit
//!
//! Crash Semantics (per §8):
//! - Steady: No effect
//! - PromotionRequested: Promotion forgotten
//! - PromotionValidating: Promotion forgotten
//! - PromotionApproved: Promotion forgotten
//! - AuthorityTransitioning: Atomic outcome enforced
//! - PromotionSucceeded: Authority preserved
//! - PromotionDenied: Promotion forgotten

use super::errors::{PromotionError, PromotionResult};
use uuid::Uuid;

/// Reason for promotion denial
///
/// Per PHASE6_INVARIANTS.md §P6-O2:
/// For every promotion decision, the system MUST be able to explain
/// why promotion was allowed or denied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenialReason {
    /// Replica WAL is behind committed primary WAL
    /// Violates P6-S1 (No Acknowledged Write Loss)
    ReplicaBehindWal,

    /// Replica replication state is not suitable
    /// Violates P6-S2 (WAL Prefix Rule)
    InvalidReplicationState,

    /// Current primary is still active
    /// Violates P6-A1 (Single Write Authority)
    PrimaryStillActive,

    /// Authority ambiguity detected
    /// Violates P6-A1 (Single Write Authority)
    AuthorityAmbiguous,

    /// MVCC visibility would be compromised
    /// Violates P6-S3 (MVCC Visibility Preservation)
    MvccVisibilityViolation,

    /// Replica is not in ReplicaActive state
    ReplicaNotActive,

    /// Replication is disabled
    ReplicationDisabled,

    /// Invalid promotion request
    InvalidRequest,
}

impl DenialReason {
    /// Get the invariant reference for this denial reason.
    pub fn invariant_reference(&self) -> &'static str {
        match self {
            Self::ReplicaBehindWal => "P6-S1",
            Self::InvalidReplicationState => "P6-S2",
            Self::PrimaryStillActive => "P6-A1",
            Self::AuthorityAmbiguous => "P6-A1",
            Self::MvccVisibilityViolation => "P6-S3",
            Self::ReplicaNotActive => "P6-A3",
            Self::ReplicationDisabled => "P5-I16",
            Self::InvalidRequest => "P6-A3",
        }
    }

    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::ReplicaBehindWal => "replica WAL position is behind committed primary WAL",
            Self::InvalidReplicationState => {
                "replica replication state does not satisfy prefix rule"
            }
            Self::PrimaryStillActive => {
                "current primary is still active; cannot have dual primaries"
            }
            Self::AuthorityAmbiguous => "write authority is ambiguous; cannot safely promote",
            Self::MvccVisibilityViolation => "promotion would violate MVCC visibility guarantees",
            Self::ReplicaNotActive => "replica is not in active replication state",
            Self::ReplicationDisabled => "replication is disabled; promotion not applicable",
            Self::InvalidRequest => "promotion request is invalid",
        }
    }
}

/// Phase 6 Promotion State Machine
///
/// Per PHASE6_STATE_MACHINE.md §4:
/// This state machine is ORTHOGONAL to Phase 5's replication state machine.
/// It observes and constrains Phase 5 transitions but does not replace them.
///
/// States are transient except for:
/// - AuthorityTransitioning: Atomic outcome enforced
/// - PromotionSucceeded: Authority preserved
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotionState {
    /// System is operating normally.
    /// No promotion attempt in progress.
    /// Replication roles are stable.
    Steady,

    /// An explicit promotion request has been issued.
    /// No validation has begun yet.
    PromotionRequested {
        /// The replica being considered for promotion
        replica_id: Uuid,
    },

    /// System is validating whether promotion is allowed.
    /// No authority change has occurred.
    PromotionValidating {
        /// The replica being validated
        replica_id: Uuid,
    },

    /// Promotion has been fully validated.
    /// Authority transition is permitted but not yet applied.
    /// Approval has NO durable effect.
    PromotionApproved {
        /// The replica approved for promotion
        replica_id: Uuid,
    },

    /// Atomic authority transfer is in progress.
    /// Per crash semantics: Atomic outcome enforced.
    AuthorityTransitioning {
        /// The replica becoming primary
        replica_id: Uuid,
    },

    /// Promotion completed successfully.
    /// New primary is authoritative.
    /// Per crash semantics: Authority preserved.
    PromotionSucceeded {
        /// The new primary's UUID
        new_primary_id: Uuid,
    },

    /// Promotion validation failed.
    /// No authority change occurred.
    PromotionDenied {
        /// The replica that was denied
        replica_id: Uuid,
        /// Explicit reason for denial
        reason: DenialReason,
    },
}

impl Default for PromotionState {
    fn default() -> Self {
        Self::new()
    }
}

impl PromotionState {
    /// Create a new state machine in Steady state.
    pub fn new() -> Self {
        Self::Steady
    }

    /// Get the state name for observability.
    pub fn state_name(&self) -> &'static str {
        match self {
            Self::Steady => "Steady",
            Self::PromotionRequested { .. } => "PromotionRequested",
            Self::PromotionValidating { .. } => "PromotionValidating",
            Self::PromotionApproved { .. } => "PromotionApproved",
            Self::AuthorityTransitioning { .. } => "AuthorityTransitioning",
            Self::PromotionSucceeded { .. } => "PromotionSucceeded",
            Self::PromotionDenied { .. } => "PromotionDenied",
        }
    }

    /// Check if a promotion is in progress.
    pub fn is_promotion_in_progress(&self) -> bool {
        !matches!(self, Self::Steady)
    }

    /// Get the replica ID if one is involved in the current state.
    pub fn replica_id(&self) -> Option<Uuid> {
        match self {
            Self::Steady => None,
            Self::PromotionRequested { replica_id } => Some(*replica_id),
            Self::PromotionValidating { replica_id } => Some(*replica_id),
            Self::PromotionApproved { replica_id } => Some(*replica_id),
            Self::AuthorityTransitioning { replica_id } => Some(*replica_id),
            Self::PromotionSucceeded { new_primary_id } => Some(*new_primary_id),
            Self::PromotionDenied { replica_id, .. } => Some(*replica_id),
        }
    }

    // =========================================================================
    // ALLOWED TRANSITIONS (per PHASE6_STATE_MACHINE.md §6)
    // =========================================================================

    /// Steady → PromotionRequested
    ///
    /// Entry: Operator or control-plane requests promotion.
    pub fn request_promotion(self, replica_id: Uuid) -> PromotionResult<Self> {
        match self {
            Self::Steady => Ok(Self::PromotionRequested { replica_id }),
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "PromotionRequested",
            )),
        }
    }

    /// PromotionRequested → PromotionValidating
    ///
    /// Begin validation of the promotion request.
    pub fn begin_validation(self) -> PromotionResult<Self> {
        match self {
            Self::PromotionRequested { replica_id } => Ok(Self::PromotionValidating { replica_id }),
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "PromotionValidating",
            )),
        }
    }

    /// PromotionRequested → Steady
    ///
    /// Request rejected immediately (e.g., invalid replica).
    pub fn reject_request(self) -> PromotionResult<Self> {
        match self {
            Self::PromotionRequested { .. } => Ok(Self::Steady),
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "Steady",
            )),
        }
    }

    /// PromotionValidating → PromotionApproved
    ///
    /// Validation succeeded; promotion is allowed.
    pub fn approve_promotion(self) -> PromotionResult<Self> {
        match self {
            Self::PromotionValidating { replica_id } => Ok(Self::PromotionApproved { replica_id }),
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "PromotionApproved",
            )),
        }
    }

    /// PromotionValidating → PromotionDenied
    ///
    /// Validation failed; promotion is denied with explicit reason.
    pub fn deny_promotion(self, reason: DenialReason) -> PromotionResult<Self> {
        match self {
            Self::PromotionValidating { replica_id } => {
                Ok(Self::PromotionDenied { replica_id, reason })
            }
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "PromotionDenied",
            )),
        }
    }

    /// PromotionApproved → AuthorityTransitioning
    ///
    /// Begin atomic authority transfer.
    pub fn begin_authority_transition(self) -> PromotionResult<Self> {
        match self {
            Self::PromotionApproved { replica_id } => {
                Ok(Self::AuthorityTransitioning { replica_id })
            }
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "AuthorityTransitioning",
            )),
        }
    }

    /// AuthorityTransitioning → PromotionSucceeded
    ///
    /// Authority transition completed atomically.
    pub fn complete_transition(self) -> PromotionResult<Self> {
        match self {
            Self::AuthorityTransitioning { replica_id } => Ok(Self::PromotionSucceeded {
                new_primary_id: replica_id,
            }),
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "PromotionSucceeded",
            )),
        }
    }

    /// PromotionSucceeded → Steady
    ///
    /// Return to steady state after successful promotion.
    pub fn acknowledge_success(self) -> PromotionResult<Self> {
        match self {
            Self::PromotionSucceeded { .. } => Ok(Self::Steady),
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "Steady",
            )),
        }
    }

    /// PromotionDenied → Steady
    ///
    /// Return to steady state after denied promotion.
    pub fn acknowledge_denial(self) -> PromotionResult<Self> {
        match self {
            Self::PromotionDenied { .. } => Ok(Self::Steady),
            _ => Err(PromotionError::forbidden_transition(
                self.state_name(),
                "Steady",
            )),
        }
    }

    // =========================================================================
    // CRASH RECOVERY (per PHASE6_STATE_MACHINE.md §8)
    // =========================================================================

    /// Recover state after crash.
    ///
    /// Per PHASE6_STATE_MACHINE.md §5:
    /// - System MUST re-enter Steady
    /// - Authority state MUST be reconstructed deterministically
    /// - No partial promotion state may persist
    ///
    /// Only AuthorityTransitioning and PromotionSucceeded have durable authority effects.
    /// All other states are forgotten on crash.
    pub fn recover_after_crash(
        authority_transition_was_atomic: bool,
        new_primary_id: Option<Uuid>,
    ) -> Self {
        if authority_transition_was_atomic {
            if let Some(id) = new_primary_id {
                // Transition completed atomically; authority is preserved
                Self::PromotionSucceeded { new_primary_id: id }
            } else {
                // Should not happen: atomic transition without new primary
                Self::Steady
            }
        } else {
            // All transient states are forgotten
            Self::Steady
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    // =========================================================================
    // ALLOWED TRANSITION TESTS
    // =========================================================================

    #[test]
    fn test_steady_to_requested() {
        let state = PromotionState::Steady;
        let replica_id = test_uuid();

        let result = state.request_promotion(replica_id);
        assert!(result.is_ok());

        match result.unwrap() {
            PromotionState::PromotionRequested { replica_id: id } => {
                assert_eq!(id, replica_id);
            }
            _ => panic!("expected PromotionRequested"),
        }
    }

    #[test]
    fn test_requested_to_validating() {
        let replica_id = test_uuid();
        let state = PromotionState::PromotionRequested { replica_id };

        let result = state.begin_validation();
        assert!(result.is_ok());

        match result.unwrap() {
            PromotionState::PromotionValidating { replica_id: id } => {
                assert_eq!(id, replica_id);
            }
            _ => panic!("expected PromotionValidating"),
        }
    }

    #[test]
    fn test_requested_to_steady() {
        let replica_id = test_uuid();
        let state = PromotionState::PromotionRequested { replica_id };

        let result = state.reject_request();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PromotionState::Steady);
    }

    #[test]
    fn test_validating_to_approved() {
        let replica_id = test_uuid();
        let state = PromotionState::PromotionValidating { replica_id };

        let result = state.approve_promotion();
        assert!(result.is_ok());

        match result.unwrap() {
            PromotionState::PromotionApproved { replica_id: id } => {
                assert_eq!(id, replica_id);
            }
            _ => panic!("expected PromotionApproved"),
        }
    }

    #[test]
    fn test_validating_to_denied() {
        let replica_id = test_uuid();
        let state = PromotionState::PromotionValidating { replica_id };
        let reason = DenialReason::ReplicaBehindWal;

        let result = state.deny_promotion(reason.clone());
        assert!(result.is_ok());

        match result.unwrap() {
            PromotionState::PromotionDenied {
                replica_id: id,
                reason: r,
            } => {
                assert_eq!(id, replica_id);
                assert_eq!(r, reason);
            }
            _ => panic!("expected PromotionDenied"),
        }
    }

    #[test]
    fn test_approved_to_transitioning() {
        let replica_id = test_uuid();
        let state = PromotionState::PromotionApproved { replica_id };

        let result = state.begin_authority_transition();
        assert!(result.is_ok());

        match result.unwrap() {
            PromotionState::AuthorityTransitioning { replica_id: id } => {
                assert_eq!(id, replica_id);
            }
            _ => panic!("expected AuthorityTransitioning"),
        }
    }

    #[test]
    fn test_transitioning_to_succeeded() {
        let replica_id = test_uuid();
        let state = PromotionState::AuthorityTransitioning { replica_id };

        let result = state.complete_transition();
        assert!(result.is_ok());

        match result.unwrap() {
            PromotionState::PromotionSucceeded { new_primary_id } => {
                assert_eq!(new_primary_id, replica_id);
            }
            _ => panic!("expected PromotionSucceeded"),
        }
    }

    #[test]
    fn test_succeeded_to_steady() {
        let new_primary_id = test_uuid();
        let state = PromotionState::PromotionSucceeded { new_primary_id };

        let result = state.acknowledge_success();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PromotionState::Steady);
    }

    #[test]
    fn test_denied_to_steady() {
        let replica_id = test_uuid();
        let state = PromotionState::PromotionDenied {
            replica_id,
            reason: DenialReason::PrimaryStillActive,
        };

        let result = state.acknowledge_denial();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PromotionState::Steady);
    }

    // =========================================================================
    // FORBIDDEN TRANSITION TESTS (per PHASE6_STATE_MACHINE.md §7)
    // =========================================================================

    #[test]
    fn test_forbidden_steady_to_transitioning() {
        // Steady → AuthorityTransitioning is FORBIDDEN
        let state = PromotionState::Steady;
        let result = state.begin_authority_transition();

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind,
            super::super::errors::PromotionErrorKind::ForbiddenTransition
        );
    }

    #[test]
    fn test_forbidden_requested_to_approved() {
        // PromotionRequested → PromotionApproved is FORBIDDEN
        let replica_id = test_uuid();
        let state = PromotionState::PromotionRequested { replica_id };
        let result = state.approve_promotion();

        assert!(result.is_err());
    }

    #[test]
    fn test_forbidden_validating_to_transitioning() {
        // PromotionValidating → AuthorityTransitioning is FORBIDDEN
        let replica_id = test_uuid();
        let state = PromotionState::PromotionValidating { replica_id };
        let result = state.begin_authority_transition();

        assert!(result.is_err());
    }

    #[test]
    fn test_forbidden_denied_to_transitioning() {
        // PromotionDenied → AuthorityTransitioning is FORBIDDEN
        let replica_id = test_uuid();
        let state = PromotionState::PromotionDenied {
            replica_id,
            reason: DenialReason::ReplicaBehindWal,
        };
        let result = state.begin_authority_transition();

        assert!(result.is_err());
    }

    // =========================================================================
    // CRASH RECOVERY TESTS
    // =========================================================================

    #[test]
    fn test_crash_recovery_no_transition() {
        // Non-atomic transition → Steady
        let state = PromotionState::recover_after_crash(false, None);
        assert_eq!(state, PromotionState::Steady);
    }

    #[test]
    fn test_crash_recovery_atomic_transition() {
        // Atomic transition → PromotionSucceeded
        let new_primary_id = test_uuid();
        let state = PromotionState::recover_after_crash(true, Some(new_primary_id));

        match state {
            PromotionState::PromotionSucceeded { new_primary_id: id } => {
                assert_eq!(id, new_primary_id);
            }
            _ => panic!("expected PromotionSucceeded"),
        }
    }

    // =========================================================================
    // STATE PROPERTY TESTS
    // =========================================================================

    #[test]
    fn test_state_names() {
        assert_eq!(PromotionState::Steady.state_name(), "Steady");

        let id = test_uuid();
        assert_eq!(
            PromotionState::PromotionRequested { replica_id: id }.state_name(),
            "PromotionRequested"
        );
        assert_eq!(
            PromotionState::PromotionValidating { replica_id: id }.state_name(),
            "PromotionValidating"
        );
        assert_eq!(
            PromotionState::PromotionApproved { replica_id: id }.state_name(),
            "PromotionApproved"
        );
        assert_eq!(
            PromotionState::AuthorityTransitioning { replica_id: id }.state_name(),
            "AuthorityTransitioning"
        );
        assert_eq!(
            PromotionState::PromotionSucceeded { new_primary_id: id }.state_name(),
            "PromotionSucceeded"
        );
        assert_eq!(
            PromotionState::PromotionDenied {
                replica_id: id,
                reason: DenialReason::InvalidRequest,
            }
            .state_name(),
            "PromotionDenied"
        );
    }

    #[test]
    fn test_is_promotion_in_progress() {
        assert!(!PromotionState::Steady.is_promotion_in_progress());

        let id = test_uuid();
        assert!(PromotionState::PromotionRequested { replica_id: id }.is_promotion_in_progress());
        assert!(PromotionState::PromotionValidating { replica_id: id }.is_promotion_in_progress());
        assert!(PromotionState::PromotionApproved { replica_id: id }.is_promotion_in_progress());
        assert!(
            PromotionState::AuthorityTransitioning { replica_id: id }.is_promotion_in_progress()
        );
        assert!(
            PromotionState::PromotionSucceeded { new_primary_id: id }.is_promotion_in_progress()
        );
        assert!(PromotionState::PromotionDenied {
            replica_id: id,
            reason: DenialReason::InvalidRequest,
        }
        .is_promotion_in_progress());
    }

    #[test]
    fn test_denial_reason_invariant_references() {
        assert_eq!(
            DenialReason::ReplicaBehindWal.invariant_reference(),
            "P6-S1"
        );
        assert_eq!(
            DenialReason::InvalidReplicationState.invariant_reference(),
            "P6-S2"
        );
        assert_eq!(
            DenialReason::PrimaryStillActive.invariant_reference(),
            "P6-A1"
        );
        assert_eq!(
            DenialReason::AuthorityAmbiguous.invariant_reference(),
            "P6-A1"
        );
        assert_eq!(
            DenialReason::MvccVisibilityViolation.invariant_reference(),
            "P6-S3"
        );
        assert_eq!(
            DenialReason::ReplicaNotActive.invariant_reference(),
            "P6-A3"
        );
        assert_eq!(
            DenialReason::ReplicationDisabled.invariant_reference(),
            "P5-I16"
        );
        assert_eq!(DenialReason::InvalidRequest.invariant_reference(), "P6-A3");
    }

    // =========================================================================
    // FULL PROMOTION LIFECYCLE TESTS
    // =========================================================================

    #[test]
    fn test_successful_promotion_lifecycle() {
        let replica_id = test_uuid();

        // Steady → PromotionRequested
        let state = PromotionState::Steady;
        let state = state.request_promotion(replica_id).unwrap();
        assert_eq!(state.state_name(), "PromotionRequested");

        // PromotionRequested → PromotionValidating
        let state = state.begin_validation().unwrap();
        assert_eq!(state.state_name(), "PromotionValidating");

        // PromotionValidating → PromotionApproved
        let state = state.approve_promotion().unwrap();
        assert_eq!(state.state_name(), "PromotionApproved");

        // PromotionApproved → AuthorityTransitioning
        let state = state.begin_authority_transition().unwrap();
        assert_eq!(state.state_name(), "AuthorityTransitioning");

        // AuthorityTransitioning → PromotionSucceeded
        let state = state.complete_transition().unwrap();
        assert_eq!(state.state_name(), "PromotionSucceeded");

        // PromotionSucceeded → Steady
        let state = state.acknowledge_success().unwrap();
        assert_eq!(state, PromotionState::Steady);
    }

    #[test]
    fn test_denied_promotion_lifecycle() {
        let replica_id = test_uuid();

        // Steady → PromotionRequested
        let state = PromotionState::Steady;
        let state = state.request_promotion(replica_id).unwrap();

        // PromotionRequested → PromotionValidating
        let state = state.begin_validation().unwrap();

        // PromotionValidating → PromotionDenied
        let state = state
            .deny_promotion(DenialReason::ReplicaBehindWal)
            .unwrap();
        assert_eq!(state.state_name(), "PromotionDenied");

        // PromotionDenied → Steady
        let state = state.acknowledge_denial().unwrap();
        assert_eq!(state, PromotionState::Steady);
    }

    #[test]
    fn test_rejected_request_lifecycle() {
        let replica_id = test_uuid();

        // Steady → PromotionRequested
        let state = PromotionState::Steady;
        let state = state.request_promotion(replica_id).unwrap();

        // PromotionRequested → Steady (rejected immediately)
        let state = state.reject_request().unwrap();
        assert_eq!(state, PromotionState::Steady);
    }
}
