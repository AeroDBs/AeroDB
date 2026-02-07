//! Replication Compatibility Assertions
//!
//! Per REPLICATION_COMPATIBILITY.md:
//! - All existing guarantees remain true
//! - No subsystem requires reinterpretation
//! - No "replication-only" semantics exist
//!
//! Per §10: "Replication adds nodes, not meanings."

use super::errors::{ReplicationError, ReplicationResult};
use super::role::ReplicationState;

/// Compatibility assertion results
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompatibilityAssertion {
    /// Compatibility preserved
    Compatible,

    /// Phase-1 compatibility violated
    Phase1Violation(String),

    /// MVCC compatibility violated
    MvccViolation(String),

    /// Snapshot compatibility violated
    SnapshotViolation(String),
}

impl CompatibilityAssertion {
    /// Check if compatible.
    pub fn is_compatible(&self) -> bool {
        matches!(self, Self::Compatible)
    }

    /// Description of violation if not compatible.
    pub fn violation_description(&self) -> Option<&str> {
        match self {
            Self::Compatible => None,
            Self::Phase1Violation(s) => Some(s),
            Self::MvccViolation(s) => Some(s),
            Self::SnapshotViolation(s) => Some(s),
        }
    }
}

/// Phase-1 compatibility invariants per §2
#[derive(Debug, Default)]
pub struct Phase1Compatibility {
    /// WAL is sole durability authority
    pub wal_is_durability_authority: bool,
    /// fsync semantics unchanged
    pub fsync_semantics_unchanged: bool,
    /// WAL replay rules identical
    pub wal_replay_identical: bool,
    /// Storage invariants intact
    pub storage_invariants_intact: bool,
    /// Query engine unchanged
    pub query_engine_unchanged: bool,
}

impl Phase1Compatibility {
    /// Create with all assertions passing.
    pub fn all_passing() -> Self {
        Self {
            wal_is_durability_authority: true,
            fsync_semantics_unchanged: true,
            wal_replay_identical: true,
            storage_invariants_intact: true,
            query_engine_unchanged: true,
        }
    }

    /// Verify all Phase-1 compatibility assertions.
    pub fn verify(&self) -> CompatibilityAssertion {
        if !self.wal_is_durability_authority {
            return CompatibilityAssertion::Phase1Violation(
                "WAL must remain sole durability authority (§2.1)".to_string(),
            );
        }

        if !self.fsync_semantics_unchanged {
            return CompatibilityAssertion::Phase1Violation(
                "fsync semantics must be unchanged (§2.1)".to_string(),
            );
        }

        if !self.wal_replay_identical {
            return CompatibilityAssertion::Phase1Violation(
                "WAL replay rules must be identical (§2.1)".to_string(),
            );
        }

        if !self.storage_invariants_intact {
            return CompatibilityAssertion::Phase1Violation(
                "Storage invariants must remain intact (§2.2)".to_string(),
            );
        }

        if !self.query_engine_unchanged {
            return CompatibilityAssertion::Phase1Violation(
                "Query engine semantics must be unchanged (§2.3)".to_string(),
            );
        }

        CompatibilityAssertion::Compatible
    }
}

/// MVCC compatibility invariants per §3
#[derive(Debug, Default)]
pub struct MvccCompatibility {
    /// CommitIds globally ordered
    pub commit_ids_ordered: bool,
    /// CommitIds immutable
    pub commit_ids_immutable: bool,
    /// CommitIds only from Primary
    pub commit_ids_from_primary: bool,
    /// Visibility semantics unchanged
    pub visibility_unchanged: bool,
    /// GC rules unchanged
    pub gc_rules_unchanged: bool,
}

impl MvccCompatibility {
    /// Create with all assertions passing.
    pub fn all_passing() -> Self {
        Self {
            commit_ids_ordered: true,
            commit_ids_immutable: true,
            commit_ids_from_primary: true,
            visibility_unchanged: true,
            gc_rules_unchanged: true,
        }
    }

    /// Verify all MVCC compatibility assertions.
    pub fn verify(&self) -> CompatibilityAssertion {
        if !self.commit_ids_ordered {
            return CompatibilityAssertion::MvccViolation(
                "CommitIds must be globally ordered (§3.1)".to_string(),
            );
        }

        if !self.commit_ids_immutable {
            return CompatibilityAssertion::MvccViolation(
                "CommitIds must be immutable (§3.1)".to_string(),
            );
        }

        if !self.commit_ids_from_primary {
            return CompatibilityAssertion::MvccViolation(
                "CommitIds must originate only from Primary (§3.1)".to_string(),
            );
        }

        if !self.visibility_unchanged {
            return CompatibilityAssertion::MvccViolation(
                "Visibility semantics must be unchanged (§3.2)".to_string(),
            );
        }

        if !self.gc_rules_unchanged {
            return CompatibilityAssertion::MvccViolation(
                "GC eligibility rules must be unchanged (§3.3)".to_string(),
            );
        }

        CompatibilityAssertion::Compatible
    }
}

/// Full compatibility check
#[derive(Debug)]
pub struct CompatibilityCheck {
    /// Phase-1 compatibility
    pub phase1: Phase1Compatibility,
    /// MVCC compatibility
    pub mvcc: MvccCompatibility,
}

impl CompatibilityCheck {
    /// Create with all assertions passing.
    pub fn all_passing() -> Self {
        Self {
            phase1: Phase1Compatibility::all_passing(),
            mvcc: MvccCompatibility::all_passing(),
        }
    }

    /// Verify all compatibility assertions.
    pub fn verify(&self) -> Vec<CompatibilityAssertion> {
        let mut results = vec![];

        let phase1_result = self.phase1.verify();
        if !phase1_result.is_compatible() {
            results.push(phase1_result);
        }

        let mvcc_result = self.mvcc.verify();
        if !mvcc_result.is_compatible() {
            results.push(mvcc_result);
        }

        if results.is_empty() {
            results.push(CompatibilityAssertion::Compatible);
        }

        results
    }

    /// Check if all compatibility assertions pass.
    pub fn is_compatible(&self) -> bool {
        self.phase1.verify().is_compatible() && self.mvcc.verify().is_compatible()
    }
}

impl Default for CompatibilityCheck {
    fn default() -> Self {
        Self::all_passing()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase1_all_passing() {
        let phase1 = Phase1Compatibility::all_passing();
        assert!(phase1.verify().is_compatible());
    }

    #[test]
    fn test_phase1_wal_authority_violation() {
        let mut phase1 = Phase1Compatibility::all_passing();
        phase1.wal_is_durability_authority = false;

        let result = phase1.verify();
        assert!(!result.is_compatible());
        assert!(result.violation_description().unwrap().contains("WAL"));
    }

    #[test]
    fn test_mvcc_all_passing() {
        let mvcc = MvccCompatibility::all_passing();
        assert!(mvcc.verify().is_compatible());
    }

    #[test]
    fn test_mvcc_commit_id_source_violation() {
        // Per §3.1: CommitIds must originate only from Primary
        let mut mvcc = MvccCompatibility::all_passing();
        mvcc.commit_ids_from_primary = false;

        let result = mvcc.verify();
        assert!(!result.is_compatible());
        assert!(result.violation_description().unwrap().contains("Primary"));
    }

    #[test]
    fn test_full_compatibility_check() {
        let check = CompatibilityCheck::all_passing();
        assert!(check.is_compatible());

        let results = check.verify();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_compatible());
    }

    #[test]
    fn test_full_compatibility_with_violation() {
        let mut check = CompatibilityCheck::all_passing();
        check.mvcc.visibility_unchanged = false;

        assert!(!check.is_compatible());

        let results = check.verify();
        assert!(results.iter().any(|r| !r.is_compatible()));
    }
}
