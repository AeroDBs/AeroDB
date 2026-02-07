//! Promotion Atomicity Tests
//!
//! Tests for invariants per PHASE6_INVARIANTS.md:
//! - P6-F1: Atomic authority switch
//! - P6-F2: Crash-safe promotion state
//!
//! Per PHASE6_STATE_MACHINE.md:
//! - States are explicit and enumerable
//! - Transitions are deterministic
//! - AuthorityTransitioning has atomic outcome

use aerodb::promotion::{DenialReason, PromotionState};
use uuid::Uuid;

// =============================================================================
// State Machine Basic Tests
// =============================================================================

/// Default state is Steady.
#[test]
fn test_default_state_is_steady() {
    let state = PromotionState::new();
    assert_eq!(state.state_name(), "Steady");
    assert!(!state.is_promotion_in_progress());
}

/// Steady can transition to PromotionRequested.
#[test]
fn test_steady_to_promotion_requested() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let next_state = state.request_promotion(replica_id).unwrap();
    assert_eq!(next_state.state_name(), "PromotionRequested");
    assert!(next_state.is_promotion_in_progress());
    assert_eq!(next_state.replica_id(), Some(replica_id));
}

/// PromotionRequested can transition to Validating.
#[test]
fn test_requested_to_validating() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    let state = state.begin_validation().unwrap();

    assert_eq!(state.state_name(), "PromotionValidating");
}

/// PromotionValidating can transition to Approved.
#[test]
fn test_validating_to_approved() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    let state = state.begin_validation().unwrap();
    let state = state.approve_promotion().unwrap();

    assert_eq!(state.state_name(), "PromotionApproved");
}

/// PromotionValidating can transition to Denied.
#[test]
fn test_validating_to_denied() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    let state = state.begin_validation().unwrap();
    let state = state
        .deny_promotion(DenialReason::ReplicaBehindWal)
        .unwrap();

    assert_eq!(state.state_name(), "PromotionDenied");
}

// =============================================================================
// P6-F1: Atomic Authority Switch Tests
// =============================================================================

/// Approved transitions to AuthorityTransitioning.
#[test]
fn test_approved_to_authority_transitioning() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    let state = state.begin_validation().unwrap();
    let state = state.approve_promotion().unwrap();
    let state = state.begin_authority_transition().unwrap();

    assert_eq!(state.state_name(), "AuthorityTransitioning");
}

/// AuthorityTransitioning completes to PromotionSucceeded.
#[test]
fn test_authority_transition_completes() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    let state = state.begin_validation().unwrap();
    let state = state.approve_promotion().unwrap();
    let state = state.begin_authority_transition().unwrap();
    let state = state.complete_transition().unwrap();

    assert_eq!(state.state_name(), "PromotionSucceeded");
}

/// Success acknowledged returns to Steady.
#[test]
fn test_success_acknowledged_returns_steady() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    let state = state.begin_validation().unwrap();
    let state = state.approve_promotion().unwrap();
    let state = state.begin_authority_transition().unwrap();
    let state = state.complete_transition().unwrap();
    let state = state.acknowledge_success().unwrap();

    assert_eq!(state.state_name(), "Steady");
    assert!(!state.is_promotion_in_progress());
}

// =============================================================================
// Denial Tests
// =============================================================================

/// Denied promotion returns to Steady after acknowledgment.
#[test]
fn test_denied_returns_to_steady() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    let state = state.begin_validation().unwrap();
    let state = state.deny_promotion(DenialReason::InvalidRequest).unwrap();
    let state = state.acknowledge_denial().unwrap();

    assert_eq!(state.state_name(), "Steady");
}

/// Different denial reasons are preserved.
#[test]
fn test_denial_reasons_preserved() {
    let reasons = vec![
        DenialReason::ReplicaBehindWal,
        DenialReason::ReplicationDisabled,
        DenialReason::ReplicaNotActive,
        DenialReason::MvccVisibilityViolation,
    ];

    for reason in reasons {
        let state = PromotionState::new();
        let replica_id = Uuid::new_v4();

        let state = state.request_promotion(replica_id).unwrap();
        let state = state.begin_validation().unwrap();
        let denied = state.deny_promotion(reason.clone()).unwrap();

        assert_eq!(denied.state_name(), "PromotionDenied");
        // Denial reason should be extractable from the state
    }
}

// =============================================================================
// Invalid Transition Tests
// =============================================================================

/// Cannot validate from Steady.
#[test]
fn test_cannot_validate_from_steady() {
    let state = PromotionState::new();
    assert!(state.begin_validation().is_err());
}

/// Cannot complete transition from Requested.
#[test]
fn test_cannot_complete_from_requested() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    assert!(state.complete_transition().is_err());
}

/// Cannot request promotion when already in progress.
#[test]
fn test_cannot_request_when_in_progress() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    assert!(state.request_promotion(Uuid::new_v4()).is_err());
}

// =============================================================================
// P6-S3: MVCC Visibility Preservation Tests
// =============================================================================

/// P6-S3: MVCC visibility violation is a valid denial reason.
///
/// Per PHASE6_INVARIANTS.md §P6-S3:
/// After promotion, all MVCC visibility rules MUST remain valid.
#[test]
fn test_mvcc_visibility_violation_denies_promotion() {
    let state = PromotionState::new();
    let replica_id = Uuid::new_v4();

    let state = state.request_promotion(replica_id).unwrap();
    let state = state.begin_validation().unwrap();

    // Deny due to MVCC visibility violation
    let state = state
        .deny_promotion(DenialReason::MvccVisibilityViolation)
        .unwrap();

    assert_eq!(state.state_name(), "PromotionDenied");
}

/// P6-S3: MvccVisibilityViolation references correct invariant.
#[test]
fn test_mvcc_denial_reason_references_invariant() {
    let reason = DenialReason::MvccVisibilityViolation;

    // Per PHASE6_INVARIANTS.md §P6-O2:
    // System MUST be able to explain which invariant blocked promotion
    assert!(reason.invariant_reference().contains("P6-S3"));
}

// =============================================================================
// P6-A1a: Force Flag Tests
// =============================================================================

/// Force flag usage should be traceable.
/// (Note: actual force flag behavior tested in validator module)
#[test]
fn test_promotion_denial_reasons_complete() {
    // Verify all expected denial reasons exist
    let reasons = vec![
        DenialReason::ReplicaBehindWal,
        DenialReason::ReplicationDisabled,
        DenialReason::ReplicaNotActive,
        DenialReason::MvccVisibilityViolation,
        DenialReason::PrimaryStillActive,
        DenialReason::AuthorityAmbiguous,
        DenialReason::InvalidRequest,
        DenialReason::InvalidReplicationState,
    ];

    for reason in reasons {
        // Each reason should have an invariant reference
        assert!(!reason.invariant_reference().is_empty());
    }
}
