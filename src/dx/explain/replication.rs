//! Replication Safety Explanation
//!
//! Per DX_EXPLANATION_MODEL.md §6.5:
//! Type: replication.safety
//! Explains: Why a replica is safe to serve reads
//!
//! Inputs:
//! - Replica WAL prefix
//! - Snapshot CommitId
//!
//! Rules Applied:
//! - WAL prefix rule
//! - MVCC snapshot safety rule
//!
//! Conclusion:
//! - Read-safe or not read-safe
//!
//! Read-only, Phase 4, no semantic authority.

use super::model::{Evidence, Explanation, ExplanationType, RuleApplication};
use super::rules::RuleRegistry;
use serde::{Deserialize, Serialize};

/// Replication safety input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationInput {
    /// Replica's durable WAL CommitId.
    pub replica_wal_commit_id: u64,
    /// Snapshot CommitId being queried.
    pub snapshot_commit_id: u64,
    /// Primary's latest CommitId (if known).
    pub primary_commit_id: Option<u64>,
    /// Whether replica is mid-recovery.
    pub is_mid_recovery: bool,
    /// Whether replica is bootstrapping.
    pub is_bootstrapping: bool,
}

/// Replication safety result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplicationSafetyResult {
    /// Replica is safe to serve reads for this snapshot.
    ReadSafe,
    /// Replica is not safe to serve reads.
    NotReadSafe,
}

/// Replication safety output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationSafetyOutput {
    /// Safety result.
    pub result: ReplicationSafetyResult,
    /// Lag in CommitIds (if applicable).
    pub lag: Option<u64>,
    /// Reason if not safe.
    pub reason: Option<String>,
}

/// Replication safety explainer.
///
/// Read-only, Phase 4, no semantic authority.
pub struct ReplicationExplainer {
    rules: RuleRegistry,
}

impl ReplicationExplainer {
    /// Create a new replication explainer.
    pub fn new() -> Self {
        Self {
            rules: RuleRegistry::new(),
        }
    }

    /// Generate replication safety explanation.
    ///
    /// Per REPL_READ_RULES.md:
    /// Read is safe iff: snapshot_commit_id <= replica_wal_commit_id
    pub fn explain(&self, input: ReplicationInput) -> Explanation {
        let snapshot_id = format!("repl-{}", input.snapshot_commit_id);

        let mut builder = Explanation::builder(
            ExplanationType::ReplicationSafety,
            &snapshot_id,
            input.snapshot_commit_id,
        )
        .input("replica_wal_commit_id", input.replica_wal_commit_id)
        .input("snapshot_commit_id", input.snapshot_commit_id)
        .input("is_mid_recovery", input.is_mid_recovery)
        .input("is_bootstrapping", input.is_bootstrapping);

        // Check REP-3: Read Safety
        let is_prefix_safe = input.snapshot_commit_id <= input.replica_wal_commit_id;
        let mut prefix_evidence = Evidence::empty();
        prefix_evidence.add("snapshot_commit_id", input.snapshot_commit_id);
        prefix_evidence.add("replica_wal_commit_id", input.replica_wal_commit_id);
        prefix_evidence.add(
            "comparison",
            format!(
                "{} <= {} = {}",
                input.snapshot_commit_id, input.replica_wal_commit_id, is_prefix_safe
            ),
        );

        if is_prefix_safe {
            builder = builder.rule(RuleApplication::satisfied(
                "REP-3",
                self.rules.description("REP-3"),
                prefix_evidence,
            ));
        } else {
            builder = builder.rule(RuleApplication::not_satisfied(
                "REP-3",
                self.rules.description("REP-3"),
                prefix_evidence,
            ));
        }

        // Check recovery/bootstrap state
        let mut state_evidence = Evidence::empty();
        state_evidence.add("is_mid_recovery", input.is_mid_recovery);
        state_evidence.add("is_bootstrapping", input.is_bootstrapping);

        let state_ok = !input.is_mid_recovery && !input.is_bootstrapping;
        if state_ok {
            builder = builder.rule(RuleApplication::satisfied(
                "REP-2",
                "Replica not in recovery or bootstrap state",
                state_evidence,
            ));
        } else {
            builder = builder.rule(RuleApplication::not_satisfied(
                "REP-2",
                "Replica must not be mid-recovery or bootstrapping",
                state_evidence,
            ));
        }

        // Calculate lag
        let lag = input.primary_commit_id.map(|p| {
            if p > input.replica_wal_commit_id {
                p - input.replica_wal_commit_id
            } else {
                0
            }
        });

        // Determine result
        let is_safe = is_prefix_safe && state_ok;
        let (result, reason) = if is_safe {
            (ReplicationSafetyResult::ReadSafe, None)
        } else if !is_prefix_safe {
            (
                ReplicationSafetyResult::NotReadSafe,
                Some("Snapshot CommitId exceeds replica's durable WAL".to_string()),
            )
        } else {
            (
                ReplicationSafetyResult::NotReadSafe,
                Some("Replica is mid-recovery or bootstrapping".to_string()),
            )
        };

        builder.conclude(ReplicationSafetyOutput {
            result,
            lag,
            reason,
        })
    }
}

impl Default for ReplicationExplainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_safe() {
        let explainer = ReplicationExplainer::new();
        let input = ReplicationInput {
            replica_wal_commit_id: 100,
            snapshot_commit_id: 50,
            primary_commit_id: Some(100),
            is_mid_recovery: false,
            is_bootstrapping: false,
        };

        let explanation = explainer.explain(input);
        assert_eq!(
            explanation.explanation_type,
            ExplanationType::ReplicationSafety
        );
    }

    #[test]
    fn test_not_safe_snapshot_ahead() {
        let explainer = ReplicationExplainer::new();
        let input = ReplicationInput {
            replica_wal_commit_id: 50,
            snapshot_commit_id: 100, // Ahead of replica
            primary_commit_id: Some(100),
            is_mid_recovery: false,
            is_bootstrapping: false,
        };

        let explanation = explainer.explain(input);
        // REP-3 should not be satisfied
        assert!(explanation
            .rules_applied
            .iter()
            .any(|r| r.rule_id == "REP-3"
                && r.evaluation == super::super::model::RuleEvaluation::False));
    }

    #[test]
    fn test_not_safe_mid_recovery() {
        let explainer = ReplicationExplainer::new();
        let input = ReplicationInput {
            replica_wal_commit_id: 100,
            snapshot_commit_id: 50,
            primary_commit_id: Some(100),
            is_mid_recovery: true,
            is_bootstrapping: false,
        };

        let explanation = explainer.explain(input);
        assert!(!explanation.rules_applied.is_empty());
    }

    #[test]
    fn test_determinism() {
        let explainer = ReplicationExplainer::new();
        let input = ReplicationInput {
            replica_wal_commit_id: 100,
            snapshot_commit_id: 50,
            primary_commit_id: Some(100),
            is_mid_recovery: false,
            is_bootstrapping: false,
        };

        let exp1 = explainer.explain(input.clone());
        let exp2 = explainer.explain(input);

        assert_eq!(
            exp1.observed_snapshot.commit_id,
            exp2.observed_snapshot.commit_id
        );
    }
}
