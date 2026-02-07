//! Result sorting for query execution
//!
//! Sorts results by indexed fields only, deterministically.

use super::result::ResultDocument;
use crate::planner::{SortDirection, SortSpec};

/// Sorts result documents
pub struct ResultSorter;

impl ResultSorter {
    /// Sorts documents according to sort specification.
    ///
    /// Sort is stable and deterministic.
    pub fn sort(documents: &mut [ResultDocument], sort_spec: &SortSpec) {
        documents.sort_by(|a, b| {
            let a_val = a.body.get(&sort_spec.field);
            let b_val = b.body.get(&sort_spec.field);

            let ordering = Self::compare_values(a_val, b_val);

            match sort_spec.direction {
                SortDirection::Asc => ordering,
                SortDirection::Desc => ordering.reverse(),
            }
        });
    }

    /// Compares two JSON values for sorting.
    ///
    /// Ordering rules:
    /// - null < bool < number < string
    /// - For same types, natural ordering
    fn compare_values(
        a: Option<&serde_json::Value>,
        b: Option<&serde_json::Value>,
    ) -> std::cmp::Ordering {
        use serde_json::Value;
        use std::cmp::Ordering;

        match (a, b) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (Some(a_val), Some(b_val)) => {
                // Compare by type first
                let type_order = |v: &Value| -> u8 {
                    match v {
                        Value::Null => 0,
                        Value::Bool(_) => 1,
                        Value::Number(_) => 2,
                        Value::String(_) => 3,
                        Value::Array(_) => 4,
                        Value::Object(_) => 5,
                    }
                };

                let a_type = type_order(a_val);
                let b_type = type_order(b_val);

                if a_type != b_type {
                    return a_type.cmp(&b_type);
                }

                // Same type, compare values
                match (a_val, b_val) {
                    (Value::Null, Value::Null) => Ordering::Equal,
                    (Value::Bool(a_b), Value::Bool(b_b)) => a_b.cmp(b_b),
                    (Value::Number(a_n), Value::Number(b_n)) => {
                        let a_f = a_n.as_f64().unwrap_or(0.0);
                        let b_f = b_n.as_f64().unwrap_or(0.0);
                        a_f.partial_cmp(&b_f).unwrap_or(Ordering::Equal)
                    }
                    (Value::String(a_s), Value::String(b_s)) => a_s.cmp(b_s),
                    _ => Ordering::Equal, // Arrays and objects not compared
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_doc(id: &str, age: i64) -> ResultDocument {
        ResultDocument::new(id, "users", "v1", json!({"_id": id, "age": age}), 0)
    }

    #[test]
    fn test_sort_ascending() {
        let mut docs = vec![make_doc("c", 30), make_doc("a", 20), make_doc("b", 25)];

        ResultSorter::sort(&mut docs, &SortSpec::asc("age"));

        assert_eq!(docs[0].id, "a");
        assert_eq!(docs[1].id, "b");
        assert_eq!(docs[2].id, "c");
    }

    #[test]
    fn test_sort_descending() {
        let mut docs = vec![make_doc("c", 30), make_doc("a", 20), make_doc("b", 25)];

        ResultSorter::sort(&mut docs, &SortSpec::desc("age"));

        assert_eq!(docs[0].id, "c");
        assert_eq!(docs[1].id, "b");
        assert_eq!(docs[2].id, "a");
    }

    #[test]
    fn test_sort_stable() {
        // Same age, original order preserved
        let mut docs = vec![make_doc("a", 25), make_doc("b", 25), make_doc("c", 25)];

        ResultSorter::sort(&mut docs, &SortSpec::asc("age"));

        // Order should be stable
        assert_eq!(docs[0].id, "a");
        assert_eq!(docs[1].id, "b");
        assert_eq!(docs[2].id, "c");
    }

    #[test]
    fn test_sort_by_string() {
        fn make_doc_with_name(id: &str, name: &str) -> ResultDocument {
            ResultDocument::new(id, "users", "v1", json!({"_id": id, "name": name}), 0)
        }

        let mut docs = vec![
            make_doc_with_name("1", "charlie"),
            make_doc_with_name("2", "alice"),
            make_doc_with_name("3", "bob"),
        ];

        ResultSorter::sort(&mut docs, &SortSpec::asc("name"));

        assert_eq!(docs[0].id, "2"); // alice
        assert_eq!(docs[1].id, "3"); // bob
        assert_eq!(docs[2].id, "1"); // charlie
    }
}
