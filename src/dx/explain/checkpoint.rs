//! Checkpoint Safety Explanation
//!
//! Per DX_EXPLANATION_MODEL.md §6.4:
//! Type: checkpoint.safety
//! Explains: Why a checkpoint is valid or not
//!
//! Inputs:
//! - Checkpoint marker
//! - Snapshot metadata
//! - WAL offsets
//!
//! Rules Applied:
//! - Durability ordering rules
//! - Snapshot completeness rules
//!
//! Read-only, Phase 4, no semantic authority.

use super::model::{Evidence, Explanation, ExplanationType, RuleApplication};
use super::rules::RuleRegistry;
use serde::{Deserialize, Serialize};

/// Checkpoint safety input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointInput {
    /// Checkpoint identifier.
    pub checkpoint_id: u64,
    /// Checkpoint CommitId.
    pub checkpoint_commit_id: u64,
    /// WAL start offset covered.
    pub wal_start_offset: u64,
    /// WAL end offset covered.
    pub wal_end_offset: u64,
    /// Whether snapshot is complete.
    pub snapshot_complete: bool,
    /// Whether checkpoint is durable (fsync'd).
    pub is_durable: bool,
}

/// Checkpoint safety result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointSafetyResult {
    /// Checkpoint is valid and safe.
    Valid,
    /// Checkpoint is invalid.
    Invalid,
    /// Checkpoint validity cannot be determined.
    Undetermined,
}

/// Checkpoint safety output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointSafetyOutput {
    /// Safety result.
    pub result: CheckpointSafetyResult,
    /// Whether checkpoint can be used for recovery.
    pub recovery_safe: bool,
    /// WAL range that can be truncated.
    pub truncatable_wal_range: Option<(u64, u64)>,
}

/// Checkpoint safety explainer.
///
/// Read-only, Phase 4, no semantic authority.
pub struct CheckpointExplainer {
    rules: RuleRegistry,
}

impl CheckpointExplainer {
    /// Create a new checkpoint explainer.
    pub fn new() -> Self {
        Self {
            rules: RuleRegistry::new(),
        }
    }

    /// Generate checkpoint safety explanation.
    pub fn explain(&self, input: CheckpointInput) -> Explanation {
        let snapshot_id = format!("ckpt-{}", input.checkpoint_id);

        let mut builder = Explanation::builder(
            ExplanationType::CheckpointSafety,
            &snapshot_id,
            input.checkpoint_commit_id,
        )
        .input("checkpoint_id", input.checkpoint_id)
        .input("checkpoint_commit_id", input.checkpoint_commit_id)
        .input(
            "wal_range",
            format!("{}-{}", input.wal_start_offset, input.wal_end_offset),
        );

        // Check durability ordering (D-1 derivative)
        let mut dur_evidence = Evidence::empty();
        dur_evidence.add("is_durable", input.is_durable);
        dur_evidence.add("checkpoint_commit_id", input.checkpoint_commit_id);

        if input.is_durable {
            builder = builder.rule(RuleApplication::satisfied(
                "D-1",
                self.rules.description("D-1"),
                dur_evidence,
            ));
        } else {
            builder = builder.rule(RuleApplication::not_satisfied(
                "D-1",
                self.rules.description("D-1"),
                dur_evidence,
            ));
        }

        // Check snapshot completeness
        let mut snap_evidence = Evidence::empty();
        snap_evidence.add("snapshot_complete", input.snapshot_complete);

        if input.snapshot_complete {
            builder = builder.rule(RuleApplication::satisfied(
                "R-3",
                self.rules.description("R-3"),
                snap_evidence,
            ));
        } else {
            builder = builder.rule(RuleApplication::not_satisfied(
                "R-3",
                self.rules.description("R-3"),
                snap_evidence,
            ));
        }

        // Determine result
        let is_valid = input.is_durable && input.snapshot_complete;
        let result = if is_valid {
            CheckpointSafetyResult::Valid
        } else {
            CheckpointSafetyResult::Invalid
        };

        let truncatable = if is_valid {
            Some((input.wal_start_offset, input.wal_end_offset))
        } else {
            None
        };

        builder.conclude(CheckpointSafetyOutput {
            result,
            recovery_safe: is_valid,
            truncatable_wal_range: truncatable,
        })
    }
}

impl Default for CheckpointExplainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_checkpoint() {
        let explainer = CheckpointExplainer::new();
        let input = CheckpointInput {
            checkpoint_id: 1,
            checkpoint_commit_id: 100,
            wal_start_offset: 0,
            wal_end_offset: 1000,
            snapshot_complete: true,
            is_durable: true,
        };

        let explanation = explainer.explain(input);
        assert_eq!(
            explanation.explanation_type,
            ExplanationType::CheckpointSafety
        );
        assert!(!explanation.rules_applied.is_empty());
    }

    #[test]
    fn test_invalid_checkpoint_not_durable() {
        let explainer = CheckpointExplainer::new();
        let input = CheckpointInput {
            checkpoint_id: 1,
            checkpoint_commit_id: 100,
            wal_start_offset: 0,
            wal_end_offset: 1000,
            snapshot_complete: true,
            is_durable: false,
        };

        let explanation = explainer.explain(input);
        // Should have D-1 not satisfied
        assert!(explanation.rules_applied.iter().any(|r| r.rule_id == "D-1"));
    }

    #[test]
    fn test_determinism() {
        let explainer = CheckpointExplainer::new();
        let input = CheckpointInput {
            checkpoint_id: 1,
            checkpoint_commit_id: 50,
            wal_start_offset: 0,
            wal_end_offset: 500,
            snapshot_complete: true,
            is_durable: true,
        };

        let exp1 = explainer.explain(input.clone());
        let exp2 = explainer.explain(input);

        assert_eq!(
            exp1.observed_snapshot.commit_id,
            exp2.observed_snapshot.commit_id
        );
        assert_eq!(exp1.rules_applied.len(), exp2.rules_applied.len());
    }
}
