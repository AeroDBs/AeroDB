//! Replication Role State Machine
//!
//! Per REPLICATION_MODEL.md §4:
//! - Disabled: Replication is off (default per P5-I16)
//! - Uninitialized: No authority, no traffic
//! - PrimaryActive: Sole write authority
//! - ReplicaActive: Follows Primary
//! - ReplicationHalted: Invariant violation detected
//!
//! Per PHASE2_REPLICATION_INVARIANTS.md §2.1:
//! - At any moment, exactly one node may acknowledge writes
//! - If authority is unclear, writes must be rejected
//!
//! Per PHASE5_INVARIANTS.md §P5-I16:
//! - Replication MUST be disableable at startup
//! - Disabling MUST NOT affect primary behavior

use super::errors::{ReplicationError, ReplicationResult};
use uuid::Uuid;

/// Replication role per REPLICATION_MODEL.md §2
///
/// A node is either a Primary (creates history) or a Replica (consumes history).
/// This is configured externally, never inferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicationRole {
    /// Node is configured as the Primary
    /// May accept writes, assign CommitIds, emit WAL records
    Primary,

    /// Node is configured as a Replica
    /// Must reject writes, apply WAL from Primary only
    Replica,
}

/// Replication state per REPLICATION_MODEL.md §4
///
/// Each node exists in exactly one of these states.
/// Per PHASE5_INVARIANTS.md §P5-I3: All replication logic MUST use
/// explicit state machines with enumerated states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplicationState {
    /// Replication is disabled (default per P5-I16).
    ///
    /// In this state the node operates as a standalone primary
    /// with identical behavior to Phase 0-4.
    /// This is the default-safe path.
    Disabled,

    /// No authoritative history, no WAL authority
    /// Node must not accept any traffic
    /// This state exists only before bootstrap
    Uninitialized,

    /// Node is the sole write authority
    /// Per PHASE2_REPLICATION_INVARIANTS.md §2.1:
    /// - May accept writes
    /// - May assign CommitIds
    /// - May emit WAL records
    /// - Must reject attempts to follow another Primary
    PrimaryActive,

    /// Node follows a specific Primary
    /// Per PHASE2_REPLICATION_INVARIANTS.md §2.1:
    /// - May apply WAL records
    /// - May serve reads (subject to REPLICATION_READ_SEMANTICS.md)
    /// - Must reject all writes
    /// - Must reject CommitId assignment
    ReplicaActive {
        /// Unique replica identifier per PHASE5_IMPLEMENTATION_ORDER.md §Stage 1
        replica_id: Uuid,
    },

    /// Node has detected an invariant violation
    /// Per REPLICATION_MODEL.md §4.4:
    /// - WAL gap detected
    /// - Divergent history detected
    /// - Authority ambiguity detected
    ///
    /// In this state:
    /// - No reads allowed
    /// - No writes allowed
    /// - Explicit operator intervention required
    ReplicationHalted {
        /// Reason for halt
        reason: HaltReason,
    },
}

/// Reason for replication halt
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaltReason {
    /// WAL gap detected during replication
    WalGapDetected,

    /// Divergent history detected
    HistoryDivergence,

    /// Authority ambiguity (e.g., dual primary suspected)
    AuthorityAmbiguity,

    /// WAL corruption during replication
    WalCorruption,

    /// Snapshot integrity failure
    SnapshotIntegrityFailure,

    /// Configuration error
    ConfigurationError,
}

impl ReplicationState {
    /// Create a new disabled state (default per P5-I16).
    pub fn new() -> Self {
        Self::Disabled
    }

    /// Create a new uninitialized state for enabled replication.
    pub fn uninitialized() -> Self {
        Self::Uninitialized
    }

    /// Transition to PrimaryActive.
    ///
    /// Per REPLICATION_MODEL.md §4.2:
    /// - Valid from Disabled or Uninitialized
    /// - External authority check must pass
    pub fn become_primary(self) -> ReplicationResult<Self> {
        match self {
            Self::Disabled => Ok(Self::PrimaryActive),
            Self::Uninitialized => Ok(Self::PrimaryActive),
            Self::PrimaryActive => Ok(Self::PrimaryActive), // Idempotent
            Self::ReplicaActive { .. } => Err(ReplicationError::illegal_transition(
                "cannot transition from Replica to Primary without explicit reconfiguration",
            )),
            Self::ReplicationHalted { .. } => Err(ReplicationError::halted(
                "cannot transition from halted state without operator intervention",
            )),
        }
    }

    /// Transition to ReplicaActive.
    ///
    /// Per REPLICATION_MODEL.md §4.3:
    /// - Valid from Disabled or Uninitialized
    /// - Requires replica_id for identity
    pub fn become_replica(self, replica_id: Uuid) -> ReplicationResult<Self> {
        match self {
            Self::Disabled => Ok(Self::ReplicaActive { replica_id }),
            Self::Uninitialized => Ok(Self::ReplicaActive { replica_id }),
            Self::ReplicaActive {
                replica_id: existing_id,
            } => {
                // Idempotent if same ID
                if existing_id == replica_id {
                    Ok(Self::ReplicaActive { replica_id })
                } else {
                    Err(ReplicationError::illegal_transition(
                        "cannot change replica_id without explicit reconfiguration",
                    ))
                }
            }
            Self::PrimaryActive => Err(ReplicationError::illegal_transition(
                "cannot transition from Primary to Replica without explicit reconfiguration",
            )),
            Self::ReplicationHalted { .. } => Err(ReplicationError::halted(
                "cannot transition from halted state without operator intervention",
            )),
        }
    }

    /// Transition to ReplicationHalted.
    ///
    /// Per REPLICATION_MODEL.md §4.4:
    /// - Valid from any state
    /// - Once halted, cannot resume without operator intervention
    pub fn halt(self, reason: HaltReason) -> Self {
        Self::ReplicationHalted { reason }
    }

    /// Check if this state allows writes.
    ///
    /// Per REPLICATION_MODEL.md §7:
    /// - Only PrimaryActive may accept writes
    /// - Disabled behaves like standalone primary
    pub fn can_write(&self) -> bool {
        matches!(self, Self::PrimaryActive | Self::Disabled)
    }

    /// Check if this state allows reads.
    ///
    /// Per REPLICATION_MODEL.md §8:
    /// - Primary, Disabled, and ReplicaActive may serve reads
    /// - ReplicationHalted must refuse reads
    /// - Uninitialized must refuse reads
    pub fn can_read(&self) -> bool {
        matches!(
            self,
            Self::PrimaryActive | Self::Disabled | Self::ReplicaActive { .. }
        )
    }

    /// Check if this state is halted.
    pub fn is_halted(&self) -> bool {
        matches!(self, Self::ReplicationHalted { .. })
    }

    /// Check if replication is disabled.
    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled)
    }

    /// Check if this is Primary.
    pub fn is_primary(&self) -> bool {
        matches!(self, Self::PrimaryActive)
    }

    /// Check if this is Replica.
    pub fn is_replica(&self) -> bool {
        matches!(self, Self::ReplicaActive { .. })
    }

    /// Get replica ID if in ReplicaActive state.
    pub fn replica_id(&self) -> Option<Uuid> {
        match self {
            Self::ReplicaActive { replica_id } => Some(*replica_id),
            _ => None,
        }
    }

    /// Get halt reason if halted.
    pub fn halt_reason(&self) -> Option<HaltReason> {
        match self {
            Self::ReplicationHalted { reason } => Some(*reason),
            _ => None,
        }
    }

    /// Get state name for observability.
    ///
    /// Per PHASE5_OBSERVABILITY_MAPPING.md §3.1:
    /// Observable state name for /v1/replication endpoint.
    pub fn state_name(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Uninitialized => "uninitialized",
            Self::PrimaryActive => "primary_active",
            Self::ReplicaActive { .. } => "replica_active",
            Self::ReplicationHalted { .. } => "halted",
        }
    }
}

impl Default for ReplicationState {
    /// Default is Disabled per P5-I16.
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_disabled() {
        // Per P5-I16: Default is Disabled
        let state = ReplicationState::new();
        assert!(state.is_disabled());
        assert!(!state.is_primary());
        assert!(!state.is_replica());
    }

    #[test]
    fn test_disabled_can_become_primary() {
        let state = ReplicationState::new();
        let result = state.become_primary();
        assert!(result.is_ok());
        assert!(result.unwrap().is_primary());
    }

    #[test]
    fn test_disabled_can_become_replica() {
        let state = ReplicationState::new();
        let replica_id = Uuid::new_v4();
        let result = state.become_replica(replica_id);
        assert!(result.is_ok());
        let replica = result.unwrap();
        assert!(replica.is_replica());
        assert_eq!(replica.replica_id(), Some(replica_id));
    }

    #[test]
    fn test_uninitialized_can_become_primary() {
        let state = ReplicationState::uninitialized();
        let result = state.become_primary();
        assert!(result.is_ok());
        assert!(result.unwrap().is_primary());
    }

    #[test]
    fn test_uninitialized_can_become_replica() {
        let state = ReplicationState::uninitialized();
        let replica_id = Uuid::new_v4();
        let result = state.become_replica(replica_id);
        assert!(result.is_ok());
        assert!(result.unwrap().is_replica());
    }

    #[test]
    fn test_replica_cannot_become_primary_directly() {
        // Per REPLICATION_MODEL.md §5: Implicit Promotion is forbidden
        let replica_id = Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };
        let result = state.become_primary();
        assert!(result.is_err());
    }

    #[test]
    fn test_primary_cannot_become_replica_directly() {
        let state = ReplicationState::PrimaryActive;
        let replica_id = Uuid::new_v4();
        let result = state.become_replica(replica_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_replica_idempotent_with_same_id() {
        let replica_id = Uuid::new_v4();
        let state = ReplicationState::ReplicaActive { replica_id };
        let result = state.become_replica(replica_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_replica_cannot_change_id() {
        let replica_id1 = Uuid::new_v4();
        let replica_id2 = Uuid::new_v4();
        let state = ReplicationState::ReplicaActive {
            replica_id: replica_id1,
        };
        let result = state.become_replica(replica_id2);
        assert!(result.is_err());
    }

    #[test]
    fn test_halted_cannot_transition() {
        let state = ReplicationState::ReplicationHalted {
            reason: HaltReason::WalGapDetected,
        };

        assert!(state.clone().become_primary().is_err());
        assert!(state.become_replica(Uuid::new_v4()).is_err());
    }

    #[test]
    fn test_disabled_and_primary_can_write() {
        // Per P5-I16: Disabled behaves like standalone primary
        assert!(ReplicationState::Disabled.can_write());
        assert!(ReplicationState::PrimaryActive.can_write());
    }

    #[test]
    fn test_replica_cannot_write() {
        // Per PHASE2_REPLICATION_INVARIANTS.md §2.1
        let replica_id = Uuid::new_v4();
        let replica = ReplicationState::ReplicaActive { replica_id };
        assert!(!replica.can_write());
    }

    #[test]
    fn test_uninitialized_and_halted_cannot_write() {
        assert!(!ReplicationState::Uninitialized.can_write());
        assert!(!ReplicationState::ReplicationHalted {
            reason: HaltReason::WalGapDetected,
        }
        .can_write());
    }

    #[test]
    fn test_halted_cannot_read() {
        // Per REPLICATION_MODEL.md §4.4
        let halted = ReplicationState::ReplicationHalted {
            reason: HaltReason::AuthorityAmbiguity,
        };
        assert!(!halted.can_read());
    }

    #[test]
    fn test_uninitialized_cannot_read() {
        assert!(!ReplicationState::Uninitialized.can_read());
    }

    #[test]
    fn test_disabled_primary_replica_can_read() {
        let replica_id = Uuid::new_v4();
        assert!(ReplicationState::Disabled.can_read());
        assert!(ReplicationState::PrimaryActive.can_read());
        assert!(ReplicationState::ReplicaActive { replica_id }.can_read());
    }

    #[test]
    fn test_any_state_can_halt() {
        let replica_id = Uuid::new_v4();
        let states = vec![
            ReplicationState::Disabled,
            ReplicationState::Uninitialized,
            ReplicationState::PrimaryActive,
            ReplicationState::ReplicaActive { replica_id },
        ];

        for state in states {
            let halted = state.halt(HaltReason::WalCorruption);
            assert!(halted.is_halted());
            assert_eq!(halted.halt_reason(), Some(HaltReason::WalCorruption));
        }
    }

    #[test]
    fn test_state_names_for_observability() {
        // Per PHASE5_OBSERVABILITY_MAPPING.md §3.1
        let replica_id = Uuid::new_v4();

        assert_eq!(ReplicationState::Disabled.state_name(), "disabled");
        assert_eq!(
            ReplicationState::Uninitialized.state_name(),
            "uninitialized"
        );
        assert_eq!(
            ReplicationState::PrimaryActive.state_name(),
            "primary_active"
        );
        assert_eq!(
            ReplicationState::ReplicaActive { replica_id }.state_name(),
            "replica_active"
        );
        assert_eq!(
            ReplicationState::ReplicationHalted {
                reason: HaltReason::WalGapDetected
            }
            .state_name(),
            "halted"
        );
    }

    #[test]
    fn test_replica_id_only_for_replicas() {
        let replica_id = Uuid::new_v4();

        assert!(ReplicationState::Disabled.replica_id().is_none());
        assert!(ReplicationState::Uninitialized.replica_id().is_none());
        assert!(ReplicationState::PrimaryActive.replica_id().is_none());
        assert_eq!(
            ReplicationState::ReplicaActive { replica_id }.replica_id(),
            Some(replica_id)
        );
    }

    // ===== Stage 1 Integration Tests =====

    #[test]
    fn test_primary_behavior_identical_enabled_vs_disabled() {
        // Per Stage 1 completion criteria: Primary runs identically with replication enabled vs disabled
        // Both Disabled and PrimaryActive should have the same write/read behavior

        let disabled = ReplicationState::Disabled;
        let primary = ReplicationState::PrimaryActive;

        // Same write behavior
        assert_eq!(disabled.can_write(), primary.can_write());
        assert!(disabled.can_write());
        assert!(primary.can_write());

        // Same read behavior
        assert_eq!(disabled.can_read(), primary.can_read());
        assert!(disabled.can_read());
        assert!(primary.can_read());
    }

    #[test]
    fn test_replica_refuses_all_writes() {
        // Per Stage 1 completion criteria: Replica mode refuses all writes
        let replica_id = Uuid::new_v4();
        let replica = ReplicationState::ReplicaActive { replica_id };

        // Replica MUST NOT be able to write
        assert!(!replica.can_write());

        // Replica CAN read (that's its purpose)
        assert!(replica.can_read());
    }

    #[test]
    fn test_disabled_state_is_default_safe_path() {
        // Per P5-I16: Replication must be removable
        // Default state MUST be Disabled
        let default_state = ReplicationState::new();
        assert!(default_state.is_disabled());

        // Disabled state MUST allow all normal operations
        assert!(default_state.can_write());
        assert!(default_state.can_read());
    }
}
