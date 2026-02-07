//! Commit Authority - WAL-based commit identity assignment
//!
//! Per MVCC_WAL_INTERACTION.md:
//! - The WAL is the sole source of truth for commit ordering
//! - Commit identities are assigned exactly once
//! - Assignment occurs as part of commit
//! - The ordering is total, strict, and replayable
//!
//! Per PHASE2_INVARIANTS.md §2.5:
//! - All committed versions have a total, deterministic order
//! - Preserved across restarts
//! - Reproducible during WAL replay
//! - No ties or ambiguous positions
//!
//! This module provides the CommitAuthority which:
//! - Tracks the highest observed commit identity (from WAL replay)
//! - Provides the next commit identity for new commits
//! - Does NOT store state outside the WAL

use crate::mvcc::CommitId;

/// Commit authority for WAL-based commit identity assignment.
///
/// Per MVCC_WAL_INTERACTION.md:
/// - No in-memory counters (only tracks highest replayed)
/// - No atomic integers (ordering from WAL)
/// - No clock usage (deterministic)
/// - No recovery-time guessing (explicit replay)
///
/// The WAL is authoritative. This struct merely tracks the current position.
#[derive(Debug)]
pub struct CommitAuthority {
    /// The highest commit identity observed during replay or assignment.
    /// This is derived from WAL, not stored independently.
    highest_commit_id: u64,
}

impl CommitAuthority {
    /// Create a new commit authority starting from zero.
    ///
    /// This should only be called for a fresh database.
    /// For recovery, use `from_replayed_commit`.
    pub fn new() -> Self {
        Self {
            highest_commit_id: 0,
        }
    }

    /// Create a commit authority from a replayed commit identity.
    ///
    /// This is called during WAL replay when an MvccCommit record is encountered.
    /// Per MVCC_WAL_INTERACTION.md §7:
    /// - WAL is replayed in strict order
    /// - Commit identities are re-established deterministically
    pub fn from_replayed_commit(commit_id: u64) -> Self {
        Self {
            highest_commit_id: commit_id,
        }
    }

    /// Update the authority with a replayed commit identity.
    ///
    /// Per MVCC_WAL_INTERACTION.md §7:
    /// - Commit identities must be strictly increasing
    /// - Recovery does not reassign commit identities
    ///
    /// Returns an error if the commit_id is not strictly greater than the current highest.
    pub fn observe_replayed_commit(&mut self, commit_id: u64) -> Result<(), CommitAuthorityError> {
        if commit_id <= self.highest_commit_id {
            return Err(CommitAuthorityError::NonMonotonic {
                observed: commit_id,
                highest: self.highest_commit_id,
            });
        }
        self.highest_commit_id = commit_id;
        Ok(())
    }

    /// Get the next commit identity to assign.
    ///
    /// This value should be persisted to WAL via MvccCommitRecord
    /// BEFORE being considered valid.
    ///
    /// Per MVCC_WAL_INTERACTION.md:
    /// - If it is not durably recorded, it does not exist
    /// - No commit identity exists outside the WAL
    pub fn next_commit_id(&self) -> CommitId {
        CommitId::new(self.highest_commit_id + 1)
    }

    /// Mark a commit identity as assigned after WAL persistence.
    ///
    /// This should ONLY be called AFTER the MvccCommitRecord has been
    /// fsynced to the WAL.
    ///
    /// Per MVCC_WAL_INTERACTION.md §3.2:
    /// - WAL fsync is the visibility barrier
    /// - If a commit identity is not durable, the commit does not exist
    pub fn mark_committed(&mut self, commit_id: CommitId) -> Result<(), CommitAuthorityError> {
        let id_value = commit_id.value();
        if id_value != self.highest_commit_id + 1 {
            return Err(CommitAuthorityError::OutOfOrder {
                attempted: id_value,
                expected: self.highest_commit_id + 1,
            });
        }
        self.highest_commit_id = id_value;
        Ok(())
    }

    /// Get the current highest commit identity.
    pub fn highest_commit_id(&self) -> Option<CommitId> {
        if self.highest_commit_id == 0 {
            None
        } else {
            Some(CommitId::new(self.highest_commit_id))
        }
    }

    /// Create a read view (snapshot) at the current commit point.
    ///
    /// Per MVCC_VISIBILITY.md §2.2:
    /// - ReadView captures exactly one value: read_upper_bound = latest_durable_commit_id
    /// - Immutable once created
    /// - Never changes during query execution
    ///
    /// Per MVCC.md §3.1:
    /// - Established at read start
    /// - Never changes during the read
    pub fn current_snapshot(&self) -> crate::mvcc::ReadView {
        crate::mvcc::ReadView::new(CommitId::new(self.highest_commit_id))
    }
}

impl Default for CommitAuthority {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors from commit authority operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitAuthorityError {
    /// Replayed commit identity is not strictly greater than highest.
    NonMonotonic { observed: u64, highest: u64 },
    /// Attempted to commit out of order.
    OutOfOrder { attempted: u64, expected: u64 },
}

impl std::fmt::Display for CommitAuthorityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommitAuthorityError::NonMonotonic { observed, highest } => {
                write!(
                    f,
                    "Non-monotonic commit identity: observed {} but highest is {}",
                    observed, highest
                )
            }
            CommitAuthorityError::OutOfOrder {
                attempted,
                expected,
            } => {
                write!(
                    f,
                    "Out of order commit: attempted {} but expected {}",
                    attempted, expected
                )
            }
        }
    }
}

impl std::error::Error for CommitAuthorityError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_authority_starts_at_zero() {
        let authority = CommitAuthority::new();
        assert!(authority.highest_commit_id().is_none());
    }

    #[test]
    fn test_next_commit_id_is_one_after_new() {
        let authority = CommitAuthority::new();
        assert_eq!(authority.next_commit_id(), CommitId::new(1));
    }

    #[test]
    fn test_mark_committed_updates_highest() {
        let mut authority = CommitAuthority::new();
        let next = authority.next_commit_id();
        authority.mark_committed(next).unwrap();

        assert_eq!(authority.highest_commit_id(), Some(CommitId::new(1)));
        assert_eq!(authority.next_commit_id(), CommitId::new(2));
    }

    #[test]
    fn test_observe_replayed_commit() {
        let mut authority = CommitAuthority::new();
        authority.observe_replayed_commit(1).unwrap();
        authority.observe_replayed_commit(2).unwrap();
        authority.observe_replayed_commit(5).unwrap(); // Gaps allowed

        assert_eq!(authority.highest_commit_id(), Some(CommitId::new(5)));
    }

    #[test]
    fn test_non_monotonic_replay_fails() {
        let mut authority = CommitAuthority::new();
        authority.observe_replayed_commit(5).unwrap();

        let result = authority.observe_replayed_commit(3);
        assert!(matches!(
            result,
            Err(CommitAuthorityError::NonMonotonic { .. })
        ));
    }

    #[test]
    fn test_duplicate_replay_fails() {
        let mut authority = CommitAuthority::new();
        authority.observe_replayed_commit(5).unwrap();

        let result = authority.observe_replayed_commit(5);
        assert!(matches!(
            result,
            Err(CommitAuthorityError::NonMonotonic { .. })
        ));
    }

    #[test]
    fn test_out_of_order_commit_fails() {
        let mut authority = CommitAuthority::new();
        let result = authority.mark_committed(CommitId::new(5));
        assert!(matches!(
            result,
            Err(CommitAuthorityError::OutOfOrder { .. })
        ));
    }

    #[test]
    fn test_from_replayed_commit() {
        let authority = CommitAuthority::from_replayed_commit(100);
        assert_eq!(authority.highest_commit_id(), Some(CommitId::new(100)));
        assert_eq!(authority.next_commit_id(), CommitId::new(101));
    }

    #[test]
    fn test_deterministic_replay() {
        // Same sequence of replayed commits produces same state
        let mut auth1 = CommitAuthority::new();
        let mut auth2 = CommitAuthority::new();

        for id in [1, 2, 5, 10, 100] {
            auth1.observe_replayed_commit(id).unwrap();
            auth2.observe_replayed_commit(id).unwrap();
        }

        assert_eq!(auth1.highest_commit_id(), auth2.highest_commit_id());
        assert_eq!(auth1.next_commit_id(), auth2.next_commit_id());
    }
}
