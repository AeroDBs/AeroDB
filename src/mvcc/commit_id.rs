//! CommitId - Totally ordered commit identity
//!
//! Per MVCC.md §2.3 and MVCC_VISIBILITY.md §2.1:
//! - Totally orders all commits
//! - Deterministic across crashes and recovery
//! - Independent of wall-clock time
//! - No two commits share the same identity
//!
//! This is a PURE TYPE with NO behavior beyond construction and access.

/// A totally ordered, opaque commit identity.
///
/// Per MVCC.md:
/// - Every committed version has a commit identity
/// - Commit identities define a strict total order
/// - This ordering is the sole authority for visibility
///
/// Per PHASE2_INVARIANTS.md §2.5:
/// - All committed versions have a total, deterministic order
/// - Preserved across restarts
/// - Reproducible during WAL replay
/// - No ties or ambiguous positions
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CommitId(u64);

impl CommitId {
    /// Creates a new CommitId with the given value.
    ///
    /// This is the only way to construct a CommitId.
    /// No Default implementation exists to prevent accidental construction.
    #[inline]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying value.
    ///
    /// This accessor exists for serialization and debugging only.
    /// Application code should not depend on the internal representation.
    #[inline]
    pub fn value(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_id_requires_explicit_construction() {
        // CommitId must be explicitly constructed
        let id = CommitId::new(42);
        assert_eq!(id.value(), 42);
    }

    #[test]
    fn test_commit_id_is_copy() {
        let id1 = CommitId::new(1);
        let id2 = id1; // Copy
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_commit_id_equality() {
        let id1 = CommitId::new(100);
        let id2 = CommitId::new(100);
        let id3 = CommitId::new(200);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_commit_id_ordering_trait_exists() {
        let id1 = CommitId::new(10);
        let id2 = CommitId::new(20);

        // Ordering traits are derived, but we do NOT assert semantic meaning here
        // This test only verifies the traits compile
        let _ = id1 < id2;
        let _ = id1 <= id2;
        let _ = id1 > id2;
        let _ = id1 >= id2;
    }

    #[test]
    fn test_commit_id_hash_trait_exists() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(CommitId::new(1));
        set.insert(CommitId::new(2));

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_commit_id_debug() {
        let id = CommitId::new(123);
        let debug = format!("{:?}", id);
        assert!(debug.contains("123"));
    }
}
