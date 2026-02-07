//! # Filter Expression AST
//!
//! Represents filter operations for REST queries.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Filter operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOperator {
    /// Equals
    #[serde(rename = "eq")]
    Eq,

    /// Not equals
    #[serde(rename = "neq")]
    Neq,

    /// Greater than
    #[serde(rename = "gt")]
    Gt,

    /// Greater than or equal
    #[serde(rename = "gte")]
    Gte,

    /// Less than
    #[serde(rename = "lt")]
    Lt,

    /// Less than or equal
    #[serde(rename = "lte")]
    Lte,

    /// Pattern match (LIKE)
    #[serde(rename = "like")]
    Like,

    /// Value in list
    #[serde(rename = "in")]
    In,

    /// Is null/not null
    #[serde(rename = "is")]
    Is,
}

impl FilterOperator {
    /// Get the operator string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterOperator::Eq => "eq",
            FilterOperator::Neq => "neq",
            FilterOperator::Gt => "gt",
            FilterOperator::Gte => "gte",
            FilterOperator::Lt => "lt",
            FilterOperator::Lte => "lte",
            FilterOperator::Like => "like",
            FilterOperator::In => "in",
            FilterOperator::Is => "is",
        }
    }
}

/// A filter expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterExpr {
    /// Field to filter on
    pub field: String,

    /// Comparison operator
    pub operator: FilterOperator,

    /// Value to compare against
    pub value: Value,
}

impl FilterExpr {
    /// Create a new filter expression
    pub fn new(field: impl Into<String>, operator: FilterOperator, value: Value) -> Self {
        Self {
            field: field.into(),
            operator,
            value,
        }
    }

    /// Create an equality filter
    pub fn eq(field: impl Into<String>, value: Value) -> Self {
        Self::new(field, FilterOperator::Eq, value)
    }

    /// Create a greater than filter
    pub fn gt(field: impl Into<String>, value: Value) -> Self {
        Self::new(field, FilterOperator::Gt, value)
    }

    /// Create an "in list" filter
    pub fn in_list(field: impl Into<String>, values: Vec<Value>) -> Self {
        Self::new(field, FilterOperator::In, Value::Array(values))
    }

    /// Check if a document matches this filter
    pub fn matches(&self, doc: &Value) -> bool {
        let field_value = match doc.get(&self.field) {
            Some(v) => v,
            None => return self.operator == FilterOperator::Is && self.value.is_null(),
        };

        match self.operator {
            FilterOperator::Eq => field_value == &self.value,
            FilterOperator::Neq => field_value != &self.value,
            FilterOperator::Gt => compare_json_values(field_value, &self.value) > 0,
            FilterOperator::Gte => compare_json_values(field_value, &self.value) >= 0,
            FilterOperator::Lt => compare_json_values(field_value, &self.value) < 0,
            FilterOperator::Lte => compare_json_values(field_value, &self.value) <= 0,
            FilterOperator::Like => {
                if let (Some(field_str), Some(pattern)) =
                    (field_value.as_str(), self.value.as_str())
                {
                    matches_like_pattern(field_str, pattern)
                } else {
                    false
                }
            }
            FilterOperator::In => {
                if let Some(arr) = self.value.as_array() {
                    arr.contains(field_value)
                } else {
                    false
                }
            }
            FilterOperator::Is => {
                if self.value.is_null() {
                    field_value.is_null()
                } else {
                    !field_value.is_null()
                }
            }
        }
    }
}

/// Compare two JSON values for ordering
fn compare_json_values(a: &Value, b: &Value) -> i32 {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let a_f = a.as_f64().unwrap_or(0.0);
            let b_f = b.as_f64().unwrap_or(0.0);
            if a_f < b_f {
                -1
            } else if a_f > b_f {
                1
            } else {
                0
            }
        }
        (Value::String(a), Value::String(b)) => a.cmp(b) as i32,
        _ => 0,
    }
}

/// Simple LIKE pattern matching (% as wildcard)
fn matches_like_pattern(value: &str, pattern: &str) -> bool {
    // Convert SQL LIKE pattern to simple matching
    // % = any sequence, _ = single char
    let pattern = pattern.replace('%', "*").replace('_', "?");

    simple_pattern_match(value, &pattern)
}

/// Simple wildcard pattern matching
fn simple_pattern_match(value: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }

    let mut pattern_chars = pattern.chars().peekable();
    let mut value_chars = value.chars().peekable();

    loop {
        match (pattern_chars.peek(), value_chars.peek()) {
            (None, None) => return true,
            (Some('*'), _) => {
                pattern_chars.next();
                if pattern_chars.peek().is_none() {
                    return true; // Trailing * matches everything
                }
                // Try matching rest of pattern at each position
                while value_chars.peek().is_some() {
                    if simple_pattern_match(
                        &value_chars.clone().collect::<String>(),
                        &pattern_chars.clone().collect::<String>(),
                    ) {
                        return true;
                    }
                    value_chars.next();
                }
                return false;
            }
            (Some('?'), Some(_)) => {
                pattern_chars.next();
                value_chars.next();
            }
            (Some(p), Some(v)) if p == v => {
                pattern_chars.next();
                value_chars.next();
            }
            _ => return false,
        }
    }
}

/// A set of filters combined with AND logic
#[derive(Debug, Clone, Default)]
pub struct FilterSet {
    pub filters: Vec<FilterExpr>,
}

impl FilterSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn and(mut self, filter: FilterExpr) -> Self {
        self.filters.push(filter);
        self
    }

    /// Check if a document matches all filters
    pub fn matches(&self, doc: &Value) -> bool {
        self.filters.iter().all(|f| f.matches(doc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_eq_filter() {
        let filter = FilterExpr::eq("name", json!("Alice"));

        assert!(filter.matches(&json!({"name": "Alice"})));
        assert!(!filter.matches(&json!({"name": "Bob"})));
    }

    #[test]
    fn test_gt_filter() {
        let filter = FilterExpr::gt("age", json!(18));

        assert!(filter.matches(&json!({"age": 21})));
        assert!(!filter.matches(&json!({"age": 18})));
        assert!(!filter.matches(&json!({"age": 15})));
    }

    #[test]
    fn test_in_filter() {
        let filter = FilterExpr::in_list("status", vec![json!("active"), json!("pending")]);

        assert!(filter.matches(&json!({"status": "active"})));
        assert!(filter.matches(&json!({"status": "pending"})));
        assert!(!filter.matches(&json!({"status": "inactive"})));
    }

    #[test]
    fn test_like_filter() {
        let filter = FilterExpr::new("name", FilterOperator::Like, json!("%son"));

        assert!(filter.matches(&json!({"name": "Johnson"})));
        assert!(filter.matches(&json!({"name": "Wilson"})));
        assert!(!filter.matches(&json!({"name": "Smith"})));
    }

    #[test]
    fn test_filter_set() {
        let filters = FilterSet::new()
            .and(FilterExpr::eq("status", json!("active")))
            .and(FilterExpr::gt("age", json!(18)));

        assert!(filters.matches(&json!({
            "status": "active",
            "age": 21
        })));

        assert!(!filters.matches(&json!({
            "status": "inactive",
            "age": 21
        })));
    }
}
