//! Promotion Validation
//!
//! Per PHASE6_ARCHITECTURE.md §3.2:
//! - Evaluates whether promotion is allowed
//! - Validates all Phase 6 safety invariants
//! - Produces an explicit allow/deny decision
//!
//! Validator logic MUST be:
//! - Deterministic
//! - Side-effect free
//! - Fully explainable

use super::state::DenialReason;
use crate::replication::{HaltReason, ReplicationState, WalPosition};
use uuid::Uuid;

/// Result of promotion validation.
///
/// Per PHASE6_INVARIANTS.md §P6-O2:
/// For every promotion decision, the system MUST be able to explain
/// why promotion was allowed or denied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Promotion is allowed.
    Allowed,

    /// Promotion is denied with explicit reason.
    Denied(DenialReason),
}

impl ValidationResult {
    /// Check if validation passed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }

    /// Get denial reason if denied.
    pub fn denial_reason(&self) -> Option<&DenialReason> {
        match self {
            Self::Denied(reason) => Some(reason),
            Self::Allowed => None,
        }
    }
}

/// Input context for promotion validation.
///
/// Per PHASE6_ARCHITECTURE.md §3.2 Inputs:
/// - Replica replication state
/// - WAL position and metadata
/// - Known primary authority state
pub struct ValidationContext {
    /// Replica's current replication state
    pub replica_state: ReplicationState,

    /// Replica's current WAL position
    pub replica_wal_position: WalPosition,

    /// Primary's last known committed WAL position
    pub primary_committed_position: Option<WalPosition>,

    /// Whether primary is known to be unavailable
    pub primary_unavailable: bool,

    /// Whether force flag is set on the request
    pub force: bool,
}

/// Promotion Validator
///
/// Per PHASE6_ARCHITECTURE.md §3.2:
/// - Evaluates whether promotion is allowed
/// - Validates all Phase 6 safety invariants
/// - Produces an explicit allow/deny decision
///
/// Inputs:
/// - Replica replication state
/// - WAL position and metadata
/// - Known primary authority state
///
/// Outputs:
/// - Deterministic validation result
/// - Explicit failure reasons if denied
pub struct PromotionValidator;

impl PromotionValidator {
    /// Validate a promotion request.
    ///
    /// Per PHASE6_INVARIANTS.md, enforces:
    /// - P6-A1: Single Write Authority
    /// - P6-S1: No Acknowledged Write Loss
    /// - P6-S2: WAL Prefix Rule Preservation
    /// - P6-S3: MVCC Visibility Preservation
    /// - P6-F1: Fail Closed, Not Open
    ///
    /// This method is:
    /// - Deterministic: Same inputs → same output
    /// - Side-effect free: No state mutation
    /// - Explainable: All denials have explicit reasons
    pub fn validate(_replica_id: Uuid, context: &ValidationContext) -> ValidationResult {
        // =====================================================================
        // Check 1: Replica must be in ReplicaActive state
        // Per P6-A3: Authority is granted only via explicit promotion
        // =====================================================================
        match &context.replica_state {
            ReplicationState::Disabled => {
                return ValidationResult::Denied(DenialReason::ReplicationDisabled);
            }
            ReplicationState::Uninitialized => {
                return ValidationResult::Denied(DenialReason::ReplicaNotActive);
            }
            ReplicationState::PrimaryActive => {
                // Already primary, nothing to promote
                return ValidationResult::Denied(DenialReason::InvalidRequest);
            }
            ReplicationState::ReplicationHalted { reason } => {
                // Halted replicas cannot be promoted
                return Self::denial_from_halt_reason(*reason);
            }
            ReplicationState::ReplicaActive { .. } => {
                // Valid state for promotion - continue validation
            }
        }

        // =====================================================================
        // Check 2: Primary authority must be clear
        // Per P6-A1: At any point in time, at most one node may hold write authority
        // =====================================================================
        if !context.primary_unavailable && !context.force {
            // Per P6-A1: Cannot promote while primary is still active
            return ValidationResult::Denied(DenialReason::PrimaryStillActive);
        }

        // =====================================================================
        // Check 3: Replica WAL must be caught up
        // Per P6-S1: No Acknowledged Write Loss
        // Per P6-S2: WAL Prefix Rule Preservation
        // =====================================================================
        if let Some(primary_pos) = &context.primary_committed_position {
            if context.replica_wal_position.sequence < primary_pos.sequence {
                // Replica is behind committed WAL - would lose acknowledged writes
                return ValidationResult::Denied(DenialReason::ReplicaBehindWal);
            }
        }

        // =====================================================================
        // All checks passed
        // =====================================================================
        ValidationResult::Allowed
    }

    /// Convert a HaltReason to a DenialReason.
    fn denial_from_halt_reason(halt: HaltReason) -> ValidationResult {
        let reason = match halt {
            HaltReason::WalGapDetected => DenialReason::InvalidReplicationState,
            HaltReason::HistoryDivergence => DenialReason::InvalidReplicationState,
            HaltReason::AuthorityAmbiguity => DenialReason::AuthorityAmbiguous,
            HaltReason::WalCorruption => DenialReason::InvalidReplicationState,
            HaltReason::SnapshotIntegrityFailure => DenialReason::InvalidReplicationState,
            HaltReason::ConfigurationError => DenialReason::InvalidReplicationState,
        };
        ValidationResult::Denied(reason)
    }

    /// Generate an explanation for the validation result.
    ///
    /// Per PHASE6_INVARIANTS.md §P6-O2:
    /// For every promotion decision, the system MUST be able to explain
    /// why promotion was allowed or denied.
    pub fn explain(result: &ValidationResult) -> String {
        match result {
            ValidationResult::Allowed => {
                "Promotion allowed: all Phase 6 invariants satisfied".to_string()
            }
            ValidationResult::Denied(reason) => {
                format!(
                    "Promotion denied: {} (invariant {})",
                    reason.description(),
                    reason.invariant_reference()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    fn make_replica_context(
        replica_seq: u64,
        primary_seq: Option<u64>,
        primary_unavailable: bool,
    ) -> ValidationContext {
        ValidationContext {
            replica_state: ReplicationState::ReplicaActive {
                replica_id: test_uuid(),
            },
            replica_wal_position: WalPosition::new(replica_seq, replica_seq * 100),
            primary_committed_position: primary_seq.map(|s| WalPosition::new(s, s * 100)),
            primary_unavailable,
            force: false,
        }
    }

    #[test]
    fn test_allowed_when_caught_up_and_primary_unavailable() {
        let context = make_replica_context(100, Some(100), true);
        let result = PromotionValidator::validate(test_uuid(), &context);

        assert!(result.is_allowed());
    }

    #[test]
    fn test_denied_when_replica_behind_wal() {
        // P6-S1: No Acknowledged Write Loss
        let context = make_replica_context(90, Some(100), true);
        let result = PromotionValidator::validate(test_uuid(), &context);

        assert!(!result.is_allowed());
        assert_eq!(
            result.denial_reason(),
            Some(&DenialReason::ReplicaBehindWal)
        );
    }

    #[test]
    fn test_denied_when_primary_still_active() {
        // P6-A1: Single Write Authority
        let context = make_replica_context(100, Some(100), false);
        let result = PromotionValidator::validate(test_uuid(), &context);

        assert!(!result.is_allowed());
        assert_eq!(
            result.denial_reason(),
            Some(&DenialReason::PrimaryStillActive)
        );
    }

    #[test]
    fn test_allowed_with_force_even_if_primary_active() {
        let mut context = make_replica_context(100, Some(100), false);
        context.force = true;

        let result = PromotionValidator::validate(test_uuid(), &context);
        assert!(result.is_allowed());
    }

    #[test]
    fn test_denied_when_replication_disabled() {
        // P5-I16: Replication removable
        let context = ValidationContext {
            replica_state: ReplicationState::Disabled,
            replica_wal_position: WalPosition::new(100, 10000),
            primary_committed_position: None,
            primary_unavailable: true,
            force: false,
        };

        let result = PromotionValidator::validate(test_uuid(), &context);

        assert!(!result.is_allowed());
        assert_eq!(
            result.denial_reason(),
            Some(&DenialReason::ReplicationDisabled)
        );
    }

    #[test]
    fn test_denied_when_replication_halted() {
        let context = ValidationContext {
            replica_state: ReplicationState::ReplicationHalted {
                reason: HaltReason::AuthorityAmbiguity,
            },
            replica_wal_position: WalPosition::new(100, 10000),
            primary_committed_position: None,
            primary_unavailable: true,
            force: false,
        };

        let result = PromotionValidator::validate(test_uuid(), &context);

        assert!(!result.is_allowed());
        assert_eq!(
            result.denial_reason(),
            Some(&DenialReason::AuthorityAmbiguous)
        );
    }

    #[test]
    fn test_denied_when_already_primary() {
        let context = ValidationContext {
            replica_state: ReplicationState::PrimaryActive,
            replica_wal_position: WalPosition::new(100, 10000),
            primary_committed_position: None,
            primary_unavailable: true,
            force: false,
        };

        let result = PromotionValidator::validate(test_uuid(), &context);

        assert!(!result.is_allowed());
        assert_eq!(result.denial_reason(), Some(&DenialReason::InvalidRequest));
    }

    #[test]
    fn test_explain_allowed() {
        let result = ValidationResult::Allowed;
        let explanation = PromotionValidator::explain(&result);

        assert!(explanation.contains("allowed"));
    }

    #[test]
    fn test_explain_denied() {
        let result = ValidationResult::Denied(DenialReason::ReplicaBehindWal);
        let explanation = PromotionValidator::explain(&result);

        assert!(explanation.contains("denied"));
        assert!(explanation.contains("P6-S1"));
    }
}
