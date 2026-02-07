//! MVCC Visibility - Deterministic snapshot isolation
//!
//! Per MVCC_VISIBILITY.md - This module implements the EXACT visibility rules:
//!
//! ## Visibility Rule (MVCC_VISIBILITY.md §3)
//!
//! Given:
//! - A read view `R`
//! - A logical document key `K`  
//! - A version chain `V₀ … Vₙ` ordered by commit identity (ascending)
//!
//! The visible version `V*` is defined as:
//! 1. Consider only versions where `V.commit_id ≤ R.read_upper_bound`
//! 2. From those, select the version with the **largest commit_id**
//! 3. If that version is a tombstone, `K` is invisible
//!
//! This rule is ABSOLUTE and admits NO EXCEPTIONS.
//!
//! ## Guarantees (MVCC_VISIBILITY.md §4)
//!
//! - Readers observe a stable snapshot
//! - Reads never block writes
//! - No dirty reads
//! - No non-repeatable reads
//! - No phantom visibility changes within a read
//!
//! ## Forbidden Behaviors (MVCC_VISIBILITY.md §9)
//!
//! - Partial visibility of a transaction
//! - Time-based visibility
//! - Thread-dependent visibility
//! - Heuristic snapshot selection
//! - Visibility influenced by index state
//! - Visibility differences between equivalent queries

use super::{CommitId, ReadView, Version, VersionChain};

/// Result of visibility evaluation for a document key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VisibilityResult<'a> {
    /// A visible document version exists
    Visible(&'a Version),
    /// Key is invisible (either no versions or latest visible is tombstone)
    Invisible,
}

impl<'a> VisibilityResult<'a> {
    /// Returns the visible version if any
    pub fn version(&self) -> Option<&'a Version> {
        match self {
            VisibilityResult::Visible(v) => Some(v),
            VisibilityResult::Invisible => None,
        }
    }

    /// Returns true if visible
    pub fn is_visible(&self) -> bool {
        matches!(self, VisibilityResult::Visible(_))
    }
}

/// Stateless visibility resolver per MVCC_VISIBILITY.md
///
/// This is a pure function module with no state.
/// Visibility is evaluated identically every time for identical inputs.
pub struct Visibility;

impl Visibility {
    /// Evaluates visibility for a version chain given a read view.
    ///
    /// Per MVCC_VISIBILITY.md §3 (EXACT implementation):
    /// 1. Consider only versions where V.commit_id ≤ R.read_upper_bound
    /// 2. Select the version with the LARGEST commit_id
    /// 3. If that version is a tombstone, K is invisible
    ///
    /// Per MVCC_VISIBILITY.md §5.1:
    /// - Visibility is determined exactly once per key per query
    /// - Subsequent reads of the same key return the same version
    pub fn visible_version<'a>(chain: &'a VersionChain, view: ReadView) -> VisibilityResult<'a> {
        let upper_bound = view.upper_bound();

        // Step 1 & 2: Find version with largest commit_id ≤ upper_bound
        let visible = chain
            .versions()
            .iter()
            .filter(|v| v.commit_id() <= upper_bound)
            .max_by_key(|v| v.commit_id());

        match visible {
            Some(version) => {
                // Step 3: If tombstone, invisible
                if version.is_tombstone() {
                    VisibilityResult::Invisible
                } else {
                    VisibilityResult::Visible(version)
                }
            }
            None => VisibilityResult::Invisible,
        }
    }

    /// Check if a single version is visible under a read view.
    ///
    /// This is a simpler check for when you have a single version,
    /// not a full chain.
    pub fn is_version_visible(version: &Version, view: ReadView) -> bool {
        version.commit_id() <= view.upper_bound() && !version.is_tombstone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_version(key: &str, data: &[u8], commit: u64) -> Version {
        Version::with_document(key.to_string(), data.to_vec(), CommitId::new(commit))
    }

    fn make_tombstone(key: &str, commit: u64) -> Version {
        Version::with_tombstone(key.to_string(), CommitId::new(commit))
    }

    // === Visibility Rule Tests (MVCC_VISIBILITY.md §3) ===

    #[test]
    fn test_visibility_selects_largest_commit_within_bound() {
        // Per spec: select version with LARGEST commit_id ≤ upper_bound
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_version("doc1", b"v2", 20));
        chain.push(make_version("doc1", b"v3", 30));

        // View with upper_bound = 25 should see v2 (commit 20)
        let view = ReadView::new(CommitId::new(25));
        let result = Visibility::visible_version(&chain, view);

        assert!(result.is_visible());
        let version = result.version().unwrap();
        assert_eq!(version.commit_id(), CommitId::new(20));
    }

    #[test]
    fn test_visibility_excludes_future_commits() {
        // Per spec: versions with commit_id > upper_bound are invisible
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_version("doc1", b"v2", 50));

        let view = ReadView::new(CommitId::new(30));
        let result = Visibility::visible_version(&chain, view);

        assert!(result.is_visible());
        let version = result.version().unwrap();
        assert_eq!(version.commit_id(), CommitId::new(10));
    }

    #[test]
    fn test_visibility_empty_chain_invisible() {
        let chain = VersionChain::new("doc1".to_string());
        let view = ReadView::new(CommitId::new(100));

        let result = Visibility::visible_version(&chain, view);
        assert!(!result.is_visible());
    }

    #[test]
    fn test_visibility_all_future_invisible() {
        // All versions have commit_id > upper_bound
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 100));
        chain.push(make_version("doc1", b"v2", 200));

        let view = ReadView::new(CommitId::new(50));
        let result = Visibility::visible_version(&chain, view);

        assert!(!result.is_visible());
    }

    // === Tombstone Tests (MVCC_VISIBILITY.md §3 step 3) ===

    #[test]
    fn test_tombstone_makes_key_invisible() {
        // Per spec: if visible version is tombstone, key is invisible
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_tombstone("doc1", 20));

        let view = ReadView::new(CommitId::new(25));
        let result = Visibility::visible_version(&chain, view);

        assert!(!result.is_visible());
    }

    #[test]
    fn test_tombstone_before_view_hides_older_versions() {
        // Delete at commit 20, view at 25 -> invisible
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_tombstone("doc1", 20));

        let view = ReadView::new(CommitId::new(25));
        assert!(!Visibility::visible_version(&chain, view).is_visible());
    }

    #[test]
    fn test_tombstone_after_view_shows_older_version() {
        // Delete at commit 30, view at 25 -> sees version at 20
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_version("doc1", b"v2", 20));
        chain.push(make_tombstone("doc1", 30));

        let view = ReadView::new(CommitId::new(25));
        let result = Visibility::visible_version(&chain, view);

        assert!(result.is_visible());
        assert_eq!(result.version().unwrap().commit_id(), CommitId::new(20));
    }

    #[test]
    fn test_delete_then_reinsert() {
        // v1@10, delete@20, v2@30, view@35 -> sees v2
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_tombstone("doc1", 20));
        chain.push(make_version("doc1", b"v2", 30));

        let view = ReadView::new(CommitId::new(35));
        let result = Visibility::visible_version(&chain, view);

        assert!(result.is_visible());
        assert_eq!(result.version().unwrap().commit_id(), CommitId::new(30));
    }

    #[test]
    fn test_delete_reinsert_view_before_reinsert() {
        // v1@10, delete@20, v2@30, view@25 -> invisible (tombstone is latest visible)
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_tombstone("doc1", 20));
        chain.push(make_version("doc1", b"v2", 30));

        let view = ReadView::new(CommitId::new(25));
        let result = Visibility::visible_version(&chain, view);

        assert!(!result.is_visible());
    }

    // === Snapshot Stability Tests (MVCC_VISIBILITY.md §5.1) ===

    #[test]
    fn test_same_view_same_result() {
        // Per spec: two reads with same view must observe identical results
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_version("doc1", b"v2", 20));

        let view = ReadView::new(CommitId::new(15));

        let result1 = Visibility::visible_version(&chain, view);
        let result2 = Visibility::visible_version(&chain, view);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_different_views_monotonic() {
        // Per MVCC_VISIBILITY.md §7: higher view may see MORE, never FEWER
        let mut chain = VersionChain::new("doc1".to_string());
        chain.push(make_version("doc1", b"v1", 10));
        chain.push(make_version("doc1", b"v2", 20));
        chain.push(make_version("doc1", b"v3", 30));

        let view_low = ReadView::new(CommitId::new(15));
        let view_high = ReadView::new(CommitId::new(25));

        let result_low = Visibility::visible_version(&chain, view_low);
        let result_high = Visibility::visible_version(&chain, view_high);

        // Higher view sees later version
        assert_eq!(result_low.version().unwrap().commit_id(), CommitId::new(10));
        assert_eq!(
            result_high.version().unwrap().commit_id(),
            CommitId::new(20)
        );
    }

    // === Single Version Visibility ===

    #[test]
    fn test_is_version_visible_within_bound() {
        let version = make_version("doc1", b"data", 10);
        let view = ReadView::new(CommitId::new(20));

        assert!(Visibility::is_version_visible(&version, view));
    }

    #[test]
    fn test_is_version_visible_at_bound() {
        let version = make_version("doc1", b"data", 20);
        let view = ReadView::new(CommitId::new(20));

        assert!(Visibility::is_version_visible(&version, view));
    }

    #[test]
    fn test_is_version_invisible_beyond_bound() {
        let version = make_version("doc1", b"data", 30);
        let view = ReadView::new(CommitId::new(20));

        assert!(!Visibility::is_version_visible(&version, view));
    }

    #[test]
    fn test_tombstone_version_invisible() {
        let version = make_tombstone("doc1", 10);
        let view = ReadView::new(CommitId::new(20));

        assert!(!Visibility::is_version_visible(&version, view));
    }
}
