//! Explain plan output per QUERY.md §292-304
//!
//! Produces deterministic, human-readable explain output.

use std::fmt;

use super::errors::PlannerError;
use super::planner::QueryPlan;

/// Explain plan output
#[derive(Debug, Clone)]
pub struct ExplainPlan {
    /// Whether planning succeeded
    pub accepted: bool,
    /// Selected index (if accepted)
    pub selected_index: Option<String>,
    /// Scan type description
    pub scan_type: Option<String>,
    /// List of predicates
    pub predicates: Vec<String>,
    /// Sort description
    pub sort: Option<String>,
    /// Limit
    pub limit: Option<u64>,
    /// Proven bounds
    pub max_scan: Option<u64>,
    /// Rejection reason (if rejected)
    pub rejection_reason: Option<String>,
    /// Rejection error code (if rejected)
    pub rejection_code: Option<String>,
}

impl ExplainPlan {
    /// Creates an explain plan from a successful query plan
    pub fn from_plan(plan: &QueryPlan) -> Self {
        let predicates: Vec<String> = plan
            .predicates
            .iter()
            .map(|p| {
                format!(
                    "{} {} {:?}",
                    p.field,
                    p.op.op_name(),
                    match &p.op {
                        super::ast::FilterOp::Eq(v) => v,
                        super::ast::FilterOp::Gte(v) => v,
                        super::ast::FilterOp::Gt(v) => v,
                        super::ast::FilterOp::Lte(v) => v,
                        super::ast::FilterOp::Lt(v) => v,
                    }
                )
            })
            .collect();

        let sort = plan
            .sort
            .as_ref()
            .map(|s| format!("{} {}", s.field, s.direction.as_str()));

        Self {
            accepted: true,
            selected_index: Some(plan.chosen_index.clone()),
            scan_type: Some(plan.scan_type.as_str().to_string()),
            predicates,
            sort,
            limit: Some(plan.limit),
            max_scan: Some(plan.bounds_proof.max_scan),
            rejection_reason: None,
            rejection_code: None,
        }
    }

    /// Creates an explain plan from a planning error
    pub fn from_error(err: &PlannerError) -> Self {
        Self {
            accepted: false,
            selected_index: None,
            scan_type: None,
            predicates: Vec::new(),
            sort: None,
            limit: None,
            max_scan: None,
            rejection_reason: Some(err.message().to_string()),
            rejection_code: Some(err.code().code().to_string()),
        }
    }
}

impl fmt::Display for ExplainPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== EXPLAIN PLAN ===")?;

        if self.accepted {
            writeln!(f, "Status: ACCEPTED")?;
            if let Some(idx) = &self.selected_index {
                writeln!(f, "Index: {}", idx)?;
            }
            if let Some(scan) = &self.scan_type {
                writeln!(f, "Scan Type: {}", scan)?;
            }
            if !self.predicates.is_empty() {
                writeln!(f, "Predicates:")?;
                for pred in &self.predicates {
                    writeln!(f, "  - {}", pred)?;
                }
            }
            if let Some(sort) = &self.sort {
                writeln!(f, "Sort: {}", sort)?;
            }
            if let Some(limit) = self.limit {
                writeln!(f, "Limit: {}", limit)?;
            }
            if let Some(max_scan) = self.max_scan {
                writeln!(f, "Max Scan: {} documents", max_scan)?;
            }
        } else {
            writeln!(f, "Status: REJECTED")?;
            if let Some(code) = &self.rejection_code {
                writeln!(f, "Error Code: {}", code)?;
            }
            if let Some(reason) = &self.rejection_reason {
                writeln!(f, "Reason: {}", reason)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::ast::{Predicate, Query};
    use crate::planner::planner::{IndexMetadata, QueryPlanner, SchemaRegistry};
    use serde_json::json;
    use std::collections::HashSet;

    struct TestSchemaRegistry;

    impl SchemaRegistry for TestSchemaRegistry {
        fn schema_exists(&self, _: &str) -> bool {
            true
        }
        fn schema_version_exists(&self, _: &str, _: &str) -> bool {
            true
        }
    }

    #[test]
    fn test_explain_accepted_plan() {
        let registry = TestSchemaRegistry;
        let indexes = IndexMetadata::with_indexes(["email"]);
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("email", json!("test@example.com")))
            .with_limit(10);

        let plan = planner.plan(&query).unwrap();
        let explain = ExplainPlan::from_plan(&plan);

        assert!(explain.accepted);
        assert_eq!(explain.selected_index, Some("email".into()));
        assert_eq!(explain.scan_type, Some("INDEX_EQ".into()));
        assert_eq!(explain.limit, Some(10));

        let output = format!("{}", explain);
        assert!(output.contains("ACCEPTED"));
        assert!(output.contains("email"));
    }

    #[test]
    fn test_explain_rejected_plan() {
        let err = PlannerError::unindexed_field("name");
        let explain = ExplainPlan::from_error(&err);

        assert!(!explain.accepted);
        assert_eq!(
            explain.rejection_code,
            Some("AERO_QUERY_UNINDEXED_FIELD".into())
        );

        let output = format!("{}", explain);
        assert!(output.contains("REJECTED"));
        assert!(output.contains("AERO_QUERY_UNINDEXED_FIELD"));
    }

    #[test]
    fn test_explain_deterministic() {
        let registry = TestSchemaRegistry;
        let indexes = IndexMetadata::with_indexes(["email"]);
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("email", json!("test@example.com")))
            .with_limit(10);

        let plan = planner.plan(&query).unwrap();
        let explain1 = format!("{}", ExplainPlan::from_plan(&plan));
        let explain2 = format!("{}", ExplainPlan::from_plan(&plan));

        assert_eq!(explain1, explain2);
    }
}
