//! ReadView - Stable snapshot boundary
//!
//! Per MVCC.md §3.1:
//! - A read view is a stable description of which versions are visible
//! - Established at read start
//! - Never changes during the read
//! - Defined purely in terms of commit identities
//!
//! Per MVCC_VISIBILITY.md §2.2:
//! - Defined by a single scalar value: read_upper_bound
//! - The maximum commit identity visible to this read
//!
//! This is a PURE TYPE with NO behavior beyond construction and access.
//! - No validation logic
//! - No visibility helpers
//! - No global state access

use super::CommitId;

/// A stable snapshot boundary for read operations.
///
/// Per MVCC_VISIBILITY.md:
/// - `read_upper_bound` represents the maximum commit identity visible
/// - All versions with commit_id > upper_bound are invisible
///
/// Per PHASE2_INVARIANTS.md §2.3:
/// - Once established, a read view never changes
/// - Never sees partial writes
/// - Never sees future versions
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ReadView {
    /// The maximum commit identity visible to this read.
    read_upper_bound: CommitId,
}

impl ReadView {
    /// Creates a new read view with the given upper bound.
    ///
    /// After construction, the read view is immutable.
    #[inline]
    pub fn new(upper_bound: CommitId) -> Self {
        Self {
            read_upper_bound: upper_bound,
        }
    }

    /// Returns the upper bound commit identity.
    ///
    /// Per MVCC_VISIBILITY.md: All versions with commit_id > this value
    /// are invisible to reads using this view.
    #[inline]
    pub fn upper_bound(&self) -> CommitId {
        self.read_upper_bound
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_view_creation() {
        let view = ReadView::new(CommitId::new(100));
        assert_eq!(view.upper_bound(), CommitId::new(100));
    }

    #[test]
    fn test_read_view_is_immutable() {
        let view = ReadView::new(CommitId::new(50));

        // ReadView has no setters - only accessor
        // After creation, upper_bound cannot be changed
        assert_eq!(view.upper_bound(), CommitId::new(50));

        // Creating a copy doesn't allow mutation either
        let view2 = view;
        assert_eq!(view.upper_bound(), view2.upper_bound());
    }

    #[test]
    fn test_read_view_is_copy() {
        let view1 = ReadView::new(CommitId::new(10));
        let view2 = view1; // Copy
        assert_eq!(view1, view2);
    }

    #[test]
    fn test_read_view_equality() {
        let view1 = ReadView::new(CommitId::new(100));
        let view2 = ReadView::new(CommitId::new(100));
        let view3 = ReadView::new(CommitId::new(200));

        assert_eq!(view1, view2);
        assert_ne!(view1, view3);
    }

    #[test]
    fn test_read_view_has_no_visibility_logic() {
        // This test documents that ReadView has NO visibility methods
        // It is purely a data holder
        let view = ReadView::new(CommitId::new(10));

        // Only these methods exist:
        let _ = view.upper_bound();

        // No is_visible() method
        // No filter() method
        // No compare_to_version() method
    }

    #[test]
    fn test_read_view_debug() {
        let view = ReadView::new(CommitId::new(42));
        let debug = format!("{:?}", view);
        assert!(debug.contains("42"));
    }
}
