//! Disk-Level Crash Tests for Phase 6 Promotion
//!
//! Per PHASE6_TESTING_STRATEGY.md §3.4:
//! Crash tests are MANDATORY, not optional.
//!
//! These tests use REAL file I/O to verify:
//! - Crash before marker write → old authority
//! - Crash after marker write → new authority
//! - Recovery reads ONLY disk state
//!
//! Per PHASE6_INVARIANTS.md:
//! - P6-F2: Crash-safe promotion
//! - P6-D2: Deterministic recovery
//! - P6-A2: Atomic authority transfer

use aerodb::promotion::{AuthorityMarker, AuthorityTransitionManager, DurableMarker};
use aerodb::replication::ReplicationState;
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

fn test_uuid() -> Uuid {
    Uuid::new_v4()
}

// =============================================================================
// P6-F2: Crash Before Marker Write
// =============================================================================

/// Crash before marker write → old authority (Replica)
///
/// Per PHASE6_INVARIANTS.md §P6-F2:
/// Before promotion completes → no authority change
#[test]
fn test_crash_before_marker_write_old_authority() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path();

    // Create manager and begin transition (no marker yet)
    let mut manager = AuthorityTransitionManager::new(data_dir);
    let replica_id = test_uuid();
    let state = ReplicationState::ReplicaActive { replica_id };

    manager.begin_transition(replica_id, &state).unwrap();

    // At this point, NO marker file exists
    assert!(!manager.has_durable_marker());

    // Simulate crash - just drop the manager
    drop(manager);

    // Recovery with NEW manager - reads only disk state
    let recovered_manager = AuthorityTransitionManager::new(data_dir);
    let (was_committed, id) = recovered_manager.recover_after_crash().unwrap();

    // No marker → old authority
    assert!(!was_committed);
    assert!(id.is_none());
}

// =============================================================================
// P6-F2: Crash After Marker Write
// =============================================================================

/// Crash after marker write → new authority (Primary)
///
/// Per PHASE6_INVARIANTS.md §P6-F2:
/// After promotion completes → new authority is authoritative
#[test]
fn test_crash_after_marker_write_new_authority() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path();

    let replica_id = test_uuid();

    // Create manager, begin transition, and apply (writes marker)
    let mut manager = AuthorityTransitionManager::new(data_dir);
    let state = ReplicationState::ReplicaActive { replica_id };

    manager.begin_transition(replica_id, &state).unwrap();
    manager.apply_transition().unwrap();

    // Marker MUST exist now
    assert!(manager.has_durable_marker());

    // Simulate crash - just drop the manager
    drop(manager);

    // Recovery with NEW manager - reads only disk state
    let recovered_manager = AuthorityTransitionManager::new(data_dir);
    let (was_committed, id) = recovered_manager.recover_after_crash().unwrap();

    // Marker exists → new authority
    assert!(was_committed);
    assert_eq!(id, Some(replica_id));
}

// =============================================================================
// P6-D2: Deterministic Recovery
// =============================================================================

/// Same disk state → identical recovery outcome
///
/// Per PHASE6_INVARIANTS.md §P6-D2:
/// After crash and recovery, authority state MUST be unambiguous.
#[test]
fn test_recovery_deterministic_from_disk() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path();

    let replica_id = test_uuid();

    // Write marker
    {
        let mut manager = AuthorityTransitionManager::new(data_dir);
        let state = ReplicationState::ReplicaActive { replica_id };
        manager.begin_transition(replica_id, &state).unwrap();
        manager.apply_transition().unwrap();
    }

    // Multiple independent recoveries must produce identical results
    let mgr1 = AuthorityTransitionManager::new(data_dir);
    let mgr2 = AuthorityTransitionManager::new(data_dir);
    let mgr3 = AuthorityTransitionManager::new(data_dir);

    let result1 = mgr1.recover_after_crash().unwrap();
    let result2 = mgr2.recover_after_crash().unwrap();
    let result3 = mgr3.recover_after_crash().unwrap();

    assert_eq!(result1, result2);
    assert_eq!(result2, result3);

    // All should see committed with correct ID
    assert!(result1.0);
    assert_eq!(result1.1, Some(replica_id));
}

// =============================================================================
// P6-A2: Marker Atomicity
// =============================================================================

/// Marker file uses atomic write pattern (write-then-rename)
///
/// Per PHASE6_ARCHITECTURE.md §4.2:
/// Atomicity is achieved via write + fsync + rename
#[test]
fn test_marker_atomicity_no_partial_state() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path();

    let marker = DurableMarker::new(data_dir);

    // Before write - no marker
    assert!(!marker.exists());

    // Write marker
    let auth_marker = AuthorityMarker::new(test_uuid(), "ReplicaActive");
    marker.write_atomic(&auth_marker).unwrap();

    // After write - marker exists
    assert!(marker.exists());

    // Read should succeed and be complete
    let read = marker.read().unwrap();
    assert!(read.is_some());
}

// =============================================================================
// Binary State Tests
// =============================================================================

/// Only two possible states: marker present or absent
///
/// Per PHASE6_ARCHITECTURE.md §4.2:
/// This marker is atomic: present = new authority, absent = old authority
#[test]
fn test_binary_state_present() {
    let tmp = TempDir::new().unwrap();

    let dm = DurableMarker::new(tmp.path());
    let marker = AuthorityMarker::new(test_uuid(), "ReplicaActive");

    dm.write_atomic(&marker).unwrap();

    // State: PRESENT
    assert!(dm.exists());
    assert!(dm.read().unwrap().is_some());
}

#[test]
fn test_binary_state_absent() {
    let tmp = TempDir::new().unwrap();

    let dm = DurableMarker::new(tmp.path());

    // State: ABSENT
    assert!(!dm.exists());
    assert!(dm.read().unwrap().is_none());
}

// =============================================================================
// Recovery Uses Only Disk State
// =============================================================================

/// Recovery does NOT use any injected or in-memory state
///
/// Per PHASE6_INVARIANTS.md §P6-D2:
/// Recovery MUST NOT "decide" promotion outcomes.
#[test]
fn test_recovery_reads_only_disk_state() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path();

    // Scenario 1: No marker file at all
    {
        let manager = AuthorityTransitionManager::new(data_dir);
        let (was_committed, _) = manager.recover_after_crash().unwrap();
        assert!(!was_committed);
    }

    // Scenario 2: Create marker file directly (simulating post-crash state)
    let replica_id = test_uuid();
    {
        let dm = DurableMarker::new(data_dir);
        let marker = AuthorityMarker::new(replica_id, "ReplicaActive");
        dm.write_atomic(&marker).unwrap();
    }

    // Scenario 3: Recovery finds the marker
    {
        let manager = AuthorityTransitionManager::new(data_dir);
        let (was_committed, id) = manager.recover_after_crash().unwrap();
        assert!(was_committed);
        assert_eq!(id, Some(replica_id));
    }
}

// =============================================================================
// Marker File Persistence Across Process Boundaries
// =============================================================================

/// Marker persists across manager instances (simulates process restart)
#[test]
fn test_marker_persists_across_instances() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path();
    let replica_id = test_uuid();

    // Instance 1: Write marker
    {
        let mut manager = AuthorityTransitionManager::new(data_dir);
        let state = ReplicationState::ReplicaActive { replica_id };
        manager.begin_transition(replica_id, &state).unwrap();
        manager.apply_transition().unwrap();
    }

    // Instance 2: Read marker
    {
        let manager = AuthorityTransitionManager::new(data_dir);
        assert!(manager.has_durable_marker());
        let (was_committed, id) = manager.recover_after_crash().unwrap();
        assert!(was_committed);
        assert_eq!(id, Some(replica_id));
    }
}

// =============================================================================
// Complete Lifecycle Across Crash
// =============================================================================

/// Full lifecycle with simulated crash at each phase
#[test]
fn test_full_lifecycle_crash_simulation() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path();
    let replica_id = test_uuid();

    // Phase 1: Begin transition (crash would result in no change)
    {
        let mut manager = AuthorityTransitionManager::new(data_dir);
        let state = ReplicationState::ReplicaActive { replica_id };
        manager.begin_transition(replica_id, &state).unwrap();
        // "crash" here
    }

    // Recovery after Phase 1 crash
    {
        let manager = AuthorityTransitionManager::new(data_dir);
        let (was_committed, _) = manager.recover_after_crash().unwrap();
        assert!(!was_committed); // No marker = old authority
    }

    // Phase 2: Apply transition (writes marker)
    {
        let mut manager = AuthorityTransitionManager::new(data_dir);
        let state = ReplicationState::ReplicaActive { replica_id };
        manager.begin_transition(replica_id, &state).unwrap();
        manager.apply_transition().unwrap();
        // "crash" here - marker already written
    }

    // Recovery after Phase 2 crash
    {
        let manager = AuthorityTransitionManager::new(data_dir);
        let (was_committed, id) = manager.recover_after_crash().unwrap();
        assert!(was_committed); // Marker exists = new authority
        assert_eq!(id, Some(replica_id));
    }
}

// =============================================================================
// Marker File Content Verification
// =============================================================================

/// Marker file contains expected content on disk
#[test]
fn test_marker_file_content_on_disk() {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path();
    let replica_id = test_uuid();

    let dm = DurableMarker::new(data_dir);
    let marker = AuthorityMarker::new(replica_id, "ReplicaActive");
    dm.write_atomic(&marker).unwrap();

    // Read raw file content
    let marker_path = dm.marker_path();
    let content = fs::read_to_string(marker_path).unwrap();

    // Verify it contains expected data
    assert!(content.contains(&replica_id.to_string()));
    assert!(content.contains("ReplicaActive"));
}
