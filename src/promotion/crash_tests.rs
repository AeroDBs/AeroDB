//! Crash & Recovery Tests for Phase 6
//!
//! Per PHASE6_TESTING_STRATEGY.md §3.4:
//! Crash tests are MANDATORY, not optional.
//!
//! Required Crash Points:
//! - Before validation
//! - During validation
//! - After validation, before authority transition
//! - During authority transition
//! - Immediately after authority transition
//!
//! Required Outcomes:
//! - No dual-primary state
//! - No lost acknowledged writes
//! - Deterministic recovery
//! - Explicit abort or completion

#[cfg(test)]
mod tests {
    use crate::promotion::*;
    use crate::replication::{ReplicationState, WalPosition};
    use tempfile::TempDir;
    use uuid::Uuid;

    fn test_uuid() -> Uuid {
        Uuid::new_v4()
    }

    fn make_manager() -> (TempDir, AuthorityTransitionManager) {
        let tmp = TempDir::new().unwrap();
        let manager = AuthorityTransitionManager::new(tmp.path());
        (tmp, manager)
    }

    // =========================================================================
    // CRASH BEFORE VALIDATION (§3.4.1)
    // =========================================================================

    #[test]
    fn test_crash_before_validation_no_authority_change() {
        // Per PHASE6_FAILURE_MODEL.md §3.2:
        // Crash before validation begins → No authority change

        let mut controller = PromotionController::new();
        let replica_id = test_uuid();

        // Request promotion
        controller.request_promotion(PromotionRequest::new(replica_id));
        assert_eq!(controller.state_name(), "PromotionRequested");

        // Simulate crash and recovery
        let recovered = PromotionController::recover_after_crash(false, None);

        // Authority unchanged
        assert_eq!(recovered.state_name(), "Steady");
        assert!(!recovered.is_promotion_in_progress());
    }

    // =========================================================================
    // CRASH DURING VALIDATION (§3.4.2)
    // =========================================================================

    #[test]
    fn test_crash_during_validation_no_authority_change() {
        // Per PHASE6_FAILURE_MODEL.md §3.2:
        // Crash while evaluating → Promotion is considered NOT attempted

        let mut controller = PromotionController::new();
        let replica_id = test_uuid();

        controller.request_promotion(PromotionRequest::new(replica_id));
        controller.begin_validation().unwrap();
        assert_eq!(controller.state_name(), "PromotionValidating");

        // Simulate crash
        let recovered = PromotionController::recover_after_crash(false, None);

        // Promotion forgotten
        assert_eq!(recovered.state_name(), "Steady");
    }

    // =========================================================================
    // CRASH AFTER VALIDATION, BEFORE TRANSITION (§3.4.3)
    // =========================================================================

    #[test]
    fn test_crash_after_approval_before_transition() {
        // Per PHASE6_FAILURE_MODEL.md §3.3:
        // Crash after validation succeeds, before authority transition
        // → Promotion is NOT applied

        let mut controller = PromotionController::new();
        let replica_id = test_uuid();

        controller.request_promotion(PromotionRequest::new(replica_id));
        controller.begin_validation().unwrap();
        controller.approve_promotion().unwrap();
        assert_eq!(controller.state_name(), "PromotionApproved");

        // Simulate crash (approval has no durable effect)
        let recovered = PromotionController::recover_after_crash(false, None);

        // Authority unchanged, must re-run validation
        assert_eq!(recovered.state_name(), "Steady");
    }

    // =========================================================================
    // CRASH DURING AUTHORITY TRANSITION (§3.4.4)
    // =========================================================================

    #[test]
    fn test_crash_during_transition_atomic_outcome() {
        // Per PHASE6_FAILURE_MODEL.md §3.4:
        // Authority transition MUST be atomic
        // Recovery observes either old or new authority, never mixed

        let replica_id = test_uuid();

        // Scenario A: Crash before atomic commit
        let recovered = PromotionController::recover_after_crash(false, None);
        assert_eq!(recovered.state_name(), "Steady");

        // Scenario B: Crash after atomic commit
        let recovered = PromotionController::recover_after_crash(true, Some(replica_id));
        assert_eq!(recovered.state_name(), "PromotionSucceeded");
    }

    // =========================================================================
    // CRASH AFTER PROMOTION COMPLETES (§3.4.5)
    // =========================================================================

    #[test]
    fn test_crash_after_promotion_authority_preserved() {
        // Per PHASE6_FAILURE_MODEL.md §3.5:
        // Crash immediately after promotion completes
        // → New authority state is authoritative

        let replica_id = test_uuid();
        let recovered = PromotionController::recover_after_crash(true, Some(replica_id));

        // Authority is the new primary
        match recovered.state() {
            PromotionState::PromotionSucceeded { new_primary_id } => {
                assert_eq!(*new_primary_id, replica_id);
            }
            _ => panic!("expected PromotionSucceeded after crash recovery"),
        }
    }

    // =========================================================================
    // AUTHORITY TRANSITION MANAGER CRASH TESTS
    // =========================================================================

    #[test]
    fn test_transition_manager_abort_before_atomic() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        manager.begin_transition(replica_id, &state).unwrap();

        // Abort is allowed before atomic marker
        assert!(manager.abort_transition().is_ok());

        let (was_atomic, _) = manager.recover_after_crash().unwrap();
        assert!(!was_atomic);
    }

    #[test]
    fn test_transition_manager_cannot_abort_after_atomic() {
        let (_tmp, mut manager) = make_manager();
        let replica_id = test_uuid();
        let state = ReplicationState::ReplicaActive { replica_id };

        manager.begin_transition(replica_id, &state).unwrap();
        manager.apply_transition().unwrap();

        // Abort is forbidden after atomic marker
        assert!(manager.abort_transition().is_err());
    }

    // =========================================================================
    // END-TO-END TESTS (Stage 6.8)
    // =========================================================================

    #[test]
    fn test_e2e_successful_promotion_path() {
        // Per PHASE6_TESTING_STRATEGY.md §3.5:
        // Successful promotion path

        let replica_id = test_uuid();
        let mut controller = PromotionController::new();
        let (_tmp, mut manager) = make_manager();
        let mut observer = PromotionObserver::new();

        // Step 1: Request
        let result = controller.request_promotion(PromotionRequest::new(replica_id));
        assert!(result.is_accepted());
        observer.emit_state_transition(
            &PromotionState::Steady,
            &PromotionState::PromotionRequested { replica_id },
        );

        // Step 2: Validate
        controller.begin_validation().unwrap();

        let context = ValidationContext {
            replica_state: ReplicationState::ReplicaActive { replica_id },
            replica_wal_position: WalPosition::new(100, 10000),
            primary_committed_position: Some(WalPosition::new(100, 10000)),
            primary_unavailable: true,
            force: false,
        };
        let validation = PromotionValidator::validate(replica_id, &context);
        assert!(validation.is_allowed());

        // Step 3: Approve
        controller.approve_promotion().unwrap();

        // Step 4: Transition
        controller.begin_authority_transition().unwrap();
        let state = ReplicationState::ReplicaActive { replica_id };
        manager.begin_transition(replica_id, &state).unwrap();
        let new_state = manager.apply_transition().unwrap();
        assert_eq!(new_state, ReplicationState::PrimaryActive);

        // Step 5: Complete
        controller.complete_transition().unwrap();
        manager.complete_transition().unwrap();

        // Step 6: Acknowledge
        controller.acknowledge_success().unwrap();
        assert_eq!(controller.state_name(), "Steady");

        // Verify observability
        assert!(observer.events().len() >= 1);
    }

    #[test]
    fn test_e2e_denied_promotion_path() {
        // Per PHASE6_TESTING_STRATEGY.md §3.5:
        // Denied promotion path

        let replica_id = test_uuid();
        let mut controller = PromotionController::new();

        // Request
        controller.request_promotion(PromotionRequest::new(replica_id));
        controller.begin_validation().unwrap();

        // Validate - replica is behind WAL
        let context = ValidationContext {
            replica_state: ReplicationState::ReplicaActive { replica_id },
            replica_wal_position: WalPosition::new(90, 9000),
            primary_committed_position: Some(WalPosition::new(100, 10000)),
            primary_unavailable: true,
            force: false,
        };
        let validation = PromotionValidator::validate(replica_id, &context);
        assert!(!validation.is_allowed());
        assert_eq!(
            validation.denial_reason(),
            Some(&DenialReason::ReplicaBehindWal)
        );

        // Deny
        controller
            .deny_promotion(DenialReason::ReplicaBehindWal)
            .unwrap();
        assert_eq!(controller.state_name(), "PromotionDenied");

        // Acknowledge
        controller.acknowledge_denial().unwrap();
        assert_eq!(controller.state_name(), "Steady");
    }

    #[test]
    fn test_e2e_repeated_promotion_attempts() {
        // Per PHASE6_TESTING_STRATEGY.md §3.5:
        // Repeated attempts

        let replica_id = test_uuid();
        let mut controller = PromotionController::new();

        // Attempt 1: Denied
        controller.request_promotion(PromotionRequest::new(replica_id));
        controller.begin_validation().unwrap();
        controller
            .deny_promotion(DenialReason::ReplicaBehindWal)
            .unwrap();
        controller.acknowledge_denial().unwrap();

        // Back to steady - can try again
        assert_eq!(controller.state_name(), "Steady");

        // Attempt 2: Should be allowed
        let result = controller.request_promotion(PromotionRequest::new(replica_id));
        assert!(result.is_accepted());
    }

    #[test]
    fn test_e2e_disablement_behavior() {
        // Per PHASE6_TESTING_STRATEGY.md §3.6:
        // If Phase 6 logic is disabled, system behaves like Phase 5

        let replica_id = test_uuid();

        // When replication is disabled, promotion is denied
        let context = ValidationContext {
            replica_state: ReplicationState::Disabled,
            replica_wal_position: WalPosition::new(100, 10000),
            primary_committed_position: None,
            primary_unavailable: true,
            force: false,
        };

        let validation = PromotionValidator::validate(replica_id, &context);
        assert!(!validation.is_allowed());
        assert_eq!(
            validation.denial_reason(),
            Some(&DenialReason::ReplicationDisabled)
        );
    }

    // =========================================================================
    // NEGATIVE TESTS (Required per §4)
    // =========================================================================

    #[test]
    fn test_negative_promotion_with_stale_replica() {
        let replica_id = test_uuid();
        let context = ValidationContext {
            replica_state: ReplicationState::ReplicaActive { replica_id },
            replica_wal_position: WalPosition::new(50, 5000),
            primary_committed_position: Some(WalPosition::new(100, 10000)),
            primary_unavailable: true,
            force: false,
        };

        let validation = PromotionValidator::validate(replica_id, &context);
        assert!(!validation.is_allowed());
    }

    #[test]
    fn test_negative_promotion_under_ambiguous_authority() {
        let replica_id = test_uuid();
        let context = ValidationContext {
            replica_state: ReplicationState::ReplicationHalted {
                reason: crate::replication::HaltReason::AuthorityAmbiguity,
            },
            replica_wal_position: WalPosition::new(100, 10000),
            primary_committed_position: None,
            primary_unavailable: true,
            force: false,
        };

        let validation = PromotionValidator::validate(replica_id, &context);
        assert!(!validation.is_allowed());
        assert_eq!(
            validation.denial_reason(),
            Some(&DenialReason::AuthorityAmbiguous)
        );
    }
}
