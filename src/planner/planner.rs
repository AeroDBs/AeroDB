//! Query planner per QUERY.md
//!
//! Produces deterministic, bounded query plans.
//!
//! Index selection priority (strict order):
//! 1. Primary key equality (_id)
//! 2. Indexed equality predicate
//! 3. Indexed range predicate with limit
//!
//! Ties broken lexicographically by field name.

use std::collections::HashSet;

use super::ast::{Predicate, Query, SortSpec};
use super::bounds::{BoundednessAnalyzer, BoundednessProof};
use super::errors::{PlannerError, PlannerResult};

/// Index metadata provided to the planner
#[derive(Debug, Clone)]
pub struct IndexMetadata {
    /// Set of indexed field names (excluding _id which is always indexed)
    pub indexed_fields: HashSet<String>,
}

impl IndexMetadata {
    /// Creates empty index metadata (only _id is indexed)
    pub fn new() -> Self {
        Self {
            indexed_fields: HashSet::new(),
        }
    }

    /// Creates index metadata with the given indexed fields
    pub fn with_indexes(fields: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            indexed_fields: fields.into_iter().map(Into::into).collect(),
        }
    }

    /// Checks if a field is indexed
    pub fn is_indexed(&self, field: &str) -> bool {
        field == "_id" || self.indexed_fields.contains(field)
    }
}

impl Default for IndexMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Scan type used by query plan
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanType {
    /// Primary key equality lookup
    PrimaryKey,
    /// Indexed equality scan
    IndexedEquality,
    /// Indexed range scan with limit
    IndexedRange,
}

impl ScanType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanType::PrimaryKey => "PK_LOOKUP",
            ScanType::IndexedEquality => "INDEX_EQ",
            ScanType::IndexedRange => "INDEX_RANGE",
        }
    }
}

/// Immutable query plan (no runtime state)
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// Collection to query
    pub collection: String,
    /// Schema ID
    pub schema_id: String,
    /// Schema version
    pub schema_version: String,
    /// Chosen index (field name)
    pub chosen_index: String,
    /// Scan type
    pub scan_type: ScanType,
    /// Filter predicates to apply
    pub predicates: Vec<Predicate>,
    /// Sort specification (if any)
    pub sort: Option<SortSpec>,
    /// Limit
    pub limit: u64,
    /// Boundedness proof
    pub bounds_proof: BoundednessProof,
}

/// Schema registry trait for planner (read-only)
pub trait SchemaRegistry {
    /// Check if schema exists
    fn schema_exists(&self, schema_id: &str) -> bool;
    /// Check if schema version exists
    fn schema_version_exists(&self, schema_id: &str, version: &str) -> bool;
}

/// Query planner that produces deterministic plans
pub struct QueryPlanner<'a, S: SchemaRegistry> {
    schema_registry: &'a S,
    index_metadata: &'a IndexMetadata,
}

impl<'a, S: SchemaRegistry> QueryPlanner<'a, S> {
    /// Creates a new planner
    pub fn new(schema_registry: &'a S, index_metadata: &'a IndexMetadata) -> Self {
        Self {
            schema_registry,
            index_metadata,
        }
    }

    /// Plans a query, returning an immutable plan or error.
    ///
    /// This method is deterministic: same inputs → same plan.
    pub fn plan(&self, query: &Query) -> PlannerResult<QueryPlan> {
        // 1. Validate schema version is present
        let schema_version = query
            .schema_version
            .as_ref()
            .ok_or_else(PlannerError::schema_version_required)?;

        // 2. Validate schema exists
        if !self.schema_registry.schema_exists(&query.schema_id) {
            return Err(PlannerError::unknown_schema(&query.schema_id));
        }

        // 3. Validate schema version exists
        if !self
            .schema_registry
            .schema_version_exists(&query.schema_id, schema_version)
        {
            return Err(PlannerError::unknown_schema_version(
                &query.schema_id,
                schema_version,
            ));
        }

        // 4. Prove boundedness BEFORE plan generation
        let analyzer = BoundednessAnalyzer::new(&self.index_metadata.indexed_fields);
        let bounds_proof = analyzer.analyze(query)?;

        // 5. Select index using strict priority order
        let (chosen_index, scan_type) = self.select_index(query)?;

        // 6. Build immutable plan
        Ok(QueryPlan {
            collection: query.collection.clone(),
            schema_id: query.schema_id.clone(),
            schema_version: schema_version.clone(),
            chosen_index,
            scan_type,
            predicates: query.predicates.clone(),
            sort: query.sort.clone(),
            limit: query.limit.unwrap(), // Already validated in bounds
            bounds_proof,
        })
    }

    /// Selects index using strict priority order per QUERY.md §230-237.
    ///
    /// Priority:
    /// 1. Primary key equality (_id)
    /// 2. Indexed equality predicate
    /// 3. Indexed range predicate with limit
    ///
    /// Ties broken lexicographically.
    fn select_index(&self, query: &Query) -> PlannerResult<(String, ScanType)> {
        // Priority 1: Primary key equality
        if query.has_pk_filter() {
            return Ok(("_id".to_string(), ScanType::PrimaryKey));
        }

        // Collect equality predicates on indexed fields
        let mut eq_candidates: Vec<&str> = query
            .predicates
            .iter()
            .filter(|p| p.is_equality() && self.index_metadata.is_indexed(&p.field))
            .map(|p| p.field.as_str())
            .collect();

        // Priority 2: Indexed equality (lexicographically smallest)
        if !eq_candidates.is_empty() {
            eq_candidates.sort();
            return Ok((eq_candidates[0].to_string(), ScanType::IndexedEquality));
        }

        // Collect range predicates on indexed fields
        let mut range_candidates: Vec<&str> = query
            .predicates
            .iter()
            .filter(|p| p.is_range() && self.index_metadata.is_indexed(&p.field))
            .map(|p| p.field.as_str())
            .collect();

        // Priority 3: Indexed range (lexicographically smallest)
        if !range_candidates.is_empty() {
            range_candidates.sort();
            return Ok((range_candidates[0].to_string(), ScanType::IndexedRange));
        }

        // No usable index found - should have been caught by bounds check
        // This path indicates a bug (empty query with no filters)
        Err(PlannerError::unbounded("No usable index found"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Simple schema registry for testing
    struct TestSchemaRegistry {
        schemas: HashSet<(String, String)>,
    }

    impl TestSchemaRegistry {
        fn new() -> Self {
            let mut schemas = HashSet::new();
            schemas.insert(("users".into(), "v1".into()));
            schemas.insert(("posts".into(), "v1".into()));
            Self { schemas }
        }
    }

    impl SchemaRegistry for TestSchemaRegistry {
        fn schema_exists(&self, schema_id: &str) -> bool {
            self.schemas.iter().any(|(id, _)| id == schema_id)
        }

        fn schema_version_exists(&self, schema_id: &str, version: &str) -> bool {
            self.schemas.contains(&(schema_id.into(), version.into()))
        }
    }

    #[test]
    fn test_pk_equality_plan() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::new();
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("_id", json!("user_123")))
            .with_limit(1);

        let plan = planner.plan(&query).unwrap();
        assert_eq!(plan.scan_type, ScanType::PrimaryKey);
        assert_eq!(plan.chosen_index, "_id");
    }

    #[test]
    fn test_indexed_equality_plan() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::with_indexes(["email"]);
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("email", json!("test@example.com")))
            .with_limit(10);

        let plan = planner.plan(&query).unwrap();
        assert_eq!(plan.scan_type, ScanType::IndexedEquality);
        assert_eq!(plan.chosen_index, "email");
    }

    #[test]
    fn test_indexed_range_plan() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::with_indexes(["age"]);
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::gte("age", json!(18)))
            .with_predicate(Predicate::lte("age", json!(30)))
            .with_limit(100);

        let plan = planner.plan(&query).unwrap();
        assert_eq!(plan.scan_type, ScanType::IndexedRange);
        assert_eq!(plan.chosen_index, "age");
    }

    #[test]
    fn test_missing_limit_rejected() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::with_indexes(["email"]);
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("email", json!("test@example.com")));
        // No limit

        let result = planner.plan(&query);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_QUERY_LIMIT_REQUIRED"
        );
    }

    #[test]
    fn test_unindexed_filter_rejected() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::new(); // No indexes
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("name", json!("Alice")))
            .with_limit(10);

        let result = planner.plan(&query);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_QUERY_UNINDEXED_FIELD"
        );
    }

    #[test]
    fn test_unindexed_sort_rejected() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::with_indexes(["email"]);
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("email", json!("test@example.com")))
            .with_sort(crate::planner::ast::SortSpec::asc("created_at"))
            .with_limit(10);

        let result = planner.plan(&query);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_QUERY_SORT_NOT_INDEXED"
        );
    }

    #[test]
    fn test_schema_missing_rejected() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::new();
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "nonexistent")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("_id", json!("user_1")))
            .with_limit(1);

        let result = planner.plan(&query);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code().code(), "AERO_UNKNOWN_SCHEMA");
    }

    #[test]
    fn test_schema_version_missing_rejected() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::new();
        let planner = QueryPlanner::new(&registry, &indexes);

        // Missing schema_version field
        let query = Query::new("users", "users")
            .with_predicate(Predicate::eq("_id", json!("user_1")))
            .with_limit(1);

        let result = planner.plan(&query);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code().code(),
            "AERO_SCHEMA_VERSION_REQUIRED"
        );
    }

    #[test]
    fn test_deterministic_planning() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::with_indexes(["email", "age"]);
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("email", json!("test@example.com")))
            .with_predicate(Predicate::gte("age", json!(18)))
            .with_limit(100);

        // Run planning multiple times
        let plan1 = planner.plan(&query).unwrap();
        let plan2 = planner.plan(&query).unwrap();
        let plan3 = planner.plan(&query).unwrap();

        // All plans must be identical
        assert_eq!(plan1.chosen_index, plan2.chosen_index);
        assert_eq!(plan2.chosen_index, plan3.chosen_index);
        assert_eq!(plan1.scan_type, plan2.scan_type);
        assert_eq!(plan2.scan_type, plan3.scan_type);
    }

    #[test]
    fn test_lexicographic_index_selection() {
        let registry = TestSchemaRegistry::new();
        let indexes = IndexMetadata::with_indexes(["zebra", "alpha", "beta"]);
        let planner = QueryPlanner::new(&registry, &indexes);

        let query = Query::new("users", "users")
            .with_schema_version("v1")
            .with_predicate(Predicate::eq("zebra", json!("z")))
            .with_predicate(Predicate::eq("beta", json!("b")))
            .with_predicate(Predicate::eq("alpha", json!("a")))
            .with_limit(10);

        let plan = planner.plan(&query).unwrap();
        // Should pick "alpha" (lexicographically smallest)
        assert_eq!(plan.chosen_index, "alpha");
    }
}
