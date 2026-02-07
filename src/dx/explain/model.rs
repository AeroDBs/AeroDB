//! Explanation Object Model
//!
//! Per DX_EXPLANATION_MODEL.md §4:
//! All explanations MUST conform to this structure:
//! - explanation_type
//! - observed_snapshot: { snapshot_id, commit_id }
//! - inputs: { ... }
//! - rules_applied: [ { rule_id, description, evaluation, evidence } ]
//! - conclusion: { ... }
//!
//! Read-only, Phase 4, no semantic authority.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Explanation type identifier.
///
/// Per DX_EXPLANATION_MODEL.md §6: Supported explanation types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExplanationType {
    /// MVCC read visibility explanation.
    #[serde(rename = "mvcc.read_visibility")]
    MvccReadVisibility,
    /// Query execution explanation.
    #[serde(rename = "query.execution")]
    QueryExecution,
    /// Recovery process explanation.
    #[serde(rename = "recovery.process")]
    RecoveryProcess,
    /// Checkpoint safety explanation.
    #[serde(rename = "checkpoint.safety")]
    CheckpointSafety,
    /// Replication safety explanation.
    #[serde(rename = "replication.safety")]
    ReplicationSafety,
}

/// Observed snapshot metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedSnapshot {
    /// Snapshot identifier.
    pub snapshot_id: String,
    /// CommitId at observation.
    pub commit_id: u64,
}

impl ObservedSnapshot {
    /// Create from snapshot id and commit id.
    pub fn new(snapshot_id: impl Into<String>, commit_id: u64) -> Self {
        Self {
            snapshot_id: snapshot_id.into(),
            commit_id,
        }
    }
}

/// Rule evaluation result.
///
/// Per DX_EXPLANATION_MODEL.md §4:
/// - evaluation MUST be explicit (true/false)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleEvaluation {
    /// Rule condition was satisfied.
    True,
    /// Rule condition was not satisfied.
    False,
}

/// Evidence for a rule application.
///
/// Per DX_EXPLANATION_MODEL.md §4:
/// - evidence MUST be raw state, not interpretation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Key-value pairs of evidence.
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

impl Evidence {
    /// Create empty evidence.
    pub fn empty() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Create evidence with a single entry.
    pub fn with(key: impl Into<String>, value: impl Serialize) -> Self {
        let mut data = HashMap::new();
        data.insert(key.into(), serde_json::to_value(value).unwrap_or_default());
        Self { data }
    }

    /// Add evidence entry.
    pub fn add(&mut self, key: impl Into<String>, value: impl Serialize) {
        self.data
            .insert(key.into(), serde_json::to_value(value).unwrap_or_default());
    }
}

/// Rule application record.
///
/// Per DX_EXPLANATION_MODEL.md §4:
/// - rule_id MUST map to documented invariant or rule
/// - evaluation MUST be explicit
/// - evidence MUST be raw state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleApplication {
    /// Stable rule identifier (e.g., "MVCC-1", "D-1").
    pub rule_id: String,
    /// Exact rule reference/description.
    pub description: String,
    /// Evaluation result.
    pub evaluation: RuleEvaluation,
    /// Evidence supporting the evaluation.
    pub evidence: Evidence,
}

impl RuleApplication {
    /// Create a rule application with true evaluation.
    pub fn satisfied(
        rule_id: impl Into<String>,
        description: impl Into<String>,
        evidence: Evidence,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            description: description.into(),
            evaluation: RuleEvaluation::True,
            evidence,
        }
    }

    /// Create a rule application with false evaluation.
    pub fn not_satisfied(
        rule_id: impl Into<String>,
        description: impl Into<String>,
        evidence: Evidence,
    ) -> Self {
        Self {
            rule_id: rule_id.into(),
            description: description.into(),
            evaluation: RuleEvaluation::False,
            evidence,
        }
    }
}

/// Conclusion status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConclusionStatus {
    /// Conclusion determined successfully.
    Determined,
    /// Conclusion could not be determined (missing evidence).
    Undetermined,
    /// Explanation generation failed.
    Failed,
}

/// Explanation conclusion.
///
/// Per DX_EXPLANATION_MODEL.md §8.1:
/// If evidence is missing, the explanation MUST say so explicitly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conclusion {
    /// Conclusion status.
    pub status: ConclusionStatus,
    /// Result if determined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Reason if undetermined or failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl Conclusion {
    /// Create a determined conclusion.
    pub fn determined(result: impl Serialize) -> Self {
        Self {
            status: ConclusionStatus::Determined,
            result: Some(serde_json::to_value(result).unwrap_or_default()),
            reason: None,
        }
    }

    /// Create an undetermined conclusion.
    pub fn undetermined(reason: impl Into<String>) -> Self {
        Self {
            status: ConclusionStatus::Undetermined,
            result: None,
            reason: Some(reason.into()),
        }
    }

    /// Create a failed conclusion.
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            status: ConclusionStatus::Failed,
            result: None,
            reason: Some(reason.into()),
        }
    }
}

/// Complete explanation object.
///
/// Per DX_EXPLANATION_MODEL.md §4.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    /// Explanation type.
    pub explanation_type: ExplanationType,
    /// Observed snapshot metadata.
    pub observed_snapshot: ObservedSnapshot,
    /// Input parameters.
    pub inputs: HashMap<String, serde_json::Value>,
    /// Rules applied in order.
    pub rules_applied: Vec<RuleApplication>,
    /// Final conclusion.
    pub conclusion: Conclusion,
}

impl Explanation {
    /// Create a new explanation builder.
    pub fn builder(
        explanation_type: ExplanationType,
        snapshot_id: impl Into<String>,
        commit_id: u64,
    ) -> ExplanationBuilder {
        ExplanationBuilder::new(explanation_type, snapshot_id, commit_id)
    }
}

/// Builder for constructing explanations.
pub struct ExplanationBuilder {
    explanation_type: ExplanationType,
    observed_snapshot: ObservedSnapshot,
    inputs: HashMap<String, serde_json::Value>,
    rules_applied: Vec<RuleApplication>,
}

impl ExplanationBuilder {
    /// Create a new builder.
    pub fn new(
        explanation_type: ExplanationType,
        snapshot_id: impl Into<String>,
        commit_id: u64,
    ) -> Self {
        Self {
            explanation_type,
            observed_snapshot: ObservedSnapshot::new(snapshot_id, commit_id),
            inputs: HashMap::new(),
            rules_applied: Vec::new(),
        }
    }

    /// Add input.
    pub fn input(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.inputs
            .insert(key.into(), serde_json::to_value(value).unwrap_or_default());
        self
    }

    /// Add rule application.
    pub fn rule(mut self, rule: RuleApplication) -> Self {
        self.rules_applied.push(rule);
        self
    }

    /// Build with determined conclusion.
    pub fn conclude(self, result: impl Serialize) -> Explanation {
        Explanation {
            explanation_type: self.explanation_type,
            observed_snapshot: self.observed_snapshot,
            inputs: self.inputs,
            rules_applied: self.rules_applied,
            conclusion: Conclusion::determined(result),
        }
    }

    /// Build with undetermined conclusion.
    pub fn undetermined(self, reason: impl Into<String>) -> Explanation {
        Explanation {
            explanation_type: self.explanation_type,
            observed_snapshot: self.observed_snapshot,
            inputs: self.inputs,
            rules_applied: self.rules_applied,
            conclusion: Conclusion::undetermined(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explanation_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ExplanationType::MvccReadVisibility).unwrap(),
            "\"mvcc.read_visibility\""
        );
    }

    #[test]
    fn test_rule_application_satisfied() {
        let rule = RuleApplication::satisfied(
            "MVCC-1",
            "Snapshot isolation rule",
            Evidence::with("commit_id", 100),
        );
        assert_eq!(rule.evaluation, RuleEvaluation::True);
    }

    #[test]
    fn test_explanation_builder() {
        let explanation = Explanation::builder(ExplanationType::MvccReadVisibility, "snap-1", 100)
            .input("doc_id", "doc-123")
            .rule(RuleApplication::satisfied(
                "MVCC-1",
                "Visibility check",
                Evidence::empty(),
            ))
            .conclude("visible");

        assert_eq!(
            explanation.explanation_type,
            ExplanationType::MvccReadVisibility
        );
        assert_eq!(explanation.observed_snapshot.commit_id, 100);
        assert_eq!(explanation.rules_applied.len(), 1);
        assert_eq!(explanation.conclusion.status, ConclusionStatus::Determined);
    }

    #[test]
    fn test_undetermined_conclusion() {
        let explanation = Explanation::builder(ExplanationType::RecoveryProcess, "snap-1", 100)
            .undetermined("WAL segment missing");

        assert_eq!(
            explanation.conclusion.status,
            ConclusionStatus::Undetermined
        );
        assert_eq!(
            explanation.conclusion.reason,
            Some("WAL segment missing".to_string())
        );
    }
}
