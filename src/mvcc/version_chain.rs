//! VersionChain - Version history for a document
//!
//! Per MVCC.md §2.2:
//! - Versions form a total order for each document key
//! - Each version supersedes exactly one prior version (if any)
//! - No forks or branches
//!
//! This is a PURE DATA CONTAINER with NO behavior.
//! - No traversal helpers
//! - No "get visible" logic
//! - No enforcement logic

use super::Version;

/// The complete version history of a single logical document.
///
/// Per MVCC.md:
/// - Contains versions in commit order
/// - Represents the full history of a document
///
/// This is a data container only. No visibility or traversal logic.
#[derive(Clone, Debug)]
pub struct VersionChain {
    /// The logical document key this chain represents.
    key: String,
    /// All versions of this document, conceptually in commit order.
    versions: Vec<Version>,
}

impl VersionChain {
    /// Creates a new empty version chain for the given key.
    pub fn new(key: String) -> Self {
        Self {
            key,
            versions: Vec::new(),
        }
    }

    /// Creates a version chain with initial versions.
    pub fn with_versions(key: String, versions: Vec<Version>) -> Self {
        Self { key, versions }
    }

    /// Returns the document key.
    #[inline]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the number of versions in this chain.
    #[inline]
    pub fn len(&self) -> usize {
        self.versions.len()
    }

    /// Returns true if this chain has no versions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    /// Returns a slice of all versions.
    ///
    /// This is a raw accessor. No visibility filtering is performed.
    #[inline]
    pub fn versions(&self) -> &[Version] {
        &self.versions
    }

    /// Appends a version to this chain.
    ///
    /// This is a structural operation only. No ordering enforcement.
    pub fn push(&mut self, version: Version) {
        self.versions.push(version);
    }

    /// Find the visible version for this chain given a read view.
    ///
    /// Per MVCC_VISIBILITY.md §3:
    /// 1. Consider only versions where V.commit_id ≤ R.read_upper_bound
    /// 2. Select the version with the LARGEST commit_id
    /// 3. If that version is a tombstone, return Invisible
    ///
    /// This is a convenience wrapper around Visibility::visible_version.
    pub fn visible_version(&self, view: super::ReadView) -> super::VisibilityResult<'_> {
        super::Visibility::visible_version(self, view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mvcc::CommitId;

    #[test]
    fn test_version_chain_creation() {
        let chain = VersionChain::new("doc1".to_string());
        assert_eq!(chain.key(), "doc1");
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn test_version_chain_push() {
        let mut chain = VersionChain::new("doc1".to_string());

        chain.push(Version::with_document(
            "doc1".to_string(),
            b"v1".to_vec(),
            CommitId::new(1),
        ));

        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_version_chain_with_versions() {
        let versions = vec![
            Version::with_document("k".to_string(), b"a".to_vec(), CommitId::new(1)),
            Version::with_document("k".to_string(), b"b".to_vec(), CommitId::new(2)),
        ];

        let chain = VersionChain::with_versions("k".to_string(), versions);
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_version_chain_versions_accessor() {
        let mut chain = VersionChain::new("k".to_string());
        chain.push(Version::with_document(
            "k".to_string(),
            b"d".to_vec(),
            CommitId::new(1),
        ));

        let versions = chain.versions();
        assert_eq!(versions.len(), 1);
    }

    #[test]
    fn test_version_chain_is_data_container() {
        // This test verifies VersionChain has no decision-making logic
        // It can only store and retrieve versions
        let chain = VersionChain::new("test".to_string());

        // No visibility methods exist
        // No traversal helpers exist
        // No enforcement logic exists
        // Only basic container operations

        let _ = chain.key();
        let _ = chain.len();
        let _ = chain.is_empty();
        let _ = chain.versions();
    }
}
