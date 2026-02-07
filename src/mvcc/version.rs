//! Version - Immutable document version
//!
//! Per MVCC.md §2.1:
//! - A version is a logically immutable representation of a document
//! - Has complete document payload OR explicit tombstone
//! - Has associated commit identity
//! - Once created, never changes
//!
//! Per PHASE2_INVARIANTS.md §2.1:
//! - Version is immutable after creation
//! - Updates create new versions only
//! - Deletes are explicit tombstone versions
//!
//! This is a PURE TYPE with NO behavior beyond construction and access.

use super::CommitId;

/// The payload of a version: either a document or an explicit tombstone.
///
/// Per MVCC.md: Deletes are represented as explicit tombstone versions,
/// fully ordered in the version chain.
///
/// Tombstone is explicit, NOT represented via Option.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VersionPayload {
    /// A complete document payload.
    Document(Vec<u8>),
    /// An explicit deletion marker.
    Tombstone,
}

impl VersionPayload {
    /// Returns true if this payload is a tombstone.
    #[inline]
    pub fn is_tombstone(&self) -> bool {
        matches!(self, VersionPayload::Tombstone)
    }

    /// Returns true if this payload is a document.
    #[inline]
    pub fn is_document(&self) -> bool {
        matches!(self, VersionPayload::Document(_))
    }
}

/// A single immutable document version.
///
/// Per MVCC.md:
/// - Represents a document at a specific point in history
/// - Immutable after creation
/// - Contains either document data or tombstone marker
///
/// All fields are private to enforce immutability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    /// The logical document key.
    key: String,
    /// The version payload: document data or tombstone.
    payload: VersionPayload,
    /// The commit identity associated with this version.
    commit_id: CommitId,
}

impl Version {
    /// Creates a new document version.
    ///
    /// After construction, the version cannot be modified.
    pub fn new(key: String, payload: VersionPayload, commit_id: CommitId) -> Self {
        Self {
            key,
            payload,
            commit_id,
        }
    }

    /// Creates a new document version with data.
    pub fn with_document(key: String, data: Vec<u8>, commit_id: CommitId) -> Self {
        Self::new(key, VersionPayload::Document(data), commit_id)
    }

    /// Creates a new tombstone version.
    pub fn with_tombstone(key: String, commit_id: CommitId) -> Self {
        Self::new(key, VersionPayload::Tombstone, commit_id)
    }

    /// Returns the document key.
    #[inline]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns a reference to the payload.
    #[inline]
    pub fn payload(&self) -> &VersionPayload {
        &self.payload
    }

    /// Returns the commit identity.
    #[inline]
    pub fn commit_id(&self) -> CommitId {
        self.commit_id
    }

    /// Returns true if this version is a tombstone.
    #[inline]
    pub fn is_tombstone(&self) -> bool {
        self.payload.is_tombstone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_immutability() {
        let version =
            Version::with_document("doc1".to_string(), b"data".to_vec(), CommitId::new(1));

        // Fields are private - only accessors available
        assert_eq!(version.key(), "doc1");
        assert_eq!(version.commit_id(), CommitId::new(1));
    }

    #[test]
    fn test_version_with_document() {
        let version =
            Version::with_document("key".to_string(), b"payload".to_vec(), CommitId::new(10));

        assert!(version.payload().is_document());
        assert!(!version.is_tombstone());
    }

    #[test]
    fn test_version_with_tombstone() {
        let version = Version::with_tombstone("deleted_key".to_string(), CommitId::new(20));

        assert!(version.is_tombstone());
        assert!(version.payload().is_tombstone());
        assert!(!version.payload().is_document());
    }

    #[test]
    fn test_tombstone_is_explicit_not_option() {
        // Tombstone is an explicit enum variant, not Option<Vec<u8>>
        let tombstone = VersionPayload::Tombstone;
        let document = VersionPayload::Document(vec![1, 2, 3]);

        // Both are the same type
        let _ = vec![tombstone, document];
    }

    #[test]
    fn test_version_clone() {
        let v1 = Version::with_document("k".to_string(), b"d".to_vec(), CommitId::new(5));
        let v2 = v1.clone();

        assert_eq!(v1, v2);
    }
}
