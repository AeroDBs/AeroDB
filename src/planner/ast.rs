//! Query AST structures per QUERY.md
//!
//! Defines the parsed query representation used by the planner.

use std::collections::HashMap;

/// Filter operation types
#[derive(Debug, Clone, PartialEq)]
pub enum FilterOp {
    /// Equality: field = value
    Eq(serde_json::Value),
    /// Greater than or equal: field >= value
    Gte(serde_json::Value),
    /// Greater than: field > value
    Gt(serde_json::Value),
    /// Less than or equal: field <= value
    Lte(serde_json::Value),
    /// Less than: field < value
    Lt(serde_json::Value),
}

impl FilterOp {
    /// Returns true if this is an equality operation
    pub fn is_equality(&self) -> bool {
        matches!(self, FilterOp::Eq(_))
    }

    /// Returns true if this is a range operation
    pub fn is_range(&self) -> bool {
        matches!(
            self,
            FilterOp::Gte(_) | FilterOp::Gt(_) | FilterOp::Lte(_) | FilterOp::Lt(_)
        )
    }

    /// Returns the operation name for explain output
    pub fn op_name(&self) -> &'static str {
        match self {
            FilterOp::Eq(_) => "eq",
            FilterOp::Gte(_) => "gte",
            FilterOp::Gt(_) => "gt",
            FilterOp::Lte(_) => "lte",
            FilterOp::Lt(_) => "lt",
        }
    }
}

/// A single predicate (field + operation)
#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    /// Field name
    pub field: String,
    /// Filter operation
    pub op: FilterOp,
}

impl Predicate {
    /// Create an equality predicate
    pub fn eq(field: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            op: FilterOp::Eq(value),
        }
    }

    /// Create a range predicate (gte)
    pub fn gte(field: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            op: FilterOp::Gte(value),
        }
    }

    /// Create a range predicate (lte)
    pub fn lte(field: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            op: FilterOp::Lte(value),
        }
    }

    /// Create a range predicate (gt)
    pub fn gt(field: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            op: FilterOp::Gt(value),
        }
    }

    /// Create a range predicate (lt)
    pub fn lt(field: impl Into<String>, value: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            op: FilterOp::Lt(value),
        }
    }

    /// Returns true if this is an equality predicate
    pub fn is_equality(&self) -> bool {
        self.op.is_equality()
    }

    /// Returns true if this is a range predicate
    pub fn is_range(&self) -> bool {
        self.op.is_range()
    }

    /// Returns true if this is a primary key predicate
    pub fn is_primary_key(&self) -> bool {
        self.field == "_id" && self.is_equality()
    }
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }
}

/// Sort specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortSpec {
    /// Field to sort by
    pub field: String,
    /// Sort direction
    pub direction: SortDirection,
}

impl SortSpec {
    pub fn asc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Asc,
        }
    }

    pub fn desc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            direction: SortDirection::Desc,
        }
    }
}

/// Parsed query AST per QUERY.md §53-80
#[derive(Debug, Clone)]
pub struct Query {
    /// Target collection name
    pub collection: String,
    /// Schema identifier
    pub schema_id: String,
    /// Schema version (required)
    pub schema_version: Option<String>,
    /// Filter predicates (all combined with AND)
    pub predicates: Vec<Predicate>,
    /// Sort specification (optional, single field only in Phase 0)
    pub sort: Option<SortSpec>,
    /// Limit (mandatory)
    pub limit: Option<u64>,
}

impl Query {
    /// Creates a new query builder
    pub fn new(collection: impl Into<String>, schema_id: impl Into<String>) -> Self {
        Self {
            collection: collection.into(),
            schema_id: schema_id.into(),
            schema_version: None,
            predicates: Vec::new(),
            sort: None,
            limit: None,
        }
    }

    /// Sets the schema version
    pub fn with_schema_version(mut self, version: impl Into<String>) -> Self {
        self.schema_version = Some(version.into());
        self
    }

    /// Adds a predicate
    pub fn with_predicate(mut self, predicate: Predicate) -> Self {
        self.predicates.push(predicate);
        self
    }

    /// Adds an equality filter
    pub fn filter_eq(self, field: impl Into<String>, value: serde_json::Value) -> Self {
        self.with_predicate(Predicate::eq(field, value))
    }

    /// Sets the sort specification
    pub fn with_sort(mut self, sort: SortSpec) -> Self {
        self.sort = Some(sort);
        self
    }

    /// Sets the limit
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Returns true if query has a primary key equality filter
    pub fn has_pk_filter(&self) -> bool {
        self.predicates.iter().any(|p| p.is_primary_key())
    }

    /// Returns predicates grouped by field
    pub fn predicates_by_field(&self) -> HashMap<&str, Vec<&Predicate>> {
        let mut map: HashMap<&str, Vec<&Predicate>> = HashMap::new();
        for pred in &self.predicates {
            map.entry(&pred.field).or_default().push(pred);
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_query_builder() {
        let query = Query::new("users", "user_schema")
            .with_schema_version("v1")
            .filter_eq("_id", json!("user_123"))
            .with_limit(1);

        assert_eq!(query.collection, "users");
        assert_eq!(query.schema_version, Some("v1".into()));
        assert_eq!(query.limit, Some(1));
        assert!(query.has_pk_filter());
    }

    #[test]
    fn test_predicate_types() {
        let eq = Predicate::eq("name", json!("Alice"));
        assert!(eq.is_equality());
        assert!(!eq.is_range());

        let gte = Predicate::gte("age", json!(18));
        assert!(!gte.is_equality());
        assert!(gte.is_range());
    }

    #[test]
    fn test_primary_key_predicate() {
        let pk = Predicate::eq("_id", json!("abc"));
        assert!(pk.is_primary_key());

        let not_pk = Predicate::eq("email", json!("x@y.com"));
        assert!(!not_pk.is_primary_key());

        let range_id = Predicate::gte("_id", json!("a"));
        assert!(!range_id.is_primary_key());
    }

    #[test]
    fn test_sort_spec() {
        let asc = SortSpec::asc("created_at");
        assert_eq!(asc.direction, SortDirection::Asc);
        assert_eq!(asc.field, "created_at");
    }
}
