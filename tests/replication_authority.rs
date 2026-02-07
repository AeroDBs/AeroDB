//! Replication Authority Tests
//!
//! Tests for invariants per PHASE2_REPLICATION_INVARIANTS.md:
//! - Single-writer invariant: exactly one Primary
//! - Commit authority invariant: only Primary assigns CommitId
//!
//! Per REPLICATION_MODEL.md §7:
//! - Write admission must be explicit
//! - Dual Primary is forbidden

use aerodb::replication::{
    check_commit_authority, check_dual_primary, check_write_admission, AuthorityCheck, HaltReason,
    ReplicationState, WriteAdmission,
};
use uuid::Uuid;

// =============================================================================
// Write Admission Tests
// =============================================================================

/// Primary admits writes.
#[test]
fn test_primary_admits_writes() {
    let state = ReplicationState::new().become_primary().unwrap();
    let admission = check_write_admission(&state);

    assert!(admission.is_admitted());
    assert!(matches!(admission, WriteAdmission::Admitted));
}

/// Replica rejects writes.
#[test]
fn test_replica_rejects_writes() {
    let replica_id = Uuid::new_v4();
    let state = ReplicationState::new().become_replica(replica_id).unwrap();
    let admission = check_write_admission(&state);

    assert!(!admission.is_admitted());
    assert!(matches!(admission, WriteAdmission::RejectedReplica));
}

/// Halted rejects writes.
#[test]
fn test_halted_rejects_writes() {
    let state = ReplicationState::new().halt(HaltReason::AuthorityAmbiguity);
    let admission = check_write_admission(&state);

    assert!(!admission.is_admitted());
    assert!(matches!(admission, WriteAdmission::RejectedHalted));
}

/// Disabled mode allows writes (standalone primary behavior per P5-I16).
#[test]
fn test_disabled_admits_writes() {
    let state = ReplicationState::new(); // Disabled by default
    let admission = check_write_admission(&state);

    assert!(admission.is_admitted());
}

// =============================================================================
// Commit Authority Tests
// =============================================================================

/// Primary has commit authority.
#[test]
fn test_primary_has_commit_authority() {
    let state = ReplicationState::new().become_primary().unwrap();
    let result = check_commit_authority(&state);

    assert!(result.is_ok());
}

/// Replica has no commit authority.
#[test]
fn test_replica_no_commit_authority() {
    let replica_id = Uuid::new_v4();
    let state = ReplicationState::new().become_replica(replica_id).unwrap();
    let result = check_commit_authority(&state);

    assert!(result.is_err());
}

/// Disabled mode has commit authority (standalone primary).
#[test]
fn test_disabled_has_commit_authority() {
    let state = ReplicationState::new();
    let result = check_commit_authority(&state);

    assert!(result.is_ok());
}

// =============================================================================
// Dual Primary Detection Tests
// =============================================================================

/// Dual primary detected when another node claims primary.
#[test]
fn test_dual_primary_detected() {
    let state = ReplicationState::new().become_primary().unwrap();
    let check = check_dual_primary(&state, true);

    assert!(matches!(check, AuthorityCheck::Ambiguous));
}

/// No conflict when primary and other is not primary.
#[test]
fn test_no_dual_primary_when_alone() {
    let state = ReplicationState::new().become_primary().unwrap();
    let check = check_dual_primary(&state, false);

    assert!(matches!(check, AuthorityCheck::Authorized));
}

/// Replica never authorized for writes even without conflict.
#[test]
fn test_replica_not_authorized() {
    let replica_id = Uuid::new_v4();
    let state = ReplicationState::new().become_replica(replica_id).unwrap();
    let check = check_dual_primary(&state, false);

    assert!(matches!(check, AuthorityCheck::NotAuthorized));
}

// =============================================================================
// State Transition Tests
// =============================================================================

/// Default state is Disabled.
#[test]
fn test_default_state_is_disabled() {
    let state = ReplicationState::new();
    assert!(state.is_disabled());
    assert!(state.can_write()); // Disabled behaves like standalone primary
}

/// Can transition from Disabled to Primary.
#[test]
fn test_disabled_to_primary() {
    let state = ReplicationState::new();
    let result = state.become_primary();

    assert!(result.is_ok());
    let primary = result.unwrap();
    assert!(primary.is_primary());
    assert!(primary.can_write());
}

/// Can transition from Disabled to Replica.
#[test]
fn test_disabled_to_replica() {
    let state = ReplicationState::new();
    let replica_id = Uuid::new_v4();
    let result = state.become_replica(replica_id);

    assert!(result.is_ok());
    let replica = result.unwrap();
    assert!(replica.is_replica());
    assert!(!replica.can_write());
}

/// Any state can transition to Halted.
#[test]
fn test_any_state_can_halt() {
    let states = vec![
        ReplicationState::new(),
        ReplicationState::new().become_primary().unwrap(),
        ReplicationState::new()
            .become_replica(Uuid::new_v4())
            .unwrap(),
    ];

    for state in states {
        let halted = state.halt(HaltReason::WalCorruption);
        assert!(halted.is_halted());
        assert!(!halted.can_write());
        assert!(!halted.can_read());
    }
}
