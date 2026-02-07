//! Replication Authority Enforcement
//!
//! Per PHASE2_REPLICATION_INVARIANTS.md §2.1-2.2:
//! - Single-writer invariant: exactly one Primary
//! - Commit authority invariant: only Primary assigns CommitId
//!
//! Per REPLICATION_MODEL.md §7:
//! - Write may be accepted only if node is PrimaryActive
//! - Write must be rejected if node is Replica or Halted

use super::errors::{ReplicationError, ReplicationResult};
use super::role::ReplicationState;

/// Authority check result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityCheck {
    /// Authority confirmed, operation may proceed
    Authorized,

    /// Not authorized, operation must be rejected
    NotAuthorized,

    /// Authority is ambiguous, system must halt
    Ambiguous,
}

/// Write admission decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAdmission {
    /// Write is admitted
    Admitted,

    /// Write is rejected - node is a Replica
    RejectedReplica,

    /// Write is rejected - node is halted
    RejectedHalted,

    /// Write is rejected - node is uninitialized
    RejectedUninitialized,
}

impl WriteAdmission {
    /// Check if write is admitted.
    pub fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted)
    }

    /// Convert to result.
    pub fn to_result(&self) -> ReplicationResult<()> {
        match self {
            Self::Admitted => Ok(()),
            Self::RejectedReplica => Err(ReplicationError::write_rejected(
                "writes are not allowed on Replica nodes",
            )),
            Self::RejectedHalted => Err(ReplicationError::write_rejected(
                "writes are not allowed when replication is halted",
            )),
            Self::RejectedUninitialized => Err(ReplicationError::write_rejected(
                "writes are not allowed before initialization",
            )),
        }
    }
}

/// Check write admission based on replication state.
///
/// Per REPLICATION_MODEL.md §7:
/// - May be accepted only if node is PrimaryActive
/// - Must be rejected if node is Replica
/// - Must be rejected if node is ReplicationHalted
///
/// Per P5-I16: Disabled mode behaves like standalone primary
pub fn check_write_admission(state: &ReplicationState) -> WriteAdmission {
    match state {
        ReplicationState::Disabled => WriteAdmission::Admitted,
        ReplicationState::PrimaryActive => WriteAdmission::Admitted,
        ReplicationState::ReplicaActive { .. } => WriteAdmission::RejectedReplica,
        ReplicationState::ReplicationHalted { .. } => WriteAdmission::RejectedHalted,
        ReplicationState::Uninitialized => WriteAdmission::RejectedUninitialized,
    }
}

/// Check if CommitId assignment is allowed.
///
/// Per PHASE2_REPLICATION_INVARIANTS.md §2.2:
/// - CommitIds are assigned only by the Primary
/// - Replicas must never generate CommitIds
///
/// Per P5-I16: Disabled mode behaves like standalone primary
pub fn check_commit_authority(state: &ReplicationState) -> ReplicationResult<()> {
    match state {
        ReplicationState::Disabled => Ok(()),
        ReplicationState::PrimaryActive => Ok(()),
        ReplicationState::ReplicaActive { .. } => Err(
            ReplicationError::commit_authority_violation("Replicas must never assign CommitIds"),
        ),
        ReplicationState::ReplicationHalted { .. } => Err(ReplicationError::halted(
            "cannot assign CommitId when replication is halted",
        )),
        ReplicationState::Uninitialized => Err(ReplicationError::commit_authority_violation(
            "cannot assign CommitId before initialization",
        )),
    }
}

/// Detect dual-primary condition.
///
/// Per REPLICATION_MODEL.md §5:
/// - Dual Primary is forbidden
/// - If two nodes believe they are Primary, system must halt writes
///
/// Per P5-I16: Disabled mode is treated as not having authority claim
/// (since it's standalone, not participating in replication).
///
/// This function would be called with external information about other nodes.
/// Returns AuthorityAmbiguity if another Primary is detected.
pub fn check_dual_primary(
    local_state: &ReplicationState,
    other_node_claims_primary: bool,
) -> AuthorityCheck {
    match local_state {
        ReplicationState::Disabled => {
            // Disabled mode is standalone, no replication authority claim
            AuthorityCheck::NotAuthorized
        }
        ReplicationState::PrimaryActive => {
            if other_node_claims_primary {
                // Per PHASE2_REPLICATION_INVARIANTS.md §2.1:
                // No two nodes may acknowledge writes concurrently
                AuthorityCheck::Ambiguous
            } else {
                AuthorityCheck::Authorized
            }
        }
        ReplicationState::ReplicaActive { .. } => AuthorityCheck::NotAuthorized,
        ReplicationState::Uninitialized => AuthorityCheck::NotAuthorized,
        ReplicationState::ReplicationHalted { .. } => AuthorityCheck::NotAuthorized,
    }
}

#[cfg(test)]
mod tests {
    use super::super::role::HaltReason;
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_write_admission_disabled_allowed() {
        // Per P5-I16: Disabled behaves like standalone primary
        let state = ReplicationState::Disabled;
        let admission = check_write_admission(&state);
        assert_eq!(admission, WriteAdmission::Admitted);
        assert!(admission.to_result().is_ok());
    }

    #[test]
    fn test_write_admission_primary_allowed() {
        // Per PHASE2_REPLICATION_INVARIANTS.md §2.1
        let state = ReplicationState::PrimaryActive;
        let admission = check_write_admission(&state);
        assert_eq!(admission, WriteAdmission::Admitted);
        assert!(admission.to_result().is_ok());
    }

    #[test]
    fn test_write_admission_replica_rejected() {
        // Per PHASE2_REPLICATION_INVARIANTS.md §2.1
        let replica_id = Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };
        let admission = check_write_admission(&state);
        assert_eq!(admission, WriteAdmission::RejectedReplica);
        assert!(admission.to_result().is_err());
    }

    #[test]
    fn test_write_admission_halted_rejected() {
        let state = ReplicationState::ReplicationHalted {
            reason: HaltReason::WalGapDetected,
        };
        let admission = check_write_admission(&state);
        assert_eq!(admission, WriteAdmission::RejectedHalted);
        assert!(admission.to_result().is_err());
    }

    #[test]
    fn test_write_admission_uninitialized_rejected() {
        let state = ReplicationState::Uninitialized;
        let admission = check_write_admission(&state);
        assert_eq!(admission, WriteAdmission::RejectedUninitialized);
    }

    #[test]
    fn test_commit_authority_disabled_allowed() {
        // Per P5-I16: Disabled behaves like standalone primary
        let state = ReplicationState::Disabled;
        assert!(check_commit_authority(&state).is_ok());
    }

    #[test]
    fn test_commit_authority_primary_allowed() {
        // Per PHASE2_REPLICATION_INVARIANTS.md §2.2
        let state = ReplicationState::PrimaryActive;
        assert!(check_commit_authority(&state).is_ok());
    }

    #[test]
    fn test_commit_authority_replica_forbidden() {
        // Per PHASE2_REPLICATION_INVARIANTS.md §2.2
        let replica_id = Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };
        let result = check_commit_authority(&state);
        assert!(result.is_err());
    }

    #[test]
    fn test_dual_primary_detection() {
        // Per REPLICATION_MODEL.md §5
        let state = ReplicationState::PrimaryActive;

        // No other primary → authorized
        assert_eq!(
            check_dual_primary(&state, false),
            AuthorityCheck::Authorized
        );

        // Another primary detected → ambiguous
        assert_eq!(check_dual_primary(&state, true), AuthorityCheck::Ambiguous);
    }

    #[test]
    fn test_disabled_not_authorized_for_dual_primary() {
        // Per P5-I16: Disabled is standalone, not participating in replication
        let state = ReplicationState::Disabled;
        assert_eq!(
            check_dual_primary(&state, false),
            AuthorityCheck::NotAuthorized
        );
    }

    #[test]
    fn test_replica_never_authorized_for_writes() {
        let replica_id = Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };

        // Even if no other primary exists, replica is not authorized
        assert_eq!(
            check_dual_primary(&state, false),
            AuthorityCheck::NotAuthorized
        );
    }
}
