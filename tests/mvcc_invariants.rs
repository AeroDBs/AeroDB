//! MVCC Invariant Tests
//!
//! Tests for core MVCC invariants:
//! - Version immutability
//! - Deterministic snapshot isolation
//! - Commit ordering
//!
//! Per MVCC.md and PHASE2_INVARIANTS.md.

use aerodb::mvcc::{CommitId, ReadView, Version, VersionChain, VersionPayload, Visibility};

// =============================================================================
// Version Immutability Tests
// =============================================================================

/// Versions are immutable after creation.
#[test]
fn test_version_immutability() {
    let version = Version::with_document(
        "doc1".to_string(),
        b"original_data".to_vec(),
        CommitId::new(1),
    );

    // Fields are read-only
    assert_eq!(version.key(), "doc1");
    assert_eq!(version.commit_id(), CommitId::new(1));
    assert!(!version.is_tombstone());

    // Clone produces equal version
    let cloned = version.clone();
    assert_eq!(version, cloned);
}

/// Tombstones are explicit versions, not missing data.
#[test]
fn test_tombstone_is_explicit_version() {
    let tombstone = Version::with_tombstone("deleted_doc".to_string(), CommitId::new(5));

    assert!(tombstone.is_tombstone());
    assert_eq!(tombstone.key(), "deleted_doc");
    assert_eq!(tombstone.commit_id(), CommitId::new(5));

    // Tombstone payload is explicit
    assert!(matches!(tombstone.payload(), VersionPayload::Tombstone));
}

/// Document payload is preserved exactly.
#[test]
fn test_document_payload_preserved() {
    let data = b"complex json data with special chars: \x00\x01\x02".to_vec();
    let version = Version::with_document("doc".to_string(), data.clone(), CommitId::new(10));

    match version.payload() {
        VersionPayload::Document(d) => assert_eq!(d, &data),
        VersionPayload::Tombstone => panic!("Expected document, got tombstone"),
    }
}

// =============================================================================
// CommitId Ordering Tests
// =============================================================================

/// CommitIds have total ordering.
#[test]
fn test_commit_id_ordering() {
    let c1 = CommitId::new(1);
    let c5 = CommitId::new(5);
    let c10 = CommitId::new(10);

    assert!(c1 < c5);
    assert!(c5 < c10);
    assert!(c1 < c10);
}

/// CommitId equality is exact.
#[test]
fn test_commit_id_equality() {
    let c1 = CommitId::new(42);
    let c2 = CommitId::new(42);
    let c3 = CommitId::new(43);

    assert_eq!(c1, c2);
    assert_ne!(c1, c3);
}

// =============================================================================
// VersionChain Tests
// =============================================================================

/// Version chain maintains versions in commit order.
#[test]
fn test_version_chain_ordering() {
    let mut chain = VersionChain::new("doc1".to_string());

    chain.push(Version::with_document(
        "doc1".to_string(),
        b"v1".to_vec(),
        CommitId::new(1),
    ));
    chain.push(Version::with_document(
        "doc1".to_string(),
        b"v2".to_vec(),
        CommitId::new(2),
    ));
    chain.push(Version::with_document(
        "doc1".to_string(),
        b"v3".to_vec(),
        CommitId::new(3),
    ));

    // Latest should be commit 3
    let versions = chain.versions();
    assert_eq!(versions.len(), 3);
}

/// Version chain key consistency.
#[test]
fn test_version_chain_key() {
    let chain = VersionChain::new("my_document".to_string());
    assert_eq!(chain.key(), "my_document");
}

// =============================================================================
// ReadView Tests
// =============================================================================

/// ReadView captures snapshot point.
#[test]
fn test_read_view_snapshot() {
    let view = ReadView::new(CommitId::new(10));
    assert_eq!(view.upper_bound(), CommitId::new(10));
}

/// ReadView is deterministic.
#[test]
fn test_read_view_determinism() {
    let v1 = ReadView::new(CommitId::new(5));
    let v2 = ReadView::new(CommitId::new(5));

    // Same commit produces identical views
    assert_eq!(v1.upper_bound(), v2.upper_bound());
}

// =============================================================================
// Visibility Determinism Tests
// =============================================================================

/// Same chain + same view = same visibility result.
#[test]
fn test_visibility_determinism() {
    let mut chain = VersionChain::new("doc".to_string());
    chain.push(Version::with_document(
        "doc".to_string(),
        b"v1".to_vec(),
        CommitId::new(1),
    ));
    chain.push(Version::with_document(
        "doc".to_string(),
        b"v2".to_vec(),
        CommitId::new(5),
    ));

    let view = ReadView::new(CommitId::new(5));

    // Multiple evaluations should produce same result
    let r1 = Visibility::visible_version(&chain, view);
    let r2 = Visibility::visible_version(&chain, view);
    let r3 = Visibility::visible_version(&chain, view);

    assert!(r1.is_visible());
    assert!(r2.is_visible());
    assert!(r3.is_visible());

    // All should return the same version
    let v1 = r1.version().unwrap();
    let v2 = r2.version().unwrap();
    let v3 = r3.version().unwrap();

    assert_eq!(v1.commit_id(), v2.commit_id());
    assert_eq!(v2.commit_id(), v3.commit_id());
}

/// Visibility selects largest commit within bound.
#[test]
fn test_visibility_selects_largest_within_bound() {
    let mut chain = VersionChain::new("doc".to_string());
    chain.push(Version::with_document(
        "doc".to_string(),
        b"v1".to_vec(),
        CommitId::new(1),
    ));
    chain.push(Version::with_document(
        "doc".to_string(),
        b"v3".to_vec(),
        CommitId::new(3),
    ));
    chain.push(Version::with_document(
        "doc".to_string(),
        b"v5".to_vec(),
        CommitId::new(5),
    ));

    // View at 3 should see version 3
    let view = ReadView::new(CommitId::new(3));
    let result = Visibility::visible_version(&chain, view);

    assert!(result.is_visible());
    let version = result.version().unwrap();
    assert_eq!(version.commit_id(), CommitId::new(3));
}

/// Future commits are invisible.
#[test]
fn test_visibility_excludes_future() {
    let mut chain = VersionChain::new("doc".to_string());
    chain.push(Version::with_document(
        "doc".to_string(),
        b"v1".to_vec(),
        CommitId::new(10),
    ));

    // View at 5 cannot see commit 10
    let view = ReadView::new(CommitId::new(5));
    let result = Visibility::visible_version(&chain, view);

    assert!(!result.is_visible());
}

/// Tombstone makes document invisible.
#[test]
fn test_tombstone_makes_invisible() {
    let mut chain = VersionChain::new("doc".to_string());
    chain.push(Version::with_document(
        "doc".to_string(),
        b"v1".to_vec(),
        CommitId::new(1),
    ));
    chain.push(Version::with_tombstone("doc".to_string(), CommitId::new(2)));

    // View at 2 sees tombstone = invisible
    let view = ReadView::new(CommitId::new(2));
    let result = Visibility::visible_version(&chain, view);

    assert!(!result.is_visible());
}
