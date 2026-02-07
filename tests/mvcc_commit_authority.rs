//! MVCC Commit Authority Tests
//!
//! Tests for commit authority invariants per MVCC_WAL_INTERACTION.md:
//! - WAL is sole source of truth for commit ordering
//! - Commit identities assigned exactly once
//! - Monotonic commit ordering

use aerodb::mvcc::{CommitAuthority, CommitId};

// =============================================================================
// Commit Authority Initialization Tests
// =============================================================================

/// New authority starts at zero.
#[test]
fn test_new_authority_starts_zero() {
    let authority = CommitAuthority::new();
    assert!(authority.highest_commit_id().is_none());
}

/// First commit is 1.
#[test]
fn test_first_commit_is_one() {
    let authority = CommitAuthority::new();
    let next = authority.next_commit_id();
    assert_eq!(next, CommitId::new(1));
}

// =============================================================================
// Commit Ordering Tests
// =============================================================================

/// Commits are strictly monotonic.
#[test]
fn test_commits_strictly_monotonic() {
    let mut authority = CommitAuthority::new();

    for i in 1..=10 {
        let next = authority.next_commit_id();
        assert_eq!(next, CommitId::new(i));
        authority.mark_committed(next).unwrap();
        assert_eq!(authority.highest_commit_id(), Some(CommitId::new(i)));
    }
}

/// Cannot commit out of order.
#[test]
fn test_cannot_commit_out_of_order() {
    let mut authority = CommitAuthority::new();

    // Get next (1)
    let _ = authority.next_commit_id();

    // Try to commit 5 directly - should fail
    let result = authority.mark_committed(CommitId::new(5));
    assert!(result.is_err());
}

// =============================================================================
// Replay Tests
// =============================================================================

/// Replayed commits establish authority.
#[test]
fn test_replayed_commit_establishes_authority() {
    let authority = CommitAuthority::from_replayed_commit(100);
    assert_eq!(authority.highest_commit_id(), Some(CommitId::new(100)));
    assert_eq!(authority.next_commit_id(), CommitId::new(101));
}

/// Replayed commits must be monotonic.
#[test]
fn test_replayed_commits_must_be_monotonic() {
    let mut authority = CommitAuthority::from_replayed_commit(10);

    // Can observe higher
    assert!(authority.observe_replayed_commit(15).is_ok());
    assert!(authority.observe_replayed_commit(20).is_ok());

    // Cannot observe lower
    assert!(authority.observe_replayed_commit(5).is_err());
}

/// Cannot replay duplicate commit.
#[test]
fn test_cannot_replay_duplicate() {
    let mut authority = CommitAuthority::from_replayed_commit(10);

    // Same commit again should fail
    let result = authority.observe_replayed_commit(10);
    assert!(result.is_err());
}

/// Replay produces deterministic result.
#[test]
fn test_replay_deterministic() {
    // Simulate same replay sequence twice
    let replay_sequence = vec![1, 5, 10, 15, 20];

    // First replay
    let mut auth1 = CommitAuthority::new();
    for &commit in &replay_sequence {
        auth1.observe_replayed_commit(commit).unwrap();
    }

    // Second replay
    let mut auth2 = CommitAuthority::new();
    for &commit in &replay_sequence {
        auth2.observe_replayed_commit(commit).unwrap();
    }

    // Both should have identical state
    assert_eq!(auth1.highest_commit_id(), auth2.highest_commit_id());
    assert_eq!(auth1.next_commit_id(), auth2.next_commit_id());
}

// =============================================================================
// Snapshot Tests
// =============================================================================

/// Current snapshot reflects highest commit.
#[test]
fn test_snapshot_reflects_highest() {
    let mut authority = CommitAuthority::new();

    let next = authority.next_commit_id();
    authority.mark_committed(next).unwrap();

    let snapshot = authority.current_snapshot();
    assert_eq!(snapshot.upper_bound(), CommitId::new(1));
}

/// Snapshot at zero is valid (empty database).
#[test]
fn test_snapshot_at_zero_valid() {
    let authority = CommitAuthority::new();
    let snapshot = authority.current_snapshot();

    // Zero commit means no visible versions
    assert_eq!(snapshot.upper_bound(), CommitId::new(0));
}

/// Snapshot after multiple commits.
#[test]
fn test_snapshot_after_multiple_commits() {
    let mut authority = CommitAuthority::new();

    for _ in 0..5 {
        let next = authority.next_commit_id();
        authority.mark_committed(next).unwrap();
    }

    let snapshot = authority.current_snapshot();
    assert_eq!(snapshot.upper_bound(), CommitId::new(5));
}
