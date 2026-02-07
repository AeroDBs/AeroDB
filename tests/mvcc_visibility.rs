//! MVCC Visibility Invariant Tests
//!
//! Tests for visibility invariants per MVCC_VISIBILITY.md:
//! - Snapshot isolation
//! - Deterministic visibility evaluation
//! - Tombstone handling

use aerodb::mvcc::{CommitId, ReadView, Version, VersionChain, Visibility, VisibilityResult};

// =============================================================================
// Helper Functions
// =============================================================================

fn make_version(key: &str, data: &[u8], commit: u64) -> Version {
    Version::with_document(key.to_string(), data.to_vec(), CommitId::new(commit))
}

fn make_tombstone(key: &str, commit: u64) -> Version {
    Version::with_tombstone(key.to_string(), CommitId::new(commit))
}

// =============================================================================
// Visibility Rule Tests (MVCC_VISIBILITY.md §3)
// =============================================================================

/// Visibility selects largest commit_id ≤ read_upper_bound.
#[test]
fn test_visibility_rule_largest_within_bound() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"v1", 1));
    chain.push(make_version("key", b"v5", 5));
    chain.push(make_version("key", b"v10", 10));

    // View at 7 should see version 5 (largest <= 7)
    let view = ReadView::new(CommitId::new(7));
    let result = Visibility::visible_version(&chain, view);

    assert!(result.is_visible());
    assert_eq!(result.version().unwrap().commit_id(), CommitId::new(5));
}

/// Empty chain is invisible.
#[test]
fn test_empty_chain_invisible() {
    let chain = VersionChain::new("empty".to_string());
    let view = ReadView::new(CommitId::new(100));

    let result = Visibility::visible_version(&chain, view);
    assert!(!result.is_visible());
}

/// All versions in future = invisible.
#[test]
fn test_all_future_invisible() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"v10", 10));
    chain.push(make_version("key", b"v20", 20));

    // View at 5 cannot see any version
    let view = ReadView::new(CommitId::new(5));
    let result = Visibility::visible_version(&chain, view);

    assert!(!result.is_visible());
}

// =============================================================================
// Tombstone Tests (MVCC_VISIBILITY.md §3 step 3)
// =============================================================================

/// Tombstone makes key invisible at that snapshot.
#[test]
fn test_tombstone_makes_invisible() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"data", 1));
    chain.push(make_tombstone("key", 5));

    // View at 5 sees tombstone = invisible
    let view = ReadView::new(CommitId::new(5));
    let result = Visibility::visible_version(&chain, view);

    assert!(!result.is_visible());
}

/// Tombstone hides older versions.
#[test]
fn test_tombstone_hides_older() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"old", 1));
    chain.push(make_version("key", b"newer", 3));
    chain.push(make_tombstone("key", 5));

    // View at 5 sees tombstone, not older versions
    let view = ReadView::new(CommitId::new(5));
    let result = Visibility::visible_version(&chain, view);

    assert!(!result.is_visible());
}

/// View before tombstone sees older version.
#[test]
fn test_view_before_tombstone_sees_data() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"visible", 3));
    chain.push(make_tombstone("key", 10));

    // View at 5 cannot see tombstone at 10, sees version 3
    let view = ReadView::new(CommitId::new(5));
    let result = Visibility::visible_version(&chain, view);

    assert!(result.is_visible());
    assert_eq!(result.version().unwrap().commit_id(), CommitId::new(3));
}

/// Delete then reinsert: reinsert visible after tombstone.
#[test]
fn test_delete_then_reinsert() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"original", 1));
    chain.push(make_tombstone("key", 5));
    chain.push(make_version("key", b"reinserted", 10));

    // View at 10 sees reinserted version
    let view = ReadView::new(CommitId::new(10));
    let result = Visibility::visible_version(&chain, view);

    assert!(result.is_visible());
    assert_eq!(result.version().unwrap().commit_id(), CommitId::new(10));
}

/// View between delete and reinsert sees tombstone (invisible).
#[test]
fn test_view_between_delete_reinsert() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"original", 1));
    chain.push(make_tombstone("key", 5));
    chain.push(make_version("key", b"reinserted", 10));

    // View at 7: sees tombstone (between delete and reinsert)
    let view = ReadView::new(CommitId::new(7));
    let result = Visibility::visible_version(&chain, view);

    assert!(!result.is_visible());
}

// =============================================================================
// Snapshot Stability Tests (MVCC_VISIBILITY.md §5.1)
// =============================================================================

/// Same view evaluates to same result (deterministic).
#[test]
fn test_snapshot_stability() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"v1", 1));
    chain.push(make_version("key", b"v5", 5));

    let view = ReadView::new(CommitId::new(5));

    // Multiple evaluations must be identical
    for _ in 0..100 {
        let result = Visibility::visible_version(&chain, view);
        assert!(result.is_visible());
        assert_eq!(result.version().unwrap().commit_id(), CommitId::new(5));
    }
}

/// Views at monotonically increasing commits see monotonic versions.
#[test]
fn test_monotonic_views_monotonic_versions() {
    let mut chain = VersionChain::new("key".to_string());
    chain.push(make_version("key", b"v1", 1));
    chain.push(make_version("key", b"v5", 5));
    chain.push(make_version("key", b"v10", 10));

    let mut last_commit = CommitId::new(0);

    for bound in [1, 5, 7, 10, 15] {
        let view = ReadView::new(CommitId::new(bound));
        let result = Visibility::visible_version(&chain, view);

        if let Some(version) = result.version() {
            // Version commit must be >= previous (monotonic)
            assert!(version.commit_id() >= last_commit);
            last_commit = version.commit_id();
        }
    }
}

// =============================================================================
// Single Version Visibility Tests
// =============================================================================

/// Single version visible within bound.
#[test]
fn test_single_version_visible() {
    let version = make_version("key", b"data", 5);
    let view = ReadView::new(CommitId::new(10));

    assert!(Visibility::is_version_visible(&version, view));
}

/// Single version at exact bound is visible.
#[test]
fn test_single_version_at_bound() {
    let version = make_version("key", b"data", 5);
    let view = ReadView::new(CommitId::new(5));

    assert!(Visibility::is_version_visible(&version, view));
}

/// Single version beyond bound is invisible.
#[test]
fn test_single_version_beyond_bound() {
    let version = make_version("key", b"data", 10);
    let view = ReadView::new(CommitId::new(5));

    assert!(!Visibility::is_version_visible(&version, view));
}

/// Tombstone version is not "visible" as document.
#[test]
fn test_tombstone_version_invisible() {
    let tombstone = make_tombstone("key", 5);
    let view = ReadView::new(CommitId::new(10));

    // Tombstones are within bound but represent deleted data
    assert!(!Visibility::is_version_visible(&tombstone, view));
}
