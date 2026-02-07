//! MVCC Read Visibility Explanation
//!
//! Per DX_EXPLANATION_MODEL.md §6.1:
//! Type: mvcc.read_visibility
//! Explains: Why a specific document version is visible or invisible
//!
//! Inputs:
//! - Document ID
//! - Snapshot CommitId
//!
//! Rules Applied:
//! - MVCC visibility rules
//! - CommitId comparisons
//! - Tombstone handling
//!
//! Read-only, Phase 4, no semantic authority.

use super::model::{Evidence, Explanation, ExplanationType, RuleApplication};
use super::rules::RuleRegistry;
use serde::{Deserialize, Serialize};

/// Version chain entry for visibility explanation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    /// Version identifier.
    pub version_id: u64,
    /// CommitId when this version was created.
    pub created_at_commit_id: u64,
    /// CommitId when this version was deleted (if tombstoned).
    pub deleted_at_commit_id: Option<u64>,
    /// Whether this version is a tombstone.
    pub is_tombstone: bool,
}

/// Visibility decision result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityResult {
    /// Version is visible.
    Visible,
    /// Version is not visible.
    NotVisible,
    /// No version exists.
    NoVersion,
}

/// Read visibility explanation input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilityInput {
    /// Document identifier.
    pub doc_id: String,
    /// Snapshot CommitId.
    pub snapshot_commit_id: u64,
}

/// Read visibility explanation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilityOutput {
    /// Visibility decision.
    pub result: VisibilityResult,
    /// Visible version ID (if any).
    pub visible_version_id: Option<u64>,
    /// Visible version CommitId (if any).
    pub visible_version_commit_id: Option<u64>,
}

/// Visibility explanation generator.
///
/// Read-only, Phase 4, no semantic authority.
pub struct VisibilityExplainer {
    rules: RuleRegistry,
}

impl VisibilityExplainer {
    /// Create a new visibility explainer.
    pub fn new() -> Self {
        Self {
            rules: RuleRegistry::new(),
        }
    }

    /// Generate visibility explanation.
    ///
    /// Per DX_EXPLANATION_MODEL.md §6.1:
    /// Conclusion: Visible version ID OR explicit "no visible version" result.
    pub fn explain(
        &self,
        doc_id: &str,
        snapshot_commit_id: u64,
        version_chain: &[VersionEntry],
    ) -> Explanation {
        let snapshot_id = format!("snap-{}", snapshot_commit_id);
        let mut builder = Explanation::builder(
            ExplanationType::MvccReadVisibility,
            &snapshot_id,
            snapshot_commit_id,
        )
        .input("doc_id", doc_id)
        .input("snapshot_commit_id", snapshot_commit_id);

        // Empty version chain
        if version_chain.is_empty() {
            let evidence = Evidence::with("version_count", 0);
            builder = builder.rule(RuleApplication::not_satisfied(
                "MVCC-VIS-1",
                self.rules.description("MVCC-VIS-1"),
                evidence,
            ));

            return builder.conclude(VisibilityOutput {
                result: VisibilityResult::NoVersion,
                visible_version_id: None,
                visible_version_commit_id: None,
            });
        }

        // Find visible version
        let mut visible_version: Option<&VersionEntry> = None;

        for version in version_chain {
            // Check MVCC-VIS-1: CommitId <= snapshot CommitId
            let is_created_visible = version.created_at_commit_id <= snapshot_commit_id;

            // Check tombstone visibility
            let is_deleted = match version.deleted_at_commit_id {
                Some(deleted_at) => deleted_at <= snapshot_commit_id,
                None => false,
            };

            let mut evidence = Evidence::empty();
            evidence.add("version_id", version.version_id);
            evidence.add("version_commit_id", version.created_at_commit_id);
            evidence.add("snapshot_commit_id", snapshot_commit_id);
            evidence.add(
                "commit_comparison",
                format!(
                    "{} <= {} = {}",
                    version.created_at_commit_id, snapshot_commit_id, is_created_visible
                ),
            );

            if is_created_visible && !is_deleted && !version.is_tombstone {
                builder = builder.rule(RuleApplication::satisfied(
                    "MVCC-VIS-1",
                    self.rules.description("MVCC-VIS-1"),
                    evidence,
                ));
                visible_version = Some(version);
                break;
            } else {
                builder = builder.rule(RuleApplication::not_satisfied(
                    "MVCC-VIS-1",
                    self.rules.description("MVCC-VIS-1"),
                    evidence,
                ));
            }
        }

        // Generate conclusion
        match visible_version {
            Some(v) => builder.conclude(VisibilityOutput {
                result: VisibilityResult::Visible,
                visible_version_id: Some(v.version_id),
                visible_version_commit_id: Some(v.created_at_commit_id),
            }),
            None => builder.conclude(VisibilityOutput {
                result: VisibilityResult::NotVisible,
                visible_version_id: None,
                visible_version_commit_id: None,
            }),
        }
    }
}

impl Default for VisibilityExplainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visible_version() {
        let explainer = VisibilityExplainer::new();
        let versions = vec![VersionEntry {
            version_id: 1,
            created_at_commit_id: 50,
            deleted_at_commit_id: None,
            is_tombstone: false,
        }];

        let explanation = explainer.explain("doc-1", 100, &versions);

        assert_eq!(
            explanation.explanation_type,
            ExplanationType::MvccReadVisibility
        );
        assert!(!explanation.rules_applied.is_empty());
    }

    #[test]
    fn test_no_version() {
        let explainer = VisibilityExplainer::new();
        let explanation = explainer.explain("doc-1", 100, &[]);

        // Should have undetermined or NotVisible result
        assert_eq!(
            explanation.explanation_type,
            ExplanationType::MvccReadVisibility
        );
    }

    #[test]
    fn test_version_not_yet_visible() {
        let explainer = VisibilityExplainer::new();
        let versions = vec![VersionEntry {
            version_id: 1,
            created_at_commit_id: 150, // Created after snapshot
            deleted_at_commit_id: None,
            is_tombstone: false,
        }];

        let explanation = explainer.explain("doc-1", 100, &versions);

        // Should have rule application showing not satisfied
        assert!(!explanation.rules_applied.is_empty());
    }

    #[test]
    fn test_determinism() {
        // Per P4-6: Deterministic Observation
        let explainer = VisibilityExplainer::new();
        let versions = vec![VersionEntry {
            version_id: 1,
            created_at_commit_id: 50,
            deleted_at_commit_id: None,
            is_tombstone: false,
        }];

        let exp1 = explainer.explain("doc-1", 100, &versions);
        let exp2 = explainer.explain("doc-1", 100, &versions);

        assert_eq!(
            exp1.observed_snapshot.commit_id,
            exp2.observed_snapshot.commit_id
        );
        assert_eq!(exp1.rules_applied.len(), exp2.rules_applied.len());
    }
}
