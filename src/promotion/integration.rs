//! Replication Integration
//!
//! Per PHASE6_IMPLEMENTATION_ORDER.md §Stage 6.5:
//! - Wire promotion outcomes to replication roles
//! - Use existing Phase 5 role transitions
//! - No new replication semantics
//! - Explicit role rebinding only
//!
//! Per PHASE6_SCOPE.md §4.1:
//! - Phase 6 may read Phase 5 replication state
//! - Phase 6 may validate Phase 5 invariants
//! - Phase 6 may NOT modify Phase 5 state machines
//! - Phase 6 may NOT add hidden transitions

use super::errors::{PromotionError, PromotionErrorKind, PromotionResult};
use super::state::DenialReason;
use crate::replication::{HaltReason, ReplicationState};
use uuid::Uuid;

/// Result of role rebinding after promotion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebindResult {
    /// Role successfully rebound; replica is now primary.
    Success {
        old_state: ReplicationState,
        new_state: ReplicationState,
    },

    /// Role rebinding failed.
    Failed { reason: String },
}

impl RebindResult {
    /// Check if rebinding succeeded.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }
}

/// Reason for invalidating the old primary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationReason {
    /// A new primary was elected through normal promotion.
    NewPrimaryElected,
    /// An operator forced demotion of this node.
    ForcedDemotion,
    /// Network partition detected, stepping down to avoid split-brain.
    NetworkPartition,
}

/// Outcome of invalidating the old primary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationOutcome {
    /// Primary was successfully invalidated.
    Invalidated {
        old_state: ReplicationState,
        new_state: ReplicationState,
    },
    /// Node was already in an invalid/halted state.
    AlreadyInvalid,
    /// Invalidation not applicable to this node state.
    NotApplicable { reason: String },
}

/// Replication Integration Layer
///
/// Per PHASE6_ARCHITECTURE.md §4.1:
/// Phase 6 reads replication role and state, validates replica readiness,
/// and updates replication role ONLY through allowed transitions.
///
/// Phase 6 MUST NOT:
/// - Modify replication protocol
/// - Introduce new replication states
/// - Alter Phase 5 authority checks
pub struct ReplicationIntegration;

impl ReplicationIntegration {
    /// Validate that a replica is eligible for promotion.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-A3:
    /// Authority is granted only via explicit promotion.
    pub fn validate_replica_eligibility(
        replica_id: Uuid,
        current_state: &ReplicationState,
    ) -> Result<(), DenialReason> {
        match current_state {
            ReplicationState::Disabled => Err(DenialReason::ReplicationDisabled),
            ReplicationState::Uninitialized => Err(DenialReason::ReplicaNotActive),
            ReplicationState::PrimaryActive => {
                // Already primary
                Err(DenialReason::InvalidRequest)
            }
            ReplicationState::ReplicationHalted { reason } => {
                // Map halt reason to denial
                match reason {
                    HaltReason::AuthorityAmbiguity => Err(DenialReason::AuthorityAmbiguous),
                    _ => Err(DenialReason::InvalidReplicationState),
                }
            }
            ReplicationState::ReplicaActive { replica_id: rid } => {
                if *rid != replica_id {
                    Err(DenialReason::InvalidRequest)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Rebind replication role after promotion approval.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-A2:
    /// Authority transfer is atomic. All-or-nothing.
    ///
    /// This method transitions the replica from ReplicaActive to PrimaryActive.
    pub fn rebind_role(
        _replica_id: Uuid,
        current_state: &ReplicationState,
    ) -> PromotionResult<RebindResult> {
        // Validate current state is ReplicaActive
        match current_state {
            ReplicationState::ReplicaActive { .. } => {
                // Transition to PrimaryActive
                let new_state = ReplicationState::PrimaryActive;

                Ok(RebindResult::Success {
                    old_state: current_state.clone(),
                    new_state,
                })
            }
            _ => Ok(RebindResult::Failed {
                reason: format!("cannot rebind from state: {:?}", current_state),
            }),
        }
    }

    /// Invalidate the old primary after successful promotion.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-A1:
    /// At any point in time, at most one node may hold write authority.
    ///
    /// This method transitions the old primary to a non-writable state.
    /// The transition is deterministic based on the invalidation reason.
    pub fn invalidate_old_primary(
        current_state: &ReplicationState,
        reason: InvalidationReason,
    ) -> InvalidationOutcome {
        match current_state {
            ReplicationState::PrimaryActive => {
                // Transition primary to halted state with appropriate reason
                let halt_reason = match reason {
                    InvalidationReason::NewPrimaryElected => HaltReason::AuthorityAmbiguity,
                    InvalidationReason::ForcedDemotion => HaltReason::AuthorityAmbiguity,
                    InvalidationReason::NetworkPartition => HaltReason::AuthorityAmbiguity,
                };

                InvalidationOutcome::Invalidated {
                    old_state: current_state.clone(),
                    new_state: ReplicationState::ReplicationHalted {
                        reason: halt_reason,
                    },
                }
            }
            ReplicationState::ReplicationHalted { .. } => {
                // Already halted, no action needed
                InvalidationOutcome::AlreadyInvalid
            }
            _ => {
                // Not a primary, cannot invalidate
                InvalidationOutcome::NotApplicable {
                    reason: format!("cannot invalidate non-primary state: {:?}", current_state),
                }
            }
        }
    }

    /// Get the current replication state description for observability.
    pub fn describe_state(state: &ReplicationState) -> &'static str {
        match state {
            ReplicationState::Disabled => "Replication disabled",
            ReplicationState::Uninitialized => "Replication uninitialized",
            ReplicationState::PrimaryActive => "Primary (write authority)",
            ReplicationState::ReplicaActive { .. } => "Replica (following primary)",
            ReplicationState::ReplicationHalted { .. } => "Replication halted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    #[test]
    fn test_validate_replica_eligibility_valid() {
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        let result = ReplicationIntegration::validate_replica_eligibility(replica_id, &state);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_replica_eligibility_disabled() {
        let replica_id = test_uuid();
        let state = ReplicationState::Disabled;

        let result = ReplicationIntegration::validate_replica_eligibility(replica_id, &state);
        assert_eq!(result, Err(DenialReason::ReplicationDisabled));
    }

    #[test]
    fn test_validate_replica_eligibility_wrong_id() {
        let replica_id = test_uuid();
        let other_id = test_uuid();
        let state = ReplicationState::ReplicaActive {
            replica_id: other_id,
        };

        let result = ReplicationIntegration::validate_replica_eligibility(replica_id, &state);
        assert_eq!(result, Err(DenialReason::InvalidRequest));
    }

    #[test]
    fn test_rebind_role_success() {
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        let result = ReplicationIntegration::rebind_role(replica_id, &state).unwrap();

        assert!(result.is_success());
        match result {
            RebindResult::Success { new_state, .. } => {
                assert_eq!(new_state, ReplicationState::PrimaryActive);
            }
            _ => panic!("expected success"),
        }
    }

    #[test]
    fn test_rebind_role_from_disabled_fails() {
        let replica_id = test_uuid();
        let state = ReplicationState::Disabled;

        let result = ReplicationIntegration::rebind_role(replica_id, &state).unwrap();

        assert!(!result.is_success());
    }

    #[test]
    fn test_describe_state() {
        assert!(
            ReplicationIntegration::describe_state(&ReplicationState::Disabled)
                .contains("disabled")
        );
        assert!(
            ReplicationIntegration::describe_state(&ReplicationState::PrimaryActive)
                .contains("Primary")
        );
    }
}
