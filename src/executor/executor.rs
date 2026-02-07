//! Query executor for aerodb
//!
//! Executes query plans against storage, producing deterministic results.
//!
//! Execution flow (strict order per QUERY.md):
//! 1. Use chosen_index to obtain candidate document offsets
//! 2. Read documents from storage
//! 3. Validate checksum on every read
//! 4. Filter documents strictly according to predicates
//! 5. Apply schema version filtering
//! 6. Apply sort (if specified)
//! 7. Apply limit
//! 8. Return ordered results

use serde_json::Value;

use crate::planner::{FilterOp, QueryPlan, ScanType};
use crate::storage::DocumentRecord;

use super::errors::{ExecutorError, ExecutorResult};
use super::filters::PredicateFilter;
use super::result::{ExecutionResult, ResultDocument};
use super::sorter::ResultSorter;

/// Trait for looking up document offsets by index
pub trait IndexLookup {
    /// Get all document offsets for a primary key
    fn lookup_pk(&self, pk: &str) -> Vec<u64>;

    /// Get all document offsets for an indexed field equality
    fn lookup_eq(&self, field: &str, value: &Value) -> Vec<u64>;

    /// Get all document offsets for an indexed field range
    fn lookup_range(&self, field: &str, min: Option<&Value>, max: Option<&Value>) -> Vec<u64>;

    /// Get all document offsets in primary key order
    fn all_offsets_pk_order(&self) -> Vec<u64>;
}

/// Trait for reading documents from storage
pub trait StorageRead {
    /// Read a document at the given offset
    /// Returns None if offset is invalid
    /// Returns Err if checksum fails (corruption)
    fn read_at(&mut self, offset: u64) -> ExecutorResult<Option<DocumentRecord>>;
}

/// Query executor that processes plans against storage
pub struct QueryExecutor<'a, I: IndexLookup, S: StorageRead> {
    index: &'a I,
    storage: &'a mut S,
}

impl<'a, I: IndexLookup, S: StorageRead> QueryExecutor<'a, I, S> {
    /// Creates a new executor
    pub fn new(index: &'a I, storage: &'a mut S) -> Self {
        Self { index, storage }
    }

    /// Executes a query plan and returns results.
    ///
    /// This method is deterministic: same plan + same data = same results.
    pub fn execute(&mut self, plan: &QueryPlan) -> ExecutorResult<ExecutionResult> {
        // Step 1: Use chosen_index to obtain candidate document offsets
        let offsets = self.get_candidate_offsets(plan);

        // Steps 2-5: Read, validate, filter, and check schema
        let mut candidates = Vec::new();
        let mut scanned_count = 0;

        for offset in offsets {
            scanned_count += 1;

            // Step 2-3: Read document with checksum validation
            let record = match self.storage.read_at(offset)? {
                Some(r) => r,
                None => continue, // Invalid offset, skip
            };

            // Skip tombstones
            if record.is_tombstone {
                continue;
            }

            // Step 5: Schema version filtering
            // Extract schema info from document_id (format: collection:id)
            if record.schema_id != plan.schema_id || record.schema_version != plan.schema_version {
                continue; // Schema mismatch, exclude (not error)
            }

            // Parse document body
            let body: Value = match serde_json::from_slice(&record.document_body) {
                Ok(v) => v,
                Err(_) => continue, // Invalid JSON, skip
            };

            // Step 4: Filter according to predicates
            if !PredicateFilter::matches(&body, &plan.predicates) {
                continue;
            }

            // Extract document ID from composite (collection:id -> id)
            let doc_id = record
                .document_id
                .split(':')
                .last()
                .unwrap_or(&record.document_id);

            candidates.push(ResultDocument::new(
                doc_id,
                &record.schema_id,
                &record.schema_version,
                body,
                offset,
            ));
        }

        // Step 6: Apply sort (if specified)
        if let Some(sort_spec) = &plan.sort {
            ResultSorter::sort(&mut candidates, sort_spec);
        }

        // Step 7: Apply limit
        let limit = plan.limit as usize;
        let limit_applied = candidates.len() > limit;
        candidates.truncate(limit);

        // Step 8: Return ordered results
        Ok(ExecutionResult {
            returned_count: candidates.len(),
            scanned_count,
            limit_applied,
            documents: candidates,
        })
    }

    /// Gets candidate document offsets based on plan's chosen index and scan type.
    fn get_candidate_offsets(&self, plan: &QueryPlan) -> Vec<u64> {
        match plan.scan_type {
            ScanType::PrimaryKey => {
                // Find the _id predicate value
                for pred in &plan.predicates {
                    if pred.field == "_id" {
                        if let FilterOp::Eq(ref val) = pred.op {
                            if let Some(pk) = val.as_str() {
                                return self.index.lookup_pk(pk);
                            }
                        }
                    }
                }
                Vec::new()
            }
            ScanType::IndexedEquality => {
                // Find the equality predicate for chosen index
                for pred in &plan.predicates {
                    if pred.field == plan.chosen_index {
                        if let FilterOp::Eq(ref val) = pred.op {
                            return self.index.lookup_eq(&plan.chosen_index, val);
                        }
                    }
                }
                Vec::new()
            }
            ScanType::IndexedRange => {
                // Find range bounds for chosen index
                let mut min = None;
                let mut max = None;

                for pred in &plan.predicates {
                    if pred.field == plan.chosen_index {
                        match &pred.op {
                            FilterOp::Gte(v) | FilterOp::Gt(v) => min = Some(v),
                            FilterOp::Lte(v) | FilterOp::Lt(v) => max = Some(v),
                            _ => {}
                        }
                    }
                }

                self.index.lookup_range(&plan.chosen_index, min, max)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::{BoundednessProof, Predicate, SortSpec};
    use crate::storage::DocumentRecord;
    use serde_json::json;
    use std::collections::HashMap;

    /// Mock index for testing
    struct MockIndex {
        pk_index: HashMap<String, Vec<u64>>,
        field_indexes: HashMap<String, HashMap<String, Vec<u64>>>,
        all_offsets: Vec<u64>,
    }

    impl MockIndex {
        fn new() -> Self {
            Self {
                pk_index: HashMap::new(),
                field_indexes: HashMap::new(),
                all_offsets: Vec::new(),
            }
        }

        fn add_pk(&mut self, pk: &str, offset: u64) {
            self.pk_index
                .entry(pk.to_string())
                .or_default()
                .push(offset);
            if !self.all_offsets.contains(&offset) {
                self.all_offsets.push(offset);
            }
        }

        fn add_field_index(&mut self, field: &str, value: &str, offset: u64) {
            self.field_indexes
                .entry(field.to_string())
                .or_default()
                .entry(value.to_string())
                .or_default()
                .push(offset);
        }
    }

    impl IndexLookup for MockIndex {
        fn lookup_pk(&self, pk: &str) -> Vec<u64> {
            self.pk_index.get(pk).cloned().unwrap_or_default()
        }

        fn lookup_eq(&self, field: &str, value: &Value) -> Vec<u64> {
            let key = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => return Vec::new(),
            };
            self.field_indexes
                .get(field)
                .and_then(|m| m.get(&key))
                .cloned()
                .unwrap_or_default()
        }

        fn lookup_range(
            &self,
            _field: &str,
            _min: Option<&Value>,
            _max: Option<&Value>,
        ) -> Vec<u64> {
            // For testing, return all offsets
            self.all_offsets.clone()
        }

        fn all_offsets_pk_order(&self) -> Vec<u64> {
            let mut offsets = self.all_offsets.clone();
            offsets.sort();
            offsets
        }
    }

    /// Mock storage for testing
    struct MockStorage {
        records: HashMap<u64, DocumentRecord>,
        corrupt_offsets: Vec<u64>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                records: HashMap::new(),
                corrupt_offsets: Vec::new(),
            }
        }

        fn add_record(&mut self, offset: u64, record: DocumentRecord) {
            self.records.insert(offset, record);
        }

        fn mark_corrupt(&mut self, offset: u64) {
            self.corrupt_offsets.push(offset);
        }
    }

    impl StorageRead for MockStorage {
        fn read_at(&mut self, offset: u64) -> ExecutorResult<Option<DocumentRecord>> {
            if self.corrupt_offsets.contains(&offset) {
                return Err(ExecutorError::data_corruption(offset, "checksum mismatch"));
            }
            Ok(self.records.get(&offset).cloned())
        }
    }

    fn make_record(id: &str, schema_id: &str, version: &str, body: Value) -> DocumentRecord {
        DocumentRecord {
            document_id: format!("{}:{}", schema_id, id),
            schema_id: schema_id.to_string(),
            schema_version: version.to_string(),
            is_tombstone: false,
            document_body: serde_json::to_vec(&body).unwrap(),
        }
    }

    fn make_plan(
        schema_id: &str,
        version: &str,
        index: &str,
        scan_type: ScanType,
        predicates: Vec<Predicate>,
        limit: u64,
    ) -> QueryPlan {
        QueryPlan {
            collection: schema_id.to_string(),
            schema_id: schema_id.to_string(),
            schema_version: version.to_string(),
            chosen_index: index.to_string(),
            scan_type,
            predicates,
            sort: None,
            limit,
            bounds_proof: BoundednessProof::pk_lookup(),
        }
    }

    #[test]
    fn test_pk_lookup_execution() {
        let mut index = MockIndex::new();
        index.add_pk("user_1", 100);

        let mut storage = MockStorage::new();
        storage.add_record(
            100,
            make_record(
                "user_1",
                "users",
                "v1",
                json!({"_id": "user_1", "name": "Alice"}),
            ),
        );

        let plan = make_plan(
            "users",
            "v1",
            "_id",
            ScanType::PrimaryKey,
            vec![Predicate::eq("_id", json!("user_1"))],
            1,
        );

        let mut executor = QueryExecutor::new(&index, &mut storage);
        let result = executor.execute(&plan).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.documents[0].id, "user_1");
    }

    #[test]
    fn test_indexed_equality_execution() {
        let mut index = MockIndex::new();
        index.add_pk("user_1", 100);
        index.add_pk("user_2", 200);
        index.add_field_index("email", "alice@example.com", 100);
        index.add_field_index("email", "bob@example.com", 200);

        let mut storage = MockStorage::new();
        storage.add_record(
            100,
            make_record(
                "user_1",
                "users",
                "v1",
                json!({"_id": "user_1", "email": "alice@example.com"}),
            ),
        );
        storage.add_record(
            200,
            make_record(
                "user_2",
                "users",
                "v1",
                json!({"_id": "user_2", "email": "bob@example.com"}),
            ),
        );

        let plan = make_plan(
            "users",
            "v1",
            "email",
            ScanType::IndexedEquality,
            vec![Predicate::eq("email", json!("alice@example.com"))],
            10,
        );

        let mut executor = QueryExecutor::new(&index, &mut storage);
        let result = executor.execute(&plan).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.documents[0].id, "user_1");
    }

    #[test]
    fn test_indexed_range_with_limit() {
        let mut index = MockIndex::new();
        for i in 1..=5 {
            index.add_pk(&format!("user_{}", i), i as u64 * 100);
        }

        let mut storage = MockStorage::new();
        for i in 1..=5 {
            storage.add_record(
                i as u64 * 100,
                make_record(
                    &format!("user_{}", i),
                    "users",
                    "v1",
                    json!({"_id": format!("user_{}", i), "age": 20 + i}),
                ),
            );
        }

        let plan = make_plan(
            "users",
            "v1",
            "age",
            ScanType::IndexedRange,
            vec![
                Predicate::gte("age", json!(21)),
                Predicate::lte("age", json!(24)),
            ],
            2,
        );

        let mut executor = QueryExecutor::new(&index, &mut storage);
        let result = executor.execute(&plan).unwrap();

        // Should return 2 (limit applied) out of 4 matching
        assert_eq!(result.len(), 2);
        assert!(result.limit_applied);
    }

    #[test]
    fn test_schema_mismatch_excluded() {
        let mut index = MockIndex::new();
        index.add_pk("user_1", 100);
        index.add_pk("user_2", 200);

        let mut storage = MockStorage::new();
        storage.add_record(
            100,
            make_record(
                "user_1",
                "users",
                "v1",
                json!({"_id": "user_1", "name": "Alice"}),
            ),
        );
        storage.add_record(
            200,
            make_record(
                "user_2",
                "users",
                "v2", // Different version!
                json!({"_id": "user_2", "name": "Bob"}),
            ),
        );

        // Query for v1 only
        let mut plan = make_plan(
            "users",
            "v1",
            "_id",
            ScanType::PrimaryKey,
            vec![Predicate::eq("_id", json!("user_2"))],
            10,
        );
        plan.chosen_index = "_id".to_string();

        // Override to get user_2
        let index2 = MockIndex {
            pk_index: {
                let mut m = HashMap::new();
                m.insert("user_2".to_string(), vec![200]);
                m
            },
            field_indexes: HashMap::new(),
            all_offsets: vec![200],
        };

        let mut executor = QueryExecutor::new(&index2, &mut storage);
        let result = executor.execute(&plan).unwrap();

        // user_2 is v2, but we query v1 -> excluded, not error
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_corruption_halts_execution() {
        let mut index = MockIndex::new();
        index.add_pk("user_1", 100);

        let mut storage = MockStorage::new();
        storage.add_record(
            100,
            make_record(
                "user_1",
                "users",
                "v1",
                json!({"_id": "user_1", "name": "Alice"}),
            ),
        );
        storage.mark_corrupt(100);

        let plan = make_plan(
            "users",
            "v1",
            "_id",
            ScanType::PrimaryKey,
            vec![Predicate::eq("_id", json!("user_1"))],
            1,
        );

        let mut executor = QueryExecutor::new(&index, &mut storage);
        let result = executor.execute(&plan);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_fatal());
        assert_eq!(err.code().code(), "AERO_DATA_CORRUPTION");
    }

    #[test]
    fn test_deterministic_ordering() {
        let mut index = MockIndex::new();
        for i in 1..=3 {
            index.add_pk(&format!("user_{}", i), i as u64 * 100);
        }

        let mut storage = MockStorage::new();
        for i in 1..=3 {
            storage.add_record(
                i as u64 * 100,
                make_record(
                    &format!("user_{}", i),
                    "users",
                    "v1",
                    json!({"_id": format!("user_{}", i), "age": 30 - i}),
                ),
            );
        }

        let mut plan = make_plan(
            "users",
            "v1",
            "age",
            ScanType::IndexedRange,
            vec![Predicate::gte("age", json!(27))],
            10,
        );
        plan.sort = Some(SortSpec::asc("age"));

        // Execute multiple times
        let mut executor = QueryExecutor::new(&index, &mut storage);
        let result1 = executor.execute(&plan).unwrap();

        let mut executor = QueryExecutor::new(&index, &mut storage);
        let result2 = executor.execute(&plan).unwrap();

        // Results must be identical
        assert_eq!(result1.len(), result2.len());
        for (d1, d2) in result1.documents.iter().zip(result2.documents.iter()) {
            assert_eq!(d1.id, d2.id);
        }
    }

    #[test]
    fn test_limit_enforced() {
        let mut index = MockIndex::new();
        for i in 1..=10 {
            index.add_pk(&format!("user_{}", i), i as u64 * 100);
        }

        let mut storage = MockStorage::new();
        for i in 1..=10 {
            storage.add_record(
                i as u64 * 100,
                make_record(
                    &format!("user_{}", i),
                    "users",
                    "v1",
                    json!({"_id": format!("user_{}", i), "age": 20 + i}),
                ),
            );
        }

        let plan = make_plan(
            "users",
            "v1",
            "age",
            ScanType::IndexedRange,
            vec![Predicate::gte("age", json!(21))],
            3, // Limit to 3
        );

        let mut executor = QueryExecutor::new(&index, &mut storage);
        let result = executor.execute(&plan).unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.limit_applied);
    }

    #[test]
    fn test_replay_stability() {
        // Same setup as deterministic_ordering
        let mut index = MockIndex::new();
        index.add_pk("user_1", 100);

        let mut storage = MockStorage::new();
        storage.add_record(
            100,
            make_record(
                "user_1",
                "users",
                "v1",
                json!({"_id": "user_1", "name": "Alice"}),
            ),
        );

        let plan = make_plan(
            "users",
            "v1",
            "_id",
            ScanType::PrimaryKey,
            vec![Predicate::eq("_id", json!("user_1"))],
            1,
        );

        // Execute 3 times
        for _ in 0..3 {
            let mut executor = QueryExecutor::new(&index, &mut storage);
            let result = executor.execute(&plan).unwrap();
            assert_eq!(result.len(), 1);
            assert_eq!(result.documents[0].id, "user_1");
        }
    }
}
