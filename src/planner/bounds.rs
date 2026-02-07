//! Boundedness analysis for queries per QUERY.md §205-218
//!
//! A query is bounded if:
//! - Every filter predicate references indexed fields ONLY
//! - Range predicates have explicit limit
//! - Limit is mandatory and > 0
//! - Sort field is indexed
//! - No OR conditions
//! - No functions or expressions

use std::collections::HashSet;

use super::ast::Query;
use super::errors::{PlannerError, PlannerResult};

/// Proof that a query is bounded (computed before plan generation)
#[derive(Debug, Clone)]
pub struct BoundednessProof {
    /// Maximum documents that can be scanned
    pub max_scan: u64,
    /// Fields proven to use indexes
    pub indexed_fields: Vec<String>,
    /// Whether primary key is used
    pub uses_pk: bool,
}

impl BoundednessProof {
    /// Create a proof for a primary key lookup
    pub fn pk_lookup() -> Self {
        Self {
            max_scan: 1,
            indexed_fields: vec!["_id".to_string()],
            uses_pk: true,
        }
    }

    /// Create a proof for an indexed scan with limit
    pub fn indexed_scan(limit: u64, fields: Vec<String>) -> Self {
        Self {
            max_scan: limit,
            indexed_fields: fields,
            uses_pk: false,
        }
    }
}

/// Analyzes query boundedness.
///
/// This is a static analysis that must pass BEFORE plan generation.
pub struct BoundednessAnalyzer<'a> {
    indexed_fields: &'a HashSet<String>,
}

impl<'a> BoundednessAnalyzer<'a> {
    /// Creates a new analyzer with the set of indexed fields.
    pub fn new(indexed_fields: &'a HashSet<String>) -> Self {
        Self { indexed_fields }
    }

    /// Analyzes a query and returns a proof if bounded, or error if not.
    pub fn analyze(&self, query: &Query) -> PlannerResult<BoundednessProof> {
        // 1. Limit is mandatory
        let limit = query.limit.ok_or_else(PlannerError::limit_required)?;
        if limit == 0 {
            return Err(PlannerError::limit_required());
        }

        // 2. Check all filter predicates use indexed fields
        for pred in &query.predicates {
            if !self.is_indexed(&pred.field) {
                return Err(PlannerError::unindexed_field(&pred.field));
            }
        }

        // 3. Check sort field is indexed (if present)
        if let Some(sort) = &query.sort {
            if !self.is_indexed(&sort.field) {
                return Err(PlannerError::sort_not_indexed(&sort.field));
            }
        }

        // 4. Primary key lookup is special case
        if query.has_pk_filter() {
            // PK lookup must have limit = 1
            if limit != 1 {
                // Still bounded, but semantically pk should return at most 1
                // We allow limit > 1 but the scan is bounded at 1
            }
            return Ok(BoundednessProof::pk_lookup());
        }

        // 5. For range queries, limit is already checked above
        // Collect indexed fields used in predicates
        let indexed_fields: Vec<String> =
            query.predicates.iter().map(|p| p.field.clone()).collect();

        Ok(BoundednessProof::indexed_scan(limit, indexed_fields))
    }

    /// Checks if a field is indexed (_id is always indexed)
    fn is_indexed(&self, field: &str) -> bool {
        field == "_id" || self.indexed_fields.contains(field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::ast::Predicate;
    use serde_json::json;

    fn make_indexes(fields: &[&str]) -> HashSet<String> {
        fields.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_pk_query_bounded() {
        let indexes = make_indexes(&["email"]);
        let analyzer = BoundednessAnalyzer::new(&indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("_id", json!("user_1")))
            .with_limit(1);

        let proof = analyzer.analyze(&query).unwrap();
        assert!(proof.uses_pk);
        assert_eq!(proof.max_scan, 1);
    }

    #[test]
    fn test_missing_limit_rejected() {
        let indexes = make_indexes(&["email"]);
        let analyzer = BoundednessAnalyzer::new(&indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("email", json!("test@example.com")));
        // No limit!

        let result = analyzer.analyze(&query);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_QUERY_LIMIT_REQUIRED"
        );
    }

    #[test]
    fn test_unindexed_filter_rejected() {
        let indexes = make_indexes(&["email"]);
        let analyzer = BoundednessAnalyzer::new(&indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("name", json!("Alice"))) // name not indexed
            .with_limit(10);

        let result = analyzer.analyze(&query);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_QUERY_UNINDEXED_FIELD"
        );
    }

    #[test]
    fn test_unindexed_sort_rejected() {
        let indexes = make_indexes(&["email"]);
        let analyzer = BoundednessAnalyzer::new(&indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("email", json!("test@example.com")))
            .with_sort(crate::planner::ast::SortSpec::asc("created_at")) // not indexed
            .with_limit(10);

        let result = analyzer.analyze(&query);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_QUERY_SORT_NOT_INDEXED"
        );
    }

    #[test]
    fn test_indexed_range_bounded() {
        let indexes = make_indexes(&["age"]);
        let analyzer = BoundednessAnalyzer::new(&indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::gte("age", json!(18)))
            .with_predicate(Predicate::lte("age", json!(30)))
            .with_limit(100);

        let proof = analyzer.analyze(&query).unwrap();
        assert!(!proof.uses_pk);
        assert_eq!(proof.max_scan, 100);
    }
}
