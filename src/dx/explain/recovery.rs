//! Recovery Process Explanation
//!
//! Per DX_EXPLANATION_MODEL.md §6.3:
//! Type: recovery.process
//! Explains: How recovery proceeded after a crash
//!
//! Inputs:
//! - Last known durable state
//!
//! Rules Applied:
//! - Checkpoint selection rules
//! - WAL replay rules
//! - Checksum validation rules
//!
//! Conclusion:
//! - Final recovered CommitId
//! - State validity result
//!
//! Read-only, Phase 4, no semantic authority.

use super::model::{Evidence, Explanation, ExplanationType, RuleApplication};
use super::rules::RuleRegistry;
use serde::{Deserialize, Serialize};

/// Recovery step type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStepType {
    /// Detected crash condition.
    CrashDetection,
    /// Selected checkpoint for recovery base.
    CheckpointSelection,
    /// Started WAL replay.
    WalReplayStart,
    /// Validated WAL entry checksum.
    ChecksumValidation,
    /// Applied WAL entry.
    WalEntryApply,
    /// Completed WAL replay.
    WalReplayComplete,
    /// Verified final state.
    StateVerification,
}

/// Individual recovery step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStep {
    /// Step type.
    pub step_type: RecoveryStepType,
    /// Step description.
    pub description: String,
    /// Relevant CommitId (if applicable).
    pub commit_id: Option<u64>,
    /// WAL offset (if applicable).
    pub wal_offset: Option<u64>,
    /// Whether step succeeded.
    pub success: bool,
}

/// Recovery input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryInput {
    /// Last known durable CommitId.
    pub last_durable_commit_id: u64,
    /// Checkpoint CommitId used.
    pub checkpoint_commit_id: Option<u64>,
    /// WAL replay start offset.
    pub wal_replay_start: Option<u64>,
    /// WAL replay end offset.
    pub wal_replay_end: Option<u64>,
}

/// Recovery output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryOutput {
    /// Final recovered CommitId.
    pub recovered_commit_id: u64,
    /// Whether recovery succeeded.
    pub recovery_success: bool,
    /// Number of WAL entries replayed.
    pub entries_replayed: u64,
    /// Recovery steps executed.
    pub steps: Vec<RecoveryStep>,
}

/// Recovery explainer.
///
/// Read-only, Phase 4, no semantic authority.
pub struct RecoveryExplainer {
    rules: RuleRegistry,
}

impl RecoveryExplainer {
    /// Create a new recovery explainer.
    pub fn new() -> Self {
        Self {
            rules: RuleRegistry::new(),
        }
    }

    /// Generate recovery explanation.
    ///
    /// Per DX_EXPLANATION_MODEL.md §6.3:
    /// If recovery has not occurred, return empty explanation.
    pub fn explain(&self, input: RecoveryInput, steps: Vec<RecoveryStep>) -> Explanation {
        let recovered_commit_id = steps
            .iter()
            .filter_map(|s| s.commit_id)
            .max()
            .unwrap_or(input.last_durable_commit_id);

        let snapshot_id = format!("recovery-{}", recovered_commit_id);

        let mut builder = Explanation::builder(
            ExplanationType::RecoveryProcess,
            &snapshot_id,
            recovered_commit_id,
        )
        .input("last_durable_commit_id", input.last_durable_commit_id)
        .input("checkpoint_commit_id", input.checkpoint_commit_id)
        .input("wal_replay_start", input.wal_replay_start)
        .input("wal_replay_end", input.wal_replay_end);

        // R-2: Recovery Is Deterministic
        let mut det_evidence = Evidence::empty();
        det_evidence.add("checkpoint_used", input.checkpoint_commit_id);
        det_evidence.add("wal_range_start", input.wal_replay_start);
        det_evidence.add("wal_range_end", input.wal_replay_end);
        builder = builder.rule(RuleApplication::satisfied(
            "R-2",
            self.rules.description("R-2"),
            det_evidence,
        ));

        // R-3: Recovery Completeness Is Verifiable
        let all_steps_success = steps.iter().all(|s| s.success);
        let mut verify_evidence = Evidence::empty();
        verify_evidence.add("steps_count", steps.len());
        verify_evidence.add("all_success", all_steps_success);

        if all_steps_success {
            builder = builder.rule(RuleApplication::satisfied(
                "R-3",
                self.rules.description("R-3"),
                verify_evidence,
            ));
        } else {
            builder = builder.rule(RuleApplication::not_satisfied(
                "R-3",
                self.rules.description("R-3"),
                verify_evidence,
            ));
        }

        let entries_replayed = steps
            .iter()
            .filter(|s| s.step_type == RecoveryStepType::WalEntryApply)
            .count() as u64;

        builder.conclude(RecoveryOutput {
            recovered_commit_id,
            recovery_success: all_steps_success,
            entries_replayed,
            steps,
        })
    }

    /// Generate empty explanation when no recovery occurred.
    pub fn no_recovery(&self, current_commit_id: u64) -> Explanation {
        let snapshot_id = format!("no-recovery-{}", current_commit_id);

        Explanation::builder(
            ExplanationType::RecoveryProcess,
            &snapshot_id,
            current_commit_id,
        )
        .input("recovery_occurred", false)
        .conclude(RecoveryOutput {
            recovered_commit_id: current_commit_id,
            recovery_success: true,
            entries_replayed: 0,
            steps: vec![],
        })
    }
}

impl Default for RecoveryExplainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_explanation() {
        let explainer = RecoveryExplainer::new();
        let input = RecoveryInput {
            last_durable_commit_id: 100,
            checkpoint_commit_id: Some(90),
            wal_replay_start: Some(91),
            wal_replay_end: Some(100),
        };
        let steps = vec![
            RecoveryStep {
                step_type: RecoveryStepType::CheckpointSelection,
                description: "Selected checkpoint at CommitId 90".to_string(),
                commit_id: Some(90),
                wal_offset: None,
                success: true,
            },
            RecoveryStep {
                step_type: RecoveryStepType::WalReplayComplete,
                description: "Replayed WAL to CommitId 100".to_string(),
                commit_id: Some(100),
                wal_offset: Some(1000),
                success: true,
            },
        ];

        let explanation = explainer.explain(input, steps);

        assert_eq!(
            explanation.explanation_type,
            ExplanationType::RecoveryProcess
        );
        assert!(!explanation.rules_applied.is_empty());
    }

    #[test]
    fn test_no_recovery() {
        let explainer = RecoveryExplainer::new();
        let explanation = explainer.no_recovery(100);

        assert_eq!(
            explanation.explanation_type,
            ExplanationType::RecoveryProcess
        );
    }

    #[test]
    fn test_determinism() {
        let explainer = RecoveryExplainer::new();
        let input = RecoveryInput {
            last_durable_commit_id: 50,
            checkpoint_commit_id: Some(40),
            wal_replay_start: Some(41),
            wal_replay_end: Some(50),
        };

        let exp1 = explainer.explain(input.clone(), vec![]);
        let exp2 = explainer.explain(input, vec![]);

        assert_eq!(
            exp1.observed_snapshot.commit_id,
            exp2.observed_snapshot.commit_id
        );
    }
}
