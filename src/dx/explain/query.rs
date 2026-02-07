//! Query Execution Explanation
//!
//! Per DX_EXPLANATION_MODEL.md §6.2:
//! Type: query.execution
//! Explains: How a query was executed
//!
//! Inputs:
//! - Query
//! - Snapshot CommitId
//!
//! Rules Applied:
//! - Query planning rules
//! - Bounds enforcement
//! - Index advisory usage
//!
//! Read-only, Phase 4, no semantic authority.

use super::model::{Evidence, Explanation, ExplanationType, RuleApplication};
use super::rules::RuleRegistry;
use serde::{Deserialize, Serialize};

/// Query plan node type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanNodeType {
    /// Index scan.
    IndexScan,
    /// Sequential scan (bounded).
    BoundedScan,
    /// Filter operation.
    Filter,
    /// Sort operation.
    Sort,
    /// Limit operation.
    Limit,
}

/// Query plan node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    /// Node type.
    pub node_type: PlanNodeType,
    /// Description.
    pub description: String,
    /// Estimated rows.
    pub estimated_rows: Option<u64>,
    /// Index used (if any).
    pub index_used: Option<String>,
}

/// Query execution output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutionOutput {
    /// Execution plan nodes.
    pub plan: Vec<PlanNode>,
    /// Bounds applied.
    pub bounds_applied: Vec<String>,
    /// Total estimated cost.
    pub estimated_cost: u64,
    /// Whether query is bounded.
    pub is_bounded: bool,
}

/// Query execution explainer.
///
/// Read-only, Phase 4, no semantic authority.
pub struct QueryExplainer {
    rules: RuleRegistry,
}

impl QueryExplainer {
    /// Create a new query explainer.
    pub fn new() -> Self {
        Self {
            rules: RuleRegistry::new(),
        }
    }

    /// Generate query execution explanation.
    ///
    /// Per DX_EXPLANATION_MODEL.md §6.2:
    /// Conclusion: Deterministic execution plan + guarantees enforced.
    pub fn explain(
        &self,
        query: &str,
        snapshot_commit_id: u64,
        plan: Vec<PlanNode>,
        bounds: Vec<String>,
    ) -> Explanation {
        let snapshot_id = format!("snap-{}", snapshot_commit_id);
        let is_bounded = !bounds.is_empty();

        let mut builder = Explanation::builder(
            ExplanationType::QueryExecution,
            &snapshot_id,
            snapshot_commit_id,
        )
        .input("query", query)
        .input("snapshot_commit_id", snapshot_commit_id);

        // Check bounded query rule
        let mut evidence = Evidence::empty();
        evidence.add("bounds_count", bounds.len());
        evidence.add("bounds", bounds.clone());

        if is_bounded {
            builder = builder.rule(RuleApplication::satisfied(
                "Q-1",
                "Query is bounded (CORE_INVARIANTS.md Q1)",
                evidence,
            ));
        } else {
            builder = builder.rule(RuleApplication::not_satisfied(
                "Q-1",
                "Query must be bounded",
                evidence,
            ));
        }

        // Check index usage (advisory)
        let uses_index = plan.iter().any(|n| n.index_used.is_some());
        let mut index_evidence = Evidence::empty();
        index_evidence.add("uses_index", uses_index);
        if uses_index {
            let indexes: Vec<_> = plan.iter().filter_map(|n| n.index_used.clone()).collect();
            index_evidence.add("indexes_used", indexes);
        }

        builder = builder.rule(RuleApplication::satisfied(
            "PERF-4",
            "Index acceleration (advisory, per PERF_INDEX_ACCELERATION.md)",
            index_evidence,
        ));

        // Calculate estimated cost
        let estimated_cost: u64 = plan.iter().map(|n| n.estimated_rows.unwrap_or(1)).sum();

        builder.conclude(QueryExecutionOutput {
            plan,
            bounds_applied: bounds,
            estimated_cost,
            is_bounded,
        })
    }
}

impl Default for QueryExplainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounded_query() {
        let explainer = QueryExplainer::new();
        let plan = vec![PlanNode {
            node_type: PlanNodeType::IndexScan,
            description: "Scan index on user_id".to_string(),
            estimated_rows: Some(10),
            index_used: Some("idx_user_id".to_string()),
        }];

        let explanation = explainer.explain(
            "SELECT * FROM users WHERE user_id = 1",
            100,
            plan,
            vec!["LIMIT 100".to_string()],
        );

        assert_eq!(
            explanation.explanation_type,
            ExplanationType::QueryExecution
        );
        assert!(!explanation.rules_applied.is_empty());
    }

    #[test]
    fn test_deterministic_plan() {
        let explainer = QueryExplainer::new();
        let plan = vec![PlanNode {
            node_type: PlanNodeType::BoundedScan,
            description: "Bounded sequential scan".to_string(),
            estimated_rows: Some(100),
            index_used: None,
        }];

        let exp1 = explainer.explain(
            "SELECT * FROM t",
            100,
            plan.clone(),
            vec!["LIMIT 10".to_string()],
        );
        let exp2 = explainer.explain("SELECT * FROM t", 100, plan, vec!["LIMIT 10".to_string()]);

        // Per P4-6: Deterministic
        assert_eq!(exp1.rules_applied.len(), exp2.rules_applied.len());
    }
}
