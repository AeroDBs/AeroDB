//! Predicate filtering for query execution
//!
//! Filters documents strictly according to predicates.
//! No type coercion, no expressions, exact match only.

use serde_json::Value;

use crate::planner::{FilterOp, Predicate};

/// Evaluates predicates against documents
pub struct PredicateFilter;

impl PredicateFilter {
    /// Checks if a document matches all predicates
    pub fn matches(document: &Value, predicates: &[Predicate]) -> bool {
        // All predicates must match (AND semantics)
        predicates
            .iter()
            .all(|pred| Self::matches_predicate(document, pred))
    }

    /// Checks if a document matches a single predicate
    fn matches_predicate(document: &Value, predicate: &Predicate) -> bool {
        let field_value = match document.get(&predicate.field) {
            Some(v) => v,
            None => return false, // Missing field = no match
        };

        // Null values never match
        if field_value.is_null() {
            return false;
        }

        match &predicate.op {
            FilterOp::Eq(expected) => Self::eq_match(field_value, expected),
            FilterOp::Gte(bound) => Self::gte_match(field_value, bound),
            FilterOp::Gt(bound) => Self::gt_match(field_value, bound),
            FilterOp::Lte(bound) => Self::lte_match(field_value, bound),
            FilterOp::Lt(bound) => Self::lt_match(field_value, bound),
        }
    }

    /// Exact equality match (no coercion)
    fn eq_match(actual: &Value, expected: &Value) -> bool {
        actual == expected
    }

    /// Greater than or equal (numeric only)
    fn gte_match(actual: &Value, bound: &Value) -> bool {
        match (actual, bound) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(af), Some(bf)) = (a.as_f64(), b.as_f64()) {
                    return af >= bf;
                }
                if let (Some(ai), Some(bi)) = (a.as_i64(), b.as_i64()) {
                    return ai >= bi;
                }
                false
            }
            (Value::String(a), Value::String(b)) => a >= b,
            _ => false,
        }
    }

    /// Greater than (numeric only)
    fn gt_match(actual: &Value, bound: &Value) -> bool {
        match (actual, bound) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(af), Some(bf)) = (a.as_f64(), b.as_f64()) {
                    return af > bf;
                }
                if let (Some(ai), Some(bi)) = (a.as_i64(), b.as_i64()) {
                    return ai > bi;
                }
                false
            }
            (Value::String(a), Value::String(b)) => a > b,
            _ => false,
        }
    }

    /// Less than or equal (numeric only)
    fn lte_match(actual: &Value, bound: &Value) -> bool {
        match (actual, bound) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(af), Some(bf)) = (a.as_f64(), b.as_f64()) {
                    return af <= bf;
                }
                if let (Some(ai), Some(bi)) = (a.as_i64(), b.as_i64()) {
                    return ai <= bi;
                }
                false
            }
            (Value::String(a), Value::String(b)) => a <= b,
            _ => false,
        }
    }

    /// Less than (numeric only)
    fn lt_match(actual: &Value, bound: &Value) -> bool {
        match (actual, bound) {
            (Value::Number(a), Value::Number(b)) => {
                if let (Some(af), Some(bf)) = (a.as_f64(), b.as_f64()) {
                    return af < bf;
                }
                if let (Some(ai), Some(bi)) = (a.as_i64(), b.as_i64()) {
                    return ai < bi;
                }
                false
            }
            (Value::String(a), Value::String(b)) => a < b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_equality_match() {
        let doc = json!({"name": "Alice", "age": 30});

        let pred = Predicate::eq("name", json!("Alice"));
        assert!(PredicateFilter::matches(&doc, &[pred]));

        let pred = Predicate::eq("name", json!("Bob"));
        assert!(!PredicateFilter::matches(&doc, &[pred]));
    }

    #[test]
    fn test_no_type_coercion() {
        let doc = json!({"value": 123});

        // String "123" should NOT match integer 123
        let pred = Predicate::eq("value", json!("123"));
        assert!(!PredicateFilter::matches(&doc, &[pred]));

        // Integer 123 should match
        let pred = Predicate::eq("value", json!(123));
        assert!(PredicateFilter::matches(&doc, &[pred]));
    }

    #[test]
    fn test_range_predicates() {
        let doc = json!({"age": 25});

        let pred = Predicate::gte("age", json!(18));
        assert!(PredicateFilter::matches(&doc, &[pred]));

        let pred = Predicate::lte("age", json!(30));
        assert!(PredicateFilter::matches(&doc, &[pred]));

        let pred = Predicate::gt("age", json!(25));
        assert!(!PredicateFilter::matches(&doc, &[pred]));

        let pred = Predicate::lt("age", json!(25));
        assert!(!PredicateFilter::matches(&doc, &[pred]));
    }

    #[test]
    fn test_multiple_predicates_and() {
        let doc = json!({"age": 25, "active": true});

        let preds = vec![
            Predicate::gte("age", json!(18)),
            Predicate::eq("active", json!(true)),
        ];
        assert!(PredicateFilter::matches(&doc, &preds));

        let preds = vec![
            Predicate::gte("age", json!(18)),
            Predicate::eq("active", json!(false)),
        ];
        assert!(!PredicateFilter::matches(&doc, &preds));
    }

    #[test]
    fn test_missing_field_no_match() {
        let doc = json!({"name": "Alice"});

        let pred = Predicate::eq("age", json!(30));
        assert!(!PredicateFilter::matches(&doc, &[pred]));
    }

    #[test]
    fn test_null_value_no_match() {
        let doc = json!({"name": null});

        let pred = Predicate::eq("name", json!("Alice"));
        assert!(!PredicateFilter::matches(&doc, &[pred]));
    }
}
